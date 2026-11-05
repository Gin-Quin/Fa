//! `fa check` and `fa build` of a package on the parallel and streaming
//! pipeline (`fa_analyzer::pipeline`, design/parallel-pipeline.md): the
//! units of the module graph are analyzed on every core, each module's
//! JavaScript is generated and written by the thread that analyzed it, and
//! a unit's trees, analyses and types are dropped once it is done. What is
//! kept to the end: the interfaces of the modules, and their diagnostics.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use fa_analyzer::ids::FileId;
use fa_analyzer::pipeline::{self, Report, UnitSink};
use fa_analyzer::{Diagnostic, Session, Severity};
use fa_codegen_js::{EntryInputs, EntryModule, EntryOptions, ModuleSink, Options, Target};

use crate::project::{self, Prepared, Project};
use crate::report as diagnostics;
use crate::timings;

/// The options of a run: `FA_SCHEDULE_SEED` shuffles the order of ready
/// units (the determinism tests).
fn run_options(entry: &str) -> pipeline::Options {
	pipeline::Options {
		entry: Some(entry.to_string()),
		seed: std::env::var("FA_SCHEDULE_SEED")
			.ok()
			.and_then(|seed| seed.parse().ok())
			.unwrap_or(0),
		measure: timings::enabled(),
		..pipeline::Options::default()
	}
}

/// How a module's path is shown (as `Loaded::shown` does).
fn shown(session: &Session, project: &Project, module: &str) -> String {
	let relative = session
		.resolver()
		.map_or_else(|| format!("{module}.fa"), |resolver| resolver.path(module));
	if project.shown.as_os_str().is_empty()
		|| module.starts_with(fa_analyzer::mounts::DEPENDENCY_PREFIX)
	{
		relative
	} else {
		project.shown.join(relative).display().to_string()
	}
}

/// The diagnostics of a run and of the configuration files, in the order
/// of the module paths.
fn render(
	prepared: &Prepared,
	report: &Report,
	extra: &[(String, Vec<Diagnostic>)],
) -> diagnostics::Report {
	let session = &prepared.session;
	// (module, path, text, diagnostics)
	let mut files: Vec<(String, String, String, Vec<Diagnostic>)> = Vec::new();
	// the configuration files, analyzed while preparing
	for file in (0..session.file_count()).map(FileId::new) {
		let module = session.module(file);
		if session.loaded_module(module) != Some(file) || session.diagnostics(file).is_empty() {
			continue;
		}
		files.push((
			module.to_string(),
			shown(session, &prepared.project, module),
			session.text(file).to_string(),
			session.diagnostics(file).to_vec(),
		));
	}
	for module in &report.modules {
		let mut diagnostics = module.diagnostics.clone();
		if let Some((_, more)) = extra.iter().find(|(other, _)| *other == module.module) {
			diagnostics.extend(more.iter().cloned());
			fa_analyzer::diagnostics::sort(&mut diagnostics);
		}
		if diagnostics.is_empty() {
			continue;
		}
		// the text is read again
		let text = session
			.resolver()
			.and_then(|resolver| resolver.source(&module.module))
			.unwrap_or_default();
		files.push((
			module.module.clone(),
			shown(session, &prepared.project, &module.module),
			text.to_string(),
			diagnostics,
		));
	}
	files.sort_by(|a, b| a.0.cmp(&b.0));
	let mut out = diagnostics::Report::default();
	for warning in &prepared.warnings {
		out.warning(warning);
	}
	for (_, path, text, diagnostics) in &files {
		out.file(path, text, diagnostics);
	}
	out
}

/// Prepares a package for a target; `types_of`: the session whose global
/// type store it shares (another target's run).
fn prepare(
	project: &Project,
	target: &str,
	types_of: Option<&Session>,
) -> Result<Prepared, String> {
	let mut session = match types_of {
		Some(other) => Session::sharing_types_of(other),
		None => Session::new(),
	};
	session.set_target(target);
	project::prepare_package(project.clone(), target, session)
}

/// Whether a project is checked and built on the pipeline: a package (not
/// a script of one, not a standalone file).
pub(crate) fn applies(project: &Project) -> bool {
	project.package && project.entry.is_none()
}

/// Records the sizes of a run for `--timings`.
fn record(report: &Report) {
	timings::record_pipeline(&report.stats, report.modules.len());
}

/// `fa check` of a package: every module analyzed for the default target,
/// then the modules that depend on the target analyzed again for each
/// other target of the configuration (bug: the whole package was analyzed
/// again, §4 #19).
pub(crate) fn check(project: Project, format: diagnostics::Format) -> i32 {
	let prepared = match prepare(&project, "bun", None) {
		Ok(prepared) => prepared,
		Err(message) => {
			crate::color::eprintln_message(&message);
			return 1;
		}
	};
	let mut prepared = prepared;
	let options = run_options(&prepared.entry);
	let report = timings::phase("pipeline", || {
		pipeline::analyze_package(&mut prepared.session, &options, None, &pipeline::NoSink)
	});
	record(&report);
	let mut rendered = timings::phase("render", || render(&prepared, &report, &[]));
	let mut errors = report.has_errors() || has_errors(&prepared.session);
	let dirty = report.modules.iter().any(|module| module.reads_target);
	if dirty {
		let targets = prepared.configuration.as_ref().map_or_else(
			|| vec!["web".to_string(), "bun".to_string(), "native".to_string()],
			|configuration| configuration.targets.clone(),
		);
		for target in targets.iter().filter(|target| *target != "bun") {
			let mut other = match prepare(&project, target, Some(&prepared.session)) {
				Ok(other) => other,
				Err(message) => {
					crate::report_target_error(&message, target);
					errors = true;
					continue;
				}
			};
			let options = pipeline::Options {
				retarget: true,
				..run_options(&other.entry)
			};
			let again = timings::phase("pipeline", || {
				pipeline::analyze_package(
					&mut other.session,
					&options,
					Some(&report),
					&pipeline::NoSink,
				)
			});
			record(&again);
			errors |= again.has_errors();
			rendered.merge_target(render(&other, &again, &[]), target);
		}
	}
	rendered.print(format);
	i32::from(errors)
}

/// Whether the configuration files have an error.
fn has_errors(session: &Session) -> bool {
	(0..session.file_count()).map(FileId::new).any(|file| {
		session
			.diagnostics(file)
			.iter()
			.any(|diagnostic| diagnostic.severity == Severity::Error)
	})
}

/// Generates the JavaScript of each unit as it is analyzed.
struct BuildSink<'a> {
	options: Options,
	/// The tests are run (`fa test`): the test files are generated with
	/// their tests.
	tests: bool,
	/// A test file has a `test only`.
	focused: AtomicBool,
	sink: &'a project::DirectorySink,
	/// A unit had an error: the next ones aren't generated.
	failed: AtomicBool,
	/// The modules written: `(module, helpers, has $main)`.
	written: Mutex<Vec<Written>>,
	/// The errors of the backend, by module.
	errors: Mutex<Vec<(String, Vec<Diagnostic>)>>,
	/// The first error writing a module.
	io_error: Mutex<Option<std::io::Error>>,
}

impl UnitSink for BuildSink<'_> {
	fn unit(&self, session: &mut Session, files: &[FileId]) {
		let erroneous = files.iter().any(|&file| {
			session
				.diagnostics(file)
				.iter()
				.any(|diagnostic| diagnostic.severity == Severity::Error)
		});
		if erroneous {
			self.failed.store(true, Ordering::Relaxed);
		}
		if self.tests {
			let focused = files.iter().any(|&file| {
				session.role(file) == fa_analyzer::FileRole::Test && {
					let tree = session.tree(file);
					(0..tree.nodes.len()).any(|index| {
						matches!(
							fa_analyzer::syntax::view(tree, fa_analyzer::ids::NodeId::new(index)),
							fa_analyzer::syntax::View::Test {
								modifier: Some(fa_analyzer::syntax::TestModifier::Only),
								..
							}
						)
					})
				}
			});
			if focused {
				self.focused.store(true, Ordering::Relaxed);
			}
		}
		if self.failed.load(Ordering::Relaxed) {
			return;
		}
		for &file in files {
			let module = session.module(file).to_string();
			match fa_codegen_js::generate_unit_module(
				session,
				file,
				&self.options,
				self.tests,
				self.sink,
			) {
				Ok(Ok(written)) => self
					.written
					.lock()
					.unwrap_or_else(|poisoned| poisoned.into_inner())
					.push((
						module,
						written.helpers,
						written.has_main,
						written.asynchronous,
					)),
				Ok(Err(diagnostics)) => {
					self.failed.store(true, Ordering::Relaxed);
					self.errors
						.lock()
						.unwrap_or_else(|poisoned| poisoned.into_inner())
						.push((module, diagnostics));
				}
				Err(error) => {
					self.failed.store(true, Ordering::Relaxed);
					self.io_error
						.lock()
						.unwrap_or_else(|poisoned| poisoned.into_inner())
						.get_or_insert(error);
				}
			}
		}
	}
}

/// The outcome of [`build`]: the modules written, or the exit code.
pub(crate) enum Built {
	/// The paths of the modules written, `main.js` first.
	Written(Vec<PathBuf>),
	/// Stopped (an error, no test to run): the messages are printed.
	Failed(i32),
}

/// What [`build`] builds the package for.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Purpose {
	/// `fa test`: the test files and a `main.js` running their tests.
	pub tests: bool,
	/// `fa run`, `fa test`: the warnings are printed only with an error.
	pub quiet: bool,
	/// The output folder is replaced as a whole (the cache of `fa run`),
	/// not merged into.
	pub replace: bool,
}

impl Purpose {
	/// The command, in messages: `fa test`, `fa run` (the cache replaced
	/// as a whole), else `fa build`.
	fn command(&self) -> &'static str {
		match (self.tests, self.replace) {
			(true, _) => "fa test",
			(false, true) => "fa run",
			(false, false) => "fa build",
		}
	}
}

/// `fa build` of a package for a JavaScript target into `directory`: the
/// modules are written into `<directory>.partial/` by the threads that
/// analyzed them, then moved into the directory (deleted on an error).
pub(crate) fn build(
	project: Project,
	target: Target,
	options: &Options,
	purpose: Purpose,
	directory_of: impl FnOnce(&Prepared) -> PathBuf,
) -> Built {
	let mut prepared = match prepare(&project, target.name(), None) {
		Ok(prepared) => prepared,
		Err(message) => {
			crate::color::eprintln_message(&message);
			return Built::Failed(1);
		}
	};
	let directory = directory_of(&prepared);
	let Some(partial) = partial_directory(&directory, purpose.command()) else {
		return Built::Failed(1);
	};
	let directory_sink = project::DirectorySink::new(&partial);
	let mut options = options.clone();
	options.path = prepared.project.shown.display().to_string();
	if options.libraries.is_empty() {
		options.libraries =
			crate::c_libraries(prepared.configuration.as_ref(), &prepared.project.base);
	}
	let sink = BuildSink::new(options.clone(), purpose.tests, &directory_sink);
	let run = run_options(&prepared.entry);
	let report = timings::phase("pipeline", || {
		pipeline::analyze_package(&mut prepared.session, &run, None, &sink)
	});
	record(&report);
	let focused = sink.focused.load(Ordering::Relaxed);
	let written = match sink.finish(&prepared, &report, purpose, &directory) {
		Some(written) => written,
		None => {
			let _ = fs::remove_dir_all(&partial);
			return Built::Failed(1);
		}
	};
	if purpose.tests
		&& let Some(code) = tests_stop(&prepared, &report, focused)
	{
		let _ = fs::remove_dir_all(&partial);
		return Built::Failed(code);
	}
	let paths = module_paths(&report, &written, purpose, options.declarations);
	let entry = Entry {
		report: &report,
		session: &prepared.session,
		written: &written,
		options: &options,
		tests: purpose.tests,
	};
	let result = write_entry(&entry, &directory_sink)
		.and_then(|()| finish_directory(&partial, &directory, &paths, target, purpose.replace));
	if let Err(error) = result {
		let _ = fs::remove_dir_all(&partial);
		eprintln!(
			"{}: can't write into `{}`: {error}",
			purpose.command(),
			directory.display()
		);
		return Built::Failed(1);
	}
	Built::Written(paths.iter().map(|path| directory.join(path)).collect())
}

/// The folder the modules of a build are written into before they are
/// moved into `directory`: one per process, so that two runs of the same
/// package don't share it; `None` (the error printed) when it can't be made.
fn partial_directory(directory: &Path, command: &str) -> Option<PathBuf> {
	let mut partial = directory.as_os_str().to_owned();
	partial.push(format!(".{}.partial", std::process::id()));
	let partial = PathBuf::from(partial);
	let _ = fs::remove_dir_all(&partial);
	if let Err(error) = fs::create_dir_all(&partial) {
		eprintln!(
			"{command}: can't write into `{}`: {error}",
			partial.display()
		);
		return None;
	}
	Some(partial)
}

/// A module written: (module, the helpers it uses, whether it has `$main`).
/// A module written: its path, its helpers, whether it has `$main`, and
/// whether it was generated in asynchronous mode.
type Written = (String, Vec<&'static str>, bool, bool);

impl<'a> BuildSink<'a> {
	fn new(options: Options, tests: bool, sink: &'a project::DirectorySink) -> Self {
		BuildSink {
			options,
			tests,
			focused: AtomicBool::new(false),
			sink,
			failed: AtomicBool::new(false),
			written: Mutex::new(Vec::new()),
			errors: Mutex::new(Vec::new()),
			io_error: Mutex::new(None),
		}
	}

	/// The end of the analysis: prints the diagnostics (with the backend's
	/// errors; only on an error for a quiet purpose), and gives the modules
	/// written, sorted, or `None` on an error.
	fn finish(
		self,
		prepared: &Prepared,
		report: &Report,
		purpose: Purpose,
		directory: &Path,
	) -> Option<Vec<Written>> {
		let backend: Vec<(String, Vec<Diagnostic>)> = self
			.errors
			.into_inner()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		let rendered = timings::phase("render", || render(prepared, report, &backend).text());
		let failed = report.has_errors() || has_errors(&prepared.session) || !backend.is_empty();
		if !purpose.quiet || failed {
			crate::color::eprint_diagnostics(&rendered);
		}
		let io_error = self
			.io_error
			.into_inner()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		if let Some(error) = &io_error {
			eprintln!(
				"{}: can't write into `{}`: {error}",
				purpose.command(),
				directory.display()
			);
		}
		if failed || io_error.is_some() {
			return None;
		}
		let mut written = self
			.written
			.into_inner()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		written.sort_by(|a, b| a.0.cmp(&b.0));
		Some(written)
	}
}

/// `fa test`: the exit code when the tests don't run (no test file, or a
/// `test only` where it is forbidden, `TEST004`).
fn tests_stop(prepared: &Prepared, report: &Report, focused: bool) -> Option<i32> {
	if report.tests.is_empty() {
		eprintln!("fa test: no test file (`*.test.fa`) in this package");
		return Some(0);
	}
	if focused && let Some(reason) = project::focus_forbidden(&prepared.project.base) {
		eprintln!(
			"error[TEST004]: a `test only` would hide the other tests, and {reason}: remove `only`"
		);
		return Some(1);
	}
	None
}

/// The paths of the modules of the output, `main.js` first: the entry
/// module, the runtime, and the modules written (the test files only for
/// the tests).
fn module_paths(
	report: &Report,
	written: &[Written],
	purpose: Purpose,
	declarations: bool,
) -> Vec<String> {
	let mut paths = vec!["main.js".to_string(), "$runtime.js".to_string()];
	for (module, _, _, _) in written.iter().filter(|(module, _, _, _)| {
		report
			.module(module)
			.is_some_and(|module| purpose.tests || module.role != fa_analyzer::FileRole::Test)
	}) {
		let path = fa_codegen_js::js_path(module);
		// `--declarations`: its `.d.ts` next to it
		if declarations {
			paths.push(fa_codegen_js::declarations_path(&path));
		}
		paths.push(path);
	}
	if declarations {
		paths.push("main.d.ts".to_string());
	}
	// the thread of a worker (`fa_codegen_js::worker_module`)
	if written
		.iter()
		.any(|(_, used, _, _)| used.contains(&"FaWorker"))
	{
		paths.push(fa_codegen_js::WORKER_MODULE.to_string());
	}
	paths
}

/// What the entry module (`main.js`) and the runtime are made from.
struct Entry<'e> {
	report: &'e Report,
	session: &'e Session,
	written: &'e [Written],
	options: &'e Options,
	tests: bool,
}

/// Writes `main.js` (the initialization order, the entry, the tests) and
/// `$runtime.js` (the helpers the modules use).
fn write_entry(entry: &Entry, sink: &project::DirectorySink) -> std::io::Result<()> {
	let Entry {
		report,
		session,
		written,
		options,
		tests,
	} = *entry;
	let mut helpers: BTreeSet<&'static str> = BTreeSet::new();
	for (_, used, _, _) in written {
		helpers.extend(used.iter().copied());
	}
	let entry_module = report.entry.as_ref().map(|module| EntryModule {
		module: module.clone(),
		has_main: written
			.iter()
			.any(|(other, _, has_main, _)| other == module && *has_main),
		export: report.entry_export(session, module),
		asynchronous: written
			.iter()
			.any(|(other, _, _, asynchronous)| other == module && *asynchronous),
	});
	let inputs = EntryInputs {
		init_order: report.init_order.clone(),
		entry: entry_module,
		tests: if tests {
			report.tests.clone()
		} else {
			Vec::new()
		},
	};
	let main = fa_codegen_js::entry_module(&inputs, options, &EntryOptions { tests }, &mut helpers);
	// the thread of a worker (J2)
	if let Some(worker) = fa_codegen_js::worker_module(&inputs, &mut helpers) {
		sink.write(fa_codegen_js::WORKER_MODULE, &worker)?;
	}
	let runtime = fa_codegen_js::runtime_module(&helpers);
	sink.write("$runtime.js", &runtime)?;
	if options.declarations {
		sink.write("main.d.ts", fa_codegen_js::ENTRY_DECLARATIONS)?;
	}
	sink.write("main.js", &main)
}

/// Moves the modules written into `partial` into `directory` (replacing it
/// as a whole with `replace`), with a `package.json` declaring ES modules
/// for Node.
fn finish_directory(
	partial: &Path,
	directory: &Path,
	paths: &[String],
	target: Target,
	replace: bool,
) -> std::io::Result<()> {
	if target == Target::Node {
		fs::write(partial.join("package.json"), "{ \"type\": \"module\" }\n")?;
	}
	timings::phase("write", || {
		if replace {
			let _ = fs::remove_dir_all(directory);
			if let Some(parent) = directory.parent() {
				fs::create_dir_all(parent)?;
			}
			return fs::rename(partial, directory);
		}
		project::move_into(partial, directory, paths)?;
		if target == Target::Node {
			fs::rename(partial.join("package.json"), directory.join("package.json"))?;
		}
		fs::remove_dir_all(partial)
	})
}

/// Whether a path is a package folder checked on the pipeline.
pub(crate) fn locate(path: Option<&Path>) -> Option<Project> {
	let project = project::locate(path).ok()?;
	applies(&project).then_some(project)
}

/// The outcome of [`build_native`].
pub(crate) enum NativeBuilt {
	/// The program, linked.
	Built(fa_codegen_llvm::Artifact),
	/// Errors: the messages to print.
	Failed(String),
	/// The program can't be compiled per unit (an asynchronous program, a
	/// construct the units don't compile yet): the reason, to compile it as
	/// one module instead.
	Fallback(String),
}

/// `fa build` (release: ThinLTO) or `fa run` (`development`: plain objects,
/// cached) of a package for an LLVM target, on the pipeline
/// (design/parallel-pipeline.md §3.3): each unit is compiled into an object
/// by the thread that analyzed it, then the program is linked. `output`:
/// the folder of the executable, else the package's `<build.output>/<target>/`.
pub(crate) fn build_native(
	backend: &dyn crate::native::Backend,
	project: Project,
	target: fa_codegen_llvm::Target,
	options: &fa_codegen_llvm::Options,
	development: bool,
	output: Option<&Path>,
	verbose: bool,
) -> NativeBuilt {
	let mut prepared = match prepare(&project, target.analysis_target(), None) {
		Ok(prepared) => prepared,
		Err(message) => return NativeBuilt::Failed(message),
	};
	let directory = match output {
		Some(output) => output.to_path_buf(),
		None => {
			let folder = prepared.configuration.as_ref().map_or_else(
				|| "build".to_string(),
				|configuration| configuration.build.output.clone(),
			);
			prepared.project.base.join(folder).join(target.name())
		}
	};
	let objects = {
		let mut name = directory.as_os_str().to_owned();
		name.push(format!(".{}.objects", std::process::id()));
		PathBuf::from(name)
	};
	let cache = crate::dependencies::cache_root().join("native");
	let split = fa_codegen_llvm::SplitOptions {
		lto: !development && options.optimized(),
		// `FA_LTO=full`: units written for full LTO from the start
		full_lto: !development && std::env::var("FA_LTO").is_ok_and(|value| value == "full"),
		verbose,
		cache: development.then(|| cache.join("objects")),
		runtime_cache: Some(cache.join("runtime")),
		jobs: 0,
		// merged from the units by `compile_package`
		facts: None,
		// set by `compile_package` from the libraries
		c_library: None,
	};
	// a C library (`build.cLibrary`, `fa build` only): the API file is the entry
	let configuration = prepared.configuration.as_ref().filter(|_| !development);
	let c_library = match crate::build::c_library_of(configuration, &prepared.project.base) {
		Ok(c_library) => c_library,
		Err(message) => return NativeBuilt::Failed(message),
	};
	let entry = c_library
		.as_ref()
		.map_or_else(|| prepared.entry.clone(), |library| library.api_module());
	let libraries = fa_codegen_llvm::Libraries {
		c_library,
		link: prepared
			.configuration
			.as_ref()
			.map(|configuration| configuration.build.link.clone())
			.unwrap_or_default(),
		paths: prepared
			.configuration
			.as_ref()
			.map(|configuration| {
				configuration
					.build
					.library_paths
					.iter()
					.map(|path| prepared.project.base.join(path))
					.collect()
			})
			.unwrap_or_default(),
	};
	let run = run_options(&entry);
	let (report, outcome) = timings::phase("pipeline", || {
		backend.compile_package(
			&mut prepared.session,
			&run,
			target,
			options,
			&split,
			&objects,
			&directory.join("main"),
			&libraries,
		)
	});
	record(&report);
	let rendered = timings::phase("render", || render(&prepared, &report, &[]).text());
	if report.has_errors() || has_errors(&prepared.session) {
		return NativeBuilt::Failed(rendered);
	}
	match outcome {
		fa_codegen_llvm::PackageOutcome::Built(artifact) => {
			crate::color::eprint_diagnostics(&rendered);
			NativeBuilt::Built(artifact)
		}
		fa_codegen_llvm::PackageOutcome::Errors => NativeBuilt::Failed(rendered),
		fa_codegen_llvm::PackageOutcome::Unsupported(reason) => NativeBuilt::Fallback(reason),
		fa_codegen_llvm::PackageOutcome::Failed(error) => {
			NativeBuilt::Failed(format!("{rendered}{error}\n"))
		}
	}
}
