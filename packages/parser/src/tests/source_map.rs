use crate::diagnostic::Diagnostic;
use crate::parsing::parse::parse;
use crate::source::{SourceFile, SourceMap, SourceSpan};

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
fn parsed_tree_has_a_source_map() {
	let tree = parse("a\n b\nc");
	assert_eq!(tree.source_map.line_starts, vec![0, 2, 5]);
}

#[test]
fn line_col_of_an_empty_source_and_of_its_end() {
	let map = SourceMap::new("");
	assert_eq!(map.line_col(0), Some((0, 0)));
	let map = SourceMap::new("ab\n");
	assert_eq!(map.line_col(3), Some((1, 0)));
}

#[test]
fn source_file_slices_its_text() {
	let source = SourceFile::with_path("main.fa", "let a = 1");
	assert_eq!(
		source.path.as_deref(),
		Some(std::path::Path::new("main.fa"))
	);
	assert_eq!(source.slice(SourceSpan::new(4, 5)), Some("a"));
	assert_eq!(source.slice(SourceSpan::new(4, 50)), None);
}

#[test]
fn warnings_are_not_errors() {
	let warning = Diagnostic::warning("Unused value", SourceSpan::new(4, 5));
	assert!(!warning.is_error());
	let map = SourceMap::new("let a = 1");
	assert_eq!(
		warning.display(&map).to_string(),
		"1:5: warning: Unused value"
	);
}

#[test]
fn related_information_is_kept() {
	let diagnostic = Diagnostic::error("Never closed", SourceSpan::new(0, 1))
		.with_related(SourceSpan::new(5, 6), "opened here");
	assert_eq!(
		diagnostic.related,
		vec![(SourceSpan::new(5, 6), "opened here".to_string())]
	);
}

#[test]
fn second_error_at_the_same_position_is_dropped() {
	// `1` is reported once, not again as an unexpected token after the statement
	let tree = parse("let 1 = 2");
	assert_eq!(tree.diagnostics.len(), 1);
}
