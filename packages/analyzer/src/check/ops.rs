//! Comparisons, equality, `is`, `is in`, logic, `??`, `?.`, and conditions:
//! the flow states where a condition is true and where it is false (design
//! §3.7). Integer comparisons refine the intervals of the local names they
//! compare (values brief §6.5, rule 2); union narrowing comes with M1b.

use std::sync::Arc;

use ethnum::I256;
use smallvec::SmallVec;

use crate::check::Walker;
use crate::check::numbers::NumKind;
use crate::diagnostics::codes::values::{E_BOOL_1, E_CMP_1, E_CMP_2, E_CMP_3, E_CMP_4, W_NUM_2};
use crate::flow::{FlowState, IntervalSet};
use crate::ids::{NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, CompareOp, CompareOps, LogicOp, NodeList, View};
use crate::types::int::INFINITY;
use crate::types::{Interval, Mode, TypeFlags, TypeKind};

/// A test between two integer values that refines them.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Test {
	Order(CompareOp),
	Equal,
	NotEqual,
}

impl Test {
	fn negate(self) -> Test {
		match self {
			Test::Order(CompareOp::Less) => Test::Order(CompareOp::GreaterOrEqual),
			Test::Order(CompareOp::LessOrEqual) => Test::Order(CompareOp::Greater),
			Test::Order(CompareOp::Greater) => Test::Order(CompareOp::LessOrEqual),
			Test::Order(CompareOp::GreaterOrEqual) => Test::Order(CompareOp::Less),
			Test::Equal => Test::NotEqual,
			Test::NotEqual => Test::Equal,
		}
	}

	/// The same test with the operands swapped (`a < b` is `b > a`).
	fn flip(self) -> Test {
		match self {
			Test::Order(CompareOp::Less) => Test::Order(CompareOp::Greater),
			Test::Order(CompareOp::LessOrEqual) => Test::Order(CompareOp::GreaterOrEqual),
			Test::Order(CompareOp::Greater) => Test::Order(CompareOp::Less),
			Test::Order(CompareOp::GreaterOrEqual) => Test::Order(CompareOp::LessOrEqual),
			other => other,
		}
	}

	/// The values of `x` for which `x test other` may hold.
	fn restrict(self, x: &IntervalSet, other: &IntervalSet) -> Option<IntervalSet> {
		let (lo, hi) = (other.hull.lo, other.hull.hi);
		let below = |bound: I256| Interval::new(-INFINITY, bound);
		let above = |bound: I256| Interval::new(bound, INFINITY);
		// an infinite bound stays infinite
		let step = |bound: I256, delta: i32| {
			if bound >= INFINITY || bound <= -INFINITY {
				bound
			} else {
				bound + I256::from(delta)
			}
		};
		match self {
			Test::Order(CompareOp::Less) => x.restrict(below(step(hi, -1))),
			Test::Order(CompareOp::LessOrEqual) => x.restrict(below(hi)),
			Test::Order(CompareOp::Greater) => x.restrict(above(step(lo, 1))),
			Test::Order(CompareOp::GreaterOrEqual) => x.restrict(above(lo)),
			Test::Equal => {
				let mut set = x.restrict(other.hull)?;
				for &excluded in &other.excluded {
					set = set.exclude(excluded)?;
				}
				Some(set)
			}
			Test::NotEqual => match other.as_exact() {
				Some(value) => x.exclude(Interval::exact(value)),
				None => Some(x.clone()),
			},
		}
	}

	/// Whether `a test b` holds for every value (`Some(true)`) or none (`Some(false)`).
	fn constant(self, a: Interval, b: Interval) -> Option<bool> {
		let always = |condition: bool, never: bool| {
			if condition {
				Some(true)
			} else if never {
				Some(false)
			} else {
				None
			}
		};
		match self {
			Test::Order(CompareOp::Less) => always(a.hi < b.lo, a.lo >= b.hi),
			Test::Order(CompareOp::LessOrEqual) => always(a.hi <= b.lo, a.lo > b.hi),
			Test::Order(CompareOp::Greater) => always(a.lo > b.hi, a.hi <= b.lo),
			Test::Order(CompareOp::GreaterOrEqual) => always(a.lo >= b.hi, a.hi < b.lo),
			Test::Equal => always(a.as_exact().is_some() && a == b, a.is_disjoint(b)),
			Test::NotEqual => Test::Equal.constant(a, b).map(|equal| !equal),
		}
	}

	fn symbol(self) -> &'static str {
		match self {
			Test::Order(op) => op.symbol(),
			Test::Equal => "==",
			Test::NotEqual => "!=",
		}
	}
}

impl Walker<'_> {
	// ── conditions ────────────────────────────────────────────────────────

	/// Checks a condition and returns the flow states where it is true and
	/// where it is false. `warn`: report conditions made constant by the
	/// operands' types (`W-NUM-2`); off for a comparison used as a value.
	pub(crate) fn condition(&mut self, n: NodeId, warn: bool) -> (FlowState, FlowState) {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Group(inner) => {
				let states = self.condition(inner, warn);
				let ty = self.type_of(inner);
				self.memo(n, |_| ty);
				states
			}
			View::Logic { op, operands } => self.logic_condition(n, op, operands, warn),
			View::Not(value) => {
				let (when_true, when_false) = self.condition(value, warn);
				self.memo(n, |_| TypeId::BOOLEAN);
				(when_false, when_true)
			}
			View::Comparison {
				operands,
				operators,
			} => self.comparison_condition(n, operands, operators, warn),
			View::Equality {
				left,
				right,
				negated,
			} => self.equality_condition(n, left, right, negated, warn),
			View::Boolean(value) => {
				self.check(n, Some(TypeId::BOOLEAN));
				let live = self.body.flow.fork();
				if value {
					(live, FlowState::unreachable())
				} else {
					(FlowState::unreachable(), live)
				}
			}
			_ => {
				let ty = self.check(n, Some(TypeId::BOOLEAN));
				self.sink_boolean(n, ty);
				(self.body.flow.fork(), self.body.flow.fork())
			}
		}
	}

	/// A `Boolean` operand or condition (`E-BOOL-1` otherwise).
	pub(crate) fn sink_boolean(&mut self, n: NodeId, actual: TypeId) {
		if actual.is_unset()
			|| self.s.types.is_recovering(actual)
			|| self.s.types.satisfies(actual, TypeId::BOOLEAN)
		{
			return;
		}
		let hint = match self.s.types.kind(actual) {
			TypeKind::Union(_) => ": test it, ex: `value is not none`",
			TypeKind::Int(_) | TypeKind::Float(_) => ": compare it, ex: `value > 0`",
			TypeKind::String | TypeKind::StringLit(_) => ": test it, ex: `text.isEmpty()`",
			_ => "",
		};
		let message = format!(
			"expected a `Boolean`, found a `{}`{hint}",
			self.display(self.s.types.widen(actual))
		);
		let diagnostic = self.diagnostic(&E_BOOL_1, n, message);
		self.report_flow(diagnostic);
	}

	/// `a and b and c` / `a or b or c`: each operand runs where the previous
	/// ones allow it (short-circuit).
	fn logic_condition(
		&mut self,
		n: NodeId,
		op: LogicOp,
		operands: NodeList<'_>,
		warn: bool,
	) -> (FlowState, FlowState) {
		let mut exits: SmallVec<[FlowState; 4]> = SmallVec::new();
		for operand in operands.iter() {
			let (when_true, when_false) = self.condition(operand, warn);
			let (go_on, exit) = match op {
				LogicOp::And => (when_true, when_false),
				LogicOp::Or => (when_false, when_true),
			};
			exits.push(exit);
			self.body.flow = go_on;
		}
		self.memo(n, |_| TypeId::BOOLEAN);
		let last = self.body.flow.fork();
		let mut exit = FlowState::unreachable();
		for state in &exits {
			exit = self.join_states(&exit, state);
		}
		match op {
			LogicOp::And => (last, exit),
			LogicOp::Or => (exit, last),
		}
	}

	/// `a < b <= c`: the true state applies every test; the false state joins
	/// the states where the first `i` tests hold and the next one fails.
	fn comparison_condition(
		&mut self,
		n: NodeId,
		operands: NodeList<'_>,
		operators: CompareOps<'_>,
		warn: bool,
	) -> (FlowState, FlowState) {
		let nodes: SmallVec<[NodeId; 4]> = operands.iter().collect();
		let types = self.check_operands(&nodes, None);
		self.memo(n, |w| {
			w.comparison_types(n, &nodes, &types, operators);
			TypeId::BOOLEAN
		});
		let mut when_true = self.body.flow.fork();
		let mut when_false = FlowState::unreachable();
		for (index, op) in operators.iter().enumerate() {
			let (Some(&x), Some(&y)) = (nodes.get(index), nodes.get(index + 1)) else {
				break;
			};
			if warn {
				self.warn_constant(n, x, y, Test::Order(op));
			}
			let mut failing = when_true.fork();
			self.refine(&mut failing, x, y, Test::Order(op).negate());
			when_false = self.join_states(&when_false, &failing);
			self.refine(&mut when_true, x, y, Test::Order(op));
		}
		(when_true, when_false)
	}

	/// `a == b`, `a != b`: integer values refine each other.
	fn equality_condition(
		&mut self,
		n: NodeId,
		left: NodeId,
		right: NodeId,
		negated: bool,
		warn: bool,
	) -> (FlowState, FlowState) {
		let types = self.check_operands(&[left, right], None);
		self.memo(n, |w| {
			w.equality_types(n, types[0], types[1]);
			TypeId::BOOLEAN
		});
		let test = if negated { Test::NotEqual } else { Test::Equal };
		if warn {
			self.warn_constant(n, left, right, test);
		}
		let mut when_true = self.body.flow.fork();
		let mut when_false = self.body.flow.fork();
		self.refine(&mut when_true, left, right, test);
		self.refine(&mut when_false, left, right, test.negate());
		(when_true, when_false)
	}

	/// Applies `x test y` to a state: refines the local integer names among
	/// `x` and `y`; the state becomes unreachable when no value can pass.
	fn refine(&mut self, state: &mut FlowState, x: NodeId, y: NodeId, test: Test) {
		if !state.reachable {
			return;
		}
		let saved = std::mem::replace(&mut self.body.flow, std::mem::take(state));
		let (Some(x_set), Some(y_set)) = (self.value_set(x), self.value_set(y)) else {
			*state = std::mem::replace(&mut self.body.flow, saved);
			return;
		};
		let mut possible = true;
		for (node, own, other, test) in
			[(x, &x_set, &y_set, test), (y, &y_set, &x_set, test.flip())]
		{
			let node = self.strip_groups(node);
			let refined = test.restrict(own, other);
			if refined.is_none() {
				possible = false;
				break;
			}
			if refined.as_ref() != Some(own)
				&& let View::Name(_) = syntax::view(&self.tree, node)
				&& let Resolution::Local(symbol) = self.a.resolution(node)
				&& self
					.s
					.types
					.as_int(self.a.symbols.symbol(symbol).ty)
					.is_some()
			{
				self.set_symbol_values(symbol, refined);
			}
		}
		if !possible {
			self.body.flow.set_unreachable();
		}
		*state = std::mem::replace(&mut self.body.flow, saved);
	}

	/// `W-NUM-2` when the operands' types alone decide an integer test
	/// (`if value < 20` with `value: Integer(min = 20)`). Literal operands count
	/// by their value; facts don't (a value bound to a literal is a fact).
	fn warn_constant(&mut self, n: NodeId, x: NodeId, y: NodeId, test: Test) {
		if !self.body.flow.reachable || (self.is_number_literal(x) && self.is_number_literal(y)) {
			return;
		}
		let range = |w: &Self, node: NodeId| w.s.types.interval_of(w.type_of(w.strip_groups(node)));
		let (Some(a), Some(b)) = (range(self, x), range(self, y)) else {
			return;
		};
		if let Some(always) = test.constant(a, b) {
			let text = self.tree.text(n.index()).to_string();
			let message = format!(
				"the condition `{text}` is always {}: the types of its operands decide it ({} {} {})",
				if always { "true" } else { "false" },
				describe_range(a),
				test.symbol(),
				describe_range(b)
			);
			let diagnostic = self.diagnostic(&W_NUM_2, n, message);
			self.report_flow(diagnostic);
		}
	}

	// ── type rules ────────────────────────────────────────────────────────

	/// The operands of an ordering must be ordered (`E-CMP-2`), of one kind
	/// (`E-CMP-1`), and a chain goes in one direction (`E-CMP-3`). Type part.
	fn comparison_types(
		&mut self,
		n: NodeId,
		nodes: &[NodeId],
		types: &[TypeId],
		operators: CompareOps<'_>,
	) {
		let ascending = |op: CompareOp| matches!(op, CompareOp::Less | CompareOp::LessOrEqual);
		let mut directions = operators.iter().map(ascending);
		if let Some(first) = directions.next()
			&& directions.any(|direction| direction != first)
		{
			let diagnostic = self.diagnostic(
				&E_CMP_3,
				n,
				"a comparison chain goes in one direction: use `<`/`<=` or `>`/`>=`, or split it with `and`",
			);
			self.report(diagnostic);
			return;
		}
		for (&node, &ty) in nodes.iter().zip(types) {
			if self.s.types.is_recovering(ty) || ty.is_unset() {
				return;
			}
			if !self.s.types.flags(ty).contains(TypeFlags::ORDERED) {
				let message = format!(
					"a `{}` has no order: `<`, `<=`, `>` and `>=` compare numbers and strings",
					self.display(self.s.types.widen(ty))
				);
				let diagnostic = self.diagnostic(&E_CMP_2, node, message);
				self.report(diagnostic);
				return;
			}
		}
		for pair in types.windows(2) {
			if !self.same_number_kind(pair[0], pair[1]) {
				let message = format!(
					"cannot compare a `{}` with a `{}`",
					self.display(self.s.types.widen(pair[0])),
					self.display(self.s.types.widen(pair[1]))
				);
				let diagnostic = self.diagnostic(&E_CMP_1, n, message);
				self.report(diagnostic);
				return;
			}
		}
	}

	/// Whether two ordered types can be compared: integers of any storage,
	/// floats of one width, `BigInt`s, percentages, text.
	fn same_number_kind(&self, a: TypeId, b: TypeId) -> bool {
		match (self.num_kind(a), self.num_kind(b)) {
			(NumKind::Int(_), NumKind::Int(_)) => true,
			(left, right) => left == right,
		}
	}

	/// The operands of `==` must be compatible (`E-CMP-1`) and comparable
	/// (`E-CMP-4` for functions). Type part.
	fn equality_types(&mut self, n: NodeId, left: TypeId, right: TypeId) {
		if left.is_unset()
			|| right.is_unset()
			|| self.s.types.is_recovering(left)
			|| self.s.types.is_recovering(right)
		{
			return;
		}
		for ty in [left, right] {
			if matches!(self.s.types.kind(ty), TypeKind::Function(_)) {
				let diagnostic =
					self.diagnostic(&E_CMP_4, n, "functions can't be compared with `==`");
				self.report(diagnostic);
				return;
			}
		}
		let (left, right) = (self.s.types.widen(left), self.s.types.widen(right));
		let compatible = match (self.num_kind(left), self.num_kind(right)) {
			(NumKind::Int(_), NumKind::Int(_)) => true,
			(
				a @ (NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage),
				b,
			)
			| (
				b,
				a @ (NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage),
			) if matches!(
				b,
				NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage
			) =>
			{
				a == b
			}
			_ => {
				left != TypeId::VOID
					&& right != TypeId::VOID
					&& (self.s.types.assignable(left, right, Mode::Read).is_ok()
						|| self.s.types.assignable(right, left, Mode::Read).is_ok()
						|| self.optional_numbers_compatible(left, right))
			}
		};
		if !compatible {
			let message = format!(
				"cannot compare a `{}` with a `{}`",
				self.display(self.s.types.widen(left)),
				self.display(self.s.types.widen(right))
			);
			let diagnostic = self.diagnostic(&E_CMP_1, n, message);
			self.report(diagnostic);
		}
	}

	/// An optional integer compared with an integer of another storage (`m == 3` with `m: Int8?`).
	fn optional_numbers_compatible(&self, a: TypeId, b: TypeId) -> bool {
		let int_member = |ty: TypeId| self.int_target(ty);
		int_member(a).is_some() && int_member(b).is_some()
	}

	// ── value position ────────────────────────────────────────────────────

	/// A condition used as a value: both outcomes continue.
	fn condition_value(&mut self, n: NodeId) -> TypeId {
		let (when_true, when_false) = self.condition(n, false);
		self.body.flow = self.join_states(&when_true, &when_false);
		self.type_of(n)
	}

	/// `a == b`, `a != b`
	pub(super) fn check_equality(
		&mut self,
		n: NodeId,
		_left: NodeId,
		_right: NodeId,
		_negated: bool,
	) -> TypeId {
		self.condition_value(n)
	}

	/// `a < b <= c`: `a < b and b <= c`, each middle operand evaluated once.
	pub(super) fn check_comparison(
		&mut self,
		n: NodeId,
		_operands: NodeList<'_>,
		_operators: CompareOps<'_>,
	) -> TypeId {
		self.condition_value(n)
	}

	/// `and` / `or`, folding left.
	pub(super) fn check_logic(
		&mut self,
		n: NodeId,
		_op: LogicOp,
		_operands: NodeList<'_>,
	) -> TypeId {
		self.condition_value(n)
	}

	/// `not x`
	pub(super) fn check_not(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.condition_value(n)
	}

	/// `a ?? b ?? c`, folding left.
	pub(super) fn check_fallback(
		&mut self,
		n: NodeId,
		operands: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		// optionals.rs (E1)
		self.check_fallback_value(n, operands, expected)
	}

	/// `a is T`, `a is not T`
	pub(super) fn check_is(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_ty: NodeId,
		_negated: bool,
	) -> TypeId {
		self.unsupported(n, "`is`")
	}

	/// `a is in b`, `a is not in b`
	pub(super) fn check_is_in(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_collection: NodeId,
		_negated: bool,
	) -> TypeId {
		self.unsupported(n, "`is in`")
	}

	/// `a?.b?.c`
	pub(super) fn check_optional_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		// optionals.rs (E1)
		self.check_optional_chain(n, target, members)
	}
}

fn describe_range(interval: Interval) -> String {
	match interval.as_exact() {
		Some(value) => value.to_string(),
		None => {
			let bound = |value: I256| {
				if value >= INFINITY || value <= -INFINITY {
					"∞".to_string()
				} else {
					value.to_string()
				}
			};
			format!("{}..={}", bound(interval.lo), bound(interval.hi))
		}
	}
}
