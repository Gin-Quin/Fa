//! The migration of the parenthesized generic lists to angle brackets (decision "Generics with
//! angle brackets", `design/decisions.md`): `type Pair(Left, Right)` becomes
//! `type Pair<Left, Right>`, `Array(Integer)` becomes `Array<Integer>`, and `sum(Float)([1, 2])`
//! becomes `sum<Float>([1, 2])`. Ordinary calls, compile-time ones included, keep their
//! parentheses.
//!
//! A declaration's parameter list is known from the syntax. An application is told apart from a
//! call by the analysis, which knows what a name denotes: the applications of types are
//! [`NodeFlags::TYPE_EXPR`], the explicit generic arguments of a function are
//! [`NodeFlags::GENERIC_APPLICATION`]. Everything else is kept byte for byte, except what the
//! new spelling requires: the spaces after the `(` (a `<` followed by a space is a comparison),
//! a space between the `>` and a following `=` (`>=` is a comparison), and parentheses around
//! an argument holding a comparison, like `Storage<(size > 255)>`.

use fa_parser::{SourceSpan, TypedSyntaxTree};

use crate::analysis::{Analysis, NodeFlags};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{self, NodeList, View};

/// A replacement of `source[start..end]` by `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
	pub start: usize,
	pub end: usize,
	pub text: &'static str,
}

/// The edits that move the generic lists of a file to angle brackets.
#[derive(Clone, Debug, Default)]
pub struct Migration {
	/// The edits, sorted by position, never overlapping.
	pub edits: Vec<Edit>,
	/// The number of generic lists rewritten.
	pub lists: usize,
	/// The calls of a capitalized name kept as calls (conversions, constructors of errors and
	/// enumeration payloads…), to review: an application the analysis didn't see stays in the
	/// parenthesized spelling, which is still accepted.
	pub kept_calls: Vec<SourceSpan>,
}

impl Migration {
	/// Whether the file changes.
	pub fn is_empty(&self) -> bool {
		self.edits.is_empty()
	}

	/// The migrated source.
	pub fn apply(&self, source: &str) -> String {
		let mut output = String::with_capacity(source.len() + self.edits.len());
		let mut position = 0;
		for edit in &self.edits {
			output.push_str(&source[position..edit.start]);
			output.push_str(edit.text);
			position = edit.end;
		}
		output.push_str(&source[position..]);
		output
	}

	/// The offset in the original source of an offset in the migrated one (the start of an
	/// edit for an offset inside its text).
	pub fn original_offset(&self, migrated: usize) -> usize {
		let mut shift: isize = 0;
		for edit in &self.edits {
			let new_start = (edit.start as isize + shift) as usize;
			if migrated < new_start {
				break;
			}
			let new_end = new_start + edit.text.len();
			if migrated < new_end {
				return edit.start;
			}
			shift += edit.text.len() as isize - (edit.end - edit.start) as isize;
		}
		(migrated as isize - shift).max(0) as usize
	}
}

/// The edits that move the parenthesized generic lists of an analyzed file to angle brackets.
pub fn angle_bracket_generics(tree: &TypedSyntaxTree, analysis: &Analysis) -> Migration {
	let source: &str = &tree.source;
	let mut migration = Migration::default();
	// the parenthesized applications, then the calls of capitalized names left
	let mut applications = vec![false; tree.nodes.len()];
	let mut calls = Vec::new();
	for index in 0..tree.nodes.len() {
		let n = NodeId::new(index);
		match syntax::view(tree, n) {
			View::TypeDecl {
				name, parameters, ..
			}
			| View::Function {
				name,
				generics: parameters,
				..
			} => {
				declaration(source, tree, name.end as usize, parameters, &mut migration);
			}
			View::ExportFunction {
				generics: parameters,
				..
			}
			| View::ExportType { parameters, .. } => {
				let start = tree.span(index).start;
				declaration(source, tree, start, parameters, &mut migration);
			}
			View::Call { callee, arguments } => {
				if parenthesized_call(source, tree, index, callee, arguments).is_none() {
					continue;
				}
				let callee_text = text(source, tree, callee);
				if analysis
					.flags_of(n)
					.intersects(NodeFlags::TYPE_EXPR | NodeFlags::GENERIC_APPLICATION)
					&& !callee_text.starts_with('@')
				{
					applications[index] = true;
				} else if is_capitalized_callee(callee_text) {
					calls.push(n);
				}
			}
			_ => {}
		}
	}
	// the arguments of a type application that the analysis didn't reach (a type function it
	// doesn't evaluate yet, like `Parameters(Type(f))`): a call of a capitalized name there is
	// the application of a type
	let mut pending: Vec<usize> = (0..tree.nodes.len())
		.filter(|index| applications[*index])
		.collect();
	while let Some(index) = pending.pop() {
		if !analysis
			.flags_of(NodeId::new(index))
			.contains(NodeFlags::TYPE_EXPR)
		{
			continue;
		}
		let View::Call { arguments, .. } = syntax::view(tree, NodeId::new(index)) else {
			continue;
		};
		for argument in arguments.iter() {
			if calls.contains(&argument) && analysis.type_of(argument) == TypeId::UNSET {
				applications[argument.index()] = true;
				calls.retain(|call| *call != argument);
				pending.push(argument.index());
			}
		}
	}
	for (index, is_application) in applications.into_iter().enumerate() {
		if !is_application {
			continue;
		}
		let View::Call { callee, arguments } = syntax::view(tree, NodeId::new(index)) else {
			continue;
		};
		if let Some((open, close)) = parenthesized_call(source, tree, index, callee, arguments) {
			list(source, tree, open, close, arguments, true, &mut migration);
		}
	}
	migration.kept_calls = calls.iter().map(|call| tree.span(call.index())).collect();
	migration.edits.sort_by_key(|edit| (edit.start, edit.end));
	migration
}

/// The `(` and the `)` of a call written with parentheses and at least one argument
/// (`Array<Integer>` has the same view).
fn parenthesized_call(
	source: &str,
	tree: &TypedSyntaxTree,
	index: usize,
	callee: NodeId,
	arguments: NodeList<'_>,
) -> Option<(usize, usize)> {
	if arguments.is_empty() {
		return None;
	}
	let open = skip_trivia(source, tree.span(callee.index()).end)?;
	let close = tree.span(index).end.checked_sub(1)?;
	let bytes = source.as_bytes();
	(bytes[open] == b'(' && bytes.get(close) == Some(&b')')).then_some((open, close))
}

/// The text of a node.
fn text<'s>(source: &'s str, tree: &TypedSyntaxTree, n: NodeId) -> &'s str {
	let span = tree.span(n.index());
	&source[span.start..span.end]
}

/// The parenthesized generic parameters of a declaration, after `after` (the declared name, or
/// the start of an `export`).
fn declaration(
	source: &str,
	tree: &TypedSyntaxTree,
	after: usize,
	parameters: NodeList<'_>,
	migration: &mut Migration,
) {
	let Some(first) = parameters.first() else {
		return;
	};
	let first_start = tree.span(first.index()).start;
	// the `(` is the last one before the first parameter (an `export` starts before it)
	let Some(open) = source[after..first_start]
		.rfind('(')
		.map(|offset| after + offset)
	else {
		return;
	};
	if source[open + 1..first_start].contains(['<', '>']) {
		return;
	}
	let last = parameters.last().expect("a parameter");
	let Some(close) = find_closing(source, tree.span(last.index()).end) else {
		return;
	};
	list(source, tree, open, close, parameters, false, migration);
}

/// Rewrite the list from the `(` at `open` to the `)` at `close`.
fn list(
	source: &str,
	tree: &TypedSyntaxTree,
	open: usize,
	close: usize,
	items: NodeList<'_>,
	wrap_comparisons: bool,
	migration: &mut Migration,
) {
	// `(` and the spaces and new lines after it: `<` must touch the first item
	let first = items
		.first()
		.map_or(close, |item| tree.span(item.index()).start);
	let mut after_open = open + 1;
	while after_open < first && source.as_bytes()[after_open].is_ascii_whitespace() {
		after_open += 1;
	}
	migration.edits.push(Edit {
		start: open,
		end: after_open,
		text: "<",
	});
	// `>=` would be a comparison: `Box(T)= 1` becomes `Box<T> = 1`
	let text = if source.as_bytes().get(close + 1) == Some(&b'=') {
		"> "
	} else {
		">"
	};
	migration.edits.push(Edit {
		start: close,
		end: close + 1,
		text,
	});
	if wrap_comparisons {
		for item in items.iter() {
			// a named argument keeps its name out of the parentheses: `max = (a > b)`
			let value = match syntax::view(tree, item) {
				View::Assignment {
					value: Some(value),
					annotation: None,
					..
				} => value,
				_ => item,
			};
			let span = tree.span(value.index());
			if has_comparison(&source[span.start..span.end]) {
				migration.edits.push(Edit {
					start: span.start,
					end: span.start,
					text: "(",
				});
				migration.edits.push(Edit {
					start: span.end,
					end: span.end,
					text: ")",
				});
			}
		}
	}
	migration.lists += 1;
}

/// Whether a `<` or a `>` is written outside of the brackets of an expression (not as part of
/// `=>` or `->`): in a generic list, a comparison goes in parentheses.
fn has_comparison(text: &str) -> bool {
	let bytes = text.as_bytes();
	let mut depth = 0usize;
	let mut index = 0;
	while index < bytes.len() {
		match bytes[index] {
			b'(' | b'[' | b'{' => depth += 1,
			b')' | b']' | b'}' => depth = depth.saturating_sub(1),
			b'"' => {
				index += 1;
				while index < bytes.len() && bytes[index] != b'"' {
					if bytes[index] == b'\\' {
						index += 1;
					}
					index += 1;
				}
			}
			b'-' if bytes.get(index + 1) == Some(&b'-') => {
				while index < bytes.len() && bytes[index] != b'\n' {
					index += 1;
				}
			}
			b'<' if depth == 0 => return true,
			b'>' if depth == 0 && index > 0 && !matches!(bytes[index - 1], b'=' | b'-') => {
				return true;
			}
			_ => {}
		}
		index += 1;
	}
	false
}

/// The `)` after `from`, past spaces, new lines, commas and comments.
fn find_closing(source: &str, from: usize) -> Option<usize> {
	let position = skip_trivia(source, from)?;
	(source.as_bytes()[position] == b')').then_some(position)
}

/// The first byte at or after `from` that is not a space, a new line, a comma or a comment.
fn skip_trivia(source: &str, from: usize) -> Option<usize> {
	let bytes = source.as_bytes();
	let mut position = from;
	while position < bytes.len() {
		match bytes[position] {
			b' ' | b'\t' | b'\n' | b'\r' | b',' => position += 1,
			b'-' if bytes.get(position + 1) == Some(&b'-') => {
				if bytes.get(position + 2) == Some(&b'-') {
					position += 3;
					while position < bytes.len() && !bytes[position..].starts_with(b"---") {
						position += 1;
					}
					position += 3;
				} else {
					while position < bytes.len() && bytes[position] != b'\n' {
						position += 1;
					}
				}
			}
			_ => return Some(position),
		}
	}
	None
}

/// `Int8`, `ApiError.Timeout`: a callee whose last name is capitalized.
fn is_capitalized_callee(text: &str) -> bool {
	text.rsplit('.')
		.next()
		.is_some_and(|name| name.starts_with(|character: char| character.is_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use super::*;
	use crate::Session;

	fn migrate(source: &str) -> String {
		let mut session = Session::new();
		let id = session.set_source("main", Arc::from(source));
		session.analyze_file(id);
		angle_bracket_generics(session.tree(id), session.analysis(id)).apply(source)
	}

	#[test]
	fn declarations_and_applications_move_to_angle_brackets() {
		assert_eq!(
			migrate(
				"type Pair(Left, Right) = { left: Left, right: Right }\nlet p: Pair(Integer, Array(String)) = Pair(Integer, Array(String)) { left = 1, right = [] }\n"
			),
			"type Pair<Left, Right> = { left: Left, right: Right }\nlet p: Pair<Integer, Array<String>> = Pair<Integer, Array<String>> { left = 1, right = [] }\n"
		);
		assert_eq!(
			migrate(
				"function first(Item) = (items: Array(Item)): Item? => items.get(0)\nlet n = first(Integer)([1])\nlet m = first([1])\n"
			),
			"function first<Item> = (items: Array<Item>): Item? => items.get(0)\nlet n = first<Integer>([1])\nlet m = first([1])\n"
		);
	}

	#[test]
	fn calls_keep_their_parentheses() {
		let source = "let small = Int8(300) catch error => 0\nlet text = String(12)\nconsole.log(max(1, 2))\n";
		assert_eq!(migrate(source), source);
	}

	#[test]
	fn ranged_integers_and_compile_time_values() {
		assert_eq!(
			migrate("type Digit = Integer(min = 0, max = 9)\n"),
			"type Digit = Integer<min = 0, max = 9>\n"
		);
	}

	#[test]
	fn what_the_new_spelling_requires_is_added() {
		assert_eq!(
			migrate("type Pair(\n\tLeft,\n\tRight\n)= { left: Left, right: Right }\n"),
			"type Pair<Left,\n\tRight\n> = { left: Left, right: Right }\n"
		);
		assert!(has_comparison("size > 255"));
		assert!(has_comparison("a < b"));
		assert!(!has_comparison("(a > b)"));
		assert!(!has_comparison("(x) => y"));
		assert!(!has_comparison("a -> b"));
		assert!(!has_comparison("\">\""));
	}

	#[test]
	fn offsets_map_back_to_the_original() {
		let migration = Migration {
			edits: vec![
				Edit {
					start: 5,
					end: 7,
					text: "<",
				},
				Edit {
					start: 10,
					end: 11,
					text: "> ",
				},
			],
			lists: 1,
			kept_calls: Vec::new(),
		};
		assert_eq!(migration.original_offset(3), 3);
		assert_eq!(migration.original_offset(5), 5);
		assert_eq!(migration.original_offset(6), 7);
		assert_eq!(migration.original_offset(8), 9);
		assert_eq!(migration.original_offset(9), 10);
		assert_eq!(migration.original_offset(12), 12);
	}
}
