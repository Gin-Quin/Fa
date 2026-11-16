//! Generated independent UTF-16 oracle, shared with the bounded program runner.

use super::*;

fn disagreement(expected: &Expected, outcome: &Outcome) -> Option<String> {
	if outcome.exit != expected.exit || !outcome.stderr.is_empty() && outcome.build_failed {
		return Some(format!("exit {}: {}", outcome.exit, outcome.stderr));
	}
	for message in &expected.stderr {
		if !outcome.stderr.contains(message) {
			return Some(format!("missing stderr {message:?}: {}", outcome.stderr));
		}
	}
	let actual: Vec<_> = outcome.stdout.lines().collect();
	if actual.len() != expected.stdout.len() {
		return Some(format!(
			"expected {} lines, got {}: {}",
			expected.stdout.len(),
			actual.len(),
			outcome.stderr
		));
	}
	for (line, (want, got)) in expected.stdout.iter().zip(actual).enumerate() {
		for (field, (want, got)) in want.split('|').zip(got.split('|')).enumerate() {
			if want != got {
				return Some(format!(
					"line {}, field {}: expected {want:?}, got {got:?}",
					line + 1,
					field + 1
				));
			}
		}
		if want != got {
			return Some(format!("line {}: differing field counts", line + 1));
		}
	}
	None
}

#[test]
fn generated_string_differential() {
	let Some(bun) = fa::find_bun() else {
		eprintln!("string differential: Bun is required for the independent oracle");
		return;
	};
	let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
		.join(format!("strings-differential-{}", std::process::id()));
	fs::create_dir_all(&scratch).expect("a scratch directory");
	let generated = scratch.join("corpus");
	let full = std::env::var("FA_STRINGS_DIFFERENTIAL_FULL").is_ok_and(|value| value == "1");
	let mut generator = Command::new("python3");
	generator
		.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/strings-differential/generate.py"))
		.arg(&generated)
		.arg(bun);
	if full {
		generator.arg("--full");
	}
	let output = generator
		.output()
		.expect("Python runs the deterministic generator");
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	let programs = fa_files(&generated);
	let queue = Mutex::new(
		programs
			.iter()
			.enumerate()
			.filter(|(_, path)| selected(path)),
	);
	let failures = Mutex::new(Vec::new());
	let started = Instant::now();
	std::thread::scope(|scope| {
		for _ in 0..WORKERS {
			scope.spawn(|| {
				loop {
					let Some((index, program)) = queue.lock().unwrap().next() else {
						break;
					};
					let found = run_shard(index, program, &scratch, full);
					eprintln!(
						"string differential: {} ({} disagreements)",
						program.display(),
						found.len()
					);
					failures.lock().unwrap().extend(found);
				}
			});
		}
	});
	let mut failures = failures.into_inner().unwrap();
	failures.sort();
	eprintln!(
		"string differential {} tier: {} shards in {:.2}s, {} disagreements",
		if full { "full" } else { "quick" },
		programs.iter().filter(|path| selected(path)).count(),
		started.elapsed().as_secs_f64(),
		failures.len()
	);
	assert!(failures.is_empty(), "{}", failures.join("\n\n"));
	fs::remove_dir_all(scratch).expect("remove successful generated corpus");
}

fn run_shard(index: usize, program: &Path, scratch: &Path, full: bool) -> Vec<String> {
	let mut failures = Vec::new();
	let mut expected = expectations(&fs::read_to_string(program).expect("generated source"));
	for target in ["bun", "node", "web"] {
		expected.target = target;
		let outcome = limited(|| {
			run(
				program,
				&expected,
				&scratch,
				index * 3
					+ match target {
						"bun" => 0,
						"node" => 1,
						_ => 2,
					},
			)
		});
		if let Some(reason) = disagreement(&expected, &outcome) {
			failures.push(format!("{} on {target}: {reason}", program.display()));
		}
	}
	if fa_codegen_llvm::llvm_available() {
		use fa_codegen_llvm::{OptLevel, Options, Target};
		for target in [Target::Native, Target::Wasm, Target::NativeFreestanding] {
			if expected
				.skip_targets
				.iter()
				.any(|skip| skip == target.name())
			{
				continue;
			}
			let levels: &[_] = if full {
				&[OptLevel::O0, OptLevel::O3]
			} else {
				&[OptLevel::O0]
			};
			for &opt_level in levels {
				let options = Options {
					opt_level,
					leak_check: true,
					..Options::default()
				};
				let destination = scratch.join(format!("{index}-{}-{opt_level:?}", target.name()));
				let compiled = limited(|| {
					std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
						fa_codegen_llvm::compile_file(program, target, &options, &destination)
					}))
				});
				let artifact = match compiled {
					Ok(Ok(artifact)) => artifact,
					Ok(Err(error)) => {
						failures.push(format!(
							"{} on {target:?}/{opt_level:?}: {error}",
							program.display()
						));
						continue;
					}
					Err(_) => {
						failures.push(format!(
							"{} on {target:?}/{opt_level:?}: compiler panicked",
							program.display()
						));
						continue;
					}
				};
				let mut command = if let Some(loader) = &artifact.loader {
					let mut command =
						Command::new(fa_codegen_llvm::javascript_runtime().expect("a wasm runner"));
					command.arg(loader);
					command
				} else {
					Command::new(&artifact.output)
				};
				let outcome = limited(|| {
					execute(
						&mut command,
						scratch,
						&format!("native-run-{index}-{}-{opt_level:?}", target.name()),
					)
				});
				if let Some(reason) = disagreement(&expected, &outcome) {
					failures.push(format!(
						"{} on {target:?}/{opt_level:?}: {reason}",
						program.display()
					));
				}
			}
		}
	}
	failures
}
