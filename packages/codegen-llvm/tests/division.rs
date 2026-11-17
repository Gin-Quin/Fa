//! Execute the guarded paths without optimization as well as in the O3 corpus.
use std::path::Path;

use fa_codegen_llvm::{OptLevel, Options, Target};

#[test]
fn constant_remainders_at_o0() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-division-o0");
	std::fs::create_dir_all(&output).unwrap();
	for target in [Target::Native, Target::Wasm, Target::NativeFreestanding] {
		for (case, source, expected) in [
			(
				"portable",
				"../cli/tests/programs/constant-remainder.fa",
				"constant remainder ok",
			),
			(
				"wide",
				"tests/programs/constant-remainder-wide.fa",
				"constant remainder wide ok",
			),
			(
				"async",
				"tests/programs/async-constant-remainder.fa",
				"async constant remainder ok",
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
			assert_eq!(result.stdout, format!("{expected}\n").as_bytes());
			assert!(result.stderr.is_empty(), "{case} {target:?}: {result:?}");
		}
	}
}
