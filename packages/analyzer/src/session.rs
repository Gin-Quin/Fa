//! The analysis session: files, their trees and analyses, the type store.
//!
//! A file is parsed and its parse diagnostics are converted at the boundary;
//! [`Session::analyze_file`] runs the walk (`check/`), which fills the rest of
//! its [`Analysis`].

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rustc_hash::FxHashMap;

use fa_parser::TypedSyntaxTree;

use crate::analysis::Analysis;
use crate::diagnostics::{Code, Diagnostic, LineIndex, Location, SourceSpan, codes};
use crate::ids::{FileId, ModuleId, NodeId, SymbolId, TypeId};
use crate::modules::{FileRole, Interface, ModuleResolver};
use crate::symbols::Resolution;
use crate::types::TypeStore;

/// One parse of one file and its analysis.
pub struct SourceFile {
	/// The module path (`main` for a standalone file).
	pub module: String,
	/// The parse tree, shared with the walk.
	pub tree: Arc<TypedSyntaxTree>,
	/// The side tables of the analysis.
	pub analysis: Analysis,
	/// Whether the walk ran (or is running).
	pub analyzed: bool,
	/// Whether the walk finished.
	pub done: bool,
	/// What the file is (a program, a module, a test file…).
	pub role: FileRole,
	/// What other files see of it (`modules.rs`).
	pub interface: Interface,
	/// Whether its walk read a declaration of a file whose analysis was in
	/// progress and not available yet: the program analysis walks it again.
	pub deferred: bool,
	/// The modules its lookups touched, found or not (incremental analysis).
	pub reads: Vec<String>,
	/// For a configuration file: its value, when it is a valid literal (D37).
	pub configuration: Option<crate::configuration::ConfigValue>,
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
	/// The module system of a multi-file program (`None`: a lone file).
	resolver: Option<Box<dyn ModuleResolver>>,
	/// The current file of each module path.
	modules: FxHashMap<String, FileId>,
	/// The file and declaration of each declared type (methods of another
	/// file's types, `check/modules.rs`).
	type_origins: FxHashMap<TypeId, (FileId, NodeId)>,
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
			resolver: None,
			modules: FxHashMap::default(),
			type_origins: FxHashMap::default(),
		}
	}

	/// Sets the module system: the other files of the program are found and
	/// read through it (design §1).
	pub fn set_resolver(&mut self, resolver: Box<dyn ModuleResolver>) {
		self.resolver = Some(resolver);
	}

	/// The module system, if any.
	pub fn resolver(&self) -> Option<&dyn ModuleResolver> {
		self.resolver.as_deref()
	}

	/// The current file of a module, parsed from the resolver's text on first
	/// use; `None` when the resolver has no such module.
	pub fn file_of_module(&mut self, module: &str) -> Option<FileId> {
		if let Some(&file) = self.modules.get(module) {
			return Some(file);
		}
		let text = self.resolver.as_ref()?.source(module)?;
		Some(self.set_source(module, text))
	}

	/// Records the declaration of a type.
	pub(crate) fn record_type_origin(&mut self, ty: TypeId, file: FileId, decl: NodeId) {
		self.type_origins.insert(ty, (file, decl));
	}

	/// The file and the declaration of a type.
	pub(crate) fn type_origin(&self, ty: TypeId) -> Option<(FileId, NodeId)> {
		self.type_origins.get(&ty).copied()
	}

	/// The current file of a module, if it is loaded.
	pub fn loaded_module(&self, module: &str) -> Option<FileId> {
		self.modules.get(module).copied()
	}

	/// The role of a file.
	pub fn role(&self, file: FileId) -> FileRole {
		self.files[file.index()].role
	}

	/// The name a file is known by, which designates its export inside it
	/// (`math` for `library/math.fa`); `None` for a lone file, a test file or
	/// a configuration file.
	pub fn own_name(&self, file: FileId) -> Option<String> {
		let module = &self.files[file.index()].module;
		self.resolver.as_ref()?.name(module)
	}

	/// The path of a file as shown in messages (`library/math.fa`).
	pub fn display_path(&self, file: FileId) -> String {
		let module = &self.files[file.index()].module;
		match &self.resolver {
			Some(resolver) => resolver.path(module),
			None => format!("{module}.fa"),
		}
	}

	/// The source file record of a file (its interface, its state).
	pub fn source_file(&self, file: FileId) -> &SourceFile {
		&self.files[file.index()]
	}

	/// The source file record of a file, to update it.
	pub(crate) fn source_file_mut(&mut self, file: FileId) -> &mut SourceFile {
		&mut self.files[file.index()]
	}

	/// The file and the symbol an [`Resolution::External`] designates: the
	/// current file of the module, and the symbol of the declaration there.
	pub fn external(&self, resolution: Resolution) -> Option<(FileId, SymbolId)> {
		let Resolution::External(module, decl) = resolution else {
			return None;
		};
		self.external_decl(module, decl).and_then(|(file, decl)| {
			Some((file, self.analysis(file).symbols.symbol_of_decl(decl)?))
		})
	}

	/// The current file of a module id, with a declaration node of it.
	pub fn external_decl(&self, module: ModuleId, decl: NodeId) -> Option<(FileId, NodeId)> {
		let path = self.types.module_path(module);
		let file = self.loaded_module(path)?;
		Some((file, decl))
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
		let role = match &self.resolver {
			Some(resolver) => resolver.role(module),
			None if crate::check::is_test_module(module) => FileRole::Test,
			None => FileRole::Program,
		};
		self.files.push(SourceFile {
			module: module.to_string(),
			tree: Arc::new(tree),
			analysis,
			analyzed: false,
			done: false,
			role,
			interface: Interface::default(),
			deferred: false,
			reads: Vec::new(),
			configuration: None,
		});
		self.modules.insert(module.to_string(), file);
		file
	}

	/// Analyzes a file (once: the result is memoized).
	pub fn analyze_file(&mut self, file: FileId) -> &Analysis {
		if !self.files[file.index()].analyzed {
			self.files[file.index()].analyzed = true;
			match self.files[file.index()].role {
				// a configuration file is evaluated, not walked (D37)
				FileRole::Configuration(kind) => {
					let kind = match kind {
						fa_codebase_parser::Configuration::Package => {
							crate::configuration::ConfigurationKind::Package
						}
						fa_codebase_parser::Configuration::Workspace => {
							crate::configuration::ConfigurationKind::Workspace
						}
						fa_codebase_parser::Configuration::Dependencies => {
							crate::configuration::ConfigurationKind::Dependencies
						}
					};
					let source = &mut self.files[file.index()];
					let (value, diagnostics) =
						crate::configuration::evaluate(file, &source.tree, kind);
					source.configuration = value;
					source.analysis.diagnostics.extend(diagnostics);
				}
				_ => crate::check::analyze(self, file),
			}
			self.files[file.index()].done = true;
		}
		&self.files[file.index()].analysis
	}

	/// Forgets the analysis of a file, so that the next
	/// [`Session::analyze_file`] walks it again (its interface is kept: the
	/// files analyzed meanwhile read it).
	pub fn reset_analysis(&mut self, file: FileId) {
		let source = &mut self.files[file.index()];
		let mut analysis = Analysis::new(file, source.tree.nodes.len());
		analysis.diagnostics = convert_all(file, &source.tree);
		source.analysis = analysis;
		source.analyzed = false;
		source.done = false;
		source.deferred = false;
		source.reads.clear();
	}

	/// Adds a diagnostic to a file's analysis (program-level checks).
	pub(crate) fn push_diagnostic(&mut self, file: FileId, diagnostic: Diagnostic) {
		let diagnostics = &mut self.files[file.index()].analysis.diagnostics;
		let duplicate = diagnostics.iter().any(|other| {
			std::ptr::eq(other.code, diagnostic.code)
				&& other.location == diagnostic.location
				&& other.message == diagnostic.message
		});
		if !duplicate {
			diagnostics.push(diagnostic);
		}
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
		// the synthetic owners of the imports and proxy scopes have no span
		self.tree(file)
			.spans
			.get(node.index())
			.map_or(SourceSpan::default(), |&span| span_of(span))
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
