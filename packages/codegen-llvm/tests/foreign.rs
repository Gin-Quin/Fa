//! Foreign code, end to end: the programs of `tests/foreign/*.fa` call C
//! functions of the C library and of a small C library (`farm.c`) compiled
//! here with `cc`, and are compared with their `-- prints:` annotations.
//! A program whose name starts with `farm` links that library.

#[cfg(not(fa_llvm))]
#[test]
fn foreign_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::{Path, PathBuf};
	use std::process::Command;

	use fa_codegen_llvm::{Libraries, Options, Target, compile_file_linking, run};

	/// The directory of the test programs.
	fn programs() -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/foreign")
	}

	/// Compiles `farm.c` into a static library of `directory`: `None`
	/// without a C compiler.
	fn farm_library(directory: &Path) -> Option<Libraries> {
		let object = directory.join("farm.o");
		let compiled = Command::new(std::env::var_os("CC").unwrap_or_else(|| "cc".into()))
			.args(["-c", "-O1", "-fPIC", "-o"])
			.arg(&object)
			.arg(programs().join("farm.c"))
			.status()
			.ok()?;
		if !compiled.success() {
			return None;
		}
		let archive = directory.join("libfarm.a");
		let archived = Command::new("ar")
			.arg("rcs")
			.arg(&archive)
			.arg(&object)
			.status()
			.ok()?;
		archived.success().then(|| Libraries {
			link: vec!["farm".into()],
			paths: vec![directory.to_path_buf()],
		})
	}

	#[test]
	fn foreign_programs_run() {
		let directory = std::env::temp_dir().join(format!("fa-foreign-{}", std::process::id()));
		std::fs::create_dir_all(&directory).expect("a temporary directory");
		let farm = farm_library(&directory);
		let mut failures = Vec::new();
		let mut paths: Vec<PathBuf> = std::fs::read_dir(programs())
			.expect("tests/foreign")
			.map(|entry| entry.expect("an entry").path())
			.filter(|path| path.extension().is_some_and(|extension| extension == "fa"))
			.collect();
		paths.sort();
		for path in paths {
			let name = path.file_name().unwrap().to_string_lossy().to_string();
			let libraries = if name.starts_with("farm") {
				match &farm {
					Some(libraries) => libraries.clone(),
					None => {
						eprintln!("skipped {name}: no C compiler (`cc`)");
						continue;
					}
				}
			} else {
				Libraries::default()
			};
			let text = std::fs::read_to_string(&path).expect("a program");
			let expected: Vec<&str> = text
				.lines()
				.filter_map(|line| line.split_once("-- prints: ").map(|(_, printed)| printed))
				.collect();
			let output = directory.join(name.trim_end_matches(".fa"));
			let artifact = match compile_file_linking(
				&path,
				Target::Native,
				&Options::default(),
				&output,
				&libraries,
			) {
				Ok(artifact) => artifact,
				Err(error) => {
					failures.push(format!("{name}: {error}"));
					continue;
				}
			};
			let result = run(&artifact, &[]).expect("the program runs");
			let printed = String::from_utf8_lossy(&result.stdout);
			let lines: Vec<&str> = printed.lines().collect();
			if lines != expected || !result.status.success() {
				failures.push(format!(
					"{name}: printed {lines:?}, expected {expected:?} ({}; stderr: {})",
					result.status,
					String::from_utf8_lossy(&result.stderr)
				));
			}
		}
		let _ = std::fs::remove_dir_all(&directory);
		assert!(failures.is_empty(), "{}", failures.join("\n"));
	}
}
