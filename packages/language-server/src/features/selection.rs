//! `textDocument/selectionRange`: expand the selection to the enclosing nodes of the tree.

use fa_parser::{Name, Node, SourceSpan, TypedSyntaxTree};
use tower_lsp::lsp_types::SelectionRange;

use crate::document::Document;
use crate::line_index::PositionEncoding;

/// The selection ranges at an offset: the innermost node containing it, then each enclosing
/// node, up to the whole document. Every range contains the previous one.
pub fn selection_range(
	tree: &TypedSyntaxTree,
	document: &Document,
	offset: usize,
	encoding: PositionEncoding,
) -> SelectionRange {
	let spans = enclosing_spans(tree, offset);
	let mut selection = SelectionRange {
		range: document.range(SourceSpan::new(0, document.text.len()), encoding),
		parent: None,
	};
	// build from the outermost range to the innermost one
	for span in spans.iter().rev() {
		let range = document.range(*span, encoding);
		if range == selection.range {
			continue;
		}
		selection = SelectionRange {
			range,
			parent: Some(Box::new(selection)),
		};
	}
	selection
}

/// The spans containing an offset, from the innermost to the outermost, each one containing
/// the previous one.
pub fn enclosing_spans(tree: &TypedSyntaxTree, offset: usize) -> Vec<SourceSpan> {
	let mut path = Vec::new();
	let mut current = tree.root;
	loop {
		let span = tree.span(current);
		if contains(span, offset) {
			path.push(span);
		}
		if let Some(name) = declared_name(tree.at(current))
			&& contains(name.span(), offset)
		{
			path.push(name.span());
			break;
		}
		let mut next = None;
		tree.at(current).for_each_child(|child| {
			if next.is_none() && child < tree.nodes.len() && contains(tree.span(child), offset) {
				next = Some(child);
			}
		});
		match next {
			Some(child) => current = child,
			None => break,
		}
	}
	// innermost first, and every span inside the next one
	path.reverse();
	for index in 1..path.len() {
		let inner = path[index - 1];
		let outer = &mut path[index];
		outer.start = outer.start.min(inner.start);
		outer.end = outer.end.max(inner.end);
	}
	path.dedup();
	path
}

fn contains(span: SourceSpan, offset: usize) -> bool {
	span.start <= offset && offset <= span.end && span.start < span.end
}

/// The name declared by a node, which is not a node of its own.
fn declared_name(node: &Node) -> Option<Name> {
	match node {
		Node::Let { name, .. }
		| Node::Mutable { name, .. }
		| Node::Use { name, .. }
		| Node::Reuse { name, .. }
		| Node::Static { name, .. }
		| Node::Type { name, .. }
		| Node::Reactive { name, .. }
		| Node::Derived { name, .. }
		| Node::Namespace { name, .. }
		| Node::Function { name, .. } => Some(*name),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The texts of the selection ranges at the first occurrence of `marker` in `source`.
	fn selections(source: &str, marker: &str) -> Vec<String> {
		let tree = fa_parser::parse(source);
		let offset = source.find(marker).unwrap();
		enclosing_spans(&tree, offset)
			.iter()
			.map(|span| source[span.start..span.end].to_string())
			.collect()
	}

	#[test]
	fn expands_from_a_name_to_the_statement() {
		assert_eq!(
			selections("let a = b + c * d", "c"),
			["c", "c * d", "b + c * d", "let a = b + c * d"]
		);
	}

	#[test]
	fn declared_name() {
		assert_eq!(
			selections("function add = (a, b) => a + b", "add"),
			["add", "function add = (a, b) => a + b"]
		);
	}

	#[test]
	fn inside_a_block() {
		let source = "function main = () => {\n\tprint(x)\n}";
		let selections = selections(source, "x");
		assert_eq!(selections[0], "x");
		assert_eq!(selections[1], "print(x)");
		assert!(selections.contains(&"{\n\tprint(x)\n}".to_string()));
		assert_eq!(selections.last().unwrap(), source);
	}

	#[test]
	fn ranges_are_nested_in_lsp_positions() {
		let source = "let s = \"é\" + f(😀x)";
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		let offset = source.find("f(").unwrap();
		let mut selection = Some(selection_range(
			&tree,
			&document,
			offset,
			PositionEncoding::Utf16,
		));
		let mut previous: Option<tower_lsp::lsp_types::Range> = None;
		let mut count = 0;
		while let Some(current) = selection {
			if let Some(previous) = previous {
				let start = (current.range.start.line, current.range.start.character);
				let end = (current.range.end.line, current.range.end.character);
				assert!(start <= (previous.start.line, previous.start.character));
				assert!(end >= (previous.end.line, previous.end.character));
				assert_ne!(current.range, previous);
			}
			previous = Some(current.range);
			selection = current.parent.map(|parent| *parent);
			count += 1;
		}
		assert!(count >= 2);
	}

	#[test]
	fn offset_outside_of_any_statement() {
		let source = "\n\nlet a = 1\n\n";
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		let selection = selection_range(&tree, &document, 0, PositionEncoding::Utf16);
		assert!(selection.parent.is_none());
		assert_eq!(selection.range.end.line, 4);
	}
}
