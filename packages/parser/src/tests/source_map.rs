use crate::source::{SourceFile, SourceMap, SourceSpan};
use crate::tokenize::tokenize_with_source_map;

#[test]
fn source_map_line_starts() {
	let map = SourceMap::new("a\nb\nc");
	assert_eq!(map.line_starts, vec![0, 2, 4]);
}

#[test]
fn source_file_line_col() {
	let source = SourceFile::new("alpha\nbeta\ngamma");
	assert_eq!(source.line_col(0), Some((0, 0)));
	assert_eq!(source.line_col(3), Some((0, 3)));
	assert_eq!(source.line_col(6), Some((1, 0)));
	assert_eq!(source.line_col(10), Some((1, 4)));
}

#[test]
fn source_file_span_to_line_col() {
	let source = SourceFile::new("ab\ncd\nef");
	let span = SourceSpan::new(3, 5);
	assert_eq!(source.span_to_line_col(span), Some(((1, 0), (1, 2))));
}

#[test]
fn tokenizer_builds_source_map() {
	let (_, source_map) = tokenize_with_source_map("a\n b\nc".as_bytes());
	assert_eq!(source_map.line_starts, vec![0, 2, 5]);
}
