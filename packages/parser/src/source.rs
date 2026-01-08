use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
	pub start: usize,
	pub end: usize,
}

impl SourceSpan {
	pub fn new(start: usize, end: usize) -> Self {
		SourceSpan { start, end }
	}
}

#[derive(Debug, Clone)]
pub struct SourceMap {
	pub line_starts: Vec<usize>,
}

impl SourceMap {
	pub fn new(text: &str) -> Self {
		let mut line_starts = vec![0];
		for (index, byte) in text.bytes().enumerate() {
			if byte == b'\n' {
				line_starts.push(index + 1);
			}
		}
		SourceMap { line_starts }
	}

	pub fn from_line_starts(line_starts: Vec<usize>) -> Self {
		SourceMap { line_starts }
	}

	pub fn line_col(&self, offset: usize) -> Option<(usize, usize)> {
		let line_index = match self.line_starts.binary_search(&offset) {
			Ok(index) => index,
			Err(0) => return None,
			Err(index) => index - 1,
		};
		let line_start = *self.line_starts.get(line_index)?;
		Some((line_index, offset.saturating_sub(line_start)))
	}

	pub fn span_to_line_col(&self, span: SourceSpan) -> Option<((usize, usize), (usize, usize))> {
		let start = self.line_col(span.start)?;
		let end = self.line_col(span.end)?;
		Some((start, end))
	}
}

#[derive(Debug, Clone)]
pub struct SourceFile {
	pub path: Option<PathBuf>,
	pub text: String,
	pub source_map: SourceMap,
}

impl SourceFile {
	pub fn new(text: impl Into<String>) -> Self {
		let text = text.into();
		let source_map = SourceMap::new(&text);
		SourceFile {
			path: None,
			text,
			source_map,
		}
	}

	pub fn with_source_map(text: impl Into<String>, source_map: SourceMap) -> Self {
		SourceFile {
			path: None,
			text: text.into(),
			source_map,
		}
	}

	pub fn with_path(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
		let text = text.into();
		let source_map = SourceMap::new(&text);
		SourceFile {
			path: Some(path.into()),
			text,
			source_map,
		}
	}

	pub fn slice(&self, span: SourceSpan) -> Option<&str> {
		self.text.get(span.start..span.end)
	}

	pub fn line_col(&self, offset: usize) -> Option<(usize, usize)> {
		self.source_map.line_col(offset)
	}

	pub fn span_to_line_col(&self, span: SourceSpan) -> Option<((usize, usize), (usize, usize))> {
		self.source_map.span_to_line_col(span)
	}
}
