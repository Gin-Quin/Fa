//! Diagnostics: plain data, converted to LSP types by the language server.

#[macro_use]
pub mod codes;

use std::fmt::Write;

use crate::ids::FileId;

/// How serious a diagnostic is.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub enum Severity {
	/// The program is rejected.
	Error,
	/// The program is accepted, but probably not what was meant.
	Warning,
	/// A suggestion; never fails a build or a test case.
	Hint,
}

impl Severity {
	/// The lowercase name, as written in test annotations.
	pub fn name(self) -> &'static str {
		match self {
			Severity::Error => "error",
			Severity::Warning => "warning",
			Severity::Hint => "hint",
		}
	}
}

/// A registered diagnostic code (see [`codes`]).
#[derive(Debug)]
pub struct CodeInfo {
	/// The canonical code, for example `E-NUM-10`.
	pub id: &'static str,
	/// The codes under which other briefs name the same condition.
	pub aliases: &'static [&'static str],
	/// The severity of every diagnostic with this code.
	pub severity: Severity,
	/// The condition, in a few words.
	pub summary: &'static str,
}

/// A diagnostic code: a reference into the registry, compared by identity.
pub type Code = &'static CodeInfo;

impl PartialEq for CodeInfo {
	fn eq(&self, other: &Self) -> bool {
		std::ptr::eq(self, other)
	}
}

impl Eq for CodeInfo {}

impl CodeInfo {
	/// Whether `name` is this code or one of its aliases.
	pub fn is_named(&self, name: &str) -> bool {
		self.id == name || self.aliases.contains(&name)
	}
}

/// A byte range of a source text.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Default)]
pub struct SourceSpan {
	/// Offset of the first byte.
	pub start: u32,
	/// Offset after the last byte.
	pub end: u32,
}

impl SourceSpan {
	/// The span `start..end`.
	pub fn new(start: u32, end: u32) -> Self {
		Self { start, end }
	}

	/// Whether `offset` is inside the span (the end included, for a cursor after a name).
	pub fn contains(self, offset: u32) -> bool {
		self.start <= offset && offset <= self.end
	}
}

/// A span in a file.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct Location {
	/// The file.
	pub file: FileId,
	/// The span in the file's text.
	pub span: SourceSpan,
}

/// A diagnostic: a code, a message written to be acted upon, and related spans.
#[derive(Clone, Debug)]
pub struct Diagnostic {
	/// Copied from the code, so that sorting and filtering don't dereference it.
	pub severity: Severity,
	/// The registered code.
	pub code: Code,
	/// The message: names the construct, the types involved and the fix.
	pub message: String,
	/// The primary span.
	pub location: Location,
	/// Secondary spans with their notes (the declaration, the invalidating call…).
	pub related: Vec<(Location, String)>,
}

impl Diagnostic {
	/// A diagnostic with the severity of its code.
	pub fn new(code: Code, location: Location, message: impl Into<String>) -> Self {
		Self {
			severity: code.severity,
			code,
			message: message.into(),
			location,
			related: Vec::new(),
		}
	}

	/// Adds a related span.
	pub fn with_related(mut self, location: Location, note: impl Into<String>) -> Self {
		self.related.push((location, note.into()));
		self
	}

	/// The order of the output: file, then span, then code.
	pub fn sort_key(&self) -> (Location, &'static str) {
		(self.location, self.code.id)
	}
}

/// Sorts diagnostics by location then code, keeping the order of equal ones.
pub fn sort(diagnostics: &mut [Diagnostic]) {
	diagnostics.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
}

/// Line starts of a text, to turn offsets into 1-based lines and columns.
#[derive(Clone, Debug)]
pub struct LineIndex {
	starts: Vec<u32>,
}

impl LineIndex {
	/// The line starts of `text`.
	pub fn new(text: &str) -> Self {
		let mut starts = vec![0];
		starts.extend(
			text.bytes()
				.enumerate()
				.filter(|&(_, byte)| byte == b'\n')
				.map(|(index, _)| index as u32 + 1),
		);
		Self { starts }
	}

	/// The 1-based line and column (in bytes) of an offset.
	pub fn line_column(&self, offset: u32) -> (u32, u32) {
		let line = self.starts.partition_point(|&start| start <= offset) - 1;
		(line as u32 + 1, offset - self.starts[line] + 1)
	}

	/// The 1-based line of an offset.
	pub fn line(&self, offset: u32) -> u32 {
		self.line_column(offset).0
	}
}

/// Renders a diagnostic for the command line:
/// `error[E-NUM-10]: message` then `  --> path:line:column` and the related notes.
pub fn render(diagnostic: &Diagnostic, path: &str, lines: &LineIndex) -> String {
	let mut out = String::new();
	let (line, column) = lines.line_column(diagnostic.location.span.start);
	let _ = writeln!(
		out,
		"{}[{}]: {}",
		diagnostic.severity.name(),
		diagnostic.code.id,
		diagnostic.message
	);
	let _ = writeln!(out, "  --> {path}:{line}:{column}");
	for (location, note) in &diagnostic.related {
		if location.file == diagnostic.location.file {
			let (line, column) = lines.line_column(location.span.start);
			let _ = writeln!(out, "  note: {note} ({path}:{line}:{column})");
		} else {
			let _ = writeln!(out, "  note: {note}");
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn location(start: u32) -> Location {
		Location {
			file: FileId(0),
			span: SourceSpan::new(start, start + 1),
		}
	}

	#[test]
	fn line_index_finds_lines_and_columns() {
		let lines = LineIndex::new("let a = 1\nlet b = 2\n\nx");
		assert_eq!(lines.line_column(0), (1, 1));
		assert_eq!(lines.line_column(4), (1, 5));
		assert_eq!(lines.line_column(10), (2, 1));
		assert_eq!(lines.line_column(20), (3, 1));
		assert_eq!(lines.line_column(21), (4, 1));
	}

	#[test]
	fn diagnostics_sort_by_location_then_code() {
		let mut diagnostics = vec![
			Diagnostic::new(&codes::values::E_NUM_10, location(8), "b"),
			Diagnostic::new(&codes::values::E_DECL_1, location(8), "a"),
			Diagnostic::new(&codes::values::E_NUM_10, location(2), "c"),
		];
		sort(&mut diagnostics);
		let order: Vec<_> = diagnostics.iter().map(|d| d.message.as_str()).collect();
		assert_eq!(order, ["c", "a", "b"]);
	}

	#[test]
	fn rendering_names_code_and_position() {
		let text = "let x = 1\nlet e: Int8 = 100 + 100";
		let diagnostic =
			Diagnostic::new(&codes::values::E_NUM_10, location(24), "certain overflow")
				.with_related(location(0), "declared here");
		let rendered = render(&diagnostic, "main.fa", &LineIndex::new(text));
		assert_eq!(
			rendered,
			"error[E-NUM-10]: certain overflow\n  --> main.fa:2:15\n  note: declared here (main.fa:1:1)\n"
		);
	}

	#[test]
	fn codes_compare_by_identity() {
		assert_eq!(&codes::values::E_NUM_10, &codes::values::E_NUM_10);
		assert_ne!(&codes::values::E_NUM_10, &codes::values::E_NUM_11);
		assert!(codes::values::E_ERR_3.is_named("EF015"));
	}
}
