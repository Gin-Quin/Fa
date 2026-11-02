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

use fa_analyzer::Session;
use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::ids::FileId;
use fa_codegen_js::{GenerateError, JsOutput, Options, Target};

mod project;

/// The exit code of a usage error.
const USAGE: i32 = 2;

const HELP: &str = "\
fa: the Fa toolchain

Usage:
  fa check [<file.fa> | <package>]
  fa build [<file.fa> | <package>] [--target=bun|node|deno|web|native|native-freestanding|wasm]
           [--output=<directory>] [--declarations] [-O0|-O1|-O2|-O3] [--size]
           [--overflow-checks=true|false]
  fa run [--target=<target>] [--overflow-checks=true|false] <file.fa | package> [--] [arguments...]
  fa test [<package>]
  fa new package <name>

Commands:
  check   analyze a file (and the files next to it) or a package, print the diagnostics
  build   compile a file or a package (into build/<target>/ by default)
  run     compile a file, a script or a package's application for Bun, and run it
  test    run the tests of the package's `*.test.fa` files
  new     create a package

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
		Some("test") => test(&rest),
		Some("new") => new(&rest),
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

/// A program loaded and analyzed: a standalone file and the files next to
/// it, or a package (`project.rs`).
struct Loaded {
	inner: project::Loaded,
	/// The file the program runs (the standalone file, the script, the
	/// application's entry), or its first file.
	file: FileId,
	/// The path of `file` as shown in messages.
	shown: String,
}

/// The JavaScript of a program: one module, or one module per file.
enum JsBuild {
	/// A standalone file using no other file.
	Single(JsOutput),
	/// Several modules, `main.js` first.
	Program(fa_codegen_js::ProgramOutput),
}

impl Loaded {
	/// Reads and analyzes a file or a package folder.
	fn new(path: &Path) -> Result<Self, String> {
		Self::load(Some(path), "bun")
	}

	/// Reads and analyzes the project of a path (the current folder's when
	/// none), for a target.
	fn load(path: Option<&Path>, target: &str) -> Result<Self, String> {
		let project = project::locate(path)?;
		let inner = project::load(project, target)?;
		let file = inner
			.program
			.entry
			.or_else(|| inner.program.files.first().copied())
			.or_else(|| inner.program.tests.first().copied())
			.ok_or_else(|| "fa: the package has no file to analyze".to_string())?;
		let shown = inner.shown(file);
		Ok(Self { inner, file, shown })
	}

	/// Whether the analysis found errors.
	fn has_errors(&self) -> bool {
		self.inner.has_errors()
	}

	/// The diagnostics of the analysis, rendered.
	fn rendered(&self) -> String {
		self.inner.rendered()
	}

	/// The session.
	fn session(&self) -> &Session {
		&self.inner.session
	}

	/// Generates the JavaScript; on failure, the rendered errors.
	fn generate(&mut self, options: &Options) -> Result<JsBuild, String> {
		if self.inner.is_single_file() {
			return match fa_codegen_js::generate(&mut self.inner.session, self.file, options) {
				Ok(output) => Ok(JsBuild::Single(output)),
				Err(GenerateError { diagnostics }) => {
					let source = SourceView::new(&self.shown, self.inner.session.text(self.file));
					Err(render_all(&diagnostics, &source))
				}
			};
		}
		self.generate_program(options, &fa_codegen_js::EntryOptions::default())
	}

	/// Generates the modules of the program (its tests with `entry.tests`).
	fn generate_program(
		&mut self,
		options: &Options,
		entry: &fa_codegen_js::EntryOptions,
	) -> Result<JsBuild, String> {
		let mut options = options.clone();
		options.path = self.inner.project.shown.display().to_string();
		let program = self.inner.program.clone();
		match fa_codegen_js::generate_program(&mut self.inner.session, &program, &options, entry) {
			Ok(output) => Ok(JsBuild::Program(output)),
			Err(GenerateError { diagnostics }) => Err(self.inner.render_errors(&diagnostics)),
		}
	}
}

/// The optional path argument of a command: a `.fa` file or a package
/// folder (`None`: the package of the current folder).
fn optional_path(arguments: &[OsString], command: &str) -> Result<Option<PathBuf>, i32> {
	match arguments {
		[] => Ok(None),
		[path] => Ok(Some(PathBuf::from(path))),
		_ => {
			eprintln!(
				"fa {command}: unexpected argument `{}`",
				arguments[1].to_string_lossy()
			);
			Err(USAGE)
		}
	}
}

/// `fa test [<package>]` (advanced brief §18.2): runs the tests of the
/// `*.test.fa` files on Bun, each with freshly initialized module-level
/// values; exits with 0 when every test that ran passed, 1 otherwise.
fn test(arguments: &[OsString]) -> i32 {
	let mut paths = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if let Some(value) = text.strip_prefix("--target=") {
			if value != "bun" {
				eprintln!("fa test: only the `bun` target runs tests for now");
				return USAGE;
			}
		} else if text.starts_with("--") {
			eprintln!("fa test: unknown option `{text}`");
			return USAGE;
		} else {
			paths.push(argument.clone());
		}
	}
	let path = match optional_path(&paths, "test") {
		Ok(path) => path,
		Err(code) => return code,
	};
	let mut loaded = match Loaded::load(path.as_deref(), "bun") {
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
	let tests = loaded.inner.program.tests.clone();
	if tests.is_empty() {
		eprintln!("fa test: no test file (`*.test.fa`) in this package");
		return 0;
	}
	let focused = tests.iter().any(|&file| {
		let tree = loaded.session().tree(file);
		(0..tree.nodes.len()).any(|index| {
			matches!(
				fa_analyzer::syntax::view(tree, fa_analyzer::ids::NodeId::new(index)),
				fa_analyzer::syntax::View::Test {
					modifier: Some(fa_analyzer::syntax::TestModifier::Only),
					..
				}
			)
		})
	});
	if focused && let Some(reason) = project::focus_forbidden(&loaded.inner.project.base) {
		eprintln!(
			"error[TEST004]: a `test only` would hide the other tests, and {reason}: remove `only`"
		);
		return 1;
	}
	let options = Options::new(Target::Bun, loaded.shown.clone());
	let entry = fa_codegen_js::EntryOptions { tests: true };
	let generated = match loaded.generate_program(&options, &entry) {
		Ok(generated) => generated,
		Err(rendered) => {
			eprint!("{rendered}");
			return 1;
		}
	};
	let Some(bun) = find_bun() else {
		eprintln!(
			"error[CLI005]: Bun is needed to run Fa code on the bun target: install it from https://bun.sh"
		);
		return 1;
	};
	let base = loaded.inner.project.base.join("package.fa");
	let script = match write_run_script(&base, &generated) {
		Ok(script) => script,
		Err(error) => {
			eprintln!("fa test: can't write the compiled tests: {error}");
			return 1;
		}
	};
	let _ = std::io::stdout().flush();
	match Command::new(&bun).arg(&script).status() {
		Ok(status) => exit_code(status),
		Err(error) => {
			eprintln!("fa test: can't start `{}`: {error}", bun.display());
			1
		}
	}
}

/// `fa new package <name>` (decision D46).
fn new(arguments: &[OsString]) -> i32 {
	let texts: Vec<String> = arguments
		.iter()
		.map(|argument| argument.to_string_lossy().to_string())
		.collect();
	match texts.as_slice() {
		[kind, name] if kind == "package" => {
			let folder = PathBuf::from(name);
			match project::new_package(name, &folder) {
				Ok(()) => {
					eprintln!(
						"fa new: created the package `{name}`: `cd {name}`, then `fa run .` or `fa test`"
					);
					0
				}
				Err(message) => {
					eprintln!("{message}");
					1
				}
			}
		}
		[kind, ..] if kind == "workspace" => {
			eprintln!("fa new workspace: not available yet; create `workspace.fa` and `packages/`");
			1
		}
		_ => {
			eprintln!("fa new: which package? ex: `fa new package hello`");
			USAGE
		}
	}
}

/// `fa check [<file.fa> | <package>]`.
fn check(arguments: &[OsString]) -> i32 {
	let path = match optional_path(arguments, "check") {
		Ok(path) => path,
		Err(code) => return code,
	};
	let loaded = match Loaded::load(path.as_deref(), "bun") {
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
	let path = match optional_path(&files, "build") {
		Ok(path) => path.unwrap_or_else(|| PathBuf::from(".")),
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
	let output_given = output.is_some();
	let directory = output.unwrap_or_else(|| {
		let folder = if path.is_dir() {
			path.as_path()
		} else {
			path.parent().unwrap_or(Path::new("."))
		};
		folder.join("build").join(target.name())
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
	// a package builds into `<package>/<build.output>/<target>/` (D36)
	let directory = match (&output_given, &loaded.inner.configuration) {
		(false, Some(configuration)) if loaded.inner.project.package => loaded
			.inner
			.project
			.base
			.join(&configuration.build.output)
			.join(target.name()),
		_ => directory,
	};
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
/// module for Node) into a directory, or the modules of a program; returns
/// the path of `main.js`.
fn write_build(directory: &Path, target: Target, output: &JsBuild) -> std::io::Result<PathBuf> {
	fs::create_dir_all(directory)?;
	let main = directory.join("main.js");
	match output {
		JsBuild::Single(output) => {
			fs::write(&main, &output.code)?;
			if let Some(declarations) = &output.declarations {
				fs::write(directory.join("main.d.ts"), declarations)?;
			}
		}
		JsBuild::Program(output) => project::write_modules(directory, &output.modules)?,
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
	// TODO(LLVM): one LLVM module for the whole program: compile every file
	// of `loaded.inner.program.files`, the module-level values initialized in
	// `program.init_order` (advanced brief §15.7), the entry function called
	// with the arguments; until then, one standalone file
	if !loaded.inner.is_single_file() {
		return Err(format!(
			"fa: the `{}` target compiles one standalone file for now: programs of several files run on `bun` (`fa run` without `--target`)\n",
			target.name()
		));
	}
	fa_codegen_llvm::compile(
		loaded.session(),
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
	let script = match write_run_script(&path, &generated) {
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
fn write_run_script(source: &Path, code: &JsBuild) -> std::io::Result<PathBuf> {
	let absolute = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
	let key = fnv1a(absolute.to_string_lossy().as_bytes());
	let directory = cache_directory().join(format!("{key:016x}"));
	fs::create_dir_all(&directory)?;
	let code = match code {
		JsBuild::Single(output) => &output.code,
		JsBuild::Program(output) => {
			// one module per file, in a folder of its own (a new one per run, renamed into place)
			let modules = directory.join("program");
			let partial = directory.join(format!("program.{}.tmp", std::process::id()));
			let _ = fs::remove_dir_all(&partial);
			project::write_modules(&partial, &output.modules)?;
			let _ = fs::remove_dir_all(&modules);
			fs::rename(&partial, &modules)?;
			return Ok(modules.join("main.js"));
		}
	};
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
