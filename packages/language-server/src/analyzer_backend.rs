//! The analyzer behind the language server: an `AnalysisBackend` over a
//! `fa_analyzer::Session`.
//!
//! A document inside a package (a folder with a `package.fa` above it) is a
//! file of that package's session: the package is scanned and analyzed when
//! its first document opens (the other files are read from the disk), and an
//! edit re-analyzes the file, then the files that use what changed
//! (`Session::update`), whose diagnostics are published too. Any other
//! document is a standalone file of a shared session. A new text is a new
//! version of the file: the previous version is forgotten. The standalone
//! session's type store only grows, so it is rebuilt from the open documents
//! when its store has grown to four times its size after the last rebuild
//! (design §4, "epoch reset").

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use fa_analyzer::ide::CompletionKind;
use fa_analyzer::ids::FileId;
use fa_analyzer::{CodebaseResolver, Session};
use fa_parser::{SourceSpan, TypedSyntaxTree};
use tower_lsp::lsp_types::{CompletionItemKind, Url};

use crate::backend::{
	AnalysisBackend, CompletionInfo, DefinitionTarget, FileDiagnostic, HoverInfo,
	RelatedInformation, Severity,
};

/// The default size of the type store below which the session is never rebuilt.
const MINIMUM_REBUILD_SIZE: usize = 50_000;

/// An open document: its file in the session, and its text (to rebuild the session).
struct OpenDocument {
	file: FileId,
	text: Arc<str>,
	/// The root of its package, when it belongs to one.
	package: Option<PathBuf>,
}

/// The package folder containing a file, if any.
fn find_package(path: &Path) -> Option<PathBuf> {
	let mut current = path.parent();
	while let Some(directory) = current {
		if directory.join("package.fa").is_file() {
			return Some(directory.to_path_buf());
		}
		current = directory.parent();
	}
	None
}

/// The module of a file of a package (`library/math`).
fn package_module(root: &Path, path: &Path) -> Option<String> {
	let relative = path.strip_prefix(root).ok()?;
	let text = relative.to_string_lossy().replace('\\', "/");
	Some(text.strip_suffix(".fa").unwrap_or(&text).to_string())
}

/// A session over the files of a package.
fn package_session(root: &Path, cancel: &Arc<AtomicBool>) -> Session {
	let mut session = Session::new();
	session.set_cancel_flag(Arc::clone(cancel));
	if let Ok(codebase) = fa_codebase_parser::Codebase::scan_package(root) {
		session.set_resolver(Box::new(CodebaseResolver::new(
			codebase,
			Some(root.to_path_buf()),
		)));
		let entry = session
			.file_of_module("application/main")
			.map(|_| "application/main");
		session.analyze_package(entry);
	}
	session
}

/// The analyzer as a language server backend.
pub struct AnalyzerBackend {
	session: Session,
	/// The session of each package with an open document, by root folder.
	packages: HashMap<PathBuf, Session>,
	documents: HashMap<Url, OpenDocument>,
	/// The size of the type store after the last rebuild.
	baseline: usize,
	/// The size of the type store below which the session is never rebuilt.
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
			baseline: session.types().len(),
			session,
			packages: HashMap::new(),
			documents: HashMap::new(),
			rebuild_minimum: MINIMUM_REBUILD_SIZE,
			cancel,
			show_unsupported: false,
		}
	}

	/// The analyzer's session, for tests and tools.
	pub fn session(&self) -> &Session {
		&self.session
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
		let size = self.session.types().len();
		if size < self.rebuild_minimum || size < self.baseline * 4 {
			return;
		}
		let mut session = Session::new();
		session.set_cancel_flag(Arc::clone(&self.cancel));
		self.session = session;
		let documents: Vec<(Url, Arc<str>)> = self
			.documents
			.iter()
			.map(|(uri, document)| (uri.clone(), Arc::clone(&document.text)))
			.collect();
		for (uri, text) in documents {
			if self
				.documents
				.get(&uri)
				.is_some_and(|document| document.package.is_some())
			{
				continue;
			}
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
		self.baseline = self.session.types().len();
	}

	fn file(&self, uri: &Url) -> Option<FileId> {
		self.located(uri).map(|(_, file)| file)
	}

	/// The session and the current file of a document, or of a file of an
	/// open package (its diagnostics are published when an edit changes them).
	fn located(&self, uri: &Url) -> Option<(&Session, FileId)> {
		if let Some(document) = self.documents.get(uri)
			&& document.package.is_none()
		{
			return Some((&self.session, document.file));
		}
		let path = uri.to_file_path().ok()?;
		let (root, session) = self
			.packages
			.iter()
			.find(|(root, _)| path.starts_with(root))?;
		let module = package_module(root, &path)?;
		Some((session, session.loaded_module(&module)?))
	}

	/// The session of a document.
	fn session_of(&self, uri: &Url) -> &Session {
		self.located(uri)
			.map_or(&self.session, |(session, _)| session)
	}

	/// The URI of a file of a session (a package's, or the standalone one).
	fn uri_of(&self, uri: &Url, file: FileId) -> Option<Url> {
		let path = uri.to_file_path().ok()?;
		let (root, session) = self
			.packages
			.iter()
			.find(|(root, _)| path.starts_with(root))?;
		Url::from_file_path(root.join(session.display_path(file))).ok()
	}

	/// A new text of a document of a package: the package's session
	/// re-analyzes it and the files using what changed; their URIs.
	fn set_package_document(&mut self, uri: &Url, root: PathBuf, text: Arc<str>) -> Vec<Url> {
		let Some(module) = uri
			.to_file_path()
			.ok()
			.and_then(|path| package_module(&root, &path))
		else {
			return Vec::new();
		};
		let cancel = Arc::clone(&self.cancel);
		let known = self
			.packages
			.get(&root)
			.is_some_and(|session| session.loaded_module(&module).is_some());
		if !known {
			// a new package, or a file the last scan didn't see: scan again
			self.packages
				.insert(root.clone(), package_session(&root, &cancel));
		}
		let session = self.packages.get_mut(&root).expect("inserted above");
		let changed = session.update(&module, Arc::clone(&text));
		let file = session.loaded_module(&module).expect("updated above");
		let urls: Vec<Url> = changed
			.iter()
			.filter_map(|&file| Url::from_file_path(root.join(session.display_path(file))).ok())
			.collect();
		self.documents.insert(
			uri.clone(),
			OpenDocument {
				file,
				text,
				package: Some(root),
			},
		);
		let mut urls = urls;
		if !urls.contains(uri) {
			urls.insert(0, uri.clone());
		}
		urls
	}
}

fn span(span: fa_analyzer::diagnostics::SourceSpan) -> SourceSpan {
	SourceSpan::new(span.start as usize, span.end as usize)
}

impl AnalysisBackend for AnalyzerBackend {
	fn set_document(&mut self, uri: &Url, text: Arc<str>) -> Vec<Url> {
		// a file of a package: the package's session (workspace folder = package)
		if let Some(root) = uri.to_file_path().ok().and_then(|path| find_package(&path)) {
			return self.set_package_document(uri, root, text);
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
		if let Some(document) = self.documents.remove(uri)
			&& document.package.is_none()
		{
			self.session.forget(document.file);
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
		let Some((session, file)) = self.located(uri) else {
			return Vec::new();
		};
		session
			.diagnostics(file)
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
			.collect()
	}

	fn hover(&self, uri: &Url, offset: usize) -> Option<HoverInfo> {
		let hover = self.session_of(uri).hover(self.file(uri)?, offset as u32)?;
		Some(HoverInfo {
			span: Some(span(hover.span)),
			markdown: hover.markdown,
		})
	}

	fn definition(&self, uri: &Url, offset: usize) -> Option<DefinitionTarget> {
		let file = self.file(uri)?;
		let definition = self.session_of(uri).definition(file, offset as u32)?;
		// a declaration of another file of the package
		let target = if definition.file == file {
			uri.clone()
		} else {
			self.uri_of(uri, definition.file)?
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
		let mut backend = backend_with("let x = f then g\n");
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
