//! The `fa` command (design/decisions.md, "Programs and the command line"):
//!
//! - `fa check <file.fa>`: analyzes a file and prints its diagnostics; exits
//!   with 1 when there are errors.
//! - `fa build <file.fa> [--target=<target>] [--output=<directory>]
//!   [--declarations]` (`build.rs`): compiles a file to `<directory>/main.js` (by default
//!   `build/<target>/` next to the file). The targets are `bun` (the
//!   default), `node` and `web`; `native` (an executable),
//!   `native-freestanding` (no C library, Linux x86-64) and `wasm` (a `.wasm`
//!   module and its `.mjs` loader) compile with the LLVM backend
//!   (`fa-codegen-llvm`, when it was built with LLVM): `-O0` to `-O3`
//!   (`-O3` by default), `--size`, `--emit-ir` (the optimized LLVM IR,
//!   `main.ll`), and `--overflow-checks=true|false`: a
//!   build doesn't check integer overflows by default (they wrap), `fa run`
//!   does. The JavaScript is minified with Bun (`bun build --minify`,
//!   imports kept external) unless `--minify=false` or the package's
//!   `build.minify` says otherwise; without Bun, it stays readable.
//! - `fa run [--target=<target>] <file.fa> [--] [arguments...]` (`run.rs`): compiles a
//!   file for Bun into a cache directory and runs it with Bun (found on
//!   `PATH` or in `~/.bun/bin`), forwarding the arguments and the exit code;
//!   with `--target=native`, `native-freestanding` or `wasm`, compiles it
//!   with the LLVM backend and runs the executable (Node or Bun for wasm).
//! - `fa format [<file.fa> | <folder>...] [--check]`: formats the source
//!   files (`format.rs`).
//! - `fa test [<package>] [--watch]` (`testing.rs`), `fa develop [<package>]`
//!   (`develop.rs`), `fa preview [<package>] [--target=…]` (`preview.rs`), `fa update
//!   [<dependency>...]` (`dependencies.rs`), `fa new package|workspace`,
//!   `fa help [<command>]` (`help.rs`).
//!
//! In a workspace (`workspace.rs`), `check`, `build`, `test`, `develop` and
//! `preview` take the name of a package; without one, they run for every
//! package that implements them (decision D44).
//! - `--timings` on `fa check` and `fa build`: the time of each phase and the
//!   sizes of the analysis, printed to stderr (`timings.rs`).
//! - `fa bind <header.h> [--output <file>]`: writes the foreign file of a C
//!   header (`bind.rs`).

use std::ffi::OsString;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use fa_analyzer::Session;
use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::ids::FileId;
use fa_codegen_js::{GenerateError, JsOutput, Options, Target};

mod bind;
mod build;
mod color;
mod dependencies;
mod develop;
mod format;
mod help;
pub mod native;
mod preview;
mod project;
mod report;
mod run;
mod sha256;
mod streaming;
mod testing;
mod timings;
mod workspace;

pub use project::editor_session;

use build::*;
use help::HELP;
use preview::*;
pub use run::find_bun;
use run::*;
use testing::*;
use workspace::{Implements, Selection};

/// The exit code of a usage error: `1`, like every other failure (the
/// commands exit with `0` or `1` only; design/decisions.md, "CLI").
pub(crate) const USAGE: i32 = 1;

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
		Some("bind") => bind::command(&rest),
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
	/// The C libraries of foreign code for `bun:ffi`:
	/// `build.link`, a name found in a folder of `build.libraryPaths` as
	/// the path of its shared library.
	fn c_libraries(&self) -> Vec<String> {
		c_libraries(self.inner.configuration.as_ref(), &self.inner.project.base)
	}

	pub(crate) fn has_errors(&self) -> bool {
		self.inner.has_errors()
	}

	/// Whether the program uses a foreign file (`*.foreign.fa`): it calls C.
	pub(crate) fn uses_foreign_code(&self) -> bool {
		self.inner
			.all_files()
			.iter()
			.any(|&file| self.session().module(file).ends_with(".foreign"))
	}

	/// Resolves the program's asynchronous mode (design/async.md A5–A7):
	/// `--asynchronous`, else `build.asynchronous`, else detected; `tests`
	/// for `fa test` (the test bodies count). Its diagnostics (`WCONC001`)
	/// join the file's.
	pub(crate) fn configure_asynchronous(&mut self, flag: Option<bool>, tests: bool) {
		let configured = flag.or_else(|| {
			self.inner
				.configuration
				.as_ref()
				.and_then(|configuration| configuration.build.asynchronous)
		});
		let mut program = self.inner.program.clone();
		self.inner
			.session
			.configure_asynchronous(&mut program, configured, tests);
		self.inner.program = program;
	}

	/// The diagnostics of the analysis.
	pub(crate) fn report(&self) -> report::Report {
		self.inner.report()
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

/// The C libraries of foreign code for `bun:ffi` of a package's
/// configuration: `build.link`, a name found in a folder of
/// `build.libraryPaths` (below `base`) as the path of its shared library.
pub(crate) fn c_libraries(
	configuration: Option<&fa_analyzer::configuration::PackageConfiguration>,
	base: &Path,
) -> Vec<String> {
	let Some(configuration) = configuration else {
		return Vec::new();
	};
	configuration
		.build
		.link
		.iter()
		.map(|library| {
			configuration
				.build
				.library_paths
				.iter()
				.flat_map(|folder| {
					["so", "dylib"].map(|extension| {
						base.join(folder).join(format!("lib{library}.{extension}"))
					})
				})
				.find(|path| path.exists())
				.map_or_else(|| library.clone(), |path| path.display().to_string())
		})
		.collect()
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
			crate::color::eprintln_message(&message);
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
					crate::color::eprintln_message(&message);
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
					crate::color::eprintln_message(&message);
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
					crate::color::eprintln_message(&message);
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

/// `fa check [<file.fa> | <package>] [--format=text|json]`.
fn check(arguments: &[OsString]) -> i32 {
	let mut format = report::Format::Text;
	let mut paths = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if let Some(value) = report::Format::option(&text) {
			let Ok(value) = value else {
				eprintln!("fa check: `{text}`: write `--format=text` or `--format=json`");
				return USAGE;
			};
			format = value;
		} else if text.starts_with("--") {
			eprintln!("fa check: unknown option `{text}`");
			return USAGE;
		} else {
			paths.push(argument.clone());
		}
	}
	let path = match optional_path(&paths, "check") {
		Ok(path) => path,
		Err(code) => return code,
	};
	on_selection(path.as_deref(), Implements::All, "check", |path| {
		check_one(path, format)
	})
}

/// `fa check` of one file or package.
fn check_one(path: Option<&Path>, format: report::Format) -> i32 {
	// a package: on the parallel and streaming pipeline (`streaming.rs`)
	if pipeline_enabled()
		&& let Some(project) = streaming::locate(path)
	{
		return streaming::check(project, format);
	}
	let loaded = match Loaded::load(path, "bun") {
		Ok(loaded) => loaded,
		Err(message) => {
			crate::color::eprintln_message(&message);
			return 1;
		}
	};
	let mut report = timings::phase("render", || loaded.report());
	let mut errors = loaded.has_errors();
	// code that reads `target` or `configuration` is checked for every
	// target of the configuration (D22); the rest is the same everywhere
	let depends = loaded
		.inner
		.all_files()
		.iter()
		.any(|&file| loaded.session().depends_on_target(file));
	if depends {
		// a standalone program binding C (a foreign file) can't run on the web
		let targets = loaded.inner.configuration.as_ref().map_or_else(
			|| default_targets(loaded.uses_foreign_code()),
			|configuration| configuration.targets.clone(),
		);
		for target in targets.iter().filter(|target| *target != "bun") {
			let Ok(other) = Loaded::load(path, target) else {
				continue;
			};
			errors |= other.has_errors();
			report.merge_target(other.report(), target);
		}
	}
	report.print(format);
	i32::from(errors)
}

/// Whether `fa check` and `fa build` of a package run on the parallel
/// pipeline (`FA_PIPELINE=0` keeps the sequential analysis, to compare).
fn pipeline_enabled() -> bool {
	std::env::var("FA_PIPELINE").map_or(true, |value| value != "0")
}

/// `--asynchronous=true|false` (design/async.md A5): `Some(Ok(mode))`
/// for it, `Some(Err(()))` for a bad value, `None` for another argument.
fn asynchronous_option(text: &str) -> Option<Result<bool, ()>> {
	let value = text.strip_prefix("--asynchronous")?;
	Some(match value {
		"=true" => Ok(true),
		"=false" => Ok(false),
		_ => Err(()),
	})
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
	let found = fa_codebase_parser::discover(&current);
	let scope = match (found.package, found.workspace) {
		(Some(package), _) => project::scope_of(&package),
		(None, Some(root)) => project::scope_of_workspace(&root),
		(None, None) => Err(
			"error[CLI001]: no `package.fa` or `workspace.fa` found in this folder or its parents"
				.to_string(),
		),
	};
	let scope = match scope {
		Ok(scope) => scope,
		Err(message) => {
			crate::color::eprintln_message(&message);
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
				color::eprintln_message(warning);
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
			crate::color::eprintln_message(&message);
			1
		}
	}
}
