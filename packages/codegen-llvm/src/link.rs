use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::compile::Artifact;
use crate::error::Error;
use crate::host::javascript_runtime;
use crate::target::{Options, Target};

/// The JavaScript loader of wasm programs, with `__FA_WASM__` standing for
/// the name of the `.wasm` file.
const LOADER: &str = include_str!("../host/loader.mjs");

/// The LLVM install the crate was built with (`LLVM_SYS_181_PREFIX`).
pub fn llvm_prefix() -> PathBuf {
	PathBuf::from(env!("FA_LLVM_PREFIX"))
}

/// Links an object into an executable (`cc`, or `$CC`; `ld.lld` when
/// freestanding) or a `.wasm` module (`wasm-ld`), from the LLVM install.
pub fn link(target: Target, options: &Options, object: &Path, output: &Path) -> Result<(), Error> {
	let mut command = match target {
		Target::Native => {
			let mut command = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
			command.arg(object).arg("-o").arg(output);
			// The compiler driver knows the startup objects and the libc;
			// `ld.lld` links (smaller and faster than GNU ld).
			let mut lld_directory = OsString::from("-B");
			lld_directory.push(llvm_prefix().join("bin"));
			command.arg("-fuse-ld=lld").arg(lld_directory);
			// `pow` and `floor` of the floats
			command.arg("-lm");
			if options.optimized() {
				command.arg("-Wl,--gc-sections");
			}
			if options.size {
				// Strip, and do not page-align the code apart from the
				// headers and data: a small program fits in a few pages.
				command.args(["-s", "-Wl,-z,noseparate-code", "-Wl,--build-id=none"]);
			}
			command
		}
		Target::NativeFreestanding => {
			let mut command = Command::new(llvm_prefix().join("bin/ld.lld"));
			command.arg(object).arg("-o").arg(output);
			command.args(["-static", "-z", "noexecstack"]);
			if options.optimized() {
				command.arg("--gc-sections");
			}
			if options.size {
				// No separate pages for the read-only data and the code, and no
				// RELRO page: two segments in all.
				command.args([
					"-s",
					"-z",
					"norelro",
					"-z",
					"noseparate-code",
					"--build-id=none",
				]);
			}
			command
		}
		Target::Wasm => {
			let mut command = Command::new(llvm_prefix().join("bin/wasm-ld"));
			command.arg(object).arg("-o").arg(output);
			// A 1 MiB stack, as Rust's wasm32 targets; the memory grows as
			// the allocator needs.
			command.args(["-z", "stack-size=1048576"]);
			if options.optimized() {
				command.args(["--gc-sections", "--strip-all"]);
			}
			command
		}
	};
	checked(&mut command)?;
	Ok(())
}

/// Writes the JavaScript loader of a `.wasm` module next to it
/// (`program.wasm` → `program.mjs`).
pub fn write_loader(wasm: &Path) -> Result<PathBuf, Error> {
	let name = wasm
		.file_name()
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_default();
	let stem = wasm.to_string_lossy();
	let loader = crate::compile::with_suffix(
		Path::new(stem.strip_suffix(".wasm").unwrap_or(&stem)),
		".mjs",
	);
	std::fs::write(&loader, LOADER.replace("__FA_WASM__", &name))?;
	Ok(loader)
}

/// Runs a linked program: directly on native, with `node` (or `bun`, or
/// `$FA_JS_RUNTIME`) and its loader on wasm.
pub fn run(artifact: &Artifact, arguments: &[&str]) -> Result<Output, Error> {
	let mut command = match (&artifact.loader, artifact.target) {
		(Some(loader), Target::Wasm) => {
			let mut command = Command::new(javascript_runtime()?);
			command.arg(loader);
			command
		}
		_ => Command::new(&artifact.output),
	};
	command.args(arguments);
	command.output().map_err(|error| Error::Tool {
		command: format!("{command:?}"),
		message: error.to_string(),
	})
}

fn checked(command: &mut Command) -> Result<Output, Error> {
	let output = command.output().map_err(|error| Error::Tool {
		command: format!("{command:?}"),
		message: error.to_string(),
	})?;
	if !output.status.success() {
		return Err(Error::Tool {
			command: format!("{command:?}"),
			message: String::from_utf8_lossy(&output.stderr).into_owned(),
		});
	}
	Ok(output)
}
