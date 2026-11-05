//! `CStruct`s by value under the C ABIs other than x86-64 System V: the
//! declarations of `@extern` functions must be the ones clang writes for the
//! same C prototypes (`clang --target=… -S -emit-llvm`, recorded below), for
//! AArch64 (AAPCS64) and wasm32. Only the IR is compared: an AArch64
//! program isn't linked on this host (`FA_NATIVE_TRIPLE` gives the native
//! target another triple), so it is compiled by a child process, which
//! writes the IR and may then stop at code generation.

#[cfg(not(fa_llvm))]
#[test]
fn llvm_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::{Path, PathBuf};
	use std::process::Command;
	use std::sync::Arc;

	use fa_analyzer::session::Session;
	use fa_codegen_llvm::{Options, Target, compile};

	/// The C functions of the test, with the C structs of their parameters
	/// and results: mixed scalars, homogeneous float aggregates of 1 to 3
	/// members, small, padded and large integer structs, an array field.
	const SOURCE: &str = r#"type Point = CStruct {
	x: Int32
	y: Float32
}
type Triple = CStruct {
	a: Float64
	b: Float64
	c: Float64
}
type Pair = CStruct {
	a: Float32
	b: Float32
}
type Color = CStruct {
	rgba: CArray<UInt8, 4>
	weight: Float32
}
type Big = CStruct {
	a: Int64
	b: Int64
	c: Int64
}
type Small = CStruct {
	a: Int16
	b: Int8
}
type Twelve = CStruct {
	a: Int64
	b: Int32
}
type Single = CStruct {
	value: Float64
}

function pointScale: (p: Point, factor: Int32) => Point = @extern("point_scale")
function tripleSum: (t: Triple) => Float64 = @extern("triple_sum")
function tripleMake: (value: Float64) => Triple = @extern("triple_make")
function pairSwap: (p: Pair) => Pair = @extern("pair_swap")
function colorSum: (c: Color) => Int32 = @extern("color_sum")
function colorMake: (value: UInt8) => Color = @extern("color_make")
function bigSum: (b: Big) => Int64 = @extern("big_sum")
function bigMake: (v: Int64) => Big = @extern("big_make")
function smallMake: (a: Int16) => Small = @extern("small_make")
function smallSum: (s: Small) => Int32 = @extern("small_sum")
function twelveMake: (a: Int64) => Twelve = @extern("twelve_make")
function twelveSum: (t: Twelve) => Int64 = @extern("twelve_sum")
function singleMake: (v: Float64) => Single = @extern("single_make")
function singleGet: (s: Single) => Float64 = @extern("single_get")

let p = pointScale(Point { x = 1, y = 2.0 }, 2)
let t = tripleMake(tripleSum(Triple { a = 1.0, b = 2.0, c = 3.0 }))
let q = pairSwap(Pair { a = 1.0, b = 2.0 })
let c = colorMake(UInt8(colorSum(Color { rgba = [1, 2, 3, 4], weight = 1.0 })) catch error => 0)
let b = bigMake(bigSum(Big { a = 1, b = 2, c = 3 }))
let s = smallSum(smallMake(1))
let w = twelveSum(twelveMake(1))
let g = singleGet(singleMake(1.0))
console.log(p.x, t.a, q.a, c.weight, b.a, s, w, g)
"#;

	/// What clang 18 declares for AArch64, without the attributes that
	/// don't change the ABI (`noundef`, `alignstack`, `align`, the struct
	/// names): `(name, declaration)`.
	const AARCH64: &[(&str, &str)] = &[
		("point_scale", "i64 @point_scale(i64, i32"),
		("triple_sum", "double @triple_sum([3 x double]"),
		(
			"triple_make",
			"{ double, double, double } @triple_make(double",
		),
		("pair_swap", "{ float, float } @pair_swap([2 x float]"),
		("color_sum", "i32 @color_sum(i64"),
		("color_make", "i64 @color_make(i8"),
		("big_sum", "i64 @big_sum(ptr"),
		(
			"big_make",
			"void @big_make(ptr sret({ i64, i64, i64 }), i64",
		),
		("small_make", "i32 @small_make(i16"),
		("small_sum", "i32 @small_sum(i64"),
		("twelve_make", "[2 x i64] @twelve_make(i64"),
		("twelve_sum", "i64 @twelve_sum([2 x i64]"),
		("single_make", "{ double } @single_make(double"),
		("single_get", "double @single_get([1 x double]"),
	];

	/// What clang 18 declares for wasm32.
	const WASM32: &[(&str, &str)] = &[
		(
			"point_scale",
			"void @point_scale(ptr sret({ i32, float }), ptr byval({ i32, float }), i32",
		),
		(
			"triple_sum",
			"double @triple_sum(ptr byval({ double, double, double })",
		),
		(
			"triple_make",
			"void @triple_make(ptr sret({ double, double, double }), double",
		),
		(
			"pair_swap",
			"void @pair_swap(ptr sret({ float, float }), ptr byval({ float, float })",
		),
		(
			"color_sum",
			"i32 @color_sum(ptr byval({ { i8, i8, i8, i8 }, float })",
		),
		(
			"color_make",
			"void @color_make(ptr sret({ { i8, i8, i8, i8 }, float }), i8",
		),
		("big_sum", "i64 @big_sum(ptr byval({ i64, i64, i64 })"),
		(
			"big_make",
			"void @big_make(ptr sret({ i64, i64, i64 }), i64",
		),
		("small_make", "void @small_make(ptr sret({ i16, i8 }), i16"),
		("small_sum", "i32 @small_sum(ptr byval({ i16, i8 })"),
		(
			"twelve_make",
			"void @twelve_make(ptr sret({ i64, i32 }), i64",
		),
		("twelve_sum", "i64 @twelve_sum(ptr byval({ i64, i32 })"),
		("single_make", "double @single_make(double"),
		("single_get", "double @single_get(double"),
	];

	/// Compiles the program for a target, keeping its IR (the link fails:
	/// the C functions don't exist), and gives the IR.
	fn compile_ir(target: Target, directory: &Path) -> String {
		let mut session = Session::new();
		let file = session.set_source("abi.foreign", Arc::from(SOURCE));
		session.set_target(target.analysis_target());
		session.analyze_file(file);
		let options = Options {
			emit_ir: true,
			..Options::default()
		};
		let output = directory.join("abi");
		let _ = compile(&session, file, "abi.foreign.fa", target, &options, &output);
		let suffix = if target == Target::Wasm {
			"abi.wasm.ll"
		} else {
			"abi.ll"
		};
		std::fs::read_to_string(directory.join(suffix))
			.unwrap_or_else(|error| panic!("no IR in {}: {error}", directory.display()))
	}

	/// Checks the declarations of the C functions against clang's.
	fn check_declarations(ir: &str, expected: &[(&str, &str)]) {
		for (name, declaration) in expected {
			let line = ir
				.lines()
				.find(|line| line.starts_with("declare ") && line.contains(&format!("@{name}(")))
				.unwrap_or_else(|| panic!("no declaration of `{name}`"));
			// without the extension attributes clang leaves out on AArch64
			let line = line.replace(" zeroext", "").replace(" signext", "");
			assert!(
				line.contains(declaration),
				"`{name}`: expected `{declaration}`, got `{line}`"
			);
		}
	}

	fn scratch(name: &str) -> PathBuf {
		let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
		let _ = std::fs::remove_dir_all(&directory);
		std::fs::create_dir_all(&directory).expect("a scratch directory");
		directory
	}

	#[test]
	fn structs_by_value_follow_the_wasm32_c_abi() {
		let ir = compile_ir(Target::Wasm, &scratch("fa-abi-wasm"));
		check_declarations(&ir, WASM32);
	}

	/// The AArch64 IR, from a child process (`FA_ABI_CHILD`): the runtime's
	/// IR is x86-64's, which LLVM can't select instructions for there.
	#[test]
	fn structs_by_value_follow_aapcs64() {
		let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-abi-aarch64");
		if std::env::var_os("FA_ABI_CHILD").is_some() {
			compile_ir(Target::Native, &directory);
			return;
		}
		let _ = std::fs::remove_dir_all(&directory);
		std::fs::create_dir_all(&directory).expect("a scratch directory");
		let _ = Command::new(std::env::current_exe().expect("the test binary"))
			.args([
				"--exact",
				"with_llvm::structs_by_value_follow_aapcs64",
				"--nocapture",
			])
			.env("FA_ABI_CHILD", "1")
			.env("FA_NATIVE_TRIPLE", "aarch64-unknown-linux-gnu")
			.output()
			.expect("the child test");
		let ir = std::fs::read_to_string(directory.join("abi.ll")).expect("the AArch64 IR");
		assert!(ir.contains("target triple = \"aarch64"), "not AArch64 IR");
		check_declarations(&ir, AARCH64);
	}
}
