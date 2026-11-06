//! Equal layouts retain the methods selected from written types, through
//! generic calls, function values, fields and generic type owners.

use std::path::PathBuf;
use std::process::Command;

use fa_analyzer::{CodebaseResolver, Session};
use fa_codegen_js::{EntryOptions, Options, Target, generate, generate_program};

fn runtime(target: Target) -> Option<PathBuf> {
	let name = target.name();
	if target == Target::Bun
		&& let Some(home) = std::env::var_os("HOME")
	{
		let executable = PathBuf::from(home).join(".bun/bin/bun");
		if executable.is_file() {
			return Some(executable);
		}
	}
	Command::new(name)
		.arg("--version")
		.output()
		.ok()
		.filter(|output| output.status.success())
		.map(|_| PathBuf::from(name))
}

fn files(source: &str) -> Vec<(String, String)> {
	let mut files: Vec<(String, String)> = Vec::new();
	for line in source.lines() {
		if let Some(path) = line.strip_prefix("-- file: ") {
			files.push((path.trim().to_string(), String::new()));
		} else if let Some((_, text)) = files.last_mut() {
			text.push_str(line);
			text.push('\n');
		}
	}
	files
}

#[test]
fn written_method_origins_survive_erasure_on_bun_and_node() {
	let fixtures = [
		include_str!("../../analyzer/tests/cases/review/written-method-origins.fa"),
		include_str!("../../analyzer/tests/cases/review/written-method-generic-owners.fa"),
		include_str!("../../analyzer/tests/cases/review/written-method-origins-async.fa"),
		include_str!("../../analyzer/tests/cases/modules/generic-hidden-arguments.fa"),
	];
	for target in [Target::Bun, Target::Node] {
		let Some(runtime) = runtime(target) else {
			eprintln!("skipped: {} is not installed", target.name());
			continue;
		};
		for (index, source) in fixtures.iter().enumerate() {
			let options = Options::new(target, "method-origins.fa");
			let mut files = files(source);
			let modules = if files.is_empty() {
				let (mut session, file) = fa_analyzer::analyze_source(source);
				vec![(
					"main.js".to_string(),
					generate(&mut session, file, &options).unwrap().code,
				)]
			} else {
				files.push((
					"package.fa".to_string(),
					"export = { name = \"case\" }\n".to_string(),
				));
				let mut session = Session::new();
				session.set_target(target.name());
				session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
				let program = session.analyze_package(Some("application/main"));
				generate_program(&mut session, &program, &options, &EntryOptions::default())
					.unwrap()
					.modules
			};
			let directory = std::env::temp_dir().join(format!(
				"fa-method-origins-{}-{}-{index}",
				std::process::id(),
				target.name()
			));
			std::fs::create_dir_all(&directory).unwrap();
			std::fs::write(directory.join("package.json"), "{\"type\":\"module\"}\n").unwrap();
			for (path, code) in modules {
				let path = directory.join(path);
				std::fs::create_dir_all(path.parent().unwrap()).unwrap();
				std::fs::write(path, code).unwrap();
			}
			let output = Command::new(&runtime)
				.arg(directory.join("main.js"))
				.output()
				.unwrap();
			assert!(
				output.status.success(),
				"{} fixture {index}: {} ({})",
				target.name(),
				String::from_utf8_lossy(&output.stderr),
				directory.display()
			);
			let expected: Vec<_> = source
				.lines()
				.filter_map(|line| line.split_once("-- prints: ").map(|(_, printed)| printed))
				.collect();
			let stdout = String::from_utf8(output.stdout).unwrap();
			assert_eq!(
				stdout.lines().collect::<Vec<_>>(),
				expected,
				"{} fixture {index}",
				target.name()
			);
			std::fs::remove_dir_all(directory).unwrap();
		}
	}
}
