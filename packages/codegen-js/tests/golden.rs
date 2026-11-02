//! Golden tests of the generated JavaScript: each `tests/golden/*.fa` is
//! compiled for Bun and compared with the `.js` file next to it, which holds
//! the list of runtime helpers used, then the program without them.
//!
//! `FA_UPDATE_GOLDEN=1 cargo test -p fa-codegen-js --test golden` rewrites
//! the `.js` files; review their diff like code.

use std::fs;
use std::path::Path;

use fa_codegen_js::{Options, Target, generate};

/// The snapshot of a program: its helpers, then its code.
fn snapshot(source: &str, name: &str) -> String {
	let (mut session, file) = fa_analyzer::analyze_source(source);
	let output = match generate(&mut session, file, &Options::new(Target::Bun, name)) {
		Ok(output) => output,
		Err(error) => panic!("{name} doesn't compile:\n{error}"),
	};
	format!(
		"// helpers: {}\n\n{}",
		output.helpers.join(", "),
		output.program
	)
}

#[test]
fn generated_code_matches_the_golden_files() {
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
	let update = std::env::var("FA_UPDATE_GOLDEN").is_ok_and(|value| value == "1");
	let mut sources: Vec<_> = fs::read_dir(&directory)
		.unwrap()
		.map(|entry| entry.unwrap().path())
		.filter(|path| path.extension().is_some_and(|extension| extension == "fa"))
		.collect();
	sources.sort();
	assert!(!sources.is_empty(), "no golden program");
	let mut differences = Vec::new();
	for source in &sources {
		let name = source.file_name().unwrap().to_string_lossy().to_string();
		let actual = snapshot(&fs::read_to_string(source).unwrap(), &name);
		let golden = source.with_extension("js");
		if update {
			fs::write(&golden, &actual).unwrap();
			continue;
		}
		let expected = fs::read_to_string(&golden).unwrap_or_default();
		if expected != actual {
			differences.push(format!(
				"{name} differs from {} (FA_UPDATE_GOLDEN=1 rewrites it):\n{actual}",
				golden.display()
			));
		}
	}
	assert!(differences.is_empty(), "{}", differences.join("\n\n"));
}
