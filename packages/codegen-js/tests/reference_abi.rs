use std::path::PathBuf;
use std::process::Command;

use fa_analyzer::{CodebaseResolver, Session, Severity};
use fa_codegen_js::{EntryOptions, Options, Target, generate, generate_program};

#[test]
fn fixed_result_keys_select_container_entries() {
	let source =
		include_str!("../../analyzer/tests/cases/references/review2-static-locator-keys.fa");
	let (mut session, file) = fa_analyzer::analyze_source(source);
	let errors: Vec<_> = session
		.diagnostics(file)
		.iter()
		.filter(|diagnostic| diagnostic.severity == Severity::Error)
		.collect();
	assert!(errors.is_empty(), "{errors:?}");
	let output = generate(&mut session, file, &Options::new(Target::Bun, "main.fa")).unwrap();
	for entry in ["fixedIndex$index", "fixedEntry$entry", "fixedWide$entry"] {
		let body = output
			.program
			.split(&format!("function {entry}("))
			.nth(1)
			.unwrap_or_else(|| panic!("missing {entry}: {}", output.program));
		let body = body.split("\n}").next().unwrap();
		assert!(!body.contains("faReferenceResult("), "{entry}: {body}");
	}
	assert!(!output.program.contains("function negativeIndex$index("));
}

#[test]
fn async_reference_exports_initialize_their_callable_families() {
	let bun = std::env::var_os("HOME")
		.map(|home| PathBuf::from(home).join(".bun/bin/bun"))
		.filter(|path| path.is_file())
		.unwrap_or_else(|| PathBuf::from("bun"));
	if Command::new(&bun).arg("--version").output().is_err() {
		eprintln!("skipped: Bun is not installed");
		return;
	}
	let source =
		include_str!("../../analyzer/tests/cases/references/review2-async-reference-imports.fa");
	let mut files = vec![(
		"package.fa".to_string(),
		"export = { name = \"references\" }\n".to_string(),
	)];
	for section in source.split("-- file: ").skip(1) {
		let (path, text) = section.split_once('\n').unwrap();
		files.push((path.trim().to_string(), text.to_string()));
	}
	for target in [Target::Bun, Target::Node, Target::Web] {
		for asynchronous in [false, true] {
			let mut session = Session::new();
			session.set_target(target.name());
			session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
			let program = session.analyze_package(Some("application/main"));
			let options = Options {
				asynchronous: Some(asynchronous),
				..Options::new(target, "references")
			};
			let output =
				generate_program(&mut session, &program, &options, &EntryOptions::default())
					.unwrap();
			let directory = std::env::temp_dir().join(format!(
				"fa-reference-exports-{}-{}-{asynchronous}",
				std::process::id(),
				target.name()
			));
			for (path, code) in output.modules {
				let path = directory.join(path);
				std::fs::create_dir_all(path.parent().unwrap()).unwrap();
				std::fs::write(path, code).unwrap();
			}
			let result = Command::new(&bun)
				.arg(directory.join("main.js"))
				.output()
				.unwrap();
			std::fs::remove_dir_all(directory).unwrap();
			assert!(
				result.status.success(),
				"{target:?} asynchronous={asynchronous}: {}",
				String::from_utf8_lossy(&result.stderr)
			);
			assert_eq!(String::from_utf8(result.stdout).unwrap(), "7\n7\n9\n");
		}
	}
}
