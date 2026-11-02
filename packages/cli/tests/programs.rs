//! End-to-end tests: Fa programs run through `fa run` (the JavaScript
//! backend, executed by Bun), their output compared with their annotations.
//!
//! Two sets of programs:
//!
//! - `tests/programs/*.fa` (this crate): the programs of the JavaScript
//!   backend;
//! - the cases of the analyzer (`packages/analyzer/tests/cases/**/*.fa`) that
//!   check without errors and say what they print, so that every brief test
//!   with an expected output runs once the analyzer and the backend support it.
//!
//! Annotations are Fa comments, on their own line or after code:
//!
//! ```fa
//! console.log(1 + 2)        -- prints: 3
//! -- prints: a second line
//! -- prints: "quoted"       (the brief's notation of a printed string: `quoted`)
//! -- prints: "line" ×3      (the line, three times; `<TAB>` is a tab)
//! -- exit: 101              (the exit code; 0 by default)
//! -- stderr: integer overflow    (a text that stderr contains)
//! -- args: first second      (the program's arguments; or `-- arguments:`)
//! -- overflow-checks: false  (`fa run --overflow-checks=false`)
//! -- skip-target: native wasm (targets the program doesn't run on: `bun` skips it here)
//! ```
//!
//! The same annotations as the LLVM backend's end-to-end tests
//! (`packages/codegen-llvm/tests/programs.rs`), which also run these
//! programs on `native` and `wasm`.
//!
//! Bun is found on the `PATH` or in `~/.bun/bin`; without it, the tests
//! print a message and pass. `FA_PROGRAMS=filter` runs only the programs
//! whose path contains `filter`; `FA_VERBOSE=1` lists every result.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// How many programs a test runs at the same time (each one is a Bun process).
const WORKERS: usize = 3;

/// How many programs run at the same time, all tests together (they run in
/// parallel): their memory adds up.
const RUNNING_PROGRAMS: usize = 6;

/// The number of programs running, and its change.
static RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

/// Runs `f` once fewer than [`RUNNING_PROGRAMS`] others are running.
fn limited<R>(f: impl FnOnce() -> R) -> R {
	let (running, changed) = &RUNNING;
	let mut count = running.lock().unwrap_or_else(|poison| poison.into_inner());
	while *count >= RUNNING_PROGRAMS {
		count = changed
			.wait(count)
			.unwrap_or_else(|poison| poison.into_inner());
	}
	*count += 1;
	drop(count);
	let result = f();
	*running.lock().unwrap_or_else(|poison| poison.into_inner()) -= 1;
	changed.notify_one();
	result
}

/// The longest a program may run.
const TIMEOUT: Duration = Duration::from_secs(20);

/// What a program is expected to do.
#[derive(Debug, Default, Clone)]
struct Expected {
	stdout: Vec<String>,
	exit: i32,
	stderr: Vec<String>,
	args: Vec<String>,
	/// Options of `fa run`, given before the file (`-- overflow-checks:`).
	options: Vec<String>,
	/// `-- skip-target: … bun …`: not run by this harness.
	skipped: bool,
	/// Run as `fa build --minify=true` then `bun main.js`, instead of `fa run`.
	minified: bool,
	/// Whether the file says anything about its output.
	annotated: bool,
	/// `-- pending: reason`: a construct of the program isn't supported yet
	/// (by the analyzer or the backend); its run isn't a failure.
	pending: Option<String>,
}

/// The start of the comment of a line (`--` outside a string).
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

/// The annotations of a program.
fn expectations(text: &str) -> Expected {
	let mut expected = Expected::default();
	for line in text.lines() {
		let Some(start) = find_comment(line) else {
			continue;
		};
		let comment = line[start + 2..].trim_start();
		if let Some(printed) = comment.strip_prefix("prints:") {
			let printed = printed.strip_prefix(' ').unwrap_or(printed).trim_end();
			printed_lines(&printed.replace("<TAB>", "\t"), &mut expected.stdout);
			expected.annotated = true;
		} else if let Some(code) = comment.strip_prefix("exit:") {
			expected.exit = code.trim().parse().unwrap_or(-1);
			expected.annotated = true;
		} else if let Some(text) = comment.strip_prefix("stderr:") {
			expected.stderr.push(text.trim().to_string());
		} else if let Some(args) = comment
			.strip_prefix("args:")
			.or_else(|| comment.strip_prefix("arguments:"))
		{
			expected.args = args.split_whitespace().map(str::to_string).collect();
		} else if let Some(checks) = comment.strip_prefix("overflow-checks:") {
			expected
				.options
				.push(format!("--overflow-checks={}", checks.trim()));
		} else if let Some(targets) = comment.strip_prefix("skip-target:") {
			expected.skipped |= targets.split_whitespace().any(|target| target == "bun");
		} else if let Some(reason) = comment.strip_prefix("pending:") {
			expected.pending = Some(reason.trim().to_string());
		}
	}
	expected
}

/// The lines of a `prints:` annotation: `"text"` is `text`, and
/// `"text" ×3` is three lines `text` (the notation of the briefs).
fn printed_lines(printed: &str, lines: &mut Vec<String>) {
	let repeated = printed.rsplit_once(" ×").and_then(|(line, count)| {
		let count: usize = count.parse().ok()?;
		let line = line.strip_prefix('"')?.strip_suffix('"')?;
		Some((line, count))
	});
	if let Some((line, count)) = repeated {
		lines.extend(std::iter::repeat_n(line.to_string(), count));
		return;
	}
	match printed
		.strip_prefix('"')
		.and_then(|line| line.strip_suffix('"'))
	{
		Some(line) if !line.contains('"') => lines.push(line.to_string()),
		_ => lines.push(printed.to_string()),
	}
}

/// What a run gave.
#[derive(Debug)]
struct Outcome {
	stdout: String,
	stderr: String,
	exit: i32,
}

/// Runs `fa run <options> <program> <args>` (or the minified build of the
/// program) with a timeout.
fn run(program: &Path, expected: &Expected, scratch: &Path, index: usize) -> Outcome {
	let stdout_path = scratch.join(format!("{index}.stdout"));
	let stderr_path = scratch.join(format!("{index}.stderr"));
	let mut command = if expected.minified {
		let directory = scratch.join(format!("{index}-build"));
		let built = Command::new(env!("CARGO_BIN_EXE_fa"))
			.args(["build", "--minify=true", "--overflow-checks=true"])
			.args(&expected.options)
			.arg(format!("--output={}", directory.display()))
			.arg(program)
			.output()
			.expect("`fa` runs");
		if !built.status.success() {
			return Outcome {
				stdout: String::new(),
				stderr: String::from_utf8_lossy(&built.stderr).into_owned(),
				exit: -3,
			};
		}
		// `--output=<directory>` writes into `<directory>/<target>/`
		let mut command = Command::new(fa::find_bun().expect("Bun"));
		command.arg(directory.join("bun").join("main.js"));
		command
	} else {
		let mut command = Command::new(env!("CARGO_BIN_EXE_fa"));
		command.arg("run").args(&expected.options).arg(program);
		command
	};
	let mut child = command
		.args(&expected.args)
		.env("XDG_CACHE_HOME", scratch.join("cache"))
		.stdin(Stdio::null())
		.stdout(fs::File::create(&stdout_path).unwrap())
		.stderr(fs::File::create(&stderr_path).unwrap())
		.spawn()
		.expect("`fa` runs");
	let started = Instant::now();
	let exit = loop {
		if let Some(status) = child.try_wait().unwrap() {
			break status.code().unwrap_or(-1);
		}
		if started.elapsed() > TIMEOUT {
			let _ = child.kill();
			let _ = child.wait();
			break -2;
		}
		std::thread::sleep(Duration::from_millis(5));
	};
	Outcome {
		stdout: fs::read_to_string(&stdout_path).unwrap_or_default(),
		stderr: fs::read_to_string(&stderr_path).unwrap_or_default(),
		exit,
	}
}

/// Why a program's run differs from its annotations, if it does.
fn compare(expected: &Expected, outcome: &Outcome) -> Option<String> {
	let printed: Vec<&str> = outcome.stdout.lines().collect();
	let mut problems = Vec::new();
	if printed != expected.stdout {
		problems.push(format!(
			"stdout: expected {:?}, got {:?}",
			expected.stdout, printed
		));
	}
	if outcome.exit != expected.exit {
		problems.push(format!(
			"exit code: expected {}, got {}",
			expected.exit, outcome.exit
		));
	}
	for text in &expected.stderr {
		if !outcome.stderr.contains(text.as_str()) {
			problems.push(format!("stderr doesn't contain {text:?}"));
		}
	}
	if problems.is_empty() {
		None
	} else {
		if !outcome.stderr.is_empty() {
			problems.push(format!("stderr: {}", outcome.stderr.trim_end()));
		}
		Some(problems.join("\n    "))
	}
}

/// Runs programs on a few workers; returns `(path, expected, outcome)` in
/// the order of `programs`.
fn run_all(
	programs: Vec<(PathBuf, Expected)>,
	scratch: &Path,
) -> Vec<(PathBuf, Expected, Outcome)> {
	fs::create_dir_all(scratch).unwrap();
	let queue = Mutex::new(programs.into_iter().enumerate());
	let results = Mutex::new(Vec::new());
	std::thread::scope(|scope| {
		for _ in 0..WORKERS {
			scope.spawn(|| {
				loop {
					let next = queue.lock().unwrap().next();
					let Some((index, (path, expected))) = next else {
						break;
					};
					let outcome = limited(|| run(&path, &expected, scratch, index));
					results
						.lock()
						.unwrap()
						.push((index, path, expected, outcome));
				}
			});
		}
	});
	let mut results = results.into_inner().unwrap();
	results.sort_by_key(|(index, ..)| *index);
	results
		.into_iter()
		.map(|(_, path, expected, outcome)| (path, expected, outcome))
		.collect()
}

/// The `.fa` files under a directory, recursively, sorted.
fn fa_files(directory: &Path) -> Vec<PathBuf> {
	let mut files = Vec::new();
	let mut pending = vec![directory.to_path_buf()];
	while let Some(directory) = pending.pop() {
		let Ok(entries) = fs::read_dir(&directory) else {
			continue;
		};
		for entry in entries {
			let path = entry.unwrap().path();
			if path.is_dir() {
				pending.push(path);
			} else if path.extension().is_some_and(|extension| extension == "fa") {
				files.push(path);
			}
		}
	}
	files.sort();
	files
}

/// Whether the programs are filtered in, by `FA_PROGRAMS`.
fn selected(path: &Path) -> bool {
	match std::env::var("FA_PROGRAMS") {
		Ok(filter) if !filter.is_empty() => path.to_string_lossy().contains(&filter),
		_ => true,
	}
}

fn verbose() -> bool {
	std::env::var("FA_VERBOSE").is_ok_and(|value| !value.is_empty() && value != "0")
}

/// Whether Bun is available (otherwise the tests are skipped).
fn has_bun() -> bool {
	if fa::find_bun().is_some() {
		return true;
	}
	eprintln!("Bun is not installed (PATH or ~/.bun/bin): the end-to-end tests are skipped");
	false
}

#[test]
fn programs_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
	let programs: Vec<(PathBuf, Expected)> = fa_files(&directory)
		.into_iter()
		.filter(|path| selected(path))
		.map(|path| {
			let text = fs::read_to_string(&path).unwrap();
			let expected = expectations(&text);
			assert!(
				expected.annotated,
				"{} has no `-- prints:` nor `-- exit:` annotation",
				path.display()
			);
			(path, expected)
		})
		.filter(|(_, expected)| !expected.skipped)
		.collect();
	let count = programs.len();
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-programs");
	let mut failures = Vec::new();
	let mut pending = 0;
	for (path, expected, outcome) in run_all(programs, &scratch) {
		let name = path.strip_prefix(&directory).unwrap_or(&path).display();
		match (compare(&expected, &outcome), &expected.pending) {
			(None, None) if verbose() => eprintln!("ok   {name}"),
			(None, None) => {}
			(None, Some(reason)) => {
				eprintln!("{name} is pending ({reason}) but passes: remove its `-- pending:`")
			}
			(Some(problem), Some(reason)) => {
				pending += 1;
				if verbose() {
					eprintln!("pending {name} ({reason}):\n    {problem}");
				}
			}
			(Some(problem), None) => failures.push(format!("{name}:\n    {problem}")),
		}
	}
	eprintln!(
		"{} of {count} programs passed ({pending} pending)",
		count - failures.len() - pending
	);
	assert!(
		failures.is_empty(),
		"{} programs failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

/// The programs also run minified (`fa build --minify=true`): printing, errors,
/// stops, unions and names are the parts a minifier could break.
const MINIFIED: &[&str] = &[
	"errors.fa",
	"javascript-names.fa",
	"named-arguments.fa",
	"printing.fa",
	"stack-overflow.fa",
	"unions-and-enums.fa",
	"value-semantics.fa",
];

#[test]
fn minified_programs_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
	let programs: Vec<(PathBuf, Expected)> = MINIFIED
		.iter()
		.map(|name| directory.join(name))
		.filter(|path| selected(path))
		.map(|path| {
			let mut expected = expectations(&fs::read_to_string(&path).unwrap());
			expected.minified = true;
			(path, expected)
		})
		.collect();
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-minified");
	let failures: Vec<String> = run_all(programs, &scratch)
		.into_iter()
		.filter_map(|(path, expected, outcome)| {
			let problem = compare(&expected, &outcome)?;
			Some(format!("{}:\n    {problem}", path.display()))
		})
		.collect();
	assert!(
		failures.is_empty(),
		"{} minified programs failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

/// Analyzer cases whose run is known to differ, because of a gap of the
/// analyzer or of the backend: `(file name, reason)`. They are run and
/// reported, not counted as failures; one that passes is reported too.
const ANALYZER_GAPS: &[(&str, &str)] = &[];

/// The analyzer cases that the analyzer checks but the JavaScript backend
/// doesn't generate, or doesn't run as annotated, yet (`fa_analyzer::analysis`
/// documents how to lower what the analyzer types). A listed case that runs
/// as annotated is reported, to be removed from the list.
const BACKEND_GAPS: &[&str] = &[
	// `start` and `await`: step J1 of design/async.md (CONC011 until then)
	"types/3.7-special-types-never-none-void-error/sp-6.fa",
];

/// The analyzer cases that describe a run: valid (no expected diagnostic),
/// not pending, one file, not a test file, with a `prints:` or `exit:`.
fn runnable_case(text: &str) -> bool {
	let mut annotated = false;
	for line in text.lines() {
		let Some(start) = find_comment(line) else {
			continue;
		};
		let comment = line[start + 2..].trim_start();
		let key = comment.split(':').next().unwrap_or("").trim();
		match key {
			"pending" | "file" | "kind" | "error" => return false,
			"prints" | "exit" => annotated = true,
			_ => {}
		}
	}
	annotated
}

#[test]
fn analyzer_cases_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../analyzer/tests/cases");
	let mut not_checked = Vec::new();
	let mut not_generated = Vec::new();
	let mut native_only = 0;
	let mut backend_gaps = 0;
	let mut programs = Vec::new();
	for path in fa_files(&directory) {
		if !selected(&path) {
			continue;
		}
		let text = fs::read_to_string(&path).unwrap();
		if !runnable_case(&text) {
			continue;
		}
		let (mut session, file) = fa_analyzer::analyze_source(&text);
		let errors = session
			.diagnostics(file)
			.iter()
			.any(|diagnostic| diagnostic.severity == fa_analyzer::Severity::Error);
		if errors {
			not_checked.push(path);
			continue;
		}
		let options = fa_codegen_js::Options::new(fa_codegen_js::Target::Bun, "main.fa");
		let relative = path
			.strip_prefix(&directory)
			.unwrap_or(&path)
			.to_string_lossy()
			.replace('\\', "/");
		let backend_gap = BACKEND_GAPS.contains(&relative.as_str());
		let generated = fa_codegen_js::generate(&mut session, file, &options);
		if backend_gap && generated.is_err() {
			backend_gaps += 1;
			continue;
		}
		if let Err(error) = generated {
			// a value certainly beyond ±(2^53 − 1): a native-only case (values brief D-9)
			if error
				.diagnostics
				.iter()
				.all(|diagnostic| diagnostic.code.id == "E-NUM-19")
			{
				native_only += 1;
				continue;
			}
			not_generated.push(format!(
				"{}: {}",
				path.strip_prefix(&directory).unwrap_or(&path).display(),
				error.to_string().trim_end()
			));
			continue;
		}
		let expected = expectations(&text);
		if !expected.skipped {
			programs.push((path, expected));
		}
	}
	let count = programs.len();
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-analyzer-cases");
	let mut failures = Vec::new();
	let mut gaps = 0;
	let mut ran_backend_gaps = 0;
	for (path, expected, outcome) in run_all(programs, &scratch) {
		let name = path.strip_prefix(&directory).unwrap_or(&path).display();
		let file = path.file_name().unwrap_or_default().to_string_lossy();
		let gap = ANALYZER_GAPS
			.iter()
			.find(|(gap, _)| *gap == file)
			.map(|(_, reason)| reason);
		let relative = name.to_string().replace('\\', "/");
		if BACKEND_GAPS.contains(&relative.as_str()) {
			match compare(&expected, &outcome) {
				None => eprintln!("{relative} passes: remove it from `BACKEND_GAPS`"),
				Some(_) => ran_backend_gaps += 1,
			}
			continue;
		}
		match (compare(&expected, &outcome), gap) {
			(None, None) if verbose() => eprintln!("ok   {name}"),
			(None, None) => {}
			(None, Some(_)) => eprintln!("{name} passes: remove it from `ANALYZER_GAPS`"),
			(Some(_), Some(reason)) => {
				gaps += 1;
				eprintln!("known analyzer gap, {name}: {reason}");
			}
			(Some(problem), None) => failures.push(format!("{name}:\n    {problem}")),
		}
	}
	eprintln!(
		"analyzer cases: {} of {count} run as annotated ({gaps} known analyzer gaps, {ran_backend_gaps} known backend gaps); {} don't check yet; {} not generated ({backend_gaps} known backend gaps); {native_only} native only (E-NUM-19)",
		count - failures.len() - gaps - ran_backend_gaps,
		not_checked.len(),
		not_generated.len()
	);
	if verbose() {
		for line in &not_generated {
			eprintln!("not generated: {line}");
		}
	}
	assert!(
		not_generated.is_empty(),
		"the JavaScript backend refused {} valid cases:\n{}",
		not_generated.len(),
		not_generated.join("\n")
	);
	assert!(
		failures.is_empty(),
		"{} cases failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}
