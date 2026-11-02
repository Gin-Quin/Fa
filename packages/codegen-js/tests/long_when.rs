//! A `when` of 20,000 arms: an `else if` chain is a nested statement for the
//! JavaScript parsers, which overflow their stack on it ("Maximum call stack
//! size exceeded" in Bun). Long `when`s are written flat: a `switch` for
//! constant patterns, a labeled block of `if`s otherwise. The programs run on
//! Bun when it is installed (skipped with a message otherwise).

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use fa_codegen_js::{Options, Target, generate};

fn find_bun() -> Option<PathBuf> {
	if let Some(home) = std::env::var_os("HOME") {
		let path = PathBuf::from(home).join(".bun/bin/bun");
		if path.is_file() {
			return Some(path);
		}
	}
	let found = Command::new("bun").arg("--version").output();
	found
		.ok()
		.filter(|output| output.status.success())
		.map(|_| PathBuf::from("bun"))
}

/// Compiles a program, checks that its code is flat, and runs it on Bun:
/// what it prints.
fn compile_and_run(name: &str, source: &str, flat_form: &str) -> Option<String> {
	let (mut session, file) = fa_analyzer::analyze_source(source);
	let output = match generate(&mut session, file, &Options::new(Target::Bun, name)) {
		Ok(output) => output,
		Err(error) => panic!("{name} doesn't compile:\n{error}"),
	};
	assert!(
		output.program.contains(flat_form),
		"{name}: no `{flat_form}` in the generated code"
	);
	let deepest = output
		.program
		.lines()
		.map(|line| line.len() - line.trim_start_matches('\t').len())
		.max()
		.unwrap_or(0);
	assert!(
		deepest < 8,
		"{name}: the generated code nests {deepest} levels"
	);
	let Some(bun) = find_bun() else {
		eprintln!("skipped: Bun is not installed");
		return None;
	};
	let directory =
		std::env::temp_dir().join(format!("fa-long-when-{}-{name}", std::process::id()));
	std::fs::create_dir_all(&directory).unwrap();
	let path = directory.join("main.js");
	std::fs::write(&path, &output.code).unwrap();
	let result = Command::new(bun).arg(&path).output().unwrap();
	std::fs::remove_dir_all(&directory).unwrap();
	assert!(
		result.status.success(),
		"{name} failed on Bun:\n{}",
		String::from_utf8_lossy(&result.stderr)
	);
	Some(String::from_utf8_lossy(&result.stdout).trim().to_string())
}

#[test]
fn a_when_of_20000_integer_arms_is_a_switch() {
	let mut source = String::from("function pick = (n: Integer): Integer => when n is {\n");
	for arm in 0..20_000 {
		writeln!(source, "\t{arm} => {}", arm * 3 % 1000).unwrap();
	}
	source.push_str("\telse => -1\n}\nmutable total = 0\nfor i in 0..25000 {\n\ttotal += pick(i)\n}\nconsole.log(total)\n");
	let expected: i64 = (0..20_000i64).map(|arm| arm * 3 % 1000).sum::<i64>() - 5000;
	if let Some(printed) = compile_and_run("integers", &source, "switch (") {
		assert_eq!(printed, expected.to_string());
	}
}

#[test]
fn a_when_of_20000_arms_with_ranges_and_breaks_is_a_labeled_block() {
	let mut source = String::from(
		"function pick = (n: Integer): Integer => {\n\tmutable result = 0\n\tloop {\n\t\twhen n is {\n",
	);
	for arm in 0..20_000 {
		writeln!(
			source,
			"\t\t\t{} => {{ result = {arm}\n\t\t\t\tbreak }}",
			arm * 2
		)
		.unwrap();
	}
	source.push_str("\t\t\t40000..50000 => { result = 7\n\t\t\t\tbreak }\n\t\t\telse => { result = -1\n\t\t\t\tbreak }\n\t\t}\n\t}\n\tresult\n}\nmutable total = 0\nfor i in 0..45000 {\n\ttotal += pick(i)\n}\nconsole.log(total)\n");
	let expected: i64 = (0..20_000i64).sum::<i64>() - 20_000 + 5000 * 7;
	if let Some(printed) = compile_and_run("ranges", &source, "break $t") {
		assert_eq!(printed, expected.to_string());
	}
}
