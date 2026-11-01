//! The analysis session: files, their trees and analyses, the type store.
//!
//! A file is parsed and its parse diagnostics are converted at the boundary;
//! [`Session::analyze_file`] runs the walk (`check/`), which fills the rest of
//! its [`Analysis`].

use std::sync::Arc;

use fa_parser::TypedSyntaxTree;

use crate::analysis::Analysis;
use crate::diagnostics::{Code, Diagnostic, LineIndex, Location, SourceSpan, codes};
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
		analysis.diagnostics = convert_all(file, &tree);
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

	/// The diagnostics of a file rendered for the command line, `path` naming
	/// the file (see [`crate::diagnostics::render`]); empty when there are none.
	pub fn render_diagnostics(&self, file: FileId, path: &str) -> String {
		let source = crate::diagnostics::SourceView::new(path, self.text(file));
		crate::diagnostics::render_all(self.diagnostics(file), &source)
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

/// The lexical codes of the values brief (§3, §6.1, §7) that the parser
/// reports, recognized by the start of its message. The parser's diagnostics
/// have no codes yet: when they do, this table goes away. A unit test pins
/// each message, so a reworded one fails the build instead of falling back
/// to `AN002` silently.
static LEXICAL_CODES: &[(&str, Code)] = &[
	("Unknown escape sequence", &codes::values::E_STR_1),
	("A string can't end with a single `\\`", &codes::values::E_STR_1),
	("`\\x` is followed by two hexadecimal digits", &codes::values::E_STR_2),
	("`\\u` is followed by a Unicode scalar value", &codes::values::E_STR_3),
	("A `}` in a string must be doubled", &codes::values::E_STR_4),
	("Empty `{}` in a string", &codes::values::E_STR_5),
	("Strings are written between double quotes", &codes::values::E_STR_13),
	("Statements are separated by new lines", &codes::values::E_LEX_1),
	("Names can only use ASCII letters", &codes::values::E_LEX_2),
	("`!` is not an operator", &codes::values::E_LEX_4),
	("This string is never closed", &codes::values::E_LEX_6),
	("This block comment is never closed", &codes::values::E_LEX_6),
];

/// The code of a parse diagnostic: a lexical code of the values brief, or
/// `AN002` / `AN003`.
fn parse_code(diagnostic: &fa_parser::Diagnostic) -> Code {
	if diagnostic.severity == fa_parser::Severity::Error
		&& let Some((_, code)) = LEXICAL_CODES
			.iter()
			.find(|(start, _)| diagnostic.message.starts_with(start))
	{
		return code;
	}
	if diagnostic.severity == fa_parser::Severity::Error
		&& diagnostic.message.contains("must be on the line of the `}` that closes")
	{
		return &codes::values::E_LEX_5;
	}
	if diagnostic.severity == fa_parser::Severity::Error && is_keyword_as_name(&diagnostic.message)
	{
		return &codes::values::E_LEX_3;
	}
	if diagnostic.message.contains("an underscore must be between two digits") {
		return &codes::values::E_NUM_2;
	}
	match diagnostic.severity {
		fa_parser::Severity::Error => &codes::analyzer::AN002,
		fa_parser::Severity::Warning => &codes::analyzer::AN003,
	}
}

/// The reserved keywords (values brief §3): a name expected but one of these
/// found is `E-LEX-3`.
const KEYWORDS: &[&str] = &[
	"and", "as", "assert", "async", "await", "break", "catch", "continue", "derived", "do",
	"else", "export", "false", "for", "function", "if", "in", "invalidates", "is", "let", "loop",
	"mutable", "mutates", "namespace", "none", "not", "or", "parallel", "provide", "reactive",
	"readonly", "requires", "return", "reuse", "start", "static", "test", "then", "throw",
	"throws", "true", "try", "type", "use", "watch", "when", "while", "with",
];

/// Whether a parse error says a keyword was found where a name was expected.
fn is_keyword_as_name(message: &str) -> bool {
	message.starts_with("Expected a name")
		&& message
			.rsplit_once("found `")
			.and_then(|(_, found)| found.strip_suffix('`'))
			.is_some_and(|found| KEYWORDS.contains(&found))
}

/// The parse diagnostics of a tree as analyzer diagnostics. After a lexical
/// error, the syntax errors of the same line are its consequences: they are
/// dropped, like a second lexical error with the same code on that line.
fn convert_all(file: FileId, tree: &TypedSyntaxTree) -> Vec<Diagnostic> {
	let lines = LineIndex::new(&tree.source);
	let mut lexical_lines: Vec<(u32, Code)> = Vec::new();
	let mut converted = Vec::with_capacity(tree.diagnostics.len());
	for diagnostic in &tree.diagnostics {
		let diagnostic = convert(file, diagnostic);
		let line = lines.line(diagnostic.location.span.start);
		let generic = diagnostic.code.id.starts_with("AN");
		let cascade = lexical_lines
			.iter()
			.any(|&(other, code)| other == line && (generic || code == diagnostic.code));
		if cascade {
			continue;
		}
		if !generic {
			lexical_lines.push((line, diagnostic.code));
		}
		converted.push(diagnostic);
	}
	converted
}

/// A parse diagnostic as an analyzer diagnostic (see [`parse_code`]).
fn convert(file: FileId, diagnostic: &fa_parser::Diagnostic) -> Diagnostic {
	let code = parse_code(diagnostic);
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

	/// The codes of a source's diagnostics.
	fn codes_of(source: &str) -> Vec<&'static str> {
		let (session, file) = analyze_source(source);
		session.diagnostics(file).iter().map(|d| d.code.id).collect()
	}

	#[test]
	fn lexical_errors_get_their_codes() {
		for (source, code) in [
			("let s = \"\\q\"\n", "E-STR-1"),
			("let s = \"\\x80\"\n", "E-STR-2"),
			("let s = \"\\x4\"\n", "E-STR-2"),
			("let s = \"\\u{D800}\"\n", "E-STR-3"),
			("let s = \"\\u{110000}\"\n", "E-STR-3"),
			("let s = \"\\u0041\"\n", "E-STR-3"),
			("let s = \"a } b\"\n", "E-STR-4"),
			("let s = \"a {} b\"\n", "E-STR-5"),
			("let s = 'a'\n", "E-STR-13"),
			("let a = 1; let b = 2\n", "E-LEX-1"),
			("let été = 1\n", "E-LEX-2"),
			("if !true { }\n", "E-LEX-4"),
			("if true {\n}\nelse {\n}\n", "E-LEX-5"),
			("let s = \"abc\n", "E-LEX-6"),
			("--- never closed\n", "E-LEX-6"),
			("let n = 1__0\n", "E-NUM-2"),
			("let loop = 1\n", "E-LEX-3"),
		] {
			let found = codes_of(source);
			assert!(found.contains(&code), "{source:?}: expected {code}, found {found:?}");
		}
		assert!(codes_of("let = 1\n").contains(&"AN002"));
		// the consequences of a lexical error on its line are dropped
		assert_eq!(codes_of("let été = 1\n"), ["E-LEX-2"]);
		assert_eq!(codes_of("let s = 'a'\n"), ["E-STR-13"]);
		assert_eq!(codes_of("if !true { }\n"), ["E-LEX-4"]);
	}

	#[test]
	fn diagnostics_render_for_the_command_line() {
		let (session, file) = analyze_source("let ok = 1\nlet s = \"\\q\"\n");
		assert_eq!(
			session.render_diagnostics(file, "bad.fa"),
			concat!(
				"bad.fa:2:10: error[E-STR-1]: Unknown escape sequence `\\q`: the escapes are ",
				"`\\\\`, `\\\"`, `\\n`, `\\r`, `\\t`, `\\0`, `\\xNN` and `\\u{...}`\n",
				"  2 | let s = \"\\q\"\n",
				"    |          ^^\n",
				"\n",
				"1 error\n"
			)
		);
		let (session, file) = analyze_source("let ok = 1\n");
		assert_eq!(session.render_diagnostics(file, "ok.fa"), "");
	}

	#[test]
	fn crlf_line_breaks_in_strings_are_newlines() {
		let (session, file) = analyze_source("let n = \"First line\r\nSecond line\".length\r\n");
		assert!(session.diagnostics(file).is_empty());
		let tree = session.tree(file);
		let length = (0..tree.nodes.len())
			.find(|&node| tree.text(node).ends_with(".length"))
			.unwrap();
		let ty = session.analysis(file).type_of(NodeId::new(length));
		assert_eq!(session.display_type(ty), "22");
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
