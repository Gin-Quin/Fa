//! The layout engine: a document of text, possible line breaks and groups, printed in the
//! narrowest way that fits the line width (the algorithm of Wadler's "prettier printer", as in
//! Prettier).
//!
//! A `Group` is printed on one line ("flat") if all of it fits in the rest of the line, otherwise
//! its `Line`s become new lines ("broken"), while its inner groups decide again for themselves.
//! A `HardLine` always breaks, and so does every group around it.

use std::borrow::Cow;
use std::rc::Rc;

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
		// `breaks`, or a group broken whatever the width inside (`has_broken_group`, computed
		// once so that measuring a group never walks its whole contents again)
		broken_inside: bool,
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
	/// A document used at several places, like the arguments of a call, in both layouts of a
	/// `Choice`: sharing it (instead of copying it) keeps the size of the document linear in the
	/// size of the code, whatever the nesting of the calls.
	Shared(Rc<Document<'a>>),
	/// The document with its last group broken (the last part of a concatenation, through the
	/// shared documents): the hugged argument of a call.
	BreakLast(Box<Document<'a>>),
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
		let broken_inside = breaks || hard || contents.has_broken_group();
		Document::Group {
			contents: Box::new(contents),
			breaks: breaks || hard,
			hard,
			broken_inside,
		}
	}

	pub fn indent(contents: Document<'a>) -> Self {
		Document::Indent(Box::new(contents))
	}

	pub fn indent_if_break(contents: Document<'a>) -> Self {
		Document::IndentIfBreak(Box::new(contents))
	}

	pub fn shared(contents: Document<'a>) -> Self {
		Document::Shared(Rc::new(contents))
	}

	pub fn break_last(contents: Document<'a>) -> Self {
		Document::BreakLast(Box::new(contents))
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
			Document::Shared(contents) => contents.breaks(),
			Document::BreakLast(contents) => contents.breaks(),
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
			Document::Shared(contents) => contents.is_broken(),
			Document::BreakLast(contents) => contents.is_broken() || contents.ends_with_group(),
			document => document.breaks(),
		}
	}

	/// True if the last part of the document (through concatenations and shared documents) is a
	/// group: the group that `BreakLast` breaks.
	fn ends_with_group(&self) -> bool {
		match self {
			Document::Group { .. } => true,
			Document::Concat(parts) => parts.last().is_some_and(Document::ends_with_group),
			Document::Shared(contents) => contents.ends_with_group(),
			_ => false,
		}
	}

	/// True if the document contains a group broken whatever the width (at any depth), so it
	/// can't be printed flat: a call whose argument is a list written over several lines.
	pub fn has_broken_group(&self) -> bool {
		match self {
			Document::Group { broken_inside, .. } => *broken_inside,
			Document::Indent(contents) | Document::IndentIfBreak(contents) => {
				contents.has_broken_group()
			}
			Document::Concat(parts) => parts.iter().any(Document::has_broken_group),
			Document::Choice { standard, .. } => standard.has_broken_group(),
			Document::Shared(contents) => contents.has_broken_group(),
			Document::BreakLast(contents) => {
				contents.has_broken_group() || contents.ends_with_group()
			}
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

	pub fn build(mut self) -> Document<'a> {
		match self.parts.len() {
			1 => self.parts.pop().expect("one part"),
			_ => {
				// the document of a large file holds many of these lists
				self.parts.shrink_to_fit();
				Document::Concat(self.parts)
			}
		}
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

/// A document to print: its indentation, its mode, and whether its last group must break
/// (`BreakLast`).
#[derive(Clone, Copy)]
struct Command<'d, 'a> {
	indentation: usize,
	mode: Mode,
	document: &'d Document<'a>,
	break_last: bool,
}

impl<'d, 'a> Command<'d, 'a> {
	fn with(self, document: &'d Document<'a>, break_last: bool) -> Self {
		Command {
			document,
			break_last,
			..self
		}
	}
}

/// Print a document, breaking its groups to keep the lines within `width` columns (tabs count
/// for `TAB_WIDTH`). Lines never end with spaces or tabs.
///
/// Linear in the size of the document: a group is measured once, and `fits` stops at the end
/// of the line or as soon as the line is too long; only the chosen layout of a `Choice` is
/// printed.
pub fn print(document: &Document, width: usize) -> String {
	let mut output = Output::new(width);
	output.print(document);
	output.finish()
}

/// A text printed document after document, to print a file statement after statement: the
/// documents of a large file are never all in memory. Printing documents one after the other
/// is the same as printing their concatenation if each one after the first starts with a hard
/// line (what follows a hard line doesn't change how the line before it is printed).
pub struct Output {
	text: String,
	// the column of the end of the text
	column: usize,
	width: usize,
	// where the comments at the end of a line (on one line) start and end in the text
	trailing_comments: Vec<(usize, usize)>,
}

impl Output {
	pub fn new(width: usize) -> Self {
		Output {
			text: String::new(),
			column: 0,
			width,
			trailing_comments: Vec::new(),
		}
	}

	/// Print a document after the text, at the indentation 0.
	pub fn print(&mut self, document: &Document) {
		print_into(
			document,
			self.width,
			&mut self.text,
			&mut self.column,
			&mut self.trailing_comments,
		);
	}

	/// The text, without spaces at its end, with the comments at the end of consecutive lines
	/// aligned (`align_trailing_comments`).
	pub fn finish(mut self) -> String {
		let trimmed = self.text.trim_end_matches([' ', '\t']).len();
		self.text.truncate(trimmed);
		align_trailing_comments(self.text, &self.trailing_comments)
	}
}

/// The indentation at the start of a line.
fn indentation(line: &str) -> &str {
	&line[..line.len() - line.trim_start_matches(['\t', ' ']).len()]
}

/// Align the comments that end consecutive lines in a column (like `gofmt`): over a run of
/// consecutive lines of the same indentation that all end with a comment written after code,
/// every comment starts one space after the longest code of the run. A blank line, a line
/// without such a comment (a comment on its own line too) or a change of indentation ends the
/// run, so the alignment never depends on the width of a tab. The comments don't count in the
/// width, so the layout doesn't change, and formatting again gives the same text.
///
/// `comments` are the starts and ends of the comments printed at the end of a line, in order.
fn align_trailing_comments(text: String, comments: &[(usize, usize)]) -> String {
	// per line: its start, its end, and where its trailing comments start (if it has some)
	let mut lines: Vec<(usize, usize, Option<usize>)> = Vec::new();
	// whether two lines are in the same run
	let continues = |previous: &(usize, usize, Option<usize>),
	                 line: &(usize, usize, Option<usize>)| {
		previous.2.is_some()
			&& line.2.is_some()
			&& indentation(&text[previous.0..previous.1]) == indentation(&text[line.0..line.1])
	};
	let mut next_comment = 0;
	let mut line_start = 0;
	let mut has_run = false;
	for line in text.split('\n') {
		let line_end = line_start + line.len();
		// the comments of the line, and the last one, which must end the line
		let first = next_comment;
		while next_comment < comments.len() && comments[next_comment].0 < line_end {
			next_comment += 1;
		}
		let mut start = None;
		if next_comment > first && comments[next_comment - 1].1 == line_end {
			// the comments that follow one another at the end of the line
			let mut index = next_comment - 1;
			while index > first && comments[index - 1].1 + 1 == comments[index].0 {
				index -= 1;
			}
			let comment_start = comments[index].0;
			if !text[line_start..comment_start].trim().is_empty() {
				start = Some(comment_start);
			}
		}
		let line = (line_start, line_end, start);
		has_run |= lines
			.last()
			.is_some_and(|previous| continues(previous, &line));
		lines.push(line);
		line_start = line_end + 1;
	}
	if !has_run {
		return text;
	}

	let mut aligned = String::with_capacity(text.len() + text.len() / 8);
	let mut index = 0;
	while index < lines.len() {
		let (start, end, comment) = lines[index];
		if comment.is_none() {
			aligned.push_str(&text[start..end]);
			aligned.push('\n');
			index += 1;
			continue;
		}
		// the run of lines ending with a comment, and the column of its comments
		let mut run_end = index + 1;
		while run_end < lines.len() && continues(&lines[run_end - 1], &lines[run_end]) {
			run_end += 1;
		}
		let code = |&(start, _, comment): &(usize, usize, Option<usize>)| {
			text[start..comment.unwrap_or(start)].trim_end()
		};
		let column = lines[index..run_end]
			.iter()
			.map(|line| code(line).chars().count())
			.max()
			.unwrap_or(0)
			+ 1;
		for line in &lines[index..run_end] {
			let code = code(line);
			aligned.push_str(code);
			for _ in code.chars().count()..column {
				aligned.push(' ');
			}
			aligned.push_str(&text[line.2.unwrap_or(line.0)..line.1]);
			aligned.push('\n');
		}
		index = run_end;
	}
	// the text doesn't end with a new line
	aligned.pop();
	aligned
}

fn print_into(
	document: &Document,
	width: usize,
	output: &mut String,
	column: &mut usize,
	trailing_comments: &mut Vec<(usize, usize)>,
) {
	let mut stack = vec![Command {
		indentation: 0,
		mode: Mode::Break,
		document,
		break_last: false,
	}];

	while let Some(command) = stack.pop() {
		let Command {
			indentation,
			mode,
			document,
			break_last,
		} = command;
		match document {
			Document::Text(text) | Document::TrailingComment(text) => {
				if matches!(document, Document::TrailingComment(_)) && !text.contains('\n') {
					trailing_comments.push((output.len(), output.len() + text.len()));
				}
				output.push_str(text);
				match text.rfind('\n') {
					Some(position) => *column = text[position + 1..].chars().count(),
					None => *column += text.chars().count(),
				}
			}
			Document::Concat(parts) => {
				for (position, part) in parts.iter().enumerate().rev() {
					let last = break_last && position + 1 == parts.len();
					stack.push(command.with(part, last));
				}
			}
			Document::Shared(contents) => stack.push(command.with(contents, break_last)),
			Document::BreakLast(contents) => stack.push(command.with(contents, true)),
			Document::Indent(contents) => stack.push(Command {
				indentation: indentation + 1,
				..command.with(contents, false)
			}),
			Document::IndentIfBreak(contents) => stack.push(Command {
				indentation: indentation + usize::from(mode == Mode::Break),
				..command.with(contents, false)
			}),
			Document::Group {
				contents, breaks, ..
			} => {
				let mode = if *breaks || break_last {
					Mode::Break
				} else if mode == Mode::Flat
					|| fits(contents, Mode::Flat, &stack, width.saturating_sub(*column))
				{
					Mode::Flat
				} else {
					Mode::Break
				};
				stack.push(Command {
					mode,
					..command.with(contents, false)
				});
			}
			Document::Choice { standard, hugged } => {
				let remaining = width.saturating_sub(*column);
				if mode == Mode::Flat
					|| (!standard.has_broken_group()
						&& fits(standard, Mode::Flat, &stack, remaining))
				{
					stack.push(command.with(standard, false));
				} else if fits(hugged, Mode::Break, &stack, remaining) {
					stack.push(Command {
						mode: Mode::Break,
						..command.with(hugged, false)
					});
				} else {
					stack.push(Command {
						mode: Mode::Break,
						..command.with(standard, false)
					});
				}
			}
			Document::IfBreak { broken, flat } => {
				let contents = if mode == Mode::Break { broken } else { flat };
				stack.push(command.with(contents, false));
			}
			Document::Line | Document::SoftLine if mode == Mode::Flat => {
				if matches!(document, Document::Line) {
					output.push(' ');
					*column += 1;
				}
			}
			Document::Line | Document::SoftLine | Document::HardLine => {
				let trimmed = output.trim_end_matches([' ', '\t']).len();
				output.truncate(trimmed);
				output.push('\n');
				for _ in 0..indentation {
					output.push('\t');
				}
				*column = indentation * TAB_WIDTH;
			}
			Document::BreakParent => {}
		}
	}
}

/// The width of `text` up to its first new line, counting at most `limit + 1` characters (more
/// doesn't fit anyway), and whether it has a new line there.
fn measure(text: &str, limit: isize) -> (isize, bool) {
	let limit = usize::try_from(limit).unwrap_or(0) + 1;
	let mut width = 0;
	for character in text.chars() {
		if character == '\n' {
			return (width as isize, true);
		}
		width += 1;
		if width >= limit {
			break;
		}
	}
	(width as isize, false)
}

/// True if the first line of `document`, printed in `mode`, fits in `width` columns, followed by
/// the rest of the line (the documents of `rest` until their first new line). The groups of
/// `document` are measured flat, unless they must break; the groups of `rest` keep their mode.
///
/// Stops at the first new line or as soon as the line is too long: it reads at most one line of
/// the output (plus the documents that print nothing).
fn fits(document: &Document, mode: Mode, rest: &[Command], width: usize) -> bool {
	let mut remaining = width as isize;
	// the mode, the document, whether it is part of `rest`, and whether its last group breaks
	let mut stack: Vec<(Mode, &Document, bool, bool)> = vec![(mode, document, false, false)];
	let mut rest_index = rest.len();

	loop {
		let Some((mode, document, is_rest, break_last)) = stack.pop() else {
			// the document fits: what follows it must fit too, up to the next new line
			if rest_index == 0 {
				return true;
			}
			rest_index -= 1;
			let command = rest[rest_index];
			stack.push((command.mode, command.document, true, command.break_last));
			continue;
		};
		match document {
			Document::Text(text) => {
				let (text_width, new_line) = measure(text, remaining);
				remaining -= text_width;
				if new_line || remaining < 0 {
					return remaining >= 0;
				}
			}
			Document::TrailingComment(text) => {
				if text.contains('\n') {
					return remaining >= 0;
				}
			}
			Document::Concat(parts) => {
				for (position, part) in parts.iter().enumerate().rev() {
					let last = break_last && position + 1 == parts.len();
					stack.push((mode, part, is_rest, last));
				}
			}
			Document::Shared(contents) => stack.push((mode, contents, is_rest, break_last)),
			Document::BreakLast(contents) => stack.push((mode, contents, is_rest, true)),
			Document::Indent(contents) | Document::IndentIfBreak(contents) => {
				stack.push((mode, contents, is_rest, false))
			}
			Document::Group {
				contents, breaks, ..
			} => {
				let mode = match (*breaks || break_last, is_rest) {
					(true, _) => Mode::Break,
					(false, true) => mode,
					(false, false) => Mode::Flat,
				};
				stack.push((mode, contents, is_rest, false));
			}
			Document::Choice { standard, .. } => {
				let mode = match (standard.breaks(), is_rest) {
					(true, _) => Mode::Break,
					(false, true) => mode,
					(false, false) => Mode::Flat,
				};
				stack.push((mode, standard, is_rest, false));
			}
			Document::IfBreak { broken, flat } => {
				let contents = if mode == Mode::Break { broken } else { flat };
				stack.push((mode, contents, is_rest, false));
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
