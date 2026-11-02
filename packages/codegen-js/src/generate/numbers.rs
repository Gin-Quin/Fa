//! Numbers (values brief §6, design §3.6): literals, arithmetic, compound
//! assignments, conversions, and the checks the analyzer asks for.
//!
//! Exactly the checks of the node flags are generated:
//!
//! - `OVERFLOW_JS` on a number-represented integer node: `faCheckInteger`
//!   (±(2^53 − 1)) on the steps of the node that may exceed it;
//! - `OVERFLOW_NATIVE` on a bigint-represented node (`Int64`, `UInt64`, the
//!   128-bit types): `faCheckWidth` against the compute type;
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
use fa_analyzer::syntax::{ArithOp, NodeList, View};
use fa_analyzer::types::int::{INFINITY, js_safe_range};
use fa_analyzer::types::{ComputeType, ConstValue, FloatBits, IntStorage, Interval, TypeKind};

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
		if let Some(folded) = self.folded(n, ty) {
			return folded;
		}
		match self.kind(ty) {
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
		let exponent_check = self.has(n, NodeFlags::EXPONENT_CHECK);
		self.fold_integer(
			op,
			&values,
			IntRepr::BigInt,
			exponent_check,
			|_, _, value| value,
		)
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
		let checks = self.step_checks(n, op, nodes, repr);
		let compute = self
			.analysis
			.arith(n)
			.map_or(ComputeType::Int64, |info| info.compute);
		let location = self.location(n);
		let exponent_check = self.has(n, NodeFlags::EXPONENT_CHECK);
		self.fold_integer(op, &values, repr, exponent_check, |g, step, value| {
			if checks.get(step).copied().unwrap_or(false) {
				g.overflow_check(value, repr, compute, &location)
			} else {
				value
			}
		})
	}

	/// `a // b` and `a %% b` on numbers, with a dividend proven
	/// non-negative and a divisor proven positive: JavaScript's `%` is the
	/// floor modulo there, and the quotient is `(a / b) | 0` within 32
	/// bits, `(a - a % b) / b` beyond (exact: `a - a % b` is a multiple of
	/// `b`), instead of the helpers' sign tests.
	fn natural_division(
		&mut self,
		n: NodeId,
		op: ArithOp,
		nodes: &[NodeId],
		values: &[Expr],
		repr: IntRepr,
	) -> Option<Expr> {
		if repr != IntRepr::Number
			|| nodes.len() != 2
			|| !matches!(op, ArithOp::IntegerDivide | ArithOp::Remainder)
			|| self.has(n, NodeFlags::DIVISOR_CHECK)
		{
			return None;
		}
		let (a, b) = (self.interval_of(nodes[0])?, self.interval_of(nodes[1])?);
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
		exponent_check: bool,
		mut check: impl FnMut(&mut Self, usize, Expr) -> Expr,
	) -> Expr {
		if values.is_empty() {
			return undefined();
		}
		let steps = values.len() - 1;
		if op == ArithOp::Power {
			let mut result = values[steps].clone();
			for (step, base) in values[..steps].iter().rev().enumerate() {
				let value = self.integer_step(op, base, &result, repr, exponent_check);
				result = check(self, step, value);
			}
			result
		} else {
			let mut result = values[0].clone();
			for (step, value) in values[1..].iter().enumerate() {
				let computed = self.integer_step(op, &result, value, repr, exponent_check);
				result = check(self, step, computed);
			}
			result
		}
	}

	/// One integer operation `a op b`, without overflow check.
	fn integer_step(
		&mut self,
		op: ArithOp,
		a: &Expr,
		b: &Expr,
		repr: IntRepr,
		exponent_check: bool,
	) -> Expr {
		let helper = |g: &mut Self, name: &'static str| {
			let name = g.helper(name);
			js::call(name, &[a.text.clone(), b.text.clone()])
		};
		match (op, repr) {
			(ArithOp::Add, _) => js::binary(a, "+", b, Prec::Additive),
			(ArithOp::Subtract, _) => js::binary(a, "-", b, Prec::Additive),
			(ArithOp::Multiply, _) => js::binary(a, "*", b, Prec::Multiplicative),
			(ArithOp::Divide, _) => js::binary(a, "/", b, Prec::Multiplicative),
			(ArithOp::IntegerDivide, IntRepr::Number) => helper(self, "faFloorDivide"),
			(ArithOp::Remainder, IntRepr::Number) => helper(self, "faFloorModulo"),
			(ArithOp::Power, IntRepr::Number) => helper(self, "faPower"),
			(ArithOp::IntegerDivide, IntRepr::BigInt) => helper(self, "faBigFloorDivide"),
			(ArithOp::Remainder, IntRepr::BigInt) => helper(self, "faBigFloorModulo"),
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
		let flag = match repr {
			IntRepr::Number => NodeFlags::OVERFLOW_JS,
			IntRepr::BigInt => NodeFlags::OVERFLOW_NATIVE,
		};
		if !self.has(n, flag) || steps == 0 {
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

	/// The interval of an integer operand: its fact, or its type's range.
	fn interval_of(&self, n: NodeId) -> Option<Interval> {
		let n = self.strip_groups(n);
		if let Some(info) = self.analysis.arith(n) {
			return Some(info.interval);
		}
		if let Some(interval) = self.analysis.interval(n) {
			return Some(interval);
		}
		self.types.interval_of(self.type_of(n))
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
				let flag = match repr {
					IntRepr::Number => NodeFlags::OVERFLOW_JS,
					IntRepr::BigInt => NodeFlags::OVERFLOW_NATIVE,
				};
				if self.has(n, flag) {
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
				if self.has(n, NodeFlags::OVERFLOW_NATIVE) {
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
		let name = target_place.text;
		let place = Expr::name(name.clone(), false);
		let operand = self.expr(value);
		let kind = self.kind(place_ty);
		let simple = |op: ArithOp| match op {
			ArithOp::Add => Some("+="),
			ArithOp::Subtract => Some("-="),
			ArithOp::Multiply => Some("*="),
			ArithOp::Divide => Some("/="),
			ArithOp::Power => Some("**="),
			_ => None,
		};
		let result = match kind {
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
				let exponent_check = self.has(n, NodeFlags::EXPONENT_CHECK);
				self.integer_step(op, &place, &operand, IntRepr::BigInt, exponent_check)
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
				let flag = match repr {
					IntRepr::Number => NodeFlags::OVERFLOW_JS,
					IntRepr::BigInt => NodeFlags::OVERFLOW_NATIVE,
				};
				let overflow = self.has(n, flag);
				let sink = self.has(n, NodeFlags::SINK_CHECK);
				if !overflow
					&& !sink && !(repr == IntRepr::Number && op == ArithOp::Power)
					&& let Some(operator) = simple(op)
					&& !(op == ArithOp::Power && exponent_check)
				{
					self.out
						.line(format!("{name} {operator} {};", operand.at(Prec::Assign)));
					return;
				}
				let mut result = self.integer_step(op, &place, &operand, repr, exponent_check);
				let location = self.location(n);
				if overflow {
					let compute = self
						.analysis
						.arith(n)
						.map_or(ComputeType::Int64, |info| info.compute);
					result = self.overflow_check(result, repr, compute, &location);
				}
				if sink {
					let target_ty = self.analysis.coercion(n).unwrap_or(place_ty);
					result = self.range_check(result, target_ty, repr, &location);
				}
				result
			}
			_ => {
				self.unsupported(n, "this compound assignment");
				return;
			}
		};
		self.out.line(format!("{name} = {};", result.text));
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
		if self.has(n, NodeFlags::SINK_CHECK) && self.types.as_int(target).is_some() {
			let repr = match self.repr(source) {
				Repr::BigInt => IntRepr::BigInt,
				_ => IntRepr::Number,
			};
			let location = self.location(n);
			value = self.range_check(value, target, repr, &location);
		}
		self.convert_representation(n, value, source, target)
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
	fn convert_representation(
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
		let location = self.location(n);
		match (self.kind(target), self.kind(source)) {
			(Kind::Int(to), Kind::Int(_)) => {
				let mut value = value;
				if check && let Some(int) = self.types.as_int(target).copied() {
					let bigint = self.repr(source) == Repr::BigInt;
					let bound = |value: I256| {
						if bigint {
							format!("{value}n")
						} else {
							value.to_string()
						}
					};
					let helper = self.helper("faConvertInteger");
					value = js::call(
						helper,
						&[
							value.text,
							bound(int.min),
							bound(int.max),
							js::string_literal(&self.types.display(target)),
						],
					);
				}
				let _ = to;
				self.convert_representation(argument, value, source, target)
			}
			(Kind::Int(to), Kind::BigInt) => {
				if to.is_js_bigint() {
					if check && let Some(int) = self.types.as_int(target).copied() {
						let helper = self.helper("faConvertInteger");
						return js::call(
							helper,
							&[
								value.text,
								format!("{}n", int.min),
								format!("{}n", int.max),
								js::string_literal(&self.types.display(target)),
							],
						);
					}
					return value;
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
					return js::call(helper, &[value.text, location]);
				}
				let Some(int) = self.types.as_int(target).copied() else {
					return value;
				};
				let helper = self.helper("faConvertInteger");
				let checked = js::call(
					helper,
					&[
						value.text,
						format!("{}n", int.min),
						format!("{}n", int.max),
						js::string_literal(&self.types.display(target)),
					],
				);
				js::call("Number", &[checked.text])
			}
			(Kind::Int(to), Kind::Float(_)) => {
				if !check
					&& let Some(interval) = self.analysis.interval(n)
					&& let Some(exact) = interval.as_exact()
				{
					return if to.is_js_bigint() {
						big_int_constant(exact)
					} else {
						self.number_constant(n, exact)
					};
				}
				if !check {
					let truncated = js::call("Math.trunc", &[value.text]);
					return if to.is_js_bigint() {
						js::call("BigInt", &[truncated.text])
					} else {
						truncated
					};
				}
				let helper = self.helper("faFloatToInteger");
				let integer = js::call(helper, &[value.text, location.clone()]);
				let range = self.types.interval_of(target);
				if to.is_js_bigint() {
					let big = js::call("BigInt", &[integer.text]);
					return self.range_check(big, target, IntRepr::BigInt, &location);
				}
				match range {
					Some(range) if !range.contains_interval(js_safe_range()) => {
						self.range_check(integer, target, IntRepr::Number, &location)
					}
					_ => integer,
				}
			}
			(Kind::Float(bits), Kind::Int(_) | Kind::BigInt) => {
				let number = match self.repr(source) {
					Repr::BigInt => js::call("Number", std::slice::from_ref(&value.text)),
					// integers are never observed as `-0` (D-76)
					_ => js::binary(
						&value,
						"+",
						&Expr::constant("0", Prec::Primary),
						Prec::Additive,
					),
				};
				match bits {
					FloatBits::F64 => number,
					FloatBits::F32 => {
						let inner = match self.repr(source) {
							Repr::BigInt => number,
							_ => value,
						};
						js::call("Math.fround", &[inner.text])
					}
				}
			}
			(Kind::Float(FloatBits::F32), Kind::Float(FloatBits::F64) | Kind::Percentage) => {
				js::call("Math.fround", &[value.text])
			}
			(Kind::Float(_), _) => value,
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

	/// The value of a float literal type, if `ty` is one.
	#[allow(dead_code)]
	pub(crate) fn float_literal_value(&self, ty: TypeId) -> Option<f64> {
		match self.types.kind(ty) {
			TypeKind::FloatLit(_, constant) => match self.types.const_value(constant) {
				ConstValue::Float(bits) => Some(f64::from_bits(bits)),
				_ => None,
			},
			_ => None,
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
