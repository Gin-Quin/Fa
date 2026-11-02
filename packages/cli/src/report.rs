//! The diagnostics of a command, kept structured until they are printed:
//! as text (`fa_analyzer::diagnostics::render`), or as JSON lines
//! (`fa check --format=json`: one object per diagnostic). `fa check`
//! analyzes again for each other target the code that depends on it, and
//! adds the diagnostics that the first analysis didn't find, marked with
//! their target ([`Report::merge_target`]).

use std::fmt::Write as _;

use fa_analyzer::diagnostics::{SourceView, render};
use fa_analyzer::{Diagnostic, Severity};

/// How diagnostics are printed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Format {
	/// Rendered with their source line, on stderr.
	#[default]
	Text,
	/// One JSON object per line, on stdout.
	Json,
}

impl Format {
	/// Reads `--format=text|json`: `None` for another argument, `Some(Err)`
	/// for a bad value.
	pub(crate) fn option(text: &str) -> Option<Result<Format, ()>> {
		let value = text.strip_prefix("--format=")?;
		Some(match value {
			"text" => Ok(Format::Text),
			"json" => Ok(Format::Json),
			_ => Err(()),
		})
	}
}

/// A diagnostic of a file, and the target it was found for when it was
/// only found for another target than the first one.
struct Entry {
	diagnostic: Diagnostic,
	target: Option<String>,
}

/// The diagnostics of a file.
struct File {
	/// Its path as shown.
	path: String,
	/// Its text, for the positions and the source lines.
	text: String,
	entries: Vec<Entry>,
}

/// The diagnostics of a command.
#[derive(Default)]
pub(crate) struct Report {
	/// Messages rendered already (`DEP004`), printed first.
	warnings: Vec<String>,
	files: Vec<File>,
}

impl Report {
	/// Adds a message rendered already.
	pub(crate) fn warning(&mut self, message: &str) {
		self.warnings.push(message.to_string());
	}

	/// Adds the diagnostics of a file (nothing when there are none).
	pub(crate) fn file(&mut self, path: &str, text: &str, diagnostics: &[Diagnostic]) {
		if diagnostics.is_empty() {
			return;
		}
		let entries = diagnostics
			.iter()
			.map(|diagnostic| Entry {
				diagnostic: diagnostic.clone(),
				target: None,
			})
			.collect();
		match self.files.iter_mut().find(|file| file.path == path) {
			Some(file) => file.entries.extend(entries),
			None => self.files.push(File {
				path: path.to_string(),
				text: text.to_string(),
				entries,
			}),
		}
	}

	/// Adds the diagnostics of the report of another target that this one
	/// doesn't have (the same code, span and message in the same file),
	/// marked with the target. The other report's warnings are the same.
	pub(crate) fn merge_target(&mut self, other: Report, target: &str) {
		for file in other.files {
			let index = match self.files.iter().position(|known| known.path == file.path) {
				Some(index) => index,
				None => {
					self.files.push(File {
						path: file.path,
						text: file.text,
						entries: Vec::new(),
					});
					self.files.len() - 1
				}
			};
			let known = &mut self.files[index];
			for entry in file.entries {
				let same = |other: &Entry| {
					let (a, b) = (&other.diagnostic, &entry.diagnostic);
					a.code.id == b.code.id
						&& a.location.span == b.location.span
						&& a.message == b.message
				};
				if !known.entries.iter().any(same) {
					known.entries.push(Entry {
						diagnostic: entry.diagnostic,
						target: Some(target.to_string()),
					});
				}
			}
		}
	}

	/// The report as text: the warnings, then each file's diagnostics and
	/// its summary line (`2 errors, 1 warning`); hints are left out.
	pub(crate) fn text(&self) -> String {
		let mut out = String::new();
		for warning in &self.warnings {
			out.push_str(warning);
			out.push_str("\n\n");
		}
		for file in &self.files {
			let source = SourceView::new(&file.path, &file.text);
			let (mut errors, mut warnings) = (0, 0);
			for entry in &file.entries {
				match entry.diagnostic.severity {
					Severity::Error => errors += 1,
					Severity::Warning => warnings += 1,
					Severity::Hint => continue,
				}
				out.push_str(&render(&entry.diagnostic, &source));
				if let Some(target) = &entry.target {
					let _ = writeln!(out, "  (for the target `{target}`)");
				}
				out.push('\n');
			}
			if let Some(summary) = summary(errors, warnings) {
				let _ = writeln!(out, "{summary}");
			}
		}
		out
	}

	/// The report as JSON lines: one object per diagnostic (hints left out)
	/// with `file`, `range` (1-based lines, columns in characters, the end
	/// excluded), `severity`, `code`, `message`, and `target` when it was
	/// found for one target only. A warning rendered already has no file.
	pub(crate) fn json(&self) -> String {
		let mut out = String::new();
		for warning in &self.warnings {
			let (severity, code, message) = split_rendered(warning);
			let _ = writeln!(
				out,
				"{{\"file\":null,\"range\":null,\"severity\":{},\"code\":{},\"message\":{}}}",
				json_string(severity),
				code.map_or_else(|| "null".to_string(), json_string),
				json_string(message)
			);
		}
		for file in &self.files {
			let source = SourceView::new(&file.path, &file.text);
			for entry in &file.entries {
				let diagnostic = &entry.diagnostic;
				if diagnostic.severity == Severity::Hint {
					continue;
				}
				let span = diagnostic.location.span;
				let (start_line, start_column) = source.position(span.start);
				let (end_line, end_column) = source.position(span.end);
				let _ = write!(
					out,
					"{{\"file\":{},\"range\":{{\"start\":{{\"line\":{start_line},\"column\":{start_column}}},\"end\":{{\"line\":{end_line},\"column\":{end_column}}}}},\"severity\":{},\"code\":{},\"message\":{}",
					json_string(&file.path),
					json_string(diagnostic.severity.name()),
					json_string(diagnostic.code.id),
					json_string(&diagnostic.message)
				);
				if let Some(target) = &entry.target {
					let _ = write!(out, ",\"target\":{}", json_string(target));
				}
				out.push_str("}\n");
			}
		}
		out
	}

	/// Prints the report: as text on stderr (colored on a terminal), or as
	/// JSON lines on stdout.
	pub(crate) fn print(&self, format: Format) {
		match format {
			Format::Text => crate::color::eprint_diagnostics(&self.text()),
			Format::Json => print!("{}", self.json()),
		}
	}
}

/// The summary line of a file: `2 errors, 1 warning`.
fn summary(errors: usize, warnings: usize) -> Option<String> {
	let plural =
		|count: usize, word: &str| format!("{count} {word}{}", if count == 1 { "" } else { "s" });
	match (errors, warnings) {
		(0, 0) => None,
		(0, _) => Some(plural(warnings, "warning")),
		(_, 0) => Some(plural(errors, "error")),
		_ => Some(format!(
			"{}, {}",
			plural(errors, "error"),
			plural(warnings, "warning")
		)),
	}
}

/// The severity, code and message of a rendered message
/// (`warning[DEP004]: text`).
fn split_rendered(text: &str) -> (&str, Option<&str>, &str) {
	let text = text.trim_end();
	let parsed = (|| {
		let (head, message) = text.split_once("]: ")?;
		let (severity, code) = head.split_once('[')?;
		["error", "warning"]
			.contains(&severity)
			.then_some((severity, Some(code), message))
	})();
	parsed.unwrap_or(("warning", None, text))
}

/// A JSON string literal.
fn json_string(text: &str) -> String {
	let mut out = String::with_capacity(text.len() + 2);
	out.push('"');
	for character in text.chars() {
		match character {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			c if (c as u32) < 0x20 => {
				let _ = write!(out, "\\u{:04x}", c as u32);
			}
			c => out.push(c),
		}
	}
	out.push('"');
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn json_strings_are_escaped() {
		assert_eq!(
			json_string("a \"b\"\n\\\u{1}"),
			"\"a \\\"b\\\"\\n\\\\\\u0001\""
		);
	}

	#[test]
	fn a_rendered_warning_is_split() {
		assert_eq!(
			split_rendered("warning[DEP004]: no range\n"),
			("warning", Some("DEP004"), "no range")
		);
		assert_eq!(split_rendered("plain"), ("warning", None, "plain"));
	}

	#[test]
	fn summaries_count_errors_and_warnings() {
		assert_eq!(summary(0, 0), None);
		assert_eq!(summary(1, 0).as_deref(), Some("1 error"));
		assert_eq!(summary(2, 1).as_deref(), Some("2 errors, 1 warning"));
	}
}
