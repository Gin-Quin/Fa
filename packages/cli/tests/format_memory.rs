//! `fa format --check` stays below 512 MB of memory, on the whole repository and on generated
//! code of every shape (deep nesting, long lines, long lists, many comments, a 5 MB file):
//! each run is limited to 512 MB of address space (`ulimit -v`), and fails if it needs more.
//!
//! The formatter once copied the arguments of a call into both of its layouts, which doubled
//! the memory at each level of `f({ a = g({ … }) })`: 20 levels took gigabytes, and killed the
//! machine running the tests.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The memory every check of validity must stay below (`fa format --check`, `fa check`, the
/// language server).
const LIMIT_KB: usize = 512 * 1024;

/// Runs `fa format --check <path>` limited to `LIMIT_KB` of address space, and checks that it
/// ends normally (0: formatted, 1: not formatted) within `seconds`.
fn assert_checks_within_limit(path: &Path, seconds: u64) {
	let started_at = Instant::now();
	let output = Command::new("sh")
		.arg("-c")
		.arg(format!(
			"ulimit -v {LIMIT_KB} && exec \"$0\" format --check \"$1\""
		))
		.arg(env!("CARGO_BIN_EXE_fa"))
		.arg(path)
		.output()
		.expect("`sh` runs");
	let elapsed = started_at.elapsed();
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		matches!(output.status.code(), Some(0 | 1)),
		"`fa format --check {}` failed within {LIMIT_KB} KB ({:?}):\n{}",
		path.display(),
		output.status,
		stderr.lines().rev().take(5).collect::<Vec<_>>().join("\n")
	);
	assert!(
		!stderr.contains("memory allocation"),
		"`fa format --check {}` ran out of memory:\n{stderr}",
		path.display()
	);
	assert!(
		elapsed <= Duration::from_secs(seconds),
		"`fa format --check {}` took {elapsed:?}",
		path.display()
	);
}

/// A scratch folder of this test binary.
fn scratch(name: &str) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	let _ = fs::remove_dir_all(&directory);
	fs::create_dir_all(&directory).unwrap();
	directory
}

#[test]
fn checking_the_repository_stays_below_512_mb() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
	assert_checks_within_limit(&root, 120);
}

/// `depth` levels of `open` … `close` around `x`.
fn nested(open: &str, close: &str, depth: usize) -> String {
	format!("let a = {}x{}\n", open.repeat(depth), close.repeat(depth))
}

#[test]
fn checking_generated_code_stays_below_512_mb() {
	let directory = scratch("format-memory");
	let names: Vec<String> = (0..20_000).map(|index| format!("x{index}")).collect();
	let numbers: Vec<String> = (0..10_000).map(|index| index.to_string()).collect();
	let mut files = vec![
		// as deep as the parser accepts
		("nested-objects.fa", nested("f({ a = ", " })", 40)),
		("nested-lists.fa", nested("f([", "])", 40)),
		("nested-lambdas.fa", nested("f((x) => { g(", ") })", 40)),
		("nested-calls.fa", nested("f(1, ", ")", 40)),
		("long-line.fa", format!("let a = f({})\n", names.join(", "))),
		("additions.fa", format!("let a = {}\n", names.join(" + "))),
		("list.fa", format!("let a = [{}]\n", numbers.join(", "))),
		(
			"comments.fa",
			(0..50_000)
				.map(|index| {
					format!("let a{index} = {index} -- comment {index}\n-- line {index}\n")
				})
				.collect(),
		),
	];
	let mut large = String::new();
	let mut index = 0;
	while large.len() < 5_000_000 {
		large.push_str(&format!(
			"let a{index} = {index} + 3 -- the first\n\
			type Point{index} = {{\n\tx: Integer\n\ty: Integer\n}}\n\
			function f{index} = (point: Point{index}): Integer => {{\n\
			\tlet values = [point.x, point.y, {index}].map((value) => value * 2)\n\
			\tif values.length > 2 {{ values[0] }} else {{ 0 }}\n}}\n\
			console.log(f{index}({{ x = 1, y = 2 }}), \"done {index}\")\n\n"
		));
		index += 1;
	}
	files.push(("large.fa", large));

	for (name, source) in files {
		let path = directory.join(name);
		fs::write(&path, source).unwrap();
		assert_checks_within_limit(&path, 60);
	}
}
