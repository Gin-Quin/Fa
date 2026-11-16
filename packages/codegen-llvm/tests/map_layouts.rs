//! O0 complements the shared O3 program corpus, including freestanding.
use std::path::Path;

use fa_codegen_llvm::{OptLevel, Options, Target};

#[test]
fn generic_map_layouts_at_o0() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-map-layouts-o0");
	std::fs::create_dir_all(&output).unwrap();
	for target in [Target::Native, Target::Wasm, Target::NativeFreestanding] {
		for (case, expected) in [
			("delete-direct", "generic delete ok"),
			("delete", "generic delete ok"),
			("reference", "generic reference ok"),
			("operations", "generic operations ok"),
			("helper-layouts", "generic helpers ok"),
			("async", "async generic maps ok"),
		] {
			let source = if case == "async" {
				manifest.join("tests/programs/async-map-generic-layout.fa")
			} else {
				manifest.join(format!("../cli/tests/programs/map-generic-{case}.fa"))
			};
			let artifact = fa_codegen_llvm::compile_file(
				&source,
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
