//! Objects linked without LTO give each function, global and jump table its
//! own section (`units.rs`, `fa_use_unique_sections`), so `--gc-sections`
//! discards the runtime functions a program doesn't use: a program without
//! JSON links no JSON reader, and freestanding links no `strtod` for it.

#[cfg(fa_llvm)]
mod llvm {
	use std::path::{Path, PathBuf};
	use std::process::Command;

	use fa_analyzer::modules::CodebaseResolver;
	use fa_codegen_llvm::{Libraries, OptLevel, Options, PackageOutcome, SplitOptions, Target};

	/// A package of two units that uses no JSON.
	fn files() -> Vec<(String, String)> {
		vec![
			(
				"package.fa".to_string(),
				"export = { name = \"sections\" }\n".to_string(),
			),
			(
				"library/describe.fa".to_string(),
				"export function = (count: Integer): String => \"{count} items\"\n".to_string(),
			),
			(
				"application/main.fa".to_string(),
				"use describe = library.describe\nexport function = () => {\n\tconsole.log(describe(3))\n}\n".to_string(),
			),
		]
	}

	/// Builds the package per unit at O0 with leak checks, without LTO.
	fn build(target: Target, directory: &Path) -> PathBuf {
		let mut session = fa_analyzer::session::Session::new();
		session.set_target(target.analysis_target());
		session.set_resolver(Box::new(CodebaseResolver::from_texts(&files(), None)));
		let run = fa_analyzer::pipeline::Options {
			entry: Some("application/main".to_string()),
			..fa_analyzer::pipeline::Options::default()
		};
		let options = Options {
			opt_level: OptLevel::O0,
			leak_check: true,
			asynchronous: Some(false),
			..Options::default()
		};
		let split = SplitOptions {
			lto: false,
			..SplitOptions::default()
		};
		let (report, outcome) = fa_codegen_llvm::compile_package(
			&mut session,
			&run,
			target,
			&options,
			&split,
			&directory.join("objects"),
			&directory.join("main"),
			&Libraries::default(),
		);
		let PackageOutcome::Built(artifact) = outcome else {
			let diagnostics: Vec<_> = report
				.modules
				.iter()
				.flat_map(|module| module.diagnostics.iter().map(|d| d.message.clone()))
				.collect();
			panic!("{}: {outcome:?} {diagnostics:?}", target.name());
		};
		let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
		assert!(result.status.success(), "{}: {result:?}", target.name());
		assert_eq!(result.stdout, b"3 items\n", "{}", target.name());
		artifact.output
	}

	/// The names of the symbols an executable defines.
	fn defined_symbols(executable: &Path) -> Vec<String> {
		let nm = Path::new(env!("FA_LLVM_PREFIX")).join("bin/llvm-nm");
		let output = Command::new(nm)
			.arg("--defined-only")
			.arg(executable)
			.output()
			.unwrap();
		assert!(output.status.success(), "{output:?}");
		String::from_utf8(output.stdout)
			.unwrap()
			.lines()
			.filter_map(|line| line.split_whitespace().last().map(str::to_string))
			.collect()
	}

	#[test]
	fn unused_runtime_functions_reached_from_jump_tables_are_discarded() {
		let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("unique-sections");
		for target in [Target::Native, Target::NativeFreestanding] {
			let directory = root.join(target.name());
			let _ = std::fs::remove_dir_all(&directory);
			let executable = build(target, &directory);
			let symbols = defined_symbols(&executable);
			for unused in ["fa_json_parse", "json_value"] {
				assert!(
					!symbols.iter().any(|symbol| symbol == unused),
					"{}: `{unused}` is linked",
					target.name()
				);
			}
		}
	}
}
