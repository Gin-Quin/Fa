use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::error::Error;
use crate::target::{Profile, Target};

/// The LLVM install the crate was built with (`LLVM_SYS_181_PREFIX`).
pub fn llvm_prefix() -> PathBuf {
	PathBuf::from(env!("FA_LLVM_PREFIX"))
}

/// The JavaScript host that instantiates a wasm program (`host/run.mjs`).
pub fn wasm_host() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("host/run.mjs")
}

/// Links an object into an executable (`cc`, or `$CC`; `ld.lld` when
/// freestanding) or a `.wasm` module (`wasm-ld`), from the LLVM install.
pub fn link(target: Target, profile: Profile, object: &Path, output: &Path) -> Result<(), Error> {
	let mut command = match target {
		Target::Native => {
			let mut command = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
			command.arg(object).arg("-o").arg(output);
			// The compiler driver knows the startup objects and the libc;
			// `ld.lld` links (smaller and faster than GNU ld).
			let mut lld_directory = OsString::from("-B");
			lld_directory.push(llvm_prefix().join("bin"));
			command.arg("-fuse-ld=lld").arg(lld_directory);
			if profile != Profile::Dev {
				command.arg("-Wl,--gc-sections");
			}
			if profile == Profile::Size {
				// Strip, and do not page-align the code apart from the
				// headers and data: a small program fits in a few pages.
				command.args(["-s", "-Wl,-z,noseparate-code", "-Wl,--build-id=none"]);
			}
			command
		}
		Target::Freestanding => {
			let mut command = Command::new(llvm_prefix().join("bin/ld.lld"));
			command.arg(object).arg("-o").arg(output);
			command.args(["-static", "-z", "noexecstack"]);
			if profile != Profile::Dev {
				command.arg("--gc-sections");
			}
			if profile == Profile::Size {
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
			if profile != Profile::Dev {
				command.args(["--gc-sections", "--strip-all"]);
			}
			command
		}
	};
	checked(&mut command)?;
	Ok(())
}

/// Runs a linked program: directly on native, with `node` (or `bun`, or
/// `$FA_JS_RUNTIME`) and `host/run.mjs` for wasm.
pub fn run(target: Target, output: &Path) -> Result<Output, Error> {
	let mut command = match target {
		Target::Native | Target::Freestanding => Command::new(output),
		Target::Wasm => {
			let mut command = Command::new(javascript_runtime()?);
			command.arg(wasm_host()).arg(output);
			command
		}
	};
	command.output().map_err(|error| Error::Tool {
		command: format!("{command:?}"),
		message: error.to_string(),
	})
}

/// The JavaScript runtime used to run wasm programs.
pub fn javascript_runtime() -> Result<OsString, Error> {
	if let Some(runtime) = env::var_os("FA_JS_RUNTIME") {
		return Ok(runtime);
	}
	let path = env::var_os("PATH").unwrap_or_default();
	for name in ["node", "bun"] {
		if env::split_paths(&path).any(|directory| directory.join(name).is_file()) {
			return Ok(name.into());
		}
	}
	let bun = env::var_os("HOME").map(|home| PathBuf::from(home).join(".bun/bin/bun"));
	match bun {
		Some(bun) if bun.is_file() => Ok(bun.into()),
		_ => Err(Error::Tool {
			command: "node".to_string(),
			message: "neither node nor bun was found (set FA_JS_RUNTIME)".to_string(),
		}),
	}
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
