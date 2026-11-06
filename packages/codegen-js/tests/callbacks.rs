//! Additional callback errors cross the receiving function's catches;
//! errors raised while evaluating its arguments remain interceptable.

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
	Command::new("bun")
		.arg("--version")
		.output()
		.ok()
		.filter(|output| output.status.success())
		.map(|_| PathBuf::from("bun"))
}

#[test]
fn callback_errors_bypass_catches_in_blocking_and_async_functions() {
	let source = include_str!("../../analyzer/tests/cases/review/callback-errors-bypass-catch.fa");
	let Some(bun) = find_bun() else {
		eprintln!("skipped: Bun is not installed");
		return;
	};
	for asynchronous in [false, true] {
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let mut options = Options::new(Target::Bun, "callbacks.fa");
		options.asynchronous = Some(asynchronous);
		let output = generate(&mut session, file, &options).unwrap();
		let directory = std::env::temp_dir().join(format!(
			"fa-callback-errors-{}-{asynchronous}",
			std::process::id()
		));
		std::fs::create_dir_all(&directory).unwrap();
		let path = directory.join("main.js");
		std::fs::write(&path, &output.code).unwrap();
		let result = Command::new(&bun).arg(&path).output().unwrap();
		std::fs::remove_dir_all(&directory).unwrap();
		assert!(
			result.status.success(),
			"asynchronous={asynchronous}: {}",
			String::from_utf8_lossy(&result.stderr)
		);
		assert_eq!(
			String::from_utf8(result.stdout).unwrap().trim(),
			"7 3 99\n17 99\n3 99\n5 99\n99",
			"asynchronous={asynchronous}"
		);
	}
}
