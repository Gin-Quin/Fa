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

use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::ids::FileId;
use fa_analyzer::pipeline::{self, Report, UnitSink};
use fa_analyzer::{Diagnostic, Session, Severity};
use fa_codegen_js::{EntryInputs, EntryModule, EntryOptions, ModuleSink, Options, Target};

use crate::project::{self, Prepared, Project};
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

/// The diagnostics of a module, rendered with its text (read again).
fn render_module(
	session: &Session,
	project: &Project,
	module: &str,
	diagnostics: &[Diagnostic],
) -> String {
	if diagnostics.is_empty() {
		return String::new();
	}
	let text = session
		.resolver()
		.and_then(|resolver| resolver.source(module))
		.unwrap_or_default();
	let path = shown(session, project, module);
	render_all(diagnostics, &SourceView::new(&path, &text))
}

/// The diagnostics of a run and of the configuration files, rendered in
/// the order of the module paths.
fn render(prepared: &Prepared, report: &Report, extra: &[(String, Vec<Diagnostic>)]) -> String {
	let session = &prepared.session;
	let mut blocks: Vec<(String, String)> = Vec::new();
	// the configuration files, analyzed while preparing
	for file in (0..session.file_count()).map(FileId::new) {
		let module = session.module(file);
		if session.loaded_module(module) != Some(file) || session.diagnostics(file).is_empty() {
			continue;
		}
		let path = shown(session, &prepared.project, module);
		let source = SourceView::new(&path, session.text(file));
		blocks.push((
			module.to_string(),
			render_all(session.diagnostics(file), &source),
		));
	}
	for module in &report.modules {
		let mut diagnostics = module.diagnostics.clone();
		if let Some((_, more)) = extra.iter().find(|(other, _)| *other == module.module) {
			diagnostics.extend(more.iter().cloned());
			fa_analyzer::diagnostics::sort(&mut diagnostics);
		}
		let rendered = render_module(session, &prepared.project, &module.module, &diagnostics);
		if !rendered.is_empty() {
			blocks.push((module.module.clone(), rendered));
		}
	}
	blocks.sort_by(|a, b| a.0.cmp(&b.0));
	let mut out = String::new();
	for warning in &prepared.warnings {
		out.push_str(warning);
		out.push_str("\n\n");
	}
	for (_, block) in blocks {
		out.push_str(&block);
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
pub(crate) fn check(project: Project) -> i32 {
	let prepared = match prepare(&project, "bun", None) {
		Ok(prepared) => prepared,
		Err(message) => {
			eprintln!("{message}");
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
			let Ok(mut other) = prepare(&project, target, Some(&prepared.session)) else {
				continue;
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
			let other_rendered = render(&other, &again, &[]);
			for block in other_rendered.split("\n\n") {
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
	sink: &'a project::DirectorySink,
	/// A unit had an error: the next ones aren't generated.
	failed: AtomicBool,
	/// The modules written: `(module, helpers, has $main)`.
	written: Mutex<Vec<(String, Vec<&'static str>, bool)>>,
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
		if self.failed.load(Ordering::Relaxed) {
			return;
		}
		for &file in files {
			let module = session.module(file).to_string();
			match fa_codegen_js::generate_unit_module(
				session,
				file,
				&self.options,
				false,
				self.sink,
			) {
				Ok(Ok(written)) => self
					.written
					.lock()
					.unwrap_or_else(|poisoned| poisoned.into_inner())
					.push((module, written.helpers, written.has_main)),
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
	/// Failed; the diagnostics are printed.
	Failed(i32),
}

/// `fa build` of a package for a JavaScript target into `directory`: the
/// modules are written into `<directory>.partial/` by the threads that
/// analyzed them, then moved into the directory (deleted on an error).
pub(crate) fn build(
	project: Project,
	target: Target,
	options: &Options,
	directory_of: impl FnOnce(&Prepared) -> PathBuf,
) -> Built {
	let mut prepared = match prepare(&project, target.name(), None) {
		Ok(prepared) => prepared,
		Err(message) => {
			eprintln!("{message}");
			return Built::Failed(1);
		}
	};
	let directory = directory_of(&prepared);
	let mut partial = directory.as_os_str().to_owned();
	partial.push(".partial");
	let partial = PathBuf::from(partial);
	let _ = fs::remove_dir_all(&partial);
	if let Err(error) = fs::create_dir_all(&partial) {
		eprintln!(
			"fa build: can't write into `{}`: {error}",
			partial.display()
		);
		return Built::Failed(1);
	}
	let directory_sink = project::DirectorySink::new(&partial);
	let mut options = options.clone();
	options.path = prepared.project.shown.display().to_string();
	let sink = BuildSink {
		options: options.clone(),
		sink: &directory_sink,
		failed: AtomicBool::new(false),
		written: Mutex::new(Vec::new()),
		errors: Mutex::new(Vec::new()),
		io_error: Mutex::new(None),
	};
	let run = run_options(&prepared.entry);
	let report = timings::phase("pipeline", || {
		pipeline::analyze_package(&mut prepared.session, &run, None, &sink)
	});
	record(&report);
	let backend: Vec<(String, Vec<Diagnostic>)> = sink
		.errors
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	let rendered = timings::phase("render", || render(&prepared, &report, &backend));
	eprint!("{rendered}");
	let failed = report.has_errors() || has_errors(&prepared.session) || !backend.is_empty();
	let io_error = sink
		.io_error
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	if failed || io_error.is_some() {
		let _ = fs::remove_dir_all(&partial);
		if let Some(error) = io_error {
			eprintln!(
				"fa build: can't write into `{}`: {error}",
				directory.display()
			);
		}
		return Built::Failed(1);
	}
	let mut written = sink
		.written
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	written.sort_by(|a, b| a.0.cmp(&b.0));
	let mut helpers: BTreeSet<&'static str> = BTreeSet::new();
	for (_, used, _) in &written {
		helpers.extend(used.iter().copied());
	}
	let entry = report.entry.as_ref().map(|module| EntryModule {
		module: module.clone(),
		has_main: written
			.iter()
			.any(|(other, _, has_main)| other == module && *has_main),
		export: report.entry_export(&prepared.session, module),
	});
	let inputs = EntryInputs {
		init_order: report.init_order.clone(),
		entry,
		tests: Vec::new(),
	};
	let main =
		fa_codegen_js::entry_module(&inputs, &options, &EntryOptions::default(), &mut helpers);
	let runtime = fa_codegen_js::runtime_module(&helpers);
	let mut paths = vec!["main.js".to_string(), "$runtime.js".to_string()];
	paths.extend(
		written
			.iter()
			.filter(|(module, _, _)| {
				report
					.module(module)
					.is_some_and(|module| module.role != fa_analyzer::FileRole::Test)
			})
			.map(|(module, _, _)| fa_codegen_js::js_path(module)),
	);
	let result = (|| -> std::io::Result<()> {
		directory_sink.write("$runtime.js", &runtime)?;
		directory_sink.write("main.js", &main)?;
		if target == Target::Node {
			fs::write(partial.join("package.json"), "{ \"type\": \"module\" }\n")?;
		}
		timings::phase("write", || {
			project::move_into(&partial, &directory, &paths)?;
			if target == Target::Node {
				fs::rename(partial.join("package.json"), directory.join("package.json"))?;
			}
			fs::remove_dir_all(&partial)
		})
	})();
	if let Err(error) = result {
		let _ = fs::remove_dir_all(&partial);
		eprintln!(
			"fa build: can't write into `{}`: {error}",
			directory.display()
		);
		return Built::Failed(1);
	}
	Built::Written(paths.iter().map(|path| directory.join(path)).collect())
}

/// Whether a path is a package folder checked on the pipeline.
pub(crate) fn locate(path: Option<&Path>) -> Option<Project> {
	let project = project::locate(path).ok()?;
	applies(&project).then_some(project)
}
