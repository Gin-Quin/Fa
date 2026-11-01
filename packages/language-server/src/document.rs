//! The text of an open document, kept in sync with the editor.

use fa_parser::SourceSpan;
use tower_lsp::lsp_types::{Position, Range, TextDocumentContentChangeEvent};

use crate::line_index::{LineIndex, PositionEncoding};

/// An open document: its text, the version the editor gave it, and the start of its lines.
#[derive(Debug, Clone)]
pub struct Document {
	pub text: String,
	pub version: i32,
	pub line_index: LineIndex,
}

impl Document {
	pub fn new(text: String, version: i32) -> Self {
		let line_index = LineIndex::new(&text);
		Document {
			text,
			version,
			line_index,
		}
	}

	/// Apply the changes of a `textDocument/didChange` notification, in order: a change with a
	/// range replaces that range (of the text as modified by the previous changes), a change
	/// without range replaces the whole text.
	pub fn apply_changes(
		&mut self,
		changes: impl IntoIterator<Item = TextDocumentContentChangeEvent>,
		version: i32,
		encoding: PositionEncoding,
	) {
		for change in changes {
			match change.range {
				Some(range) => {
					let span = self.span(range, encoding);
					self.text.replace_range(span.start..span.end, &change.text);
				}
				None => self.text = change.text,
			}
			self.line_index = LineIndex::new(&self.text);
		}
		self.version = version;
	}

	pub fn position(&self, offset: usize, encoding: PositionEncoding) -> Position {
		self.line_index.position(&self.text, offset, encoding)
	}

	pub fn offset(&self, position: Position, encoding: PositionEncoding) -> usize {
		self.line_index.offset(&self.text, position, encoding)
	}

	pub fn range(&self, span: SourceSpan, encoding: PositionEncoding) -> Range {
		self.line_index.range(&self.text, span, encoding)
	}

	pub fn span(&self, range: Range, encoding: PositionEncoding) -> SourceSpan {
		self.line_index.span(&self.text, range, encoding)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use PositionEncoding::{Utf8, Utf16};

	fn change(start: (u32, u32), end: (u32, u32), text: &str) -> TextDocumentContentChangeEvent {
		TextDocumentContentChangeEvent {
			range: Some(Range::new(
				Position::new(start.0, start.1),
				Position::new(end.0, end.1),
			)),
			range_length: None,
			text: text.to_string(),
		}
	}

	fn full(text: &str) -> TextDocumentContentChangeEvent {
		TextDocumentContentChangeEvent {
			range: None,
			range_length: None,
			text: text.to_string(),
		}
	}

	#[test]
	fn insertion() {
		let mut document = Document::new("let a = 1".to_string(), 1);
		document.apply_changes([change((0, 5), (0, 5), "bc")], 2, Utf16);
		assert_eq!(document.text, "let abc = 1");
		assert_eq!(document.version, 2);
	}

	#[test]
	fn deletion() {
		let mut document = Document::new("let abc = 1".to_string(), 1);
		document.apply_changes([change((0, 4), (0, 6), "")], 2, Utf16);
		assert_eq!(document.text, "let c = 1");
	}

	#[test]
	fn replacement_across_lines() {
		let mut document = Document::new("let a = 1\nlet b = 2\nlet c = 3".to_string(), 1);
		document.apply_changes([change((0, 8), (2, 4), "4\nlet ")], 2, Utf16);
		assert_eq!(document.text, "let a = 4\nlet c = 3");
		assert_eq!(document.line_index.line_count(), 2);
	}

	#[test]
	fn changes_apply_in_order() {
		let mut document = Document::new("abc".to_string(), 1);
		document.apply_changes(
			[
				change((0, 3), (0, 3), "\nd"),
				change((1, 1), (1, 1), "e"),
				change((0, 0), (0, 1), "A"),
			],
			2,
			Utf16,
		);
		assert_eq!(document.text, "Abc\nde");
	}

	#[test]
	fn full_replacement() {
		let mut document = Document::new("abc".to_string(), 1);
		document.apply_changes([full("x\ny")], 5, Utf16);
		assert_eq!(document.text, "x\ny");
		assert_eq!(document.line_index.line_count(), 2);
		assert_eq!(document.version, 5);
	}

	#[test]
	fn edit_after_an_accented_letter() {
		let mut document = Document::new("let café = 1".to_string(), 1);
		// UTF-16 column 8 is after `café`
		document.apply_changes([change((0, 8), (0, 8), "s")], 2, Utf16);
		assert_eq!(document.text, "let cafés = 1");
	}

	#[test]
	fn edit_after_an_emoji() {
		let mut document = Document::new("\"😀\" + x".to_string(), 1);
		// UTF-16: `"` is 1 unit, the emoji 2 units, `"` 1 unit
		document.apply_changes([change((0, 3), (0, 4), "!\"")], 2, Utf16);
		assert_eq!(document.text, "\"😀!\" + x");
	}

	#[test]
	fn replace_an_emoji() {
		let mut document = Document::new("a😀b".to_string(), 1);
		document.apply_changes([change((0, 1), (0, 3), "é")], 2, Utf16);
		assert_eq!(document.text, "aéb");
	}

	#[test]
	fn edit_cjk_text_in_utf8() {
		let mut document = Document::new("名前 = 1".to_string(), 1);
		// UTF-8 columns: `名` is bytes 0..3, `前` bytes 3..6
		document.apply_changes([change((0, 3), (0, 6), "字")], 2, Utf8);
		assert_eq!(document.text, "名字 = 1");
	}

	#[test]
	fn insertion_at_the_end_of_the_text() {
		let mut document = Document::new("a\n".to_string(), 1);
		document.apply_changes([change((1, 0), (1, 0), "b")], 2, Utf16);
		assert_eq!(document.text, "a\nb");
	}

	#[test]
	fn range_past_the_end_is_clamped() {
		let mut document = Document::new("abc".to_string(), 1);
		document.apply_changes([change((0, 2), (9, 9), "")], 2, Utf16);
		assert_eq!(document.text, "ab");
	}

	#[test]
	fn edit_on_a_crlf_document() {
		let mut document = Document::new("a\r\nb\r\nc".to_string(), 1);
		document.apply_changes([change((1, 0), (1, 1), "B")], 2, Utf16);
		assert_eq!(document.text, "a\r\nB\r\nc");
		document.apply_changes([change((0, 1), (1, 0), "")], 3, Utf16);
		assert_eq!(document.text, "aB\r\nc");
	}

	#[test]
	fn positions_follow_the_edits() {
		let mut document = Document::new("x".to_string(), 1);
		document.apply_changes([change((0, 0), (0, 0), "😀\n")], 2, Utf16);
		assert_eq!(
			document.position(document.text.len(), Utf16),
			Position::new(1, 1)
		);
		assert_eq!(document.offset(Position::new(0, 2), Utf16), 4);
	}
}
