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
			let engine = if target == Target::Node {
				PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/bin/node")
			} else {
				bun.clone()
			};
			assert!(
				engine.is_file(),
				"JavaScript test engine {} is required",
				engine.display()
			);
			let result = Command::new(engine).arg(&script).output().unwrap();
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
