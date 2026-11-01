//! The analysis session: files, their trees and analyses, the type store.
//!
//! A file is parsed and its parse diagnostics are converted at the boundary;
//! [`Session::analyze_file`] runs the walk (`check/`), which fills the rest of
//! its [`Analysis`].

use std::sync::Arc;

use fa_parser::TypedSyntaxTree;

use crate::analysis::Analysis;
use crate::diagnostics::{Diagnostic, Location, SourceSpan, codes};
use crate::ids::{FileId, NodeId};
use crate::types::TypeStore;

/// One parse of one file and its analysis.
pub struct SourceFile {
	/// The module path (`main` for a standalone file).
	pub module: String,
	/// The parse tree, shared with the walk.
	pub tree: Arc<TypedSyntaxTree>,
	/// The side tables of the analysis.
	pub analysis: Analysis,
	/// Whether the walk ran.
	pub analyzed: bool,
}

/// Everything the analysis of a program shares: files and types.
#[derive(Default)]
pub struct Session {
	/// The types of every file.
	pub types: TypeStore,
	files: Vec<SourceFile>,
}

impl Session {
	/// An empty session.
	pub fn new() -> Self {
		Self::default()
	}

	/// Parses a version of a module's text: a new [`FileId`] per version.
	pub fn set_source(&mut self, module: &str, text: Arc<str>) -> FileId {
		let file = FileId::new(self.files.len());
		let tree = fa_parser::parse_shared(text);
		let mut analysis = Analysis::new(file, tree.nodes.len());
		analysis.diagnostics = tree
			.diagnostics
			.iter()
			.map(|diagnostic| convert(file, diagnostic))
			.collect();
		self.files.push(SourceFile {
			module: module.to_string(),
			tree: Arc::new(tree),
			analysis,
			analyzed: false,
		});
		file
	}

	/// Analyzes a file (once: the result is memoized).
	pub fn analyze_file(&mut self, file: FileId) -> &Analysis {
		if !self.files[file.index()].analyzed {
			self.files[file.index()].analyzed = true;
			crate::check::analyze(self, file);
		}
		&self.files[file.index()].analysis
	}

	/// The tree of a file and its analysis, taken out for the walk
	/// (put back with [`Session::put_analysis`]).
	pub(crate) fn take_analysis(&mut self, file: FileId) -> (Arc<TypedSyntaxTree>, Analysis) {
		let source = &mut self.files[file.index()];
		let empty = Analysis::new(file, 0);
		(
			Arc::clone(&source.tree),
			std::mem::replace(&mut source.analysis, empty),
		)
	}

	/// Puts back the analysis taken by [`Session::take_analysis`].
	pub(crate) fn put_analysis(&mut self, file: FileId, analysis: Analysis) {
		self.files[file.index()].analysis = analysis;
	}

	/// The module path of a file.
	pub fn module(&self, file: FileId) -> &str {
		&self.files[file.index()].module
	}

	/// The analysis of a file.
	pub fn analysis(&self, file: FileId) -> &Analysis {
		&self.files[file.index()].analysis
	}

	/// The diagnostics of a file.
	pub fn diagnostics(&self, file: FileId) -> &[Diagnostic] {
		&self.analysis(file).diagnostics
	}

	/// The parse tree of a file.
	pub fn tree(&self, file: FileId) -> &TypedSyntaxTree {
		&self.files[file.index()].tree
	}

	/// The text of a file.
	pub fn text(&self, file: FileId) -> &str {
		&self.tree(file).source
	}

	/// The span of a node.
	pub fn span(&self, file: FileId, node: NodeId) -> SourceSpan {
		span_of(self.tree(file).spans[node.index()])
	}

	/// The type store.
	pub fn types(&self) -> &TypeStore {
		&self.types
	}

	/// Prints a type.
	pub fn display_type(&self, ty: crate::ids::TypeId) -> String {
		self.types.display(ty)
	}
}

/// Analyzes a standalone file: tests, `fa run x.fa`.
pub fn analyze_source(text: &str) -> (Session, FileId) {
	let mut session = Session::new();
	let file = session.set_source("main", Arc::from(text));
	session.analyze_file(file);
	(session, file)
}

fn span_of(span: fa_parser::SourceSpan) -> SourceSpan {
	SourceSpan::new(span.start as u32, span.end as u32)
}

/// A parse diagnostic as an analyzer diagnostic (`AN002` / `AN003`).
fn convert(file: FileId, diagnostic: &fa_parser::Diagnostic) -> Diagnostic {
	let code = match diagnostic.severity {
		fa_parser::Severity::Error => &codes::analyzer::AN002,
		fa_parser::Severity::Warning => &codes::analyzer::AN003,
	};
	let location = |span| Location {
		file,
		span: span_of(span),
	};
	let mut converted =
		Diagnostic::new(code, location(diagnostic.span), diagnostic.message.clone());
	for (span, note) in &diagnostic.related {
		converted = converted.with_related(location(*span), note.clone());
	}
	converted
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Severity;

	#[test]
	fn a_valid_file_has_no_diagnostic() {
		let (session, file) = analyze_source("let answer = 42\nconsole.log(answer)\n");
		assert!(session.diagnostics(file).is_empty());
		assert!(session.tree(file).nodes.len() > 3);
		assert_eq!(
			session.analysis(file).types.len(),
			session.tree(file).nodes.len()
		);
	}

	#[test]
	fn parse_errors_become_an002() {
		let (session, file) = analyze_source("let = = (\n");
		let diagnostics = session.diagnostics(file);
		assert!(!diagnostics.is_empty());
		assert!(diagnostics.iter().all(|d| d.code.id.starts_with("AN00")));
		assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
	}

	#[test]
	fn every_version_gets_a_new_file() {
		let mut session = Session::new();
		let a = session.set_source("main", Arc::from("let a = 1"));
		let b = session.set_source("main", Arc::from("let a = 2"));
		assert_ne!(a, b);
		assert_eq!(session.text(b), "let a = 2");
	}
}
