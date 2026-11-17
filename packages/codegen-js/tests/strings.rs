//! UTF-16 strings and bytes run unchanged on every JavaScript target.
use std::path::PathBuf;
use std::process::Command;

use fa_codegen_js::{Options, Target, generate};

#[test]
fn strings_and_bytes_agree_on_javascript_targets() {
	let bun = PathBuf::from(std::env::var_os("HOME").unwrap()).join(".bun/bin/bun");
	assert!(bun.is_file(), "Bun is required for these runtime tests");
	let directory = std::env::temp_dir().join(format!("fa-strings-js-{}", std::process::id()));
	std::fs::create_dir_all(&directory).unwrap();
	for target in [Target::Bun, Target::Node, Target::Web] {
		for fixture in [
			"utf16",
			"bytes",
			"replacement",
			"ownership",
			"json",
			"files",
		] {
			if fixture == "files" && target == Target::Web {
				continue;
			}
			let source = std::fs::read_to_string(
				PathBuf::from(env!("CARGO_MANIFEST_DIR"))
					.join(format!("tests/strings/{fixture}.fa")),
			)
			.unwrap();
			let source = source.replace("__TEST_DIRECTORY__", directory.to_str().unwrap());
			let (mut session, file) = fa_analyzer::analyze_source(&source);
			let output = generate(&mut session, file, &Options::new(target, fixture)).unwrap();
			let script = directory.join(format!("{fixture}-{}.mjs", target.name()));
			std::fs::write(&script, &output.code).unwrap();
			let engine = if target != Target::Bun {
				PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/bin/node")
			} else {
				bun.clone()
			};
			assert!(
				engine.is_file(),
				"JavaScript test engine {} is required",
				engine.display()
			);
			let mut command = Command::new(engine);
			if target == Target::Web {
				command.args([
					"--experimental-vm-modules",
					"--disable-warning=ExperimentalWarning",
				]);
				command.arg(
					PathBuf::from(env!("CARGO_MANIFEST_DIR"))
						.join("../cli/tests/programs/web-runner.mjs"),
				);
			}
			let result = command.arg(&script).output().unwrap();
			assert!(
				result.status.success(),
				"{} {fixture}: {}",
				target.name(),
				String::from_utf8_lossy(&result.stderr)
			);
			let expected = source
				.lines()
				.filter_map(|line| line.strip_prefix("-- prints: "))
				.collect::<Vec<_>>()
				.join("\n") + "\n";
			assert_eq!(
				String::from_utf8(result.stdout).unwrap(),
				expected,
				"{} {fixture} ({})",
				target.name(),
				script.display()
			);
		}
	}
	std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn explicit_views_only_link_the_runtime_they_use() {
	for (source, required, absent) in [
		(
			"for byte in \"é😀\".toUtf8() { console.log(byte) }",
			"faUtf8Encoder",
			"faUtf8Values",
		),
		(
			"function count = (text: String) => text.codePoints().length\nconsole.log(count(\"é😀\"))",
			"faCodePointCount",
			"faStringCodePoints",
		),
		(
			"use { Json } = std.json\nlet value: Integer = Json.parse(\"1\") catch error => 0\nconsole.log(value)",
			"faJsonParse",
			"faJsonReadBytes",
		),
	] {
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let output = generate(&mut session, file, &Options::new(Target::Bun, "views.fa")).unwrap();
		assert!(output.helpers.contains(&required), "{required}");
		assert!(!output.helpers.contains(&absent), "{absent}");
	}
}

#[test]
fn constants_preserve_raw_units_and_compact_bytes() {
	let source = include_str!(
		"../../analyzer/tests/cases/values/07-strings-and-characters/constant-code-units.fa"
	);
	for target in [Target::Bun, Target::Node, Target::Web] {
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let output = generate(&mut session, file, &Options::new(target, "constants.fa")).unwrap();
		assert!(output.code.contains(r#""\ud83d""#), "{}", output.code);
		assert!(output.code.contains(r#""\ude00""#), "{}", output.code);
		assert!(output.code.contains("new Uint8Array([195, 169])"));
	}
}

#[test]
fn counts_stay_allocation_free_through_method_chains() {
	let source = include_str!("../../cli/tests/programs/review-code-point-count-array.fa");
	for target in [Target::Bun, Target::Node, Target::Web] {
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let output = generate(&mut session, file, &Options::new(target, "counts.fa")).unwrap();
		assert!(output.helpers.contains(&"faCodePointCount"));
		assert!(
			!output.helpers.contains(&"faStringCodePoints"),
			"{}",
			output.program
		);
	}
}
