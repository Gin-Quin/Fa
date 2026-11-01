use std::fmt;

use crate::source::{SourceMap, SourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
	Error,
	Warning,
}

/// A problem found while tokenizing or parsing. The message says what is wrong and how to fix
/// it; the span covers the code to fix (it can be empty, for something missing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
	pub severity: Severity,
	pub message: String,
	pub span: SourceSpan,
	// other places that explain the problem, like where an unclosed bracket was opened
	pub related: Vec<(SourceSpan, String)>,
}

impl Diagnostic {
	pub fn error(message: impl Into<String>, span: SourceSpan) -> Self {
		Diagnostic {
			severity: Severity::Error,
			message: message.into(),
			span,
			related: Vec::new(),
		}
	}

	pub fn warning(message: impl Into<String>, span: SourceSpan) -> Self {
		Diagnostic {
			severity: Severity::Warning,
			message: message.into(),
			span,
			related: Vec::new(),
		}
	}

	pub fn with_related(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
		self.related.push((span, message.into()));
		self
	}

	pub fn is_error(&self) -> bool {
		self.severity == Severity::Error
	}

	/// Format the diagnostic with a 1-based line and column, like `3:14: error: ...`.
	pub fn display<'a>(&'a self, source_map: &'a SourceMap) -> DiagnosticDisplay<'a> {
		DiagnosticDisplay {
			diagnostic: self,
			source_map,
		}
	}
}

pub struct DiagnosticDisplay<'a> {
	diagnostic: &'a Diagnostic,
	source_map: &'a SourceMap,
}

impl fmt::Display for DiagnosticDisplay<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let (line, column) = self
			.source_map
			.line_col(self.diagnostic.span.start)
			.unwrap_or((0, 0));
		let severity = match self.diagnostic.severity {
			Severity::Error => "error",
			Severity::Warning => "warning",
		};
		write!(
			formatter,
			"{}:{}: {severity}: {}",
			line + 1,
			column + 1,
			self.diagnostic.message
		)
	}
}
