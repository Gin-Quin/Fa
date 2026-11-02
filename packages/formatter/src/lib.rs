//! The code formatter of Fa (`fa format`): parses a source and prints its syntax tree back in
//! the canonical layout, with every comment of the source.
//!
//! - Indentation with tabs, lines of at most `Options::width` columns (a tab counts for
//!   `document::TAB_WIDTH`) when the code can be broken.
//! - A list, a call, an object or a parameter list is printed on one line if it fits, otherwise
//!   with one item per line, without commas. Objects, lists, blocks of a single statement and
//!   chains of calls written over several lines stay over several lines.
//! - At most one blank line between two statements; none at the start or end of a block.
//! - The comments that end consecutive lines are aligned in a column.
//! - Literals are kept as written (digits, escapes); strings are always between double quotes.
//! - The canonical rewrites asked by the documentation (`Options::rewrites`), like
//!   `Percentage(x)` into `(x)%`. See `design/decisions.md`.
//!
//! The formatter refuses code with syntax errors, and never drops a comment. Its memory is linear
//! in the size of the code: a module is printed statement after statement, and the documents
//! used in two layouts are shared (`Document::Shared`), never copied.

mod comments;
pub mod document;
mod printer;

use std::borrow::Cow;

use fa_parser::{Diagnostic, SourceSpan, TypedSyntaxTree, parse};

use crate::printer::Printer;

/// The maximum width of a line, like the TypeScript code of the repository.
pub const DEFAULT_WIDTH: usize = 100;

#[derive(Debug, Clone)]
pub struct Options {
	/// The maximum width of a line, in columns.
	pub width: usize,
	/// Apply the canonical rewrites of the documentation, which change the syntax tree but not
	/// the meaning: `Percentage(x)` is `(x)%`, `score is in 0..60` is `score is 0..60`,
	/// `-2.4.abs()` is `(-2.4).abs()`, `{ id = id }` is `{ id }`, `let p: Point = Point { }`
	/// is `let p: Point = { }`, `({ a }: { a: T })` is `({ a: T })`.
	pub rewrites: bool,
}

impl Default for Options {
	fn default() -> Self {
		Options {
			width: DEFAULT_WIDTH,
			rewrites: true,
		}
	}
}

/// Format a source with the default options. Returns the errors of the source if it can't be
/// parsed.
pub fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
	format_with(source, &Options::default())
}

/// Format a source. Returns the errors of the source if it can't be parsed.
pub fn format_with(source: &str, options: &Options) -> Result<String, Vec<Diagnostic>> {
	// a line break is always `\n`, even in a string (values/strings.md)
	let source: Cow<str> = if source.contains('\r') {
		Cow::Owned(source.replace("\r\n", "\n"))
	} else {
		Cow::Borrowed(source)
	};
	let tree = parse(&source);
	if tree.has_errors() {
		return Err(tree
			.diagnostics
			.into_iter()
			.filter(Diagnostic::is_error)
			.collect());
	}
	format_tree(&tree, options)
}

/// True if the source is already formatted (and has no syntax error).
pub fn is_formatted(source: &str) -> bool {
	format(source).is_ok_and(|formatted| formatted == source)
}

/// The stack of the thread that formats deeply nested code: the printer is recursive, and the
/// code nested as deep as the parser allows needs a few megabytes in a debug build, more than
/// the 2 MB of a thread (the language server's). Only the pages used are allocated.
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// The nesting of the syntax tree above which the formatter runs on a thread of `STACK_SIZE`
/// (starting a thread with a large stack costs more than formatting a small file).
const DEEP_NESTING: usize = 32;

/// Format a parsed source, which must have no error.
pub fn format_tree(tree: &TypedSyntaxTree, options: &Options) -> Result<String, Vec<Diagnostic>> {
	if nesting(tree) <= DEEP_NESTING {
		return print_tree(tree, options);
	}
	std::thread::scope(|scope| {
		match std::thread::Builder::new()
			.name("fa-formatter".to_string())
			.stack_size(STACK_SIZE)
			.spawn_scoped(scope, || print_tree(tree, options))
		{
			Ok(thread) => thread
				.join()
				.unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
			// no thread available: format on this one
			Err(_) => print_tree(tree, options),
		}
	})
}

/// The depth of the syntax tree (its deepest node), without recursion.
fn nesting(tree: &TypedSyntaxTree) -> usize {
	let mut deepest = 0;
	let mut stack = vec![(tree.root, 1)];
	while let Some((index, depth)) = stack.pop() {
		deepest = deepest.max(depth);
		tree.at(index)
			.for_each_child(|child| stack.push((child, depth + 1)));
	}
	deepest
}

fn print_tree(tree: &TypedSyntaxTree, options: &Options) -> Result<String, Vec<Diagnostic>> {
	let (formatted, unprinted_comments) = Printer::new(tree, options).module();
	if unprinted_comments > 0 {
		return Err(vec![internal_error(
			"the formatter could not place every comment",
		)]);
	}
	Ok(formatted)
}

fn internal_error(message: &str) -> Diagnostic {
	Diagnostic::error(
		format!("Internal error of the formatter: {message}. Please report it."),
		SourceSpan::new(0, 0),
	)
}
