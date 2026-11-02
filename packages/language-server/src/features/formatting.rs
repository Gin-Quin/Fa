//! `textDocument/formatting`: the document formatted by `fa-formatter`, as one edit of the part
//! that changes (so that the editor keeps the cursor and the scroll position where it can).

use fa_parser::SourceSpan;
use tower_lsp::lsp_types::TextEdit;

use crate::document::Document;
use crate::line_index::PositionEncoding;

/// The edits that format the document: none if it is already formatted, `None` if it has a
/// syntax error (the formatter refuses it).
pub fn format_document(document: &Document, encoding: PositionEncoding) -> Option<Vec<TextEdit>> {
	let text = &document.text;
	let formatted = fa_formatter::format(text).ok()?;
	if formatted == *text {
		return Some(Vec::new());
	}

	// the edit covers what differs between the common prefix and the common suffix
	let mut prefix = text
		.bytes()
		.zip(formatted.bytes())
		.take_while(|(before, after)| before == after)
		.count();
	while !text.is_char_boundary(prefix) || !formatted.is_char_boundary(prefix) {
		prefix -= 1;
	}
	let longest_suffix = text.len().min(formatted.len()) - prefix;
	let mut suffix = text
		.bytes()
		.rev()
		.zip(formatted.bytes().rev())
		.take(longest_suffix)
		.take_while(|(before, after)| before == after)
		.count();
	while !text.is_char_boundary(text.len() - suffix)
		|| !formatted.is_char_boundary(formatted.len() - suffix)
	{
		suffix -= 1;
	}

	let range = document.range(SourceSpan::new(prefix, text.len() - suffix), encoding);
	Some(vec![TextEdit {
		range,
		new_text: formatted[prefix..formatted.len() - suffix].to_string(),
	}])
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The text of the document after the edits.
	fn apply(document: &Document, edits: &[TextEdit]) -> String {
		let mut text = document.text.clone();
		for edit in edits.iter().rev() {
			let span = document.span(edit.range, PositionEncoding::Utf16);
			text.replace_range(span.start..span.end, &edit.new_text);
		}
		text
	}

	#[test]
	fn formats_only_the_part_that_changes() {
		let document = Document::new("let a = 1\nlet b=2\nlet c = 3\n".to_string(), 1);
		let edits = format_document(&document, PositionEncoding::Utf16).unwrap();
		assert_eq!(edits.len(), 1);
		assert_eq!(edits[0].range.start.line, 1);
		assert_eq!(edits[0].range.end.line, 1);
		assert_eq!(
			apply(&document, &edits),
			"let a = 1\nlet b = 2\nlet c = 3\n"
		);
	}

	#[test]
	fn formats_around_multi_byte_characters() {
		let document = Document::new("let e = \"ü\"\nlet b=\"€\"\n".to_string(), 1);
		let edits = format_document(&document, PositionEncoding::Utf16).unwrap();
		assert_eq!(apply(&document, &edits), "let e = \"ü\"\nlet b = \"€\"\n");
	}

	#[test]
	fn leaves_a_formatted_document_and_a_broken_one() {
		let document = Document::new("let a = 1\n".to_string(), 1);
		assert_eq!(
			format_document(&document, PositionEncoding::Utf16),
			Some(Vec::new())
		);
		let document = Document::new("let a = (\n".to_string(), 1);
		assert_eq!(format_document(&document, PositionEncoding::Utf16), None);
	}
}
