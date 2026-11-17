//! Landing contract between array optimizations, UTF-16 values and owner paths.
use std::path::Path;

use fa_codegen_llvm::{OptLevel, Options, Target};

#[test]
fn returned_references_and_utf16_values_survive_array_loops() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let source = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../cli/tests/programs/array-string-references.fa");
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("array-string-references");
	for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
		for opt_level in [OptLevel::O0, OptLevel::O3] {
			for overflow_checks in [false, true] {
				let case = format!("{}-{opt_level:?}-{overflow_checks}", target.name());
				let artifact = fa_codegen_llvm::compile_file(
					&source,
					target,
					&Options {
						opt_level,
						overflow_checks,
						leak_check: true,
						asynchronous: Some(false),
						..Options::default()
					},
					&output.join(&case),
				)
				.unwrap_or_else(|error| panic!("{case}: {error}"));
				let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
				assert!(result.status.success(), "{case}: {result:?}");
				assert_eq!(result.stdout, b"array string references ok\n", "{case}");
				assert!(result.stderr.is_empty(), "{case}: {result:?}");
			}
		}
	}
}
