//! The analyzer behind the language server: an `AnalysisBackend` over
//! `fa_analyzer` sessions.
//!
//! A document inside a package (a folder with a `package.fa` above it) is a
//! file of that package, analyzed with the parallel pipeline
//! (design/parallel-pipeline.md §3.4):
//!
//! - **Initial load**: when the first document of a package opens, the
//!   package is scanned, its dependencies resolved like the command line
//!   does (`fa::editor_session`, the lockfile read, never written), and
//!   every module analyzed in parallel, unit by unit. The open documents'
//!   units keep their session (trees, analyses) for the queries; the other
//!   files keep only their interfaces (their trees are dropped). An error
//!   of the resolution is, like for `fa check`, the only diagnostic of the
//!   package's documents (the package is then analyzed without its
//!   dependencies, for the queries); its warnings (`DEP004`) are shown on
//!   `dependencies.fa`.
//! - **Save** of a configuration file (`package.fa`, `dependencies.fa`,
//!   `workspace.fa`, `dependencies.lock`): the packages below its folder are
//!   loaded again (their dependencies resolved again).
//! - **Edit, open**: the document's text replaces the disk's (an overlay of
//!   the module system), and an incremental run analyzes its module again,
//!   compares its interface hash, and analyzes the units that read it again
//!   only when it changed, in parallel by the order of the module graph,
//!   stopping between units when a newer edit sets the cancel flag.
//! - **Go-to-definition** into a file whose tree is dropped uses the spans
//!   its interface recorded for the declaration's key.
//!
//! Any other document on the disk is a standalone file, analyzed like
//! `fa check <file>` does: in a session of its own whose module system is
//! its folder (the files and folders next to it are its siblings), with the
//! texts of the open documents of that folder in place of the disk's. An
//! edit of a document analyzes the open documents of its folder that read
//! it again. A document that isn't on the disk (or whose folder can't be
//! read) is a standalone file of a shared session, without siblings.
//!
//! A new text is a new version of the file: the previous version is
//! forgotten. A standalone session's type store and its list of file
//! versions only grow, so it is rebuilt from its documents when its store
//! has grown to four times its size after the last rebuild (design §4,
//! "epoch reset"), or when it holds many forgotten versions; a package is
//! loaded again when its global type store has grown to four times its
//! size after the load.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use fa_analyzer::ide::CompletionKind;
use fa_analyzer::ids::FileId;
use fa_analyzer::modules::OverlayResolver;
use fa_analyzer::pipeline::{self, Report, UnitSink};
use fa_analyzer::{CodebaseResolver, ModuleResolver, Session};
use fa_parser::{SourceSpan, TypedSyntaxTree};
use rustc_hash::FxHashSet;
use tower_lsp::lsp_types::{CompletionItemKind, Url};

use crate::backend::{
	AnalysisBackend, CompletionInfo, DefinitionTarget, FileDiagnostic, HoverInfo,
	RelatedInformation, Severity,
};

/// The default size of the type store below which the session is never rebuilt.
const MINIMUM_REBUILD_SIZE: usize = 50_000;

/// The number of forgotten file versions above which a session is rebuilt.
const MAXIMUM_FORGOTTEN_FILES: usize = 1_000;

/// The size of a session after its last rebuild.
#[derive(Clone, Copy)]
struct Baseline {
	types: usize,
	files: usize,
}

impl Baseline {
	fn of(session: &Session) -> Self {
		Baseline {
			types: session.types().len(),
			files: session.file_count(),
		}
	}

	/// Whether a session has grown enough since to be rebuilt.
	fn outgrown_by(&self, session: &Session, minimum: usize) -> bool {
		let types = session.types().len();
		(types >= minimum && types >= self.types * 4)
			|| session.file_count() > self.files + MAXIMUM_FORGOTTEN_FILES
	}
}

/// The session of a standalone document on the disk: its folder is its
/// module system (`fa check <file>`).
struct Standalone {
	session: Session,
	/// Its folder.
	folder: PathBuf,
	/// Its module (the file name without `.fa`).
	module: String,
	/// The size of the session after its last rebuild.
	baseline: Baseline,
}

impl Standalone {
	/// A session over the folder of a document, its text analyzed; `None`
	/// when the folder can't be read.
	fn new(path: &Path, module: &str, texts: &Texts, cancel: &Arc<AtomicBool>) -> Option<Self> {
		let folder = path.parent()?.to_path_buf();
		let (codebase, _) = fa_codebase_parser::Codebase::scan_standalone(path).ok()?;
		let mut session = Session::new();
		session.set_cancel_flag(Arc::clone(cancel));
		session.set_resolver(Box::new(OverlayResolver {
			inner: Box::new(CodebaseResolver::new(codebase, Some(folder.clone()))),
			texts: Arc::clone(texts),
		}));
		Some(Standalone {
			baseline: Baseline::of(&session),
			session,
			folder,
			module: module.to_string(),
		})
	}

	/// Parses and analyzes a new text of the document.
	fn load(&mut self, text: Arc<str>) -> FileId {
		let file = self.session.set_source(&self.module, text);
		self.session.analyze_file(file);
		file
	}
}

/// An open document: its file in the session, and its text (to rebuild the session).
struct OpenDocument {
	file: FileId,
	text: Arc<str>,
	/// The root of its package, when it belongs to one.
	package: Option<PathBuf>,
}

/// The package folder containing a file, if the file is one of its sources
/// (one the package's scan reads): a file of `build/`, or of a folder other
/// than `library/`, `application/` and `scripts/`, is unknown to the
/// package, and would load it again at each edit; it is a standalone file.
fn find_package(path: &Path) -> Option<PathBuf> {
	let directory = fa_codebase_parser::discover(path.parent()?).package?;
	let relative = path.strip_prefix(&directory).ok()?;
	fa_codebase_parser::is_package_source(relative).then_some(directory)
}

/// The module of a file of a package (`library/math`).
fn package_module(root: &Path, path: &Path) -> Option<String> {
	let relative = path.strip_prefix(root).ok()?;
	let text = relative.to_string_lossy().replace('\\', "/");
	Some(text.strip_suffix(".fa").unwrap_or(&text).to_string())
}

/// The texts of the open documents of a package, by module (shared with
/// the module system of its runs).
type Texts = Arc<RwLock<rustc_hash::FxHashMap<String, Arc<str>>>>;

/// The session of a unit of the last runs that holds an open document:
/// its files keep their trees and analyses.
struct KeptUnit {
	/// The modules of the unit.
	modules: Vec<String>,
	session: Session,
}

/// What a run keeps: the sessions of the units of the open documents.
struct Keep<'k> {
	open: &'k FxHashSet<String>,
	kept: Mutex<Vec<KeptUnit>>,
}

impl UnitSink for Keep<'_> {
	fn unit(&self, _session: &mut Session, _files: &[FileId]) {}

	fn keep(&self, session: Session, files: &[FileId]) {
		let modules: Vec<String> = files
			.iter()
			.map(|&file| session.module(file).to_string())
			.collect();
		if modules.iter().any(|module| self.open.contains(module)) {
			self.kept
				.lock()
				.unwrap_or_else(|poisoned| poisoned.into_inner())
				.push(KeptUnit { modules, session });
		}
	}
}

/// A package with an open document.
pub struct Package {
	/// The session the runs start from: the module system (with the open
	/// documents' texts), the configuration, the global type store.
	root: Session,
	/// The module of the application's entry.
	entry: String,
	/// The texts of the open documents.
	texts: Texts,
	/// The last complete run: the interfaces and the diagnostics of every
	/// module.
	report: Option<Report>,
	/// The sessions of the units holding an open document.
	units: Vec<KeptUnit>,
	/// The modules edited since the last complete run.
	pending: FxHashSet<String>,
	/// The size of the global type store after the initial load.
	baseline: usize,
	/// The error of the resolution of the dependencies, as `fa check`
	/// prints it: the package is then analyzed without its dependencies,
	/// and its documents show this error only (like `fa check`).
	resolution_error: Option<String>,
	/// The warnings of the resolution (`DEP004`), shown on `dependencies.fa`.
	resolution_warnings: Vec<String>,
}

impl Package {
	/// Scans a package (its dependencies resolved), not analyzed yet.
	fn new(root: &Path, texts: Texts, cancel: &Arc<AtomicBool>) -> Package {
		let overlay = |inner: Box<dyn ModuleResolver>| -> Box<dyn ModuleResolver> {
			Box::new(OverlayResolver {
				inner,
				texts: Arc::clone(&texts),
			})
		};
		let mut resolution_error = None;
		let mut resolution_warnings = Vec::new();
		let (mut session, entry) = match fa::editor_session(root, overlay) {
			Ok(found) => {
				resolution_warnings = found.warnings;
				(found.session, found.entry)
			}
			// the dependencies can't be resolved: the package's own files
			Err(error) => {
				resolution_error = Some(error);
				let mut session = Session::new();
				if let Ok(codebase) = fa_codebase_parser::Codebase::scan_package(root) {
					let resolver = CodebaseResolver::new(codebase, Some(root.to_path_buf()));
					session.set_resolver(Box::new(OverlayResolver {
						inner: Box::new(resolver),
						texts: Arc::clone(&texts),
					}));
				}
				(session, "application/main".to_string())
			}
		};
		session.set_cancel_flag(Arc::clone(cancel));
		Package {
			root: session,
			entry,
			texts,
			report: None,
			units: Vec::new(),
			pending: FxHashSet::default(),
			baseline: 0,
			resolution_error,
			resolution_warnings,
		}
	}

	/// The diagnostics of the resolution of the dependencies for a module:
	/// its error for every module (instead of their own diagnostics), its
	/// warnings on `dependencies.fa`.
	fn resolution_diagnostics(&self, module: &str) -> Option<Vec<FileDiagnostic>> {
		if let Some(error) = &self.resolution_error {
			return Some(vec![resolution_diagnostic(error, Severity::Error)]);
		}
		(module == "dependencies" && !self.resolution_warnings.is_empty()).then(|| {
			self.resolution_warnings
				.iter()
				.map(|warning| resolution_diagnostic(warning, Severity::Warning))
				.collect()
		})
	}

	/// Whether the module system of the package knows a module.
	fn knows(&self, module: &str) -> bool {
		self.root
			.resolver()
			.is_some_and(|resolver| resolver.modules().iter().any(|known| known == module))
	}

	/// The modules of the open documents.
	fn open(&self) -> FxHashSet<String> {
		self.texts
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.keys()
			.cloned()
			.collect()
	}

	/// Runs the pipeline: every module (no run completed yet), or
	/// incrementally from the last complete run, the edited modules
	/// analyzed again. Returns the modules analyzed, or `None` when the
	/// cancel flag stopped the run (the edited modules wait for the next).
	fn run(&mut self, dirty: Option<String>) -> Option<Vec<String>> {
		if let Some(module) = dirty {
			self.pending.insert(module);
		}
		let incremental = self.report.is_some();
		let open = self.open();
		let options = pipeline::Options {
			entry: Some(self.entry.clone()),
			dirty: incremental.then(|| self.pending.clone()),
			..pipeline::Options::default()
		};
		let sink = Keep {
			open: &open,
			kept: Mutex::new(Vec::new()),
		};
		let report =
			pipeline::analyze_package(&mut self.root, &options, self.report.as_ref(), &sink);
		if report.cancelled {
			return None;
		}
		self.pending.clear();
		let analyzed: Vec<String> = report
			.modules
			.iter()
			.filter(|module| module.analyzed)
			.map(|module| module.module.clone())
			.collect();
		let kept = sink
			.kept
			.into_inner()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		for unit in kept {
			self.units.retain(|old| {
				!old.modules
					.iter()
					.any(|module| unit.modules.contains(module))
			});
			self.units.push(unit);
		}
		// the closed documents' units are dropped; the others read the
		// newest interfaces (the keys of their resolutions still lead there)
		self.units
			.retain(|unit| unit.modules.iter().any(|module| open.contains(module)));
		for unit in &mut self.units {
			unit.session.set_shared(Some(Arc::clone(&report.shared)));
		}
		if !incremental {
			self.baseline = report.stats.global_types;
		}
		self.report = Some(report);
		Some(analyzed)
	}

	/// Whether the global type store has grown enough to load the package
	/// again (the epoch reset).
	fn outgrown(&self, minimum: usize) -> bool {
		self.report.as_ref().is_some_and(|report| {
			let types = report.stats.global_types;
			types >= minimum && types >= self.baseline * 4
		})
	}

	/// The kept session of a module and its file there.
	fn located(&self, module: &str) -> Option<(&Session, FileId)> {
		self.units.iter().find_map(|unit| {
			let file = unit.session.loaded_module(module)?;
			Some((&unit.session, file))
		})
	}

	/// The diagnostics of a module from the last complete run (the
	/// program's checks included).
	fn diagnostics(&self, module: &str) -> Option<&[fa_analyzer::Diagnostic]> {
		Some(&self.report.as_ref()?.module(module)?.diagnostics)
	}

	/// The file on the disk of a module.
	fn file_path(&self, module: &str) -> Option<PathBuf> {
		self.root.resolver()?.file(module)
	}

	/// The number of trees kept (the files of the kept units), for tests.
	pub fn kept_trees(&self) -> usize {
		self.units
			.iter()
			.map(|unit| unit.session.file_count())
			.sum()
	}
}

/// The analyzer as a language server backend.
pub struct AnalyzerBackend {
	session: Session,
	/// Each package with an open document, by root folder.
	packages: HashMap<PathBuf, Package>,
	documents: HashMap<Url, OpenDocument>,
	/// The size of the standalone session after its last rebuild.
	baseline: Baseline,
	/// The sessions of the standalone documents on the disk.
	standalone: HashMap<Url, Standalone>,
	/// The texts of the open standalone documents, by folder, then module.
	folders: HashMap<PathBuf, Texts>,
	/// The size of the type store below which a session is never rebuilt.
	rebuild_minimum: usize,
	cancel: Arc<AtomicBool>,
	/// Whether to publish `AN001` ("not supported yet by the analyzer"). Off by
	/// default: it says nothing about the user's code.
	pub show_unsupported: bool,
}

impl Default for AnalyzerBackend {
	fn default() -> Self {
		Self::new()
	}
}

impl AnalyzerBackend {
	/// A backend with no document.
	pub fn new() -> Self {
		let cancel = Arc::new(AtomicBool::new(false));
		let mut session = Session::new();
		session.set_cancel_flag(Arc::clone(&cancel));
		AnalyzerBackend {
			baseline: Baseline::of(&session),
			session,
			standalone: HashMap::new(),
			folders: HashMap::new(),
			packages: HashMap::new(),
			documents: HashMap::new(),
			rebuild_minimum: MINIMUM_REBUILD_SIZE,
			cancel,
			show_unsupported: false,
		}
	}

	/// The analyzer's shared standalone session (the documents that aren't
	/// on the disk), for tests and tools.
	pub fn session(&self) -> &Session {
		&self.session
	}

	/// The package of a folder, if one of its documents is open.
	pub fn package(&self, root: &Path) -> Option<&Package> {
		self.packages.get(root)
	}

	/// The module path of a document: its file name without extension.
	fn module(uri: &Url) -> String {
		let name = uri.path().rsplit('/').next().unwrap_or("");
		let name = name.strip_suffix(".fa").unwrap_or(name);
		if name.is_empty() {
			"main".to_string()
		} else {
			name.to_string()
		}
	}

	/// Parses and analyzes a text as the current version of a document.
	fn load(&mut self, uri: &Url, text: Arc<str>) -> FileId {
		let file = self
			.session
			.set_source(&Self::module(uri), Arc::clone(&text));
		self.session.analyze_file(file);
		file
	}

	/// Rebuilds the session from the open documents when its type store has grown too much.
	fn rebuild_if_grown(&mut self) {
		if !self
			.baseline
			.outgrown_by(&self.session, self.rebuild_minimum)
		{
			return;
		}
		let mut session = Session::new();
		session.set_cancel_flag(Arc::clone(&self.cancel));
		self.session = session;
		let documents: Vec<(Url, Arc<str>)> = self
			.documents
			.iter()
			.filter(|(uri, document)| {
				document.package.is_none() && !self.standalone.contains_key(*uri)
			})
			.map(|(uri, document)| (uri.clone(), Arc::clone(&document.text)))
			.collect();
		for (uri, text) in documents {
			let file = self.load(&uri, Arc::clone(&text));
			self.documents.insert(
				uri,
				OpenDocument {
					file,
					text,
					package: None,
				},
			);
		}
		self.baseline = Baseline::of(&self.session);
	}

	fn file(&self, uri: &Url) -> Option<FileId> {
		self.located(uri).map(|(_, file)| file)
	}

	/// The package of a document and its module, if it is in one.
	fn package_of(&self, uri: &Url) -> Option<(&Package, String)> {
		let path = uri.to_file_path().ok()?;
		let (root, package) = self
			.packages
			.iter()
			.find(|(root, _)| path.starts_with(root))?;
		let module = package_module(root, &path)?;
		Some((package, module))
	}

	/// The session and the current file of a document: the standalone
	/// session's, or the kept session of its unit in its package.
	fn located(&self, uri: &Url) -> Option<(&Session, FileId)> {
		if let Some(document) = self.documents.get(uri)
			&& document.package.is_none()
		{
			let session = self
				.standalone
				.get(uri)
				.map_or(&self.session, |standalone| &standalone.session);
			return Some((session, document.file));
		}
		let (package, module) = self.package_of(uri)?;
		package.located(&module)
	}

	/// The session of a document.
	fn session_of(&self, uri: &Url) -> &Session {
		self.located(uri)
			.map_or(&self.session, |(session, _)| session)
	}

	/// A new text of a document of a package: an incremental run analyzes
	/// it again and the files using what changed (the whole package the
	/// first time); the URIs of the open documents analyzed.
	fn set_package_document(&mut self, uri: &Url, root: PathBuf, text: Arc<str>) -> Vec<Url> {
		let Some(module) = uri
			.to_file_path()
			.ok()
			.and_then(|path| package_module(&root, &path))
		else {
			return Vec::new();
		};
		self.documents.insert(
			uri.clone(),
			OpenDocument {
				file: FileId::new(0),
				text: Arc::clone(&text),
				package: Some(root.clone()),
			},
		);
		let reload = self.packages.get(&root).is_none_or(|package| {
			!package.knows(&module) || package.outgrown(self.rebuild_minimum)
		});
		if reload {
			// a new package, a file the last scan didn't see, or a store
			// grown too much: scan and analyze again
			let texts: Texts = match self.packages.remove(&root) {
				Some(package) => package.texts,
				None => Arc::default(),
			};
			let package = Package::new(&root, texts, &self.cancel);
			self.packages.insert(root.clone(), package);
		}
		let package = self.packages.get_mut(&root).expect("inserted above");
		package
			.texts
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.insert(module.clone(), text);
		let analyzed = if package.report.is_none() {
			package.run(None)
		} else {
			package.run(Some(module))
		};
		let package = &self.packages[&root];
		let mut urls: Vec<Url> = analyzed
			.unwrap_or_default()
			.iter()
			.filter_map(|module| package.file_path(module))
			.filter_map(|path| Url::from_file_path(path).ok())
			.filter(|url| url != uri && self.documents.contains_key(url))
			.collect();
		urls.insert(0, uri.clone());
		urls
	}
}

impl AnalyzerBackend {
	/// The file on the disk of a module seen from a document: of its
	/// package, or of the folder of a standalone document.
	fn file_path(&self, uri: &Url, session: &Session, module: &str) -> Option<PathBuf> {
		match self.package_of(uri) {
			Some((package, _)) => package.file_path(module),
			None => session.resolver()?.file(module),
		}
	}

	/// A new text of a standalone document on the disk, analyzed in the
	/// session of its folder; the documents of the folder that read it are
	/// analyzed again. `None` when it isn't on the disk or its folder can't
	/// be read: it goes to the shared session.
	fn set_standalone_document(&mut self, uri: &Url, text: Arc<str>) -> Option<Vec<Url>> {
		let path = uri.to_file_path().ok()?;
		let module = Self::module(uri);
		let folder = path.parent()?.to_path_buf();
		let texts = self.folders.get(&folder).cloned().unwrap_or_default();
		if !self.standalone.contains_key(uri) {
			let standalone = Standalone::new(&path, &module, &texts, &self.cancel)?;
			self.standalone.insert(uri.clone(), standalone);
			self.folders.insert(folder.clone(), Arc::clone(&texts));
		}
		texts
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.insert(module.clone(), Arc::clone(&text));
		let standalone = self.standalone.get_mut(uri).expect("inserted above");
		let file = standalone.load(Arc::clone(&text));
		if let Some(previous) = self.documents.insert(
			uri.clone(),
			OpenDocument {
				file,
				text,
				package: None,
			},
		) {
			standalone.session.forget(previous.file);
		}
		if standalone
			.baseline
			.outgrown_by(&standalone.session, self.rebuild_minimum)
		{
			self.rebuild_standalone(uri);
		}
		let mut analyzed = vec![uri.clone()];
		analyzed.extend(self.reload_readers(uri, &folder, &module));
		Some(analyzed)
	}

	/// Closes a standalone document on the disk: the documents of its folder
	/// that read it read the disk's text again (they are analyzed again,
	/// and returned).
	fn close_standalone_document(&mut self, uri: &Url) -> Vec<Url> {
		let Some(standalone) = self.standalone.remove(uri) else {
			return Vec::new();
		};
		if let Some(texts) = self.folders.get(&standalone.folder) {
			let mut texts = texts
				.write()
				.unwrap_or_else(|poisoned| poisoned.into_inner());
			texts.remove(&standalone.module);
			let empty = texts.is_empty();
			drop(texts);
			if empty {
				self.folders.remove(&standalone.folder);
			}
		}
		self.reload_readers(uri, &standalone.folder, &standalone.module)
	}

	/// Analyzes again, in new sessions, the other standalone documents of a
	/// folder whose sessions loaded a module (its text changed).
	fn reload_readers(&mut self, edited: &Url, folder: &Path, module: &str) -> Vec<Url> {
		let readers: Vec<Url> = self
			.standalone
			.iter()
			.filter(|(uri, standalone)| {
				*uri != edited
					&& standalone.folder == folder
					&& standalone.session.loaded_module(module).is_some()
			})
			.map(|(uri, _)| uri.clone())
			.collect();
		for reader in &readers {
			self.rebuild_standalone(reader);
		}
		readers
	}

	/// Replaces the session of a standalone document by a new one, its text
	/// analyzed again (the store had grown, or a sibling changed).
	fn rebuild_standalone(&mut self, uri: &Url) {
		let (Some(standalone), Some(document)) =
			(self.standalone.get(uri), self.documents.get(uri))
		else {
			return;
		};
		let Some(path) = uri.to_file_path().ok() else {
			return;
		};
		let texts = self
			.folders
			.get(&standalone.folder)
			.cloned()
			.unwrap_or_default();
		let text = Arc::clone(&document.text);
		let Some(mut fresh) = Standalone::new(&path, &standalone.module, &texts, &self.cancel)
		else {
			return;
		};
		let file = fresh.load(Arc::clone(&text));
		fresh.baseline = Baseline::of(&fresh.session);
		self.standalone.insert(uri.clone(), fresh);
		self.documents.insert(
			uri.clone(),
			OpenDocument {
				file,
				text,
				package: None,
			},
		);
	}
}

/// A message of the resolution of the dependencies (`<file>: error[DEP008]:
/// …`, `warning[DEP004]: …`, `error: …`) as a diagnostic at the start of a
/// file.
fn resolution_diagnostic(message: &str, severity: Severity) -> FileDiagnostic {
	let marker = match severity {
		Severity::Error => "error",
		_ => "warning",
	};
	let (code, text) = match message.find(&format!("{marker}[")) {
		Some(start) => {
			let rest = &message[start + marker.len() + 1..];
			match rest.split_once("]: ") {
				Some((code, text)) => (Some(code.to_string()), text),
				None => (None, message),
			}
		}
		None => (
			None,
			message
				.find(&format!("{marker}: "))
				.map_or(message, |start| &message[start + marker.len() + 2..]),
		),
	};
	FileDiagnostic {
		severity,
		code,
		message: text.trim_end().to_string(),
		span: SourceSpan::new(0, 0),
		related: Vec::new(),
		unnecessary: false,
	}
}

/// Whether a file is a configuration file of packages: their dependencies
/// or their configuration are read again when it is saved.
fn is_configuration(path: &Path) -> bool {
	path.file_name().is_some_and(|name| {
		matches!(
			name.to_str(),
			Some("package.fa" | "dependencies.fa" | "workspace.fa" | "dependencies.lock")
		)
	})
}

fn span(span: fa_analyzer::diagnostics::SourceSpan) -> SourceSpan {
	SourceSpan::new(span.start as usize, span.end as usize)
}

impl AnalysisBackend for AnalyzerBackend {
	fn set_document(&mut self, uri: &Url, text: Arc<str>) -> Vec<Url> {
		// a file of a package: the package's runs (workspace folder = package)
		if let Some(root) = uri.to_file_path().ok().and_then(|path| find_package(&path)) {
			return self.set_package_document(uri, root, text);
		}
		if let Some(analyzed) = self.set_standalone_document(uri, Arc::clone(&text)) {
			return analyzed;
		}
		let file = self.load(uri, Arc::clone(&text));
		if let Some(previous) = self.documents.insert(
			uri.clone(),
			OpenDocument {
				file,
				text,
				package: None,
			},
		) {
			self.session.forget(previous.file);
		}
		self.rebuild_if_grown();
		vec![uri.clone()]
	}

	fn close_document(&mut self, uri: &Url) -> Vec<Url> {
		let Some(document) = self.documents.remove(uri) else {
			return Vec::new();
		};
		match &document.package {
			None if self.standalone.contains_key(uri) => {
				return self.close_standalone_document(uri);
			}
			None => self.session.forget(document.file),
			Some(root) => {
				if let Some(package) = self.packages.get_mut(root)
					&& let Ok(path) = uri.to_file_path()
					&& let Some(module) = package_module(root, &path)
				{
					// the disk's text again at the next run; the unit's
					// session is dropped then
					package
						.texts
						.write()
						.unwrap_or_else(|poisoned| poisoned.into_inner())
						.remove(&module);
					package.pending.insert(module.clone());
					let open = package.open();
					package
						.units
						.retain(|unit| unit.modules.iter().any(|module| open.contains(module)));
				}
				// the last document of a package: the package is dropped
				let open = self
					.documents
					.values()
					.any(|other| other.package.as_ref() == Some(root));
				if !open {
					self.packages.remove(root);
				} else if let Some(package) = self.packages.get_mut(root)
					&& package.report.is_some()
				{
					// the file is read from the disk again: the open files
					// using it are analyzed again (its unsaved text may have
					// changed their diagnostics)
					let analyzed = package.run(None).unwrap_or_default();
					return analyzed
						.iter()
						.filter_map(|module| package.file_path(module))
						.filter_map(|path| Url::from_file_path(path).ok())
						.filter(|url| url != uri && self.documents.contains_key(url))
						.collect();
				}
			}
		}
		Vec::new()
	}

	fn tree(&self, uri: &Url) -> Option<&TypedSyntaxTree> {
		if !self.documents.contains_key(uri) {
			return None;
		}
		self.located(uri).map(|(session, file)| session.tree(file))
	}

	fn diagnostics(&self, uri: &Url) -> Vec<FileDiagnostic> {
		// the diagnostics of the dependencies of a package's document: like
		// `fa check`, an error of the resolution alone
		let in_package = self
			.documents
			.get(uri)
			.is_some_and(|document| document.package.is_some());
		let resolution =
			self.package_of(uri)
				.filter(|_| in_package)
				.and_then(|(package, module)| {
					let found = package.resolution_diagnostics(&module)?;
					Some((found, package.resolution_error.is_some()))
				});
		let resolution = match resolution {
			Some((found, true)) => return found,
			Some((found, false)) => found,
			None => Vec::new(),
		};
		let Some((session, file)) = self.located(uri) else {
			return resolution;
		};
		// a package's module: the diagnostics of the last complete run
		// (the program's checks included)
		let found = match self.package_of(uri) {
			Some((package, module)) if in_package => package
				.diagnostics(&module)
				.unwrap_or(session.diagnostics(file)),
			_ => session.diagnostics(file),
		};
		let mut converted: Vec<FileDiagnostic> = found
			.iter()
			.filter(|diagnostic| self.show_unsupported || diagnostic.code.id != "AN001")
			.map(|diagnostic| FileDiagnostic {
				severity: match diagnostic.severity {
					fa_analyzer::Severity::Error => Severity::Error,
					fa_analyzer::Severity::Warning => Severity::Warning,
					fa_analyzer::Severity::Hint => Severity::Hint,
				},
				// a syntax error has no code worth showing
				code: (!matches!(diagnostic.code.id, "AN002" | "AN003"))
					.then(|| diagnostic.code.id.to_string()),
				message: diagnostic.message.clone(),
				span: span(diagnostic.location.span),
				related: diagnostic
					.related
					.iter()
					.filter(|(location, _)| location.file == file)
					.map(|(location, message)| RelatedInformation {
						uri: uri.clone(),
						span: span(location.span),
						message: message.clone(),
					})
					.collect(),
				// unreachable code (values brief §2.5)
				unnecessary: diagnostic.code.id == "W-DO-2",
			})
			.collect();
		converted.extend(resolution);
		converted
	}

	fn saved(&mut self, uri: &Url) -> Vec<Url> {
		let Ok(path) = uri.to_file_path() else {
			return Vec::new();
		};
		if !is_configuration(&path) {
			return Vec::new();
		}
		// the packages it configures: its own, or every package of a workspace below it
		let folder = path.parent().unwrap_or(Path::new("")).to_path_buf();
		let roots: Vec<PathBuf> = self
			.packages
			.keys()
			.filter(|root| root.starts_with(&folder))
			.cloned()
			.collect();
		let mut changed = Vec::new();
		for root in roots {
			let Some(old) = self.packages.remove(&root) else {
				continue;
			};
			let mut package = Package::new(&root, old.texts, &self.cancel);
			package.run(None);
			self.packages.insert(root.clone(), package);
			changed.extend(
				self.documents
					.iter()
					.filter(|(_, document)| document.package.as_ref() == Some(&root))
					.map(|(uri, _)| uri.clone()),
			);
		}
		changed.sort();
		changed
	}

	fn hover(&self, uri: &Url, offset: usize) -> Option<HoverInfo> {
		let hover = self.session_of(uri).hover(self.file(uri)?, offset as u32)?;
		Some(HoverInfo {
			span: Some(span(hover.span)),
			markdown: hover.markdown,
		})
	}

	fn definition(&self, uri: &Url, offset: usize) -> Option<DefinitionTarget> {
		let (session, file) = self.located(uri)?;
		let definition = session.definition(file, offset as u32)?;
		// a declaration of another file of the package: of the unit's
		// session, or of a file whose tree is dropped (its interface)
		let target = match definition.module {
			Some(module) => {
				let path = self.file_path(uri, session, session.types().module_path(module))?;
				Url::from_file_path(path).ok()?
			}
			None if definition.file == file => uri.clone(),
			None => {
				let path = self.file_path(uri, session, session.module(definition.file))?;
				Url::from_file_path(path).ok()?
			}
		};
		Some(DefinitionTarget {
			uri: target,
			span: span(definition.span),
			name_span: span(definition.name_span),
		})
	}

	fn completions(&self, uri: &Url, offset: usize) -> Vec<CompletionInfo> {
		let Some(file) = self.file(uri) else {
			return Vec::new();
		};
		self.session_of(uri)
			.completions(file, offset as u32)
			.into_iter()
			.map(|completion| CompletionInfo {
				label: completion.label,
				kind: match completion.kind {
					CompletionKind::Value => CompletionItemKind::CONSTANT,
					CompletionKind::Variable => CompletionItemKind::VARIABLE,
					CompletionKind::Function => CompletionItemKind::FUNCTION,
					CompletionKind::Type => CompletionItemKind::STRUCT,
					CompletionKind::Namespace => CompletionItemKind::MODULE,
					CompletionKind::Method => CompletionItemKind::METHOD,
					CompletionKind::Field => CompletionItemKind::FIELD,
				},
				detail: completion.detail,
			})
			.collect()
	}

	fn cancel_flag(&self) -> Option<Arc<AtomicBool>> {
		Some(Arc::clone(&self.cancel))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn uri(name: &str) -> Url {
		Url::parse(&format!("file:///project/{name}")).unwrap()
	}

	fn backend_with(text: &str) -> AnalyzerBackend {
		let mut backend = AnalyzerBackend::new();
		assert_eq!(
			backend.set_document(&uri("main.fa"), Arc::from(text)),
			vec![uri("main.fa")]
		);
		backend
	}

	#[test]
	fn diagnostics_have_codes_and_byte_spans() {
		let backend = backend_with("let a = 1\nlet b: Int8 = 300\n");
		let diagnostics = backend.diagnostics(&uri("main.fa"));
		assert_eq!(diagnostics.len(), 1);
		assert_eq!(diagnostics[0].code.as_deref(), Some("E-NUM-6"));
		assert_eq!(diagnostics[0].severity, Severity::Error);
		assert_eq!(diagnostics[0].span, SourceSpan::new(24, 27));
		assert!(backend.diagnostics(&uri("other.fa")).is_empty());
	}

	#[test]
	fn syntax_errors_have_no_code_and_unsupported_constructs_are_hidden() {
		let backend = backend_with("let a = (1\n");
		let diagnostics = backend.diagnostics(&uri("main.fa"));
		assert!(!diagnostics.is_empty());
		assert!(
			diagnostics
				.iter()
				.all(|diagnostic| diagnostic.code.is_none())
		);
		let mut backend = backend_with("let x = f with (y = g)\n");
		assert!(backend.diagnostics(&uri("main.fa")).is_empty());
		backend.show_unsupported = true;
		assert_eq!(backend.diagnostics(&uri("main.fa")).len(), 1);
	}

	#[test]
	fn queries_answer_with_byte_offsets() {
		let backend = backend_with("let answer = 42\nconsole.log(answer)\n");
		let hover = backend.hover(&uri("main.fa"), 30).unwrap();
		assert_eq!(hover.markdown, "```fa\nlet answer: Integer\n```");
		assert_eq!(hover.span, Some(SourceSpan::new(28, 34)));
		let definition = backend.definition(&uri("main.fa"), 30).unwrap();
		assert_eq!(definition.uri, uri("main.fa"));
		assert_eq!(definition.name_span, SourceSpan::new(4, 10));
		let completions = backend.completions(&uri("main.fa"), 16);
		let answer = completions
			.iter()
			.find(|completion| completion.label == "answer")
			.unwrap();
		assert_eq!(answer.kind, CompletionItemKind::CONSTANT);
		assert_eq!(answer.detail.as_deref(), Some("Integer"));
		assert!(backend.hover(&uri("other.fa"), 0).is_none());
	}

	#[test]
	fn versions_replace_each_other_and_close_forgets() {
		let mut backend = backend_with("let a: Int8 = 300\n");
		// a folder that isn't on the disk: the shared session
		assert!(backend.standalone.is_empty() && backend.folders.is_empty());
		assert_eq!(backend.diagnostics(&uri("main.fa")).len(), 1);
		backend.set_document(&uri("main.fa"), Arc::from("let a: Int8 = 3\n"));
		assert!(backend.diagnostics(&uri("main.fa")).is_empty());
		assert_eq!(
			backend.tree(&uri("main.fa")).unwrap().source.as_ref(),
			"let a: Int8 = 3\n"
		);
		assert!(backend.close_document(&uri("main.fa")).is_empty());
		assert!(backend.tree(&uri("main.fa")).is_none());
		assert!(backend.cancel_flag().is_some());
	}

	#[test]
	fn the_session_is_rebuilt_when_its_types_grow() {
		let mut backend = backend_with("let a = 1\n");
		backend.rebuild_minimum = 0;
		let flag = backend.cancel_flag().unwrap();
		// many distinct literal types grow the store
		let text: String = (0..backend.session().types().len() * 4)
			.map(|index| format!("let v{index}: Int64 = {index}\n"))
			.collect();
		backend.set_document(&uri("big.fa"), Arc::from(text.as_str()));
		assert!(
			backend.session().file_count() <= 2,
			"rebuilt from the open documents"
		);
		assert!(backend.diagnostics(&uri("main.fa")).is_empty());
		assert!(Arc::ptr_eq(&flag, &backend.cancel_flag().unwrap()));
		assert!(backend.hover(&uri("main.fa"), 4).is_some());
	}

	#[test]
	fn a_package_is_analyzed_as_a_whole() {
		let directory = tempfile::TempDir::new().unwrap();
		let root = directory.path();
		for (path, text) in [
			("package.fa", "export = { name = \"p\" }\n"),
			("library/answer.fa", "export function = (): Integer => 42\n"),
			(
				"application/main.fa",
				"export function = () => {\n\tlet value: Integer = library.answer()\n\tconsole.log(value)\n}\n",
			),
		] {
			let path = root.join(path);
			std::fs::create_dir_all(path.parent().unwrap()).unwrap();
			std::fs::write(path, text).unwrap();
		}
		let main = Url::from_file_path(root.join("application/main.fa")).unwrap();
		let answer = Url::from_file_path(root.join("library/answer.fa")).unwrap();
		let mut backend = AnalyzerBackend::new();
		let text = std::fs::read_to_string(root.join("application/main.fa")).unwrap();
		backend.set_document(&main, Arc::from(text.as_str()));
		assert!(backend.diagnostics(&main).is_empty());
		// the definition is in another file
		let offset = text.find("answer()").unwrap() + 1;
		let definition = backend.definition(&main, offset).unwrap();
		assert_eq!(definition.uri, answer);
		// an edit that changes the interface republishes the files using it
		let changed = backend.set_document(
			&answer,
			Arc::from("export function = (): String => \"x\"\n"),
		);
		assert!(changed.contains(&main), "{changed:?}");
		let diagnostics = backend.diagnostics(&main);
		assert_eq!(diagnostics.len(), 1);
		assert_eq!(diagnostics[0].code.as_deref(), Some("E-TYPE-1"));
		// closing the edited file without saving it: the disk's text again,
		// and the files using it are analyzed again
		let changed = backend.close_document(&answer);
		assert_eq!(changed, vec![main.clone()]);
		assert!(backend.diagnostics(&main).is_empty());
	}

	/// A file of a package that its scan doesn't read (a note, a build
	/// output) is a standalone file: it doesn't load the package.
	#[test]
	fn a_file_outside_the_sources_of_a_package_is_standalone() {
		let directory = tempfile::TempDir::new().unwrap();
		let root = directory.path().canonicalize().unwrap();
		write_files(
			&root,
			&[
				("package.fa", "export = { name = \"p\" }\n"),
				("library/answer.fa", "export function = (): Integer => 42\n"),
				("notes/draft.fa", "let a: Integer = \"x\"\n"),
			],
		);
		let draft = Url::from_file_path(root.join("notes/draft.fa")).unwrap();
		let mut backend = AnalyzerBackend::new();
		for _ in 0..2 {
			let analyzed = backend.set_document(&draft, Arc::from("let a: Integer = \"x\"\n"));
			assert_eq!(analyzed, vec![draft.clone()]);
		}
		assert!(backend.package(&root).is_none(), "the package isn't loaded");
		let diagnostics = backend.diagnostics(&draft);
		assert_eq!(diagnostics.len(), 1);
		assert_eq!(diagnostics[0].code.as_deref(), Some("E-TYPE-1"));
		// a source of the package still is one
		let answer = Url::from_file_path(root.join("library/answer.fa")).unwrap();
		backend.set_document(&answer, Arc::from("export function = (): Integer => 42\n"));
		assert!(backend.package(&root).is_some());
	}

	/// Writes files below a folder.
	fn write_files(root: &Path, files: &[(&str, &str)]) {
		for (path, text) in files {
			let path = root.join(path);
			std::fs::create_dir_all(path.parent().unwrap()).unwrap();
			std::fs::write(path, text).unwrap();
		}
	}

	#[test]
	fn closed_files_keep_their_interfaces_only() {
		let directory = tempfile::TempDir::new().unwrap();
		let root = directory.path().canonicalize().unwrap();
		write_files(
			&root,
			&[
				("package.fa", "export = { name = \"p\" }\n"),
				("library/a.fa", "export function = (): Integer => 42\n"),
				("library/b.fa", "-- b\n\nexport function = () => a()\n"),
				(
					"application/main.fa",
					"export function = () => {\n\tlet value: Integer = library.b()\n\tconsole.log(value)\n}\n",
				),
			],
		);
		let main = Url::from_file_path(root.join("application/main.fa")).unwrap();
		let a = Url::from_file_path(root.join("library/a.fa")).unwrap();
		let b = Url::from_file_path(root.join("library/b.fa")).unwrap();
		let text = std::fs::read_to_string(root.join("application/main.fa")).unwrap();
		let mut backend = AnalyzerBackend::new();
		backend.set_document(&main, Arc::from(text.as_str()));
		assert!(backend.diagnostics(&main).is_empty());
		// only the open document's unit keeps its tree
		assert_eq!(backend.package(&root).unwrap().kept_trees(), 1);
		assert!(backend.tree(&b).is_none());
		// the definition in a closed file: the spans of its interface
		let offset = text.find("b()").unwrap();
		let definition = backend.definition(&main, offset).unwrap();
		assert_eq!(definition.uri, b);
		assert_eq!(definition.span.start, 6);
		// an edit of a file two modules away: the interface of `b` changes
		// too, so `main` is analyzed again
		let a_text = "export function = (): String => \"x\"\n";
		let changed = backend.set_document(&a, Arc::from(a_text));
		assert!(changed.contains(&main), "{changed:?}");
		let codes: Vec<_> = backend
			.diagnostics(&main)
			.into_iter()
			.filter_map(|diagnostic| diagnostic.code)
			.collect();
		assert_eq!(codes, ["E-TYPE-1"]);
		assert_eq!(backend.package(&root).unwrap().kept_trees(), 2);
		// a body change keeps the interface: only the edited file
		let changed =
			backend.set_document(&a, Arc::from("export function = (): String => \"y\"\n"));
		assert_eq!(changed, std::slice::from_ref(&a));
		// closing the document drops its tree
		backend.close_document(&a);
		assert!(backend.tree(&a).is_none());
		assert!(backend.tree(&main).is_some());
	}

	#[test]
	fn the_dependencies_of_a_package_are_resolved() {
		let directory = tempfile::TempDir::new().unwrap();
		let root = directory.path().canonicalize().unwrap();
		write_files(
			&root,
			&[
				("workspace.fa", "export = { name = \"w\" }\n"),
				(
					"packages/shared/package.fa",
					"export = { name = \"shared\" }\n",
				),
				(
					"packages/shared/library/greet.fa",
					"export function = (name: String): String => \"hello {name}\"\n",
				),
				("packages/app/package.fa", "export = { name = \"app\" }\n"),
				(
					"packages/app/dependencies.fa",
					"export = {\n\tshared = \"../shared\"\n}\n",
				),
				(
					"packages/app/application/main.fa",
					"export function = () => {\n\tconsole.log(shared.greet(\"you\"))\n}\n",
				),
			],
		);
		let main = Url::from_file_path(root.join("packages/app/application/main.fa")).unwrap();
		let text = std::fs::read_to_string(root.join("packages/app/application/main.fa")).unwrap();
		let mut backend = AnalyzerBackend::new();
		backend.set_document(&main, Arc::from(text.as_str()));
		assert!(
			backend.diagnostics(&main).is_empty(),
			"{:?}",
			backend.diagnostics(&main)
		);
		// the definition is in the dependency's file
		let offset = text.find("greet").unwrap() + 1;
		let definition = backend.definition(&main, offset).unwrap();
		assert_eq!(
			definition.uri,
			Url::from_file_path(root.join("packages/shared/library/greet.fa")).unwrap()
		);
		assert!(
			!root.join("dependencies.lock").exists(),
			"the lockfile is never written"
		);
	}

	/// A standalone file sees the files next to it, like `fa check` does;
	/// an open sibling's text replaces the disk's.
	#[test]
	fn a_standalone_file_sees_its_siblings() {
		let directory = tempfile::TempDir::new().unwrap();
		let root = directory.path().canonicalize().unwrap();
		write_files(
			&root,
			&[
				("helper.fa", "export function = (): Integer => 42\n"),
				(
					"main.fa",
					"let value: Integer = helper()\nconsole.log(value)\n",
				),
			],
		);
		let main = Url::from_file_path(root.join("main.fa")).unwrap();
		let helper = Url::from_file_path(root.join("helper.fa")).unwrap();
		let text = std::fs::read_to_string(root.join("main.fa")).unwrap();
		let mut backend = AnalyzerBackend::new();
		assert_eq!(
			backend.set_document(&main, Arc::from(text.as_str())),
			vec![main.clone()]
		);
		assert!(
			backend.diagnostics(&main).is_empty(),
			"{:?}",
			backend.diagnostics(&main)
		);
		// the definition is in the sibling file
		let offset = text.find("helper").unwrap() + 1;
		let definition = backend.definition(&main, offset).unwrap();
		assert_eq!(definition.uri, helper);
		// an unsaved edit of the sibling: the file reading it is analyzed again
		let changed = backend.set_document(
			&helper,
			Arc::from("export function = (): String => \"x\"\n"),
		);
		assert_eq!(changed, vec![helper.clone(), main.clone()]);
		let codes: Vec<_> = backend
			.diagnostics(&main)
			.into_iter()
			.filter_map(|diagnostic| diagnostic.code)
			.collect();
		assert_eq!(codes, ["E-TYPE-1"]);
		// closed without saving: the disk's text again
		assert_eq!(backend.close_document(&helper), vec![main.clone()]);
		assert!(backend.diagnostics(&main).is_empty());
		assert!(backend.close_document(&main).is_empty());
		assert!(backend.folders.is_empty());
	}

	#[test]
	fn module_names_come_from_the_file_name() {
		assert_eq!(AnalyzerBackend::module(&uri("math.fa")), "math");
		assert_eq!(
			AnalyzerBackend::module(&Url::parse("untitled:Untitled-1").unwrap()),
			"Untitled-1"
		);
	}
}
