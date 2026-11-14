//! Converts a syntax tree into a `Document`, with the comments of the source.

use std::rc::Rc;

use fa_parser::nodes::{
	ComparisonOperator, FunctionSignature, Name, PatternField, TestModifier, WhenBranch,
	WhenBranchPattern,
};
use fa_parser::{Node, TypedSyntaxTree};

use crate::Options;
use crate::comments::Comments;
use crate::concat;
use crate::document::{BreakParent, Document, HardLine, Line, Output, Parts, SoftLine};

pub struct Printer<'a> {
	tree: &'a TypedSyntaxTree,
	source: &'a str,
	comments: Comments<'a>,
	options: &'a Options,
	// true while printing a type, where `{ id = id }` is not an object literal
	in_type: bool,
	// the text of the module, printed statement after statement (see `sequence`)
	output: Output,
}

/// What separates the elements of a sequence.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Separator {
	/// New lines, ex: the statements of a block.
	NewLines,
	/// Commas on one line, new lines otherwise, ex: the items of a list.
	Commas,
}

/// A part of the source printed as a sequence: a statement, an item, a `when` branch...
#[derive(Clone, Copy)]
struct Element {
	start: usize,
	end: usize,
	// the element starts with a token that continues the previous line, like `.red`: the
	// element before it ends with a comma
	continues_line: bool,
}

/// The elements of a sequence, with their comments.
struct Sequence<'a> {
	// the comments after the opening bracket, on its line
	opening: Document<'a>,
	// the elements and their separators, to print after a new line
	body: Document<'a>,
	has_body: bool,
	// a comment or a blank line forces the elements onto separate lines
	breaks: bool,
}

/// A link of a chain of accesses and calls, ex: `.filter`, `(user)`, `[0]`.
enum Link {
	Member { name: usize, is_optional: bool },
	Call { node: usize },
	Index { node: usize },
}

impl<'a> Printer<'a> {
	pub fn new(tree: &'a TypedSyntaxTree, options: &'a Options) -> Self {
		Printer {
			tree,
			source: &tree.source,
			comments: Comments::new(tree),
			options,
			in_type: false,
			output: Output::new(options.width),
		}
	}

	/// The text of the whole module, ending with a new line (empty for an empty module), and the
	/// number of comments left unprinted. The statements are printed one after the other: the
	/// document of the whole module is never in memory.
	pub fn module(mut self) -> (String, usize) {
		let root = self.tree.root;
		let statements = match self.tree.at(root) {
			Node::Module { statements, .. } => statements.clone(),
			_ => vec![root],
		};
		let elements = self.statement_elements(&statements);
		let sequence = self.sequence(
			None,
			self.source.len(),
			&elements,
			Separator::NewLines,
			|printer, index| printer.node(statements[index]),
		);
		let remaining = self.comments.remaining();
		let mut parts = Parts::new();
		parts.push(sequence.opening);
		parts.push(sequence.body);
		if sequence.has_body {
			parts.push(HardLine);
		}
		self.output.print(&parts.build());
		(self.output.finish(), remaining)
	}

	/* ------------------------------- Helpers ------------------------------- */

	fn text(&self, index: usize) -> &'a str {
		let span = self.tree.span(index);
		self.source.get(span.start..span.end).unwrap_or("")
	}

	fn name(&self, name: Name) -> &'a str {
		self.source
			.get(name.start as usize..name.end as usize)
			.unwrap_or("")
	}

	fn start(&self, index: usize) -> usize {
		self.tree.span(index).start
	}

	fn end(&self, index: usize) -> usize {
		self.tree.span(index).end
	}

	fn has_newline(&self, from: usize, to: usize) -> bool {
		from < to && self.source.as_bytes()[from..to].contains(&b'\n')
	}

	fn has_blank_line(&self, from: usize, to: usize) -> bool {
		from < to
			&& self.source.as_bytes()[from..to]
				.iter()
				.filter(|byte| **byte == b'\n')
				.nth(1)
				.is_some()
	}

	/// The position of `byte` at or after `from`, skipping spaces, new lines, commas and
	/// comments; `from` if something else comes first.
	fn find(&self, from: usize, byte: u8) -> usize {
		let bytes = self.source.as_bytes();
		let mut position = from;
		while position < bytes.len() {
			match bytes[position] {
				found if found == byte => return position,
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
				_ => return from,
			}
		}
		from
	}

	/// The leftmost node of an expression, which holds its first token.
	fn leftmost(&self, index: usize) -> usize {
		match self.tree.at(index) {
			Node::Access { operands }
			| Node::OptionalAccess { operands }
			| Node::Add { operands }
			| Node::Subtract { operands }
			| Node::Multiply { operands }
			| Node::Divide { operands }
			| Node::IntegerDivide { operands }
			| Node::Remainder { operands }
			| Node::Fallback { operands }
			| Node::Power { operands }
			| Node::Equal { operands }
			| Node::NotEqual { operands }
			| Node::Comparison { operands, .. }
			| Node::And { operands }
			| Node::Or { operands }
			| Node::Union { operands }
			| Node::Compose { operands }
				if !operands.is_empty() =>
			{
				self.leftmost(operands[0])
			}
			Node::Tuple { items } if !items.is_empty() => self.leftmost(items[0]),
			Node::FunctionCall { function, .. } => self.leftmost(*function),
			Node::GenericApplication { target, .. } => self.leftmost(*target),
			Node::Index { target, .. } | Node::MemberIndex { target, .. } => self.leftmost(*target),
			Node::Is { left, .. }
			| Node::IsIn { left, .. }
			| Node::In { left, .. }
			| Node::Relation { left, .. } => self.leftmost(*left),
			Node::Range {
				start: Some(start), ..
			} => self.leftmost(*start),
			Node::Percentage { value } | Node::Optional { value } => self.leftmost(*value),
			Node::Catch { expression, .. } => self.leftmost(*expression),
			Node::PartialApplication { function, .. } => self.leftmost(*function),
			Node::Assignment { name, .. } => self.leftmost(*name),
			Node::CompoundAssignment { target, .. } => self.leftmost(*target),
			Node::Throws { signature, .. }
			| Node::Requires { signature, .. }
			| Node::Effect { signature, .. } => self.leftmost(*signature),
			_ => index,
		}
	}

	/// True if the node starts with a token that continues the previous line, like `.red`.
	fn continues_line(&self, index: usize) -> bool {
		matches!(
			self.tree.at(self.leftmost(index)),
			Node::ImplicitMember { .. } | Node::InRelation { .. }
		)
	}

	fn is_number_literal(&self, index: usize) -> bool {
		matches!(
			self.tree.at(index),
			Node::Integer(_) | Node::Number(_) | Node::BigInteger(_)
		)
	}

	fn element(&self, index: usize) -> Element {
		Element {
			start: self.start(index),
			end: self.end(index),
			continues_line: self.continues_line(index),
		}
	}

	fn statement_elements(&self, statements: &[usize]) -> Vec<Element> {
		statements
			.iter()
			.map(|statement| Element {
				continues_line: false,
				..self.element(*statement)
			})
			.collect()
	}

	fn item_elements(&self, items: &[usize]) -> Vec<Element> {
		items.iter().map(|item| self.element(*item)).collect()
	}

	/* ------------------------------- Comments ------------------------------- */

	/// Print the comments before `position` after what was printed: the first one on the same
	/// line, the next ones on their own lines. Used where a comment was written inside an
	/// expression, at a place that can't hold one: it moves to the end of the expression.
	fn trailing_comments(&mut self, parts: &mut Parts<'a>, position: usize) {
		let mut previous_is_line_comment = false;
		while let Some(comment) = self.comments.peek_before(position) {
			self.comments.take();
			if previous_is_line_comment {
				parts.push(HardLine);
			} else {
				parts.text(" ");
			}
			parts.push(self.comments.trailing_document(comment));
			previous_is_line_comment = self.comments.is_line_comment(comment);
			if previous_is_line_comment {
				parts.push(BreakParent);
			}
		}
	}

	/// Print the comments before `position` (written between two operands of an operation,
	/// between two links of a chain...) at the end of the line, followed by a new line: the
	/// caller must print a line break that is allowed there.
	fn comments_before_break(&mut self, parts: &mut Parts<'a>, position: usize) {
		let mut previous_is_line_comment = false;
		while let Some(comment) = self.comments.peek_before(position) {
			self.comments.take();
			if previous_is_line_comment {
				parts.push(HardLine);
			} else {
				parts.text(" ");
			}
			parts.push(self.comments.trailing_document(comment));
			previous_is_line_comment = self.comments.is_line_comment(comment);
			parts.push(BreakParent);
		}
	}

	/// Print a sequence of elements written between `open` (after the opening bracket, `None`
	/// for a module) and `close` (the closing bracket), with the comments between them.
	fn sequence(
		&mut self,
		open: Option<usize>,
		close: usize,
		elements: &[Element],
		separator: Separator,
		mut print: impl FnMut(&mut Self, usize) -> Document<'a>,
	) -> Sequence<'a> {
		let mut opening = Parts::new();
		let mut body = Parts::new();
		let mut breaks = false;
		let mut previous_end = open.unwrap_or(0);
		// the statements of a module are printed as soon as the next one starts (on a new line)
		let is_module = open.is_none();
		let mut printed = false;
		let first_start = elements.first().map_or(close, |element| element.start);

		// the comments on the line of the opening bracket
		if open.is_some() {
			while let Some(comment) = self.comments.peek_before(first_start.min(close)) {
				if self.has_newline(previous_end, comment.start) {
					break;
				}
				self.comments.take();
				opening.text(" ");
				opening.push(self.comments.trailing_document(comment));
				opening.push(BreakParent);
				breaks = true;
				previous_end = comment.end;
			}
		}

		for (position, element) in elements.iter().enumerate() {
			if is_module && !body.is_empty() {
				self.output.print(&std::mem::take(&mut body).build());
				printed = true;
			}
			// the comments on their own lines before the element
			while let Some(comment) = self.comments.peek_before(element.start.min(close)) {
				self.comments.take();
				if printed || !body.is_empty() {
					body.push(HardLine);
					if self.has_blank_line(previous_end, comment.start) {
						body.push(HardLine);
					}
				}
				body.push(self.comments.document(comment));
				breaks = true;
				previous_end = comment.end;
			}

			if printed || !body.is_empty() {
				match separator {
					Separator::NewLines => body.push(HardLine),
					Separator::Commas => body.push(Line),
				};
				if self.has_blank_line(previous_end, element.start) {
					body.push(HardLine);
					breaks = true;
				}
			}
			body.push(print(self, position));

			let next = elements.get(position + 1);
			if let Some(next) = next {
				let comma = if next.continues_line { "," } else { "" };
				match separator {
					Separator::Commas => body.push(Document::if_break(
						Document::text(comma),
						Document::text(","),
					)),
					Separator::NewLines => body.text(comma),
				};
			}

			// the comments left inside the element, and the ones after it on its line
			let next_start = next.map_or(close, |next| next.start).min(close);
			let mut end = element.end;
			let mut previous_is_line_comment = false;
			while let Some(comment) = self.comments.peek_before(next_start) {
				if comment.start >= element.end && self.has_newline(end, comment.start) {
					break;
				}
				self.comments.take();
				if previous_is_line_comment {
					body.push(HardLine);
				} else {
					body.text(" ");
				}
				body.push(self.comments.trailing_document(comment));
				previous_is_line_comment = self.comments.is_line_comment(comment);
				if previous_is_line_comment {
					body.push(BreakParent);
					breaks = true;
				}
				end = end.max(comment.end);
			}
			previous_end = end;
		}

		// the comments after the last element
		while let Some(comment) = self.comments.peek_before(close) {
			self.comments.take();
			if printed || !body.is_empty() {
				body.push(HardLine);
				if self.has_blank_line(previous_end, comment.start) {
					body.push(HardLine);
				}
			}
			body.push(self.comments.document(comment));
			breaks = true;
			previous_end = comment.end;
		}

		let has_body = printed || !body.is_empty();
		Sequence {
			opening: opening.build(),
			body: body.build(),
			has_body,
			breaks,
		}
	}

	/// A sequence of items between brackets, ex: `[a, b]` or `(a, b)`: on one line if it fits,
	/// otherwise one item per line. `spaced` puts spaces inside the brackets (`{ a }`).
	#[allow(clippy::too_many_arguments)]
	fn bracketed(
		&mut self,
		opening: &'static str,
		closing: &'static str,
		open: usize,
		close: usize,
		items: &[usize],
		spaced: bool,
		keep_broken: bool,
		mut print: impl FnMut(&mut Self, usize) -> Document<'a>,
	) -> Document<'a> {
		let elements = self.item_elements(items);
		self.bracketed_elements(
			opening,
			closing,
			open,
			close,
			&elements,
			spaced,
			keep_broken,
			|printer, position| print(printer, items[position]),
		)
	}

	#[allow(clippy::too_many_arguments)]
	fn bracketed_elements(
		&mut self,
		opening: &'static str,
		closing: &'static str,
		open: usize,
		close: usize,
		elements: &[Element],
		spaced: bool,
		keep_broken: bool,
		print: impl FnMut(&mut Self, usize) -> Document<'a>,
	) -> Document<'a> {
		let keep_broken = keep_broken
			&& self.has_newline(
				open,
				elements.first().map_or(close, |element| element.start),
			);
		let sequence = self.sequence(Some(open), close, elements, Separator::Commas, print);
		if !sequence.has_body {
			return empty_sequence(opening, sequence, closing);
		}
		// an item written over several lines puts every item on its own line
		let keep_broken = keep_broken || sequence.body.is_broken();
		let line = if spaced { Line } else { SoftLine };
		Document::group_breaking(
			concat![
				opening,
				sequence.opening,
				Document::indent(concat![line.clone(), sequence.body]),
				line,
				closing,
			],
			sequence.breaks || keep_broken,
		)
	}

	/// A list of nodes separated by `, ` on one line, ex: the paths of `mutates a, b`.
	fn comma_list(&mut self, items: &[usize]) -> Document<'a> {
		let mut parts = Parts::new();
		for (position, item) in items.iter().enumerate() {
			if position > 0 {
				parts.text(", ");
			}
			parts.push(self.node(*item));
		}
		parts.build()
	}

	/* -------------------------------- Nodes -------------------------------- */

	fn type_node(&mut self, index: usize) -> Document<'a> {
		let in_type = std::mem::replace(&mut self.in_type, true);
		let document = self.node(index);
		self.in_type = in_type;
		document
	}

	fn value_node(&mut self, index: usize) -> Document<'a> {
		let in_type = std::mem::replace(&mut self.in_type, false);
		let document = self.node(index);
		self.in_type = in_type;
		document
	}

	pub fn node(&mut self, index: usize) -> Document<'a> {
		let tree = self.tree;
		match tree.at(index) {
			Node::Module { statements, .. } | Node::Block { statements, .. } => {
				self.block(index, statements)
			}
			Node::Invalid
			| Node::Placeholder
			| Node::Identifier { .. }
			| Node::ImplicitMember { .. }
			| Node::Integer(_)
			| Node::Number(_)
			| Node::BigInteger(_)
			| Node::StringLiteral(_)
			| Node::StringTemplate { .. }
			| Node::None
			| Node::Error
			| Node::Boolean(_)
			| Node::Continue
			| Node::EmptyGroup => Document::text(self.literal(index)),

			Node::List { items } => {
				let span = tree.span(index);
				self.bracketed(
					"[",
					"]",
					span.start + 1,
					span.end - 1,
					items,
					false,
					true,
					|printer, item| printer.node(item),
				)
			}
			Node::Tuple { items } => self.comma_list(items),
			Node::Members { items } => {
				let span = tree.span(index);
				self.members(span.start + 1, span.end - 1, items)
			}

			Node::Negate { right } => {
				let leftmost = self.leftmost(*right);
				// `- 5` is not `-5`, the constant, and `--x` would be a comment
				let spaced = self.is_number_literal(leftmost)
					|| matches!(tree.at(*right), Node::Negate { .. });
				concat![if spaced { "- " } else { "-" }, self.node(*right)]
			}
			Node::Spread { right } => concat!["...", self.node(*right)],
			Node::Not { right } => concat!["not ", self.node(*right)],
			Node::Try { right } => concat!["try ", self.node(*right)],
			Node::Throw { right } => concat!["throw ", self.node(*right)],
			Node::Await { right } => concat!["await ", self.node(*right)],
			Node::Start { right } => concat!["start ", self.node(*right)],
			Node::Assert { right } => concat!["assert ", self.node(*right)],
			Node::InRelation { right } => concat!["in ", self.node(*right)],
			Node::Readonly { right } => concat!["readonly ", self.node(*right)],
			Node::Return { expression } => match expression {
				Some(expression) => concat!["return ", self.node(*expression)],
				None => Document::text("return"),
			},
			Node::Break { expression } => match expression {
				Some(expression) => concat!["break ", self.node(*expression)],
				None => Document::text("break"),
			},
			Node::Catch {
				expression,
				error_name,
				handler,
			} => concat![
				self.node(*expression),
				" catch ",
				self.name(*error_name),
				self.arrow_body(*handler),
			],
			Node::Provide {
				type_expression,
				expression,
			} => concat![
				"provide ",
				self.type_node(*type_expression),
				" = ",
				self.value_node(*expression),
			],

			Node::For {
				pattern,
				index: position,
				iterable,
				parallel,
				body,
				is_compile_time,
			} => {
				let mut parts = Parts::new();
				parts.text(if *is_compile_time { "@for" } else { "for" });
				if let Some(arguments) = parallel {
					parts.text(" parallel");
					if !arguments.is_empty() {
						let keyword_end = self.start(index)
							+ self
								.text(index)
								.find("parallel")
								.map_or(0, |offset| offset + 8);
						let open = self.find(keyword_end, b'(');
						let close = self.find(self.end(arguments[arguments.len() - 1]), b')');
						parts.push(self.bracketed(
							"(",
							")",
							open + 1,
							close,
							arguments,
							false,
							false,
							|printer, item| printer.node(item),
						));
					}
				}
				if let Some(pattern) = pattern {
					parts.text(" ");
					parts.push(self.node(*pattern));
					if let Some(position) = position {
						parts.text(", ");
						parts.text(self.name(*position));
					}
					parts.text(" in");
				}
				parts.text(" ");
				parts.push(self.node(*iterable));
				parts.text(" ");
				parts.push(self.node(*body));
				parts.build()
			}
			Node::While { expression, body } => {
				concat!["while ", self.node(*expression), " ", self.node(*body)]
			}
			Node::WhileLet {
				pattern,
				expression,
				body,
			} => concat![
				"while let ",
				self.node(*pattern),
				" = ",
				self.node(*expression),
				" ",
				self.node(*body),
			],
			Node::Loop { body } => concat!["loop ", self.node(*body)],
			Node::Do { body } => concat!["do ", self.node(*body)],
			Node::Watch { body } => concat!["watch ", self.node(*body)],
			Node::If {
				is_compile_time,
				condition,
				then_body,
				else_body,
			} => {
				let (if_keyword, else_keyword) = if *is_compile_time {
					("@if ", " @else ")
				} else {
					("if ", " else ")
				};
				let mut parts = Parts::new();
				parts.text(if_keyword);
				parts.push(self.node(*condition));
				parts.text(" ");
				parts.push(self.node(*then_body));
				if let Some(else_body) = else_body {
					parts.text(else_keyword);
					parts.push(self.node(*else_body));
				}
				parts.build()
			}
			Node::When {
				expression,
				branches,
			} => self.when(index, *expression, branches),

			Node::Add { operands } => self.operation(operands, "+"),
			Node::Multiply { operands } => self.operation(operands, "*"),
			Node::Divide { operands } => self.operation(operands, "/"),
			Node::IntegerDivide { operands } => self.operation(operands, "//"),
			Node::Remainder { operands } => self.operation(operands, "%%"),
			Node::Fallback { operands } => self.operation(operands, "??"),
			Node::Power { operands } => self.operation(operands, "**"),
			Node::Equal { operands } => self.operation(operands, "=="),
			Node::NotEqual { operands } => self.operation(operands, "!="),
			Node::And { operands } => self.operation(operands, "and"),
			Node::Or { operands } => self.operation(operands, "or"),
			// only the union of a type declaration breaks (see `type_declaration_value`)
			Node::Union { operands } => self.flat_operation(operands, " | "),
			Node::Compose { operands } => self.operation(operands, "then"),
			// a line can't start or end with a binary `-`: a subtraction stays on one line
			Node::Subtract { operands } => self.flat_operation(operands, " - "),
			Node::Comparison {
				operands,
				operators,
			} => {
				let operators: Vec<&'static str> =
					operators.iter().map(ComparisonOperator::symbol).collect();
				self.operation_with(operands, |position| {
					operators.get(position).copied().unwrap_or("<")
				})
			}
			Node::Is {
				left,
				right,
				negated,
			} => {
				let operator = if *negated { "is not" } else { "is" };
				self.operation_with(&[*left, *right], |_| operator)
			}
			Node::IsIn {
				left,
				right,
				negated,
			} => {
				// `score is in 0..60` is `score is 0..60` (values/ranges.md)
				let tests_a_range =
					self.options.rewrites && matches!(tree.at(*right), Node::Range { .. });
				let operator = match (*negated, tests_a_range) {
					(false, false) => "is in",
					(true, false) => "is not in",
					(false, true) => "is",
					(true, true) => "is not",
				};
				self.operation_with(&[*left, *right], |_| operator)
			}
			Node::In { left, right } => concat![self.node(*left), " in ", self.node(*right)],
			Node::Relation { left, right } => {
				concat![self.node(*left), "->", self.node(*right)]
			}
			Node::Range {
				start,
				end,
				inclusive,
			} => {
				let mut parts = Parts::new();
				if let Some(start) = start {
					parts.push(self.node(*start));
				}
				parts.text(if *inclusive { "..=" } else { ".." });
				if let Some(end) = end {
					parts.push(self.node(*end));
				}
				parts.build()
			}
			Node::PartialApplication {
				function,
				arguments,
			} => {
				let function_document = self.node(*function);
				let function_end = self.end(*function);
				let keyword_end = function_end
					+ self.source[function_end..]
						.find("with")
						.map_or(0, |offset| offset + 4);
				let open = self.find(keyword_end, b'(');
				let close = tree.span(index).end - 1;
				let arguments = self.bracketed(
					"(",
					")",
					open + 1,
					close,
					arguments,
					false,
					false,
					|printer, item| printer.node(item),
				);
				concat![function_document, " with ", arguments]
			}
			Node::Percentage { value } => concat![self.node(*value), "%"],
			Node::Optional { value } => concat![self.node(*value), "?"],

			Node::FunctionCall { .. } => match self.percentage(index) {
				Some(document) => document,
				None => self.chain(index),
			},
			Node::GenericApplication { target, arguments } => {
				let target_document = self.node(*target);
				let arguments_document = self.generic_list(arguments);
				concat![target_document, arguments_document]
			}
			Node::Access { .. } | Node::OptionalAccess { .. } | Node::Index { .. } => {
				self.chain(index)
			}
			Node::MemberIndex { target, members } => {
				let target_document = self.node(*target);
				let open = self.find(self.end(*target), b'{');
				let close = tree.span(index).end - 1;
				let members_document = self.members(open + 1, close, members);
				concat![target_document, " ", members_document]
			}

			Node::ObjectPattern { fields, rest } => self.object_pattern(index, fields, *rest),
			Node::ArrayPattern { items, rest } => {
				let span = tree.span(index);
				let mut all = items.clone();
				all.extend(rest.iter().copied());
				let rest = *rest;
				self.bracketed(
					"[",
					"]",
					span.start + 1,
					span.end - 1,
					&all,
					false,
					false,
					|printer, item| {
						if Some(item) == rest {
							concat!["...", printer.node(item)]
						} else {
							printer.node(item)
						}
					},
				)
			}
			Node::TuplePattern { items } => {
				let span = tree.span(index);
				self.bracketed(
					"(",
					")",
					span.start + 1,
					span.end - 1,
					items,
					false,
					false,
					|printer, item| printer.node(item),
				)
			}
			Node::Destructuring {
				kind,
				pattern,
				expression,
			} => concat![
				kind.keyword(),
				" ",
				self.node(*pattern),
				" = ",
				self.value_node(*expression),
			],

			Node::Group { expression } => concat!["(", self.node(*expression), ")"],

			Node::Let {
				name,
				type_expression,
				expression,
			} => self.declaration("let ", *name, *type_expression, *expression),
			Node::Mutable {
				name,
				type_expression,
				expression,
			} => self.declaration("mutable ", *name, *type_expression, *expression),
			Node::Use {
				name,
				type_expression,
				expression,
				is_mutable,
			} => self.declaration(
				if *is_mutable { "mutable use " } else { "use " },
				*name,
				*type_expression,
				Some(*expression),
			),
			Node::Reuse { name, expression } => {
				self.declaration("reuse ", *name, None, Some(*expression))
			}
			Node::Static {
				name,
				type_expression,
				expression,
			} => self.declaration("static ", *name, *type_expression, Some(*expression)),
			Node::Reactive {
				name,
				type_expression,
				expression,
			} => self.declaration("reactive ", *name, *type_expression, *expression),
			Node::Derived {
				name,
				type_expression,
				expression,
			} => self.declaration("derived ", *name, *type_expression, *expression),
			Node::Namespace { name, expression } => {
				self.declaration("namespace ", *name, None, Some(*expression))
			}
			Node::Test {
				name,
				modifier,
				body,
				is_compile_time,
			} => {
				let keyword = if *is_compile_time { "@test" } else { "test" };
				let modifier = match modifier {
					Some(TestModifier::Skip) => " skip",
					Some(TestModifier::Only) => " only",
					None => "",
				};
				concat![
					keyword,
					modifier,
					" ",
					self.node(*name),
					" ",
					self.node(*body)
				]
			}
			Node::Type {
				name,
				parameters,
				expression,
			} => {
				let mut parts = Parts::new();
				parts.text("type ");
				parts.text(self.name(*name));
				parts.push(self.generic_parameters(name.end as usize, parameters));
				parts.text(" = ");
				parts.push(self.type_declaration_value(*expression));
				parts.build()
			}
			Node::Function {
				name,
				is_async,
				type_expression,
				expression,
			} => {
				let mut parts = Parts::new();
				parts.text(if *is_async {
					"async function "
				} else {
					"function "
				});
				parts.text(self.name(*name));
				if let Some(type_expression) = type_expression {
					parts.text(": ");
					parts.push(self.type_node(*type_expression));
				}
				parts.text(" = ");
				parts.push(self.value_node(*expression));
				parts.build()
			}
			Node::ExportValue {
				is_mutable,
				type_expression,
				expression,
			} => {
				let mut parts = Parts::new();
				parts.text(if *is_mutable {
					"export mutable"
				} else {
					"export"
				});
				if let Some(type_expression) = type_expression {
					parts.text(": ");
					parts.push(self.type_node(*type_expression));
				}
				parts.text(" = ");
				parts.push(self.initializer(*type_expression, *expression));
				parts.build()
			}
			Node::ExportFunction {
				is_async,
				type_expression,
				expression,
			} => {
				let mut parts = Parts::new();
				parts.text(if *is_async {
					"export async function"
				} else {
					"export function"
				});
				if let Some(type_expression) = type_expression {
					parts.text(": ");
					parts.push(self.type_node(*type_expression));
				}
				parts.text(" = ");
				parts.push(self.value_node(*expression));
				parts.build()
			}
			Node::ExportType {
				parameters,
				expression,
			} => {
				let mut parts = Parts::new();
				parts.text("export type");
				let keyword_end = self.start(index)
					+ self.text(index).find("type").map_or(0, |offset| offset + 4);
				parts.push(self.generic_parameters(keyword_end, parameters));
				parts.text(" = ");
				parts.push(self.type_declaration_value(*expression));
				parts.build()
			}
			Node::ExportNamespace { expression } => {
				concat!["export namespace = ", self.value_node(*expression)]
			}
			Node::Parameters { items } => {
				let span = tree.span(index);
				self.bracketed(
					"(",
					")",
					span.start + 1,
					span.end - 1,
					items,
					false,
					false,
					|printer, item| printer.node(item),
				)
			}
			Node::DestructuredParameter {
				pattern,
				type_expression,
			} => self.destructured_parameter(*pattern, *type_expression),
			Node::ArrowFunction { signature, body } => self.arrow_function(index, signature, *body),
			Node::Throws { signature, errors } => {
				concat![self.node(*signature), " throws ", self.node(*errors)]
			}
			Node::Requires {
				signature,
				requirements,
			} => concat![
				self.node(*signature),
				" requires ",
				self.comma_list(requirements),
			],
			Node::Effect {
				signature,
				kind,
				paths,
			} => concat![
				self.node(*signature),
				" ",
				kind.keyword(),
				" ",
				self.comma_list(paths),
			],
			Node::CompoundAssignment {
				operator,
				target,
				expression,
			} => concat![
				self.node(*target),
				" ",
				operator.symbol(),
				" ",
				self.value_node(*expression),
			],
			Node::Assignment {
				name,
				type_expression,
				expression,
			} => {
				let mut parts = Parts::new();
				parts.push(self.node(*name));
				if let Some(type_expression) = type_expression {
					parts.text(": ");
					parts.push(self.type_node(*type_expression));
				}
				if let Some(expression) = expression {
					parts.text(" = ");
					parts.push(self.initializer(*type_expression, *expression));
				}
				parts.build()
			}
		}
	}

	/// A literal or a name, as written (numbers keep their digits, strings their escapes).
	fn literal(&self, index: usize) -> &'a str {
		match self.tree.at(index) {
			Node::None => "none",
			Node::Error => "Error",
			Node::Boolean(true) => "true",
			Node::Boolean(false) => "false",
			Node::Continue => "continue",
			Node::EmptyGroup => "()",
			Node::Placeholder => "...",
			_ => self.text(index),
		}
	}

	/// A declaration: `let name: Type = value`.
	fn declaration(
		&mut self,
		keyword: &'static str,
		name: Name,
		type_expression: Option<usize>,
		expression: Option<usize>,
	) -> Document<'a> {
		let mut parts = Parts::new();
		parts.text(keyword);
		parts.text(self.name(name));
		if let Some(type_expression) = type_expression {
			parts.text(": ");
			parts.push(self.type_node(type_expression));
		}
		if let Some(expression) = expression {
			parts.text(" = ");
			parts.push(self.initializer(type_expression, expression));
		}
		parts.build()
	}

	/// The value of a declaration. With the rewrites, an object literal of the declared type is
	/// written without the type: `let point: Point = Point { x = 1 }` is `let point: Point =
	/// { x = 1 }` (types/declaring-types.md).
	fn initializer(&mut self, type_expression: Option<usize>, expression: usize) -> Document<'a> {
		if self.options.rewrites
			&& let Some(type_expression) = type_expression
			&& let Node::MemberIndex { target, members } = self.tree.at(expression)
			&& self.tree.node_to_string(*target) == self.tree.node_to_string(type_expression)
		{
			let open = self.find(self.end(*target), b'{');
			let close = self.end(expression) - 1;
			// the comments written in the type are kept
			let mut parts = Parts::new();
			self.trailing_comments(&mut parts, open);
			let members =
				self.with_in_type(false, |printer| printer.members(open + 1, close, members));
			return concat![members, parts.build()];
		}
		self.value_node(expression)
	}

	fn with_in_type(
		&mut self,
		in_type: bool,
		print: impl FnOnce(&mut Self) -> Document<'a>,
	) -> Document<'a> {
		let saved = std::mem::replace(&mut self.in_type, in_type);
		let document = print(self);
		self.in_type = saved;
		document
	}

	/// The generic parameters of a declaration, ex: `<Item>`, written after `after`.
	fn generic_parameters(&mut self, _after: usize, parameters: &[usize]) -> Document<'a> {
		if parameters.is_empty() {
			return Document::empty();
		}
		self.generic_list(parameters)
	}

	/// A generic list, ex: `<Item, Value: Integer | Float>` or `<String, Integer>`. It stays on
	/// the line of its `<` (a `<` followed by a line break is a comparison): only its items
	/// break, like the object type of `Map<String, {\n\tid: Integer\n}>`.
	fn generic_list(&mut self, items: &[usize]) -> Document<'a> {
		let mut parts = Parts::new();
		parts.text("<");
		for (position, item) in items.iter().enumerate() {
			if position > 0 {
				parts.text(", ");
			}
			let document = match self.tree.at(*item) {
				// `min = 0` in `Integer<min = 0>`: a value
				Node::Assignment {
					type_expression: None,
					expression: Some(_),
					..
				} => self.value_node(*item),
				_ => self.type_node(*item),
			};
			parts.push(document);
		}
		parts.text(">");
		parts.build()
	}

	/// A block: `{` statements `}`. A block of a single statement written on one line stays on
	/// one line if it fits.
	fn block(&mut self, index: usize, statements: &[usize]) -> Document<'a> {
		let span = self.tree.span(index);
		let (open, close) = (
			span.start + 1,
			span.end.saturating_sub(1).max(span.start + 1),
		);
		let elements = self.statement_elements(statements);
		let one_line = statements.len() == 1
			&& !self.has_newline(span.start, span.end)
			&& !self.comments.any_before(close);
		let sequence = self.sequence(
			Some(open),
			close,
			&elements,
			Separator::NewLines,
			|printer, position| printer.value_node(statements[position]),
		);
		if !sequence.has_body {
			return empty_sequence("{", sequence, "}");
		}
		if one_line {
			return Document::group(concat![
				"{",
				Document::indent(concat![Line, sequence.body]),
				Line,
				"}",
			]);
		}
		concat![
			"{",
			sequence.opening,
			Document::indent(concat![HardLine, sequence.body]),
			HardLine,
			"}",
		]
	}

	/// The members of an object literal or of an object type: `{ a = 1, b: String }`.
	fn members(&mut self, open: usize, close: usize, items: &[usize]) -> Document<'a> {
		let rewrites = self.options.rewrites && !self.in_type;
		self.bracketed("{", "}", open, close, items, true, true, |printer, item| {
			// `{ id = id }` is `{ id }` (values/objects.md)
			if rewrites
				&& let Node::Assignment {
					name,
					type_expression: None,
					expression: Some(expression),
				} = printer.tree.at(item)
				&& let Node::Identifier { name: field } = printer.tree.at(*name)
				&& let Node::Identifier { name: value } = printer.tree.at(*expression)
				&& printer.name(*field) == printer.name(*value)
			{
				return Document::text(printer.name(*field));
			}
			printer.node(item)
		})
	}

	fn object_pattern(
		&mut self,
		index: usize,
		fields: &'a [PatternField],
		rest: Option<usize>,
	) -> Document<'a> {
		let span = self.tree.span(index);
		let mut elements: Vec<Element> = fields
			.iter()
			.map(|field| {
				let mut end = field.name.end as usize;
				if let Some(alias) = field.alias {
					end = end.max(alias.end as usize);
				}
				for child in [field.nested, field.default].into_iter().flatten() {
					end = end.max(self.end(child));
				}
				Element {
					start: field.name.start as usize,
					end,
					continues_line: false,
				}
			})
			.collect();
		if let Some(rest) = rest {
			elements.push(self.element(rest));
		}
		self.bracketed_elements(
			"{",
			"}",
			span.start + 1,
			span.end - 1,
			&elements,
			true,
			true,
			|printer, position| match fields.get(position) {
				Some(field) => printer.pattern_field(field),
				None => concat!["...", printer.node(rest.unwrap_or(index))],
			},
		)
	}

	fn pattern_field(&mut self, field: &PatternField) -> Document<'a> {
		let mut parts = Parts::new();
		parts.text(self.name(field.name));
		if let Some(alias) = field.alias {
			parts.text(" as ");
			parts.text(self.name(alias));
		}
		if let Some(nested) = field.nested {
			parts.text(" ");
			parts.push(self.node(nested));
		}
		if let Some(default) = field.default {
			parts.text(" = ");
			parts.push(self.value_node(default));
		}
		parts.build()
	}

	/// A destructured parameter, `{ x, y }: Point`. With the rewrites, a parameter destructured
	/// with an inline anonymous type is written as an anonymous structure parameter:
	/// `({ name }: { name: String })` is `({ name: String })` (functions/defining-functions.md).
	fn destructured_parameter(&mut self, pattern: usize, type_expression: usize) -> Document<'a> {
		if self.options.rewrites
			&& let Some(members) = self.anonymous_structure(pattern, type_expression)
		{
			let open = self.start(pattern) + 1;
			let close = self.end(type_expression) - 1;
			let items: Vec<usize> = members.iter().map(|(member, _)| *member).collect();
			let defaults: Vec<Option<usize>> =
				members.iter().map(|(_, default)| *default).collect();
			return self.bracketed(
				"{",
				"}",
				open,
				close,
				&items,
				true,
				false,
				|printer, item| {
					let position = items.iter().position(|member| *member == item).unwrap_or(0);
					let mut parts = Parts::new();
					parts.push(printer.node(item));
					if let Some(default) = defaults[position] {
						parts.text(" = ");
						parts.push(printer.value_node(default));
					}
					parts.build()
				},
			);
		}
		concat![self.node(pattern), ": ", self.type_node(type_expression)]
	}

	/// The members of the anonymous type of a destructured parameter, with the default value
	/// given by the pattern, when the parameter can be written as an anonymous structure: the
	/// pattern lists the fields of the type, without renaming nor nesting them.
	fn anonymous_structure(
		&self,
		pattern: usize,
		type_expression: usize,
	) -> Option<Vec<(usize, Option<usize>)>> {
		let Node::ObjectPattern { fields, rest: None } = self.tree.at(pattern) else {
			return None;
		};
		let Node::Members { items } = self.tree.at(type_expression) else {
			return None;
		};
		if fields.len() != items.len() || self.comments.any_before(self.end(type_expression)) {
			return None;
		}
		let mut members = Vec::with_capacity(items.len());
		for item in items {
			let Node::Assignment {
				name,
				type_expression: Some(_),
				expression: None,
			} = self.tree.at(*item)
			else {
				return None;
			};
			let Node::Identifier { name } = self.tree.at(*name) else {
				return None;
			};
			let field = fields
				.iter()
				.find(|field| self.name(field.name) == self.name(*name))?;
			if field.alias.is_some() || field.nested.is_some() {
				return None;
			}
			members.push((*item, field.default));
		}
		Some(members)
	}

	/// A lambda, or a function type: `(a: Integer): Integer => a + 1`.
	fn arrow_function(
		&mut self,
		index: usize,
		signature: &'a FunctionSignature,
		body: usize,
	) -> Document<'a> {
		let mut parts = Parts::new();
		if signature.is_async {
			parts.text("async ");
		}
		// `<Item>(…)`: the parentheses come after the generic list
		let parameters_from = match signature.generic_parameters.last() {
			Some(last) => {
				parts.push(self.generic_list(&signature.generic_parameters));
				self.find(self.end(*last), b'>') + 1
			}
			None => self.start(index) + if signature.is_async { 5 } else { 0 },
		};
		let open = self.find(parameters_from, b'(');
		let after_parameters = signature
			.parameters
			.last()
			.map_or(open + 1, |parameter| self.end(*parameter));
		let close = self.find(after_parameters, b')');
		parts.push(self.bracketed(
			"(",
			")",
			open + 1,
			close,
			&signature.parameters,
			false,
			false,
			|printer, item| printer.node(item),
		));
		// the return type and the clauses written on one line stay on one line when there are
		// parameters to break instead: `(\n\trequest: Context\n): { ...Context, ...Logged } => {`
		let clauses_end = [
			signature.return_type_expression,
			signature.errors,
			signature.requirements.last().copied(),
			signature.mutations.last().copied(),
			signature.invalidations.last().copied(),
		]
		.into_iter()
		.flatten()
		.map(|clause| self.end(clause))
		.max();
		let flat_clauses = clauses_end.is_some_and(|end| {
			!signature.parameters.is_empty()
				&& !self.has_newline(close, end)
				&& !self.comments.any_in(close, end)
		});
		let mut clauses = Parts::new();
		if let Some(return_type) = signature.return_type_expression {
			clauses.text(": ");
			clauses.push(self.type_node(return_type));
		}
		if let Some(errors) = signature.errors {
			clauses.text(" throws ");
			clauses.push(self.type_node(errors));
		}
		for (keyword, items) in [
			(" requires ", &signature.requirements),
			(" mutates ", &signature.mutations),
			(" invalidates ", &signature.invalidations),
		] {
			if !items.is_empty() {
				clauses.text(keyword);
				clauses.push(self.comma_list(items));
			}
		}
		let clauses = clauses.build();
		parts.push(if flat_clauses { flat(clauses) } else { clauses });
		parts.push(self.arrow_body(body));
		parts.build()
	}

	/// ` => body`, after a signature, a `catch` or a `when` pattern. A body that doesn't start
	/// with a bracket goes to the next line if it doesn't fit.
	fn arrow_body(&mut self, body: usize) -> Document<'a> {
		if self.in_type {
			// a function type: `(a: Integer) => Integer`
			return concat![" => ", self.node(body)];
		}
		// an object or a list stays after `=>` and breaks at its brackets, like when it is
		// written over several lines: moving it to the next line would not be idempotent
		let opens_a_block = matches!(
			self.tree.at(body),
			Node::Block { .. }
				| Node::When { .. }
				| Node::Do { .. }
				| Node::Loop { .. }
				| Node::Watch { .. }
				| Node::Members { .. }
				| Node::List { .. }
				| Node::MemberIndex { .. }
		);
		// a comment after `=>`
		let mut comments = Parts::new();
		self.comments_before_break(&mut comments, self.start(body));
		let document = self.value_node(body);
		if !comments.is_empty() {
			return concat![
				" =>",
				Document::indent(concat![comments.build(), HardLine, document]),
			];
		}
		if opens_a_block || document.is_broken() {
			concat![" => ", document]
		} else {
			concat![
				" =>",
				Document::group(Document::indent_if_break(concat![Line, document])),
			]
		}
	}

	/// `when subject is { pattern => value ... }`, one branch per line.
	fn when(
		&mut self,
		index: usize,
		expression: usize,
		branches: &'a [WhenBranch],
	) -> Document<'a> {
		let subject = self.node(expression);
		let span = self.tree.span(index);
		let open = self.find(
			self.end(expression)
				+ self.source[self.end(expression)..span.end]
					.find("is")
					.map_or(0, |offset| offset + 2),
			b'{',
		);
		let close = span.end - 1;
		let mut previous_end = open + 1;
		let elements: Vec<Element> = branches
			.iter()
			.map(|branch| {
				let start = match branch.pattern {
					WhenBranchPattern::Expression(pattern) => self.start(pattern),
					WhenBranchPattern::Else => {
						let before = branch
							.binding
							.map_or(self.start(branch.value), |binding| self.start(binding));
						self.source[previous_end..before]
							.rfind("else")
							.map_or(previous_end, |offset| previous_end + offset)
					}
				};
				let element = Element {
					start,
					end: self.end(branch.value),
					continues_line: match branch.pattern {
						WhenBranchPattern::Expression(pattern) => self.continues_line(pattern),
						WhenBranchPattern::Else => false,
					},
				};
				previous_end = element.end;
				element
			})
			.collect();
		let sequence = self.sequence(
			Some(open + 1),
			close,
			&elements,
			Separator::NewLines,
			|printer, position| {
				let branch = &branches[position];
				let mut parts = Parts::new();
				match branch.pattern {
					WhenBranchPattern::Expression(pattern) => parts.push(printer.node(pattern)),
					WhenBranchPattern::Else => parts.text("else"),
				};
				if let Some(binding) = branch.binding {
					parts.text(" as ");
					parts.push(printer.node(binding));
				}
				parts.push(printer.arrow_body(branch.value));
				parts.build()
			},
		);
		if !sequence.has_body {
			return concat!["when ", subject, " is ", empty_sequence("{", sequence, "}")];
		}
		// a single branch written on one line stays on one line if it fits, like a block of a
		// single statement: `when error is { IndexError => 0 }`
		if branches.len() == 1
			&& !self.has_newline(open, span.end)
			&& !self.comments.any_before(close)
		{
			return concat![
				"when ",
				subject,
				" is ",
				Document::group(concat![
					"{",
					Document::indent(concat![Line, sequence.body]),
					Line,
					"}",
				]),
			];
		}
		concat![
			"when ",
			subject,
			" is {",
			sequence.opening,
			Document::indent(concat![HardLine, sequence.body]),
			HardLine,
			"}",
		]
	}

	/// A chain of the same operator: `a + b + c`, on one line if it fits, otherwise with an
	/// operand per line, after the operator.
	fn operation(&mut self, operands: &[usize], operator: &'static str) -> Document<'a> {
		self.operation_with(operands, |_| operator)
	}

	fn operation_with(
		&mut self,
		operands: &[usize],
		operator: impl Fn(usize) -> &'static str,
	) -> Document<'a> {
		let Some((first, rest)) = operands.split_first() else {
			return Document::empty();
		};
		let first = self.node(*first);
		let mut tail = Parts::new();
		let mut has_comments = false;
		let mut operands_break = first.breaks();
		for (position, operand) in rest.iter().enumerate() {
			let mut comments = Parts::new();
			self.comments_before_break(&mut comments, self.start(*operand));
			has_comments |= !comments.is_empty();
			tail.push(comments.build());
			tail.push(Line);
			tail.text(operator(position));
			tail.text(" ");
			let operand = self.node(*operand);
			operands_break |= operand.breaks();
			tail.push(operand);
		}
		if operands_break && !has_comments {
			// an operand with a block, like `a + do {`: the operators stay on their lines
			return concat![first, flatten_lines(tail.build())];
		}
		Document::group(concat![first, Document::indent_if_break(tail.build())])
	}

	/// The operands of an operation on one line: `a - b`.
	fn flat_operation(&mut self, operands: &[usize], operator: &'static str) -> Document<'a> {
		let mut parts = Parts::new();
		for (position, operand) in operands.iter().enumerate() {
			if position > 0 {
				parts.text(operator);
			}
			parts.push(self.node(*operand));
		}
		parts.build()
	}

	/// The value of a type declaration: a union breaks before each `|` if it is too long (and has
	/// no object written over several lines).
	fn type_declaration_value(&mut self, expression: usize) -> Document<'a> {
		self.with_in_type(true, |printer| {
			let Node::Union { operands } = printer.tree.at(expression) else {
				return printer.node(expression);
			};
			let mut has_comments = false;
			let mut printed = Vec::with_capacity(operands.len());
			for (position, operand) in operands.iter().enumerate() {
				let mut comments = Parts::new();
				if position > 0 {
					printer.comments_before_break(&mut comments, printer.start(*operand));
				}
				has_comments |= !comments.is_empty();
				printed.push((comments.build(), printer.node(*operand)));
			}
			let flat = !has_comments && printed.iter().any(|(_, operand)| operand.breaks());
			let mut tail = Parts::new();
			let mut printed = printed.into_iter();
			let (_, first) = printed
				.next()
				.unwrap_or((Document::empty(), Document::empty()));
			for (comments, operand) in printed {
				tail.push(comments);
				tail.push(if flat { Document::text(" ") } else { Line });
				tail.text("| ");
				tail.push(operand);
			}
			if flat {
				concat![first, tail.build()]
			} else {
				Document::group(concat![first, Document::indent_if_break(tail.build())])
			}
		})
	}

	/* -------------------------------- Chains -------------------------------- */

	/// Flatten a chain of accesses, calls and indices: `a.b(c)[0]` is `a` and the links `.b`,
	/// `(c)`, `[0]`.
	fn flatten_chain(&self, index: usize, links: &mut Vec<Link>) -> usize {
		match self.tree.at(index) {
			Node::Access { operands } | Node::OptionalAccess { operands }
				if !operands.is_empty() =>
			{
				let is_optional = matches!(self.tree.at(index), Node::OptionalAccess { .. });
				let head = self.flatten_chain(operands[0], links);
				for operand in &operands[1..] {
					links.push(Link::Member {
						name: *operand,
						is_optional,
					});
				}
				head
			}
			Node::FunctionCall { function, .. } => {
				let head = self.flatten_chain(*function, links);
				links.push(Link::Call { node: index });
				head
			}
			Node::Index { target, .. } => {
				let head = self.flatten_chain(*target, links);
				links.push(Link::Index { node: index });
				head
			}
			_ => index,
		}
	}

	/// A chain of accesses and calls. A chain with several calls, like `users.filter(f).map(g)`,
	/// is broken before each `.` if it doesn't fit, or if it was written that way.
	fn chain(&mut self, index: usize) -> Document<'a> {
		let mut links = Vec::new();
		let head = self.flatten_chain(index, &mut links);

		// `-2.4.abs()` is `(-2.4).abs()` (values/numbers.md)
		let head_document = if self.options.rewrites
			&& self.is_number_literal(head)
			&& self.text(head).starts_with('-')
			&& matches!(links.first(), Some(Link::Member { .. }))
		{
			concat!["(", self.node(head), ")"]
		} else {
			self.node(head)
		};

		// the links before the `.member` of the first call stay with the head
		let split = match links
			.iter()
			.position(|link| matches!(link, Link::Call { .. }))
		{
			None => links.len(),
			Some(call) if call > 0 && matches!(links[call - 1], Link::Member { .. }) => call - 1,
			Some(call) => links[call..]
				.iter()
				.position(|link| matches!(link, Link::Member { .. }))
				.map_or(links.len(), |position| call + position),
		};
		let segments_start = split;
		let calls = (segments_start..links.len())
			.filter(|position| {
				matches!(links[*position], Link::Member { .. })
					&& matches!(links.get(position + 1), Some(Link::Call { .. }))
			})
			.count();
		let written_broken = links[segments_start..].iter().any(|link| match link {
			Link::Member { name, .. } => {
				let dot = self.dot_before(*name);
				self.source[..dot]
					.trim_end_matches([' ', '\t'])
					.ends_with('\n')
			}
			_ => false,
		});
		let breakable = calls >= 2 || (written_broken && calls >= 1);

		let mut parts = Parts::new();
		parts.push(head_document);
		for link in &links[..segments_start] {
			let link = self.link(link);
			parts.push(link);
		}
		if !breakable {
			for link in &links[segments_start..] {
				let link = self.link(link);
				parts.push(link);
			}
			return parts.build();
		}

		let mut tail = Parts::new();
		for link in &links[segments_start..] {
			if let Link::Member { name, .. } = link {
				let dot = self.dot_before(*name);
				self.comments_before_break(&mut tail, dot);
				tail.push(SoftLine);
			}
			let link = self.link(link);
			tail.push(link);
		}
		parts.push(Document::indent_if_break(tail.build()));
		Document::group_breaking(parts.build(), written_broken)
	}

	/// The position of the `.` or `?.` before the name of a member.
	fn dot_before(&self, name: usize) -> usize {
		let start = self.start(name);
		self.source[..start]
			.trim_end_matches([' ', '\t', '\n'])
			.trim_end_matches(['.', '?'])
			.len()
	}

	fn link(&mut self, link: &Link) -> Document<'a> {
		match *link {
			Link::Member { name, is_optional } => {
				concat![if is_optional { "?." } else { "." }, self.node(name)]
			}
			Link::Index { node } => {
				let Node::Index { index, .. } = self.tree.at(node) else {
					return Document::empty();
				};
				concat!["[", self.node(*index), "]"]
			}
			Link::Call { node } => self.call(node),
		}
	}

	/// The arguments of a call: `(a, b)`. A last argument that is a lambda with a block or an
	/// object literal "hugs" the parentheses: `items.map((item) => {` ... `})`.
	fn call(&mut self, node: usize) -> Document<'a> {
		let tree = self.tree;
		let Node::FunctionCall {
			function,
			parameters,
		} = tree.at(node)
		else {
			return Document::empty();
		};

		// `Percentage(value)` is `(value)%` (values/numbers.md): handled by `percentage`
		let open = self.find(self.end(*function), b'(');
		let close = self.end(node) - 1;
		if self.in_type && !self.comments.any_before(close) {
			// the arguments of a type, like `Map(String, Integer)`, stay on one line
			return concat!["(", self.comma_list(parameters), ")"];
		}
		// a lambda with a block or an object as the last argument (or a lambda with a block as
		// the first one) hugs the parentheses; the comments inside it are printed with it, the
		// others need the sequence of the arguments
		let count = parameters.len();
		let hugged_position = match (parameters.first(), parameters.last()) {
			(_, Some(last))
				if self.is_huggable(*last)
					&& !self.comments.any_in(open, self.start(*last))
					&& !self.comments.any_in(self.end(*last), close) =>
			{
				Some(count - 1)
			}
			(Some(first), _)
				if count >= 2
					&& self.is_block_lambda(*first)
					&& !self.comments.any_in(open, self.start(*first))
					&& !self.comments.any_in(self.end(*first), close) =>
			{
				Some(0)
			}
			_ => None,
		};
		let Some(hugged_position) = hugged_position else {
			return self.bracketed(
				"(",
				")",
				open + 1,
				close,
				parameters,
				false,
				false,
				|printer, item| printer.node(item),
			);
		};
		let mut arguments: Vec<Document<'a>> =
			parameters.iter().map(|item| self.node(*item)).collect();
		let others_break = arguments
			.iter()
			.enumerate()
			.any(|(position, argument)| position != hugged_position && argument.is_broken());
		if !others_break {
			// the arguments are in both layouts: shared, never copied, else the document of
			// nested calls (`f({ a = g({ … }) })`) would double at each level
			arguments = arguments.into_iter().map(Document::shared).collect();
		}

		let mut standard = Parts::new();
		for (position, argument) in arguments.iter_mut().enumerate() {
			if position > 0 {
				standard.push(Line);
			}
			// a shared document is cloned in constant time
			standard.push(if others_break {
				std::mem::replace(argument, Document::empty())
			} else {
				argument.clone()
			});
			if let Some(next) = parameters.get(position + 1) {
				let comma = if self.continues_line(*next) { "," } else { "" };
				standard.push(Document::if_break(
					Document::text(comma),
					Document::text(","),
				));
			}
		}
		let standard = Document::group(concat![
			"(",
			Document::indent(concat![SoftLine, standard.build()]),
			SoftLine,
			")",
		]);
		if others_break {
			return standard;
		}

		let mut hugged = Parts::new();
		hugged.text("(");
		for (position, argument) in arguments.into_iter().enumerate() {
			if position > 0 {
				hugged.text(", ");
			}
			if position == hugged_position {
				hugged.push(Document::break_last(argument));
			} else {
				hugged.push(argument);
			}
		}
		hugged.text(")");
		Document::Choice {
			standard: Box::new(standard),
			hugged: Box::new(hugged.build()),
		}
	}

	fn is_block_lambda(&self, index: usize) -> bool {
		matches!(self.tree.at(index), Node::ArrowFunction { body, .. } if matches!(self.tree.at(*body), Node::Block { .. }))
	}

	fn is_huggable(&self, index: usize) -> bool {
		match self.tree.at(index) {
			Node::ArrowFunction { body, .. } => matches!(self.tree.at(*body), Node::Block { .. }),
			Node::Members { .. }
			| Node::List { .. }
			| Node::MemberIndex { .. }
			| Node::When { .. }
			| Node::Do { .. } => true,
			_ => false,
		}
	}

	/// `Percentage(value)`, which the rewrites write `(value)%`, or `None`.
	pub fn percentage(&mut self, index: usize) -> Option<Document<'a>> {
		let Node::FunctionCall {
			function,
			parameters,
		} = self.tree.at(index)
		else {
			return None;
		};
		let [value] = parameters.as_slice() else {
			return None;
		};
		if !self.options.rewrites
			|| self.text(*function) != "Percentage"
			|| matches!(
				self.tree.at(*value),
				Node::Assignment { .. } | Node::Spread { .. }
			) || self.comments.any_before(self.end(index))
		{
			return None;
		}
		let plain = matches!(self.tree.at(*value), Node::Group { .. })
			|| (self.is_number_literal(*value)
				&& !self.text(*value).starts_with('-')
				&& !matches!(self.tree.at(*value), Node::BigInteger(_)));
		let value = self.node(*value);
		Some(if plain {
			concat![value, "%"]
		} else {
			concat!["(", value, ")%"]
		})
	}
}

/// Brackets with only comments between them: `{ -- comment` and `}` on the next line.
fn empty_sequence<'a>(
	opening: &'static str,
	sequence: Sequence<'a>,
	closing: &'static str,
) -> Document<'a> {
	if sequence.breaks {
		concat![opening, sequence.opening, HardLine, closing]
	} else {
		concat![opening, sequence.opening, closing]
	}
}

/// The document printed on one line: its `Line`s are spaces, its groups flat. Only for code
/// written on one line, without comments (no hard line inside).
fn flat(document: Document) -> Document {
	match document {
		Line => Document::text(" "),
		SoftLine => Document::empty(),
		Document::Concat(parts) => Document::Concat(parts.into_iter().map(flat).collect()),
		Document::Group { contents, .. }
		| Document::Indent(contents)
		| Document::IndentIfBreak(contents)
		| Document::BreakLast(contents) => flat(*contents),
		Document::IfBreak { flat: contents, .. } => flat(*contents),
		Document::Choice { standard, hugged } => {
			// the hugged layout shares the arguments: without it, they are no longer shared
			drop(hugged);
			flat(*standard)
		}
		Document::Shared(contents) => flat(Rc::unwrap_or_clone(contents)),
		document => document,
	}
}

/// The document with its `Line`s (outside of groups) written as spaces.
fn flatten_lines(document: Document) -> Document {
	match document {
		Line => Document::text(" "),
		Document::Concat(parts) => Document::Concat(parts.into_iter().map(flatten_lines).collect()),
		document => document,
	}
}
