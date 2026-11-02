//! The comments of a source, handed out in source order to the printer, which places each of
//! them at the first place after it where a comment (and the new line after a `--` comment) can
//! be written: no comment is ever dropped.

use fa_parser::{Node, SourceSpan, TypedSyntaxTree};

use crate::concat;
use crate::document::{Document, HardLine};

pub struct Comments<'a> {
	source: &'a str,
	spans: Vec<SourceSpan>,
	next: usize,
}

impl<'a> Comments<'a> {
	/// The comments of the tree. Comments written one right after the other (`------`) are
	/// merged, since they must stay together to be read the same way, and the ones inside a
	/// string (in an interpolation) are left out, since the string is printed as written.
	pub fn new(tree: &'a TypedSyntaxTree) -> Self {
		let source: &'a str = &tree.source;
		let mut strings: Vec<SourceSpan> = tree
			.nodes
			.iter()
			.enumerate()
			.filter(|(_, node)| matches!(node, Node::StringTemplate { .. }))
			.map(|(index, _)| tree.span(index))
			.collect();
		strings.sort_by_key(|span| span.start);

		let mut spans: Vec<SourceSpan> = Vec::with_capacity(tree.comments.len());
		let mut string_index = 0;
		for comment in &tree.comments {
			while string_index < strings.len() && strings[string_index].end <= comment.start {
				string_index += 1;
			}
			if let Some(string) = strings.get(string_index)
				&& string.start <= comment.start
				&& comment.end <= string.end
			{
				continue;
			}
			match spans.last_mut() {
				Some(previous) if previous.end == comment.start => previous.end = comment.end,
				_ => spans.push(*comment),
			}
		}
		Comments {
			source,
			spans,
			next: 0,
		}
	}

	/// The next comment, if it starts before `position`.
	pub fn peek_before(&self, position: usize) -> Option<SourceSpan> {
		self.spans
			.get(self.next)
			.copied()
			.filter(|span| span.start < position)
	}

	/// True if a comment starts before `position`.
	pub fn any_before(&self, position: usize) -> bool {
		self.peek_before(position).is_some()
	}

	/// True if a comment not printed yet starts between `from` and `to`.
	pub fn any_in(&self, from: usize, to: usize) -> bool {
		let pending = &self.spans[self.next..];
		let first = pending.partition_point(|span| span.start < from);
		pending.get(first).is_some_and(|span| span.start < to)
	}

	/// Take the next comment.
	pub fn take(&mut self) -> SourceSpan {
		let span = self.spans[self.next];
		self.next += 1;
		span
	}

	/// The number of comments not printed yet.
	pub fn remaining(&self) -> usize {
		self.spans.len() - self.next
	}

	/// True for a `--` comment, which runs to the end of its line (so a new line must follow it).
	pub fn is_line_comment(&self, span: SourceSpan) -> bool {
		!self.source[span.start..span.end].starts_with("---")
	}

	/// The document of a comment. The lines of a block comment are re-indented: the indentation
	/// of the line where the comment starts is replaced by the indentation of the code around it.
	pub fn document(&self, span: SourceSpan) -> Document<'a> {
		let text = &self.source[span.start..span.end];
		if !text.contains('\n') {
			return Document::text(text.trim_end());
		}
		self.block_document(span)
	}

	/// The document of a comment at the end of a line, which doesn't count in the width.
	pub fn trailing_document(&self, span: SourceSpan) -> Document<'a> {
		let text = &self.source[span.start..span.end];
		if !text.contains('\n') {
			return Document::TrailingComment(text.trim_end().into());
		}
		self.block_document(span)
	}

	fn block_document(&self, span: SourceSpan) -> Document<'a> {
		let text = &self.source[span.start..span.end];
		let line_start = self.source[..span.start]
			.rfind('\n')
			.map_or(0, |position| position + 1);
		let line = &self.source[line_start..span.start];
		let base = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];

		let mut parts = Vec::new();
		for (index, line) in text.split('\n').enumerate() {
			if index > 0 {
				parts.push(HardLine);
			}
			let line = if index == 0 {
				line
			} else {
				strip_indentation(line, base)
			};
			let line = line.trim_end();
			if !line.is_empty() {
				parts.push(Document::text(line));
			}
		}
		concat![Document::Concat(parts)]
	}
}

/// `line` without the `base` indentation, or without as much of it as it starts with.
fn strip_indentation<'a>(line: &'a str, base: &str) -> &'a str {
	let mut rest = line;
	for character in base.chars() {
		match rest.strip_prefix(character) {
			Some(stripped) => rest = stripped,
			None => break,
		}
	}
	rest
}
