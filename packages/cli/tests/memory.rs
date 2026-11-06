//! The memory of `fa check`: checking a program must stay below 512 MB.
//!
//! Each test runs `fa check` with its address space limited to 512 MB
//! (`ulimit -v`, which bounds the resident memory too): a check that needs
//! more fails to allocate, and aborts. The checks must end with their
//! diagnostics instead (exit code 0 or 1). Unix only.
//!
//! The large inputs are ignored by default:
//!
//! ```sh
//! cargo test -p fa --test memory -- --include-ignored
//! ```

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The memory budget of a check, in kB.
const BUDGET_KB: u64 = 512 * 1024;

/// A file in a fresh folder of the test's temporary directory.
fn write(name: &str, text: &str) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-memory");
	fs::create_dir_all(&directory).unwrap();
	let path = directory.join(name);
	fs::write(&path, text).unwrap();
	path
}

/// Runs `fa check` on a file within the memory budget: its exit code and output.
fn check_within_budget(path: &Path) -> (i32, String) {
	let output = Command::new("sh")
		.arg("-c")
		.arg(format!("ulimit -v {BUDGET_KB} && exec \"$0\" check \"$1\""))
		.arg(env!("CARGO_BIN_EXE_fa"))
		.arg(path)
		.output()
		.expect("`sh` runs");
	let text = format!(
		"{}{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	(output.status.code().unwrap_or(-1), text)
}

/// Asserts that `fa check` ends normally within the budget; its output.
fn assert_checks_within_budget(name: &str, text: &str) -> String {
	let path = write(name, text);
	let (code, output) = check_within_budget(&path);
	assert!(
		code == 0 || code == 1,
		"`fa check {name}` failed within {} MB (exit {code}):\n{}",
		BUDGET_KB / 1024,
		output.chars().take(2000).collect::<String>()
	);
	output
}

/// About `lines` lines of typical code: the benchmark programs, each in its
/// own scope.
fn typical_code(lines: usize) -> String {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks");
	let mut names: Vec<_> = fs::read_dir(&root)
		.map(|entries| entries.flatten().map(|entry| entry.path()).collect())
		.unwrap_or_default();
	names.sort();
	let mut programs = String::new();
	for directory in names {
		if let Ok(text) = fs::read_to_string(directory.join("main.fa")) {
			programs += &format!("do {{\n{text}\n}}\n");
		}
	}
	let mut text = String::new();
	while text.lines().count() < lines {
		text += &programs;
	}
	text
}

/// `count` blocks of straight-line declarations, conditions and calls (5 lines each).
fn flat(count: usize) -> String {
	(0..count)
		.map(|k| {
			format!(
				"let a{k} = {} + 3\n\
				 let b{k}: Integer = a{k} * 2 - 1\n\
				 let c{k} = if b{k} > 4 {{ b{k} }} else {{ 0 }}\n\
				 let d{k} = 1 < c{k} and c{k} <= 3\n\
				 console.log(a{k}, d{k})\n",
				k % 10
			)
		})
		.collect()
}

/// A static value building an array of `count` integers.
fn static_array(count: usize) -> String {
	format!(
		"function build = (): Array<Integer> => {{\n\
		 \tmutable items: Array<Integer> = []\n\
		 \tfor index in 0..{count} {{\n\
		 \t\titems.push(index)\n\
		 \t}}\n\
		 \titems\n\
		 }}\n\
		 static big = @build()\n\
		 console.log(big.length)\n"
	)
}

#[test]
fn a_typical_file_of_5000_lines_is_checked_within_512_mb() {
	assert_checks_within_budget("typical.fa", &typical_code(5000));
}

#[test]
fn compile_time_evaluation_runs_within_512_mb() {
	// the evaluation thread's stack is smaller when the address space is limited
	let statics = fs::read_to_string(
		Path::new(env!("CARGO_MANIFEST_DIR")).join("../analyzer/tests/cases/comptime/statics.fa"),
	)
	.unwrap();
	let output = assert_checks_within_budget("statics.fa", &statics);
	assert!(!output.contains("error"), "{output}");
	// an array of 100 000 items: built in place, typed without a type per item
	let output = assert_checks_within_budget("array.fa", &static_array(100_000));
	assert!(!output.contains("error"), "{output}");
	// a text doubled until it doesn't fit: an error, not an abort
	let doubling = "function build = (): String => {\n\
		\tmutable text = \"ab\"\n\
		\tfor index in 0..40 {\n\
		\t\ttext += text\n\
		\t}\n\
		\ttext\n\
		}\n\
		static big = @build()\n";
	let output = assert_checks_within_budget("doubling.fa", doubling);
	assert!(output.contains("META003"), "{output}");
	// a recursion 1 000 calls deep fits in the smaller stack
	let recursion = "function down = (n: Integer): Integer => if n == 0 { 0 } else { 1 + down(n - 1) }\n\
		static depth = @down(1000)\n";
	let output = assert_checks_within_budget("recursion.fa", recursion);
	assert!(!output.contains("error"), "{output}");
	// a loop over a range of 20 million items doesn't build the items
	let range = "function count = (): Integer => {\n\
		\tmutable total = 0\n\
		\tfor index in 0..20_000_000 {\n\
		\t\tif index == 3 { return 1 }\n\
		\t}\n\
		\ttotal\n\
		}\n\
		static counted = @count()\n";
	let output = assert_checks_within_budget("range.fa", range);
	assert!(!output.contains("error"), "{output}");
}

#[test]
#[ignore]
fn a_file_of_50000_lines_is_checked_within_512_mb() {
	assert_checks_within_budget("typical-50k.fa", &typical_code(50_000));
	assert_checks_within_budget("flat-50k.fa", &flat(10_000));
}

#[test]
#[ignore]
fn a_static_array_of_millions_of_items_is_an_error_within_512_mb() {
	let output = assert_checks_within_budget("millions.fa", &static_array(5_000_000));
	assert!(output.contains("META003"), "{output}");
}
