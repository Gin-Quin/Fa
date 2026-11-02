//! Format every ```fa block of the documentation and every `.fa` file of the repository, and
//! check that:
//!
//! - without the rewrites, the formatted code parses to the same tree (`tree.to_string()`),
//! - with them, it parses without error,
//! - formatting is idempotent: `format(format(x)) == format(x)`,
//! - every comment is kept, in the same order.
//!
//! `cargo test -p fa-formatter --test corpus -- --nocapture` prints the statistics: how many
//! blocks are already formatted, and the lines that stay longer than the width.

#[path = "../../parser/tests/support/snippets.rs"]
mod support;

use std::fs;
use std::path::{Path, PathBuf};

use fa_formatter::{DEFAULT_WIDTH, Options, document::TAB_WIDTH, format_with};
use fa_parser::parse;

use support::{repository_root, snippets};

fn without_rewrites() -> Options {
	Options {
		rewrites: false,
		..Options::default()
	}
}

/// The comments of a source, without their spaces (a block comment is re-indented).
fn comments(source: &str) -> Vec<String> {
	let tree = parse(source);
	tree.comments
		.iter()
		.map(|span| source[span.start..span.end].split_whitespace().collect())
		.collect()
}

/// The failures described in full: the others are only counted, so that a bug failing every
/// case doesn't hold thousands of sources in memory.
const DESCRIBED_FAILURES: usize = 20;

/// The results of the cases, checked one at a time (only the failures and the long lines are
/// kept).
#[derive(Default)]
struct Report {
	checked: usize,
	skipped: usize,
	formatted: usize,
	long_lines: Vec<String>,
	failed: usize,
	failures: Vec<String>,
}

impl Report {
	fn fail(&mut self, description: String) {
		self.failed += 1;
		if self.failures.len() < DESCRIBED_FAILURES {
			self.failures.push(description);
		}
	}

	/// Check the formatting of `source`. Sources with syntax errors are skipped.
	fn check(&mut self, location: &str, source: &str) {
		if parse(source).has_errors() {
			self.skipped += 1;
			return;
		}
		self.checked += 1;
		let original_tree = parse(source).to_string();
		let original_comments = comments(source);

		// the tree is preserved without the rewrites
		match format_with(source, &without_rewrites()) {
			Err(errors) => {
				self.fail(format!("{location}: refused: {}", errors[0].message));
				return;
			}
			Ok(formatted) => {
				let tree = parse(&formatted);
				if tree.has_errors() {
					self.fail(format!(
						"{location}: the formatted code (without rewrites) has errors: {}\n{formatted}",
						tree.diagnostics[0].message
					));
					return;
				}
				if tree.to_string() != original_tree {
					self.fail(format!(
						"{location}: the tree changed\n--- source\n{source}\n--- formatted\n{formatted}\n--- tree before\n{original_tree}\n--- tree after\n{tree}"
					));
					return;
				}
				match format_with(&formatted, &without_rewrites()) {
					Ok(again) if again == formatted => {}
					Ok(again) => {
						self.fail(format!(
							"{location}: not idempotent (without rewrites)\n--- once\n{formatted}\n--- twice\n{again}"
						));
						return;
					}
					Err(errors) => {
						self.fail(format!(
							"{location}: refused its own output: {}",
							errors[0].message
						));
						return;
					}
				}
			}
		}

		// with the rewrites: valid, idempotent, with every comment
		let formatted = match format_with(source, &Options::default()) {
			Ok(formatted) => formatted,
			Err(errors) => {
				self.fail(format!("{location}: refused: {}", errors[0].message));
				return;
			}
		};
		let tree = parse(&formatted);
		if tree.has_errors() {
			self.fail(format!(
				"{location}: the formatted code has errors: {}\n{formatted}",
				tree.diagnostics[0].message
			));
			return;
		}
		if comments(&formatted) != original_comments {
			self.fail(format!(
				"{location}: the comments changed\n--- source\n{source}\n--- formatted\n{formatted}"
			));
			return;
		}
		match format_with(&formatted, &Options::default()) {
			Ok(again) if again == formatted => {}
			Ok(again) => {
				self.fail(format!(
					"{location}: not idempotent\n--- once\n{formatted}\n--- twice\n{again}"
				));
				return;
			}
			Err(errors) => {
				self.fail(format!(
					"{location}: refused its own output: {}",
					errors[0].message
				));
				return;
			}
		}
		if formatted == source {
			self.formatted += 1;
		}
		for (number, line) in formatted.lines().enumerate() {
			let width: usize = line
				.chars()
				.map(|character| if character == '\t' { TAB_WIDTH } else { 1 })
				.sum();
			if width > DEFAULT_WIDTH {
				self.long_lines
					.push(format!("{location} (+{}): {line}", number + 1));
			}
		}
	}

	fn finish(self, name: &str) {
		println!(
			"{name}: {} checked, {} already formatted, {} skipped (syntax errors), {} lines longer than {DEFAULT_WIDTH} columns",
			self.checked,
			self.formatted,
			self.skipped,
			self.long_lines.len()
		);
		for line in &self.long_lines {
			println!("  long line: {line}");
		}
		assert!(
			self.failed == 0,
			"{} of {} sources fail (the first {DESCRIBED_FAILURES}):\n\n{}",
			self.failed,
			self.checked,
			self.failures.join("\n\n")
		);
	}
}

#[test]
fn formats_the_documentation() {
	let mut report = Report::default();
	for snippet in snippets() {
		report.check(&snippet.location(), &snippet.code);
	}
	report.finish("documentation blocks");
}

fn collect_fa_files(directory: &Path, files: &mut Vec<PathBuf>) {
	// `build` folders hold outputs, and the generated packages of `benchmarks/build/compiler/`
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let name = entry.file_name();
		let name = name.to_string_lossy();
		if path.is_dir() {
			if !matches!(
				name.as_ref(),
				"target" | "node_modules" | ".git" | ".claude" | "build"
			) {
				collect_fa_files(&path, files);
			}
		} else if path.extension().is_some_and(|extension| extension == "fa") {
			files.push(path);
		}
	}
}

#[test]
fn formats_the_files_of_the_repository() {
	let mut files = Vec::new();
	collect_fa_files(&repository_root(), &mut files);
	files.sort();
	let mut report = Report::default();
	for file in files {
		let Ok(source) = fs::read_to_string(&file) else {
			continue;
		};
		report.check(&file.display().to_string(), &source);
	}
	report.finish(".fa files");
}

/// Every line of `code` followed by a comment.
fn with_trailing_comments(code: &str) -> String {
	code.lines()
		.enumerate()
		.map(|(index, line)| format!("{line} -- c{index}\n"))
		.collect()
}

/// A comment on its own line after every line of `code`, and a block comment before the last
/// word of the first line.
fn with_comment_lines(code: &str) -> String {
	let mut result = String::new();
	for (index, line) in code.lines().enumerate() {
		match line.rfind(' ') {
			Some(space) if index == 0 => result.push_str(&format!(
				"{} --- b{index} --- {}\n",
				&line[..space],
				&line[space + 1..]
			)),
			_ => {
				result.push_str(line);
				result.push('\n');
			}
		}
		result.push_str(&format!("\t-- l{index}\n"));
	}
	result
}

#[test]
fn keeps_the_comments_added_to_the_documentation() {
	let mut report = Report::default();
	for snippet in snippets() {
		report.check(
			&format!("{} (trailing comments)", snippet.location()),
			&with_trailing_comments(&snippet.code),
		);
		report.check(
			&format!("{} (comment lines)", snippet.location()),
			&with_comment_lines(&snippet.code),
		);
	}
	report.finish("documentation blocks with comments");
}
