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
		ir_with(name, source, true)
	}

	/// The IR with or without the overflow checks (`--overflow-checks=false`).
	fn ir_with(name: &str, source: &str, overflow_checks: bool) -> String {
		ir_at(name, source, overflow_checks, OptLevel::O0)
	}

	/// The optimized IR (`O3`, the default of `fa build`, which wraps).
	fn optimized_ir(name: &str, source: &str) -> String {
		ir_at(name, source, false, OptLevel::O3)
	}

	fn ir_at(name: &str, source: &str, overflow_checks: bool, opt_level: OptLevel) -> String {
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(source));
		session.analyze_file(file);
		let options = Options {
			opt_level,
			emit_ir: true,
			overflow_checks,
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
		// bounded by the storage of its operands: computed in 16 bits
		assert_eq!(count(&ir, "add nsw i16"), 1, "{ir}");
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

	#[test]
	fn without_overflow_checks_integers_wrap() {
		let source = "function fib = (n: Integer): Integer => if n < 2 { n } else { fib(n - 1) + fib(n - 2) }\nfunction add = (a: Int8, b: Int8): Int8 => a + b\nfunction product = (a: Integer, b: Integer): Integer => a * b\nconsole.log(fib(30), add(1, 2), product(6, 7))\n";
		let checked = ir_of("checked", source);
		assert_eq!(count(&checked, "with.overflow"), 2, "{checked}");
		assert_eq!(count(&checked, "call void @fa_overflow("), 3, "{checked}");
		let unchecked = ir_with("unchecked", source, false);
		assert_eq!(count(&unchecked, "with.overflow"), 0, "{unchecked}");
		assert_eq!(count(&unchecked, "@fa_overflow"), 0, "{unchecked}");
		assert_eq!(count(&unchecked, " nsw "), 0, "{unchecked}");
		assert_eq!(count(&unchecked, " nuw "), 0, "{unchecked}");
		// the `Int8` sink truncates
		assert!(unchecked.contains("trunc i64"), "{unchecked}");
	}

	#[test]
	fn without_overflow_checks_the_other_checks_stay() {
		let source = "function divide = (a: Integer, b: Integer): Integer throws DivisionByZero => try a // b\nfunction narrow = (value: Integer): Int8 throws ArithmeticError => try Int8(value)\nconsole.log(divide(7, 2) catch error => 0, narrow(5) catch error => 0)\n";
		let unchecked = ir_with("other-checks", source, false);
		assert_eq!(
			count(&unchecked, "call ptr @fa_error_division_by_zero()"),
			1,
			"{unchecked}"
		);
		assert_eq!(
			count(&unchecked, "call ptr @fa_error_conversion("),
			1,
			"{unchecked}"
		);
		// `MIN // -1` divides by 1 instead of stopping
		assert_eq!(count(&unchecked, "@fa_overflow"), 0, "{unchecked}");
		assert_eq!(count(&unchecked, "select i1"), 2, "{unchecked}");
	}

	/// The text of one function of the IR.
	fn function<'ir>(ir: &'ir str, name: &str) -> &'ir str {
		let start = ir
			.find(&format!("@{name}("))
			.unwrap_or_else(|| panic!("no `{name}` in {ir}"));
		let end = ir[start..].find("\n}").map_or(ir.len(), |end| start + end);
		&ir[start..end]
	}

	#[test]
	fn a_byte_kernel_with_checked_indices_is_vectorized() {
		// `blur`: the checks `0 <= i + k < length` leave the main loop
		// (`irce`), and the sums of bytes are computed on 16-bit lanes
		let ir = optimized_ir(
			"blur",
			"function blur = (source: Array(UInt8), destination: Array(UInt8), width: Integer) throws IndexError mutates destination => {\n\tfor y in 1..width - 1 {\n\t\tfor x in 1..width - 1 {\n\t\t\tlet i = y * width + x\n\t\t\tlet sum: Integer = try source[i - width] + source[i - 1] * 2 + source[i] * 4 + source[i + 1] * 2 + source[i + width]\n\t\t\ttry destination[i] = sum // 16\n\t\t}\n\t}\n}\nlet width = arguments.length + 40\nmutable a = Array(UInt8).from([], capacity = width * width)\nmutable b = Array(UInt8).from([], capacity = width * width)\nfor index in 0..width * width {\n\ta.push(index %% 256)\n\tb.push(0)\n}\nfor 0..4 {\n\ttry blur(a, b, width)\n\ttry blur(b, a, width)\n}\nconsole.log(a.get(width + 1) ?? 0)\n",
		);
		let blur = function(&ir, "fa.blur");
		assert!(blur.contains("load <16 x i8>"), "{blur}");
		assert!(blur.contains("<16 x i16>"), "{blur}");
	}

	#[test]
	fn byte_arithmetic_is_computed_in_16_bits() {
		let ir = ir_of(
			"narrow",
			"function mix = (a: UInt8, b: UInt8, c: UInt8): Integer => a + b * 2 + c * 4\nconsole.log(mix(1, 2, 3))\n",
		);
		assert_eq!(count(&ir, "mul nuw i16"), 2, "{ir}");
		assert_eq!(count(&ir, "add nuw i16"), 2, "{ir}");
		assert_eq!(count(&ir, "zext i16"), 3, "{ir}");
	}

	#[test]
	fn ascii_literals_are_simple_strings() {
		// the top bit of the capacity word: one byte per character
		let ir = ir_of(
			"simple",
			"let ascii = \"abc\"\nlet other = \"héllo\"\nconsole.log(ascii.length, other.length)\n",
		);
		assert!(ir.contains("i64 3, i64 -9223372036854775808 }"), "{ir}");
		assert!(ir.contains("i64 6, i64 0 }"), "{ir}");
	}

	#[test]
	fn a_recursive_field_is_boxed_before_its_value_is_computed() {
		// a tree built recursively is laid out in the order of a walk
		let ir = ir_of(
			"early-box",
			"type Node = {\n\tvalue: Integer\n\tleft: Node?\n\n\tfunction build = (depth: Integer): Node => if depth > 0 {\n\t\tNode { value = depth, left = Node.build(depth - 1) }\n\t} else {\n\t\tNode { value = 0 }\n\t}\n}\nconsole.log(Node.build(3).value)\n",
		);
		let build = function(&ir, "fa.build");
		let allocation = build.find("@fa_alloc(").expect("an allocation");
		let recursion = build.find("@fa.build(i64 %").expect("a recursive call");
		assert!(allocation < recursion, "{build}");
	}
}
