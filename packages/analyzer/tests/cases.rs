//! The data-driven harness (design §6): every `tests/cases/**/*.fa` file is
//! analyzed and its diagnostics compared with its annotations.
//!
//! Annotations are Fa comments:
//!
//! ```fa
//! let e: Int8 = 100 + 100          -- error: E-NUM-10
//! let s = a + b                    -- error: E-NUM-15, warning: W-NUM-2
//! let x = compute()
//! -- ^ error: E-ERR-3              (applies to the previous code line)
//! -- error: TY002                  (a standalone line: anywhere in the file)
//! let wide: Int16 = a + b          -- no-check
//! let avg: Int8 = f(a)             -- check: SINK_CHECK
//! let count = 0                    -- type count: Integer
//! console.log(x)                   -- prints: 3   (ignored: backend tests)
//! ```
//!
//! Headers: `-- pending: M2 (reason)` (an expected failure, reported when it
//! starts passing), `-- kind: …`, `-- target: js` (backend-only: skipped),
//! `-- file: path` (starts a file of a multi-file program, M2). The colon
//! after `error`/`warning` is optional and codes may be followed by a note in
//! parentheses. Every expected diagnostic must be emitted on its line with its
//! code or an alias; every emitted error or warning must be expected (hints
//! are ignored).
//!
//! `FA_STRICT_PENDING=1` makes a pending case that passes fail the run.
//! `FA_CASES=filter` runs only the cases whose path contains `filter`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::diagnostics::{LineIndex, codes};
use fa_analyzer::ids::NodeId;
use fa_analyzer::{Severity, analyze_source};

/// An expected diagnostic: its canonical code, and its line (`None`: anywhere).
#[derive(Clone, Debug, PartialEq)]
struct Expected {
	line: Option<u32>,
	code: &'static str,
	severity: Severity,
}

/// A per-line expectation about the analysis tables.
#[derive(Clone, Debug)]
enum LineCheck {
	/// No runtime check on the line.
	NoCheck(u32),
	/// These flags on the line.
	Flags(u32, NodeFlags),
	/// The hover type of the first occurrence of a name on the line.
	Type(u32, String, String),
}

/// One file of a case.
struct CaseFile {
	path: String,
	text: String,
	expected: Vec<Expected>,
	checks: Vec<LineCheck>,
}

/// A parsed case.
struct Case {
	pending: Option<String>,
	backend_only: bool,
	files: Vec<CaseFile>,
	errors: Vec<String>,
}

fn find_comment(line: &str) -> Option<usize> {
	let bytes = line.as_bytes();
	let mut in_string = false;
	let mut index = 0;
	while index + 1 < bytes.len() {
		match bytes[index] {
			b'\\' if in_string => index += 1,
			b'"' => in_string = !in_string,
			b'-' if !in_string && bytes[index + 1] == b'-' => return Some(index),
			_ => {}
		}
		index += 1;
	}
	None
}

/// The codes after each `error` / `warning` keyword of a comment.
fn diagnostics_in(comment: &str, errors: &mut Vec<String>) -> Vec<(&'static str, Severity)> {
	let mut found = Vec::new();
	let words: Vec<&str> = comment
		.split(|c: char| c.is_whitespace() || c == ',' || c == ':' || c == ';')
		.filter(|word| !word.is_empty())
		.collect();
	let mut index = 0;
	while index < words.len() {
		let severity = match words[index] {
			"error" => Severity::Error,
			"warning" => Severity::Warning,
			_ => {
				index += 1;
				continue;
			}
		};
		index += 1;
		while index < words.len() && is_code_shaped(words[index]) {
			match codes::lookup(words[index]) {
				Some(code) => found.push((code.id, severity)),
				None => errors.push(format!("unknown diagnostic code `{}`", words[index])),
			}
			index += 1;
		}
	}
	found
}

fn is_code_shaped(word: &str) -> bool {
	word.len() >= 4
		&& word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
		&& word.chars().last().is_some_and(|c| c.is_ascii_digit())
		&& word
			.chars()
			.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

fn parse_case(text: &str) -> Case {
	let mut case = Case {
		pending: None,
		backend_only: false,
		files: Vec::new(),
		errors: Vec::new(),
	};
	let mut current = CaseFile {
		path: "main.fa".into(),
		text: String::new(),
		expected: Vec::new(),
		checks: Vec::new(),
	};
	let mut line_number = 0u32;
	let mut last_code_line = None;
	for line in text.lines() {
		let trimmed = line.trim_start();
		if let Some(path) = trimmed.strip_prefix("-- file:") {
			let has_code = current.text.lines().any(|line| {
				let line = line.trim();
				!line.is_empty() && !line.starts_with("--")
			});
			if has_code || !current.expected.is_empty() || !case.files.is_empty() {
				case.files.push(current);
			}
			current = CaseFile {
				path: path.trim().into(),
				text: String::new(),
				expected: Vec::new(),
				checks: Vec::new(),
			};
			line_number = 0;
			last_code_line = None;
			continue;
		}
		line_number += 1;
		current.text.push_str(line);
		current.text.push('\n');
		let Some(start) = find_comment(line) else {
			if !trimmed.is_empty() {
				last_code_line = Some(line_number);
			}
			continue;
		};
		let standalone = line[..start].trim().is_empty();
		let mut comment = line[start + 2..].trim();
		let target = if standalone {
			if let Some(rest) = comment.strip_prefix('^') {
				comment = rest.trim();
				last_code_line
			} else {
				None
			}
		} else {
			last_code_line = Some(line_number);
			Some(line_number)
		};
		if standalone && target.is_none() {
			let key = comment.split(':').next().unwrap_or("").trim();
			match key {
				"pending" => {
					case.pending = Some(
						comment["pending:".len().min(comment.len())..]
							.trim()
							.to_string(),
					);
					continue;
				}
				"target" => {
					case.backend_only = true;
					continue;
				}
				"brief" | "context" | "review" | "kind" | "prints" | "exit" | "note" => continue,
				_ => {}
			}
			if !(comment.starts_with("error") || comment.starts_with("warning")) {
				continue;
			}
		}
		for (code, severity) in diagnostics_in(comment, &mut case.errors) {
			current.expected.push(Expected {
				line: target,
				code,
				severity,
			});
		}
		let Some(line) = target else { continue };
		if comment.contains("no-check") {
			current.checks.push(LineCheck::NoCheck(line));
		}
		if let Some(rest) = comment
			.split("check:")
			.nth(1)
			.filter(|_| !comment.contains("no-check:"))
		{
			for name in rest
				.split(|c: char| c == ',' || c.is_whitespace())
				.filter(|word| !word.is_empty())
			{
				if !name.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
					break;
				}
				match NodeFlags::from_name(name) {
					Some(flag) => current.checks.push(LineCheck::Flags(line, flag)),
					None => case.errors.push(format!("unknown flag `{name}`")),
				}
			}
		}
		if let Some(rest) = comment.strip_prefix("type ")
			&& let Some((name, ty)) = rest.split_once(':')
		{
			current.checks.push(LineCheck::Type(
				line,
				name.trim().to_string(),
				ty.trim().to_string(),
			));
		}
	}
	case.files.push(current);
	case
}

/// Runs one file and returns what doesn't match.
fn run_file(file: &CaseFile) -> Vec<String> {
	let mut problems = Vec::new();
	let (session, id) = analyze_source(&file.text);
	let lines = LineIndex::new(&file.text);
	let mut emitted: Vec<(u32, &'static str, Severity, String)> = session
		.diagnostics(id)
		.iter()
		.filter(|d| d.severity != Severity::Hint)
		.map(|d| {
			(
				lines.line(d.location.span.start),
				d.code.id,
				d.severity,
				d.message.clone(),
			)
		})
		.collect();
	let mut expected = file.expected.clone();
	// line-anchored expectations first, then the ones that may be anywhere
	expected.sort_by_key(|e| e.line.is_none());
	for expectation in &expected {
		let position = emitted.iter().position(|(line, code, _, _)| {
			*code == expectation.code && expectation.line.is_none_or(|l| l == *line)
		});
		match position {
			Some(index) => {
				let (_, _, severity, _) = emitted.remove(index);
				if severity != expectation.severity {
					problems.push(format!(
						"{}: {} is a {}, not a {}",
						file.path,
						expectation.code,
						severity.name(),
						expectation.severity.name()
					));
				}
			}
			None => match expectation.line {
				Some(line) => problems.push(format!(
					"{}:{line}: expected {} {}",
					file.path,
					expectation.severity.name(),
					expectation.code
				)),
				None => problems.push(format!(
					"{}: expected {} {} somewhere",
					file.path,
					expectation.severity.name(),
					expectation.code
				)),
			},
		}
	}
	for (line, code, severity, message) in emitted {
		problems.push(format!(
			"{}:{line}: unexpected {} {code}: {message}",
			file.path,
			severity.name()
		));
	}

	let tree = session.tree(id);
	let analysis = session.analysis(id);
	let flags_on = |line: u32| {
		(0..tree.nodes.len())
			.filter(|&node| lines.line(tree.spans[node].start as u32) == line)
			.fold(NodeFlags::empty(), |flags, node| {
				flags | analysis.flags_of(NodeId::new(node))
			})
	};
	for check in &file.checks {
		match check {
			LineCheck::NoCheck(line) => {
				let found = flags_on(*line) & NodeFlags::CHECKS;
				if !found.is_empty() {
					problems.push(format!(
						"{}:{line}: expected no runtime check, found {found:?}",
						file.path
					));
				}
			}
			LineCheck::Flags(line, flag) => {
				if !flags_on(*line).contains(*flag) {
					problems.push(format!("{}:{line}: expected {flag:?}", file.path));
				}
			}
			LineCheck::Type(line, name, ty) => {
				let actual = type_of_name(&session, id, &file.text, *line, name);
				if actual.as_deref() != Some(ty.as_str()) {
					problems.push(format!(
						"{}:{line}: expected `{name}: {ty}`, found {actual:?}",
						file.path
					));
				}
			}
		}
	}
	problems
}

/// The type of the innermost node starting at the first occurrence of `name` on `line`.
fn type_of_name(
	session: &fa_analyzer::Session,
	file: fa_analyzer::ids::FileId,
	text: &str,
	line: u32,
	name: &str,
) -> Option<String> {
	let line_start: usize = text
		.split_inclusive('\n')
		.take(line as usize - 1)
		.map(str::len)
		.sum();
	let line_text = text[line_start..].lines().next()?;
	let column = line_text
		.match_indices(name)
		.map(|(index, _)| index)
		.find(|&index| {
			let before = line_text[..index].chars().next_back();
			let after = line_text[index + name.len()..].chars().next();
			!before.is_some_and(|c| c.is_alphanumeric() || c == '_')
				&& !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
		})?;
	let start = line_start + column;
	let tree = session.tree(file);
	let node = (0..tree.nodes.len())
		.filter(|&node| {
			tree.spans[node].start == start && tree.spans[node].end >= start + name.len()
		})
		.min_by_key(|&node| tree.spans[node].end - tree.spans[node].start)?;
	let ty = session.analysis(file).type_of(NodeId::new(node));
	(!ty.is_unset()).then(|| session.display_type(ty))
}

fn collect(directory: &Path, out: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect(&path, out);
		} else if path.extension().is_some_and(|extension| extension == "fa") {
			out.push(path);
		}
	}
}

#[derive(Default)]
struct Tally {
	passed: usize,
	pending: usize,
	pending_passing: usize,
	failed: usize,
	skipped: usize,
	/// Cases the parser doesn't accept (an unexpected `AN002`).
	syntax_errors: usize,
}

#[test]
fn cases() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("cases");
	let mut paths = Vec::new();
	collect(&root, &mut paths);
	paths.sort();
	let filter = std::env::var("FA_CASES").ok();
	let strict = std::env::var("FA_STRICT_PENDING").is_ok_and(|value| value == "1");

	let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
	let mut failures = String::new();
	let mut now_passing = Vec::new();
	for path in &paths {
		let relative = path
			.strip_prefix(&root)
			.unwrap()
			.to_string_lossy()
			.replace('\\', "/");
		if filter
			.as_ref()
			.is_some_and(|filter| !relative.contains(filter.as_str()))
		{
			continue;
		}
		let group = relative.split('/').next().unwrap_or("").to_string();
		let tally = tallies.entry(group).or_default();
		let text = std::fs::read_to_string(path).unwrap();
		let case = parse_case(&text);
		if !case.errors.is_empty() {
			tally.failed += 1;
			let _ = writeln!(
				failures,
				"{relative}: invalid annotations: {}",
				case.errors.join("; ")
			);
			continue;
		}
		if case.backend_only {
			tally.skipped += 1;
			continue;
		}
		let result = panic::catch_unwind(AssertUnwindSafe(|| {
			if case.files.len() > 1 {
				// multi-file programs need `Session::analyze_program` (M2): each file must at least not panic
				let mut problems: Vec<String> = case.files.iter().flat_map(run_file).collect();
				problems.push("multi-file programs are analyzed from M2 on".into());
				problems
			} else {
				run_file(&case.files[0])
			}
		}));
		let problems = result.unwrap_or_else(|_| vec!["the analyzer panicked".into()]);
		let panicked = problems.iter().any(|problem| problem.contains("panicked"));
		if problems
			.iter()
			.any(|problem| problem.contains("unexpected error AN002"))
		{
			tally.syntax_errors += 1;
		}
		match (&case.pending, problems.is_empty()) {
			(Some(_), _) if panicked => {
				tally.failed += 1;
				let _ = writeln!(failures, "{relative}: the analyzer panicked");
			}
			(Some(_), false) => tally.pending += 1,
			(Some(_), true) => {
				tally.pending_passing += 1;
				now_passing.push(relative.clone());
			}
			(None, true) => tally.passed += 1,
			(None, false) => {
				tally.failed += 1;
				let _ = writeln!(failures, "{relative}:\n  {}", problems.join("\n  "));
			}
		}
	}

	println!("\ncase results:");
	for (group, tally) in &tallies {
		println!(
			"  {group:10} {} passed, {} failed, {} pending ({} of them now pass), {} backend-only, {} with syntax errors",
			tally.passed,
			tally.failed,
			tally.pending + tally.pending_passing,
			tally.pending_passing,
			tally.skipped,
			tally.syntax_errors
		);
	}
	if !now_passing.is_empty() {
		println!(
			"{} pending cases now pass, remove their `-- pending` line:",
			now_passing.len()
		);
		for path in now_passing.iter().take(20) {
			println!("  {path}");
		}
		if now_passing.len() > 20 {
			println!("  … (set FA_CASES to list a subset)");
		}
	}
	let strict_failure = strict && !now_passing.is_empty();
	assert!(
		failures.is_empty() && !strict_failure,
		"failing cases:\n{failures}"
	);
}

/// The analyzer never panics on a snippet of the documentation (design §6, corpus tests).
#[test]
fn documentation_snippets_do_not_panic() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../documentation");
	let mut pages = Vec::new();
	fn markdown(directory: &Path, out: &mut Vec<PathBuf>) {
		let Ok(entries) = std::fs::read_dir(directory) else {
			return;
		};
		for entry in entries.flatten() {
			let path = entry.path();
			if path.is_dir() {
				markdown(&path, out);
			} else if path.extension().is_some_and(|extension| extension == "md") {
				out.push(path);
			}
		}
	}
	markdown(&root, &mut pages);
	let mut snippets = 0;
	for page in pages {
		let text = std::fs::read_to_string(&page).unwrap();
		let mut lines = text.lines();
		while let Some(line) = lines.next() {
			if line.trim() != "```fa" {
				continue;
			}
			let snippet: Vec<&str> = lines
				.by_ref()
				.take_while(|line| line.trim() != "```")
				.collect();
			let snippet = snippet.join("\n");
			let result = panic::catch_unwind(|| {
				analyze_source(&snippet)
					.0
					.diagnostics(fa_analyzer::ids::FileId(0))
					.len()
			});
			assert!(
				result.is_ok(),
				"the analyzer panicked on a snippet of {}:\n{snippet}",
				page.display()
			);
			snippets += 1;
		}
	}
	assert!(snippets > 100, "only {snippets} snippets found");
}

#[test]
fn annotations_are_parsed() {
	let case = parse_case(
		"-- pending: M2 (later)\n\
		 -- error: TY002\n\
		 let e: Int8 = 100 + 100 -- error: E-NUM-10\n\
		 let s = a + b -- error E-NUM-15, warning: W-NUM-2 (note)\n\
		 let x = compute()\n\
		 -- ^ error: EF015\n\
		 let wide: Int16 = a + b -- no-check\n\
		 let avg: Int8 = f(a) -- check: SINK_CHECK, OVERFLOW_NATIVE\n\
		 let count = 0 -- type count: Integer\n\
		 let text = \"a -- b\" -- prints: 3\n",
	);
	assert!(case.errors.is_empty(), "{:?}", case.errors);
	assert_eq!(case.pending.as_deref(), Some("M2 (later)"));
	let file = &case.files[0];
	let expected: Vec<(Option<u32>, &str)> =
		file.expected.iter().map(|e| (e.line, e.code)).collect();
	assert_eq!(
		expected,
		[
			(None, "TY002"),
			(Some(3), "E-NUM-10"),
			(Some(4), "E-NUM-15"),
			(Some(4), "W-NUM-2"),
			(Some(5), "E-ERR-3")
		]
	);
	assert_eq!(file.expected[3].severity, Severity::Warning);
	assert_eq!(file.checks.len(), 4);
	assert!(
		matches!(&file.checks[3], LineCheck::Type(9, name, ty) if name == "count" && ty == "Integer")
	);

	let unknown = parse_case("let a = 1 -- error: E-NOPE-1\n");
	assert_eq!(unknown.errors.len(), 1);

	let multi =
		parse_case("-- file: a.fa\nexport = 1\n-- file: main.fa\nlet b = 2 -- error: MOD012\n");
	assert_eq!(multi.files.len(), 2);
	assert_eq!(multi.files[1].path, "main.fa");
	assert_eq!(multi.files[1].expected[0].line, Some(1));
}
