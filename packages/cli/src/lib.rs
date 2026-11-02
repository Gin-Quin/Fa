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
//!   (`-O3` by default), `--size`, `--emit-ir` (the optimized LLVM IR,
//!   `main.ll`), and `--overflow-checks=true|false`: a
//!   build doesn't check integer overflows by default (they wrap), `fa run`
//!   does. The JavaScript is minified with Bun (`bun build --minify`,
//!   imports kept external) unless `--minify=false` or the package's
//!   `build.minify` says otherwise; without Bun, it stays readable.
//! - `fa run [--target=<target>] <file.fa> [--] [arguments...]`: compiles a
//!   file for Bun into a cache directory and runs it with Bun (found on
//!   `PATH` or in `~/.bun/bin`), forwarding the arguments and the exit code;
//!   with `--target=native`, `native-freestanding` or `wasm`, compiles it
//!   with the LLVM backend and runs the executable (Node or Bun for wasm).
//! - `fa format [<file.fa> | <folder>...] [--check]`: formats the source
//!   files (`format.rs`).
//! - `fa test [<package>] [--watch]`, `fa develop [<package>]`
//!   (`develop.rs`), `fa preview [<package>] [--target=…]`, `fa update
//!   [<dependency>...]` (`dependencies.rs`), `fa new package|workspace`,
//!   `fa help [<command>]` (`help.rs`).
//!
//! In a workspace (`workspace.rs`), `check`, `build`, `test`, `develop` and
//! `preview` take the name of a package; without one, they run for every
//! package that implements them (decision D44).
//! - `--timings` on `fa check` and `fa build`: the time of each phase and the
//!   sizes of the analysis, printed to stderr (`timings.rs`).

use std::ffi::OsString;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use fa_analyzer::Session;
use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::ids::FileId;
use fa_codegen_js::{GenerateError, JsOutput, Options, Target};

mod dependencies;
mod develop;
mod format;
mod help;
mod project;
mod sha256;
mod timings;
mod workspace;

use help::HELP;
use workspace::{Implements, Selection};

/// The exit code of a usage error.
pub(crate) const USAGE: i32 = 2;

/// Runs the command line `arguments` (without the program name) and returns
/// the exit code.
pub fn main(arguments: Vec<OsString>) -> i32 {
	let mut arguments = arguments.into_iter();
	let Some(command) = arguments.next() else {
		eprint!("{HELP}");
		return USAGE;
	};
	let mut rest: Vec<OsString> = arguments.collect();
	// `--timings`: the phases and sizes of `check` and `build`, on stderr
	if matches!(command.to_str(), Some("check" | "build"))
		&& let Some(position) = rest.iter().position(|argument| argument == "--timings")
	{
		rest.remove(position);
		timings::enable();
	}
	match command.to_str() {
		Some("check") => {
			let code = check(&rest);
			timings::print("check");
			code
		}
		Some("build") => {
			let code = build(&rest);
			timings::print("build");
			code
		}
		Some("run") => run(&rest),
		Some("test") => test(&rest),
		Some("format") => format::format(&rest),
		Some("new") => new(&rest),
		Some("develop") => develop::develop(&rest),
		Some("preview") => preview(&rest),
		Some("update") => update(&rest),
		Some("help" | "--help" | "-h") => match rest.first().map(|name| name.to_string_lossy()) {
			None => {
				print!("{HELP}");
				0
			}
			Some(name) => match help::command(&name) {
				Some(text) => {
					print!("{text}");
					0
				}
				None => {
					eprintln!("fa help: unknown command `{name}`\n\n{HELP}");
					USAGE
				}
			},
		},
		Some("version" | "--version" | "-V") => {
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
pub(crate) struct Loaded {
	pub(crate) inner: project::Loaded,
	/// The file the program runs (the standalone file, the script, the
	/// application's entry), or its first file.
	pub(crate) file: FileId,
	/// The path of `file` as shown in messages.
	pub(crate) shown: String,
}

/// The JavaScript of a program: one module, or one module per file.
pub(crate) enum JsBuild {
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
	pub(crate) fn load(path: Option<&Path>, target: &str) -> Result<Self, String> {
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
	pub(crate) fn has_errors(&self) -> bool {
		self.inner.has_errors()
	}

	/// The diagnostics of the analysis, rendered.
	pub(crate) fn rendered(&self) -> String {
		self.inner.rendered()
	}

	/// The session.
	fn session(&self) -> &Session {
		&self.inner.session
	}

	/// Generates the JavaScript; on failure, the rendered errors.
	pub(crate) fn generate(&mut self, options: &Options) -> Result<JsBuild, String> {
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

/// Runs a command on what it selects (decision D44): one file or package,
/// or each package of a workspace that implements the command; returns the
/// first failing exit code.
fn on_selection(
	argument: Option<&Path>,
	implements: Implements,
	command: &str,
	mut run: impl FnMut(Option<&Path>) -> i32,
) -> i32 {
	let selection = match workspace::select(argument, implements) {
		Ok(selection) => selection,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	match selection {
		Selection::One(path) => run(path.as_deref()),
		Selection::Packages(packages) => {
			if packages.is_empty() {
				let why = match implements {
					Implements::All => {
						"the workspace has no package yet: create one in `packages/` with `fa new package <name>`"
					}
					Implements::Application => {
						"no package of the workspace has an application (`application/main.fa`)"
					}
					Implements::Tests => "no package of the workspace has test files (`*.test.fa`)",
				};
				eprintln!("fa {command}: {why}");
				return 0;
			}
			let mut code = 0;
			for (name, directory) in &packages {
				if packages.len() > 1 {
					eprintln!("fa {command}: package `{name}` ({})", directory.display());
				}
				let result = run(Some(directory));
				if code == 0 {
					code = result;
				}
			}
			code
		}
	}
}

/// `fa test [<package>] [--watch]` (advanced brief §18.2): runs the tests
/// of the `*.test.fa` files on Bun, each with freshly initialized
/// module-level values; exits with 0 when every test that ran passed, 1
/// otherwise.
fn test(arguments: &[OsString]) -> i32 {
	let mut paths = Vec::new();
	let mut watch = false;
	for argument in arguments {
		let text = argument.to_string_lossy();
		if let Some(value) = text.strip_prefix("--target=") {
			if value != "bun" {
				eprintln!("fa test: only the `bun` target runs tests for now");
				return USAGE;
			}
		} else if text == "--watch" {
			watch = true;
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
	if watch {
		return match workspace::select(path.as_deref(), Implements::Tests) {
			Ok(selection) => develop::watch_tests(selection),
			Err(message) => {
				eprintln!("{message}");
				1
			}
		};
	}
	on_selection(
		path.as_deref(),
		Implements::Tests,
		"test",
		|path| match Loaded::load(path, "bun") {
			Ok(mut loaded) => run_tests(&mut loaded),
			Err(message) => {
				eprintln!("{message}");
				1
			}
		},
	)
}

/// Runs the tests of a loaded package.
pub(crate) fn run_tests(loaded: &mut Loaded) -> i32 {
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
		[kind] if kind == "workspace" => {
			let name = std::env::current_dir()
				.ok()
				.and_then(|directory| {
					directory
						.file_name()
						.map(|name| name.to_string_lossy().to_string())
				})
				.unwrap_or_else(|| "workspace".to_string());
			match workspace::new_workspace(Path::new("."), &name) {
				Ok(()) => {
					eprintln!(
						"fa new: created the workspace `{name}`: create its packages in `packages/` with `fa new package <name>`"
					);
					0
				}
				Err(message) => {
					eprintln!("{message}");
					1
				}
			}
		}
		[kind, name] if kind == "workspace" => {
			let folder = PathBuf::from(name);
			if folder.exists()
				&& fs::read_dir(&folder)
					.map(|mut entries| entries.next().is_some())
					.unwrap_or(true)
			{
				eprintln!(
					"error[CLI007]: `{name}` already exists and isn't empty: choose another name, or empty it"
				);
				return 1;
			}
			let shown = folder
				.file_name()
				.map_or(name.clone(), |shown| shown.to_string_lossy().to_string());
			match workspace::new_workspace(&folder, &shown) {
				Ok(()) => {
					eprintln!(
						"fa new: created the workspace `{shown}`: `cd {name}/packages`, then `fa new package <name>`"
					);
					0
				}
				Err(message) => {
					eprintln!("{message}");
					1
				}
			}
		}
		_ => {
			eprintln!("fa new: what to create? ex: `fa new package hello`, `fa new workspace`");
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
	on_selection(path.as_deref(), Implements::All, "check", check_one)
}

/// `fa check` of one file or package.
fn check_one(path: Option<&Path>) -> i32 {
	let loaded = match Loaded::load(path, "bun") {
		Ok(loaded) => loaded,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	let mut rendered = timings::phase("render", || loaded.rendered());
	let mut errors = loaded.has_errors();
	// code that reads `target` or `configuration` is checked for every
	// target of the configuration (D22); the rest is the same everywhere
	let depends = loaded
		.inner
		.all_files()
		.iter()
		.any(|&file| loaded.session().depends_on_target(file));
	if depends {
		let targets = loaded.inner.configuration.as_ref().map_or_else(
			|| vec!["web".to_string(), "bun".to_string(), "native".to_string()],
			|configuration| configuration.targets.clone(),
		);
		for target in targets.iter().filter(|target| *target != "bun") {
			let Ok(other) = Loaded::load(path, target) else {
				continue;
			};
			errors |= other.has_errors();
			for block in other.rendered().split("\n\n") {
				if !block.trim().is_empty()
					&& !rendered.contains(block.trim())
					&& !block.trim_start().starts_with(|c: char| c.is_ascii_digit())
				{
					rendered.push_str(&format!(
						"{}\n  (for the target `{target}`)\n\n",
						block.trim_end()
					));
				}
			}
		}
	}
	eprint!("{rendered}");
	i32::from(errors)
}

/// The options of `fa build`.
#[derive(Clone)]
struct BuildOptions {
	/// The file or the package to build.
	path: PathBuf,
	/// Whether the path was given (else: the current folder's package, or
	/// the packages of its workspace).
	given: bool,
	/// The target, as named on the command line.
	target: String,
	/// `--output=<directory>`.
	output: Option<PathBuf>,
	/// `--declarations`: the TypeScript declarations of the exports.
	declarations: bool,
	/// `--minify=true|false`; `None`: the package's `build.minify` (`true`
	/// by default, also for a standalone file).
	minify: Option<bool>,
	/// The options of the LLVM backend, and `--overflow-checks` for every
	/// target: a release build doesn't check overflows (they wrap, like
	/// Rust's; design/decisions.md, "Overflow checks").
	native: fa_codegen_llvm::Options,
}

/// Reads the command line of `fa build`; on a usage error, its exit code.
fn build_options(arguments: &[OsString]) -> Result<BuildOptions, i32> {
	let mut options = BuildOptions {
		path: PathBuf::from("."),
		given: false,
		target: "bun".to_string(),
		output: None,
		declarations: false,
		minify: None,
		native: fa_codegen_llvm::Options {
			overflow_checks: false,
			..fa_codegen_llvm::Options::default()
		},
	};
	let mut files = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if native_option(&text, &mut options.native) {
			continue;
		}
		if let Some(value) = text.strip_prefix("--target=") {
			options.target = value.to_string();
		} else if let Some(value) = text.strip_prefix("--output=") {
			options.output = Some(PathBuf::from(value));
		} else if text == "--declarations" {
			options.declarations = true;
		} else if let Some(value) = text.strip_prefix("--minify") {
			options.minify = match value {
				"=true" => Some(true),
				"=false" => Some(false),
				_ => {
					eprintln!("fa build: `{text}`: write `--minify=true` or `--minify=false`");
					return Err(USAGE);
				}
			};
		} else if text.starts_with("--") {
			eprintln!("fa build: unknown option `{text}`");
			return Err(USAGE);
		} else {
			files.push(argument.clone());
		}
	}
	if let Some(path) = optional_path(&files, "build")? {
		options.path = path;
		options.given = true;
	}
	Ok(options)
}

/// `fa build [<file.fa> | <package>] [--target=…] [--output=…]
/// [--declarations] [--minify=true|false] [native options]`.
fn build(arguments: &[OsString]) -> i32 {
	let options = match build_options(arguments) {
		Ok(options) => options,
		Err(code) => return code,
	};
	let argument = options.given.then(|| options.path.clone());
	on_selection(
		argument.as_deref(),
		Implements::Application,
		"build",
		|path| {
			let mut options = options.clone();
			options.path = path.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
			build_one(&options)
		},
	)
}

/// `fa build` of one file or package.
fn build_one(options: &BuildOptions) -> i32 {
	if let Some(native_target) = fa_codegen_llvm::Target::from_name(&options.target) {
		return build_native_target(options, native_target);
	}
	let Some(target) = Target::from_name(&options.target) else {
		eprintln!(
			"fa build: unknown target `{}`: the targets are `bun`, `node`, `deno`, `web`, `native` and `wasm`",
			options.target
		);
		return USAGE;
	};
	build_javascript(options, target)
}

/// `fa build` for `native`, `native-freestanding` or `wasm`.
fn build_native_target(options: &BuildOptions, target: fa_codegen_llvm::Target) -> i32 {
	let directory = options.output.clone().unwrap_or_else(|| {
		options
			.path
			.parent()
			.unwrap_or(Path::new("."))
			.join("build")
			.join(target.name())
	});
	match build_native(&options.path, target, &options.native, &directory) {
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
	}
}

/// `fa build` for a JavaScript target: `main.js` (and the other modules of
/// a program), minified unless `build.minify` or `--minify=false` says
/// otherwise (the generated code is readable before minification).
fn build_javascript(options: &BuildOptions, target: Target) -> i32 {
	let path = &options.path;
	let directory = options.output.clone().unwrap_or_else(|| {
		let folder = if path.is_dir() {
			path.as_path()
		} else {
			path.parent().unwrap_or(Path::new("."))
		};
		folder.join("build").join(target.name())
	});
	let mut loaded = match Loaded::load(Some(path), target.name()) {
		Ok(loaded) => loaded,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	eprint!("{}", timings::phase("render", || loaded.rendered()));
	if loaded.has_errors() {
		return 1;
	}
	let mut js_options = Options::new(target, loaded.shown.clone());
	js_options.declarations = options.declarations;
	js_options.overflow_checks = options.native.overflow_checks;
	let configuration = loaded
		.inner
		.configuration
		.as_ref()
		.filter(|_| loaded.inner.project.package);
	// a package builds into `<package>/<build.output>/<target>/` (D36)
	let directory = match (&options.output, configuration) {
		(None, Some(configuration)) => loaded
			.inner
			.project
			.base
			.join(&configuration.build.output)
			.join(target.name()),
		_ => directory,
	};
	let minify = options
		.minify
		.unwrap_or_else(|| configuration.is_none_or(|configuration| configuration.build.minify));
	let generated = match timings::phase("generation", || loaded.generate(&js_options)) {
		Ok(generated) => generated,
		Err(rendered) => {
			eprint!("{rendered}");
			return 1;
		}
	};
	let main = match timings::phase("write", || write_build(&directory, target, &generated)) {
		Ok(main) => main,
		Err(error) => {
			eprintln!(
				"fa build: can't write into `{}`: {error}",
				directory.display()
			);
			return 1;
		}
	};
	if minify {
		let modules: Vec<PathBuf> = match &generated {
			JsBuild::Single(_) => vec![main.clone()],
			JsBuild::Program(output) => output
				.modules
				.iter()
				.map(|(module, _)| directory.join(module))
				.collect(),
		};
		for module in &modules {
			match timings::phase("minify", || minify_with_bun(module, target)) {
				Ok(()) => {}
				// the default without Bun: the readable code stays
				Err(MinifyError::NoBun) if options.minify.is_none() => {
					eprintln!(
						"fa build: Bun isn't installed: the JavaScript is not minified (`--minify=false` says so explicitly)"
					);
					break;
				}
				Err(error) => {
					eprintln!("fa build: {error}");
					return 1;
				}
			}
		}
	}
	eprintln!("fa build: wrote {}", main.display());
	0
}

/// Why a module couldn't be minified.
enum MinifyError {
	/// Bun is neither on the `PATH` nor in `~/.bun/bin`.
	NoBun,
	/// Bun failed: the message.
	Failed(String),
}

impl std::fmt::Display for MinifyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MinifyError::NoBun => write!(
				f,
				"--minify=true needs Bun, which is neither on the PATH nor in ~/.bun/bin: install it from https://bun.sh, or build with --minify=false"
			),
			MinifyError::Failed(message) => f.write_str(message),
		}
	}
}

/// Minifies a generated module in place with `bun build --minify`: the
/// identifiers, whitespace and syntax, its imports kept external and its
/// exports kept (`build.minify`, documentation/architecture/configuration.md).
fn minify_with_bun(main: &Path, target: Target) -> Result<(), MinifyError> {
	let Some(bun) = find_bun() else {
		return Err(MinifyError::NoBun);
	};
	let minified = main.with_extension("min.js");
	let bun_target = match target {
		Target::Bun => "bun",
		Target::Node | Target::Deno => "node",
		Target::Web => "browser",
	};
	let result = Command::new(&bun)
		.arg("build")
		.arg(main)
		.args(["--minify", "--format=esm", "--external", "*"])
		.arg(format!("--target={bun_target}"))
		.arg("--outfile")
		.arg(&minified)
		.output()
		.map_err(|error| {
			MinifyError::Failed(format!("can't start `{}`: {error}", bun.display()))
		})?;
	if !result.status.success() {
		let _ = fs::remove_file(&minified);
		return Err(MinifyError::Failed(format!(
			"`bun build --minify` failed: {}",
			String::from_utf8_lossy(&result.stderr).trim_end()
		)));
	}
	fs::rename(&minified, main)
		.map_err(|error| MinifyError::Failed(format!("can't write `{}`: {error}", main.display())))
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

/// The options of the LLVM backend: `-O0` to `-O3`, `--size`, `--emit-ir`
/// (the optimized IR next to the output, `main.ll`),
/// `--overflow-checks=true|false`.
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
		"--emit-ir" => {
			options.emit_ir = true;
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
	let loaded = Loaded::load(Some(path), target.language_target())?;
	let rendered = loaded.rendered();
	if loaded.has_errors() {
		return Err(rendered);
	}
	eprint!("{rendered}");
	// a program of several files: one LLVM module (advanced brief §15.7)
	if !loaded.inner.is_single_file() {
		return fa_codegen_llvm::compile_program(
			loaded.session(),
			&loaded.inner.program,
			target,
			options,
			&directory.join("main"),
		)
		.map_err(|error| format!("{error}\n"));
	}
	// generation, optimization and link: one call for now
	timings::phase("llvm", || {
		fa_codegen_llvm::compile(
			loaded.session(),
			loaded.file,
			&loaded.shown,
			target,
			options,
			&directory.join("main"),
		)
	})
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
	let mut path = PathBuf::from(file);
	// the name of a package of the workspace
	if !path.exists() && path.extension().is_none_or(|extension| extension != "fa") {
		match workspace::select(Some(&path), Implements::Application) {
			Ok(Selection::One(Some(directory))) => path = directory,
			Ok(_) => {}
			Err(message) => {
				eprintln!("{message}");
				return 1;
			}
		}
	}
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
pub(crate) fn exit_code(status: std::process::ExitStatus) -> i32 {
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
pub(crate) fn write_run_script(source: &Path, code: &JsBuild) -> std::io::Result<PathBuf> {
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

/// `fa preview [<package>] [--target=<target>]` (advanced brief §20): runs
/// the output of `fa build`, for the target named, else for the first
/// configured target that runs here (`native`, then `bun`, `node`, `deno`;
/// `web` is served on `develop.port`).
fn preview(arguments: &[OsString]) -> i32 {
	let mut target = None;
	let mut paths = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if let Some(value) = text.strip_prefix("--target=") {
			target = Some(value.to_string());
		} else if text.starts_with("--") {
			eprintln!("fa preview: unknown option `{text}`");
			return USAGE;
		} else {
			paths.push(argument.clone());
		}
	}
	let path = match optional_path(&paths, "preview") {
		Ok(path) => path,
		Err(code) => return code,
	};
	on_selection(
		path.as_deref(),
		Implements::Application,
		"preview",
		|path| preview_one(path, target.as_deref()),
	)
}

/// The runtime that runs the output of a target here, if any.
fn preview_command(target: &str, directory: &Path, port: u16) -> Option<Command> {
	let on_path = |name: &str| -> Option<PathBuf> {
		let path = std::env::var_os("PATH")?;
		std::env::split_paths(&path)
			.map(|folder| folder.join(name))
			.find(|candidate| candidate.is_file())
	};
	match target {
		"native" => {
			let program = directory.join("main");
			program.is_file().then(|| Command::new(program))
		}
		"wasm" => {
			let loader = directory.join("main.mjs");
			let runtime = fa_codegen_llvm::javascript_runtime().ok()?;
			loader.is_file().then(|| {
				let mut command = Command::new(runtime);
				command.arg(loader);
				command
			})
		}
		"bun" | "node" | "deno" => {
			let main = directory.join("main.js");
			if !main.is_file() {
				return None;
			}
			let runtime = match target {
				"bun" => find_bun()?,
				other => on_path(other)?,
			};
			let mut command = Command::new(runtime);
			if target == "deno" {
				command.args(["run", "--allow-all"]);
			}
			command.arg(main);
			Some(command)
		}
		"web" => {
			let main = directory.join("main.js");
			if !main.is_file() {
				return None;
			}
			let bun = find_bun()?;
			let server = format!(
				"const root = {root:?};\nconst page = '<!doctype html><meta charset=\"utf-8\"><script type=\"module\" src=\"/main.js\"></script>';\nBun.serve({{ port: {port}, async fetch(request) {{\n\tconst path = decodeURIComponent(new URL(request.url).pathname);\n\tif (path === '/' || path === '/index.html') {{\n\t\tconst index = Bun.file(root + '/index.html');\n\t\treturn (await index.exists()) ? new Response(index) : new Response(page, {{ headers: {{ 'content-type': 'text/html' }} }});\n\t}}\n\tif (path.includes('..')) return new Response('not found', {{ status: 404 }});\n\tconst file = Bun.file(root + path);\n\treturn (await file.exists()) ? new Response(file) : new Response('not found', {{ status: 404 }});\n}} }});\nconsole.error('fa preview: serving ' + root + ' on http://localhost:{port}');\n",
				root = directory.display().to_string()
			);
			let script = cache_directory().join(format!("preview-{port}.mjs"));
			fs::create_dir_all(script.parent()?).ok()?;
			fs::write(&script, server).ok()?;
			let mut command = Command::new(bun);
			command.arg(script);
			Some(command)
		}
		_ => None,
	}
}

/// `fa preview` of one package.
fn preview_one(path: Option<&Path>, target: Option<&str>) -> i32 {
	let project = match project::locate(path) {
		Ok(project) => project,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	if !project.package || project.entry.is_some() {
		eprintln!(
			"fa preview: previews the application of a package; run a file with `fa run <file.fa>`"
		);
		return USAGE;
	}
	let written = workspace::package_configuration(&project.base);
	let workspace_written = project::workspace_root(&project.base)
		.and_then(|root| workspace::workspace_configuration(&root));
	let configuration_for = |target: &str| {
		let merged = written.as_ref().map(|written| {
			fa_analyzer::configuration::merge(workspace_written.as_ref(), written, target)
		});
		fa_analyzer::configuration::PackageConfiguration::from_value(
			&merged.unwrap_or(fa_analyzer::configuration::ConfigValue::Object(Vec::new())),
		)
	};
	let candidates: Vec<String> = match target {
		Some(target) => vec![target.to_string()],
		None => {
			let configured = configuration_for("bun").targets;
			["native", "bun", "node", "deno", "wasm", "web"]
				.iter()
				.filter(|target| configured.iter().any(|other| other == *target))
				.map(|target| target.to_string())
				.collect()
		}
	};
	let mut looked = Vec::new();
	for target in &candidates {
		let configuration = configuration_for(target);
		let directory = project.base.join(&configuration.build.output).join(target);
		looked.push(dependencies::shown_path(&directory));
		let Some(mut command) = preview_command(target, &directory, configuration.develop.port)
		else {
			continue;
		};
		eprintln!("fa preview: running the `{target}` build");
		let _ = std::io::stdout().flush();
		return match command.status() {
			Ok(status) => exit_code(status),
			Err(error) => {
				eprintln!("fa preview: can't start the program: {error}");
				1
			}
		};
	}
	eprintln!(
		"error[CLI006]: nothing to preview: no build that runs here in {}\n  help: build it first with `fa build{}`",
		looked
			.iter()
			.map(|path| format!("`{path}`"))
			.collect::<Vec<_>>()
			.join(", "),
		target.map_or(String::new(), |target| format!(" --target={target}"))
	);
	1
}

/// `fa update [<dependency>...]` (architecture/dependencies.md): resolves
/// the dependencies again, all of them or the ones named, and rewrites the
/// lockfile of the package or the workspace.
fn update(arguments: &[OsString]) -> i32 {
	let mut keys = std::collections::BTreeSet::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if text.starts_with("--") {
			eprintln!("fa update: unknown option `{text}`");
			return USAGE;
		}
		keys.insert(text.to_string());
	}
	let current = std::env::current_dir()
		.ok()
		.and_then(|directory| fs::canonicalize(directory).ok())
		.unwrap_or_else(|| PathBuf::from("."));
	let package = current
		.ancestors()
		.find(|directory| directory.join("package.fa").is_file());
	let scope = match (package, fa_codebase_parser::find_workspace(&current)) {
		(Some(package), _) => project::scope_of(package),
		(None, Some(root)) => project::scope_of_workspace(&root),
		(None, None) => Err(
			"error[CLI001]: no `package.fa` or `workspace.fa` found in this folder or its parents"
				.to_string(),
		),
	};
	let scope = match scope {
		Ok(scope) => scope,
		Err(message) => {
			eprintln!("{message}");
			return 1;
		}
	};
	let mode = if keys.is_empty() {
		dependencies::Update::All
	} else {
		dependencies::Update::Keys(keys)
	};
	match dependencies::resolve(&scope, &mode, true) {
		Ok(resolution) => {
			for warning in &resolution.warnings {
				eprintln!("{warning}");
			}
			if resolution.changes.is_empty() {
				eprintln!("fa update: the dependencies are up to date");
			}
			for (source, before, after) in &resolution.changes {
				match (before, after) {
					(Some(before), Some(after)) => {
						eprintln!("fa update: {source} {before} -> {after}");
					}
					(None, Some(after)) => eprintln!("fa update: {source} {after} (new)"),
					(Some(before), None) => eprintln!("fa update: {source} {before} (removed)"),
					(None, None) => {}
				}
			}
			0
		}
		Err(message) => {
			eprintln!("{message}");
			1
		}
	}
}
