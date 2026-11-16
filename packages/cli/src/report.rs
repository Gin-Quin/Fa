//! The diagnostics of a command, kept structured until they are printed:
//! as text (`fa_analyzer::diagnostics::render`), or as JSON lines
//! (`fa check --format=json`: one object per diagnostic). `fa check`
//! analyzes again for each other target the code that depends on it, and
//! adds the diagnostics that the first analysis didn't find, marked with
//! their target ([`Report::merge_target`]).

use std::collections::HashMap;
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
	/// The index of each file in `files`, by path.
	index: HashMap<String, usize>,
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
		let entries: Vec<Entry> = diagnostics
			.iter()
			.map(|diagnostic| Entry {
				diagnostic: diagnostic.clone(),
				target: None,
			})
			.collect();
		let index = self.index_of(path, text);
		self.files[index].entries.extend(entries);
	}

	/// The index of the file of a path, added with no diagnostic when new.
	fn index_of(&mut self, path: &str, text: &str) -> usize {
		if let Some(&index) = self.index.get(path) {
			return index;
		}
		self.files.push(File {
			path: path.to_string(),
			text: text.to_string(),
			entries: Vec::new(),
		});
		self.index.insert(path.to_string(), self.files.len() - 1);
		self.files.len() - 1
	}

	/// Adds the diagnostics of the report of another target that this one
	/// doesn't have (the same code, span and message in the same file),
	/// marked with the target. The other report's warnings are the same.
	pub(crate) fn merge_target(&mut self, other: Report, target: &str) {
		for file in other.files {
			let index = self.index_of(&file.path, &file.text);
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

	/// The report as text: the warnings, then each file's diagnostics, then
	/// a summary line: `2 errors, 1 warning` for one file, `3 errors in 2
	/// files` for several; hints are left out.
	pub(crate) fn text(&self) -> String {
		let mut out = String::new();
		for warning in &self.warnings {
			out.push_str(warning);
			out.push_str("\n\n");
		}
		let shown = |file: &&File| {
			file.entries
				.iter()
				.any(|entry| entry.diagnostic.severity != Severity::Hint)
		};
		let several = self.files.iter().filter(shown).count() > 1;
		let (mut total_errors, mut total_warnings, mut total_files) = (0, 0, 0);
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
			total_errors += errors;
			total_warnings += warnings;
			total_files += usize::from(errors + warnings > 0);
			if !several && let Some(summary) = summary(errors, warnings) {
				let _ = writeln!(out, "{summary}");
			}
		}
		if several && let Some(summary) = summary(total_errors, total_warnings) {
			let _ = writeln!(out, "{summary} in {total_files} files");
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
				let fixes: Vec<_> = diagnostic
					.fixes
					.iter()
					.filter(|fix| {
						fix.edits
							.iter()
							.all(|edit| edit.location.file == diagnostic.location.file)
					})
					.collect();
				if !fixes.is_empty() {
					out.push_str(",\"fixes\":[");
					for (index, fix) in fixes.iter().enumerate() {
						if index != 0 {
							out.push(',');
						}
						let _ = write!(out, "{{\"title\":{},\"edits\":[", json_string(&fix.title));
						for (index, edit) in fix.edits.iter().enumerate() {
							if index != 0 {
								out.push(',');
							}
							let span = edit.location.span;
							let (start_line, start_column) = source.position(span.start);
							let (end_line, end_column) = source.position(span.end);
							let _ = write!(
								out,
								"{{\"file\":{},\"span\":{{\"start\":{},\"end\":{}}},\"range\":{{\"start\":{{\"line\":{start_line},\"column\":{start_column}}},\"end\":{{\"line\":{end_line},\"column\":{end_column}}}}},\"replacement\":{}}}",
								json_string(&file.path),
								span.start,
								span.end,
								json_string(&edit.replacement)
							);
						}
						out.push_str("]}");
					}
					out.push(']');
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

	#[test]
	fn json_preserves_machine_applicable_copy_edits() {
		let source = "let items = [1]\nlet duplicate = items\nconsole.log(items, duplicate)\n";
		let (session, file) = fa_analyzer::analyze_source(source);
		let mut report = Report::default();
		report.file("main.fa", source, session.diagnostics(file));
		let json = report.json();
		assert_eq!(json.lines().count(), 1, "{json}");
		assert!(json.contains("\"code\":\"MEM004\""), "{json}");
		assert!(
			json.contains("\"fixes\":[{\"title\":\"Make an independent copy of `items`\""),
			"{json}"
		);
		assert!(
			json.contains("\"file\":\"main.fa\",\"span\":{\"start\":32,\"end\":32}"),
			"{json}"
		);
		assert!(json.contains("\"replacement\":\"copy \""), "{json}");
	}
}
