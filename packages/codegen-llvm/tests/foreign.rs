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
			..Libraries::default()
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

	/// Foreign code on wasm: `tests/foreign/wasm/geometry.c`, compiled to
	/// WebAssembly by clang 18 (the LLVM install's, skipped without it), is
	/// linked with the program; the function no C defines is an import of
	/// `env`, which the loader takes from `env.mjs` next to it.
	#[test]
	fn foreign_programs_run_on_wasm() {
		let clang = std::env::var_os("LLVM_SYS_181_PREFIX")
			.map(|prefix| PathBuf::from(prefix).join("bin/clang"))
			.filter(|clang| clang.is_file());
		let Some(clang) = clang else {
			eprintln!("skipped: no clang in the LLVM install");
			return;
		};
		let sources = programs().join("wasm");
		let directory =
			std::env::temp_dir().join(format!("fa-foreign-wasm-{}", std::process::id()));
		std::fs::create_dir_all(&directory).expect("a temporary directory");
		let object = directory.join("geometry.o");
		let compiled = Command::new(&clang)
			.args(["--target=wasm32-unknown-unknown", "-O1", "-c", "-o"])
			.arg(&object)
			.arg(sources.join("geometry.c"))
			.status()
			.expect("clang runs");
		assert!(compiled.success(), "clang compiles geometry.c to wasm32");
		let libraries = Libraries {
			link: vec![object.display().to_string()],
			paths: Vec::new(),
			..Libraries::default()
		};
		let path = sources.join("geometry.foreign.fa");
		let output = directory.join("geometry");
		let artifact = compile_file_linking(
			&path,
			Target::Wasm,
			&Options::default(),
			&output,
			&libraries,
		)
		.unwrap_or_else(|error| panic!("geometry.foreign.fa: {error}"));
		let loader = artifact.loader.clone().expect("a loader");
		std::fs::copy(sources.join("env.mjs"), loader.with_file_name("env.mjs"))
			.expect("env.mjs next to the loader");
		let text = std::fs::read_to_string(&path).expect("the program");
		let expected: Vec<&str> = text
			.lines()
			.filter_map(|line| line.split_once("-- prints: ").map(|(_, printed)| printed))
			.collect();
		let result = run(&artifact, &[]).expect("the program runs");
		let printed = String::from_utf8_lossy(&result.stdout);
		let lines: Vec<&str> = printed.lines().collect();
		let _ = std::fs::remove_dir_all(&directory);
		assert!(
			lines == expected && result.status.success(),
			"printed {lines:?}, expected {expected:?} ({}; stderr: {})",
			result.status,
			String::from_utf8_lossy(&result.stderr)
		);
	}
}
