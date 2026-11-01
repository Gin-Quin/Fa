//! Prints the sizes and timings of the proof of concept, and compares the
//! recursive Fibonacci with the same program compiled by `rustc -O`.
//!
//! ```bash
//! cargo run -p fa-codegen-llvm --release --example report
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

	use fa_codegen_llvm::{Artifact, Profile, Program, Target, compile, run};

	const RUNS: usize = 5;

	pub fn main() -> Result<(), Box<dyn std::error::Error>> {
		let directory =
			Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/fa-codegen-llvm-report");
		std::fs::create_dir_all(&directory)?;

		println!("## Sizes and compile times (milliseconds)\n");
		println!(
			"| program | target | profile | object | output | build | optimize | emit | link |"
		);
		println!("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |");
		for program in [
			Program::Hello,
			Program::Fibonacci(30),
			Program::Concatenation,
		] {
			for target in [Target::Native, Target::Freestanding, Target::Wasm] {
				for profile in [Profile::Dev, Profile::Release, Profile::Max, Profile::Size] {
					let artifact = compile(program, target, profile, &directory)?;
					check(&artifact)?;
					let t = artifact.timings;
					println!(
						"| {} | {target:?} | {profile:?} | {} | {} | {:.2} | {:.2} | {:.2} | {:.1} |",
						program.name(),
						artifact.object_size,
						artifact.output_size,
						milliseconds(t.build),
						milliseconds(t.optimize),
						milliseconds(t.emit),
						milliseconds(t.link),
					);
				}
			}
		}

		println!(
			"\n## Recursive Fibonacci, best of {RUNS} runs (milliseconds, process included)\n"
		);
		println!(
			"| n | Fa native O3 | Fa freestanding O3 | Fa wasm O3 (node) | rustc -O, overflow checks | rustc -O, no checks |"
		);
		println!("| ---: | ---: | ---: | ---: | ---: | ---: |");
		let rust_checked = rustc(&directory, true)?;
		let rust_unchecked = rustc(&directory, false)?;
		for n in [30, 40] {
			let mut row = format!("| {n} |");
			for target in [Target::Native, Target::Freestanding, Target::Wasm] {
				let artifact = compile(Program::Fibonacci(n), target, Profile::Max, &directory)?;
				let time = best_of(|| check(&artifact))?;
				row.push_str(&format!(" {:.1} |", milliseconds(time)));
			}
			for executable in [&rust_checked, &rust_unchecked] {
				let time = best_of(|| {
					let output = Command::new(executable).arg(n.to_string()).output()?;
					assert_eq!(
						String::from_utf8_lossy(&output.stdout),
						Program::Fibonacci(n).expected_output()
					);
					Ok(())
				})?;
				row.push_str(&format!(" {:.1} |", milliseconds(time)));
			}
			println!("{row}");
		}
		for executable in [&rust_checked, &rust_unchecked] {
			println!(
				"\n{}: {} bytes (stripped)",
				executable.file_name().unwrap().to_string_lossy(),
				std::fs::metadata(executable)?.len()
			);
		}
		Ok(())
	}

	fn check(artifact: &Artifact) -> Result<(), Box<dyn std::error::Error>> {
		let output = run(artifact)?;
		if !output.status.success()
			|| String::from_utf8_lossy(&output.stdout) != artifact.program.expected_output()
		{
			return Err(format!("{:?} printed {:?}", artifact.output, output).into());
		}
		Ok(())
	}

	fn rustc(
		directory: &Path,
		overflow_checks: bool,
	) -> Result<PathBuf, Box<dyn std::error::Error>> {
		let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/fibonacci.rs");
		let name = if overflow_checks {
			"rust-fibonacci-checked"
		} else {
			"rust-fibonacci"
		};
		let output = directory.join(name);
		let status = Command::new("rustc")
			.args(["-O", "-C", "strip=symbols", "-C"])
			.arg(format!(
				"overflow-checks={}",
				if overflow_checks { "on" } else { "off" }
			))
			.arg(&source)
			.arg("-o")
			.arg(&output)
			.status()?;
		if !status.success() {
			return Err("rustc failed".into());
		}
		Ok(output)
	}

	fn best_of(
		mut run: impl FnMut() -> Result<(), Box<dyn std::error::Error>>,
	) -> Result<Duration, Box<dyn std::error::Error>> {
		let mut best = Duration::MAX;
		for _ in 0..RUNS {
			let start = Instant::now();
			run()?;
			best = best.min(start.elapsed());
		}
		Ok(best)
	}

	fn milliseconds(duration: Duration) -> f64 {
		duration.as_secs_f64() * 1000.0
	}
}
