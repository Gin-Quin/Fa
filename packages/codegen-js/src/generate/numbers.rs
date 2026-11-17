//! Numbers (values brief §6, design §3.6): literals, arithmetic, compound
//! assignments, conversions, and the checks the analyzer asks for.
//!
//! Exactly the checks of the node flags are generated:
//!
//! - `OVERFLOW_JS` on a number-represented integer node: `faCheckInteger`
//!   (±(2^53 − 1)) on the steps of the node that may exceed it;
//! - `OVERFLOW_NATIVE` on a bigint-represented node (`Int64`, `UInt64`, the
//!   128-bit types): `faCheckWidth` against the compute type; with overflow
//!   checks off, `BigInt.asIntN` wraps it to the compute type instead, as
//!   native code wraps (design/decisions.md, "After the backends review");
//! - `SINK_CHECK` with the coercion of the node: `faCheckRange` against the
//!   sink type;
//! - `DIVISOR_CHECK`: the floor division helpers throw `DivisionByZero`;
//! - `EXPONENT_CHECK`: `faPower` / `faBigPower` throw `ArithmeticError`;
//! - `CONVERSION_CHECK`: `faConvertInteger`, `faBigToInteger`,
//!   `faFloatToInteger`.
//!
//! A node of a number-represented integer type whose values are certainly
//! beyond ±(2^53 − 1) is `E-NUM-19`, the one error of the JavaScript backend
//! (values brief D-9).

use ethnum::I256;
use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::diagnostics::codes::values::E_NUM_19;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::lower::index::IndexAccess;
use fa_analyzer::syntax::{ArithOp, NodeList, View};
use fa_analyzer::types::int::{INFINITY, js_safe_range};
use fa_analyzer::types::{ComputeType, FloatBits, IntStorage, Interval, TypeKind};

use crate::generate::{Generator, Kind, Repr};
use crate::js::{self, Expr, Prec};

/// What a step of an integer operation is computed on.
#[derive(Copy, Clone, Eq, PartialEq)]
enum IntRepr {
	Number,
	BigInt,
}

impl Generator<'_> {
	// ── literals ─────────────────────────────────────────────────────────

	/// An integer literal in the representation of its type.
	pub(crate) fn integer_literal(&mut self, n: NodeId, value: I256) -> Expr {
		let ty = self.type_of(n);
		match self.kind(ty) {
			Kind::Float(bits) => self.float_constant(value.as_f64(), bits),
			Kind::BigInt => big_int_constant(value),
			Kind::Int(storage) if storage.is_js_bigint() => big_int_constant(value),
			_ => self.number_constant(n, value),
		}
	}

	/// An integer value as a number literal; `E-NUM-19` beyond ±(2^53 − 1).
	fn number_constant(&mut self, n: NodeId, value: I256) -> Expr {
		if !js_safe_range().contains(value) {
			self.error(
				&E_NUM_19,
				n,
				format!(
					"{value} doesn't fit in an `Integer` on JavaScript, which holds ±(2^53 − 1): use an `Int64` or a `BigInt`"
				),
			);
		}
		let prec = if value < I256::ZERO {
			Prec::Unary
		} else {
			Prec::Primary
		};
		Expr::constant(value.to_string(), prec)
	}

	/// A float literal of a float type (`Math.fround` for a `Float32` that
	/// isn't exact).
	fn float_constant(&mut self, value: f64, bits: FloatBits) -> Expr {
		let literal = js::float_literal(value);
		if bits == FloatBits::F32 && (value as f32) as f64 != value {
			return Expr::constant(format!("Math.fround({})", literal.text), Prec::Primary);
		}
		literal
	}

	/// A float literal node: `Float`, or `Float32` where one is expected.
	pub(crate) fn float_value(&mut self, n: NodeId, value: f64) -> Expr {
		match self.kind(self.type_of(n)) {
			Kind::Float(bits) => self.float_constant(value, bits),
			_ => js::float_literal(value),
		}
	}

	/// A big integer literal: `12_000n` → `12000n`.
	pub(crate) fn big_int_literal(&mut self, text: &str) -> Expr {
		let text: String = text.chars().filter(|&c| c != '_').collect();
		let prec = if text.starts_with('-') {
			Prec::Unary
		} else {
			Prec::Primary
		};
		Expr::constant(text, prec)
	}

	/// `50%`, `(x)%`: the fraction.
	pub(crate) fn percentage(&mut self, n: NodeId, value: NodeId) -> Expr {
		let inner = self.strip_groups(value);
		match self.view(inner) {
			View::Integer(literal) => js::float_literal(literal.as_f64() / 100.0),
			View::Float(literal) => js::float_literal(literal / 100.0),
			_ => {
				let value = self.expr(value);
				let value = match self.repr(self.type_of(inner)) {
					Repr::BigInt => js::call("Number", &[value.text]),
					_ => value,
				};
				let _ = n;
				js::binary(
					&value,
					"/",
					&Expr::constant("100", Prec::Primary),
					Prec::Multiplicative,
				)
			}
		}
	}

	/// An exact integer computed by the analyzer, in the representation of a type.
	fn folded(&mut self, n: NodeId, ty: TypeId) -> Option<Expr> {
		if !self.has(n, NodeFlags::CONST_FOLDED) {
			return None;
		}
		let value = self.analysis.arith(n)?.interval.as_exact()?;
		Some(match self.repr(ty) {
			Repr::BigInt => big_int_constant(value),
			_ => self.number_constant(n, value),
		})
	}

	// ── arithmetic ───────────────────────────────────────────────────────

	/// `a + b`, `a // b`… (n-ary: folds left, `**` right).
	pub(crate) fn arithmetic(&mut self, n: NodeId, op: ArithOp, operands: NodeList<'_>) -> Expr {
		let ty = self.type_of(n);
		let nodes: Vec<NodeId> = operands.iter().collect();
		if self.is_fields(ty) {
			return self.fields_arithmetic(n, op, &nodes);
		}
		if let Some(folded) = self.folded(n, ty) {
			return folded;
		}
		match self.number_kind(ty) {
			Kind::Text => {
				let values = self.in_order(&nodes, |g, operand| g.expr(operand));
				fold_left(&values, |a, b| js::binary(a, "+", b, Prec::Additive))
			}
			Kind::Int(storage) => self.integer_arithmetic(n, op, &nodes, storage),
			Kind::Float(bits) => {
				if op == ArithOp::Divide
					&& nodes.iter().all(|&operand| {
						matches!(
							self.kind(self.type_of(operand)),
							Kind::Int(_) | Kind::BigInt
						)
					}) {
					return self.integer_true_division(&nodes, bits);
				}
				let values = self.in_order(&nodes, |g, operand| g.expr(operand));
				self.float_arithmetic(op, &values, bits)
			}
			Kind::BigInt => self.big_int_arithmetic(n, op, &nodes),
			Kind::Percentage => {
				let values = self.in_order(&nodes, |g, operand| g.expr(operand));
				self.float_arithmetic(op, &values, FloatBits::F64)
			}
			_ => self.unsupported(n, "this operation"),
		}
	}

	/// `/` between integers: a `Float`.
	fn integer_true_division(&mut self, nodes: &[NodeId], bits: FloatBits) -> Expr {
		let values = self.in_order(nodes, |g, operand| g.expr(operand));
		let values: Vec<Expr> = values
			.into_iter()
			.zip(nodes)
			.map(|(value, &operand)| match self.repr(self.type_of(operand)) {
				Repr::BigInt => js::call("Number", &[value.text]),
				_ => value,
			})
			.collect();
		self.float_arithmetic(ArithOp::Divide, &values, bits)
	}

	/// Float (and percentage) operations; a `Float32` is rounded after each one.
	fn float_arithmetic(&mut self, op: ArithOp, values: &[Expr], bits: FloatBits) -> Expr {
		let step = |g: &mut Self, a: &Expr, b: &Expr| {
			let result = match op {
				ArithOp::Add => js::binary(a, "+", b, Prec::Additive),
				ArithOp::Subtract => js::binary(a, "-", b, Prec::Additive),
				ArithOp::Multiply => js::binary(a, "*", b, Prec::Multiplicative),
				ArithOp::Divide => js::binary(a, "/", b, Prec::Multiplicative),
				ArithOp::Power => js::binary(a, "**", b, Prec::Exponent),
				ArithOp::IntegerDivide => {
					let helper = g.helper("faFloatFloorDivide");
					js::call(helper, &[a.text.clone(), b.text.clone()])
				}
				ArithOp::Remainder => {
					let helper = g.helper("faFloatFloorModulo");
					js::call(helper, &[a.text.clone(), b.text.clone()])
				}
			};
			if bits == FloatBits::F32 {
				js::call("Math.fround", &[result.text])
			} else {
				result
			}
		};
		if op == ArithOp::Power {
			let mut iter = values.iter().rev();
			let mut result = iter.next().cloned().unwrap_or_else(undefined);
			for base in iter {
				result = step(self, base, &result);
			}
			result
		} else {
			let mut iter = values.iter();
			let mut result = iter.next().cloned().unwrap_or_else(undefined);
			for value in iter {
				result = step(self, &result, value);
			}
			result
		}
	}

	/// Operations on `BigInt` (never overflow).
	fn big_int_arithmetic(&mut self, n: NodeId, op: ArithOp, nodes: &[NodeId]) -> Expr {
		let values = self.in_order(nodes, |g, operand| g.expr(operand));
		let values: Vec<Expr> = values
			.into_iter()
			.zip(nodes)
			.map(|(value, &operand)| match self.repr(self.type_of(operand)) {
				Repr::Number => self.big_int_of(operand, &value),
				_ => value,
			})
			.collect();
		let flags = self.analysis.flags_of(n);
		self.fold_integer(op, &values, IntRepr::BigInt, flags, |_, _, value| value)
	}

	/// Operations on integer types: exact, with the overflow checks of the flags.
	fn integer_arithmetic(
		&mut self,
		n: NodeId,
		op: ArithOp,
		nodes: &[NodeId],
		storage: IntStorage,
	) -> Expr {
		let repr = if storage.is_js_bigint() {
			IntRepr::BigInt
		} else {
			IntRepr::Number
		};
		self.certain_js_overflow(n, repr);
		let values = self.in_order(nodes, |g, operand| g.expr(operand));
		let values: Vec<Expr> = values
			.into_iter()
			.zip(nodes)
			.map(|(value, &operand)| {
				if repr == IntRepr::BigInt && self.repr(self.type_of(operand)) == Repr::Number {
					self.big_int_of(operand, &value)
				} else {
					value
				}
			})
			.collect();
		if let Some(inline) = self.natural_division(n, op, nodes, &values, repr) {
			return inline;
		}
		// overflow checks off: a product wrapped to 32 bits is `Math.imul`
		// (exact, where `a * b` may round beyond 2^53)
		if repr == IntRepr::Number
			&& op == ArithOp::Multiply
			&& values.len() == 2
			&& let Some(target) = self.analysis.coercion(n)
			&& self.wrapped_width(n, target) == Some(32)
		{
			return js::call(
				"Math.imul",
				&[values[0].text.clone(), values[1].text.clone()],
			);
		}
		let checks = self.step_checks(n, op, nodes, repr);
		let compute = self
			.analysis
			.arith(n)
			.map_or(ComputeType::Int64, |info| info.compute);
		let location = self.location(n);
		let flags = self.analysis.flags_of(n);
		self.fold_integer(op, &values, repr, flags, |g, step, value| {
			if checks.get(step).copied().unwrap_or(false) {
				g.overflow_check(value, repr, compute, &location)
			} else {
				value
			}
		})
	}

	/// `a // b` and `a %% b` with a dividend proven non-negative and a
	/// divisor proven positive: on bigints, JavaScript's `/` and `%`; on
	/// numbers, JavaScript's `%` is the
	/// floor modulo there, and the quotient is `(a / b) | 0` within 32
	/// bits, `(a - a % b) / b` beyond (exact: `a - a % b` is a multiple of
	/// `b`), instead of the helpers' sign tests. A quotient by a constant
	/// power of two is `Math.floor(a / b)` whatever the dividend.
	fn natural_division(
		&mut self,
		n: NodeId,
		op: ArithOp,
		nodes: &[NodeId],
		values: &[Expr],
		repr: IntRepr,
	) -> Option<Expr> {
		if nodes.len() != 2 || !matches!(op, ArithOp::IntegerDivide | ArithOp::Remainder) {
			return None;
		}
		if repr == IntRepr::BigInt {
			// bigints: `/` and `%` truncate, which is the floor division
			// when both operands are non-negative (and the divisor not 0)
			let (a, b) = (self.interval_of(nodes[0])?, self.interval_of(nodes[1])?);
			if a.lo < I256::ZERO || b.lo < I256::ONE {
				return None;
			}
			let operator = if op == ArithOp::Remainder { "%" } else { "/" };
			return Some(js::binary(
				&values[0],
				operator,
				&values[1],
				Prec::Multiplicative,
			));
		}
		if self.has(n, NodeFlags::DIVISOR_CHECK) {
			return None;
		}
		let (a, b) = (self.interval_of(nodes[0])?, self.interval_of(nodes[1])?);
		// `a // 2^k` with any dividend: `Math.floor(a / 2^k)`, exact (a
		// division by a power of two only changes the exponent)
		if op == ArithOp::IntegerDivide
			&& b.lo == b.hi
			&& b.lo >= I256::ONE
			&& b.lo <= I256::from(1u64 << 52)
			&& b.lo.count_ones() == 1
			&& a.lo < I256::ZERO
		{
			let quotient = js::binary(&values[0], "/", &values[1], Prec::Multiplicative);
			return Some(Expr::primary(format!("Math.floor({})", quotient.text)));
		}
		if a.lo < I256::ZERO || b.lo < I256::ONE {
			return None;
		}
		let (dividend, divisor) = (&values[0], &values[1]);
		if op == ArithOp::Remainder {
			return Some(js::binary(dividend, "%", divisor, Prec::Multiplicative));
		}
		let int32 = I256::from(i32::MAX);
		if a.hi <= int32 && b.hi <= int32 {
			let quotient = js::binary(dividend, "/", divisor, Prec::Multiplicative);
			let zero = Expr::constant("0", Prec::Primary);
			return Some(js::binary(&quotient, "|", &zero, Prec::BitOr));
		}
		// both operands are read twice: only plain reads
		if !(dividend.pure && divisor.pure) {
			return None;
		}
		let remainder = js::binary(dividend, "%", divisor, Prec::Multiplicative);
		let multiple = js::binary(dividend, "-", &remainder, Prec::Additive);
		Some(js::binary(&multiple, "/", divisor, Prec::Multiplicative))
	}

	/// Folds integer operands: left, or right for `**`; `check` wraps the
	/// result of each step (numbered in evaluation order).
	fn fold_integer(
		&mut self,
		op: ArithOp,
		values: &[Expr],
		repr: IntRepr,
		flags: NodeFlags,
		mut check: impl FnMut(&mut Self, usize, Expr) -> Expr,
	) -> Expr {
		if values.is_empty() {
			return undefined();
		}
		let steps = values.len() - 1;
		if op == ArithOp::Power {
			let mut result = values[steps].clone();
			for (step, base) in values[..steps].iter().rev().enumerate() {
				let value = self.integer_step(op, base, &result, repr, flags);
				result = check(self, step, value);
			}
			result
		} else {
			let mut result = values[0].clone();
			for (step, value) in values[1..].iter().enumerate() {
				let computed = self.integer_step(op, &result, value, repr, flags);
				result = check(self, step, computed);
			}
			result
		}
	}

	/// One integer operation `a op b`, without overflow check; the node's
	/// flags say whether the exponent and the divisor are checked (a
	/// divisor proven non-zero uses the helpers without the test).
	fn integer_step(
		&mut self,
		op: ArithOp,
		a: &Expr,
		b: &Expr,
		repr: IntRepr,
		flags: NodeFlags,
	) -> Expr {
		let exponent_check = flags.contains(NodeFlags::EXPONENT_CHECK);
		let divisor_check = flags.contains(NodeFlags::DIVISOR_CHECK);
		let helper = |g: &mut Self, name: &'static str| {
			let name = g.helper(name);
			js::call(name, &[a.text.clone(), b.text.clone()])
		};
		match (op, repr) {
			(ArithOp::Add, _) => js::binary(a, "+", b, Prec::Additive),
			(ArithOp::Subtract, _) => js::binary(a, "-", b, Prec::Additive),
			(ArithOp::Multiply, _) => js::binary(a, "*", b, Prec::Multiplicative),
			(ArithOp::Divide, _) => js::binary(a, "/", b, Prec::Multiplicative),
			(ArithOp::IntegerDivide, IntRepr::Number) if divisor_check => {
				helper(self, "faFloorDivide")
			}
			(ArithOp::IntegerDivide, IntRepr::Number) => helper(self, "faFloorQuotient"),
			(ArithOp::Remainder, IntRepr::Number) if divisor_check => helper(self, "faFloorModulo"),
			(ArithOp::Remainder, IntRepr::Number) => helper(self, "faFloorRemainder"),
			(ArithOp::Power, IntRepr::Number) => helper(self, "faPower"),
			(ArithOp::IntegerDivide, IntRepr::BigInt) if divisor_check => {
				helper(self, "faBigFloorDivide")
			}
			(ArithOp::IntegerDivide, IntRepr::BigInt) => helper(self, "faBigFloorQuotient"),
			(ArithOp::Remainder, IntRepr::BigInt) if divisor_check => {
				helper(self, "faBigFloorModulo")
			}
			(ArithOp::Remainder, IntRepr::BigInt) => helper(self, "faBigFloorRemainder"),
			(ArithOp::Power, IntRepr::BigInt) if exponent_check => helper(self, "faBigPower"),
			(ArithOp::Power, IntRepr::BigInt) => js::binary(a, "**", b, Prec::Exponent),
		}
	}

	/// The overflow check of a step: ±(2^53 − 1) for numbers, the compute
	/// type's width for bigints.
	fn overflow_check(
		&mut self,
		value: Expr,
		repr: IntRepr,
		compute: ComputeType,
		location: &str,
	) -> Expr {
		match repr {
			IntRepr::Number => {
				let helper = self.helper("faCheckInteger");
				js::call(helper, &[value.text, location.to_string()])
			}
			IntRepr::BigInt => {
				let (bits, signed, name) = match compute {
					ComputeType::Int64 => (64, true, "Int64"),
					ComputeType::Int128 => (128, true, "Int128"),
					ComputeType::UInt128 => (128, false, "UInt128"),
				};
				// checks off: the step wraps, as natively
				if !self.options.overflow_checks {
					return big_wrap(value, bits, signed);
				}
				let helper = self.helper("faCheckWidth");
				js::call(
					helper,
					&[
						value.text,
						bits.to_string(),
						signed.to_string(),
						js::string_literal(name),
						location.to_string(),
					],
				)
			}
		}
	}

	/// Which steps of an integer node need an overflow check: none without
	/// the node's flag; otherwise the steps whose interval may leave the
	/// checked range (recomputed from the operands' facts, as the analyzer
	/// does), at least the last one.
	fn step_checks(&self, n: NodeId, op: ArithOp, nodes: &[NodeId], repr: IntRepr) -> Vec<bool> {
		let steps = nodes.len().saturating_sub(1);
		if !self.overflows(n, repr) || steps == 0 {
			return vec![false; steps];
		}
		let Some(info) = self.analysis.arith(n) else {
			return vec![true; steps];
		};
		let range = match repr {
			IntRepr::Number => js_safe_range(),
			IntRepr::BigInt => info.compute.range(),
		};
		let intervals: Option<Vec<Interval>> = nodes
			.iter()
			.map(|&operand| self.interval_of(operand))
			.collect();
		let Some(intervals) = intervals else {
			return vec![true; steps];
		};
		let mut checks = vec![false; steps];
		let step = |a: Interval, b: Interval| -> Interval {
			match op {
				ArithOp::Add => a + b,
				ArithOp::Subtract => a - b,
				ArithOp::Multiply => a * b,
				ArithOp::IntegerDivide => a.floor_div(b).unwrap_or(Interval::full()),
				ArithOp::Remainder => a.modulo(b).unwrap_or(Interval::full()),
				ArithOp::Power => b
					.intersect(Interval::new(I256::ZERO, INFINITY))
					.and_then(|exponent| a.pow(exponent))
					.unwrap_or(Interval::full()),
				ArithOp::Divide => Interval::full(),
			}
		};
		if op == ArithOp::Power {
			let mut result = intervals[steps];
			for (index, &base) in intervals[..steps].iter().rev().enumerate() {
				result = step(base, result);
				checks[index] = !range.contains_interval(result);
			}
		} else {
			let mut result = intervals[0];
			for (index, &value) in intervals[1..].iter().enumerate() {
				result = step(result, value);
				checks[index] = !range.contains_interval(result);
			}
		}
		if !checks.contains(&true) {
			checks[steps - 1] = true;
		}
		checks
	}

	/// How `items[index]` (the `Index` node `n`) is checked and normalized
	/// (`fa_analyzer::lower::index`).
	/// The proofs of the analysis drop more checks (`fa_analyzer::proofs`,
	/// as natively): an index proven in bounds is read directly, and a
	/// proven one proven never negative doesn't count from the end.
	pub(crate) fn index_access(&self, n: NodeId, index: NodeId) -> IndexAccess {
		let access =
			fa_analyzer::lower::index::index_access(self.tree, self.analysis, self.types, n, index);
		match access {
			IndexAccess::Checked if self.index_facts().proven_index(n) => IndexAccess::Direct,
			IndexAccess::FromEnd if self.index_facts().proven_non_negative(index) => {
				IndexAccess::Direct
			}
			_ => access,
		}
	}

	/// The index proofs of the file (`Options::proofs`): all of them when it
	/// is the whole program; private-array indices and never-negative
	/// places by the merged claims in a program of several files. Tasks change nothing:
	/// the claims see the stores of their bodies like any other.
	fn index_facts(&self) -> &fa_analyzer::proofs::IndexFacts {
		self.index_facts.get_or_init(|| {
			use fa_analyzer::proofs::{ProofScope, index_facts};
			// During a key loop `self.analysis` is one key's snapshot. The
			// program's stores belong to all keys, whose common summary is
			// not available yet; never cache the first key's proof for all.
			if !self.session.analysis(self.file).key_loop_bodies.is_empty() {
				return fa_analyzer::proofs::IndexFacts::default();
			}
			let checks = self.options.overflow_checks;
			match &self.options.proofs {
				crate::ProgramProofs::None => index_facts(
					self.tree,
					self.analysis,
					self.types,
					ProofScope::Nothing,
					checks,
				),
				crate::ProgramProofs::Alone => index_facts(
					self.tree,
					self.analysis,
					self.types,
					ProofScope::WholeFile,
					checks,
				),
				crate::ProgramProofs::Claims(failed) => {
					let failed = |key: &str| failed.contains(key);
					let scope = ProofScope::Unit {
						module: self.session.module(self.file),
						failed: &failed,
					};
					index_facts(self.tree, self.analysis, self.types, scope, checks)
				}
			}
		})
	}

	/// Whether a written index must go through `faNormalizeIndex` (`faIndexSet`):
	/// not proven, or proven from `-length` on (it may count from the end).
	pub(crate) fn index_needs_normalizing(&self, n: NodeId, index: NodeId) -> bool {
		self.index_access(n, index).normalizes()
	}

	/// The interval of an integer operand: its fact, or its type's range.
	pub(crate) fn interval_of(&self, n: NodeId) -> Option<Interval> {
		fa_analyzer::lower::index::interval(self.tree, self.analysis, self.types, n)
	}

	/// `E-NUM-19` for a number-represented integer node that certainly
	/// exceeds ±(2^53 − 1).
	fn certain_js_overflow(&mut self, n: NodeId, repr: IntRepr) {
		if repr != IntRepr::Number {
			return;
		}
		if let Some(info) = self.analysis.arith(n)
			&& info.interval.is_disjoint(js_safe_range())
		{
			self.error(
				&E_NUM_19,
				n,
				"this value is always beyond ±(2^53 − 1), the range of an `Integer` on JavaScript: use an `Int64` or a `BigInt`",
			);
		}
	}

	/// `-x`.
	pub(crate) fn negate(&mut self, n: NodeId, value: NodeId) -> Expr {
		let ty = self.type_of(n);
		if let Some(folded) = self.folded(n, ty) {
			return folded;
		}
		let operand = self.expr(value);
		let negated = js::prefix("-", &operand);
		match self.kind(ty) {
			Kind::Int(storage) => {
				let repr = if storage.is_js_bigint() {
					IntRepr::BigInt
				} else {
					IntRepr::Number
				};
				let operand =
					if repr == IntRepr::BigInt && self.repr(self.type_of(value)) == Repr::Number {
						self.big_int_of(value, &operand)
					} else {
						operand
					};
				let negated = js::prefix("-", &operand);
				if self.overflows(n, repr) {
					let compute = self
						.analysis
						.arith(n)
						.map_or(ComputeType::Int64, |info| info.compute);
					let location = self.location(n);
					self.overflow_check(negated, repr, compute, &location)
				} else {
					negated
				}
			}
			_ => negated,
		}
	}

	/// `x.abs()` on a number.
	pub(crate) fn abs(&mut self, n: NodeId, target: NodeId) -> Expr {
		let value = self.expr(target);
		self.abs_of(n, value)
	}

	/// The absolute value of `value`, the receiver of `abs()` at node `n`.
	pub(crate) fn abs_of(&mut self, n: NodeId, value: Expr) -> Expr {
		let ty = self.type_of(n);
		match self.kind(ty) {
			Kind::Int(storage) if storage.is_js_bigint() => {
				let (first, later) = self.twice(value);
				let result = js::conditional(
					&js::binary(
						&first,
						"<",
						&Expr::constant("0n", Prec::Primary),
						Prec::Relational,
					),
					&js::prefix("-", &later),
					&later,
				);
				if self.overflows(n, IntRepr::BigInt) {
					let compute = self
						.analysis
						.arith(n)
						.map_or(ComputeType::Int64, |info| info.compute);
					let location = self.location(n);
					self.overflow_check(result, IntRepr::BigInt, compute, &location)
				} else {
					result
				}
			}
			Kind::BigInt => {
				let (first, later) = self.twice(value);
				js::conditional(
					&js::binary(
						&first,
						"<",
						&Expr::constant("0n", Prec::Primary),
						Prec::Relational,
					),
					&js::prefix("-", &later),
					&later,
				)
			}
			_ => js::call("Math.abs", &[value.text]),
		}
	}

	// ── compound assignment ──────────────────────────────────────────────

	/// `x += value` and the other compound assignments: the target is
	/// evaluated once; the result is checked like an arithmetic root whose
	/// sink is the target's type.
	pub(crate) fn compound_assignment(
		&mut self,
		n: NodeId,
		op: ArithOp,
		target: NodeId,
		value: NodeId,
	) {
		let Some(target_place) = self.place(target, !self.needs_statements(value)) else {
			self.unsupported(target, "this assignment target");
			return;
		};
		let place_ty = target_place.ty;
		let name = target_place.text.clone();
		let place = Expr::name(name.clone(), false);
		let operand = self.expr(value);
		let kind = self.number_kind(place_ty);
		// `x op= v`, unless the place is a value of a map (`map.set`)
		let direct = target_place.map.is_none();
		let simple = |op: ArithOp| match op {
			ArithOp::Add if direct => Some("+="),
			ArithOp::Subtract if direct => Some("-="),
			ArithOp::Multiply if direct => Some("*="),
			ArithOp::Divide if direct => Some("/="),
			ArithOp::Power if direct => Some("**="),
			_ => None,
		};
		let result = match kind {
			// `set += flags`, `set -= flags` (fields.rs)
			Kind::Other if self.is_fields(place_ty) => {
				self.fields_step(place_ty, op, &place, &operand)
			}
			Kind::Text | Kind::Percentage | Kind::Float(FloatBits::F64) => {
				if let Some(operator) = simple(op) {
					self.out
						.line(format!("{name} {operator} {};", operand.at(Prec::Assign)));
					return;
				}
				self.float_arithmetic(op, &[place, operand], FloatBits::F64)
			}
			Kind::Float(FloatBits::F32) => {
				self.float_arithmetic(op, &[place, operand], FloatBits::F32)
			}
			Kind::BigInt => {
				let operand = match self.repr(self.type_of(value)) {
					Repr::Number => self.big_int_of(value, &operand),
					_ => operand,
				};
				if let Some(operator) = simple(op)
					&& !(op == ArithOp::Power && self.has(n, NodeFlags::EXPONENT_CHECK))
				{
					self.out
						.line(format!("{name} {operator} {};", operand.at(Prec::Assign)));
					return;
				}
				let flags = self.analysis.flags_of(n);
				self.integer_step(op, &place, &operand, IntRepr::BigInt, flags)
			}
			Kind::Int(storage) => {
				let repr = if storage.is_js_bigint() {
					IntRepr::BigInt
				} else {
					IntRepr::Number
				};
				let operand =
					if repr == IntRepr::BigInt && self.repr(self.type_of(value)) == Repr::Number {
						self.big_int_of(value, &operand)
					} else {
						operand
					};
				let exponent_check = self.has(n, NodeFlags::EXPONENT_CHECK);
				let overflow = self.overflows(n, repr);
				let sink = self.has(n, NodeFlags::SINK_CHECK);
				let target_ty = self.analysis.coercion(n).unwrap_or(place_ty);
				let wraps = self.wrapped_width(n, target_ty).is_some();
				if !overflow
					&& !sink && !wraps
					&& !(repr == IntRepr::Number && op == ArithOp::Power)
					&& let Some(operator) = simple(op)
					&& !(op == ArithOp::Power && exponent_check)
				{
					self.out
						.line(format!("{name} {operator} {};", operand.at(Prec::Assign)));
					return;
				}
				let flags = self.analysis.flags_of(n);
				let mut result = self.integer_step(op, &place, &operand, repr, flags);
				let location = self.location(n);
				if overflow {
					let compute = self
						.analysis
						.arith(n)
						.map_or(ComputeType::Int64, |info| info.compute);
					result = self.overflow_check(result, repr, compute, &location);
				}
				if sink {
					result = self.range_check(result, target_ty, repr, &location);
				}
				if wraps && let Some(wrapped) = self.wrap_unchecked(n, &result, repr, target_ty) {
					result = wrapped;
				}
				result
			}
			_ => {
				self.unsupported(n, "this compound assignment");
				return;
			}
		};
		self.out.line(target_place.write(&result.text));
	}

	// ── sinks and conversions ────────────────────────────────────────────

	/// The conversion of a value at its sink (`Analysis::coercions`): the
	/// range check of `SINK_CHECK`, then the change of representation
	/// (`Integer` ↔ `Int64`…).
	pub(crate) fn sink(&mut self, n: NodeId, value: Expr) -> Expr {
		let Some(target) = self.analysis.coercion(n) else {
			return value;
		};
		if matches!(self.view(n), View::CompoundAssignment { .. }) {
			return value;
		}
		let source = self.type_of(n);
		let mut value = value;
		// overflow checks off: a sized integer wraps (a number becomes a
		// bigint first when the target is one: wrapped after the conversion)
		let source_repr = match self.repr(source) {
			Repr::BigInt => IntRepr::BigInt,
			_ => IntRepr::Number,
		};
		let wrap_after = source_repr == IntRepr::Number && self.repr(target) == Repr::BigInt;
		if !wrap_after && let Some(wrapped) = self.wrap_unchecked(n, &value, source_repr, target) {
			value = wrapped;
		}
		if self.has(n, NodeFlags::SINK_CHECK) && self.types.as_int(target).is_some() {
			let repr = match self.repr(source) {
				Repr::BigInt => IntRepr::BigInt,
				_ => IntRepr::Number,
			};
			let location = self.location(n);
			value = self.range_check(value, target, repr, &location);
		}
		// a value stored as another object type (structural typing): a copy
		// with the target's fields only (element by element for an array)
		if source != target
			&& matches!(
				self.types.kind(target),
				TypeKind::Object(_)
					| TypeKind::Collection(
						fa_analyzer::types::CollKind::Array
							| fa_analyzer::types::CollKind::InlineArray
							| fa_analyzer::types::CollKind::GrowingArray,
						..
					)
			) && let Some(copy) = self.narrowing_copy(target)
		{
			return js::call(&copy, &[value.text]);
		}
		let converted = self.convert_representation(n, value, source, target);
		if wrap_after
			&& let Some(wrapped) = self.wrap_unchecked(n, &converted, IntRepr::BigInt, target)
		{
			return wrapped;
		}
		converted
	}

	/// Whether an integer node needs an overflow check (checks on), or a
	/// wrap to its compute type (checks off, a bigint: `Int64` and wider
	/// wrap as natively; `Integer` doesn't).
	fn overflows(&self, n: NodeId, repr: IntRepr) -> bool {
		match repr {
			IntRepr::Number => self.has(n, NodeFlags::OVERFLOW_JS),
			IntRepr::BigInt if self.options.overflow_checks => {
				self.has(n, NodeFlags::OVERFLOW_NATIVE)
			}
			IntRepr::BigInt => self.analysis.has_flags(n, NodeFlags::OVERFLOW_NATIVE),
		}
	}

	/// The width of a sized integer type that a value at a node wraps to:
	/// with overflow checks off, where the analyzer asked for a range check
	/// at the sink (design/decisions.md, "Backends (after the backend
	/// review)", "After the backends review"). `Integer` doesn't wrap.
	pub(crate) fn wrapped_width(&self, n: NodeId, target: TypeId) -> Option<u32> {
		if self.options.overflow_checks || !self.analysis.has_flags(n, NodeFlags::SINK_CHECK) {
			return None;
		}
		let storage = self.types.as_int(target)?.storage;
		match storage {
			IntStorage::I8 | IntStorage::U8 => Some(8),
			IntStorage::I16 | IntStorage::U16 => Some(16),
			IntStorage::I32 | IntStorage::U32 => Some(32),
			IntStorage::I64 | IntStorage::U64 => Some(64),
			IntStorage::I128 | IntStorage::U128 => Some(128),
			IntStorage::Default => None,
		}
	}

	/// A value (of representation `repr`) wrapped in two's complement to
	/// its sink's type ([`Generator::wrapped_width`]): `(x << 24) >> 24`,
	/// `x & 255`, `x | 0`, `x >>> 0` on numbers, `BigInt.asIntN(64, x)` on
	/// bigints.
	fn wrap_unchecked(
		&self,
		n: NodeId,
		value: &Expr,
		repr: IntRepr,
		target: TypeId,
	) -> Option<Expr> {
		let bits = self.wrapped_width(n, target)?;
		let signed = matches!(
			self.types.as_int(target)?.storage,
			IntStorage::I8 | IntStorage::I16 | IntStorage::I32 | IntStorage::I64 | IntStorage::I128
		);
		if repr == IntRepr::BigInt {
			return Some(big_wrap(value.clone(), bits, signed));
		}
		Some(crate::generate::number_api::wrap(
			value.clone(),
			bits,
			signed,
		))
	}

	/// `faCheckRange(value, min, max, "Type", location)`.
	fn range_check(&mut self, value: Expr, target: TypeId, repr: IntRepr, location: &str) -> Expr {
		let Some(int) = self.types.as_int(target).copied() else {
			return value;
		};
		let bound = |value: I256| match repr {
			IntRepr::Number => value.to_string(),
			IntRepr::BigInt => format!("{value}n"),
		};
		let helper = self.helper("faCheckRange");
		js::call(
			helper,
			&[
				value.text,
				bound(int.min),
				bound(int.max),
				js::string_literal(&self.types.display(target)),
				location.to_string(),
			],
		)
	}

	/// A value of type `source` where a `target` is expected: `BigInt(x)`,
	/// `Number(x)` (checked against ±(2^53 − 1) when the target is wider).
	pub(super) fn convert_representation(
		&mut self,
		n: NodeId,
		value: Expr,
		source: TypeId,
		target: TypeId,
	) -> Expr {
		match (self.repr(source), self.repr(target)) {
			(Repr::Number, Repr::BigInt) => self.big_int_of(n, &value),
			(Repr::BigInt, Repr::Number) => self.number_of(n, value, target),
			_ => value,
		}
	}

	/// A bigint as a number of type `target`.
	fn number_of(&mut self, n: NodeId, value: Expr, target: TypeId) -> Expr {
		let fits = self
			.types
			.interval_of(target)
			.is_none_or(|range| js_safe_range().contains_interval(range));
		if fits {
			return js::call("Number", &[value.text]);
		}
		let helper = self.helper("faInt64ToInteger");
		let location = self.location(n);
		js::call(helper, &[value.text, location])
	}

	/// A number-represented integer (node `n`, value `value`) as a bigint:
	/// a literal gets the `n` suffix, any other value goes through `BigInt`.
	pub(crate) fn big_int_of(&mut self, n: NodeId, value: &Expr) -> Expr {
		let inner = self.strip_groups(n);
		if let View::Integer(literal) = self.view(inner) {
			return big_int_constant(literal);
		}
		if let Some(info) = self.analysis.arith(inner)
			&& self.has(inner, NodeFlags::CONST_FOLDED)
			&& let Some(exact) = info.interval.as_exact()
		{
			return big_int_constant(exact);
		}
		js::call("BigInt", std::slice::from_ref(&value.text))
	}

	/// `T(value)` for a number type `T` (values brief §6.4): the check of
	/// `CONVERSION_CHECK`, then the change of representation.
	pub(crate) fn conversion(&mut self, n: NodeId, target: TypeId, argument: NodeId) -> Expr {
		let source = self.type_of(argument);
		let value = self.expr(argument);
		let check = self.has(n, NodeFlags::CONVERSION_CHECK);
		match (self.kind(target), self.kind(source)) {
			(Kind::Int(_), Kind::Int(_)) => {
				let value = if check {
					self.checked_integer(value, target, self.repr(source) == Repr::BigInt)
				} else {
					value
				};
				self.convert_representation(argument, value, source, target)
			}
			(Kind::Int(to), Kind::BigInt) => self.big_int_to_integer(n, value, target, to, check),
			(Kind::Int(to), Kind::Float(_)) => self.float_to_integer(n, value, target, to, check),
			(Kind::Float(bits), Kind::Int(_) | Kind::BigInt) => {
				self.integer_to_float(value, source, bits)
			}
			(Kind::Float(FloatBits::F32), Kind::Float(FloatBits::F64) | Kind::Percentage) => {
				js::call("Math.fround", &[value.text])
			}
			(Kind::Float(_), _) => value,
			// `Integer(set)`: the bits of a `Fields` value (fields.rs)
			(Kind::Int(_), Kind::Other) if self.is_fields(source) => {
				self.convert_representation(argument, value, source, target)
			}
			(Kind::BigInt, Kind::Int(_)) => match self.repr(source) {
				Repr::Number => self.big_int_of(argument, &value),
				_ => value,
			},
			(Kind::Percentage, Kind::Int(_) | Kind::Float(_)) => js::binary(
				&value,
				"/",
				&Expr::constant("100", Prec::Primary),
				Prec::Multiplicative,
			),
			_ => value,
		}
	}

	/// `faConvertInteger(value, min, max, "Type")`: `ArithmeticError` when
	/// an integer (a number, or a bigint when `bigint`) misses the target type.
	fn checked_integer(&mut self, value: Expr, target: TypeId, bigint: bool) -> Expr {
		let Some(int) = self.types.as_int(target).copied() else {
			return value;
		};
		let bound = |value: I256| {
			if bigint {
				format!("{value}n")
			} else {
				value.to_string()
			}
		};
		let helper = self.helper("faConvertInteger");
		js::call(
			helper,
			&[
				value.text,
				bound(int.min),
				bound(int.max),
				js::string_literal(&self.types.display(target)),
			],
		)
	}

	/// `Int8(big)`, `Integer(big)`: a `BigInt` to an integer type, checked
	/// when the analyzer asks for it.
	fn big_int_to_integer(
		&mut self,
		n: NodeId,
		value: Expr,
		target: TypeId,
		to: IntStorage,
		check: bool,
	) -> Expr {
		if to.is_js_bigint() {
			return if check {
				self.checked_integer(value, target, true)
			} else {
				value
			};
		}
		if !check {
			return self.number_of(n, value, target);
		}
		let full_integer = self
			.types
			.interval_of(target)
			.is_some_and(|range| !js_safe_range().contains_interval(range));
		if full_integer {
			let helper = self.helper("faBigToInteger");
			let location = self.location(n);
			return js::call(helper, &[value.text, location]);
		}
		if self.types.as_int(target).is_none() {
			return value;
		}
		let checked = self.checked_integer(value, target, true);
		js::call("Number", &[checked.text])
	}

	/// `Integer(x)` of a float: truncated, checked (NaN, infinite, out of
	/// range) unless the analyzer proved it; a proven literal is folded.
	fn float_to_integer(
		&mut self,
		n: NodeId,
		value: Expr,
		target: TypeId,
		to: IntStorage,
		check: bool,
	) -> Expr {
		if !check {
			if let Some(interval) = self.analysis.interval(n)
				&& let Some(exact) = interval.as_exact()
			{
				return if to.is_js_bigint() {
					big_int_constant(exact)
				} else {
					self.number_constant(n, exact)
				};
			}
			let truncated = js::call("Math.trunc", &[value.text]);
			return if to.is_js_bigint() {
				js::call("BigInt", &[truncated.text])
			} else {
				truncated
			};
		}
		let location = self.location(n);
		let helper = self.helper("faFloatToInteger");
		let integer = js::call(helper, &[value.text, location.clone()]);
		if to.is_js_bigint() {
			let big = js::call("BigInt", &[integer.text]);
			return self.range_check(big, target, IntRepr::BigInt, &location);
		}
		match self.types.interval_of(target) {
			Some(range) if !range.contains_interval(js_safe_range()) => {
				self.range_check(integer, target, IntRepr::Number, &location)
			}
			_ => integer,
		}
	}

	/// `Float(n)`, `Float32(n)` of an integer or a `BigInt`.
	fn integer_to_float(&mut self, value: Expr, source: TypeId, bits: FloatBits) -> Expr {
		let bigint = self.repr(source) == Repr::BigInt;
		let number = if bigint {
			js::call("Number", std::slice::from_ref(&value.text))
		} else {
			// integers are never observed as `-0` (D-76)
			js::binary(
				&value,
				"+",
				&Expr::constant("0", Prec::Primary),
				Prec::Additive,
			)
		};
		match bits {
			FloatBits::F64 => number,
			FloatBits::F32 => js::call("Math.fround", &[if bigint { number } else { value }.text]),
		}
	}
}

/// `value` as a bigint literal.
fn big_int_constant(value: I256) -> Expr {
	let prec = if value < I256::ZERO {
		Prec::Unary
	} else {
		Prec::Primary
	};
	Expr::constant(format!("{value}n"), prec)
}

/// `undefined`.
fn undefined() -> Expr {
	Expr::constant("undefined", Prec::Primary)
}

/// Folds values left with a binary operation.
fn fold_left(values: &[Expr], mut f: impl FnMut(&Expr, &Expr) -> Expr) -> Expr {
	let mut iter = values.iter();
	let mut result = iter.next().cloned().unwrap_or_else(undefined);
	for value in iter {
		result = f(&result, value);
	}
	result
}

/// A bigint wrapped in two's complement to `bits`: `BigInt.asIntN(64, x)`,
/// `BigInt.asUintN(64, x)`.
fn big_wrap(value: Expr, bits: u32, signed: bool) -> Expr {
	let function = if signed {
		"BigInt.asIntN"
	} else {
		"BigInt.asUintN"
	};
	js::call(function, &[bits.to_string(), value.text])
}
