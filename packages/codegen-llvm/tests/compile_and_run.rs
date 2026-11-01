//! Compiles the hand-built programs for native and wasm, runs them, and checks
//! their output. Without LLVM (`LLVM_SYS_181_PREFIX` unset at build time), a
//! single test prints why the others were not built.

#[cfg(not(fa_llvm))]
#[test]
fn llvm_tests_are_skipped_without_llvm() {
	assert!(!fa_codegen_llvm::llvm_available());
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::PathBuf;

	use fa_codegen_llvm::runtime::{OVERFLOW_MESSAGE, PANIC_EXIT_CODE};
	use fa_codegen_llvm::{Artifact, Profile, Program, Target, compile, run};

	fn test_directory(name: &str) -> PathBuf {
		PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-codegen-llvm")
			.join(name)
	}

	fn assert_runs(program: Program, target: Target, profile: Profile) -> Artifact {
		let directory = test_directory(&format!("{program:?}-{target:?}-{profile:?}"));
		let artifact = compile(program, target, profile, &directory)
			.unwrap_or_else(|error| panic!("{program:?} on {target:?} ({profile:?}): {error}"));
		let output = run(&artifact).expect("the program runs");
		let stdout = String::from_utf8_lossy(&output.stdout);
		let stderr = String::from_utf8_lossy(&output.stderr);
		if program == Program::Overflow {
			assert_eq!(
				output.status.code(),
				Some(PANIC_EXIT_CODE),
				"stderr: {stderr}"
			);
			assert_eq!(stderr, OVERFLOW_MESSAGE);
		} else {
			assert!(
				output.status.success(),
				"{program:?} on {target:?}: {stderr}"
			);
			assert_eq!(stdout, program.expected_output());
		}
		artifact
	}

	#[test]
	fn native_fibonacci_with_checked_overflow() {
		assert_runs(Program::Fibonacci(30), Target::Native, Profile::Release);
	}

	#[test]
	fn wasm_fibonacci_with_checked_overflow() {
		let artifact = assert_runs(Program::Fibonacci(30), Target::Wasm, Profile::Release);
		let bytes = std::fs::read(&artifact.output).unwrap();
		assert_eq!(&bytes[..4], b"\0asm");
	}

	#[test]
	fn native_overflow_stops_the_program() {
		assert_runs(Program::Overflow, Target::Native, Profile::Release);
	}

	#[test]
	fn wasm_overflow_stops_the_program() {
		assert_runs(Program::Overflow, Target::Wasm, Profile::Release);
	}

	#[test]
	fn concatenation_allocates_on_both_targets() {
		assert_runs(Program::Concatenation, Target::Native, Profile::Size);
		assert_runs(Program::Concatenation, Target::Wasm, Profile::Size);
	}

	#[test]
	fn optimized_fibonacci_keeps_the_recursion_and_the_check() {
		let artifact = assert_runs(Program::Fibonacci(30), Target::Native, Profile::Max);
		let ir = std::fs::read_to_string(&artifact.ir).unwrap();
		assert!(ir.contains("@llvm.sadd.with.overflow.i64"), "{ir}");
		assert!(ir.contains("call fastcc"), "{ir}");
		assert!(
			ir.contains("!prof"),
			"the overflow branch keeps its weights: {ir}"
		);
	}

	#[test]
	fn wasm_hello_world_is_tiny() {
		let artifact = assert_runs(Program::Hello, Target::Wasm, Profile::Size);
		assert!(artifact.output_size < 200, "{} bytes", artifact.output_size);
	}

	/// Every program, target and profile.
	#[test]
	#[ignore]
	fn every_program_target_and_profile() {
		let programs = [
			Program::Fibonacci(25),
			Program::Hello,
			Program::Concatenation,
			Program::Overflow,
		];
		for program in programs {
			for target in [Target::Native, Target::Wasm] {
				for profile in [Profile::Dev, Profile::Release, Profile::Max, Profile::Size] {
					assert_runs(program, target, profile);
				}
			}
		}
	}
}
