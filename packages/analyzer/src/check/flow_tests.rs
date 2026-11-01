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
		self.session
			.analysis(self.file)
			.arith
			.get(&self.outer(text))
			.copied()
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
const UNKNOWN_INT8: &str =
	"for i in -1000..1000 {\n\tlet a = Int8(i)\n\tlet b = Int8(i)\n\tlet u = UInt8(i)\n";

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
		"for i in 0.. {\n\tlet d = Int8(i)\n\tlet q = 10 // d\n\tif d != 0 {\n\t\tlet r = 10 // d\n\t}\n\tlet s = 10 // 3\n}\n",
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
	let c = analyze("for x in 0.. {\n\tlet a = UInt64(x)\n\tlet s: UInt64 = a * a\n}\n");
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
	// widening lost the upper bound: after the loop, `i >= 100` only
	assert_eq!(c.codes_on(6), ["E-NUM-14"]);
	// a decreasing counter
	let c = analyze(
		"mutable remaining = 3\nwhile remaining > 0 {\n\tremaining -= 1\n}\nlet r: UInt8 = remaining\n",
	);
	// widening lost the lower bound: `remaining <= 0` only after the loop
	assert_eq!(c.codes(), ["E-NUM-14"]);
	assert_eq!(c.arith("remaining -= 1").unwrap().interval, i(0, 2));
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
