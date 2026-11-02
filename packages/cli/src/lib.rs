//! The `fa` command (design/decisions.md, "Programs and the command line"):
//!
//! - `fa check <file.fa>`: analyzes a file and prints its diagnostics; exits
//!   with 1 when there are errors.
//! - `fa build <file.fa> [--target=<target>] [--output=<directory>]
//!   [--declarations]`: compiles a file to `<directory>/main.js` (by default
//!   `build/<target>/` next to the file). The targets are `bun` (the
//!   default), `node`, `deno` and `web`; `native` (an executable),
//!   `native-freestanding` (no C library, Linux x86-64) and `wasm` (a `.wasm`
//!   module and its `.mjs` loader) compile with the LLVM backend
//!   (`fa-codegen-llvm`, when it was built with LLVM): `-O0` to `-O3`
//!   (`-O3` by default), `--size`, and `--overflow-checks=true|false`: a
//!   build doesn't check integer overflows by default (they wrap), `fa run`
//!   does.
//! - `fa run [--target=<target>] <file.fa> [--] [arguments...]`: compiles a
//!   file for Bun into a cache directory and runs it with Bun (found on
//!   `PATH` or in `~/.bun/bin`), forwarding the arguments and the exit code;
//!   with `--target=native`, `native-freestanding` or `wasm`, compiles it
//!   with the LLVM backend and runs the executable (Node or Bun for wasm).

use std::ffi::OsString;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::ids::FileId;
use fa_analyzer::{Session, Severity};
use fa_codegen_js::{GenerateError, JsOutput, Options, Target};

/// The exit code of a usage error.
const USAGE: i32 = 2;

const HELP: &str = "\
fa: the Fa toolchain

Usage:
  fa check <file.fa>
  fa build <file.fa> [--target=bun|node|deno|web|native|native-freestanding|wasm]
           [--output=<directory>] [--declarations] [-O0|-O1|-O2|-O3] [--size]
           [--overflow-checks=true|false]
  fa run [--target=<target>] [--overflow-checks=true|false] <file.fa> [--] [arguments...]

Commands:
  check   analyze a file and print its diagnostics
  build   compile a file (into build/<target>/ next to it by default)
  run     compile a file for Bun and run it with its arguments

Integer overflows stop the program in `fa run` and are not checked in
`fa build` (they wrap, or lose precision beyond 2^53 on JavaScript);
--overflow-checks overrides it, on every target.
";

/// Runs the command line `arguments` (without the program name) and returns
/// the exit code.
pub fn main(arguments: Vec<OsString>) -> i32 {
	let mut arguments = arguments.into_iter();
	let Some(command) = arguments.next() else {
		eprint!("{HELP}");
		return USAGE;
	};
	let rest: Vec<OsString> = arguments.collect();
	match command.to_str() {
		Some("check") => check(&rest),
		Some("build") => build(&rest),
		Some("run") => run(&rest),
		Some("help" | "--help" | "-h") => {
			print!("{HELP}");
			0
		}
		Some("--version" | "-V") => {
			println!("fa {}", env!("CARGO_PKG_VERSION"));
			0
		}
		_ => {
			eprintln!(
				"fa: unknown command `{}`\n\n{HELP}",
				command.to_string_lossy()
			);
			USAGE
		}
	}
}

/// A source file loaded and analyzed.
struct Loaded {
	session: Session,
	file: FileId,
	/// The path as written on the command line, for the messages.
	shown: String,
}

impl Loaded {
	/// Reads and analyzes a file.
	fn new(path: &Path) -> Result<Self, String> {
		let text = fs::read_to_string(path)
			.map_err(|error| format!("fa: can't read `{}`: {error}", path.display()))?;
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(text));
		session.analyze_file(file);
		Ok(Self {
			session,
			file,
			shown: path.display().to_string(),
		})
	}

	/// Whether the analysis found errors.
	fn has_errors(&self) -> bool {
		self.session
			.diagnostics(self.file)
			.iter()
			.any(|diagnostic| diagnostic.severity == Severity::Error)
	}

	/// The diagnostics of the analysis, rendered.
	fn rendered(&self) -> String {
		self.session.render_diagnostics(self.file, &self.shown)
	}

	/// Generates the JavaScript; on failure, the rendered errors.
	fn generate(&mut self, options: &Options) -> Result<JsOutput, String> {
		match fa_codegen_js::generate(&mut self.session, self.file, options) {
			Ok(output) => Ok(output),
			Err(GenerateError { diagnostics }) => {
				let source = SourceView::new(&self.shown, self.session.text(self.file));
				Err(render_all(&diagnostics, &source))
			}
		}
	}
}

/// The single file argument of a command.
fn file_argument(arguments: &[OsString], command: &str) -> Result<PathBuf, i32> {
	match arguments {
		[file] => Ok(PathBuf::from(file)),
		[] => {
			eprintln!("fa {command}: which file? ex: `fa {command} main.fa`");
			Err(USAGE)
		}
		_ => {
			eprintln!(
				"fa {command}: unexpected argument `{}`",
				arguments[1].to_string_lossy()
			);
			Err(USAGE)
		}
	}
}

/// `fa check <file.fa>`.
fn check(arguments: &[OsString]) -> i32 {
	let path = match file_argument(arguments, "check") {
		Ok(path) => path,
		Err(code) => return code,
	};
	let loaded = match Loaded::new(&path) {
		Ok(loaded) => loaded,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	eprint!("{}", loaded.rendered());
	i32::from(loaded.has_errors())
}

/// `fa build <file.fa> [--target=…] [--output=…] [--declarations]`.
fn build(arguments: &[OsString]) -> i32 {
	let mut target = "bun".to_string();
	let mut output: Option<PathBuf> = None;
	let mut declarations = false;
	// a release build: integers wrap, like Rust's (design/decisions.md,
	// "Overflow checks"); `--overflow-checks=true` checks them
	let mut native = fa_codegen_llvm::Options {
		overflow_checks: false,
		..fa_codegen_llvm::Options::default()
	};
	let mut files = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if native_option(&text, &mut native) {
			continue;
		}
		if let Some(value) = text.strip_prefix("--target=") {
			target = value.to_string();
		} else if let Some(value) = text.strip_prefix("--output=") {
			output = Some(PathBuf::from(value));
		} else if text == "--declarations" {
			declarations = true;
		} else if text.starts_with("--") {
			eprintln!("fa build: unknown option `{text}`");
			return USAGE;
		} else {
			files.push(argument.clone());
		}
	}
	let path = match file_argument(&files, "build") {
		Ok(path) => path,
		Err(code) => return code,
	};
	if let Some(native_target) = fa_codegen_llvm::Target::from_name(&target) {
		let directory = output.unwrap_or_else(|| {
			path.parent()
				.unwrap_or(Path::new("."))
				.join("build")
				.join(native_target.name())
		});
		return match build_native(&path, native_target, &native, &directory) {
			Ok(artifact) => {
				eprintln!(
					"fa build: wrote {}",
					artifact
						.loader
						.as_ref()
						.unwrap_or(&artifact.output)
						.display()
				);
				0
			}
			Err(message) => {
				eprint!("{message}");
				1
			}
		};
	}
	let target = match Target::from_name(&target) {
		Some(target) => target,
		None => {
			eprintln!(
				"fa build: unknown target `{target}`: the targets are `bun`, `node`, `deno`, `web`, `native` and `wasm`"
			);
			return USAGE;
		}
	};
	let directory = output.unwrap_or_else(|| {
		path.parent()
			.unwrap_or(Path::new("."))
			.join("build")
			.join(target.name())
	});
	let mut loaded = match Loaded::new(&path) {
		Ok(loaded) => loaded,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	eprint!("{}", loaded.rendered());
	if loaded.has_errors() {
		return 1;
	}
	let mut options = Options::new(target, loaded.shown.clone());
	options.declarations = declarations;
	// one `--overflow-checks` for every target
	options.overflow_checks = native.overflow_checks;
	let generated = match loaded.generate(&options) {
		Ok(generated) => generated,
		Err(rendered) => {
			eprint!("{rendered}");
			return 1;
		}
	};
	match write_build(&directory, target, &generated) {
		Ok(main) => {
			eprintln!("fa build: wrote {}", main.display());
			0
		}
		Err(error) => {
			eprintln!(
				"fa build: can't write into `{}`: {error}",
				directory.display()
			);
			1
		}
	}
}

/// Writes `main.js` (and `main.d.ts`, and a `package.json` declaring an ES
/// module for Node) into a directory; returns the path of `main.js`.
fn write_build(directory: &Path, target: Target, output: &JsOutput) -> std::io::Result<PathBuf> {
	fs::create_dir_all(directory)?;
	let main = directory.join("main.js");
	fs::write(&main, &output.code)?;
	if let Some(declarations) = &output.declarations {
		fs::write(directory.join("main.d.ts"), declarations)?;
	}
	if target == Target::Node {
		fs::write(directory.join("package.json"), "{ \"type\": \"module\" }\n")?;
	}
	Ok(main)
}

/// The options of the LLVM backend: `-O0` to `-O3`, `--size`,
/// `--overflow-checks=false`.
fn native_option(text: &str, options: &mut fa_codegen_llvm::Options) -> bool {
	match text {
		"--overflow-checks=false" => {
			options.overflow_checks = false;
			return true;
		}
		"--overflow-checks=true" => {
			options.overflow_checks = true;
			return true;
		}
		_ => {}
	}
	options.opt_level = match text {
		"-O0" => fa_codegen_llvm::OptLevel::O0,
		"-O1" => fa_codegen_llvm::OptLevel::O1,
		"-O2" => fa_codegen_llvm::OptLevel::O2,
		"-O3" => fa_codegen_llvm::OptLevel::O3,
		"--size" => {
			options.size = true;
			return true;
		}
		_ => return false,
	};
	true
}

/// Compiles a file with the LLVM backend into `directory` (`main`, or
/// `main.wasm` and `main.mjs`); on failure, the message to print.
fn build_native(
	path: &Path,
	target: fa_codegen_llvm::Target,
	options: &fa_codegen_llvm::Options,
	directory: &Path,
) -> Result<fa_codegen_llvm::Artifact, String> {
	if !fa_codegen_llvm::llvm_available() {
		return Err(format!(
			"fa: the `{}` target needs the LLVM backend, and this `fa` was built without LLVM: run scripts/setup-llvm.sh, export LLVM_SYS_181_PREFIX and rebuild\n",
			target.name()
		));
	}
	let loaded = Loaded::new(path)?;
	let rendered = loaded.rendered();
	if loaded.has_errors() {
		return Err(rendered);
	}
	eprint!("{rendered}");
	fa_codegen_llvm::compile(
		&loaded.session,
		loaded.file,
		&loaded.shown,
		target,
		options,
		&directory.join("main"),
	)
	.map_err(|error| format!("{error}\n"))
}

/// `fa run [--target=…] <file.fa> [--] [arguments...]`.
fn run(arguments: &[OsString]) -> i32 {
	let mut native = fa_codegen_llvm::Options::default();
	let mut arguments = arguments;
	let mut native_target = None;
	while let Some((first, rest)) = arguments.split_first() {
		let text = first.to_string_lossy();
		if native_option(&text, &mut native) {
			arguments = rest;
		} else if let Some(value) = text.strip_prefix("--target=") {
			if value != "bun" {
				match fa_codegen_llvm::Target::from_name(value) {
					Some(target) => native_target = Some(target),
					None => {
						eprintln!(
							"fa run: unknown target `{value}`: `bun`, `native`, `native-freestanding` or `wasm`"
						);
						return USAGE;
					}
				}
			}
			arguments = rest;
		} else {
			break;
		}
	}
	if let Some(target) = native_target {
		return run_native(arguments, target, &native);
	}
	let Some((file, rest)) = arguments.split_first() else {
		eprintln!("fa run: which file? ex: `fa run main.fa`");
		return USAGE;
	};
	let program_arguments = match rest.split_first() {
		Some((separator, rest)) if separator == "--" => rest,
		_ => rest,
	};
	let path = PathBuf::from(file);
	let mut loaded = match Loaded::new(&path) {
		Ok(loaded) => loaded,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	if loaded.has_errors() {
		eprint!("{}", loaded.rendered());
		return 1;
	}
	let mut options = Options::new(Target::Bun, loaded.shown.clone());
	options.overflow_checks = native.overflow_checks;
	let generated = match loaded.generate(&options) {
		Ok(generated) => generated,
		Err(rendered) => {
			eprint!("{rendered}");
			return 1;
		}
	};
	let Some(bun) = find_bun() else {
		eprintln!(
			"fa run: Bun is needed to run Fa programs, and it is neither on the PATH nor in ~/.bun/bin: install it from https://bun.sh"
		);
		return 1;
	};
	let script = match write_run_script(&path, &generated.code) {
		Ok(script) => script,
		Err(error) => {
			eprintln!("fa run: can't write the compiled program: {error}");
			return 1;
		}
	};
	let _ = std::io::stdout().flush();
	match Command::new(&bun)
		.arg(&script)
		.args(program_arguments)
		.status()
	{
		Ok(status) => exit_code(status),
		Err(error) => {
			eprintln!("fa run: can't start `{}`: {error}", bun.display());
			1
		}
	}
}

/// `fa run --target=native|wasm`: compiles into the cache and runs.
fn run_native(
	arguments: &[OsString],
	target: fa_codegen_llvm::Target,
	options: &fa_codegen_llvm::Options,
) -> i32 {
	let Some((file, rest)) = arguments.split_first() else {
		eprintln!("fa run: which file? ex: `fa run main.fa`");
		return USAGE;
	};
	let program_arguments = match rest.split_first() {
		Some((separator, rest)) if separator == "--" => rest,
		_ => rest,
	};
	let path = PathBuf::from(file);
	let source = fs::read(&path).unwrap_or_default();
	let directory = cache_directory().join(format!(
		"{}-{:016x}",
		target.name(),
		fnv1a(&[path.display().to_string().as_bytes(), &source].concat())
	));
	let artifact = match build_native(&path, target, options, &directory) {
		Ok(artifact) => artifact,
		Err(message) => {
			eprint!("{message}");
			return 1;
		}
	};
	let mut command = match &artifact.loader {
		Some(loader) => match fa_codegen_llvm::javascript_runtime() {
			Ok(runtime) => {
				let mut command = Command::new(runtime);
				command.arg(loader);
				command
			}
			Err(error) => {
				eprintln!("fa run: {error}");
				return 1;
			}
		},
		None => Command::new(&artifact.output),
	};
	let _ = std::io::stdout().flush();
	match command.args(program_arguments).status() {
		Ok(status) => exit_code(status),
		Err(error) => {
			eprintln!("fa run: can't start the program: {error}");
			1
		}
	}
}

/// The exit code of a finished process (128 + the signal when killed).
fn exit_code(status: std::process::ExitStatus) -> i32 {
	if let Some(code) = status.code() {
		return code;
	}
	#[cfg(unix)]
	{
		use std::os::unix::process::ExitStatusExt as _;
		if let Some(signal) = status.signal() {
			return 128 + signal;
		}
	}
	1
}

/// The cache directory of `fa run`: `$XDG_CACHE_HOME/fa/run`,
/// `~/.cache/fa/run`, or the temporary directory.
fn cache_directory() -> PathBuf {
	if let Some(cache) = std::env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
		return PathBuf::from(cache).join("fa").join("run");
	}
	if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
		return PathBuf::from(home).join(".cache").join("fa").join("run");
	}
	std::env::temp_dir().join("fa-run")
}

/// Writes the compiled program of a source file into the cache: one
/// directory per source path, so that concurrent runs of different files
/// don't collide (and a file is written to a new name, then renamed).
fn write_run_script(source: &Path, code: &str) -> std::io::Result<PathBuf> {
	let absolute = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
	let key = fnv1a(absolute.to_string_lossy().as_bytes());
	let directory = cache_directory().join(format!("{key:016x}"));
	fs::create_dir_all(&directory)?;
	let stem = source.file_stem().map_or("main".to_string(), |stem| {
		stem.to_string_lossy().to_string()
	});
	let script = directory.join(format!("{stem}.mjs"));
	let partial = directory.join(format!("{stem}.{}.tmp", std::process::id()));
	fs::write(&partial, code)?;
	fs::rename(&partial, &script)?;
	Ok(script)
}

/// The FNV-1a hash of bytes.
fn fnv1a(bytes: &[u8]) -> u64 {
	let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
	for &byte in bytes {
		hash ^= u64::from(byte);
		hash = hash.wrapping_mul(0x0100_0000_01b3);
	}
	hash
}

/// The `bun` executable: on the `PATH`, or in `~/.bun/bin`.
pub fn find_bun() -> Option<PathBuf> {
	let name = if cfg!(windows) { "bun.exe" } else { "bun" };
	if let Some(path) = std::env::var_os("PATH") {
		for directory in std::env::split_paths(&path) {
			let candidate = directory.join(name);
			if candidate.is_file() {
				return Some(candidate);
			}
		}
	}
	let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
	let candidate = PathBuf::from(home).join(".bun").join("bin").join(name);
	candidate.is_file().then_some(candidate)
}
