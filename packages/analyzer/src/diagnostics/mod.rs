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

/// A file's text, prepared for rendering its diagnostics on the command line.
pub struct SourceView<'t> {
	/// The path printed before the position.
	pub path: &'t str,
	/// The text of the file.
	pub text: &'t str,
	lines: LineIndex,
}

impl<'t> SourceView<'t> {
	/// Prepares a file's text.
	pub fn new(path: &'t str, text: &'t str) -> Self {
		Self {
			path,
			text,
			lines: LineIndex::new(text),
		}
	}

	/// The 1-based line and column of an offset, the column counted in
	/// characters (Unicode scalar values), as editors show it.
	pub fn position(&self, offset: u32) -> (u32, u32) {
		let offset = (offset as usize).min(self.text.len()) as u32;
		let (line, _) = self.lines.line_column(offset);
		let start = self.lines.starts[line as usize - 1] as usize;
		let end = floor_char_boundary(self.text, offset as usize);
		(line, self.text[start..end].chars().count() as u32 + 1)
	}

	/// The text of a 1-based line, without its line break.
	pub fn line_text(&self, line: u32) -> &'t str {
		let start = self.lines.starts[line as usize - 1] as usize;
		let end = self
			.lines
			.starts
			.get(line as usize)
			.map_or(self.text.len(), |&next| next as usize);
		self.text[start..end].trim_end_matches(['\n', '\r'])
	}
}

/// The largest char boundary of `text` at or before `index`.
fn floor_char_boundary(text: &str, mut index: usize) -> usize {
	while index > 0 && !text.is_char_boundary(index) {
		index -= 1;
	}
	index
}

/// Renders a diagnostic for the command line (`fa check`):
///
/// ```text
/// main.fa:2:15: error[E-NUM-10]: 200 doesn't fit in an `Int8`
///   2 | let e: Int8 = 100 + 100
///     |               ^^^^^^^^^
///   note: declared here (main.fa:1:5)
/// ```
///
/// The caret line keeps the tabs of the source line, so it stays aligned;
/// a span over several lines is underlined to the end of its first line.
pub fn render(diagnostic: &Diagnostic, source: &SourceView) -> String {
	let mut out = String::new();
	let span = diagnostic.location.span;
	let (line, column) = source.position(span.start);
	let _ = writeln!(
		out,
		"{}:{line}:{column}: {}[{}]: {}",
		source.path,
		diagnostic.severity.name(),
		diagnostic.code.id,
		diagnostic.message
	);
	let text = source.line_text(line);
	let number = line.to_string();
	let gutter = " ".repeat(number.len());
	let _ = writeln!(out, "  {number} | {text}");
	let line_start = source.lines.starts[line as usize - 1] as usize;
	let start = floor_char_boundary(source.text, span.start as usize).max(line_start);
	let end = floor_char_boundary(
		source.text,
		(span.end as usize).min(line_start + text.len()),
	);
	let before: String = source.text[line_start..start.min(line_start + text.len())]
		.chars()
		.map(|c| if c == '\t' { '\t' } else { ' ' })
		.collect();
	let width = source.text[start.min(end)..end].chars().count().max(1);
	let _ = writeln!(out, "  {gutter} | {before}{}", "^".repeat(width));
	for (location, note) in &diagnostic.related {
		if location.file == diagnostic.location.file {
			let (line, column) = source.position(location.span.start);
			let _ = writeln!(out, "  note: {note} ({}:{line}:{column})", source.path);
		} else {
			let _ = writeln!(out, "  note: {note}");
		}
	}
	out
}

/// Renders the diagnostics of a file, then a summary line
/// (`2 errors, 1 warning`); hints are left out. Empty when there is nothing
/// to report.
pub fn render_all(diagnostics: &[Diagnostic], source: &SourceView) -> String {
	let mut out = String::new();
	let (mut errors, mut warnings) = (0, 0);
	for diagnostic in diagnostics {
		match diagnostic.severity {
			Severity::Error => errors += 1,
			Severity::Warning => warnings += 1,
			Severity::Hint => continue,
		}
		out.push_str(&render(diagnostic, source));
		out.push('\n');
	}
	if errors + warnings > 0 {
		let plural = |count: usize, word: &str| {
			format!("{count} {word}{}", if count == 1 { "" } else { "s" })
		};
		let summary = match (errors, warnings) {
			(0, _) => plural(warnings, "warning"),
			(_, 0) => plural(errors, "error"),
			_ => format!(
				"{}, {}",
				plural(errors, "error"),
				plural(warnings, "warning")
			),
		};
		let _ = writeln!(out, "{summary}");
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
	fn rendering_names_file_position_code_line_and_caret() {
		let text = "let x = 1\nlet e: Int8 = 100 + 100\n";
		let diagnostic = Diagnostic::new(
			&codes::values::E_NUM_10,
			Location {
				file: FileId(0),
				span: SourceSpan::new(24, 33),
			},
			"200 doesn't fit in an `Int8`",
		)
		.with_related(location(4), "declared here");
		let rendered = render(&diagnostic, &SourceView::new("main.fa", text));
		assert_eq!(
			rendered,
			concat!(
				"main.fa:2:15: error[E-NUM-10]: 200 doesn't fit in an `Int8`\n",
				"  2 | let e: Int8 = 100 + 100\n",
				"    |               ^^^^^^^^^\n",
				"  note: declared here (main.fa:1:5)\n"
			)
		);
	}

	#[test]
	fn rendering_keeps_tabs_and_counts_characters() {
		let text = "do {\n\tlet é = \"x\"\n}";
		let start = text.find('é').unwrap() as u32;
		let diagnostic = Diagnostic::new(
			&codes::values::E_LEX_2,
			Location {
				file: FileId(0),
				span: SourceSpan::new(start, start + 2),
			},
			"ASCII only",
		);
		let source = SourceView::new("a.fa", text);
		assert_eq!(source.position(start + 2), (2, 7));
		let rendered = render(&diagnostic, &source);
		assert_eq!(
			rendered,
			"a.fa:2:6: error[E-LEX-2]: ASCII only\n  2 | \tlet é = \"x\"\n    | \t    ^\n"
		);
		// an empty span still gets a caret, a span over lines stops at the line end
		let empty = Diagnostic::new(&codes::values::E_LEX_6, location(0), "x");
		let empty = Diagnostic {
			location: Location {
				file: FileId(0),
				span: SourceSpan::new(3, 3),
			},
			..empty
		};
		assert!(render(&empty, &source).ends_with("  1 | do {\n    |    ^\n"));
		let long = Diagnostic {
			location: Location {
				file: FileId(0),
				span: SourceSpan::new(3, text.len() as u32),
			},
			..empty
		};
		assert!(render(&long, &source).ends_with("    |    ^\n"));
	}

	#[test]
	fn rendering_all_ends_with_a_summary() {
		let text = "let a = 1\n";
		let source = SourceView::new("main.fa", text);
		assert_eq!(render_all(&[], &source), "");
		let error = Diagnostic::new(&codes::values::E_NUM_10, location(0), "e");
		let warning = Diagnostic::new(&codes::values::W_BOOL_1, location(4), "w");
		let rendered = render_all(&[error.clone(), warning, error], &source);
		assert!(rendered.ends_with("\n2 errors, 1 warning\n"), "{rendered}");
		assert_eq!(rendered.matches("main.fa:1:").count(), 3);
	}

	#[test]
	fn codes_compare_by_identity() {
		assert_eq!(&codes::values::E_NUM_10, &codes::values::E_NUM_10);
		assert_ne!(&codes::values::E_NUM_10, &codes::values::E_NUM_11);
		assert!(codes::values::E_ERR_3.is_named("EF015"));
	}
}
