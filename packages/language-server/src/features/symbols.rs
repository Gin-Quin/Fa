//! `textDocument/documentSymbol`: the outline of a document, from its syntax tree.

use fa_parser::{Name, Node, SourceSpan, TypedSyntaxTree};
use tower_lsp::lsp_types::{DocumentSymbol, SymbolKind};

use crate::document::Document;
use crate::line_index::PositionEncoding;

/// The declarations of a document, nested like in the source: the members of types and
/// namespaces, and the functions and types declared inside functions. The values declared
/// inside function bodies (locals) are left out.
pub fn document_symbols(
	tree: &TypedSyntaxTree,
	document: &Document,
	encoding: PositionEncoding,
) -> Vec<DocumentSymbol> {
	let collector = Collector {
		tree,
		document,
		encoding,
	};
	let mut symbols = Vec::new();
	if let Some(Node::Module { statements, .. }) = tree.nodes.get(tree.root) {
		for statement in statements {
			collector.collect(*statement, Scope::Module, &mut symbols);
		}
	}
	symbols
}

/// Where a declaration is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
	Module,
	Namespace,
	// the members of an object type
	Type,
	// the members of `Enum { ... }`, `Union { ... }`, `Errors { ... }` or `Fields { ... }`
	Variants,
	// a function body, or any other block: only functions and types are listed
	Body,
}

struct Collector<'a> {
	tree: &'a TypedSyntaxTree,
	document: &'a Document,
	encoding: PositionEncoding,
}

impl Collector<'_> {
	fn collect(&self, index: usize, scope: Scope, out: &mut Vec<DocumentSymbol>) {
		let tree = self.tree;
		let Some(node) = tree.nodes.get(index) else {
			return;
		};
		match node {
			Node::Function {
				name,
				type_expression,
				expression,
				..
			} => {
				let kind = if scope == Scope::Type {
					SymbolKind::METHOD
				} else {
					SymbolKind::FUNCTION
				};
				let detail = self.function_detail(*type_expression, *expression);
				let children = self.children_of(*expression, Scope::Body);
				self.push(out, index, *name, kind, detail, children);
			}
			Node::Type {
				name, expression, ..
			} => {
				let (kind, children) = self.type_symbols(*expression);
				self.push(out, index, *name, kind, Some("type".to_string()), children);
			}
			Node::Namespace { name, expression } => {
				let children = self.members(*expression, Scope::Namespace);
				self.push(
					out,
					index,
					*name,
					SymbolKind::NAMESPACE,
					Some("namespace".to_string()),
					children,
				);
			}
			Node::Let {
				name,
				type_expression,
				expression,
			}
			| Node::Mutable {
				name,
				type_expression,
				expression,
			}
			| Node::Reactive {
				name,
				type_expression,
				expression,
			}
			| Node::Derived {
				name,
				type_expression,
				expression,
			} => {
				let keyword = match node {
					Node::Let { .. } => "let",
					Node::Mutable { .. } => "mutable",
					Node::Reactive { .. } => "reactive",
					_ => "derived",
				};
				self.value(
					out,
					index,
					scope,
					(*name, keyword, SymbolKind::VARIABLE),
					*type_expression,
					*expression,
				);
			}
			Node::Static {
				name,
				type_expression,
				expression,
			} => self.value(
				out,
				index,
				scope,
				(*name, "static", SymbolKind::CONSTANT),
				*type_expression,
				Some(*expression),
			),
			Node::Use {
				name,
				type_expression,
				expression,
				is_mutable,
			} => {
				let keyword = if *is_mutable { "mutable use" } else { "use" };
				self.value(
					out,
					index,
					scope,
					(*name, keyword, SymbolKind::VARIABLE),
					*type_expression,
					Some(*expression),
				);
			}
			Node::Assignment {
				name,
				type_expression,
				expression,
			} if matches!(scope, Scope::Type | Scope::Namespace | Scope::Variants) => {
				let Some(Node::Identifier { name }) = tree.nodes.get(*name) else {
					return self.descend(index, Scope::Body, out);
				};
				let (kind, children) = match scope {
					Scope::Variants => (
						SymbolKind::ENUM_MEMBER,
						expression.map_or_else(Vec::new, |expression| {
							self.members(expression, Scope::Type)
						}),
					),
					Scope::Type => (
						SymbolKind::FIELD,
						expression.map_or_else(Vec::new, |expression| {
							self.children_of(expression, Scope::Body)
						}),
					),
					_ => (
						SymbolKind::VARIABLE,
						expression.map_or_else(Vec::new, |expression| {
							self.children_of(expression, Scope::Body)
						}),
					),
				};
				let detail = type_expression.map(|type_expression| self.text(type_expression));
				self.push(out, index, *name, kind, detail, children);
			}
			Node::Identifier { name } if matches!(scope, Scope::Variants) => {
				self.push(out, index, *name, SymbolKind::ENUM_MEMBER, None, Vec::new());
			}
			Node::Test {
				name,
				modifier,
				body,
				..
			} => {
				let label = match tree.nodes.get(*name) {
					Some(Node::StringLiteral(text)) => format!("test \"{text}\""),
					_ => format!("test {}", tree.text(*name)),
				};
				let detail = modifier.map(|modifier| match modifier {
					fa_parser::nodes::TestModifier::Skip => "skip".to_string(),
					fa_parser::nodes::TestModifier::Only => "only".to_string(),
				});
				let children = self.children_of(*body, Scope::Body);
				let name_span = tree.span(*name);
				self.push_named(
					out,
					index,
					label,
					name_span,
					SymbolKind::FUNCTION,
					detail,
					children,
				);
			}
			Node::ExportFunction {
				type_expression,
				expression,
				..
			} => {
				let detail = self.function_detail(*type_expression, *expression);
				let children = self.children_of(*expression, Scope::Body);
				self.push_export(
					out,
					index,
					"export function",
					SymbolKind::FUNCTION,
					detail,
					children,
				);
			}
			Node::ExportType { expression, .. } => {
				let (kind, children) = self.type_symbols(*expression);
				self.push_export(out, index, "export type", kind, None, children);
			}
			Node::ExportNamespace { expression } => {
				let children = self.members(*expression, Scope::Namespace);
				self.push_export(
					out,
					index,
					"export namespace",
					SymbolKind::NAMESPACE,
					None,
					children,
				);
			}
			Node::ExportValue {
				is_mutable,
				type_expression,
				expression,
			} => {
				let detail = type_expression.map(|type_expression| self.text(type_expression));
				let children = self.children_of(*expression, Scope::Body);
				let label = if *is_mutable {
					"export mutable"
				} else {
					"export"
				};
				self.push_export(out, index, label, SymbolKind::VARIABLE, detail, children);
			}
			_ => self.descend(index, Scope::Body, out),
		}
	}

	/// A value declaration: listed outside of function bodies.
	fn value(
		&self,
		out: &mut Vec<DocumentSymbol>,
		index: usize,
		scope: Scope,
		(name, keyword, kind): (Name, &str, SymbolKind),
		type_expression: Option<usize>,
		expression: Option<usize>,
	) {
		let children = expression.map_or_else(Vec::new, |expression| {
			self.children_of(expression, Scope::Body)
		});
		if scope == Scope::Body {
			out.extend(children);
			return;
		}
		let detail = match type_expression {
			Some(type_expression) => format!("{keyword}: {}", self.text(type_expression)),
			None => keyword.to_string(),
		};
		self.push(out, index, name, kind, Some(detail), children);
	}

	/// The kind and the members of a type declared as `expression`.
	fn type_symbols(&self, expression: usize) -> (SymbolKind, Vec<DocumentSymbol>) {
		let tree = self.tree;
		match tree.nodes.get(expression) {
			Some(Node::Members { .. }) => {
				(SymbolKind::STRUCT, self.members(expression, Scope::Type))
			}
			Some(Node::MemberIndex { target, members }) => {
				let constructor = match tree.nodes.get(*target) {
					Some(Node::Identifier { name }) => tree.name(*name),
					_ => "",
				};
				let scope = match constructor {
					"Enum" | "Union" | "Errors" | "Fields" => Scope::Variants,
					_ => Scope::Type,
				};
				let kind = if scope == Scope::Variants {
					SymbolKind::ENUM
				} else {
					SymbolKind::STRUCT
				};
				let mut children = Vec::new();
				for member in members {
					self.collect(*member, scope, &mut children);
				}
				(kind, children)
			}
			Some(Node::Union { operands })
				if operands.iter().all(|operand| {
					matches!(tree.nodes.get(*operand), Some(Node::ImplicitMember { .. }))
				}) =>
			{
				let children = operands
					.iter()
					.filter_map(|operand| match tree.nodes.get(*operand) {
						Some(Node::ImplicitMember { name }) => {
							self.symbol(*operand, *name, SymbolKind::ENUM_MEMBER, None, Vec::new())
						}
						_ => None,
					})
					.collect();
				(SymbolKind::ENUM, children)
			}
			Some(Node::ArrowFunction { .. }) => (SymbolKind::INTERFACE, Vec::new()),
			_ => (
				SymbolKind::TYPE_PARAMETER,
				self.children_of(expression, Scope::Body),
			),
		}
	}

	/// The symbols of the members of `{ ... }`, or of the declarations inside any other node.
	fn members(&self, index: usize, scope: Scope) -> Vec<DocumentSymbol> {
		let mut symbols = Vec::new();
		match self.tree.nodes.get(index) {
			Some(Node::Members { items }) => {
				for item in items {
					self.collect(*item, scope, &mut symbols);
				}
			}
			Some(_) => self.collect(index, Scope::Body, &mut symbols),
			None => {}
		}
		symbols
	}

	fn children_of(&self, index: usize, scope: Scope) -> Vec<DocumentSymbol> {
		let mut symbols = Vec::new();
		self.collect(index, scope, &mut symbols);
		symbols
	}

	fn descend(&self, index: usize, scope: Scope, out: &mut Vec<DocumentSymbol>) {
		if let Some(node) = self.tree.nodes.get(index) {
			node.for_each_child(|child| self.collect(child, scope, out));
		}
	}

	/// `(a: Integer): Integer`: the signature of a lambda, or the declared function type.
	fn function_detail(&self, type_expression: Option<usize>, expression: usize) -> Option<String> {
		if let Some(type_expression) = type_expression {
			return Some(self.text(type_expression));
		}
		let Some(Node::ArrowFunction { body, .. }) = self.tree.nodes.get(expression) else {
			return None;
		};
		let start = self.tree.span(expression).start;
		let end = self.tree.span(*body).start.max(start);
		let signature = self.tree.source.get(start..end).unwrap_or("").trim_end();
		let signature = signature.strip_suffix("=>").unwrap_or(signature).trim_end();
		Some(collapse_whitespace(signature))
	}

	fn text(&self, index: usize) -> String {
		collapse_whitespace(self.tree.text(index))
	}

	fn push(
		&self,
		out: &mut Vec<DocumentSymbol>,
		index: usize,
		name: Name,
		kind: SymbolKind,
		detail: Option<String>,
		children: Vec<DocumentSymbol>,
	) {
		match self.symbol(index, name, kind, detail, children.clone()) {
			Some(symbol) => out.push(symbol),
			// a declaration without name, after a syntax error: keep what it contains
			None => out.extend(children),
		}
	}

	fn push_export(
		&self,
		out: &mut Vec<DocumentSymbol>,
		index: usize,
		label: &str,
		kind: SymbolKind,
		detail: Option<String>,
		children: Vec<DocumentSymbol>,
	) {
		let span = self.tree.span(index);
		let keyword_span = SourceSpan::new(span.start, (span.start + "export".len()).min(span.end));
		self.push_named(
			out,
			index,
			label.to_string(),
			keyword_span,
			kind,
			detail,
			children,
		);
	}

	#[allow(clippy::too_many_arguments)]
	fn push_named(
		&self,
		out: &mut Vec<DocumentSymbol>,
		index: usize,
		label: String,
		name_span: SourceSpan,
		kind: SymbolKind,
		detail: Option<String>,
		children: Vec<DocumentSymbol>,
	) {
		out.push(self.build(index, label, name_span, kind, detail, children));
	}

	fn symbol(
		&self,
		index: usize,
		name: Name,
		kind: SymbolKind,
		detail: Option<String>,
		children: Vec<DocumentSymbol>,
	) -> Option<DocumentSymbol> {
		let label = self.tree.name(name);
		if name.is_empty() || label.is_empty() {
			return None;
		}
		Some(self.build(
			index,
			label.to_string(),
			name.span(),
			kind,
			detail,
			children,
		))
	}

	fn build(
		&self,
		index: usize,
		name: String,
		name_span: SourceSpan,
		kind: SymbolKind,
		detail: Option<String>,
		children: Vec<DocumentSymbol>,
	) -> DocumentSymbol {
		let span = self.tree.span(index);
		// the range must contain the selection range
		let span = SourceSpan::new(span.start.min(name_span.start), span.end.max(name_span.end));
		#[allow(deprecated)]
		DocumentSymbol {
			name,
			detail: detail.filter(|detail| !detail.is_empty()),
			kind,
			tags: None,
			deprecated: None,
			range: self.document.range(span, self.encoding),
			selection_range: self.document.range(name_span, self.encoding),
			children: if children.is_empty() {
				None
			} else {
				Some(children)
			},
		}
	}
}

/// Replace every run of whitespace (including new lines) with one space.
fn collapse_whitespace(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The outline as `kind name [detail]` lines, indented by nesting.
	fn outline(source: &str) -> String {
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		let symbols = document_symbols(&tree, &document, PositionEncoding::Utf16);
		let mut lines = Vec::new();
		fn walk(symbols: &[DocumentSymbol], depth: usize, lines: &mut Vec<String>) {
			for symbol in symbols {
				let detail = symbol
					.detail
					.as_ref()
					.map(|detail| format!(" [{detail}]"))
					.unwrap_or_default();
				lines.push(format!(
					"{}{:?} {}{detail}",
					"  ".repeat(depth),
					symbol.kind,
					symbol.name
				));
				assert!(
					contains(symbol.range, symbol.selection_range),
					"{}: the range must contain the selection range",
					symbol.name
				);
				if let Some(children) = &symbol.children {
					walk(children, depth + 1, lines);
				}
			}
		}
		walk(&symbols, 0, &mut lines);
		lines.join("\n")
	}

	fn contains(outer: tower_lsp::lsp_types::Range, inner: tower_lsp::lsp_types::Range) -> bool {
		let before = |a: tower_lsp::lsp_types::Position, b: tower_lsp::lsp_types::Position| {
			(a.line, a.character) <= (b.line, b.character)
		};
		before(outer.start, inner.start) && before(inner.end, outer.end)
	}

	#[test]
	fn functions_and_values() {
		assert_eq!(
			outline(
				"let a = 1\nmutable b: Integer = 2\nfunction add = (x: Integer, y: Integer): Integer => x + y"
			),
			"Variable a [let]\nVariable b [mutable: Integer]\nFunction add [(x: Integer, y: Integer): Integer]"
		);
	}

	#[test]
	fn static_reactive_and_derived_values() {
		assert_eq!(
			outline("static version = 3\nreactive score = 0\nderived doubled = score * 2"),
			"Constant version [static]\nVariable score [reactive]\nVariable doubled [derived]"
		);
	}

	#[test]
	fn object_type_with_fields_and_methods() {
		let source = "type User = {\n\tname: String\n\tage: Integer = 0\n\n\tfunction greet = (self): String => \"Hello, {self.name}\"\n}";
		assert_eq!(
			outline(source),
			"Struct User [type]\n  Field name [String]\n  Field age [Integer]\n  Method greet [(self): String]"
		);
	}

	#[test]
	fn namespace_members() {
		let source = "namespace utils = {\n\tlet x = 1\n\tfunction add = (a: Integer, b: Integer): Integer => a + b\n}";
		assert_eq!(
			outline(source),
			"Namespace utils [namespace]\n  Variable x [let]\n  Function add [(a: Integer, b: Integer): Integer]"
		);
	}

	#[test]
	fn nested_namespaces() {
		let source = "namespace a = {\n\tnamespace b = {\n\t\tfunction c = () => 1\n\t}\n}";
		assert_eq!(
			outline(source),
			"Namespace a [namespace]\n  Namespace b [namespace]\n    Function c [()]"
		);
	}

	#[test]
	fn functions_inside_functions_but_not_locals() {
		let source = "function main = () => {\n\tlet user = 1\n\tfunction helper = () => user\n\ttype Local = { x: Integer }\n}";
		assert_eq!(
			outline(source),
			"Function main [()]\n  Function helper [()]\n  Struct Local [type]\n    Field x [Integer]"
		);
	}

	#[test]
	fn functions_inside_control_flow() {
		let source = "function main = () => {\n\tif ready {\n\t\tfunction inner = () => 1\n\t}\n}";
		assert_eq!(outline(source), "Function main [()]\n  Function inner [()]");
	}

	#[test]
	fn enumerations_and_named_unions() {
		let source = "type Color = Enum { red, green, blue }\ntype Shape = Union {\n\tCircle = { radius: Float }\n\tPoint\n}";
		assert_eq!(
			outline(source),
			"Enum Color [type]\n  EnumMember red\n  EnumMember green\n  EnumMember blue\nEnum Shape [type]\n  EnumMember Circle\n    Field radius [Float]\n  EnumMember Point"
		);
	}

	#[test]
	fn union_of_implicit_members() {
		assert_eq!(
			outline("type Size = .small | .large"),
			"Enum Size [type]\n  EnumMember small\n  EnumMember large"
		);
	}

	#[test]
	fn type_aliases_and_function_types() {
		assert_eq!(
			outline(
				"type Value = String | Integer\ntype Comparator = (a: Integer, b: Integer) => Integer"
			),
			"TypeParameter Value [type]\nInterface Comparator [type]"
		);
	}

	#[test]
	fn tests_and_their_nested_functions() {
		let source = "test \"adds\" {\n\tfunction helper = () => 1\n\tassert helper() == 1\n}\ntest skip \"later\" { }";
		assert_eq!(
			outline(source),
			"Function test \"adds\"\n  Function helper [()]\nFunction test \"later\" [skip]"
		);
	}

	#[test]
	fn exports() {
		let source =
			"export function = (a: Integer) => a\nexport type = { x: Integer }\nexport = 42";
		assert_eq!(
			outline(source),
			"Function export function [(a: Integer)]\nStruct export type\n  Field x [Integer]\nVariable export"
		);
	}

	#[test]
	fn function_with_a_declared_type() {
		assert_eq!(
			outline("function ascending: Comparator = (a, b) => a - b"),
			"Function ascending [Comparator]"
		);
	}

	#[test]
	fn multi_line_signature_is_collapsed() {
		assert_eq!(
			outline("function add = (\n\ta: Integer,\n\tb: Integer\n) => a + b"),
			"Function add [( a: Integer, b: Integer )]"
		);
	}

	#[test]
	fn broken_code_keeps_the_valid_declarations() {
		let source = "function ok = () => 1\nlet = 3\ntype = \nfunction alsoOk = () => 2";
		let outline = outline(source);
		assert!(outline.contains("Function ok"), "{outline}");
		assert!(outline.contains("Function alsoOk"), "{outline}");
	}

	#[test]
	fn empty_document_has_no_symbols() {
		assert_eq!(outline(""), "");
		assert_eq!(outline("-- just a comment"), "");
	}

	#[test]
	fn ranges_use_the_negotiated_encoding() {
		// `let s = "é😀"`: 13 UTF-16 units, 16 bytes
		let source = "let s = \"é😀\"\nfunction f = () => 1";
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		let symbols = document_symbols(&tree, &document, PositionEncoding::Utf16);
		assert_eq!(symbols.len(), 2);
		assert_eq!(symbols[0].range.end.character, 13);
		assert_eq!(symbols[1].range.start.line, 1);
		let symbols = document_symbols(&tree, &document, PositionEncoding::Utf8);
		assert_eq!(symbols[0].range.end.character, 16);
	}
}
