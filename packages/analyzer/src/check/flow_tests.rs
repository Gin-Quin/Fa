//! Unit tests of numbers, conditions and loops (`numbers.rs`, `ops.rs`,
//! `control.rs`): diagnostics, check bits, arithmetic info and coercions.
//!
//! Until functions are analyzed, unknown values come from loops: `for i in
//! 0..` gives `i` the values `[0, Integer.max]`, and a conversion `Int8(i)`
//! of a wide value gives a value spanning `Int8`.

use crate::analysis::{ArithInfo, NodeFlags};
use crate::diagnostics::LineIndex;
use crate::ids::{FileId, NodeId, TypeId};
use crate::session::Session;
use crate::types::{ComputeType, Interval};
use crate::{Severity, analyze_source};

struct Checked {
	session: Session,
	file: FileId,
	text: String,
}

fn analyze(source: &str) -> Checked {
	let (session, file) = analyze_source(source);
	Checked {
		session,
		file,
		text: source.to_string(),
	}
}

impl Checked {
	/// The codes emitted, in order.
	fn codes(&self) -> Vec<&'static str> {
		self.session
			.diagnostics(self.file)
			.iter()
			.map(|diagnostic| diagnostic.code.id)
			.collect()
	}

	/// The codes emitted on a line (1-based).
	fn codes_on(&self, line: u32) -> Vec<&'static str> {
		let lines = LineIndex::new(&self.text);
		self.session
			.diagnostics(self.file)
			.iter()
			.filter(|diagnostic| diagnostic.severity != Severity::Hint)
			.filter(|diagnostic| lines.line(diagnostic.location.span.start) == line)
			.map(|diagnostic| diagnostic.code.id)
			.collect()
	}

	/// The outermost node whose text is exactly `text`.
	fn outer(&self, text: &str) -> NodeId {
		let tree = self.session.tree(self.file);
		(0..tree.nodes.len())
			.filter(|&node| tree.text(node) == text)
			.max_by_key(|&node| tree.spans[node].end - tree.spans[node].start)
			.map(NodeId::new)
			.unwrap_or_else(|| panic!("no node `{text}`"))
	}

	fn flags(&self, text: &str) -> NodeFlags {
		self.session.analysis(self.file).flags_of(self.outer(text))
	}

	/// The check bits of every node on a line.
	fn checks_on(&self, line: u32) -> NodeFlags {
		let lines = LineIndex::new(&self.text);
		let tree = self.session.tree(self.file);
		let analysis = self.session.analysis(self.file);
		(0..tree.nodes.len())
			.filter(|&node| lines.line(tree.spans[node].start as u32) == line)
			.fold(NodeFlags::empty(), |flags, node| {
				flags | analysis.flags_of(NodeId::new(node))
			}) & NodeFlags::CHECKS
	}

	fn arith(&self, text: &str) -> Option<ArithInfo> {
		self.session.analysis(self.file).arith(self.outer(text))
	}

	fn coercion(&self, text: &str) -> Option<String> {
		self.session
			.analysis(self.file)
			.coercions
			.get(&self.outer(text))
			.map(|&ty| self.session.display_type(ty))
	}

	fn type_of(&self, text: &str) -> String {
		let ty = self.session.analysis(self.file).type_of(self.outer(text));
		self.session.display_type(ty)
	}

	/// The type of the symbol declared with this name (the last one).
	fn symbol_type(&self, name: &str) -> String {
		let analysis = self.session.analysis(self.file);
		let symbol = analysis
			.symbols
			.symbols()
			.iter()
			.rev()
			.find(|symbol| self.session.types.syms.resolve(symbol.name) == name)
			.unwrap_or_else(|| panic!("no symbol `{name}`"));
		self.session.display_type(symbol.ty)
	}
}

fn codes(source: &str) -> Vec<&'static str> {
	analyze(source).codes()
}

fn i(lo: i128, hi: i128) -> Interval {
	Interval::from_i128(lo, hi)
}

/// Unknown `Int8` values `a` and `b` (and `u: UInt8`) inside a loop body.
const UNKNOWN_INT8: &str = "for i in -1000..1000 {\n\tlet a = try Int8(i)\n\tlet b = try Int8(i)\n\tlet u = try UInt8(i)\n";

// ── golden flag tests (values brief T-NUM-11, 12, 13, 14, 17) ─────────────

#[test]
fn golden_wide_sinks_and_floor_division_need_no_check() {
	// T-NUM-11 and T-NUM-12 with unknown operands
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tlet wide: Int16 = a + b\n\tlet avg: Int8 = (b + a) // 2\n}}\n"
	));
	assert_eq!(c.codes_on(5), Vec::<&str>::new());
	assert_eq!(c.checks_on(5), NodeFlags::empty(), "Int16 = a + b");
	assert_eq!(c.checks_on(6), NodeFlags::empty(), "Int8 = (a + b) // 2");
	assert_eq!(c.arith("b + a").map(|info| info.root), Some(false));
	assert_eq!(c.arith("a + b").map(|info| info.root), Some(true));
	assert_eq!(c.arith("(b + a) // 2").unwrap().interval, i(-128, 127));
	assert_eq!(c.arith("(b + a) // 2").unwrap().compute, ComputeType::Int64);
	assert_eq!(
		c.coercion("a + b"),
		Some("Int16".into()),
		"a widening coercion"
	);
}

#[test]
fn golden_narrow_sink_of_an_arithmetic_root_is_checked() {
	// T-NUM-13 / T-NUM-14: `Int8 + Int8` into an `Int8`, annotated or not
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tlet d: Int8 = a + b\n\tlet s = a - b\n}}\n"
	));
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.checks_on(5), NodeFlags::SINK_CHECK);
	assert_eq!(c.coercion("a + b"), Some("Int8".into()));
	assert_eq!(
		c.checks_on(6),
		NodeFlags::SINK_CHECK,
		"unannotated: base type Int8"
	);
	assert_eq!(c.coercion("a - b"), Some("Int8".into()));
	let info = c.arith("a + b").unwrap();
	assert_eq!(info.interval, i(-256, 254));
	assert!(info.root);
}

#[test]
fn golden_ranged_constant_product_needs_no_check() {
	// T-NUM-17 without ranged types: the fact of `small` proves the product
	let c = analyze("let small: Int16 = 87\nlet bounded: Int16 = small * 2\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.checks_on(2), NodeFlags::empty());
	assert_eq!(c.arith("small * 2").unwrap().interval, i(174, 174));
}

// ── constants and sinks ───────────────────────────────────────────────────

#[test]
fn certain_overflows_and_literals() {
	assert_eq!(codes("let e: Int8 = 100 + 100\n"), ["E-NUM-10"]);
	assert_eq!(codes("let a: UInt8 = 200 + 100\n"), ["E-NUM-10"]);
	assert_eq!(codes("let x: UInt8 = 255\nlet y = x + 1\n"), ["E-NUM-10"]);
	assert_eq!(codes("let small: UInt8 = 300\n"), ["E-NUM-6"]);
	assert_eq!(
		codes("let huge = 10_000_000_000_000_000_000\n"),
		["E-NUM-7"]
	);
	assert_eq!(codes("let fine: Int8 = 100 + 27\n"), Vec::<&str>::new());
	assert_eq!(codes("let lowest: Int8 = -128\n"), Vec::<&str>::new());
	let c = analyze("let x = 100 + 27\n");
	assert!(c.flags("100 + 27").contains(NodeFlags::CONST_FOLDED));
	assert_eq!(c.symbol_type("x"), "Integer");
}

#[test]
fn non_arithmetic_values_need_a_proof() {
	let c = analyze(&format!("{UNKNOWN_INT8}\tlet byte: UInt8 = a\n}}\n"));
	assert_eq!(c.codes_on(5), ["E-NUM-14"]);
	// a fact proves it
	assert_eq!(
		codes("let a: Int8 = 5\nlet byte: UInt8 = a\n"),
		Vec::<&str>::new()
	);
	// a value certainly outside
	assert_eq!(
		codes("let a: Int8 = -5\nlet byte: UInt8 = a\n"),
		["E-NUM-10"]
	);
}

#[test]
fn literal_operands_contribute_their_value() {
	// values brief D-94: `a + 1000` with `a: Int8` has the base type `Int8`
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tlet s = a + 1000\n\tlet t: Integer = a + 1000\n}}\n"
	));
	assert_eq!(c.codes_on(5), ["E-NUM-10"]);
	assert_eq!(c.codes_on(6), Vec::<&str>::new());
	assert_eq!(c.arith("a + 1000").unwrap().interval, i(872, 1127));
	assert_eq!(c.type_of("a + 1000"), "Int8");
}

#[test]
fn base_types_of_mixed_storages() {
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tlet s = a + u\n\tlet t: Int16 = a + u\n\tlet w = a + Int32(i)\n}}\n"
	));
	assert_eq!(c.codes_on(5), ["E-NUM-15"]);
	assert_eq!(c.codes_on(6), Vec::<&str>::new());
	assert_eq!(c.checks_on(6), NodeFlags::empty());
	assert_eq!(c.codes_on(7), Vec::<&str>::new());
	assert_eq!(c.symbol_type("w"), "Int32");
}

// ── operators ─────────────────────────────────────────────────────────────

#[test]
fn kinds_do_not_mix() {
	assert_eq!(
		codes("let count = 3\nlet ratio = 0.5\nlet r = count * ratio\n"),
		["E-NUM-11"]
	);
	assert_eq!(
		codes("let a: Float32 = 1.0\nlet b: Float = 2.0\nlet c = a + b\n"),
		["E-NUM-11"]
	);
	assert_eq!(codes("let n = 3\nlet p = n * 50%\n"), ["E-NUM-11"]);
	assert_eq!(codes("let q = 50% * 2\n"), ["E-NUM-11"]);
	assert_eq!(codes("let r = 50% + 0.5\n"), ["E-NUM-11"]);
	assert_eq!(codes("let t = true + 1\n"), ["E-NUM-18"]);
	assert_eq!(codes("let t = -true\n"), ["E-NUM-18"]);
	// a literal takes the kind of the other operand
	let c = analyze("let ratio = 0.5\nlet r = ratio * 2\nlet h = 1 / 2\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("r"), "Float");
	assert_eq!(c.symbol_type("h"), "Float");
	let c = analyze("let s: Float32 = 0.1\nlet t = s * 2\nlet p = 20% + 30%\nlet f = 40.0 * 50%\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("t"), "Float32");
	assert_eq!(c.symbol_type("p"), "Percentage");
	assert_eq!(c.symbol_type("f"), "Float");
	assert_eq!(
		codes("let big = 12n\nlet t: BigInt = 12\nlet s = big + t + 1\n"),
		Vec::<&str>::new()
	);
	assert_eq!(codes("let f: Float32 = 1e39\n"), ["E-NUM-8"]);
	assert_eq!(codes("let g = 1e400\n"), ["E-NUM-8"]);
}

#[test]
fn division_by_zero() {
	assert_eq!(codes("let x = 5 // 0\n"), ["E-NUM-12"]);
	assert_eq!(codes("let y = 5 %% 0\n"), ["E-NUM-12"]);
	assert_eq!(codes("let z = 5n // 0n\n"), ["E-NUM-12"]);
	let c = analyze(
		"for i in 0.. {\n\tlet d = try Int8(i)\n\tlet q = try 10 // d\n\tif d != 0 {\n\t\tlet r = 10 // d\n\t}\n\tlet s = 10 // 3\n}\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.checks_on(3), NodeFlags::DIVISOR_CHECK);
	assert!(
		c.flags("10 // d").contains(NodeFlags::CAN_FAIL)
			|| c.checks_on(3).contains(NodeFlags::DIVISOR_CHECK)
	);
	assert_eq!(c.checks_on(5), NodeFlags::empty(), "narrowed by `d != 0`");
	assert_eq!(c.checks_on(7), NodeFlags::empty());
	// a divisor whose type excludes zero (a loop variable from 1)
	let c = analyze("for d in 1..10 {\n\tlet q = 100 // d\n}\n");
	assert_eq!(c.checks_on(2), NodeFlags::empty());
	assert_eq!(c.arith("100 // d").unwrap().interval, i(11, 100));
}

#[test]
fn minimum_divided_by_minus_one() {
	// values brief D-49: `x // -1` and `x %% -1` when `x` may be the minimum
	let c = analyze(
		"for i in 0.. {\n\tlet x = -i - 1\n\tlet q: Integer = x // -1\n\tlet r = x %% -1\n}\n",
	);
	assert!(c.flags("x // -1").contains(NodeFlags::MIN_DIV_GUARD));
	assert!(c.flags("x // -1").contains(NodeFlags::OVERFLOW_NATIVE));
	assert!(c.flags("x %% -1").contains(NodeFlags::MIN_DIV_GUARD));
	assert!(!c.flags("x %% -1").contains(NodeFlags::OVERFLOW_NATIVE));
	assert_eq!(c.arith("x %% -1").unwrap().interval, i(0, 0));
	assert_eq!(
		c.arith("-i - 1").unwrap().interval.lo,
		i(i64::MIN as i128, 0).lo
	);
}

#[test]
fn exponents() {
	assert_eq!(codes("let x = 2 ** -1\n"), ["E-NUM-10"]);
	let c = analyze("for e in -3..3 {\n\tlet p = 2 ** e\n}\n");
	assert_eq!(c.checks_on(2), NodeFlags::EXPONENT_CHECK);
	assert!(c.flags("2 ** e").contains(NodeFlags::CAN_FAIL));
	assert_eq!(c.arith("2 ** e").unwrap().interval, i(1, 4));
	// right-associative: 2 ** 3 ** 2 = 512
	let c = analyze("let x = 2 ** 3 ** 2\n");
	assert_eq!(c.arith("2 ** 3 ** 2").unwrap().interval, i(512, 512));
}

#[test]
fn overflow_bits_follow_the_compute_type() {
	// beyond JavaScript's safe integers, within Int64: only `OVERFLOW_JS`
	let c = analyze("let big: Integer = 2 ** 60\n");
	assert_eq!(
		c.flags("2 ** 60") & NodeFlags::CHECKS,
		NodeFlags::OVERFLOW_JS
	);
	// `Int64` is a JavaScript `bigint`: no JS check
	let c = analyze("let big: Int64 = 2 ** 60\n");
	assert_eq!(c.flags("2 ** 60") & NodeFlags::CHECKS, NodeFlags::empty());
	// unknown `Integer` operands: a native overflow check, and nothing more at an `Integer` sink
	let c = analyze("for x in 0.. {\n\tlet y: Integer = x * x\n}\n");
	let flags = c.flags("x * x");
	assert!(flags.contains(NodeFlags::OVERFLOW_NATIVE | NodeFlags::OVERFLOW_JS));
	assert!(!flags.contains(NodeFlags::SINK_CHECK));
	// `UInt64` operands compute in `Int128`
	let c = analyze("for x in 0.. {\n\tlet a = try UInt64(x)\n\tlet s: UInt64 = a * a\n}\n");
	let info = c.arith("a * a").unwrap();
	assert_eq!(info.compute, ComputeType::Int128);
	assert!(!c.flags("a * a").contains(NodeFlags::OVERFLOW_NATIVE));
	assert!(c.flags("a * a").contains(NodeFlags::SINK_CHECK));
	assert!(!c.flags("a * a").contains(NodeFlags::OVERFLOW_JS));
}

#[test]
fn negation_and_abs_are_arithmetic() {
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tlet n: Int8 = -a\n\tlet p: Int8 = a.abs()\n\tlet m: Int16 = -a\n\tlet q: UInt8 = -u\n}}\n"
	));
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.checks_on(5), NodeFlags::SINK_CHECK);
	assert_eq!(c.arith("-a").unwrap().interval, i(-127, 128));
	assert_eq!(c.checks_on(6), NodeFlags::SINK_CHECK);
	assert_eq!(c.arith("a.abs()").unwrap().interval, i(0, 128));
	assert_eq!(c.checks_on(7), NodeFlags::empty());
	assert_eq!(c.checks_on(8), NodeFlags::SINK_CHECK, "values brief D-104");
	assert_eq!(codes("let x: UInt8 = 5\nlet y: UInt8 = -x\n"), ["E-NUM-10"]);
}

// ── compound assignments ──────────────────────────────────────────────────

#[test]
fn compound_assignments() {
	let c = analyze(
		"mutable count = 10\ncount += 1\ncount -= 2\ncount *= 3\ncount //= 4\ncount %%= 5\ncount **= 2\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.arith("count //= 4").unwrap().interval, i(6, 6));
	assert_eq!(c.arith("count **= 2").unwrap().interval, i(0, 16));
	assert_eq!(codes("mutable count = 10\ncount /= 4\n"), ["E-NUM-21"]);
	assert_eq!(
		codes("mutable small: Int8 = 100\nsmall += 100\n"),
		["E-NUM-10"]
	);
	assert_eq!(codes("let fixed = 1\nfixed += 1\n"), ["E-DECL-3"]);
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tmutable total: Int8 = a\n\ttotal += b\n}}\n"
	));
	assert_eq!(c.checks_on(6), NodeFlags::SINK_CHECK);
	assert_eq!(c.coercion("total += b"), Some("Int8".into()));
	let c = analyze("mutable ratio = 1.5\nratio += 1\nratio /= 2\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
}

// ── conversions ───────────────────────────────────────────────────────────

#[test]
fn integer_conversions() {
	assert_eq!(codes("let r = Int8(300)\n"), ["E-NUM-10"]);
	assert_eq!(codes("let i = 300\nlet r = Int8(i)\n"), ["E-NUM-10"]);
	let c = analyze("let j: Integer = 3\nlet r = Int8(j)\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.checks_on(2), NodeFlags::empty(), "proven by the fact");
	assert_eq!(c.symbol_type("r"), "Int8");
	let c = analyze("for i in 0..1000 {\n\tlet r = Int8(i)\n}\n");
	assert_eq!(c.checks_on(2), NodeFlags::CONVERSION_CHECK);
	assert!(c.flags("Int8(i)").contains(NodeFlags::CAN_FAIL));
	assert_eq!(codes("let x: Int8 = 1\nlet y = Int16(x)\n"), ["W-NUM-1"]);
	assert_eq!(codes("let b = BigInt(1.5)\n"), ["E-NUM-16"]);
	assert_eq!(codes("let p = (5n)%\n"), ["E-NUM-16"]);
	assert_eq!(
		codes("let f = Float(3)\nlet g = Float(50%)\nlet h = BigInt(3)\n"),
		Vec::<&str>::new()
	);
	assert_eq!(
		codes("let x = Integer(2.5)\nlet y = Int8(1000.0)\n"),
		["E-NUM-10"]
	);
	let c = analyze("let ratio = 0.5\nlet x = Int8(ratio)\n");
	assert_eq!(
		c.checks_on(2),
		NodeFlags::CONVERSION_CHECK,
		"a stop for NaN and out of range"
	);
}

// ── conditions ────────────────────────────────────────────────────────────

#[test]
fn comparisons_narrow_integers() {
	// T-NUM-18
	assert_eq!(
		codes("let a: Int8 = -5\nif a >= 0 { let p: UInt8 = a }\n"),
		Vec::<&str>::new()
	);
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tif a >= 0 {{\n\t\tlet p: UInt8 = a\n\t}} else {{\n\t\tlet n: UInt8 = a\n\t}}\n\tif 0 < a < 100 {{ let small: Int8 = a * 2 }}\n\tif a < 0 or a > 10 {{ }} else {{ let q: UInt8 = a }}\n\tif not (a < 0) {{ let r: UInt8 = a }}\n}}\n"
	));
	assert_eq!(c.codes_on(6), Vec::<&str>::new());
	assert_eq!(c.codes_on(8), ["E-NUM-10"]);
	assert_eq!(c.checks_on(10), NodeFlags::SINK_CHECK, "a * 2 in [2, 198]");
	assert_eq!(c.codes_on(11), Vec::<&str>::new());
	assert_eq!(c.codes_on(12), Vec::<&str>::new());
	// equality with a literal
	assert_eq!(
		codes("for i in 0..1000 {\n\tif i == 5 { let b: UInt8 = i }\n}\n"),
		Vec::<&str>::new()
	);
}

#[test]
fn constant_conditions() {
	// values brief T-NUM-52: decided by the types
	assert_eq!(codes("let a: UInt8 = 5\nif a < 0 { }\n"), ["W-NUM-2"]);
	assert_eq!(codes("let a: UInt8 = 5\nif a <= 255 { }\n"), ["W-NUM-2"]);
	// facts of literal initializers don't warn (T-IF-1)
	assert_eq!(
		codes("let t = 35\nif t > 30 { } else if t > 15 { } else { }\n"),
		Vec::<&str>::new()
	);
	assert_eq!(codes("if true { }\n"), ["W-BOOL-1"]);
	assert_eq!(codes("while true { break }\n"), ["W-BOOL-1"]);
	// a value comparison is not a condition
	assert_eq!(
		codes("let a: UInt8 = 5\nlet b = a < 0\n"),
		Vec::<&str>::new()
	);
}

#[test]
fn comparison_and_equality_types() {
	assert_eq!(codes("let x = true < false\n"), ["E-CMP-2"]);
	assert_eq!(codes("let x = (1 < 2) < 3\n"), ["E-CMP-2"]);
	assert_eq!(
		codes("let a = 1\nlet b = 2\nlet c = 3\nlet x = a < b > c\n"),
		["E-CMP-3"]
	);
	assert_eq!(codes("let n = 1\nlet a = n == 1.0\n"), ["E-CMP-1"]);
	assert_eq!(codes("let a = 1 == 1.0\n"), Vec::<&str>::new());
	assert_eq!(
		codes("let a: Int8 = 5\nlet b: UInt16 = 5\nlet c = a == b and a < b\n"),
		Vec::<&str>::new()
	);
	assert_eq!(
		codes("let x = true == true and true != false\n"),
		Vec::<&str>::new()
	);
	assert_eq!(
		codes("let a: Float32 = 1.0\nlet b: Float = 2.0\nlet d = a == b\n"),
		["E-CMP-1"]
	);
	assert_eq!(codes("let x = 50% > 25%\n"), Vec::<&str>::new());
	assert_eq!(codes("let count = 0\nif count { }\n"), ["E-BOOL-1"]);
	assert_eq!(codes("let count = 0\nlet b = not count\n"), ["E-BOOL-1"]);
	assert_eq!(codes("let x = 1 and true\n"), ["E-BOOL-1"]);
	let c = analyze("let x = 50\nlet y = 0 < x <= 100\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("y"), "Boolean");
}

// ── if ────────────────────────────────────────────────────────────────────

#[test]
fn if_values() {
	assert_eq!(
		codes("let c = true\nlet x = if c { 1 } else { 0.5 }\n"),
		["E-IF-2"]
	);
	assert_eq!(codes("let c = true\nlet x = if c { 1 }\n"), ["E-IF-1"]);
	assert_eq!(
		codes("let c = true\nmutable n = 0\nlet x = if c { n = 1 } else { 2 }\n"),
		["E-IF-3"]
	);
	let c = analyze("let c = true\nlet ratio: Float = if c { 1 } else { 0.5 }\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	// a literal branch adapts to the other one (D-29)
	let c = analyze(&format!(
		"{UNKNOWN_INT8}\tlet c = a > 0\n\tlet x = if c {{ a }} else {{ 1 }}\n\tlet y = if c {{ a }} else {{ u }}\n\tlet z = if c {{ a }} else {{ Int16(i) }}\n}}\n"
	));
	assert_eq!(c.symbol_type("x"), "Int8");
	assert_eq!(c.codes_on(7), ["E-IF-2"]);
	assert_eq!(c.symbol_type("z"), "Int16");
	assert_eq!(c.codes().len(), 1);
	// an `else if` chain as a value, with a `Never` branch
	let c = analyze(
		"for i in 0..10 {\n\tlet x = if i < 3 { 1 } else if i < 6 { 2 } else { continue }\n\tlet y: UInt8 = x\n}\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("x"), "Integer");
}

#[test]
fn facts_join_after_branches() {
	let source = "for i in 0..2 {\n\tmutable x = 0\n\tif i == 0 { x = 10 } else { x = 20 }\n\tlet y: UInt8 = x\n\tlet z: Int8 = x * 10\n}\n";
	let c = analyze(source);
	assert_eq!(c.codes_on(4), Vec::<&str>::new(), "x in [10, 20]");
	assert_eq!(
		c.checks_on(5),
		NodeFlags::SINK_CHECK,
		"x * 10 in [100, 200]"
	);
	// an early exit: the code after continues from the other branch
	let c = analyze("for i in 0..1000 {\n\tif i > 255 { continue }\n\tlet y: UInt8 = i\n}\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
}

// ── loops ─────────────────────────────────────────────────────────────────

#[test]
fn loop_fixpoint_widens_counters() {
	// T-NUM-53: widening keeps the lower bound, `count += 1` keeps a check
	let c = analyze("mutable count = 0\nfor 0..10 { count += 1 }\nlet c: UInt64 = count\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	let flags = c.flags("count += 1");
	assert!(flags.contains(NodeFlags::OVERFLOW_NATIVE));
	assert!(!flags.contains(NodeFlags::SINK_CHECK));
	// a counter bounded by the condition
	let c = analyze(
		"mutable i = 0\nwhile i < 100 {\n\tlet b: UInt8 = i\n\ti += 1\n}\nlet after: UInt8 = i\n",
	);
	assert_eq!(
		c.codes_on(3),
		Vec::<&str>::new(),
		"i in [0, 99] in the body"
	);
	assert_eq!(c.flags("i += 1") & NodeFlags::CHECKS, NodeFlags::empty());
	assert_eq!(c.arith("i += 1").unwrap().interval, i(1, 100));
	// the descending round gives the upper bound back: `i == 100` after the loop
	assert_eq!(c.codes_on(6), Vec::<&str>::new());
	// a decreasing counter
	let c = analyze(
		"mutable remaining = 3\nwhile remaining > 0 {\n\tremaining -= 1\n}\nlet r: UInt8 = remaining\n",
	);
	// `remaining == 0` after the loop
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.arith("remaining -= 1").unwrap().interval, i(0, 2));
	// the review's case: counting up to a bound, then a narrow sink
	let c = analyze(
		"mutable i = 0\nwhile i < 10 { i += 1 }\nlet b: UInt8 = i\nmutable j = 300\nwhile j > 0 { j -= 1 }\nlet d: UInt8 = j\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	// an unbounded counter keeps its widened bound
	let c = analyze(
		"mutable i = 0\nmutable go = true\nwhile go {\n\ti += 1\n\tgo = i != 5\n}\nlet b: UInt8 = i\n",
	);
	assert_eq!(c.codes(), ["E-NUM-14"]);
}

#[test]
fn loop_values_and_jumps_across_functions() {
	// the value of a `loop`: the hull of its `break` values
	let c = analyze(
		"mutable n = 0\nlet x = loop {\n\tn += 1\n\tif n > 3 { break 2 }\n\tif n > 5 { break 7 }\n}\nlet y: UInt8 = x\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	// `break` and `continue` don't cross a lambda or a local function
	let c = analyze("for i in 0..3 {\n\tlet f = [1].map((x) => {\n\t\tbreak\n\t\tx\n\t})\n}\n");
	assert_eq!(c.codes_on(3), ["E-LOOP-13"]);
	let c = analyze("for i in 0..3 {\n\tfunction g = () => {\n\t\tcontinue\n\t}\n\tg()\n}\n");
	assert_eq!(c.codes_on(3), ["E-LOOP-13"]);
	let c = analyze("function g = () => {\n\tbreak\n}\n");
	assert_eq!(c.codes_on(2), ["E-LOOP-12"]);
}

#[test]
fn flow_diagnostics_come_from_the_stable_round() {
	// round 1 sees x = 0 (fits), the stable round x in [0, 300]: one E-NUM-14
	let c = analyze("mutable x = 0\nfor i in 0..10 {\n\tlet y: UInt8 = x\n\tx = 300\n}\n");
	assert_eq!(c.codes(), ["E-NUM-14"]);
	// nested loops
	let c = analyze(
		"mutable x = 0\nfor i in 0..10 {\n\tfor j in 0..10 {\n\t\tlet y: UInt8 = x\n\t\tx = 300\n\t}\n}\n",
	);
	assert_eq!(c.codes(), ["E-NUM-14"]);
}

#[test]
fn for_over_ranges() {
	let c = analyze(
		"for i in 0..256 {\n\tlet b: UInt8 = i\n}\nfor i in 0..=255 { let b: UInt8 = i }\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(
		codes("for i in 0..300 {\n\tlet b: UInt8 = i\n}\n"),
		["E-NUM-14"]
	);
	assert_eq!(codes("for i in 0..3 { i = 1 }\n"), ["E-DECL-3"]);
	assert_eq!(codes("let n = 5\nfor i in n { }\n"), ["E-LOOP-1"]);
	assert_eq!(codes("let x = for i in 0..3 { }\n"), ["E-LOOP-6"]);
	assert_eq!(codes("for i in ..3 { }\n"), ["E-RNG-3"]);
	assert_eq!(
		codes("let x = 5\nfor x in 0..2 { }\nlet y: Int8 = x\n"),
		Vec::<&str>::new()
	);
	let c = analyze("for i in 0..10 { }\n");
	assert_eq!(c.symbol_type("i"), "Integer");
}

#[test]
fn loop_values_and_jumps() {
	let c = analyze(
		"let target = 50\nmutable candidate = 1\nlet root = loop {\n\tif candidate * candidate >= target { break candidate }\n\tcandidate += 1\n}\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("root"), "Integer");
	assert_eq!(
		codes("mutable i = 0\nlet r = loop {\n\ti += 1\n\tif i > 3 { break }\n}\n"),
		["E-LOOP-11"]
	);
	assert_eq!(
		codes("mutable i = 0\nloop {\n\ti += 1\n\tif i > 2 { break i }\n}\n"),
		["W-LOOP-1"]
	);
	assert_eq!(codes("for i in 0..3 { break i }\n"), ["E-LOOP-7"]);
	assert_eq!(codes("break\n"), ["E-LOOP-12"]);
	assert_eq!(codes("continue\n"), ["E-LOOP-12"]);
	// FN014 is to become an alias of E-DO-2 (decisions.md)
	assert!(matches!(
		codes("return 1\n").as_slice(),
		["E-DO-2" | "FN014"]
	));
	// `loop` without `break` is `Never`: the code after it is unreachable
	let c = analyze("let x = 1\nloop { }\nlet y: UInt8 = 300\n");
	assert_eq!(c.type_of("loop { }"), "Never");
	// a `break` state flows to the code after the loop
	let c = analyze("mutable x = 0\nloop {\n\tx = 5\n\tbreak\n}\nlet y: UInt8 = x\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.type_of("loop {\n\tx = 5\n\tbreak\n}"), "Void");
	let c = analyze("mutable x = 0\nloop {\n\tx = 300\n\tbreak\n}\nlet y: UInt8 = x\n");
	assert_eq!(c.codes(), ["E-NUM-10"]);
}

#[test]
fn do_blocks() {
	let c = analyze("let value = do {\n\tlet a = 2\n\tlet b = 3\n\ta * b\n}\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("value"), "Integer");
	assert_eq!(codes("let x = do {\n\tlet a = 1\n}\n"), ["E-DO-1"]);
	assert_eq!(codes("let y = do { }\n"), ["E-DO-1"]);
	assert_eq!(
		codes("do {\n\tlet a = 1\n}\nconsole.log(a)\n"),
		["E-DECL-1"]
	);
}

#[test]
fn arithmetic_on_unknown_types_is_quiet() {
	assert_eq!(codes("let x = nope + 1\n"), ["E-DECL-1"]);
	assert_eq!(
		codes("let x = -nope\nlet y = nope < 3\n"),
		["E-DECL-1", "E-DECL-1"]
	);
	let c = analyze("let x = 1\nlet y = x + 2\n");
	assert_eq!(c.type_of("x + 2"), "Integer");
	let _ = TypeId::UNSET;
}

// ── errors ────────────────────────────────────────────────────────────────

#[test]
fn failing_expressions_must_be_covered() {
	assert_eq!(codes("let v = Integer.parse(\"1\")\n"), ["E-ERR-3"]);
	assert_eq!(
		codes("let v = try Integer.parse(\"1\")\n"),
		Vec::<&str>::new()
	);
	let c = analyze("let v = Integer.parse(\"1\") catch error => 0\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("v"), "Integer");
	assert_eq!(c.symbol_type("error"), "ParseError");
	assert!(
		c.flags("Integer.parse(\"1\")")
			.contains(NodeFlags::CAN_FAIL)
	);
	// one `catch` covers every failing expression of its left operand
	assert_eq!(
		codes("let t = (Integer.parse(\"1\") + Integer.parse(\"2\")) catch error => -1\n"),
		Vec::<&str>::new()
	);
	// divisions and conversions not proven
	assert_eq!(
		codes("for i in 0..10 {\n\tlet q = 10 // i\n\tlet r = Int8(i * 100)\n}\n"),
		["E-ERR-3", "E-ERR-3"]
	);
	let c = analyze("for i in 0..10 {\n\tlet q = 10 // i catch error => 0\n}\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("error"), "DivisionByZero");
}

#[test]
fn errors_leaving_a_function_are_declared() {
	assert_eq!(
		codes(
			"function parse = (t: String): Integer throws ParseError => try Integer.parse(t)\nlet v = parse(\"1\") catch e => 0\n"
		),
		Vec::<&str>::new()
	);
	assert_eq!(
		codes("function parse = (t: String): Integer => {\n\ttry Integer.parse(t)\n}\n"),
		["E-ERR-4"]
	);
	// a single-expression body infers its errors: its calls must be covered
	assert_eq!(
		codes("function parse = (t: String) => try Integer.parse(t)\nlet v = parse(\"1\")\n"),
		["E-ERR-3"]
	);
	assert_eq!(
		codes("function f = (): Integer throws ParseError => {\n\t1\n}\n"),
		["W-ERR-3"]
	);
	assert_eq!(codes("function f = () => {\n\tthrow 5\n}\n"), ["E-ERR-1"]);
	// a `return` in a handler leaves the function
	assert_eq!(
		codes(
			"function g = (): Integer => {\n\tlet v = Integer.parse(\"x\") catch error => {\n\t\treturn 3\n\t}\n\tv\n}\n"
		),
		Vec::<&str>::new()
	);
}

#[test]
fn misused_try_and_catch() {
	assert_eq!(codes("let y = 5 catch e => 0\n"), ["W-ERR-2"]);
	assert_eq!(
		codes("function g = (x: Integer): Integer => try x + 1\n"),
		["W-ERR-1"]
	);
	assert_eq!(
		codes("let v = (try Integer.parse(\"1\")) + 1 catch e => 0\n"),
		["E-ERR-6"]
	);
	// typed like the branches of an `if` (D-98)
	assert_eq!(
		codes("let v = Integer.parse(\"1\") catch error => 0.5\n"),
		["E-IF-2"]
	);
	assert_eq!(
		codes("let v: Float = Integer.parse(\"1\") catch error => 0.5\n"),
		["E-TYPE-1"]
	);
}

// ── narrowing ─────────────────────────────────────────────────────────────

#[test]
fn none_tests_narrow_optionals() {
	let c = analyze(
		"function f = (x: Integer?) => {\n\tif x is not none {\n\t\tconsole.log(x + 1)\n\t}\n\tif x != none {\n\t\tlet y: Integer = x\n\t}\n\tif x == none {\n\t\treturn\n\t}\n\tconsole.log(x * 2)\n}\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert!(c.flags("x * 2").is_empty() || c.codes().is_empty());
	// without a test: E-OPT-1
	assert_eq!(
		codes("function f = (x: Integer?) => {\n\tconsole.log(x + 1)\n}\n"),
		["E-OPT-1"]
	);
	assert_eq!(
		codes("function f = (x: Integer?) => {\n\tlet y: Integer = x\n}\n"),
		["E-OPT-1"]
	);
	assert_eq!(
		codes("function f = (x: Integer?) => {\n\tif x < 3 { }\n}\n"),
		["E-OPT-1"]
	);
	// `and`, `or`, `not`, `assert`
	assert_eq!(
		codes(
			"function f = (x: Integer?) => {\n\tif x is not none and x > 3 { }\n\tif x is none or x > 3 { }\n\tif not (x is none) { console.log(x + 1) }\n\tassert x is not none\n\tconsole.log(x - 1)\n}\n"
		),
		Vec::<&str>::new()
	);
}

#[test]
fn narrowed_reads_are_marked_and_revalidated() {
	let c = analyze(
		"function f = (x: Integer?) => {\n\tif x is not none {\n\t\tconsole.log(x)\n\t}\n}\n",
	);
	let tree = c.session.tree(c.file);
	let analysis = c.session.analysis(c.file);
	let read = (0..tree.nodes.len())
		.filter(|&node| tree.text(node) == "x")
		.map(NodeId::new)
		.find(|&node| analysis.flags_of(node).contains(NodeFlags::NARROWED));
	assert!(read.is_some(), "the read in the branch is narrowed");
	assert_eq!(
		c.session.display_type(analysis.type_of(read.unwrap())),
		"Integer"
	);
	// an assignment ends the narrowing (T-NAR-9)
	assert_eq!(
		codes(
			"function find = (): Integer? => none\nmutable x: Integer? = 1\nif x is not none {\n\tx = find()\n\tconsole.log(x + 1)\n}\n"
		),
		["E-OPT-1"]
	);
	// an assigned value narrows (T-NAR-8)
	assert_eq!(
		codes("mutable x: Integer? = none\nif x is none {\n\tx = 0\n\tconsole.log(x + 1)\n}\n"),
		Vec::<&str>::new()
	);
	// a declaration keeps its declared type (T-OPT-22)
	assert_eq!(codes("let x: Integer? = 1\nlet y = x + 1\n"), ["E-OPT-1"]);
	// lost at the loop fixpoint: the first round is narrowed, the stable one isn't
	assert_eq!(
		codes(
			"function find = (): Integer? => none\nmutable x: Integer? = 1\nwhile true {\n\tif x is not none { }\n\tlet y: Integer = x ?? 0\n\tx = find()\n\tbreak\n}\n"
		),
		["W-BOOL-1"]
	);
}

#[test]
fn type_level_verdicts_and_fallbacks() {
	assert_eq!(codes("let n = 5\nif n is none { }\n"), ["E-CMP-7"]);
	assert_eq!(codes("let n = 5\nif n is not none { }\n"), ["W-CMP-1"]);
	assert_eq!(codes("let n = 5\nlet x = n is String\n"), ["E-CMP-7"]);
	assert_eq!(codes("let n = 5\nlet y = n is Integer\n"), ["W-CMP-1"]);
	assert_eq!(codes("let n = 5\nlet m = n ?? 0\n"), ["E-OPT-6"]);
	assert_eq!(
		codes("let a: Integer? = none\nlet r = a ?? 1 ?? 2\n"),
		["E-OPT-6"]
	);
	let c = analyze("let a: Integer? = none\nlet b: Integer? = 2\nlet r = a ?? b ?? 3\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert_eq!(c.symbol_type("r"), "Integer");
	assert_eq!(
		codes("function load = (): Integer throws ParseError => 1\nlet v = load() ?? 0\n"),
		["E-OPT-6"]
	);
	// `?? return` (D-100)
	assert_eq!(
		codes("function f = (x: Integer?): Integer => {\n\tlet y = x ?? return 0\n\ty + 1\n}\n"),
		Vec::<&str>::new()
	);
}

#[test]
fn literal_and_range_patterns_refine_integers() {
	let c = analyze(
		"for score in 0..=100 {\n\tif score is 0..60 { continue }\n\tlet s: UInt8 = score\n\tif score is 60 | 70 { let t: UInt8 = score }\n\tif score is not 90..=100 { let u: Int8 = score }\n}\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
}

// ── index proofs ──────────────────────────────────────────────────────────

#[test]
fn indices_proven_by_lengths() {
	let c = analyze("let array = [1, 2, 3]\nlet a = array[2]\nlet b = array[-3]\n");
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert!(!c.flags("array[2]").contains(NodeFlags::INDEX_CHECK));
	assert_eq!(
		codes("let array = [1, 2, 3]\nlet item = array[3]\n"),
		["E-NUM-17"]
	);
	assert_eq!(codes("let s = \"abc\"\nlet c = s[5]\n"), ["E-NUM-17"]);
	assert_eq!(codes("let s = \"abc\"\nlet c = s[1]\n"), Vec::<&str>::new());
	// an index fact (T-NUM-54)
	assert_eq!(
		codes("let i: Integer(min = 0, max = 2) = 1\nlet array = [1, 2, 3]\nlet x = array[i]\n"),
		Vec::<&str>::new()
	);
	// not proven: a check and the IndexError path
	let c = analyze(
		"function f = (xs: Array(Integer), i: Integer) throws IndexError => {\n\tlet x = try xs[i]\n}\n",
	);
	assert_eq!(c.codes(), Vec::<&str>::new());
	assert!(
		c.flags("xs[i]")
			.contains(NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL)
	);
	assert_eq!(
		codes("function f = (xs: Array(Integer), i: Integer) => {\n\tlet x = xs[i]\n}\n"),
		["E-ERR-3"]
	);
	// a method changing the collection drops its length
	assert_eq!(
		codes("mutable xs = [1, 2]\nlet a = xs[1]\nxs.push(3)\nlet b = xs[1]\n"),
		["E-ERR-3"]
	);
}

#[test]
fn indices_proven_by_relations() {
	let source = "function sum = (xs: Array(Integer)): Integer => {\n\tmutable total = 0\n\tfor i in 0..xs.length {\n\t\ttotal = xs[i]\n\t}\n\tfor i in 0..xs.length - 1 {\n\t\ttotal = xs[i + 1]\n\t}\n\tfor x, i in xs {\n\t\ttotal = xs[i]\n\t}\n\tmutable j = 0\n\twhile j < xs.length {\n\t\ttotal = xs[j]\n\t\tj += 1\n\t}\n\tif j < xs.length {\n\t\ttotal = xs[j]\n\t}\n\ttotal\n}\n";
	assert_eq!(codes(source), Vec::<&str>::new());
	// beyond the relation: `i + 2` with `i < xs.length - 1`
	assert_eq!(
		codes(
			"function f = (xs: Array(Integer)) => {\n\tfor i in 0..xs.length - 1 {\n\t\tlet x = xs[i + 2]\n\t}\n}\n"
		),
		["E-ERR-3"]
	);
	// an assignment ends the relation
	assert_eq!(
		codes(
			"function f = (xs: Array(Integer)) => {\n\tmutable j = 0\n\tif j < xs.length {\n\t\tj = 5\n\t\tlet x = xs[j]\n\t}\n}\n"
		),
		["E-ERR-3"]
	);
}

// ── when ──────────────────────────────────────────────────────────────────

#[test]
fn when_over_integers() {
	// covered by ranges and literals (T-WHEN-8, T-WHEN-9)
	assert_eq!(
		codes(
			"function sign = (n: Integer): String => when n is {\n\t..0 => \"negative\"\n\t0 => \"zero\"\n\t1.. => \"positive\"\n}\n"
		),
		Vec::<&str>::new()
	);
	assert_eq!(
		codes("function f = (n: Integer) => when n is {\n\t0..10 => 1\n\t20.. => 2\n}\n"),
		["E-WHEN-1"]
	);
	// the facts count (T-WHEN-10, D-55)
	assert_eq!(
		codes(
			"function f = (x: Integer) => {\n\tif x < 0 { return }\n\tconsole.log(when x is { 0..10 => \"small\"\n\t10.. => \"big\" })\n}\n"
		),
		Vec::<&str>::new()
	);
	// each arm sees the subject narrowed, the next ones what is left
	assert_eq!(
		codes(
			"function f = (x: Integer) => when x is {\n\t0..=255 => {\n\t\tlet b: UInt8 = x\n\t\tb\n\t}\n\telse => 0\n}\n"
		),
		Vec::<&str>::new()
	);
}

#[test]
fn when_narrows_optionals_and_unions() {
	assert_eq!(
		codes(
			"function k = (u: Integer?) => {\n\twhen u is {\n\t\tnone => console.log(\"nobody\")\n\t\tInteger => console.log(u + 1)\n\t}\n}\n"
		),
		Vec::<&str>::new()
	);
	assert_eq!(
		codes(
			"type Shape = Union { Circle = { radius: Float }, Square = { side: Float } }\nfunction area = (shape: Shape): Float => when shape is {\n\tShape.Circle => shape.radius * shape.radius\n\tShape.Square => shape.side * shape.side\n}\n"
		),
		Vec::<&str>::new()
	);
	// `is` narrows a named union too
	assert_eq!(
		codes(
			"type Shape = Union { Circle = { radius: Float }, Square = { side: Float } }\nfunction r = (shape: Shape): Float => {\n\tif shape is Shape.Circle {\n\t\treturn shape.radius\n\t}\n\tshape.side\n}\n"
		),
		Vec::<&str>::new()
	);
}
