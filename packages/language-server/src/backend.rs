//! The boundary between the language server and the code that understands Fa.
//!
//! The language server keeps the documents in sync with the editor and converts positions; an
//! `AnalysisBackend` parses and analyzes their text, and answers the queries with byte offsets.
//! Today the backend is `ParseBackend`, which only parses; the analyzer's `Session` implements
//! the same trait to add the type checker's diagnostics, hover, definition and completions.
//!
//! The backend lives on the analysis thread (see `worker.rs`): its methods are never called
//! concurrently, and can take `&mut self` without locking.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use fa_parser::{SourceSpan, TypedSyntaxTree};
use tower_lsp::lsp_types::{CompletionItemKind, Url};

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
	Error,
	Warning,
	Information,
	Hint,
}

/// A diagnostic of a file, with byte spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiagnostic {
	pub severity: Severity,
	/// A stable code, like `E-NUM-11` (a plain syntax error has none).
	pub code: Option<String>,
	pub message: String,
	pub span: SourceSpan,
	/// Other places that explain the problem, possibly in other files.
	pub related: Vec<RelatedInformation>,
	/// Code that is unused or unreachable: editors fade it out.
	pub unnecessary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
	pub uri: Url,
	pub span: SourceSpan,
	pub message: String,
}

/// The documentation of the code under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
	/// The code the hover is about (highlighted by the editor).
	pub span: Option<SourceSpan>,
	pub markdown: String,
}

/// Where a name is declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionTarget {
	pub uri: Url,
	/// The whole declaration.
	pub span: SourceSpan,
	/// The declared name, inside `span`.
	pub name_span: SourceSpan,
}

/// A suggestion of completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionInfo {
	pub label: String,
	pub kind: CompletionItemKind,
	/// A short description, like the type of the value.
	pub detail: Option<String>,
}

/// What the language server needs from the code that understands Fa. Offsets are byte offsets
/// into the text given to `set_document`.
pub trait AnalysisBackend: Send + 'static {
	/// Set the text of a document (opened, or changed after the debounce delay), parse and
	/// analyze it. Return the documents whose diagnostics may have changed: this one, and the
	/// ones that depend on it.
	fn set_document(&mut self, uri: &Url, text: Arc<str>) -> Vec<Url>;

	/// Forget a document closed by the editor. Return the other documents whose diagnostics
	/// may have changed.
	fn close_document(&mut self, uri: &Url) -> Vec<Url>;

	/// The syntax tree of a document, for the features that only need the syntax (symbols,
	/// folding, highlighting, selection).
	fn tree(&self, uri: &Url) -> Option<&TypedSyntaxTree>;

	/// Every diagnostic of a document.
	fn diagnostics(&self, uri: &Url) -> Vec<FileDiagnostic>;

	fn hover(&self, _uri: &Url, _offset: usize) -> Option<HoverInfo> {
		None
	}

	fn definition(&self, _uri: &Url, _offset: usize) -> Option<DefinitionTarget> {
		None
	}

	/// The names that can be written at an offset (the keywords are added by the server).
	fn completions(&self, _uri: &Url, _offset: usize) -> Vec<CompletionInfo> {
		Vec::new()
	}

	/// A flag that stops a running analysis when set, checked by the backend between
	/// declarations. The server sets it when an edit arrives, and the analysis thread clears
	/// it before calling `set_document`.
	fn cancel_flag(&self) -> Option<Arc<AtomicBool>> {
		None
	}
}

/// The diagnostic of the parser, as a `FileDiagnostic` of `uri`.
pub fn parser_diagnostic(uri: &Url, diagnostic: &fa_parser::Diagnostic) -> FileDiagnostic {
	FileDiagnostic {
		severity: match diagnostic.severity {
			fa_parser::Severity::Error => Severity::Error,
			fa_parser::Severity::Warning => Severity::Warning,
		},
		code: diagnostic.code.map(str::to_string),
		message: diagnostic.message.clone(),
		span: diagnostic.span,
		related: diagnostic
			.related
			.iter()
			.map(|(span, message)| RelatedInformation {
				uri: uri.clone(),
				span: *span,
				message: message.clone(),
			})
			.collect(),
		unnecessary: false,
	}
}

/// A backend that only parses: its diagnostics are the syntax errors.
#[derive(Debug, Default)]
pub struct ParseBackend {
	trees: HashMap<Url, TypedSyntaxTree>,
}

impl AnalysisBackend for ParseBackend {
	fn set_document(&mut self, uri: &Url, text: Arc<str>) -> Vec<Url> {
		self.trees
			.insert(uri.clone(), fa_parser::parse_shared(text));
		vec![uri.clone()]
	}

	fn close_document(&mut self, uri: &Url) -> Vec<Url> {
		self.trees.remove(uri);
		Vec::new()
	}

	fn tree(&self, uri: &Url) -> Option<&TypedSyntaxTree> {
		self.trees.get(uri)
	}

	fn diagnostics(&self, uri: &Url) -> Vec<FileDiagnostic> {
		let Some(tree) = self.trees.get(uri) else {
			return Vec::new();
		};
		tree.diagnostics
			.iter()
			.map(|diagnostic| parser_diagnostic(uri, diagnostic))
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn uri(name: &str) -> Url {
		Url::parse(&format!("file:///project/{name}")).unwrap()
	}

	#[test]
	fn parse_backend_reports_syntax_errors() {
		let mut backend = ParseBackend::default();
		let changed = backend.set_document(&uri("a.fa"), Arc::from("let a = (1"));
		assert_eq!(changed, vec![uri("a.fa")]);
		let diagnostics = backend.diagnostics(&uri("a.fa"));
		assert!(!diagnostics.is_empty());
		assert!(
			diagnostics
				.iter()
				.all(|diagnostic| diagnostic.severity == Severity::Error)
		);
	}

	#[test]
	fn parse_backend_has_no_diagnostics_for_valid_code() {
		let mut backend = ParseBackend::default();
		backend.set_document(&uri("a.fa"), Arc::from("let a = 1\nlet b = a + 2"));
		assert!(backend.diagnostics(&uri("a.fa")).is_empty());
		assert!(backend.tree(&uri("a.fa")).is_some());
	}

	#[test]
	fn parse_backend_replaces_and_forgets_documents() {
		let mut backend = ParseBackend::default();
		backend.set_document(&uri("a.fa"), Arc::from("let a = (1"));
		backend.set_document(&uri("a.fa"), Arc::from("let a = (1)"));
		assert!(backend.diagnostics(&uri("a.fa")).is_empty());
		assert!(backend.close_document(&uri("a.fa")).is_empty());
		assert!(backend.tree(&uri("a.fa")).is_none());
		assert!(backend.diagnostics(&uri("a.fa")).is_empty());
	}

	#[test]
	fn parse_backend_answers_no_semantic_query() {
		let mut backend = ParseBackend::default();
		backend.set_document(&uri("a.fa"), Arc::from("let a = 1"));
		assert_eq!(backend.hover(&uri("a.fa"), 4), None);
		assert_eq!(backend.definition(&uri("a.fa"), 4), None);
		assert!(backend.completions(&uri("a.fa"), 4).is_empty());
		assert!(backend.cancel_flag().is_none());
	}

	#[test]
	fn related_information_points_to_the_same_file() {
		let diagnostic = fa_parser::Diagnostic::error("unclosed", SourceSpan::new(5, 6))
			.with_related(SourceSpan::new(0, 1), "opened here");
		let converted = parser_diagnostic(&uri("a.fa"), &diagnostic);
		assert_eq!(converted.related.len(), 1);
		assert_eq!(converted.related[0].uri, uri("a.fa"));
		assert_eq!(converted.related[0].span, SourceSpan::new(0, 1));
	}
}
