//! The runtime checks in the IR are exactly the ones the analyzer's bits ask
//! for (design/analyzer-design.md §3.6): each program is compiled at `O0`
//! (no pass but dead-code removal) and its IR is searched.

#[cfg(not(fa_llvm))]
#[test]
fn llvm_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::PathBuf;
	use std::sync::Arc;

	use fa_analyzer::session::Session;
	use fa_codegen_llvm::{OptLevel, Options, Target, compile};

	/// The IR of the Fa functions and of `main` (the runtime functions start with `fa_`).
	fn ir_of(name: &str, source: &str) -> String {
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(source));
		session.analyze_file(file);
		let options = Options {
			opt_level: OptLevel::O0,
			emit_ir: true,
			..Options::default()
		};
		let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join(name);
		let artifact = compile(&session, file, "test.fa", Target::Native, &options, &output)
			.unwrap_or_else(|error| panic!("{name}: {error}"));
		let ir = std::fs::read_to_string(artifact.ir.expect("the IR")).expect("the IR file");
		// keep the program's own functions
		let mut kept = String::new();
		let mut keep = false;
		for line in ir.lines() {
			if line.starts_with("define ") {
				keep = line.contains("@main(") || line.contains("@fa.");
			}
			if keep {
				kept.push_str(line);
				kept.push('\n');
			}
			if line == "}" {
				keep = false;
			}
		}
		kept
	}

	fn count(ir: &str, pattern: &str) -> usize {
		ir.matches(pattern).count()
	}

	#[test]
	fn an_unproven_sum_is_checked_and_proven_differences_are_nsw() {
		let ir = ir_of(
			"fib",
			"function fib = (n: Integer): Integer => if n < 2 { n } else { fib(n - 1) + fib(n - 2) }\nconsole.log(fib(30))\n",
		);
		assert_eq!(
			count(&ir, "call { i64, i1 } @llvm.sadd.with.overflow.i64"),
			1,
			"{ir}"
		);
		assert_eq!(count(&ir, "@llvm.ssub.with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "sub nsw i64"), 2, "{ir}");
	}

	#[test]
	fn a_sum_of_int8_into_int16_has_no_check() {
		let ir = ir_of(
			"no-check",
			"function widen = (a: Int8, b: Int8): Int16 => a + b\nconsole.log(widen(100, 100))\n",
		);
		assert_eq!(count(&ir, "with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "@fa_overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "add nsw i64"), 1, "{ir}");
	}

	#[test]
	fn a_sum_of_int8_into_int8_is_checked_at_its_sink_only() {
		let ir = ir_of(
			"sink",
			"function add = (a: Int8, b: Int8): Int8 => a + b\nconsole.log(add(100, 27))\n",
		);
		assert_eq!(count(&ir, "with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "call void @fa_overflow("), 1, "{ir}");
		assert_eq!(count(&ir, "icmp slt i64"), 1, "{ir}");
		assert_eq!(count(&ir, "icmp sgt i64"), 1, "{ir}");
	}

	#[test]
	fn a_product_of_integers_is_checked_once() {
		let ir = ir_of(
			"product",
			"function product = (a: Integer, b: Integer): Integer => a * b\nconsole.log(product(6, 7))\n",
		);
		assert_eq!(count(&ir, "@llvm.smul.with.overflow.i64("), 1, "{ir}");
		assert_eq!(count(&ir, "call void @fa_overflow("), 1, "{ir}");
	}

	#[test]
	fn unsigned_64_bit_operands_compute_in_128_bits() {
		let ir = ir_of(
			"uint64",
			"function add = (a: UInt64, b: UInt64): UInt64 => a + b\nconsole.log(add(1, 2))\n",
		);
		assert_eq!(count(&ir, "with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "add nsw i128"), 1, "{ir}");
		assert_eq!(count(&ir, "call void @fa_overflow("), 1, "{ir}");
	}

	#[test]
	fn divisions_check_the_divisor_and_the_minimum_only_when_needed() {
		let ir = ir_of(
			"divisions",
			"function half = (a: Integer): Integer => a // 2\nfunction divide = (a: Integer, b: Integer): Integer throws DivisionByZero => try a // b\nfunction rest = (a: Int8, b: Int8): Int8 throws DivisionByZero => try a %% b\nconsole.log(half(7), divide(7, 2) catch error => 0, rest(7, 2) catch error => 0)\n",
		);
		assert_eq!(
			count(&ir, "call ptr @fa_error_division_by_zero()"),
			2,
			"{ir}"
		);
		// `Integer.minValue // -1` overflows; the `Int8` remainder computes in 64 bits
		let minimum_guards = ir
			.lines()
			.filter(|line| line.contains("icmp eq i64") && line.ends_with(", -9223372036854775808"))
			.count();
		assert_eq!(minimum_guards, 1, "{ir}");
		assert_eq!(count(&ir, "sdiv i64"), 3, "{ir}");
	}

	#[test]
	fn an_unproven_conversion_is_checked() {
		let ir = ir_of(
			"conversion",
			"function narrow = (value: Integer): Int8 throws ArithmeticError => try Int8(value)\nfunction small = (value: Int8): Int16 => Int16(value)\nconsole.log(narrow(5) catch error => 0, small(5))\n",
		);
		assert_eq!(count(&ir, "call ptr @fa_error_conversion("), 1, "{ir}");
	}
}
