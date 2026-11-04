//! `textDocument/foldingRange`: the blocks, brackets and comments that span several lines.

use fa_parser::{Node, SourceSpan, TypedSyntaxTree};
use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

use crate::document::Document;

/// The folding ranges of a document, sorted by start line, at most one per start line. Folding
/// stops before a line that starts with the closing bracket, so that `}` stays visible.
pub fn folding_ranges(tree: &TypedSyntaxTree, document: &Document) -> Vec<FoldingRange> {
	let mut ranges = Vec::new();
	for (index, node) in tree.nodes.iter().enumerate() {
		let foldable = matches!(
			node,
			Node::Block { .. }
				| Node::Members { .. }
				| Node::List { .. }
				| Node::Tuple { .. }
				| Node::Parameters { .. }
				| Node::When { .. }
				| Node::MemberIndex { .. }
				| Node::FunctionCall { .. }
				| Node::GenericApplication { .. }
				| Node::ObjectPattern { .. }
				| Node::ArrayPattern { .. }
				| Node::StringLiteral(_)
				| Node::StringTemplate { .. }
		);
		if foldable {
			let span = match node {
				// fold the arguments, not the function or the generic type
				Node::FunctionCall { function, .. }
				| Node::GenericApplication {
					target: function, ..
				} => SourceSpan::new(tree.span(*function).end, tree.span(index).end),
				Node::MemberIndex { target, .. } => {
					SourceSpan::new(tree.span(*target).end, tree.span(index).end)
				}
				_ => tree.span(index),
			};
			push_region(&mut ranges, document, span, None);
		}
	}
	comment_ranges(tree, document, &mut ranges);

	// one range per start line: the largest one (editors keep only one)
	ranges.sort_by_key(|range| (range.start_line, std::cmp::Reverse(range.end_line)));
	ranges.dedup_by_key(|range| range.start_line);
	ranges
}

fn push_region(
	ranges: &mut Vec<FoldingRange>,
	document: &Document,
	span: SourceSpan,
	kind: Option<FoldingRangeKind>,
) {
	if span.end <= span.start {
		return;
	}
	let index = &document.line_index;
	let start_line = index.line_of(span.start);
	let mut end_line = index.line_of(span.end);
	// keep the line of the closing bracket visible when it starts that line
	let last = document.text.as_bytes().get(span.end - 1).copied();
	if kind.is_none() && matches!(last, Some(b'}' | b']' | b')')) {
		let line_start = index.line_start(end_line).unwrap_or(0);
		let before = document.text.get(line_start..span.end - 1).unwrap_or("");
		if before.trim().is_empty() && end_line > start_line {
			end_line -= 1;
		}
	}
	if end_line > start_line {
		ranges.push(FoldingRange {
			start_line: start_line as u32,
			start_character: None,
			end_line: end_line as u32,
			end_character: None,
			kind,
			collapsed_text: None,
		});
	}
}

/// A block comment on several lines, or several line comments on consecutive lines.
fn comment_ranges(tree: &TypedSyntaxTree, document: &Document, ranges: &mut Vec<FoldingRange>) {
	let index = &document.line_index;
	let text = &document.text;
	// the current run of line comments: (first line, last line, span)
	let mut run: Option<(usize, usize, SourceSpan)> = None;
	let flush = |run: &mut Option<(usize, usize, SourceSpan)>, ranges: &mut Vec<FoldingRange>| {
		if let Some((_, _, span)) = run.take() {
			push_region(ranges, document, span, Some(FoldingRangeKind::Comment));
		}
	};
	for comment in &tree.comments {
		let comment_text = text.get(comment.start..comment.end).unwrap_or("");
		let start_line = index.line_of(comment.start);
		let end_line = index.line_of(comment.end);
		let is_block = comment_text.starts_with("---");
		let alone_on_its_line = index
			.line_start(start_line)
			.and_then(|line_start| text.get(line_start..comment.start))
			.is_some_and(|before| before.trim().is_empty());
		if is_block || !alone_on_its_line {
			flush(&mut run, ranges);
			if is_block {
				push_region(ranges, document, *comment, Some(FoldingRangeKind::Comment));
			}
			continue;
		}
		match &mut run {
			Some((_, last_line, span)) if *last_line + 1 == start_line => {
				*last_line = end_line;
				span.end = comment.end;
			}
			_ => {
				flush(&mut run, ranges);
				run = Some((start_line, end_line, *comment));
			}
		}
	}
	flush(&mut run, ranges);
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The folding ranges as `start..end` lines, with `c` for comments.
	fn folds(source: &str) -> Vec<String> {
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		folding_ranges(&tree, &document)
			.iter()
			.map(|range| {
				let kind = if range.kind == Some(FoldingRangeKind::Comment) {
					" c"
				} else {
					""
				};
				format!("{}..{}{kind}", range.start_line, range.end_line)
			})
			.collect()
	}

	#[test]
	fn function_body() {
		assert_eq!(
			folds("function main = () => {\n\tlet a = 1\n\tlet b = 2\n}"),
			["0..2"]
		);
	}

	#[test]
	fn nested_blocks() {
		let source =
			"function main = () => {\n\tif a {\n\t\tb()\n\t}\n\tfor x in y {\n\t\tz()\n\t}\n}";
		assert_eq!(folds(source), ["0..6", "1..2", "4..5"]);
	}

	#[test]
	fn single_line_block_is_not_folded() {
		assert!(folds("function f = () => { 1 }").is_empty());
		assert!(folds("type Point = { x: Integer, y: Integer }").is_empty());
	}

	#[test]
	fn type_and_namespace_members() {
		let source = "type User = {\n\tname: String\n}\nnamespace utils = {\n\tlet x = 1\n}";
		assert_eq!(folds(source), ["0..1", "3..4"]);
	}

	#[test]
	fn closing_bracket_after_code_is_folded() {
		assert_eq!(folds("let a = [\n\t1,\n\t2]"), ["0..2"]);
	}

	#[test]
	fn multi_line_list_and_call() {
		let source = "let a = [\n\t1,\n\t2,\n]\nprint(\n\ta,\n\tb\n)";
		assert_eq!(folds(source), ["0..2", "4..6"]);
	}

	#[test]
	fn when_branches() {
		let source = "let a = when x is {\n\t1 => \"one\"\n\telse => \"other\"\n}";
		assert_eq!(folds(source), ["0..2"]);
	}

	#[test]
	fn consecutive_line_comments() {
		let source = "-- one\n-- two\n-- three\nlet a = 1\n-- alone\nlet b = 2";
		assert_eq!(folds(source), ["0..2 c"]);
	}

	#[test]
	fn block_comment() {
		let source = "---\nA long\ncomment\n---\nlet a = 1";
		assert_eq!(folds(source), ["0..3 c"]);
	}

	#[test]
	fn trailing_comments_are_not_grouped() {
		let source = "let a = 1 -- one\nlet b = 2 -- two\n";
		assert!(folds(source).is_empty());
	}

	#[test]
	fn comment_runs_separated_by_a_blank_line() {
		let source = "-- a\n-- b\n\n-- c\n-- d\n";
		assert_eq!(folds(source), ["0..1 c", "3..4 c"]);
	}

	#[test]
	fn multi_line_string() {
		let source = "let text = \"first\nsecond\nthird\"";
		assert_eq!(folds(source), ["0..2"]);
	}

	#[test]
	fn one_range_per_start_line() {
		// the call and the lambda block start on the same line
		let source = "items.forEach((item) => {\n\tprint(item)\n})";
		let folds = folds(source);
		assert_eq!(folds.len(), 1, "{folds:?}");
		assert!(folds[0].starts_with("0.."), "{folds:?}");
	}

	#[test]
	fn crlf_line_breaks() {
		assert_eq!(
			folds("function main = () => {\r\n\tlet a = 1\r\n}\r\n"),
			["0..1"]
		);
	}

	#[test]
	fn broken_code_still_folds() {
		let source = "function main = () => {\n\tlet a = (\n\tlet b = 2\n}";
		assert!(!folds(source).is_empty());
	}
}
