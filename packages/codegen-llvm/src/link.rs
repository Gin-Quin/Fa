use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::compile::Artifact;
use crate::error::Error;
use crate::host::javascript_runtime;
use crate::options::Libraries;
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
pub fn link(
	target: Target,
	options: &Options,
	object: &Path,
	output: &Path,
	libraries: &Libraries,
) -> Result<(), Error> {
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
			// the threads of `for parallel` (`fa_parallel_for`)
			command.arg("-pthread");
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
	// the C libraries of foreign code, after the object that uses them
	for path in &libraries.paths {
		command.arg(format!("-L{}", path.display()));
		if target == Target::Native {
			command.arg(format!("-Wl,-rpath,{}", path.display()));
		}
	}
	for library in &libraries.link {
		let file = [".a", ".so", ".o", ".dylib"]
			.iter()
			.any(|extension| library.ends_with(extension))
			|| library.contains('/');
		if file {
			command.arg(library);
		} else {
			command.arg(format!("-l{library}"));
		}
	}
	checked(&mut command)?;
	Ok(())
}

/// The size (instructions) of the functions ThinLTO imports into the units
/// calling them (100 by default): importing the runtime's medium-sized
/// functions (the allocator's slow paths) into every unit, where they are
/// inlined, made a 100-file program 2.4 times larger.
const IMPORT_LIMIT: u32 = 40; // FA_IMPORT_LIMIT overrides (experiments)

fn import_limit() -> u32 {
	std::env::var("FA_IMPORT_LIMIT")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(IMPORT_LIMIT)
}

/// Links the objects of a program compiled per unit (`units.rs`): ThinLTO
/// bitcode in release builds (optimized across the units by the linker, on
/// `split.jobs` threads), plain objects with `--gc-sections` in development.
pub fn link_objects(
	target: Target,
	options: &Options,
	split: &crate::split::SplitOptions,
	objects: &[PathBuf],
	output: &Path,
	libraries: &Libraries,
) -> Result<(), Error> {
	let jobs = match split.jobs {
		0 => std::thread::available_parallelism().map_or(1, usize::from),
		jobs => jobs,
	};
	// the optimization level of the link's ThinLTO backends
	let lto_level = match options.opt_level {
		_ if options.size => "2",
		crate::OptLevel::O0 | crate::OptLevel::O1 => "1",
		crate::OptLevel::O2 => "2",
		crate::OptLevel::O3 => "3",
	};
	let mut command = match target {
		Target::Native => {
			let mut command = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
			command.args(objects).arg("-o").arg(output);
			let mut lld_directory = OsString::from("-B");
			lld_directory.push(llvm_prefix().join("bin"));
			command.arg("-fuse-ld=lld").arg(lld_directory);
			command.arg("-lm").arg("-pthread");
			command.arg("-Wl,--gc-sections");
			if split.lto {
				command.arg(format!("-Wl,--thinlto-jobs={jobs}"));
				command.arg(format!("-Wl,--lto-O{lto_level}"));
				command.arg(format!("-Wl,-mllvm,-import-instr-limit={}", import_limit()));
			}
			if options.size {
				command.args(["-s", "-Wl,-z,noseparate-code", "-Wl,--build-id=none"]);
			}
			command
		}
		Target::NativeFreestanding => {
			let mut command = Command::new(llvm_prefix().join("bin/ld.lld"));
			command.args(objects).arg("-o").arg(output);
			command.args(["-static", "-z", "noexecstack", "--gc-sections"]);
			if split.lto {
				command.arg(format!("--thinlto-jobs={jobs}"));
				command.arg(format!("--lto-O{lto_level}"));
				command.args(["-mllvm", &format!("-import-instr-limit={}", import_limit())]);
			}
			if options.size {
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
			command.args(objects).arg("-o").arg(output);
			command.args(["-z", "stack-size=1048576", "--gc-sections"]);
			if split.lto {
				command.arg(format!("--thinlto-jobs={jobs}"));
				command.arg(format!("--lto-O{lto_level}"));
				command.args(["-mllvm", &format!("-import-instr-limit={}", import_limit())]);
			}
			if options.optimized() {
				command.arg("--strip-all");
			}
			command
		}
	};
	for path in &libraries.paths {
		command.arg(format!("-L{}", path.display()));
		if target == Target::Native {
			command.arg(format!("-Wl,-rpath,{}", path.display()));
		}
	}
	for library in &libraries.link {
		let file = [".a", ".so", ".o", ".dylib"]
			.iter()
			.any(|extension| library.ends_with(extension))
			|| library.contains('/');
		if file {
			command.arg(library);
		} else {
			command.arg(format!("-l{library}"));
		}
	}
	checked(&mut command)?;
	Ok(())
}

/// The functions a `.wasm` module imports from the `fa` module of its
/// loader (its import section).
pub fn wasm_imports(path: &Path) -> Result<Vec<String>, Error> {
	let bytes = std::fs::read(path)?;
	let invalid = || Error::Llvm(format!("`{}` is not a valid wasm module", path.display()));
	fn leb(bytes: &[u8], at: &mut usize) -> Option<u64> {
		let mut value = 0u64;
		let mut shift = 0;
		loop {
			let byte = *bytes.get(*at)?;
			*at += 1;
			value |= u64::from(byte & 0x7f) << shift;
			if byte & 0x80 == 0 {
				return Some(value);
			}
			shift += 7;
			if shift > 63 {
				return None;
			}
		}
	}
	fn name<'b>(bytes: &'b [u8], at: &mut usize) -> Option<&'b [u8]> {
		let length = leb(bytes, at)? as usize;
		let text = bytes.get(*at..*at + length)?;
		*at += length;
		Some(text)
	}
	fn limits(bytes: &[u8], at: &mut usize) -> Option<()> {
		let flags = *bytes.get(*at)?;
		*at += 1;
		leb(bytes, at)?;
		if flags & 1 != 0 {
			leb(bytes, at)?;
		}
		Some(())
	}
	let mut imports = Vec::new();
	let mut at = 8;
	while at < bytes.len() {
		let id = bytes[at];
		at += 1;
		let size = leb(&bytes, &mut at).ok_or_else(invalid)? as usize;
		let end = at + size;
		if id != 2 {
			at = end;
			continue;
		}
		let parse = |at: &mut usize, imports: &mut Vec<String>| -> Option<()> {
			let count = leb(&bytes, at)?;
			for _ in 0..count {
				let module = name(&bytes, at)?;
				let field = name(&bytes, at)?;
				let kind = *bytes.get(*at)?;
				*at += 1;
				match kind {
					0 => {
						leb(&bytes, at)?;
					}
					1 => {
						*at += 1;
						limits(&bytes, at)?;
					}
					2 => limits(&bytes, at)?,
					3 => *at += 2,
					_ => return None,
				}
				if kind == 0 && module == b"fa" {
					imports.push(String::from_utf8_lossy(field).into_owned());
				}
			}
			Some(())
		};
		parse(&mut at, &mut imports).ok_or_else(invalid)?;
		break;
	}
	Ok(imports)
}

/// The loader for a module importing these functions from `fa`: the parts
/// between `// @import <name>` and `// @end` stay only when `<name>` is
/// imported.
pub fn loader_for(imports: &[String]) -> String {
	let mut loader = String::new();
	let mut kept = true;
	for line in LOADER.lines() {
		let trimmed = line.trim();
		if let Some(name) = trimmed.strip_prefix("// @import ") {
			kept = imports.iter().any(|import| import == name);
			continue;
		}
		if trimmed == "// @end" {
			kept = true;
			continue;
		}
		if kept {
			loader.push_str(line);
			loader.push('\n');
		}
	}
	loader
}

/// Writes the JavaScript loader of a `.wasm` module next to it
/// (`program.wasm` → `program.mjs`), for the functions it imports.
pub fn write_loader(wasm: &Path, imports: &[String]) -> Result<PathBuf, Error> {
	let name = wasm
		.file_name()
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_default();
	let stem = wasm.to_string_lossy();
	let loader = crate::compile::with_suffix(
		Path::new(stem.strip_suffix(".wasm").unwrap_or(&stem)),
		".mjs",
	);
	std::fs::write(&loader, loader_for(imports).replace("__FA_WASM__", &name))?;
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
