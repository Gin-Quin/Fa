//! The analysis session: files, their trees and analyses, the type store.
//!
//! A file is parsed and its parse diagnostics are converted at the boundary;
//! [`Session::analyze_file`] runs the walk (`check/`), which fills the rest of
//! its [`Analysis`].

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use fa_parser::TypedSyntaxTree;

use crate::analysis::Analysis;
use crate::diagnostics::{Code, Diagnostic, LineIndex, Location, SourceSpan, codes};
use crate::ids::{FileId, NodeId, TypeId};
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
pub struct Session {
	/// The types of every file.
	pub types: TypeStore,
	files: Vec<SourceFile>,
	/// The type of every built-in (its value type, or the type it denotes),
	/// built once, for the queries that can't intern (hover, completions).
	prelude: Vec<Option<TypeId>>,
	/// Set to stop a running analysis (the language server sets it on an edit).
	cancel: Arc<AtomicBool>,
}

impl Default for Session {
	fn default() -> Self {
		Self::new()
	}
}

impl Session {
	/// An empty session.
	pub fn new() -> Self {
		let mut types = TypeStore::new();
		let prelude = (0..crate::builtins::all().len())
			.map(|index| {
				let id = crate::ids::BuiltinId::new(index);
				crate::builtins::value_type(&mut types, id)
					.or_else(|| crate::builtins::denoted_type(&mut types, id))
			})
			.collect();
		Self {
			types,
			files: Vec::new(),
			prelude,
			cancel: Arc::new(AtomicBool::new(false)),
		}
	}

	/// The type of a built-in: its value type (`console.log`, `arguments`),
	/// or the type it denotes (`Integer`); `None` for a namespace.
	pub fn builtin_type(&self, id: crate::ids::BuiltinId) -> Option<TypeId> {
		self.prelude.get(id.index()).copied().flatten()
	}

	/// The flag that stops a running analysis when set (checked between
	/// declarations). Clear it before analyzing again.
	pub fn cancel_flag(&self) -> Arc<AtomicBool> {
		Arc::clone(&self.cancel)
	}

	/// Shares a flag owned by the caller (the language server keeps one flag
	/// across sessions rebuilt from the open files).
	pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
		self.cancel = flag;
	}

	/// Drops the tree and the analysis of an old version of a file (its
	/// [`FileId`] stays valid, with an empty text).
	pub fn forget(&mut self, file: FileId) {
		if let Some(source) = self.files.get_mut(file.index()) {
			source.tree = Arc::new(fa_parser::parse(""));
			source.analysis = Analysis::new(file, source.tree.nodes.len());
			source.analyzed = true;
		}
	}

	/// The number of files set, versions included.
	pub fn file_count(&self) -> usize {
		self.files.len()
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

/// The code of a parse diagnostic: the code the parser gives it (a rule of
/// the specification it breaks, `E-STR-1`, `E-DST-9`), or `AN002` / `AN003`
/// for a plain syntax error. A code the registry doesn't know is a bug of
/// the parser: it falls back to `AN002` (a unit test checks every code the
/// parser uses).
fn parse_code(diagnostic: &fa_parser::Diagnostic) -> Code {
	if let Some(code) = diagnostic.code.and_then(codes::lookup) {
		return code;
	}
	match diagnostic.severity {
		fa_parser::Severity::Error => &codes::analyzer::AN002,
		fa_parser::Severity::Warning => &codes::analyzer::AN003,
	}
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
		session
			.diagnostics(file)
			.iter()
			.map(|d| d.code.id)
			.collect()
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
			assert!(
				found.contains(&code),
				"{source:?}: expected {code}, found {found:?}"
			);
		}
		assert!(codes_of("let = 1\n").contains(&"AN002"));
		for (source, code) in [
			("let x = .5\n", "E-NUM-1"),
			("let x = 0XFF\n", "E-NUM-3"),
			("let x = 12.5n\n", "E-NUM-4"),
			("let pair = (1, 2)\nlet x = pair.0\n", "E-TUP-4"),
			("let [a = 0] = [1]\n", "E-DST-6"),
			("let (a, ...r) = (1, 2)\n", "E-DST-6"),
			("let { a: first } = { a = 1 }\n", "E-DST-8"),
			("let [...r, last] = [1, 2]\n", "E-DST-9"),
			("mutable a = 1\nmutable b = 2\na = b = 3\n", "E-DECL-8"),
		] {
			let found = codes_of(source);
			assert!(
				found.contains(&code),
				"{source:?}: expected {code}, found {found:?}"
			);
		}
		// the consequences of a lexical error on its line are dropped
		assert_eq!(codes_of("let été = 1\n"), ["E-LEX-2"]);
		assert_eq!(codes_of("let s = 'a'\n"), ["E-STR-13"]);
		assert_eq!(codes_of("if !true { }\n"), ["E-LEX-4"]);
	}

	/// Every code written in the parser's sources (`"E-STR-1"`) is in the registry.
	#[test]
	fn every_code_of_the_parser_is_registered() {
		fn visit(dir: &std::path::Path, found: &mut Vec<String>) {
			for entry in std::fs::read_dir(dir).expect("the parser's sources") {
				let path = entry.expect("an entry").path();
				if path.is_dir() {
					if path.file_name().is_some_and(|name| name != "tests") {
						visit(&path, found);
					}
					continue;
				}
				let text = std::fs::read_to_string(&path).expect("a source file");
				for part in text.split('"').skip(1).step_by(2) {
					let is_code = (part.starts_with("E-") || part.starts_with("W-"))
						&& part.len() <= 12
						&& part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
						&& part.ends_with(|c: char| c.is_ascii_digit());
					if is_code {
						found.push(part.to_string());
					}
				}
			}
		}
		let mut found = Vec::new();
		let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../parser/src");
		visit(&dir, &mut found);
		assert!(
			found.len() > 20,
			"the parser's codes are not found: {found:?}"
		);
		for code in found {
			assert!(codes::lookup(&code).is_some(), "{code} is not registered");
		}
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
