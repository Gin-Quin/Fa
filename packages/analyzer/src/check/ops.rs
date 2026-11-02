//! Comparisons, equality, `is`, `is in`, logic, `??`, `?.`, and conditions:
//! the flow states where a condition is true and where it is false (design
//! §3.7). Integer comparisons refine the intervals of the local names they
//! compare (values brief §6.5, rule 2); union narrowing comes with M1b.

use std::sync::Arc;

use ethnum::I256;
use smallvec::SmallVec;

use crate::check::Walker;
use crate::check::numbers::NumKind;
use crate::diagnostics::codes::values::{
	E_BOOL_1, E_CMP_1, E_CMP_2, E_CMP_3, E_CMP_4, E_CMP_5, E_OPT_1, E_OPT_6, E_RNG_4, W_NUM_2,
	W_RNG_1,
};
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
			View::Is { value, ty, negated } => {
				self.check(value, None);
				let (when_true, when_false) = self.pattern_test(n, value, ty, Some(!negated));
				self.memo(n, |_| TypeId::BOOLEAN);
				if negated {
					(when_false, when_true)
				} else {
					(when_true, when_false)
				}
			}
			View::Boolean(value) => {
				self.check(n, Some(TypeId::BOOLEAN));
				let live = self.body.flow.fork();
				if value {
					(live, FlowState::impossible())
				} else {
					(FlowState::impossible(), live)
				}
			}
			_ => {
				let ty = self.check(n, Some(TypeId::BOOLEAN));
				self.sink_boolean(n, ty);
				let when_false = self.body.flow.fork();
				(std::mem::take(&mut self.body.flow), when_false)
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
		// `x == none`: a none test (narrowing.rs)
		let is_none = |w: &Self, node: NodeId| {
			matches!(syntax::view(&w.tree, w.strip_groups(node)), View::None)
		};
		let none_side = match (is_none(self, left), is_none(self, right)) {
			(false, true) => Some(left),
			(true, false) => Some(right),
			_ => None,
		};
		self.memo(n, |w| {
			// type level: the declared types, not the narrowed ones (D-28)
			let declared = |w: &Self, node: NodeId, ty: TypeId| {
				if w.read_local(node).is_some() {
					w.declared_type_of(node)
				} else {
					ty
				}
			};
			let (left_ty, right_ty) = (declared(w, left, types[0]), declared(w, right, types[1]));
			w.equality_types(n, left_ty, right_ty);
			if let Some(value) = none_side {
				let declared = w.declared_type_of(value);
				w.type_level_verdict(n, declared, TypeId::NONE, !negated);
			}
			TypeId::BOOLEAN
		});
		if let Some(value) = none_side {
			return self.none_test(n, value, !negated, None);
		}
		// `c == Color.Red`: a member of an enumeration is a single value, so
		// the test narrows like `c is Color.Red` (values brief D-55)
		let member_test =
			[(left, types[1]), (right, types[0])]
				.into_iter()
				.find(|&(local, member)| {
					self.read_local(local).is_some()
						&& matches!(self.s.types.kind(member), TypeKind::EnumMember(..))
				});
		if let Some((local, member)) = member_test {
			let (equal, different) = self.type_test(n, local, member, None);
			return if negated {
				(different, equal)
			} else {
				(equal, different)
			};
		}
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
			let original = node;
			let node = self.strip_groups(node);
			let refined = test.restrict(own, other);
			if refined.is_none() {
				possible = false;
				break;
			}
			// `c.length >= 2`: a length fact of `c` (destructuring, index proofs)
			if let Some(refined) = refined.as_ref()
				&& let Some(collection) = self.length_access(node)
			{
				self.refine_length(collection, refined.hull);
			}
			if refined.as_ref() != Some(own)
				&& let View::Name(_) = syntax::view(&self.tree, node)
				&& let Resolution::Local(symbol) = self.a.resolution(node)
				&& self.int_range(self.a.symbols.symbol(symbol).ty).is_some()
			{
				self.set_symbol_values(symbol, refined);
			}
			// `i < c.length - k`: a relation for index proofs (proofs.rs)
			if let Test::Order(op @ (CompareOp::Less | CompareOp::LessOrEqual)) = test
				&& let Some(symbol) = self.read_local(node)
				&& let Some(relation) =
					self.length_relation(other_node(x, y, original), op == CompareOp::Less)
			{
				self.relate_below(symbol, relation);
			}
		}
		if !possible {
			self.body.flow.set_impossible();
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
			if self.has_none(ty) {
				let text = self.tree.text(node.index()).to_string();
				let message = format!(
					"`{text}` may be `none`: test it with `if {text} is not none` first, or use `??`"
				);
				let diagnostic = self.diagnostic(&E_OPT_1, node, message);
				self.report(diagnostic);
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
			// a function, or a type holding one, has no equality (`TY025`)
			let comparable = self.s.types.flags(ty).contains(TypeFlags::COMPARABLE)
				|| matches!(ty, TypeId::NEVER | TypeId::VOID);
			if !comparable || matches!(self.s.types.kind(ty), TypeKind::Function(_)) {
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

	/// `a ?? b ?? c`, folding left: the left side must be optional
	/// (`E-OPT-6`); the result is the left without `None`, joined with the
	/// right (values brief §9, D-100).
	pub(super) fn check_fallback(
		&mut self,
		n: NodeId,
		operands: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let Some(first) = operands.first() else {
			return TypeId::UNKNOWN;
		};
		// the errors of the left side are not what `??` handles (`E-OPT-6`, "use `catch`")
		self.open_handler(crate::check::errors::HandlerKind::Try, n);
		let first_expected = expected.map(|expected| self.s.types.optional(expected));
		let mut left_ty = self.check(first, first_expected);
		let left_errors = self.close_handler();
		let mut left_declared = self.declared_type_of(first);
		let mut left_node = first;
		let mut problems: SmallVec<[(NodeId, bool, TypeId); 2]> = SmallVec::new();
		let first_optional =
			self.has_none(left_declared) || self.s.types.is_recovering(left_declared);
		if left_errors != TypeId::NEVER && first_optional {
			self.can_fail(first, left_errors);
		}
		for right in operands.iter().skip(1) {
			let recovering = self.s.types.is_recovering(left_declared);
			if !recovering && !self.has_none(left_declared) {
				let failing = left_node == first && left_errors != TypeId::NEVER;
				problems.push((left_node, failing, left_declared));
			}
			let some = if recovering {
				left_ty
			} else {
				self.without_none(left_ty)
			};
			// the right side runs when the left is `none`
			let entry = self.body.flow.fork();
			if let Some(symbol) = self.read_local(left_node) {
				let current = self.current_type(symbol);
				if self.has_none(current) {
					self.narrow_symbol(symbol, TypeId::NONE);
				}
			}
			let hint =
				expected.or_else(|| (some != TypeId::NEVER).then(|| self.s.types.widen(some)));
			let right_ty = self.check(right, hint);
			let end = std::mem::take(&mut self.body.flow);
			self.body.flow = self.join_states(&entry, &end);
			left_ty = if right_ty == TypeId::NEVER || self.s.types.is_recovering(right_ty) {
				some
			} else if some == TypeId::NEVER {
				self.s.types.widen(right_ty)
			} else {
				let right_ty = self.s.types.widen(right_ty);
				self.s.types.union(&[some, right_ty])
			};
			// the next `??` looks at the declared types (narrowing doesn't count, D-28)
			let right_declared = self.declared_type_of(right);
			let some_declared = if recovering {
				left_declared
			} else {
				self.without_none(left_declared)
			};
			left_declared =
				if right_declared == TypeId::NEVER || self.s.types.is_recovering(right_declared) {
					some_declared
				} else if some_declared == TypeId::NEVER {
					self.s.types.widen(right_declared)
				} else {
					let right_declared = self.s.types.widen(right_declared);
					self.s.types.union(&[some_declared, right_declared])
				};
			left_node = right;
		}
		let result = left_ty;
		self.memo(n, |w| {
			for (node, failing, ty) in problems {
				let message = if failing {
					"`??` replaces `none`, not an error: handle the error with `catch`, ex: `… catch error => fallback`".to_string()
				} else {
					format!(
						"`??` only applies to optionals, and a `{}` is never `none`: remove `?? …`",
						w.display(ty)
					)
				};
				let diagnostic = w.diagnostic(&E_OPT_6, node, message);
				w.report(diagnostic);
			}
			result
		})
	}

	/// `a is T`, `a is not T`
	pub(super) fn check_is(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_ty: NodeId,
		_negated: bool,
	) -> TypeId {
		self.condition_value(n)
	}

	/// The test of `value is pattern` (the value is checked): `none`, a
	/// literal, an integer range, a type, or alternatives `P1 | P2`.
	pub(crate) fn pattern_test(
		&mut self,
		n: NodeId,
		value: NodeId,
		pattern: NodeId,
		verdict: Option<bool>,
	) -> (FlowState, FlowState) {
		let tree = Arc::clone(&self.tree);
		let inner = self.strip_groups(pattern);
		match syntax::view(&tree, inner) {
			View::None => {
				self.check(pattern, None);
				self.none_test(n, value, true, verdict)
			}
			View::Union { operands } => {
				// alternatives: true when one holds, false when none does
				let mut trues: SmallVec<[FlowState; 4]> = SmallVec::new();
				for alternative in operands.iter() {
					let (when_true, when_false) = self.pattern_test(n, value, alternative, None);
					trues.push(when_true);
					self.body.flow = when_false;
				}
				self.memo(pattern, |_| TypeId::BOOLEAN);
				let when_false = std::mem::take(&mut self.body.flow);
				let mut when_true = FlowState::impossible();
				for state in &trues {
					when_true = self.join_states(&when_true, state);
				}
				(when_true, when_false)
			}
			View::Range {
				start,
				end,
				inclusive,
			} => {
				let mut bounds = [None, None];
				for (slot, bound) in [start, end].into_iter().enumerate() {
					if let Some(bound) = bound {
						let ty = self.check(bound, Some(TypeId::INTEGER));
						self.sink(bound, ty, TypeId::INTEGER);
						bounds[slot] = self.value_set(bound).and_then(|set| set.as_exact());
					}
				}
				self.memo(pattern, |_| TypeId::RANGE);
				// a range pattern matches integers (`E-RNG-4`)
				let subject = self.type_of(self.strip_groups(value));
				if !subject.is_unset()
					&& !self.s.types.is_recovering(subject)
					&& !self.is_integer_subject(subject)
				{
					let diagnostic = self.diagnostic(
						&E_RNG_4,
						pattern,
						"a range pattern matches integers only: compare floats with `<` and `>=`",
					);
					self.report_once(pattern, diagnostic);
				}
				self.range_test(value, bounds, (start.is_some(), end.is_some()), inclusive)
			}
			View::Integer(_)
			| View::Float(_)
			| View::String(_)
			| View::Boolean(_)
			| View::Negate(_) => {
				let actual = self.type_of(self.strip_groups(value));
				let hint =
					(!self.s.types.is_recovering(actual)).then(|| self.s.types.widen(actual));
				let literal = self.check(pattern, hint);
				if verdict.is_some() {
					self.memo(n, |w| {
						w.equality_types(n, actual, literal);
						TypeId::BOOLEAN
					});
				}
				let mut when_true = self.body.flow.fork();
				let mut when_false = std::mem::take(&mut self.body.flow);
				self.refine(&mut when_true, value, pattern, Test::Equal);
				self.refine(&mut when_false, value, pattern, Test::NotEqual);
				(when_true, when_false)
			}
			// `x is passing` with a range value: `E-CMP-5`
			View::Name(name) if self.names_range_value(name) => {
				let text = self.tree.name(name).to_string();
				self.memo(pattern, |w| {
					let diagnostic = w.diagnostic(
						&E_CMP_5,
						pattern,
						format!(
							"`{text}` is a value, not a pattern: test the membership with `is in {text}`"
						),
					);
					w.report(diagnostic);
					TypeId::UNKNOWN
				});
				(self.body.flow.fork(), self.body.flow.fork())
			}
			_ => {
				// a pattern already typed (a `when` arm) keeps its type
				let known = self.type_of(pattern);
				let ty = if known.is_unset() {
					self.check_type_expr(pattern)
				} else {
					known
				};
				self.type_test(n, value, ty, verdict)
			}
		}
	}

	/// Whether a range can test a value of this type: integers, a union of
	/// integers (optional ones too: `none` is outside every range).
	fn is_integer_subject(&self, ty: TypeId) -> bool {
		self.s
			.types
			.union_members(&ty)
			.iter()
			.all(|&member| member == TypeId::NONE || self.s.types.as_int(member).is_some())
	}

	/// Whether a name is a local value holding a `Range` (`x is passing`).
	fn names_range_value(&mut self, name: crate::syntax::Name) -> bool {
		let sym = self.s.types.syms.intern(self.tree.name(name));
		self.a
			.symbols
			.lookup(self.body.scope, sym)
			.is_some_and(|symbol| {
				let symbol = self.a.symbols.symbol(symbol);
				symbol.kind != crate::symbols::SymbolKind::Type && symbol.ty == TypeId::RANGE
			})
	}

	/// `value is a..b` on an integer local: the range, and its complement.
	fn range_test(
		&mut self,
		value: NodeId,
		bounds: [Option<I256>; 2],
		given: (bool, bool),
		inclusive: bool,
	) -> (FlowState, FlowState) {
		let mut when_true = self.body.flow.fork();
		let mut when_false = self.body.flow.fork();
		let Some(symbol) = self.read_local(value) else {
			return (when_true, when_false);
		};
		let Some(values) = self.symbol_values(symbol) else {
			return (when_true, when_false);
		};
		// a bound that is not a constant: no refinement
		if (given.0 && bounds[0].is_none()) || (given.1 && bounds[1].is_none()) {
			return (when_true, when_false);
		}
		let lo = bounds[0].unwrap_or(-INFINITY);
		let hi = match bounds[1] {
			Some(end) if inclusive => end,
			Some(end) => end - 1,
			None => INFINITY,
		};
		if lo > hi {
			return (FlowState::impossible(), when_false);
		}
		let range = Interval::new(lo, hi);
		// an optional is an integer where the test holds
		let current = self.current_type(symbol);
		if self.has_none(current) {
			let integer = self.without_none(current);
			self.narrow_in(&mut when_true, symbol, integer);
		}
		for (state, set) in [
			(&mut when_true, values.restrict(range)),
			(&mut when_false, values.exclude(range)),
		] {
			let saved = std::mem::replace(&mut self.body.flow, std::mem::take(state));
			match set {
				Some(set) => self.set_symbol_values(symbol, Some(set)),
				None => self.body.flow.set_impossible(),
			}
			*state = std::mem::replace(&mut self.body.flow, saved);
		}
		(when_true, when_false)
	}

	/// `a is in b`, `a is not in b`: membership in a `Range` value (values
	/// brief §12); `W-RNG-1` with a range literal (`x is 0..10` says it).
	/// Collections: with the collections.
	pub(super) fn check_is_in(
		&mut self,
		n: NodeId,
		value: NodeId,
		collection: NodeId,
		_negated: bool,
	) -> TypeId {
		let collection_ty = self.check(collection, None);
		if collection_ty != TypeId::RANGE {
			return self.unsupported(n, "`is in` on a collection");
		}
		let value_ty = self.check(value, Some(TypeId::INTEGER));
		self.memo(n, |w| {
			if !w.s.types.is_recovering(value_ty) && !w.is_integer_subject(value_ty) {
				let message = format!(
					"a range holds integers, not a `{}`: compare floats with `<` and `>=`",
					w.display(w.s.types.widen(value_ty))
				);
				let diagnostic = w.diagnostic(&E_RNG_4, value, message);
				w.report(diagnostic);
			}
			let literal = w.strip_groups(collection);
			if let View::Range { .. } = w.view(literal) {
				let text = w.tree.text(literal.index()).to_string();
				let diagnostic = w.diagnostic(
					&W_RNG_1,
					n,
					format!("`is in` takes a range value: with a range literal, write `is {text}`"),
				);
				w.report(diagnostic);
			}
			TypeId::BOOLEAN
		})
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

/// The other operand of a pair, given one of them.
fn other_node(x: NodeId, y: NodeId, node: NodeId) -> NodeId {
	if node == x { y } else { x }
}
