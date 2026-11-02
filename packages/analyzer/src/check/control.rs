//! Control flow: `if`, `when`, `while`, `for`, `loop`, `do`, `break`,
//! `continue`, `return`; forks, joins, block values and the loop fixpoint
//! (design §3.3).
//!
//! A loop walks its body once per round, from a head state that grows (join
//! with the ends of the body and the `continue`s, widened from the second
//! round, values brief D-96) until it is stable. Every node of the body is
//! visited every round: types are memoized, check bits and flow diagnostics
//! are recomputed, and only the diagnostics of the stable round are kept
//! (decision A3).

use std::sync::Arc;

use smallvec::SmallVec;

use crate::check::numbers::NumKind;
use crate::check::{LoopCx, Walker};
use crate::diagnostics::codes::values::{
	E_DO_1, E_IF_1, E_IF_2, E_IF_3, E_LOOP_1, E_LOOP_3, E_LOOP_6, E_LOOP_7, E_LOOP_11, E_LOOP_12,
	E_LOOP_13, E_RNG_3, W_BOOL_1, W_DO_2, W_LOOP_1,
};
use crate::flow::{FlowState, IntervalSet, PlaceRoot};
use crate::ids::{NodeId, PlaceId, TypeId};
use crate::symbols::{ScopeKind, SymbolKind};
use crate::syntax::{self, Name, NodeList, View, WhenArms};
use crate::types::int::INFINITY;
use crate::types::{IntStorage, Interval};

/// The kind of a loop, for `break` and `continue`.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LoopKind {
	/// `for … in …`
	For,
	/// `while …`, `while let …`
	While,
	/// `loop`: may have a value (`break value`).
	Loop,
}

/// The rounds after which a loop's changing facts are dropped (design §3.3).
const WIDENING_CAP: u32 = 8;

/// The descending rounds after the widened fixpoint of a loop: each one
/// walks the body from a smaller head, which gives back the bounds a
/// widening sent to the type's limits.
const NARROWING_ROUNDS: u32 = 2;

impl Walker<'_> {
	// ── blocks ────────────────────────────────────────────────────────────

	/// The node giving a block its value (its last statement), for sinks and messages.
	fn block_value_node(&self, block: NodeId) -> NodeId {
		match syntax::view(&self.tree, block) {
			View::Block { statements } => statements.last().unwrap_or(block),
			_ => block,
		}
	}

	/// Joins two flow states.
	pub(crate) fn join_states(&mut self, a: &FlowState, b: &FlowState) -> FlowState {
		let mut joined = a.join(b, &mut self.facts, &mut self.s.types);
		if joined == *a || joined == *b {
			return joined;
		}
		// a refinement of an immutable binding that no longer narrows it
		let candidates: SmallVec<[PlaceId; 16]> = if joined.facts.same_base(&a.facts) {
			joined.facts.overlay_places()
		} else {
			joined
				.facts
				.iter()
				.map(|(place, _)| place)
				.filter(|&place| self.places.is_bound(place))
				.collect()
		};
		for place in candidates {
			if !self.places.is_bound(place) {
				continue;
			}
			if let PlaceRoot::Symbol(symbol) = self.places.get(place).root
				&& joined.facts.get(place).is_some()
				&& self.facts.binding(symbol) == joined.facts.get(place)
			{
				joined.facts.set(place, crate::ids::FactId(0));
			}
		}
		joined
	}

	// ── if ────────────────────────────────────────────────────────────────

	/// `if condition { … } else …`: a statement (`Void`) when
	/// `self.is_statement(n)`, otherwise a value (values brief §17.1, D-29).
	pub(super) fn check_if(
		&mut self,
		n: NodeId,
		condition: NodeId,
		then_body: NodeId,
		else_body: Option<NodeId>,
		compile_time: bool,
		expected: Option<TypeId>,
	) -> TypeId {
		if compile_time {
			return self.unsupported(n, "`@if`");
		}
		let (when_true, when_false) = self.check_cond(condition);
		if self.is_statement(n) {
			self.body.flow = when_true;
			self.check_stmt(then_body);
			let end_then = std::mem::take(&mut self.body.flow);
			self.body.flow = when_false;
			if let Some(else_body) = else_body {
				self.check_stmt(else_body);
			}
			let end_else = std::mem::take(&mut self.body.flow);
			self.body.flow = self.join_states(&end_then, &end_else);
			return self.memo(n, |w| {
				w.literal_condition_warning(condition, false);
				TypeId::VOID
			});
		}
		let Some(else_body) = else_body else {
			self.body.flow = when_true;
			self.check_block_value(then_body, expected);
			let end_then = std::mem::take(&mut self.body.flow);
			self.body.flow = self.join_states(&end_then, &when_false);
			return self.memo(n, |w| {
				w.literal_condition_warning(condition, false);
				let diagnostic = w.diagnostic(
					&E_IF_1,
					n,
					"this `if` is used as a value, so it needs an `else`: add `else { … }`",
				);
				w.report(diagnostic);
				TypeId::UNKNOWN
			});
		};
		// a literal branch takes the type of the other one: check the other first
		let then_value = self.block_value_node(then_body);
		let else_value = self.block_value_node(else_body);
		let then_literal = self.is_number_literal(then_value);
		let else_literal = self.is_number_literal(else_value);
		let mut branches = [
			(
				then_body,
				then_value,
				when_true,
				TypeId::UNSET,
				FlowState::unreachable(),
			),
			(
				else_body,
				else_value,
				when_false,
				TypeId::UNSET,
				FlowState::unreachable(),
			),
		];
		let order: [usize; 2] = if then_literal && !else_literal {
			[1, 0]
		} else {
			[0, 1]
		};
		let mut hint = None;
		// the integer values of the branches that reach the end (`None` inside: not all integers)
		let mut values: Option<Option<Interval>> = Some(None);
		for index in order {
			let (body, value, ref start, _, _) = branches[index];
			self.body.flow = start.fork();
			let literal = self.is_number_literal(value);
			let branch_expected = expected.or(if literal { hint } else { None });
			let ty = self.check_block_value(body, branch_expected).0;
			if let Some(expected) = expected
				&& ty != TypeId::VOID
			{
				self.sink(value, ty, expected);
			}
			if !literal && ty != TypeId::NEVER && !self.s.types.is_recovering(ty) {
				hint = Some(self.s.types.widen(ty));
			}
			if self.body.flow.reachable && ty != TypeId::NEVER {
				let set = self.value_set(value);
				values = values.and_then(|hull| {
					let set = set?;
					Some(Some(
						hull.map_or(set.hull, |hull: Interval| hull.hull(set.hull)),
					))
				});
			}
			branches[index].3 = ty;
			branches[index].4 = std::mem::take(&mut self.body.flow);
		}
		self.body.flow = self.join_states(&branches[0].4, &branches[1].4);
		let types = [
			(branches[0].0, branches[0].3),
			(branches[1].0, branches[1].3),
		];
		let ty = self.memo(n, |w| {
			w.literal_condition_warning(condition, false);
			w.branches_type(n, &types, expected)
		});
		// the values of the `if`: the hull of its branches'
		match (values.flatten(), self.s.types.interval_of(ty)) {
			(Some(hull), Some(range)) => match hull.intersect(range) {
				Some(inside) => self.record_interval(n, inside, range),
				None => {
					self.a.intervals.remove(&n);
				}
			},
			_ => {
				self.a.intervals.remove(&n);
			}
		}
		ty
	}

	/// The type of a value with several branches (`if`, `when`, `loop` with
	/// `break` values): the expected type, or the union of the widened branch
	/// types without one (`Never` branches add nothing, D-29); `E-IF-3` for a
	/// branch without value, `E-IF-2` for an invalid union. Type part.
	pub(crate) fn branches_type(
		&mut self,
		n: NodeId,
		branches: &[(NodeId, TypeId)],
		expected: Option<TypeId>,
	) -> TypeId {
		let mut types: SmallVec<[TypeId; 4]> = SmallVec::new();
		for &(branch, ty) in branches {
			if ty == TypeId::VOID {
				let diagnostic = self.diagnostic(
					&E_IF_3,
					branch,
					"this branch has no value: it ends with a statement, so end it with the value it gives",
				);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			if self.s.types.is_recovering(ty) || ty.is_unset() {
				return TypeId::UNKNOWN;
			}
			if ty != TypeId::NEVER {
				types.push(self.s.types.widen(ty));
			}
		}
		if types.is_empty() {
			return TypeId::NEVER;
		}
		if let Some(expected) = expected {
			return expected;
		}
		// integers: the common base type (`Int8 | Int16` is an `Int16`)
		let kinds: SmallVec<[NumKind; 4]> = types.iter().map(|&ty| self.num_kind(ty)).collect();
		if kinds.iter().all(|kind| matches!(kind, NumKind::Int(_))) {
			let mut base: Option<IntStorage> = None;
			for kind in &kinds {
				let NumKind::Int(storage) = *kind else {
					continue;
				};
				base = match base {
					None => Some(storage),
					Some(current) => match current.common_base(storage) {
						Some(common) => Some(common),
						None => {
							self.report_branch_mix(n, types[0], types[1]);
							return TypeId::UNKNOWN;
						}
					},
				};
			}
			return TypeId::of_storage(base.unwrap_or(IntStorage::Default));
		}
		let numbers: SmallVec<[NumKind; 4]> = kinds
			.iter()
			.copied()
			.filter(|kind| {
				matches!(
					kind,
					NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage
				)
			})
			.collect();
		let mixed = numbers.iter().any(|kind| {
			let first = numbers[0];
			!matches!((first, *kind), (NumKind::Int(_), NumKind::Int(_))) && first != *kind
		});
		if mixed {
			let (a, b) = (types[0], types[1]);
			self.report_branch_mix(n, a, b);
			return TypeId::UNKNOWN;
		}
		let union = self.s.types.union(&types);
		if let Some((a, b)) = self.s.types.indistinguishable_pair(union) {
			self.report_branch_mix(n, a, b);
			return TypeId::UNKNOWN;
		}
		union
	}

	fn report_branch_mix(&mut self, n: NodeId, a: TypeId, b: TypeId) {
		let message = format!(
			"the branches give a `{}` and a `{}`, which don't make a valid union: annotate the value, or convert one branch",
			self.display(a),
			self.display(b)
		);
		let diagnostic = self.diagnostic(&E_IF_2, n, message);
		self.report(diagnostic);
	}

	/// `W-BOOL-1` for a literal `true`/`false` condition (values brief D-38). Type part.
	fn literal_condition_warning(&mut self, condition: NodeId, is_while: bool) {
		let inner = self.strip_groups(condition);
		let View::Boolean(value) = syntax::view(&self.tree, inner) else {
			return;
		};
		let message = match (is_while, value) {
			(true, true) => "this condition is always true: write `loop { … }`".to_string(),
			(_, value) => format!("this condition is always {value}"),
		};
		let diagnostic = self.diagnostic(&W_BOOL_1, condition, message);
		self.report(diagnostic);
	}

	/// Before each statement of a block: `UNREACHABLE` when no execution
	/// reaches it, and `W-DO-2` on the first one after a `Never` statement
	/// (not when the facts alone rule the path out). `reported` starts true
	/// for a block that is itself unreachable.
	pub(crate) fn note_reachability(&mut self, statement: NodeId, reported: &mut bool) {
		self.clear_flag(statement, crate::analysis::NodeFlags::UNREACHABLE);
		if self.body.flow.reachable {
			return;
		}
		self.set_flag(statement, crate::analysis::NodeFlags::UNREACHABLE);
		if *reported || self.body.flow.vacuous {
			return;
		}
		*reported = true;
		let diagnostic = self.diagnostic(
			&W_DO_2,
			statement,
			"this code is never run: the code before it always leaves (`return`, `throw`, `break`, `continue`)",
		);
		self.report_flow(diagnostic);
	}

	// ── when (M1b) ────────────────────────────────────────────────────────

	/// `when subject { … }`
	pub(super) fn check_when(
		&mut self,
		n: NodeId,
		subject: NodeId,
		arms: WhenArms<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		// typing and patterns: patterns.rs (E1)
		self.check_when_value(n, subject, arms, expected)
	}

	// ── loops ─────────────────────────────────────────────────────────────

	/// Pushes the context of a loop.
	fn enter_loop(
		&mut self,
		n: NodeId,
		kind: LoopKind,
		expected: Option<TypeId>,
		value_used: bool,
	) {
		let function_depth = self.body.functions.len();
		self.body.loops.push(LoopCx {
			node: n,
			breaks: Vec::new(),
			continues: Vec::new(),
			value: TypeId::UNSET,
			kind,
			expected,
			value_used,
			function_depth,
			break_types: Vec::new(),
			break_values: Some(None),
		});
	}

	/// The loop fixpoint (design §3.3). `round` walks the body once from the
	/// current state (the head) and returns the state where the loop exits
	/// by its condition (`None`: the head itself, for `for`; an unreachable
	/// state for `loop`). Returns that exit state of the stable round; the
	/// `break` states are left in the loop context.
	fn run_loop(&mut self, mut round: impl FnMut(&mut Self) -> Option<FlowState>) -> FlowState {
		let node = self.body.loops.last().map(|cx| cx.node);
		let mut head = self.body.flow.fork();
		// walked again inside an outer loop: start from the previous stable head
		if head.reachable
			&& let Some(previous) = node.and_then(|node| self.facts.loop_head(node)).cloned()
		{
			head = self.join_states(&head, &previous);
		}
		let entry = head.clone();
		let mut rounds = 0u32;
		// after the widened fixpoint: the descending rounds (no widening)
		let mut narrowing = 0u32;
		loop {
			self.push_flow_buffer();
			self.body.flow = head.fork();
			if let Some(cx) = self.body.loops.last_mut() {
				cx.breaks.clear();
				cx.continues.clear();
				cx.break_types.clear();
				cx.break_values = Some(None);
			}
			let exit = round(self).unwrap_or_else(|| head.fork());
			let end = std::mem::take(&mut self.body.flow);
			let continues = self
				.body
				.loops
				.last_mut()
				.map(|cx| std::mem::take(&mut cx.continues))
				.unwrap_or_default();
			rounds += 1;
			if narrowing > 0 {
				// a descending round: the head is the entry joined with the
				// ends of the body, without widening; every such head is
				// still a post-fixpoint, so this round's walk is sound
				let mut next = self.join_states(&entry, &end);
				for state in &continues {
					next = self.join_states(&next, state);
				}
				if next != head && narrowing < NARROWING_ROUNDS {
					self.pop_flow_buffer(false);
					head = next;
					narrowing += 1;
					continue;
				}
				self.pop_flow_buffer(true);
				if let Some(cx) = self.body.loops.last_mut() {
					cx.continues = continues;
				}
				if let Some(node) = node {
					self.facts.set_loop_head(node, head);
				}
				return exit;
			}
			let mut joined = self.join_states(&head, &end);
			for state in &continues {
				joined = self.join_states(&joined, state);
			}
			let next = {
				let (places, symbols, types) = (&self.places, &self.a.symbols, &self.s.types);
				let limit = |place: PlaceId| place_limit(places, symbols, types, place);
				head.widen(&joined, &mut self.facts, limit)
			};
			if next == head || rounds > 2 * WIDENING_CAP {
				// widened: the bounds a widening sent to the type's limits may
				// come back (`while i < 10 { i += 1 }` ends with `i == 10`)
				if rounds > 1 && rounds <= 2 * WIDENING_CAP {
					let mut narrowed = self.join_states(&entry, &end);
					for state in &continues {
						narrowed = self.join_states(&narrowed, state);
					}
					if narrowed != head {
						self.pop_flow_buffer(false);
						head = narrowed;
						narrowing = 1;
						continue;
					}
				}
				self.pop_flow_buffer(true);
				if let Some(cx) = self.body.loops.last_mut() {
					cx.continues = continues;
				}
				if let Some(node) = node {
					self.facts.set_loop_head(node, head);
				}
				return exit;
			}
			self.pop_flow_buffer(false);
			head = if rounds >= WIDENING_CAP {
				head.forget_changed(&next)
			} else {
				next
			};
		}
	}

	/// Pops the loop context and joins the exit state with the `break`s.
	fn leave_loop(&mut self, exit: FlowState) -> LoopCx {
		let cx = self.body.loops.pop().expect("a loop context");
		let mut after = exit;
		for state in &cx.breaks {
			after = self.join_states(&after, state);
		}
		self.body.flow = after;
		cx
	}

	/// `E-LOOP-6` when a `for` or a `while` is used as a value. Type part.
	fn loop_statement_type(&mut self, n: NodeId, keyword: &str) -> TypeId {
		if !self.is_statement(n) {
			let message = format!(
				"a `{keyword}` loop has no value: compute the value in a variable, or use `loop` with `break value`"
			);
			let diagnostic = self.diagnostic(&E_LOOP_6, n, message);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		TypeId::VOID
	}

	/// `while condition { … }`: the condition narrows the body; its negation
	/// holds after the loop, joined with the `break`s.
	pub(super) fn check_while(&mut self, n: NodeId, condition: NodeId, body: NodeId) -> TypeId {
		self.enter_loop(n, LoopKind::While, None, false);
		let exit = self.run_loop(|w| {
			let (when_true, when_false) = w.check_cond(condition);
			w.body.flow = when_true;
			w.check_stmt(body);
			Some(when_false)
		});
		self.leave_loop(exit);
		self.memo(n, |w| {
			w.literal_condition_warning(condition, true);
			w.loop_statement_type(n, "while")
		})
	}

	/// `while let name = value { … }` (M1b, optionals).
	pub(super) fn check_while_let(
		&mut self,
		n: NodeId,
		_pattern: NodeId,
		_value: NodeId,
		_body: NodeId,
	) -> TypeId {
		self.unsupported(n, "`while let`")
	}

	/// `for pattern, index in iterable { … }`: ranges for now. The loop
	/// variable of a range is an immutable `Integer` copy with the fact
	/// `start <= i < end` (decisions.md, "Index proofs").
	#[allow(clippy::too_many_arguments)]
	pub(super) fn check_for(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<Name>,
		iterable: NodeId,
		body: NodeId,
		parallel: Option<NodeList<'_>>,
		compile_time: bool,
	) -> TypeId {
		if parallel.is_some() {
			return self.unsupported(n, "`for parallel`");
		}
		if compile_time {
			return self.unsupported(n, "`@for`");
		}
		let tree = Arc::clone(&self.tree);
		let View::Range {
			start,
			end,
			inclusive,
		} = syntax::view(&tree, iterable)
		else {
			let ty = self.check(iterable, None);
			// a `Range` value: its bounds are not known here
			if ty == TypeId::RANGE {
				return self.check_for_range_value(n, pattern, index, body);
			}
			if let Some(item) = self.iteration_item(ty) {
				return self.check_for_items(n, pattern, index, (iterable, item), body);
			}
			if self.s.types.as_int(ty).is_some() {
				self.memo(n, |w| {
					let text = w.tree.text(iterable.index()).to_string();
					let message =
						format!("cannot iterate over an integer: use a range, ex: `0..{text}`");
					let diagnostic = w.diagnostic(&E_LOOP_1, iterable, message);
					w.report(diagnostic);
					TypeId::VOID
				});
				return TypeId::VOID;
			}
			return self.unsupported(n, "`for` over this value");
		};
		// the bounds
		let mut bounds = [None, None];
		for (slot, bound) in [start, end].into_iter().enumerate() {
			if let Some(bound) = bound {
				let ty = self.check(bound, Some(TypeId::INTEGER));
				self.sink(bound, ty, TypeId::INTEGER);
				bounds[slot] = self.value_set(bound).map(|set| set.hull);
			}
		}
		self.memo(iterable, |w| {
			if start.is_none() {
				let diagnostic = w.diagnostic(
					&E_RNG_3,
					iterable,
					"a range without a start can't be iterated: give it one, ex: `0..`",
				);
				w.report(diagnostic);
			}
			TypeId::RANGE
		});
		let integer = IntStorage::Default.range();
		let lo = bounds[0].map_or(integer.lo, |bound| bound.lo);
		let hi = match bounds[1] {
			Some(bound) if inclusive => bound.hi,
			Some(bound) if bound.hi >= INFINITY => bound.hi,
			Some(bound) => bound.hi - 1,
			None => integer.hi,
		};
		let values = (lo <= hi).then(|| IntervalSet::of(Interval::new(lo, hi)));
		// `for i in 0..c.length`: `i < c.length` (proofs.rs)
		let relation = end.and_then(|end| self.length_relation_in(end, !inclusive, Some(body)));
		// the loop variable, in a scope around the body
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		let variable = pattern.and_then(|pattern| self.loop_variable(pattern));
		self.memo(n, |w| {
			if index.is_some() {
				let diagnostic = w.diagnostic(
					&E_LOOP_3,
					iterable,
					"a range has no index: its values are already the positions",
				);
				w.report(diagnostic);
			}
			w.loop_statement_type(n, "for")
		});
		self.enter_loop(n, LoopKind::For, None, false);
		let exit = self.run_loop(|w| {
			match &values {
				Some(values) => {
					if let Some(symbol) = variable {
						w.set_symbol_values(symbol, Some(values.clone()));
						if let Some(relation) = relation {
							w.relate_below(symbol, relation);
						}
					}
				}
				None => w.body.flow.set_impossible(),
			}
			w.check_stmt(body);
			if let Some(symbol) = variable {
				let place = w.symbol_place(symbol);
				w.body.flow.remove_fact(place);
			}
			None
		});
		let mut exit = exit;
		if let Some(symbol) = variable {
			let place = self.symbol_place(symbol);
			exit.remove_fact(place);
		}
		self.leave_loop(exit);
		self.body.scope = parent;
		self.type_of(n)
	}

	/// `for item, index in collection { … }` (written by E1): the variable
	/// holds the items (read-only until M2), the index an `Integer(min = 0)`.
	fn check_for_items(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<Name>,
		(iterable, item): (NodeId, TypeId),
		body: NodeId,
	) -> TypeId {
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		let tree = Arc::clone(&self.tree);
		if let Some(pattern) = pattern {
			match syntax::view(&tree, pattern) {
				View::Name(name) => {
					self.memo(pattern, |w| {
						if !name.is_empty() {
							let symbol =
								w.declare(pattern, name, SymbolKind::LoopVariable { copy: false });
							w.a.symbols.symbol_mut(symbol).ty = item;
						}
						item
					});
				}
				// `for (key, value) in pairs`, `for { name } in people` (destructuring.rs)
				_ => {
					let mut names = super::destructuring::PatternNames::default();
					let kind = SymbolKind::LoopVariable { copy: false };
					self.destructure(pattern, item, kind, None, &mut names);
				}
			}
		}
		self.memo(n, |w| {
			if let Some(index) = index.filter(|index| !index.is_empty()) {
				let symbol = w.declare(n, index, SymbolKind::LoopVariable { copy: true });
				let natural = IntStorage::Default.range();
				let ty =
					w.s.types
						.int_range(ethnum::I256::ZERO, natural.hi, IntStorage::Default);
				w.a.symbols.symbol_mut(symbol).ty = ty;
			}
			w.loop_statement_type(n, "for")
		});
		// the index is below the length of the collection (proofs.rs)
		let index_symbol = index.and(self.a.symbols.symbol_of_decl(n));
		let relation = self
			.read_local(iterable)
			.map(|collection| (self.symbol_place(collection), 0));
		self.enter_loop(n, LoopKind::For, None, false);
		let exit = self.run_loop(|w| {
			if let (Some(symbol), Some(relation)) = (index_symbol, relation) {
				w.relate_below(symbol, relation);
			}
			w.check_stmt(body);
			if let Some(symbol) = index_symbol {
				let place = w.symbol_place(symbol);
				w.body.flow.remove_fact(place);
			}
			None
		});
		self.leave_loop(exit);
		self.body.scope = parent;
		self.type_of(n)
	}

	/// `for i in range` over a `Range` value: an `Integer` variable, without
	/// facts (values brief §12). A range without start stops the program.
	fn check_for_range_value(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<Name>,
		body: NodeId,
	) -> TypeId {
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		let variable = pattern.and_then(|pattern| self.loop_variable(pattern));
		self.memo(n, |w| {
			if let Some(index) = index.filter(|index| !index.is_empty()) {
				let _ = index;
				let diagnostic = w.diagnostic(
					&E_LOOP_3,
					n,
					"a range has no index: its values are already the positions",
				);
				w.report(diagnostic);
			}
			w.loop_statement_type(n, "for")
		});
		self.enter_loop(n, LoopKind::For, None, false);
		let exit = self.run_loop(|w| {
			if let Some(symbol) = variable {
				w.set_symbol_values(symbol, None);
			}
			w.check_stmt(body);
			None
		});
		self.leave_loop(exit);
		self.body.scope = parent;
		self.type_of(n)
	}

	/// Declares the variable of a `for` over a range: an immutable `Integer` copy.
	fn loop_variable(&mut self, pattern: NodeId) -> Option<crate::ids::SymbolId> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, pattern) else {
			self.unsupported(pattern, "destructuring a range value");
			return None;
		};
		self.memo(pattern, |w| {
			if !name.is_empty() {
				let symbol = w.declare(pattern, name, SymbolKind::LoopVariable { copy: true });
				w.a.symbols.symbol_mut(symbol).ty = TypeId::INTEGER;
			}
			TypeId::INTEGER
		});
		self.a.symbols.symbol_of_decl(pattern)
	}

	/// `loop { … }`: `Never` without a `break`; otherwise, as a value, the
	/// union of the `break` values (values brief §18).
	pub(super) fn check_loop(
		&mut self,
		n: NodeId,
		body: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let value_used = !self.is_statement(n);
		self.enter_loop(n, LoopKind::Loop, expected, value_used);
		let exit = self.run_loop(|w| {
			w.check_stmt(body);
			Some(FlowState::unreachable())
		});
		let cx = self.leave_loop(exit);
		let ty = self.memo(n, |w| {
			if cx.break_types.is_empty() {
				return TypeId::NEVER;
			}
			if !value_used {
				return TypeId::VOID;
			}
			let branches: SmallVec<[(NodeId, TypeId); 4]> = cx
				.break_types
				.iter()
				.filter(|(_, ty)| *ty != TypeId::VOID)
				.copied()
				.collect();
			if branches.is_empty() {
				return TypeId::UNKNOWN;
			}
			w.branches_type(n, &branches, expected)
		});
		// the values of the `loop`: the hull of its `break` values'
		if value_used {
			match (cx.break_values.flatten(), self.s.types.interval_of(ty)) {
				(Some(hull), Some(range)) => match hull.intersect(range) {
					Some(inside) => self.record_interval(n, inside, range),
					None => {
						self.a.intervals.remove(&n);
					}
				},
				_ => {
					self.a.intervals.remove(&n);
				}
			}
		}
		ty
	}

	/// `do { … }`: a scope, or a value (its last expression, `E-DO-1` without one).
	pub(super) fn check_do(&mut self, n: NodeId, body: NodeId, expected: Option<TypeId>) -> TypeId {
		if self.is_statement(n) {
			self.check_stmt(body);
			return self.memo(n, |_| TypeId::VOID);
		}
		let ty = self.check_block_value(body, expected).0;
		let value = self.block_value_node(body);
		if let Some(expected) = expected
			&& ty != TypeId::VOID
		{
			self.sink(value, ty, expected);
		}
		self.memo(n, |w| {
			if ty == TypeId::VOID {
				let diagnostic = w.diagnostic(
					&E_DO_1,
					n,
					"this block has no value: it ends with a statement, so end it with the value it gives",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			ty
		})
	}

	/// The innermost loop a `break` or `continue` acts on: `E-LOOP-12`
	/// outside a loop, `E-LOOP-13` across a function. Type part.
	fn jump_target(&mut self, n: NodeId, keyword: &str) -> bool {
		let depth = self.body.functions.len();
		match self.body.loops.last() {
			None if self.body.inside_loop => {
				let message = format!(
					"`{keyword}` can't leave a function to act on the loop around it: return a value and test it in the loop"
				);
				let diagnostic = self.diagnostic(&E_LOOP_13, n, message);
				self.report(diagnostic);
				false
			}
			None => {
				let message = format!(
					"`{keyword}` is outside of a loop: it can only be used in `for`, `while` or `loop`"
				);
				let diagnostic = self.diagnostic(&E_LOOP_12, n, message);
				self.report(diagnostic);
				false
			}
			Some(cx) if cx.function_depth != depth => {
				let message = format!(
					"`{keyword}` can't leave a function to act on the loop around it: return a value and test it in the loop"
				);
				let diagnostic = self.diagnostic(&E_LOOP_13, n, message);
				self.report(diagnostic);
				false
			}
			Some(_) => true,
		}
	}

	/// `break`, `break value`
	pub(super) fn check_break(&mut self, n: NodeId, value: Option<NodeId>) -> TypeId {
		let target = self
			.body
			.loops
			.last()
			.filter(|cx| cx.function_depth == self.body.functions.len())
			.map(|cx| (cx.kind, cx.expected, cx.value_used));
		let expected = target.and_then(|(_, expected, used)| expected.filter(|_| used));
		let value_ty = value.map(|value| {
			let ty = self.check(value, expected);
			if let Some(expected) = expected {
				self.sink(value, ty, expected);
			}
			ty
		});
		self.memo(n, |w| {
			if !w.jump_target(n, "break") {
				return TypeId::NEVER;
			}
			let Some((kind, _, used)) = target else {
				return TypeId::NEVER;
			};
			match (kind, value, used) {
				(LoopKind::For | LoopKind::While, Some(value), _) => {
					let diagnostic = w.diagnostic(
						&E_LOOP_7,
						value,
						"`for` and `while` loops have no value: use `loop` to break with a value",
					);
					w.report(diagnostic);
				}
				(LoopKind::Loop, Some(value), false) => {
					let diagnostic = w.diagnostic(
						&W_LOOP_1,
						value,
						"the value of this `break` is dropped: the `loop` is not used as a value",
					);
					w.report(diagnostic);
				}
				(LoopKind::Loop, None, true) => {
					let diagnostic = w.diagnostic(
						&E_LOOP_11,
						n,
						"this `loop` is used as a value: give the `break` one, ex: `break result`",
					);
					w.report(diagnostic);
				}
				_ => {}
			}
			TypeId::NEVER
		});
		if target.is_some() {
			let set = match value {
				Some(value) if self.body.flow.reachable => {
					self.value_set(value).map(|set| set.hull)
				}
				_ => None,
			};
			let state = self.body.flow.fork();
			let cx = self.body.loops.last_mut().expect("a loop");
			if value.is_some() && state.reachable {
				cx.break_values = cx.break_values.and_then(|hull| {
					let set = set?;
					Some(Some(hull.map_or(set, |hull: Interval| hull.hull(set))))
				});
			}
			cx.break_types.push((n, value_ty.unwrap_or(TypeId::VOID)));
			if state.reachable {
				cx.breaks.push(state);
			}
		}
		self.body.flow.set_unreachable();
		TypeId::NEVER
	}

	/// `continue`
	pub(super) fn check_continue(&mut self, n: NodeId) -> TypeId {
		let valid = self
			.body
			.loops
			.last()
			.is_some_and(|cx| cx.function_depth == self.body.functions.len());
		self.memo(n, |w| {
			w.jump_target(n, "continue");
			TypeId::NEVER
		});
		if valid && self.body.flow.reachable {
			let state = self.body.flow.fork();
			self.body
				.loops
				.last_mut()
				.expect("a loop")
				.continues
				.push(state);
		}
		self.body.flow.set_unreachable();
		TypeId::NEVER
	}

	/// `return`, `return value`: implemented with the functions (E1).
	pub(super) fn check_return(&mut self, n: NodeId, value: Option<NodeId>) -> TypeId {
		self.check_return_value(n, value)
	}
}

/// The range of an integer place's type, for widening.
fn place_limit(
	places: &crate::flow::Places,
	symbols: &crate::symbols::SymbolTable,
	types: &crate::types::TypeStore,
	place: PlaceId,
) -> Option<Interval> {
	let entry = places.get(place);
	match (entry.root, entry.parent) {
		(PlaceRoot::Symbol(symbol), None) => types.interval_of(symbols.symbol(symbol).ty),
		_ => None,
	}
}
