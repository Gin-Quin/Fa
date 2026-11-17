//! Every helper, collected alone with its dependencies, evaluates in Bun:
//! the chunk is valid JavaScript, its classes are declared before the classes
//! extending them, and it defines the helper. Skipped (with a message) when
//! Bun isn't installed.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use fa_codegen_js::runtime::{collect_helpers, collect_helpers_inline, helpers};

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

fn run_with_bun(name: &str, script: &str) {
	let Some(bun) = find_bun() else {
		eprintln!("skipped: Bun is not installed");
		return;
	};
	let directory =
		std::env::temp_dir().join(format!("fa-codegen-js-{}-{name}", std::process::id()));
	std::fs::create_dir_all(&directory).unwrap();
	let file = directory.join("check.mjs");
	std::fs::write(&file, script).unwrap();
	let output = Command::new(bun).arg(&file).output().unwrap();
	std::fs::remove_dir_all(&directory).unwrap();
	assert!(
		output.status.success(),
		"Bun failed:\n{}\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
}

#[test]
fn every_helper_evaluates_alone() {
	let mut script = String::new();
	for helper in helpers() {
		let name = helper.name;
		let chunk = collect_helpers_inline([name]);
		writeln!(
			script,
			"{{\n{chunk}\nif (typeof {name} === \"undefined\") throw new Error(\"`{name}` is not defined\");\n}}"
		)
		.unwrap();
	}
	script.push_str("console.log(\"ok\");\n");
	run_with_bun("alone", &script);
}

#[test]
fn a_collected_module_runs_a_program() {
	let mut script = collect_helpers([
		"faRun",
		"faFormat",
		"faFloorDivide",
		"faCodePointCount",
		"faCaught",
		"faFormatMember",
	]);
	script.push_str(
		r#"
let caught;
try {
	faFloorDivide(1, 0);
} catch ($e) {
	caught = faCaught($e);
}
if (String(caught) !== "DivisionByZero") throw new Error(String(caught));
if (faFloorDivide(-7, 2) !== -4) throw new Error("floor division");
if (faCodePointCount("🇫🇷é") !== 4) throw new Error("code points");
if (faFormatMember("Shape.Point") !== "Shape.Point") throw new Error("member");
faRun(() => console.log(faFormat("ok")));
"#,
	);
	run_with_bun("module", &script);
}
