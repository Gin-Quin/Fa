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
//! let r = try g()                  -- no-check: NARROWED, UNREACHABLE  (none of these flags)
//! let count = 0                    -- type count: Integer
//! console.log(x)                   -- prints: 3   (ignored: backend tests)
//! ```
//!
//! Headers: `-- pending: M2 (reason)` (an expected failure, reported when it
//! starts passing), `-- kind: test` (test files: `main.test.fa`), `-- target: js` (backend-only: skipped),
//! `-- analysis-target: web` (the target of the analysis, `bun` by default),
//! `-- file: path` (starts a file of a multi-file program, M2). The colon
//! after `error`/`warning` is optional and codes may be followed by a note in
//! parentheses. Every expected diagnostic must be emitted on its line with its
//! code or an alias; every emitted error or warning must be expected (hints
//! are ignored, and so is the unused-value warning `W-DECL-2` in the files
//! that don't name it).
//!
//! `FA_STRICT_PENDING=1` makes a pending case that passes fail the run.
//! `FA_CASES=filter` runs only the cases whose path contains `filter`.
//! `FA_VERBOSE=1` also prints the problems of the pending cases.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::diagnostics::{LineIndex, codes};
use fa_analyzer::ids::NodeId;
use fa_analyzer::{CodebaseResolver, Session, Severity};

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
	/// None of these flags on the line.
	Absent(u32, NodeFlags),
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
	/// `-- kind: test`: the files are test files (`main.test.fa`).
	test_files: bool,
	/// `-- analysis-target: web`: the target of the analysis (`bun` otherwise).
	analysis_target: Option<String>,
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
		test_files: false,
		analysis_target: None,
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
				"analysis-target" => {
					case.analysis_target = comment.split(':').nth(1).map(|t| t.trim().to_string());
					continue;
				}
				"kind" => {
					case.test_files = comment.contains("test");
					continue;
				}
				// `-- native` (the brief's notation): `-- analysis-target: native`
				"native" => {
					case.analysis_target = Some("native".to_string());
					continue;
				}
				"brief" | "context" | "review" | "prints" | "exit" | "note" => continue,
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
		let absent = comment.contains("no-check:");
		if comment.contains("no-check") && !absent {
			current.checks.push(LineCheck::NoCheck(line));
		}
		if let Some(rest) = comment.split("check:").nth(1) {
			for name in rest
				.split(|c: char| c == ',' || c.is_whitespace())
				.filter(|word| !word.is_empty())
			{
				if !name.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
					break;
				}
				match NodeFlags::from_name(name) {
					Some(flag) if absent => current.checks.push(LineCheck::Absent(line, flag)),
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

/// Runs one file and returns what doesn't match. A test file is the module
/// `main.test` (its path without `.fa` otherwise).
fn run_file(file: &CaseFile, test_file: bool, target: Option<&str>) -> Vec<String> {
	let stem = file.path.strip_suffix(".fa").unwrap_or(&file.path);
	let module = if test_file && !stem.ends_with(".test") {
		format!("{stem}.test")
	} else {
		stem.to_string()
	};
	let mut session = Session::new();
	if let Some(target) = target {
		session.set_target(target);
	}
	let id = session.set_source(&module, Arc::from(file.text.as_str()));
	session.analyze_file(id);
	compare(&session, id, file)
}

/// Runs a multi-file program (the `-- file:` sections): a package when a
/// path starts with `library/`, `application/` or `scripts/`, a standalone
/// program (`main.fa`, or the first file, with its siblings) otherwise.
/// Every file is analyzed, and compared with its own annotations.
fn run_program(files: &[CaseFile], target: Option<&str>) -> Vec<String> {
	let texts: Vec<(String, String)> = files
		.iter()
		.map(|file| (file.path.clone(), file.text.clone()))
		.collect();
	let entry = files
		.iter()
		.find(|file| file.path == "main.fa")
		.or_else(|| files.iter().find(|file| file.path.ends_with(".fa")))
		.unwrap_or(&files[0])
		.path
		.clone();
	let resolver = CodebaseResolver::from_texts(&texts, Some(&entry));
	let is_package = resolver.codebase.layout == fa_codebase_parser::Layout::Package;
	let mut session = Session::new();
	if let Some(target) = target {
		session.set_target(target);
	}
	session.set_resolver(Box::new(resolver));
	if is_package {
		let entry = files
			.iter()
			.any(|file| file.path == "application/main.fa")
			.then_some("application/main");
		session.analyze_package(entry);
	} else {
		let module = entry.strip_suffix(".fa").unwrap_or(&entry).to_string();
		session.analyze_program(&module);
		// the siblings the program doesn't use are checked too
		for file in files.iter().filter(|file| file.path.ends_with(".fa")) {
			let module = file.path.strip_suffix(".fa").unwrap_or(&file.path);
			if let Some(id) = session.file_of_module(module) {
				session.analyze_file(id);
			}
		}
	}
	let mut problems = Vec::new();
	// the other files are build inputs (`@readFile`)
	for file in files.iter().filter(|file| file.path.ends_with(".fa")) {
		let module = file.path.strip_suffix(".fa").unwrap_or(&file.path);
		match session.loaded_module(module) {
			Some(id) => problems.extend(compare(&session, id, file)),
			None => problems.push(format!("{}: not loaded", file.path)),
		}
	}
	problems
}

/// Compares the analysis of a file with its annotations.
fn compare(session: &Session, id: fa_analyzer::ids::FileId, file: &CaseFile) -> Vec<String> {
	let mut problems = Vec::new();
	let lines = LineIndex::new(&file.text);
	let mut emitted: Vec<(u32, &'static str, Severity, String)> = session
		.diagnostics(id)
		.iter()
		.filter(|d| d.severity != Severity::Hint)
		// the unused-value warning is checked by the cases that name it
		.filter(|d| d.code.id != "W-DECL-2" || file.text.contains("W-DECL-2"))
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
			LineCheck::Absent(line, flag) => {
				if flags_on(*line).intersects(*flag) {
					problems.push(format!("{}:{line}: expected no {flag:?}", file.path));
				}
			}
			LineCheck::Type(line, name, ty) => {
				let actual = type_of_name(session, id, &file.text, *line, name);
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
	// a declared name is not a node: the type of its symbol
	let declared = session
		.analysis(file)
		.symbols
		.symbols()
		.iter()
		.find(|symbol| symbol.name_span.start as usize == start);
	if let Some(symbol) = declared {
		return (!symbol.ty.is_unset()).then(|| session.display_type(symbol.ty));
	}
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
	let verbose = std::env::var("FA_VERBOSE").is_ok_and(|value| value == "1");

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
			if case.files.len() > 1 || case.files[0].path != "main.fa" {
				run_program(&case.files, case.analysis_target.as_deref())
			} else {
				run_file(
					&case.files[0],
					case.test_files,
					case.analysis_target.as_deref(),
				)
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
			(Some(_), false) => {
				tally.pending += 1;
				if verbose {
					println!("{relative} (pending):\n  {}", problems.join("\n  "));
				}
			}
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
		for path in now_passing
			.iter()
			.take(if verbose { usize::MAX } else { 20 })
		{
			println!("  {path}");
		}
		if !verbose && now_passing.len() > 20 {
			println!("  … (set FA_CASES to list a subset)");
		}
	}
	let strict_failure = strict && !now_passing.is_empty();
	assert!(
		failures.is_empty() && !strict_failure,
		"failing cases:\n{failures}"
	);
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
			(None, "E-TYPE-1"),
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
