//! The layout engine: a document of text, possible line breaks and groups, printed in the
//! narrowest way that fits the line width (the algorithm of Wadler's "prettier printer", as in
//! Prettier).
//!
//! A `Group` is printed on one line ("flat") if all of it fits in the rest of the line, otherwise
//! its `Line`s become new lines ("broken"), while its inner groups decide again for themselves.
//! A `HardLine` always breaks, and so does every group around it.

use std::borrow::Cow;

/// The width of a tab, to measure the lines.
pub const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone)]
pub enum Document<'a> {
	/// Text without a new line, except a string literal or a comment written over several lines,
	/// which is printed as it is.
	Text(Cow<'a, str>),
	/// A comment at the end of a line: printed like text, but it doesn't count in the width, so
	/// that a long comment doesn't break the code before it.
	TrailingComment(Cow<'a, str>),
	Concat(Vec<Document<'a>>),
	Group {
		contents: Box<Document<'a>>,
		// printed broken: a hard line inside, or broken on purpose (written over several lines)
		breaks: bool,
		// a hard line inside, which breaks the groups around too
		hard: bool,
	},
	Indent(Box<Document<'a>>),
	/// Indented only if the group around it is broken: the operands of an operation, whose
	/// object written over several lines keeps the indentation of the line it starts on.
	IndentIfBreak(Box<Document<'a>>),
	/// A space in a flat group, a new line in a broken one.
	Line,
	/// Nothing in a flat group, a new line in a broken one.
	SoftLine,
	/// Always a new line.
	HardLine,
	/// Different contents in a broken and in a flat group, ex: a comma that separates items only
	/// when they are on one line.
	IfBreak {
		broken: Box<Document<'a>>,
		flat: Box<Document<'a>>,
	},
	/// Breaks the groups around it, without printing anything (after a line comment).
	BreakParent,
	/// A call whose last argument can "hug" the parentheses, ex: a lambda with a block body.
	/// `standard` (a group) is printed flat if it fits; otherwise `hugged` is printed if its first
	/// line fits, else `standard`, broken.
	Choice {
		standard: Box<Document<'a>>,
		hugged: Box<Document<'a>>,
	},
}

pub use Document::{BreakParent, HardLine, Line, SoftLine};

impl<'a> Document<'a> {
	pub fn text(text: impl Into<Cow<'a, str>>) -> Self {
		Document::Text(text.into())
	}

	pub fn empty() -> Self {
		Document::Concat(Vec::new())
	}

	pub fn group(contents: Document<'a>) -> Self {
		Document::group_breaking(contents, false)
	}

	/// A group, broken whatever its width if `breaks` is true. Unlike a hard line, this doesn't
	/// break the groups around it.
	pub fn group_breaking(contents: Document<'a>, breaks: bool) -> Self {
		let hard = contents.breaks();
		Document::Group {
			contents: Box::new(contents),
			breaks: breaks || hard,
			hard,
		}
	}

	pub fn indent(contents: Document<'a>) -> Self {
		Document::Indent(Box::new(contents))
	}

	pub fn indent_if_break(contents: Document<'a>) -> Self {
		Document::IndentIfBreak(Box::new(contents))
	}

	pub fn if_break(broken: Document<'a>, flat: Document<'a>) -> Self {
		Document::IfBreak {
			broken: Box::new(broken),
			flat: Box::new(flat),
		}
	}

	/// True if the document contains a hard line (outside of an `IfBreak`), which breaks every
	/// group around it. Constant time for a group.
	pub fn breaks(&self) -> bool {
		match self {
			Document::Text(_)
			| Document::TrailingComment(_)
			| Document::Line
			| Document::SoftLine
			| Document::IfBreak { .. } => false,
			Document::HardLine | Document::BreakParent => true,
			Document::Group { hard, .. } => *hard,
			Document::Indent(contents) | Document::IndentIfBreak(contents) => contents.breaks(),
			Document::Concat(parts) => parts.iter().any(Document::breaks),
			// the hugged argument is broken on purpose: only a hard line inside breaks the groups
			Document::Choice { standard, .. } => standard.breaks(),
		}
	}

	/// True if the document is printed broken whatever the width: a hard line, or a group broken
	/// on purpose.
	pub fn is_broken(&self) -> bool {
		match self {
			Document::Group { breaks, .. } => *breaks,
			Document::Indent(contents) | Document::IndentIfBreak(contents) => contents.is_broken(),
			Document::Concat(parts) => parts.iter().any(Document::is_broken),
			Document::Choice { standard, .. } => standard.is_broken(),
			document => document.breaks(),
		}
	}

	/// True if the document contains a group broken whatever the width (at any depth), so it
	/// can't be printed flat: a call whose argument is a list written over several lines.
	pub fn has_broken_group(&self) -> bool {
		match self {
			Document::Group {
				breaks, contents, ..
			} => *breaks || contents.has_broken_group(),
			Document::Indent(contents) | Document::IndentIfBreak(contents) => {
				contents.has_broken_group()
			}
			Document::Concat(parts) => parts.iter().any(Document::has_broken_group),
			Document::Choice { standard, .. } => standard.has_broken_group(),
			document => document.breaks(),
		}
	}
}

/// A list of documents, built piece by piece.
#[derive(Debug, Default)]
pub struct Parts<'a> {
	parts: Vec<Document<'a>>,
}

impl<'a> Parts<'a> {
	pub fn new() -> Self {
		Parts { parts: Vec::new() }
	}

	pub fn push(&mut self, document: Document<'a>) -> &mut Self {
		match document {
			Document::Concat(parts) => self.parts.extend(parts),
			document => self.parts.push(document),
		}
		self
	}

	pub fn text(&mut self, text: impl Into<Cow<'a, str>>) -> &mut Self {
		self.parts.push(Document::Text(text.into()));
		self
	}

	pub fn is_empty(&self) -> bool {
		self.parts.is_empty()
	}

	pub fn build(self) -> Document<'a> {
		Document::Concat(self.parts)
	}
}

/// Concatenate documents: `concat![a, b, c]`.
#[macro_export]
macro_rules! concat {
	($($part:expr),* $(,)?) => {{
		let mut parts = $crate::document::Parts::new();
		$(parts.push($crate::document::IntoDocument::into_document($part));)*
		parts.build()
	}};
}

pub trait IntoDocument<'a> {
	fn into_document(self) -> Document<'a>;
}

impl<'a> IntoDocument<'a> for Document<'a> {
	fn into_document(self) -> Document<'a> {
		self
	}
}

impl<'a> IntoDocument<'a> for &'a str {
	fn into_document(self) -> Document<'a> {
		Document::Text(Cow::Borrowed(self))
	}
}

impl<'a> IntoDocument<'a> for String {
	fn into_document(self) -> Document<'a> {
		Document::Text(Cow::Owned(self))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
	Flat,
	Break,
}

/// Print a document, breaking its groups to keep the lines within `width` columns (tabs count
/// for `TAB_WIDTH`). Lines never end with spaces or tabs.
pub fn print(document: &Document, width: usize) -> String {
	let mut output = String::new();
	// the column of the end of the output
	let mut column = 0usize;
	let mut stack: Vec<(usize, Mode, &Document)> = vec![(0, Mode::Break, document)];

	while let Some((indentation, mode, document)) = stack.pop() {
		match document {
			Document::Text(text) | Document::TrailingComment(text) => {
				output.push_str(text);
				match text.rfind('\n') {
					Some(position) => column = text[position + 1..].chars().count(),
					None => column += text.chars().count(),
				}
			}
			Document::Concat(parts) => {
				for part in parts.iter().rev() {
					stack.push((indentation, mode, part));
				}
			}
			Document::Indent(contents) => stack.push((indentation + 1, mode, contents)),
			Document::IndentIfBreak(contents) => {
				let indentation = indentation + usize::from(mode == Mode::Break);
				stack.push((indentation, mode, contents));
			}
			Document::Group {
				contents, breaks, ..
			} => {
				let group_mode = if *breaks {
					Mode::Break
				} else if mode == Mode::Flat
					|| fits(contents, Mode::Flat, &stack, width.saturating_sub(column))
				{
					Mode::Flat
				} else {
					Mode::Break
				};
				stack.push((indentation, group_mode, contents));
			}
			Document::Choice { standard, hugged } => {
				let remaining = width.saturating_sub(column);
				if mode == Mode::Flat
					|| (!standard.has_broken_group()
						&& fits(standard, Mode::Flat, &stack, remaining))
				{
					stack.push((indentation, mode, standard));
				} else if fits(hugged, Mode::Break, &stack, remaining) {
					stack.push((indentation, Mode::Break, hugged));
				} else {
					stack.push((indentation, Mode::Break, standard));
				}
			}
			Document::IfBreak { broken, flat } => {
				stack.push((
					indentation,
					mode,
					if mode == Mode::Break { broken } else { flat },
				));
			}
			Document::Line | Document::SoftLine if mode == Mode::Flat => {
				if matches!(document, Document::Line) {
					output.push(' ');
					column += 1;
				}
			}
			Document::Line | Document::SoftLine | Document::HardLine => {
				let trimmed = output.trim_end_matches([' ', '\t']).len();
				output.truncate(trimmed);
				output.push('\n');
				for _ in 0..indentation {
					output.push('\t');
				}
				column = indentation * TAB_WIDTH;
			}
			Document::BreakParent => {}
		}
	}
	let trimmed = output.trim_end_matches([' ', '\t']).len();
	output.truncate(trimmed);
	output
}

/// True if the first line of `document`, printed in `mode`, fits in `width` columns, followed by
/// the rest of the line (the documents of `rest` until their first new line). The groups of
/// `document` are measured flat, unless they must break; the groups of `rest` keep their mode.
fn fits(document: &Document, mode: Mode, rest: &[(usize, Mode, &Document)], width: usize) -> bool {
	let mut remaining = width as isize;
	// the mode, the document, and whether it is part of `rest`
	let mut stack: Vec<(Mode, &Document, bool)> = vec![(mode, document, false)];
	let mut rest_index = rest.len();

	loop {
		let Some((mode, document, is_rest)) = stack.pop() else {
			// the document fits: what follows it must fit too, up to the next new line
			if rest_index == 0 {
				return true;
			}
			rest_index -= 1;
			let (_, mode, document) = rest[rest_index];
			stack.push((mode, document, true));
			continue;
		};
		match document {
			Document::Text(text) => {
				match text.find('\n') {
					Some(position) => {
						remaining -= text[..position].chars().count() as isize;
						return remaining >= 0;
					}
					None => remaining -= text.chars().count() as isize,
				}
				if remaining < 0 {
					return false;
				}
			}
			Document::TrailingComment(text) => {
				if text.contains('\n') {
					return remaining >= 0;
				}
			}
			Document::Concat(parts) => {
				for part in parts.iter().rev() {
					stack.push((mode, part, is_rest));
				}
			}
			Document::Indent(contents) | Document::IndentIfBreak(contents) => {
				stack.push((mode, contents, is_rest))
			}
			Document::Group {
				contents, breaks, ..
			} => {
				let mode = match (*breaks, is_rest) {
					(true, _) => Mode::Break,
					(false, true) => mode,
					(false, false) => Mode::Flat,
				};
				stack.push((mode, contents, is_rest));
			}
			Document::Choice { standard, .. } => {
				let mode = match (standard.breaks(), is_rest) {
					(true, _) => Mode::Break,
					(false, true) => mode,
					(false, false) => Mode::Flat,
				};
				stack.push((mode, standard, is_rest));
			}
			Document::IfBreak { broken, flat } => {
				let contents = if mode == Mode::Break { broken } else { flat };
				stack.push((mode, contents, is_rest));
			}
			Document::Line | Document::SoftLine => {
				if mode == Mode::Break {
					return true;
				}
				if matches!(document, Document::Line) {
					remaining -= 1;
					if remaining < 0 {
						return false;
					}
				}
			}
			Document::HardLine => return true,
			Document::BreakParent => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn items<'a>(items: &[&'a str]) -> Document<'a> {
		let mut parts = Parts::new();
		parts.text("[");
		let mut inner = Parts::new();
		for (index, item) in items.iter().enumerate() {
			if index > 0 {
				inner.push(Document::if_break(Document::empty(), Document::text(",")));
				inner.push(Line);
			} else {
				inner.push(SoftLine);
			}
			inner.text(*item);
		}
		parts.push(Document::indent(inner.build()));
		parts.push(SoftLine);
		parts.text("]");
		Document::group(parts.build())
	}

	#[test]
	fn prints_a_group_flat_when_it_fits() {
		assert_eq!(print(&items(&["a", "b", "c"]), 20), "[a, b, c]");
	}

	#[test]
	fn breaks_a_group_that_does_not_fit() {
		assert_eq!(
			print(&items(&["first", "second", "third"]), 10),
			"[\n\tfirst\n\tsecond\n\tthird\n]"
		);
	}

	#[test]
	fn a_hard_line_breaks_the_groups_around_it() {
		let document = Document::group(concat!["a", Line, "b", HardLine, "c"]);
		assert_eq!(print(&document, 80), "a\nb\nc");
	}

	#[test]
	fn the_rest_of_the_line_counts() {
		let document = concat![items(&["a", "b"]), "-------"];
		assert_eq!(
			print(&document, 10),
			"[a, b]-------".replace("[a, b]", "[\n\ta\n\tb\n]")
		);
		assert_eq!(print(&document, 20), "[a, b]-------");
	}

	#[test]
	fn lines_never_end_with_spaces() {
		let document = concat![
			"a ",
			HardLine,
			HardLine,
			Document::indent(concat![HardLine, "b"])
		];
		assert_eq!(print(&document, 80), "a\n\n\n\tb");
	}
}
