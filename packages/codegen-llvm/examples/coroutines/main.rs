//! The coroutine spike (`design/async-spike.md`): builds the LLVM coroutines
//! and state machines of `spike.rs` for native and wasm32, and prints their
//! frame sizes, code sizes and cost per switch.
//!
//! ```bash
//! cargo run -p fa-codegen-llvm --release --example coroutines [-- <switches>]
//! ```
//!
//! The generators switch `<switches>` times (default 10^8), the tasks await
//! a tenth as many children.

#[cfg(not(fa_llvm))]
fn main() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

// The crate links `libLLVM` (its build script): link it although `spike`
// only uses inkwell and llvm-sys.
#[cfg(fa_llvm)]
extern crate fa_codegen_llvm;

#[cfg(fa_llvm)]
#[allow(dead_code)]
mod spike;

#[cfg(fa_llvm)]
fn main() -> spike::Result<()> {
	use std::path::Path;

	use spike::{BENCHMARKS, SpikeTarget, build, expected, run, wasm_available};

	let switches: i64 = std::env::args()
		.nth(1)
		.map(|argument| argument.parse())
		.transpose()?
		.unwrap_or(100_000_000);
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/coroutines-spike");
	let mut targets = vec![SpikeTarget::Native];
	if wasm_available() {
		targets.push(SpikeTarget::Wasm);
	} else {
		eprintln!("wasm skipped: needs clang in the LLVM install and node");
	}

	for pipeline in ["default<O0>", "default<O3>"] {
		let built = build(SpikeTarget::Native, &directory.join("check"), pipeline)?;
		println!(
			"{pipeline}: {} llvm.coro calls left, split functions: {}",
			built.coro_calls_left,
			built.split_functions.join(" ")
		);
	}
	for target in targets {
		let built = build(target, &directory.join(target.name()), "default<O3>")?;
		println!("\n## {} (default<O3>)\n", target.name());
		println!("frames (bytes): {:?}", built.frame_sizes);
		println!("llvm.coro calls left: {}", built.coro_calls_left);
		println!("code (bytes):");
		for (name, size) in &built.code_sizes {
			println!("  {name}: {size}");
		}
		println!("\n| benchmark | n | allocations | ns per switch or await |");
		println!("| --- | ---: | ---: | ---: |");
		for name in BENCHMARKS {
			let n = if name.starts_with("task") {
				switches / 10
			} else {
				switches
			};
			let mut best: Option<spike::Measure> = None;
			for _ in 0..3 {
				let measure = run(&built, n, Some(name))?.pop().ok_or("no output")?;
				if measure.result != expected(name, n) {
					return Err(format!("{name}: {} instead of {}", measure.result, expected(name, n)).into());
				}
				if best
					.as_ref()
					.is_none_or(|best| measure.nanoseconds < best.nanoseconds)
				{
					best = Some(measure);
				}
			}
			let best = best.unwrap();
			println!(
				"| {name} | {} | {} | {:.3} |",
				best.n,
				best.allocations,
				best.nanoseconds / best.n as f64
			);
		}
	}
	Ok(())
}
