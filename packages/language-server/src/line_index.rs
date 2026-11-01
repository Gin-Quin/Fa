//! Conversions between the byte offsets of the parser and the positions of LSP.
//!
//! The parser's spans are byte offsets into the UTF-8 source. An LSP position is a line and a
//! column counted in code units of the negotiated encoding: UTF-16 by default, UTF-8 or UTF-32
//! when the client supports them (`general.positionEncodings`, LSP 3.17).

use fa_parser::SourceSpan;
use tower_lsp::lsp_types::{Position, PositionEncodingKind, Range};

/// The unit in which the columns of LSP positions are counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionEncoding {
	/// Bytes: no conversion needed.
	Utf8,
	/// UTF-16 code units: the default of LSP, the only one every client supports.
	#[default]
	Utf16,
	/// Unicode scalar values (`char`s).
	Utf32,
}

impl PositionEncoding {
	/// Choose the encoding from the ones the client supports: UTF-8 when possible (no
	/// conversion), else UTF-16, which every client must support.
	pub fn negotiate(supported: Option<&[PositionEncodingKind]>) -> Self {
		let Some(supported) = supported else {
			return PositionEncoding::Utf16;
		};
		if supported.contains(&PositionEncodingKind::UTF8) {
			PositionEncoding::Utf8
		} else {
			PositionEncoding::Utf16
		}
	}

	pub fn kind(self) -> PositionEncodingKind {
		match self {
			PositionEncoding::Utf8 => PositionEncodingKind::UTF8,
			PositionEncoding::Utf16 => PositionEncodingKind::UTF16,
			PositionEncoding::Utf32 => PositionEncodingKind::UTF32,
		}
	}

	/// The length of a text in code units of this encoding.
	pub fn measure(self, text: &str) -> usize {
		if text.is_ascii() {
			return text.len();
		}
		match self {
			PositionEncoding::Utf8 => text.len(),
			PositionEncoding::Utf16 => text.chars().map(char::len_utf16).sum(),
			PositionEncoding::Utf32 => text.chars().count(),
		}
	}

	fn char_width(self, character: char) -> usize {
		match self {
			PositionEncoding::Utf8 => character.len_utf8(),
			PositionEncoding::Utf16 => character.len_utf16(),
			PositionEncoding::Utf32 => 1,
		}
	}
}

/// The start of every line of a text. Lines end with `\n`, `\r\n` or `\r`, like in LSP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
	// the byte offset of the first character of each line; always starts with 0
	line_starts: Vec<usize>,
	length: usize,
}

impl LineIndex {
	pub fn new(text: &str) -> Self {
		let bytes = text.as_bytes();
		let mut line_starts = vec![0];
		let mut index = 0;
		while index < bytes.len() {
			match bytes[index] {
				b'\n' => line_starts.push(index + 1),
				b'\r' => {
					if bytes.get(index + 1) == Some(&b'\n') {
						index += 1;
					}
					line_starts.push(index + 1);
				}
				_ => {}
			}
			index += 1;
		}
		LineIndex {
			line_starts,
			length: text.len(),
		}
	}

	/// The number of lines (a text without line break has one line, even if empty).
	pub fn line_count(&self) -> usize {
		self.line_starts.len()
	}

	/// The byte offset where a line starts.
	pub fn line_start(&self, line: usize) -> Option<usize> {
		self.line_starts.get(line).copied()
	}

	/// The byte offset where the content of a line ends, before its line break.
	pub fn line_end(&self, text: &str, line: usize) -> usize {
		let Some(&next_start) = self.line_starts.get(line + 1) else {
			return self.length;
		};
		let bytes = text.as_bytes();
		let mut end = next_start;
		if end > 0 && bytes.get(end - 1) == Some(&b'\n') {
			end -= 1;
		}
		if end > 0 && bytes.get(end - 1) == Some(&b'\r') {
			end -= 1;
		}
		end
	}

	/// The (0-based) line that contains a byte offset. An offset past the end is on the last
	/// line.
	pub fn line_of(&self, offset: usize) -> usize {
		self.line_starts.partition_point(|&start| start <= offset) - 1
	}

	/// The LSP position of a byte offset. An offset inside a character counts as the start of
	/// that character, an offset inside a line break as the end of the line, and an offset past
	/// the end as the end of the text.
	pub fn position(&self, text: &str, offset: usize, encoding: PositionEncoding) -> Position {
		let offset = floor_char_boundary(text, offset.min(self.length));
		let line = self.line_of(offset);
		let start = self.line_starts[line];
		let end = offset.min(self.line_end(text, line));
		let character = encoding.measure(&text[start..end]);
		Position::new(line as u32, character as u32)
	}

	/// The byte offset of an LSP position. Like the LSP specification says, a column past the
	/// end of the line is the end of the line, and a line past the end of the text is the end
	/// of the text. A column in the middle of a character (half of a UTF-16 surrogate pair)
	/// is the start of that character.
	pub fn offset(&self, text: &str, position: Position, encoding: PositionEncoding) -> usize {
		let line = position.line as usize;
		let Some(&start) = self.line_starts.get(line) else {
			return self.length;
		};
		let end = self.line_end(text, line);
		let line_text = &text[start..end];
		let target = position.character as usize;
		if encoding == PositionEncoding::Utf8 || line_text.is_ascii() {
			return floor_char_boundary(text, start + target.min(line_text.len()));
		}
		let mut units = 0;
		for (index, character) in line_text.char_indices() {
			let width = encoding.char_width(character);
			if units + width > target {
				return start + index;
			}
			units += width;
		}
		end
	}

	pub fn range(&self, text: &str, span: SourceSpan, encoding: PositionEncoding) -> Range {
		Range::new(
			self.position(text, span.start, encoding),
			self.position(text, span.end.max(span.start), encoding),
		)
	}

	/// The byte span of an LSP range (its start and end in order).
	pub fn span(&self, text: &str, range: Range, encoding: PositionEncoding) -> SourceSpan {
		let start = self.offset(text, range.start, encoding);
		let end = self.offset(text, range.end, encoding);
		SourceSpan::new(start.min(end), start.max(end))
	}
}

/// The largest character boundary at or before `offset`.
pub fn floor_char_boundary(text: &str, offset: usize) -> usize {
	let mut offset = offset.min(text.len());
	while !text.is_char_boundary(offset) {
		offset -= 1;
	}
	offset
}

#[cfg(test)]
mod tests {
	use super::*;

	use PositionEncoding::{Utf8, Utf16, Utf32};

	fn position(line: u32, character: u32) -> Position {
		Position::new(line, character)
	}

	/// Check that every character boundary converts to a position and back.
	fn assert_round_trip(text: &str) {
		let index = LineIndex::new(text);
		for encoding in [Utf8, Utf16, Utf32] {
			for offset in (0..=text.len()).filter(|offset| text.is_char_boundary(*offset)) {
				let line = index.line_of(offset);
				// offsets inside a `\r\n` are not reachable from a position
				if offset > 0
					&& text.as_bytes()[offset - 1] == b'\r'
					&& text[offset..].starts_with('\n')
				{
					continue;
				}
				let converted = index.position(text, offset, encoding);
				assert_eq!(converted.line as usize, line, "{text:?} at {offset}");
				assert_eq!(
					index.offset(text, converted, encoding),
					offset,
					"{text:?} at {offset} with {encoding:?}"
				);
			}
		}
	}

	#[test]
	fn line_starts_of_every_line_break() {
		let index = LineIndex::new("a\nb\r\nc\rd");
		assert_eq!(index.line_count(), 4);
		assert_eq!(index.line_start(0), Some(0));
		assert_eq!(index.line_start(1), Some(2));
		assert_eq!(index.line_start(2), Some(5));
		assert_eq!(index.line_start(3), Some(7));
		assert_eq!(index.line_start(4), None);
	}

	#[test]
	fn empty_text_has_one_line() {
		let index = LineIndex::new("");
		assert_eq!(index.line_count(), 1);
		assert_eq!(index.position("", 0, Utf16), position(0, 0));
		assert_eq!(index.offset("", position(0, 5), Utf16), 0);
		assert_eq!(index.offset("", position(3, 0), Utf16), 0);
	}

	#[test]
	fn trailing_new_line_adds_an_empty_line() {
		let text = "let a = 1\n";
		let index = LineIndex::new(text);
		assert_eq!(index.line_count(), 2);
		assert_eq!(index.position(text, 10, Utf16), position(1, 0));
		assert_eq!(index.offset(text, position(1, 0), Utf16), 10);
	}

	#[test]
	fn line_end_excludes_the_line_break() {
		let text = "ab\r\ncd\nef\rgh";
		let index = LineIndex::new(text);
		assert_eq!(index.line_end(text, 0), 2);
		assert_eq!(index.line_end(text, 1), 6);
		assert_eq!(index.line_end(text, 2), 9);
		assert_eq!(index.line_end(text, 3), 12);
	}

	#[test]
	fn line_of_offsets() {
		let index = LineIndex::new("ab\ncd\n");
		assert_eq!(index.line_of(0), 0);
		assert_eq!(index.line_of(2), 0);
		assert_eq!(index.line_of(3), 1);
		assert_eq!(index.line_of(6), 2);
		assert_eq!(index.line_of(100), 2);
	}

	#[test]
	fn ascii_positions_are_bytes() {
		let text = "let a = 1\nlet b = 2";
		let index = LineIndex::new(text);
		for encoding in [Utf8, Utf16, Utf32] {
			assert_eq!(index.position(text, 14, encoding), position(1, 4));
			assert_eq!(index.offset(text, position(1, 4), encoding), 14);
		}
	}

	#[test]
	fn accented_letter_is_one_utf16_unit_and_two_bytes() {
		// `é` is U+00E9: 2 bytes, 1 UTF-16 unit
		let text = "let s = \"café\" + x";
		let index = LineIndex::new(text);
		let x = text.find('x').unwrap();
		assert_eq!(x, 18);
		assert_eq!(index.position(text, x, Utf8), position(0, 18));
		assert_eq!(index.position(text, x, Utf16), position(0, 17));
		assert_eq!(index.position(text, x, Utf32), position(0, 17));
		assert_eq!(index.offset(text, position(0, 17), Utf16), x);
	}

	#[test]
	fn emoji_is_a_surrogate_pair() {
		// `😀` is U+1F600: 4 bytes, 2 UTF-16 units, 1 char
		let text = "\"😀\" x";
		let index = LineIndex::new(text);
		let x = text.find('x').unwrap();
		assert_eq!(x, 7);
		assert_eq!(index.position(text, x, Utf8), position(0, 7));
		assert_eq!(index.position(text, x, Utf16), position(0, 5));
		assert_eq!(index.position(text, x, Utf32), position(0, 4));
		assert_eq!(index.offset(text, position(0, 5), Utf16), x);
		assert_eq!(index.offset(text, position(0, 4), Utf32), x);
	}

	#[test]
	fn column_inside_a_surrogate_pair_is_the_start_of_the_character() {
		let text = "\"😀\"";
		let index = LineIndex::new(text);
		// UTF-16 column 2 is between the two halves of the emoji, which starts at column 1
		assert_eq!(index.offset(text, position(0, 2), Utf16), 1);
		assert_eq!(index.offset(text, position(0, 3), Utf16), 5);
	}

	#[test]
	fn byte_offset_inside_a_character_is_the_start_of_the_character() {
		let text = "é";
		let index = LineIndex::new(text);
		assert_eq!(index.position(text, 1, Utf16), position(0, 0));
		assert_eq!(index.position(text, 2, Utf16), position(0, 1));
	}

	#[test]
	fn utf8_column_inside_a_character_is_the_start_of_the_character() {
		let text = "aé";
		let index = LineIndex::new(text);
		assert_eq!(index.offset(text, position(0, 2), Utf8), 1);
		assert_eq!(index.offset(text, position(0, 3), Utf8), 3);
	}

	#[test]
	fn cjk_characters_are_one_utf16_unit_and_three_bytes() {
		let text = "let 名前 = \"日本語\"\nx";
		let index = LineIndex::new(text);
		let quote = text.rfind('"').unwrap();
		assert_eq!(index.position(text, quote, Utf16), position(0, 13));
		assert_eq!(index.position(text, quote, Utf8), position(0, 23));
		assert_eq!(index.offset(text, position(0, 13), Utf16), quote);
		assert_eq!(index.position(text, text.len() - 1, Utf16), position(1, 0));
	}

	#[test]
	fn supplementary_cjk_character_is_a_surrogate_pair() {
		// `𠮷` is U+20BB7, outside the basic plane
		let text = "𠮷a";
		let index = LineIndex::new(text);
		assert_eq!(index.position(text, 4, Utf16), position(0, 2));
		assert_eq!(index.offset(text, position(0, 2), Utf16), 4);
	}

	#[test]
	fn column_past_the_end_of_the_line_is_the_end_of_the_line() {
		let text = "abc\ndef";
		let index = LineIndex::new(text);
		assert_eq!(index.offset(text, position(0, 99), Utf16), 3);
		assert_eq!(index.offset(text, position(0, 99), Utf8), 3);
		assert_eq!(index.offset(text, position(1, 99), Utf16), 7);
	}

	#[test]
	fn column_past_the_end_of_a_crlf_line_stops_before_the_line_break() {
		let text = "abc\r\ndef";
		let index = LineIndex::new(text);
		assert_eq!(index.offset(text, position(0, 4), Utf16), 3);
		assert_eq!(index.offset(text, position(1, 0), Utf16), 5);
	}

	#[test]
	fn line_past_the_end_is_the_end_of_the_text() {
		let text = "abc\ndef";
		let index = LineIndex::new(text);
		assert_eq!(index.offset(text, position(5, 0), Utf16), 7);
	}

	#[test]
	fn offset_past_the_end_is_the_end_of_the_text() {
		let text = "ab\ncd";
		let index = LineIndex::new(text);
		assert_eq!(index.position(text, 50, Utf16), position(1, 2));
	}

	#[test]
	fn offset_inside_a_crlf_is_the_end_of_the_line() {
		let text = "ab\r\ncd";
		let index = LineIndex::new(text);
		assert_eq!(index.position(text, 3, Utf16), position(0, 2));
	}

	#[test]
	fn lone_carriage_return_is_a_line_break() {
		let text = "a\rb";
		let index = LineIndex::new(text);
		assert_eq!(index.position(text, 2, Utf16), position(1, 0));
	}

	#[test]
	fn ranges_and_spans() {
		let text = "let é = 1\nlet 😀 = 2";
		let index = LineIndex::new(text);
		let second = text.find('😀').unwrap();
		let span = SourceSpan::new(second, second + '😀'.len_utf8());
		let range = index.range(text, span, Utf16);
		assert_eq!(range, Range::new(position(1, 4), position(1, 6)));
		assert_eq!(index.span(text, range, Utf16), span);
	}

	#[test]
	fn reversed_range_gives_an_ordered_span() {
		let text = "abcdef";
		let index = LineIndex::new(text);
		let span = index.span(text, Range::new(position(0, 4), position(0, 1)), Utf16);
		assert_eq!(span, SourceSpan::new(1, 4));
	}

	#[test]
	fn round_trips() {
		assert_round_trip("");
		assert_round_trip("let a = 1");
		assert_round_trip("café\nnaïve\r\nx");
		assert_round_trip("😀😃\n\n𠮷野家\rend");
		assert_round_trip("日本語\r\n中文\n한국어");
		assert_round_trip("-- comment é\n\"emoji 👩‍👩‍👧 family\"\n");
		assert_round_trip("\n\n\r\n\r");
	}

	#[test]
	fn measures_in_each_encoding() {
		assert_eq!(Utf8.measure("é😀名"), 2 + 4 + 3);
		assert_eq!(Utf16.measure("é😀名"), 1 + 2 + 1);
		assert_eq!(Utf32.measure("é😀名"), 3);
		assert_eq!(Utf16.measure("abc"), 3);
	}

	#[test]
	fn negotiation_prefers_utf8() {
		assert_eq!(PositionEncoding::negotiate(None), Utf16);
		assert_eq!(PositionEncoding::negotiate(Some(&[])), Utf16);
		assert_eq!(
			PositionEncoding::negotiate(Some(&[PositionEncodingKind::UTF16])),
			Utf16
		);
		assert_eq!(
			PositionEncoding::negotiate(Some(&[
				PositionEncodingKind::UTF16,
				PositionEncodingKind::UTF8
			])),
			Utf8
		);
		assert_eq!(
			PositionEncoding::negotiate(Some(&[PositionEncodingKind::UTF32])),
			Utf16
		);
		assert_eq!(Utf8.kind(), PositionEncodingKind::UTF8);
	}
}
