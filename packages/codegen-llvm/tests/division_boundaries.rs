//! The same arithmetic is evaluated by the compile-time interpreter and by
//! Bun (the CLI corpus), then by every LLVM target with checks on and off.
use std::path::Path;

use fa_codegen_llvm::{OptLevel, Options, Target};

#[test]
fn division_boundaries_at_every_width_and_optimization_level() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let programs = manifest.join("../cli/tests/programs");
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-division-boundaries");
	for name in [
		"Int8", "Int16", "Int32", "Int64", "Int128", "UInt8", "UInt16", "UInt32", "UInt64",
		"UInt128", "Integer",
	] {
		let source = programs.join(format!("division-boundaries-{}.fa", name.to_lowercase()));
		for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
			for opt_level in [OptLevel::O0, OptLevel::O3] {
				for overflow_checks in [false, true] {
					let case = format!("{name}-{}-{opt_level:?}-{overflow_checks}", target.name());
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
					assert_eq!(
						result.stdout,
						format!("division {name} ok\n").as_bytes(),
						"{case}"
					);
					assert!(result.stderr.is_empty(), "{case}: {result:?}");
				}
			}
		}
	}
}
