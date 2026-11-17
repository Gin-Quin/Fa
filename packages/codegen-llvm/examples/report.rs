//! Prints the sizes and compile times of the programs of `bench/` on every
//! target, and compares their run times with the same programs compiled by
//! `rustc` (`benchmarks/fib/main.rs`, `bench/collatz.rs`).
//!
//! ```bash
//! cargo run -p fa-codegen-llvm --release --example report
//! cargo run -p fa-codegen-llvm --release --example report -- --sizes-only
//! ```

#[cfg(not(fa_llvm))]
fn main() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
	with_llvm::main()
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::{Path, PathBuf};
	use std::process::Command;
	use std::time::{Duration, Instant};

	use fa_codegen_llvm::{Artifact, OptLevel, Options, Target, compile_file, run};

	const RUNS: usize = 3;

	fn manifest() -> &'static Path {
		Path::new(env!("CARGO_MANIFEST_DIR"))
	}

	pub fn main() -> Result<(), Box<dyn std::error::Error>> {
		let directory = manifest().join("../../target/fa-codegen-llvm-report");
		std::fs::create_dir_all(&directory)?;
		let profiles = [
			(
				"O0",
				Options {
					opt_level: OptLevel::O0,
					..Options::default()
				},
			),
			("O3", Options::default()),
			(
				"size",
				Options {
					size: true,
					..Options::default()
				},
			),
		];

		println!("## Sizes (bytes) and compile times (milliseconds)\n");
		println!("| program | target | options | output | build | optimize | emit | link |");
		println!("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |");
		for program in ["hello", "fib", "collatz"] {
			let source = manifest().join(format!("bench/{program}.fa"));
			for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
				for (name, options) in &profiles {
					let output = directory.join(format!("{program}-{}-{name}", target.name()));
					let artifact = compile_file(&source, target, options, &output)?;
					let t = artifact.timings;
					println!(
						"| {program} | {} | {name} | {} | {:.2} | {:.2} | {:.2} | {:.1} |",
						target.name(),
						artifact.output_size,
						milliseconds(t.build),
						milliseconds(t.optimize),
						milliseconds(t.emit),
						milliseconds(t.link),
					);
				}
			}
		}
		// Size audits need no timed program runs, which may require a
		// different CPU affinity from compiler and linker processes.
		if std::env::args().any(|argument| argument == "--sizes-only") {
			return Ok(());
		}

		println!("\n## Run times, best of {RUNS} (milliseconds, process included)\n");
		println!(
			"| program | Fa native | Fa freestanding | Fa wasm (node) | rustc -O3, overflow checks | rustc -O3 |"
		);
		println!("| --- | ---: | ---: | ---: | ---: | ---: |");
		let rust_sources = [
			("fib", manifest().join("../../benchmarks/fib/main.rs"), "42"),
			("collatz", manifest().join("bench/collatz.rs"), ""),
		];
		let mut sizes = Vec::new();
		for (program, rust_source, argument) in &rust_sources {
			let source = manifest().join(format!("bench/{program}.fa"));
			let mut row = format!("| {program} |");
			let mut expected = None;
			for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
				let output = directory.join(format!("{program}-{}-run", target.name()));
				let artifact = compile_file(&source, target, &Options::default(), &output)?;
				let (time, stdout) = best_of(|| fa_run(&artifact))?;
				check_same(&mut expected, stdout)?;
				row.push_str(&format!(" {:.0} |", milliseconds(time)));
				if target == Target::Native {
					let stripped = strip(&artifact.output, &directory)?;
					sizes.push(format!(
						"{program}: Fa native O3 {stripped} bytes (stripped)"
					));
				}
			}
			for checks in [true, false] {
				let executable = rustc(rust_source, &directory, checks)?;
				let (time, stdout) = best_of(|| {
					let mut command = Command::new(&executable);
					if !argument.is_empty() {
						command.arg(argument);
					}
					Ok(String::from_utf8(command.output()?.stdout)?)
				})?;
				check_same(&mut expected, stdout)?;
				row.push_str(&format!(" {:.0} |", milliseconds(time)));
				if checks {
					sizes.push(format!(
						"{program}: rustc -O3 {} bytes (stripped, panic=abort)",
						std::fs::metadata(&executable)?.len()
					));
				}
			}
			println!("{row}");
		}
		println!();
		for size in sizes {
			println!("- {size}");
		}
		Ok(())
	}

	fn fa_run(artifact: &Artifact) -> Result<String, Box<dyn std::error::Error>> {
		let output = run(artifact, &[])?;
		if !output.status.success() {
			return Err(format!("{:?} failed: {output:?}", artifact.output).into());
		}
		Ok(String::from_utf8(output.stdout)?)
	}

	fn check_same(
		expected: &mut Option<String>,
		stdout: String,
	) -> Result<(), Box<dyn std::error::Error>> {
		match expected {
			Some(expected) if *expected != stdout => {
				Err(format!("the outputs differ: {expected:?} and {stdout:?}").into())
			}
			Some(_) => Ok(()),
			None => {
				*expected = Some(stdout);
				Ok(())
			}
		}
	}

	/// The size of a stripped copy of an executable.
	fn strip(executable: &Path, directory: &Path) -> Result<u64, Box<dyn std::error::Error>> {
		let copy = directory.join("stripped");
		std::fs::copy(executable, &copy)?;
		Command::new("strip").arg(&copy).status()?;
		Ok(std::fs::metadata(&copy)?.len())
	}

	fn rustc(
		source: &Path,
		directory: &Path,
		overflow_checks: bool,
	) -> Result<PathBuf, Box<dyn std::error::Error>> {
		let stem = source
			.parent()
			.and_then(Path::file_name)
			.filter(|name| *name != "bench")
			.or_else(|| source.file_stem())
			.map(|name| name.to_string_lossy().into_owned())
			.unwrap_or_default();
		let name = format!(
			"rust-{stem}{}",
			if overflow_checks { "-checked" } else { "" }
		);
		let output = directory.join(name);
		let status = Command::new("rustc")
			.args([
				"-C",
				"opt-level=3",
				"-C",
				"panic=abort",
				"-C",
				"strip=symbols",
			])
			.arg("-C")
			.arg(format!(
				"overflow-checks={}",
				if overflow_checks { "on" } else { "off" }
			))
			.arg(source)
			.arg("-o")
			.arg(&output)
			.status()?;
		if !status.success() {
			return Err("rustc failed".into());
		}
		Ok(output)
	}

	fn best_of(
		mut run: impl FnMut() -> Result<String, Box<dyn std::error::Error>>,
	) -> Result<(Duration, String), Box<dyn std::error::Error>> {
		let mut best = Duration::MAX;
		let mut output = String::new();
		for _ in 0..RUNS {
			let start = Instant::now();
			output = run()?;
			best = best.min(start.elapsed());
		}
		Ok((best, output))
	}

	fn milliseconds(duration: Duration) -> f64 {
		duration.as_secs_f64() * 1000.0
	}
}
