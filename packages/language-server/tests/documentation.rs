//! Run every feature on every ```fa block of the documentation, and on truncated versions of
//! them: no feature may panic, and the results must be well-formed LSP.

use std::fs;
use std::path::{Path, PathBuf};

use fa_language_server::document::Document;
use fa_language_server::features::{folding, selection, semantic_tokens, symbols};
use fa_language_server::line_index::PositionEncoding;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range};

fn markdown_files(directory: &Path, files: &mut Vec<PathBuf>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			markdown_files(&path, files);
		} else if path.extension().is_some_and(|extension| extension == "md") {
			files.push(path);
		}
	}
}

/// The code of every ```fa block of the documentation.
fn snippets() -> Vec<String> {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../documentation");
	let mut files = Vec::new();
	markdown_files(&root, &mut files);
	files.sort();
	let mut snippets = Vec::new();
	for file in files {
		let text = fs::read_to_string(&file).unwrap();
		let mut current: Option<String> = None;
		for line in text.lines() {
			match &mut current {
				None if line.trim_start() == "```fa" => current = Some(String::new()),
				Some(code) if line.trim_start() == "```" => {
					snippets.push(std::mem::take(code));
					current = None;
				}
				Some(code) => {
					code.push_str(line);
					code.push('\n');
				}
				None => {}
			}
		}
	}
	snippets
}

fn before_or_equal(a: Position, b: Position) -> bool {
	(a.line, a.character) <= (b.line, b.character)
}

fn assert_range(range: Range, document: &Document, encoding: PositionEncoding) {
	assert!(before_or_equal(range.start, range.end), "{range:?}");
	let end = document.position(document.text.len(), encoding);
	assert!(before_or_equal(range.end, end), "{range:?} after {end:?}");
}

fn assert_symbols(symbols: &[DocumentSymbol], document: &Document, encoding: PositionEncoding) {
	for symbol in symbols {
		assert!(!symbol.name.is_empty());
		assert_range(symbol.range, document, encoding);
		assert!(before_or_equal(
			symbol.range.start,
			symbol.selection_range.start
		));
		assert!(before_or_equal(
			symbol.selection_range.end,
			symbol.range.end
		));
		if let Some(children) = &symbol.children {
			assert_symbols(children, document, encoding);
		}
	}
}

fn check(code: &str) {
	let tree = fa_parser::parse(code);
	let document = Document::new(code.to_string(), 1);
	for encoding in [PositionEncoding::Utf16, PositionEncoding::Utf8] {
		assert_symbols(
			&symbols::document_symbols(&tree, &document, encoding),
			&document,
			encoding,
		);

		let mut line = 0;
		let mut character = 0;
		for token in semantic_tokens::semantic_tokens(&tree, &document, encoding) {
			assert!(token.length > 0);
			if token.delta_line > 0 {
				line += token.delta_line;
				character = token.delta_start;
			} else {
				character += token.delta_start;
			}
			let line_end = document.line_index.line_end(&document.text, line as usize);
			let line_end = document.position(line_end, encoding).character;
			assert!(character + token.length <= line_end, "token past its line");
		}

		for offset in (0..=code.len()).step_by(7) {
			let offset = fa_language_server::line_index::floor_char_boundary(code, offset);
			let mut range = Some(selection::selection_range(
				&tree, &document, offset, encoding,
			));
			let mut previous: Option<Range> = None;
			while let Some(current) = range {
				assert_range(current.range, &document, encoding);
				if let Some(previous) = previous {
					assert!(before_or_equal(current.range.start, previous.start));
					assert!(before_or_equal(previous.end, current.range.end));
				}
				previous = Some(current.range);
				range = current.parent.map(|parent| *parent);
			}
		}
	}

	let folds = folding::folding_ranges(&tree, &document);
	for window in folds.windows(2) {
		assert!(window[0].start_line < window[1].start_line);
	}
	for fold in folds {
		assert!(fold.start_line < fold.end_line);
		assert!((fold.end_line as usize) < document.line_index.line_count());
	}
}

#[test]
fn every_documentation_snippet() {
	let snippets = snippets();
	assert!(snippets.len() > 100, "found {} snippets", snippets.len());
	for snippet in &snippets {
		check(snippet);
	}
}

#[test]
fn truncated_documentation_snippets() {
	for snippet in snippets() {
		let boundaries: Vec<usize> = (0..snippet.len())
			.filter(|offset| snippet.is_char_boundary(*offset))
			.collect();
		for offset in boundaries.iter().step_by(boundaries.len() / 4 + 1) {
			check(&snippet[..*offset]);
		}
	}
}

#[test]
fn multi_byte_text_everywhere() {
	check("let s = \"é😀日本\"\n-- 😀 comment é\n---\nblock 日本\n---\nlet t = \"{s}😀{s}\"");
	check("😀😀😀\n\"unclosed é");
	check("let a = \"{ \"😀\" }\"\r\nlet b = 1\r\n");
}
