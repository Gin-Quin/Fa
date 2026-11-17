//! Exact extents and the unproven generic/async cases execute with leak checks at O0.
use std::path::Path;

use fa_codegen_llvm::{OptLevel, Options, Target};

#[test]
fn array_extents_at_o0() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-array-extents-o0");
	for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
		for (case, source, expected) in [
			(
				"portable",
				"../cli/tests/programs/array-extents.fa",
				"array extents ok\n",
			),
			(
				"async",
				"tests/programs/async-array-extents.fa",
				"async array extents ok\n",
			),
		] {
			let artifact = fa_codegen_llvm::compile_file(
				&manifest.join(source),
				target,
				&Options {
					opt_level: OptLevel::O0,
					leak_check: true,
					asynchronous: (case != "async").then_some(false),
					..Options::default()
				},
				&output.join(format!("{case}-{}", target.name())),
			)
			.unwrap_or_else(|error| panic!("{case} {target:?}: {error}"));
			let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
			assert!(result.status.success(), "{case} {target:?}: {result:?}");
			assert_eq!(result.stdout, expected.as_bytes());
			assert!(result.stderr.is_empty(), "{case} {target:?}: {result:?}");
		}
	}
}
