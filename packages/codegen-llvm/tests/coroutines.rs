//! The coroutine spike (`examples/coroutines/`, `design/async-spike.md`):
//! LLVM coroutines built by hand are lowered by the `coro-*` passes of the
//! `default<O0>` and `default<O3>` pipelines, run natively and on wasm
//! (when clang and node are available), and `coro-elide` removes the frames
//! of the coroutines that do not outlive their caller.

#[cfg(not(fa_llvm))]
#[test]
fn coroutine_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

// The crate links `libLLVM` (its build script): link it although `spike`
// only uses inkwell and llvm-sys.
#[cfg(fa_llvm)]
extern crate fa_codegen_llvm;

#[cfg(fa_llvm)]
#[allow(dead_code)]
#[path = "../examples/coroutines/spike.rs"]
mod spike;

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::Path;

	use crate::spike::{BENCHMARKS, SpikeTarget, build, expected, run, wasm_available};

	fn assert_runs(target: SpikeTarget, pipeline: &str, optimized: bool) {
		let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
			.join("coroutines")
			.join(format!("{}-{}", target.name(), optimized));
		let built = build(target, &directory, pipeline).unwrap();
		assert_eq!(
			built.coro_calls_left, 0,
			"{pipeline} left coroutine intrinsics"
		);
		assert!(
			built
				.split_functions
				.contains(&"gen_new.resume".to_string()),
			"coro-split did not run: {:?}",
			built.split_functions
		);
		let n = 1000;
		let measures = run(&built, n, None).unwrap();
		assert_eq!(measures.len(), BENCHMARKS.len());
		for measure in &measures {
			assert_eq!(measure.result, expected(&measure.name, n), "{measure:?}");
			let allocations = match measure.name.as_str() {
				// one heap frame (or state machine), switched n times
				"gen_opaque" | "sm_direct" | "sm_indirect" => Some(1),
				// a frame per child, plus the parent's (and its array)
				"task_yield" => Some(n as u64 + 1),
				"task_spawn" => Some(n as u64 + 2),
				// coro-elide: the generator's frame goes on the stack
				"gen_inline" | "sm_inline" if optimized => Some(0),
				// coro-elide: the children that never suspend are inlined
				"task_sync" if optimized => Some(1),
				_ => None,
			};
			if let Some(allocations) = allocations {
				assert_eq!(measure.allocations, allocations, "{measure:?}");
			}
		}
	}

	#[test]
	fn coroutines_run_natively() {
		assert_runs(SpikeTarget::Native, "default<O3>", true);
		assert_runs(SpikeTarget::Native, "default<O0>", false);
	}

	#[test]
	fn coroutines_run_on_wasm() {
		if !wasm_available() {
			eprintln!("skipped: needs clang in the LLVM install and node");
			return;
		}
		assert_runs(SpikeTarget::Wasm, "default<O3>", true);
	}
}
