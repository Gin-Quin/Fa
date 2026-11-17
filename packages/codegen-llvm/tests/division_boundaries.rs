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
		"Int8",
		"Int16",
		"Int32",
		"Int64",
		"Int128",
		"UInt8",
		"UInt16",
		"UInt32",
		"UInt64",
		"UInt128",
		"Integer",
		"IntegerNative",
		"Byte",
		"BigInt",
	] {
		let source = if name == "IntegerNative" {
			manifest.join("tests/programs/division-boundaries-integer-native.fa")
		} else {
			programs.join(format!("division-boundaries-{}.fa", name.to_lowercase()))
		};
		let printed_name = if name == "IntegerNative" {
			"Integer native"
		} else {
			name
		};
		for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
			for opt_level in [OptLevel::O0, OptLevel::O3] {
				for overflow_checks in [false, true] {
					let case = format!("{name}-{}-{opt_level:?}-{overflow_checks}", target.name());
					assert_program(
						&source,
						target,
						opt_level,
						overflow_checks,
						&output.join(case),
						&format!("division {printed_name} ok\n"),
					);
				}
			}
		}
	}
}

#[test]
fn minimum_signed_quotients_wrap_when_checks_are_disabled() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let programs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cli/tests/programs");
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-division-minimum-wrapping");
	for (name, minimum) in [
		("Int8", "-128"),
		("Int16", "-32768"),
		("Int32", "-2147483648"),
		("Int64", "-9223372036854775808"),
		("Int128", "-170141183460469231731687303715884105728"),
	] {
		for form in ["literal", "dynamic"] {
			let source = programs.join(format!(
				"division-minimum-{}-{form}.fa",
				name.to_lowercase()
			));
			for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
				for opt_level in [OptLevel::O0, OptLevel::O3] {
					let case = format!("{name}-{form}-{}-{opt_level:?}", target.name());
					assert_program(
						&source,
						target,
						opt_level,
						false,
						&output.join(case),
						&format!("before overflow\n{minimum}\n"),
					);
				}
			}
		}
	}
}

fn assert_program(
	source: &Path,
	target: Target,
	opt_level: OptLevel,
	overflow_checks: bool,
	output: &Path,
	expected: &str,
) {
	let case = output.display();
	eprintln!("division boundary {case}");
	let artifact = fa_codegen_llvm::compile_file(
		source,
		target,
		&Options {
			opt_level,
			overflow_checks,
			leak_check: true,
			asynchronous: Some(false),
			..Options::default()
		},
		output,
	)
	.unwrap_or_else(|error| panic!("{case}: {error}"));
	let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
	assert!(result.status.success(), "{case}: {result:?}");
	assert_eq!(result.stdout, expected.as_bytes(), "{case}");
	assert!(result.stderr.is_empty(), "{case}: {result:?}");
}
