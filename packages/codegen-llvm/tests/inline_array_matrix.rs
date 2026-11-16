//! Generated item × place × operation coverage, with interpreter and Bun
//! oracles. Every LLVM program runs at O0/O3 on native, wasm and freestanding.
//! The generator and the generated, annotated sources are deterministic.

#[cfg(not(fa_llvm))]
#[test]
fn llvm_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::fs;
	use std::path::{Path, PathBuf};
	use std::process::{Command, Stdio};
	use std::sync::Arc;
	use std::time::{Duration, Instant};

	use fa_analyzer::Session;
	use fa_codegen_llvm::{OptLevel, Options, Target, compile_file, run};

	fn directory() -> PathBuf {
		Path::new(env!("CARGO_TARGET_TMPDIR")).join("inline-array-matrix")
	}

	fn bun() -> PathBuf {
		let local =
			PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".bun/bin/bun");
		if local.is_file() {
			local
		} else {
			PathBuf::from("bun")
		}
	}

	fn reference(path: &Path) -> String {
		let source = fs::read_to_string(path).unwrap();
		let mut session = Session::new();
		session.set_target("bun");
		let file = session.set_source("main.foreign", Arc::from(source.as_str()));
		session.analyze_file(file);
		let js = fa_codegen_js::generate(
			&mut session,
			file,
			&fa_codegen_js::Options::new(fa_codegen_js::Target::Bun, path.display().to_string()),
		)
		.unwrap_or_else(|error| panic!("{}: {error}", path.display()));
		let script = path.with_extension("js");
		let helpers = fa_codegen_js::collect_helpers_inline(&js.helpers);
		fs::write(&script, format!("{helpers}\n{}", js.program)).unwrap();
		let output = Command::new(bun())
			.arg(&script)
			.output()
			.expect("Bun runs the reference");
		assert!(
			output.status.success(),
			"{}: {}",
			path.display(),
			String::from_utf8_lossy(&output.stderr)
		);
		let stdout = String::from_utf8(output.stdout).unwrap();
		let lines: Vec<_> = stdout.lines().collect();
		assert_eq!(lines.len() % 2, 0, "an oracle/runtime pair per position");
		for pair in lines.as_chunks::<2>().0 {
			assert_eq!(
				pair[0],
				pair[1],
				"{}: interpreter differs from Bun",
				path.display()
			);
		}
		let mut annotated = source;
		for line in lines {
			annotated.push_str(&format!("\n-- prints: {line}"));
		}
		fs::write(path, annotated).unwrap();
		fs::write(path.with_extension("stdout"), &stdout).unwrap();
		stdout
	}

	/// A fresh process per compilation bounds LLVM's peak memory and isolates
	/// compiler panics. Save its log, and enforce the machine's 512 MiB RSS cap.
	fn child(path: &Path, target: Target, level: &str) -> Result<(), String> {
		let log = path.with_extension(format!("{}-{level}.log", target.name()));
		let output = fs::File::create(&log).unwrap();
		let mut child = Command::new(std::env::current_exe().unwrap())
			.args(["--exact", "with_llvm::matrix_case", "--nocapture"])
			.env("FA_INLINE_MATRIX_FILE", path)
			.env("FA_INLINE_MATRIX_TARGET", target.name())
			.env("FA_INLINE_MATRIX_OPT", level)
			.stdout(Stdio::from(output.try_clone().unwrap()))
			.stderr(Stdio::from(output))
			.spawn()
			.unwrap();
		let start = Instant::now();
		let status = loop {
			if let Some(status) = child.try_wait().unwrap() {
				break status;
			}
			let rss = fs::read_to_string(format!("/proc/{}/status", child.id()))
				.unwrap_or_default()
				.lines()
				.find_map(|line| {
					line.strip_prefix("VmRSS:")?
						.split_whitespace()
						.next()?
						.parse::<u64>()
						.ok()
				})
				.unwrap_or(0);
			if rss > 512 * 1024 || start.elapsed() > Duration::from_secs(180) {
				child.kill().unwrap();
				child.wait().unwrap();
				return Err(format!(
					"{} {} O{level}: RSS {rss} KiB, elapsed {:?}",
					path.display(),
					target.name(),
					start.elapsed()
				));
			}
			std::thread::sleep(Duration::from_millis(20));
		};
		if status.success() {
			Ok(())
		} else {
			Err(format!(
				"{} {} O{level}: {}",
				path.display(),
				target.name(),
				fs::read_to_string(log).unwrap()
			))
		}
	}

	#[test]
	fn generated_inline_array_matrix() {
		let directory = directory();
		fs::create_dir_all(&directory).unwrap();
		assert!(
			Command::new("python3")
				.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/inline_array_matrix.py"))
				.arg(&directory)
				.status()
				.unwrap()
				.success()
		);
		let filter = std::env::var("FA_INLINE_MATRIX_FILTER").unwrap_or_default();
		let mut files: Vec<_> = fs::read_dir(&directory)
			.unwrap()
			.map(|entry| entry.unwrap().path())
			.filter(|path| {
				path.to_string_lossy().ends_with(".fa")
					&& path
						.file_name()
						.unwrap()
						.to_string_lossy()
						.contains(&filter)
			})
			.collect();
		files.sort();
		let mut failures = Vec::new();
		for path in &files {
			if path.to_string_lossy().contains(".reject.") {
				let source = fs::read_to_string(path).unwrap();
				let expected = source
					.lines()
					.find_map(|line| line.strip_prefix("-- error: "))
					.unwrap();
				for target in ["bun", "native", "wasm", "native-freestanding"] {
					let mut session = Session::new();
					session.set_target(target);
					let file = session.set_source("main.foreign", Arc::from(source.as_str()));
					session.analyze_file(file);
					assert!(
						session
							.diagnostics(file)
							.iter()
							.any(|diagnostic| diagnostic.code.id == expected),
						"{} {target}: {:?}",
						path.display(),
						session.diagnostics(file)
					);
				}
				continue;
			}
			reference(path);
			for target in [Target::Native, Target::Wasm, Target::NativeFreestanding] {
				for level in ["0", "3"] {
					if let Err(error) = child(path, target, level) {
						failures.push(error);
					}
				}
			}
			eprintln!(
				"matrix: {} checked",
				path.file_name().unwrap().to_string_lossy()
			);
		}
		assert!(
			failures.is_empty(),
			"{} matrix failures:\n{}",
			failures.len(),
			failures.join("\n")
		);
	}

	#[test]
	fn matrix_case() {
		let Some(path) = std::env::var_os("FA_INLINE_MATRIX_FILE") else {
			return;
		};
		let path = PathBuf::from(path);
		let target = Target::from_name(&std::env::var("FA_INLINE_MATRIX_TARGET").unwrap()).unwrap();
		let level = std::env::var("FA_INLINE_MATRIX_OPT").unwrap();
		let options = Options {
			leak_check: true,
			opt_level: if level == "0" {
				OptLevel::O0
			} else {
				OptLevel::O3
			},
			..Options::default()
		};
		let output = path.with_extension(format!("{}-O{level}", target.name()));
		let compiled = compile_file(&path, target, &options, &output);
		// The target's documented libc-free boundary (decisions.md, LLVM M1).
		// Check its precise refusal; do not silently skip a failed compilation.
		if target == Target::NativeFreestanding
			&& path
				.file_name()
				.unwrap()
				.to_string_lossy()
				.starts_with("float")
		{
			let error = compiled.expect_err("freestanding float formatting requires libc");
			assert!(
				error
					.to_string()
					.contains("floats (`snprintf` of the C library) on the freestanding target"),
				"{error}"
			);
			return;
		}
		let artifact = compiled.unwrap();
		let result = run(&artifact, &[]).unwrap();
		assert!(
			result.status.success(),
			"{}: {}",
			result.status,
			String::from_utf8_lossy(&result.stderr)
		);
		let actual = String::from_utf8(result.stdout).unwrap();
		let expected = fs::read_to_string(path.with_extension("stdout")).unwrap();
		assert_eq!(
			actual.lines().count(),
			expected.lines().count(),
			"the output line count"
		);
		for (actual, expected) in actual.lines().zip(expected.lines()) {
			assert_eq!(
				actual,
				expected,
				"{} {} O{level}",
				path.display(),
				target.name()
			);
		}
		assert!(
			result.stderr.is_empty(),
			"{}",
			String::from_utf8_lossy(&result.stderr)
		);
	}
}
