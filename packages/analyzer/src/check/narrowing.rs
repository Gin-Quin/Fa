//! Narrowing (design §3.7, values brief §14): what a test, an assignment or
//! an early exit tells about the union member a local holds.
//!
//! A narrowing is the `narrowed` part of the place's fact, never a new
//! declaration: the place keeps its declared type (`mutable x: Integer? = 5`
//! can still be assigned `none`). A read of a narrowed local has the narrowed
//! type (memoized at its first visit, with `NARROWED`), and every later visit
//! re-validates it against the current fact (decision A2): a narrowing lost
//! at the fixpoint of a loop is `E-OPT-1` / `E-NAR-1` there.
//!
//! Type-level verdicts (`E-CMP-7` / `W-CMP-1`, `E-OPT-6`) look at the
//! declared type, not at the facts (values brief D-28).

use std::sync::Arc;

use smallvec::SmallVec;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::values::{E_CMP_7, E_NAR_1, E_OPT_1, E_WHEN_1, W_CMP_1};
use crate::flow::{Fact, FlowState, IntervalSet};
use crate::ids::{NodeId, PlaceId, SymbolId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, NodeList, View};
use crate::types::{Interval, TypeKind};

impl Walker<'_> {
	// ── the narrowed type of a local ──────────────────────────────────────

	/// The fact of a local here: its flow fact, else its binding's.
	pub(crate) fn symbol_fact(&mut self, symbol: SymbolId) -> Option<Fact> {
		let place = self.symbol_place(symbol);
		let fact = self
			.body
			.flow
			.fact(place)
			.or_else(|| self.visible_binding(symbol))?;
		Some(self.facts.fact_value(fact))
	}

	/// The type a local has here: its narrowed type, else its declared type.
	pub(crate) fn current_type(&mut self, symbol: SymbolId) -> TypeId {
		let declared = self.a.symbols.symbol(symbol).ty;
		match self.symbol_fact(symbol) {
			Some(fact) if !fact.narrowed.is_unset() => fact.narrowed,
			_ => declared,
		}
	}

	/// Narrows a local to `ty` in the current state (its other facts kept);
	/// `ty` equal to the declared type removes the narrowing.
	pub(crate) fn narrow_symbol(&mut self, symbol: SymbolId, ty: TypeId) {
		let declared = self.a.symbols.symbol(symbol).ty;
		let place = self.symbol_place(symbol);
		let mut fact = self
			.body
			.flow
			.fact(place)
			.map_or(Fact::TOP, |fact| self.facts.fact_value(fact));
		fact.narrowed = if ty == declared { TypeId::UNSET } else { ty };
		let id = self.facts.fact(fact);
		self.body.flow.set_fact(place, id);
	}

	/// The type of a read of a local (type part, from `check_name`): its
	/// narrowed type when the facts narrow it here, with `NARROWED`.
	pub(crate) fn narrowed_read(
		&mut self,
		n: NodeId,
		symbol: SymbolId,
		declared: TypeId,
	) -> TypeId {
		if declared.is_unset() || self.s.types.is_recovering(declared) {
			return declared;
		}
		let current = self.current_type(symbol);
		if current != declared && !current.is_unset() {
			self.set_flag(n, NodeFlags::NARROWED);
			return current;
		}
		declared
	}

	/// The flow part of a read of a local: a narrowed read stays valid
	/// (decision A2), `E-OPT-1` / `E-NAR-1` otherwise.
	pub(crate) fn revalidate_read(&mut self, n: NodeId) {
		if !self.a.flags_of(n).contains(NodeFlags::NARROWED) || !self.body.flow.reachable {
			return;
		}
		let Resolution::Local(symbol) = self.a.resolution(n) else {
			return;
		};
		let read = self.type_of(n);
		let current = self.current_type(symbol);
		if self.s.types.satisfies(current, read) {
			return;
		}
		let text = self.tree.text(n.index()).to_string();
		let may_be_none = self.has_none(current) && !self.has_none(read);
		let (code, message) = if may_be_none {
			(
				&E_OPT_1,
				format!(
					"`{text}` may be `none` here: test it with `if {text} is not none` first, or use `?.` or `??`"
				),
			)
		} else {
			(
				&E_NAR_1,
				format!(
					"`{text}` may be a `{}` here, not only a `{}`: test it with `is` first",
					self.display(current),
					self.display(read)
				),
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
	}

	/// The declared type of an expression: a local's declared type, else its type.
	pub(crate) fn declared_type_of(&self, n: NodeId) -> TypeId {
		let n = self.strip_groups(n);
		match (syntax::view(&self.tree, n), self.a.resolution(n)) {
			(View::Name(_), Resolution::Local(symbol)) => self.a.symbols.symbol(symbol).ty,
			// a narrowed field: the field's type
			(View::Access { members, .. }, _)
				if members
					.last()
					.is_some_and(|last| self.a.flags_of(last).contains(NodeFlags::NARROWED)) =>
			{
				match members.last().map(|last| self.a.resolution(last)) {
					Some(Resolution::Field(shape, index)) => {
						self.s.types.shape_fields(shape)[index as usize].ty
					}
					_ => self.type_of(n),
				}
			}
			_ => self.type_of(n),
		}
	}

	/// The local a node reads, when it is a plain name (`(x)` included).
	pub(crate) fn read_local(&self, n: NodeId) -> Option<SymbolId> {
		let n = self.strip_groups(n);
		match (syntax::view(&self.tree, n), self.a.resolution(n)) {
			(View::Name(_), Resolution::Local(symbol)) => Some(symbol),
			_ => None,
		}
	}

	// ── union members ─────────────────────────────────────────────────────

	/// The members of a type: those of an anonymous union, and each member of
	/// a named union or an enumeration (itself for any other type).
	pub(crate) fn members_of(&mut self, ty: TypeId) -> SmallVec<[TypeId; 4]> {
		let parts: SmallVec<[TypeId; 4]> =
			self.s.types.union_members(&ty).iter().copied().collect();
		let mut members = SmallVec::new();
		for part in parts {
			match self.s.types.kind(part) {
				TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
					let enumeration = matches!(self.s.types.kind(part), TypeKind::Enum(..));
					let count = self
						.s
						.types
						.sym_list(self.s.types.nominal(nominal).members)
						.len() as u16;
					for index in 0..count {
						if !self.s.types.mask_has(mask, index) {
							continue;
						}
						members.push(if enumeration {
							let single = self.s.types.single_mask(index);
							self.s.types.enumeration(nominal, single)
						} else {
							self.s.types.member(nominal, index)
						});
					}
				}
				_ => members.push(part),
			}
		}
		members
	}

	/// Whether a type has several members (narrowing applies to it).
	pub(crate) fn is_union_like(&self, ty: TypeId) -> bool {
		matches!(
			self.s.types.kind(ty),
			TypeKind::Union(_) | TypeKind::NamedUnion(..) | TypeKind::Enum(..)
		)
	}

	/// The union of some members (`NEVER` for none).
	pub(crate) fn union_of(&mut self, members: &[TypeId]) -> TypeId {
		match members {
			[] => TypeId::NEVER,
			[one] => *one,
			_ => self.s.types.union(members),
		}
	}

	/// Whether a type is optional (has `None` among its members).
	pub(crate) fn has_none(&self, ty: TypeId) -> bool {
		self.s.types.union_members(&ty).contains(&TypeId::NONE)
	}

	/// A type without `None`.
	pub(crate) fn without_none(&mut self, ty: TypeId) -> TypeId {
		let members: SmallVec<[TypeId; 4]> = self
			.members_of(ty)
			.into_iter()
			.filter(|&member| member != TypeId::NONE)
			.collect();
		self.union_of(&members)
	}

	/// The narrowing an assigned value gives a local of a union type: the
	/// value's type when it is narrower than the declared one (values brief
	/// §14: "the assigned value's facts replace them").
	pub(crate) fn assignment_narrowing(&mut self, declared: TypeId, value: TypeId) -> TypeId {
		if !self.is_union_like(declared)
			|| value.is_unset()
			|| self.s.types.is_recovering(value)
			|| value == TypeId::NEVER
		{
			return TypeId::UNSET;
		}
		let value = self.s.types.widen(value);
		// the declared members that the value may be
		let declared_members = self.members_of(declared);
		let value_members = self.members_of(value);
		let members: SmallVec<[TypeId; 4]> = declared_members
			.iter()
			.copied()
			.filter(|&member| {
				value_members
					.iter()
					.any(|&part| self.s.types.satisfies(part, member))
			})
			.collect();
		if members.is_empty() || members.len() == declared_members.len() {
			return TypeId::UNSET;
		}
		self.union_of(&members)
	}

	// ── tests that narrow ─────────────────────────────────────────────────

	/// `value is none`, `value == none` and their negations: the true and
	/// false states (`is_none` gives the polarity), with the type-level
	/// verdicts (`E-CMP-7` / `W-CMP-1`) with `verdict` (`Some(positive)`:
	/// whether the written test is positive, `is` rather than `is not`).
	pub(crate) fn none_test(
		&mut self,
		n: NodeId,
		value: NodeId,
		is_none: bool,
		verdict: Option<bool>,
	) -> (FlowState, FlowState) {
		let declared = self.declared_type_of(value);
		if let Some(positive) = verdict {
			self.memo(n, |w| {
				w.type_level_verdict(n, declared, TypeId::NONE, positive);
				TypeId::BOOLEAN
			});
		}
		let mut when_none = self.body.flow.fork();
		let mut when_some = self.body.flow.fork();
		if let Some(symbol) = self.read_local(value) {
			let current = self.current_type(symbol);
			if self.has_none(current) {
				let some = self.without_none(current);
				self.narrow_in(&mut when_none, symbol, TypeId::NONE);
				self.narrow_in(&mut when_some, symbol, some);
			} else if !self.s.types.is_recovering(current) {
				// the facts rule `none` out
				when_none = FlowState::impossible();
			}
		} else if let Some((place, declared)) = self.read_field_path(value) {
			// a field path (`user.address is not none`)
			let current = self.narrowed_place_type(place, declared);
			if self.has_none(current) {
				let some = self.without_none(current);
				self.narrow_place_in(&mut when_none, place, declared, TypeId::NONE);
				self.narrow_place_in(&mut when_some, place, declared, some);
			} else if self.has_none(declared) && !self.s.types.is_recovering(current) {
				when_none = FlowState::impossible();
			}
		}
		if is_none {
			(when_none, when_some)
		} else {
			(when_some, when_none)
		}
	}

	/// `value is Type`: the members matching the type, and the others.
	pub(crate) fn type_test(
		&mut self,
		n: NodeId,
		value: NodeId,
		ty: TypeId,
		verdict: Option<bool>,
	) -> (FlowState, FlowState) {
		let declared = self.declared_type_of(value);
		if let Some(positive) = verdict {
			self.memo(n, |w| {
				w.type_level_verdict(n, declared, ty, positive);
				TypeId::BOOLEAN
			});
		}
		let mut matching = self.body.flow.fork();
		let mut others = self.body.flow.fork();
		if self.s.types.is_recovering(ty) {
			return (matching, others);
		}
		// a local, or a path of fields (`shape.inner is Circle`)
		let (current, field) = if let Some(symbol) = self.read_local(value) {
			(self.current_type(symbol), Err(symbol))
		} else if let Some((place, declared)) = self.read_field_path(value) {
			if !self.is_union_like(declared) {
				return (matching, others);
			}
			(
				self.narrowed_place_type(place, declared),
				Ok((place, declared)),
			)
		} else {
			return (matching, others);
		};
		let members = self.members_of(current);
		let (yes, no): (SmallVec<[TypeId; 4]>, SmallVec<[TypeId; 4]>) = members
			.into_iter()
			.partition(|&member| self.s.types.satisfies(member, ty));
		match (yes.is_empty(), no.is_empty()) {
			(true, _) => matching = FlowState::impossible(),
			(_, true) => others = FlowState::impossible(),
			_ => {
				let (yes, no) = (self.union_of(&yes), self.union_of(&no));
				match field {
					Err(symbol) => {
						self.narrow_in(&mut matching, symbol, yes);
						self.narrow_in(&mut others, symbol, no);
					}
					Ok((place, declared)) => {
						self.narrow_place_in(&mut matching, place, declared, yes);
						self.narrow_place_in(&mut others, place, declared, no);
					}
				}
			}
		}
		(matching, others)
	}

	/// Narrows a local in another state than the current one.
	pub(crate) fn narrow_in(&mut self, state: &mut FlowState, symbol: SymbolId, ty: TypeId) {
		if !state.reachable {
			return;
		}
		let saved = std::mem::replace(&mut self.body.flow, std::mem::take(state));
		self.narrow_symbol(symbol, ty);
		*state = std::mem::replace(&mut self.body.flow, saved);
	}

	// ── field paths ───────────────────────────────────────────────────────

	/// A path of fields read from a place (`user.address`, `self.owner`):
	/// its place and its declared type, or `None` when it is something else
	/// (a call result, an item, a built-in property).
	pub(crate) fn read_field_path(&mut self, n: NodeId) -> Option<(PlaceId, TypeId)> {
		let n = self.strip_groups(n);
		let tree = Arc::clone(&self.tree);
		let View::Access { target, members } = syntax::view(&tree, n) else {
			return None;
		};
		// fields, none of them a relation side (relations.rs)
		let mut owner = self.type_of(target);
		for member in members.iter() {
			if !matches!(self.a.resolution(member), Resolution::Field(..))
				|| self.is_relation_side(owner, tree.text(member.index()))
			{
				return None;
			}
			owner = self.type_of(member);
		}
		let Resolution::Field(shape, index) = self.a.resolution(members.last()?) else {
			return None;
		};
		let declared = self.s.types.shape_fields(shape).get(index as usize)?.ty;
		let place = self.place_of(n)?;
		Some((place, declared))
	}

	/// The type a field path has here: its narrowing, else `declared`.
	pub(crate) fn narrowed_place_type(&mut self, place: PlaceId, declared: TypeId) -> TypeId {
		match self.body.flow.fact(place) {
			Some(fact) => {
				let narrowed = self.facts.fact_value(fact).narrowed;
				if narrowed.is_unset() {
					declared
				} else {
					narrowed
				}
			}
			None => declared,
		}
	}

	/// Narrows a field path to `ty` in `state` (its other facts kept).
	pub(crate) fn narrow_place_in(
		&mut self,
		state: &mut FlowState,
		place: PlaceId,
		declared: TypeId,
		ty: TypeId,
	) {
		if !state.reachable {
			return;
		}
		let mut fact = state
			.fact(place)
			.map_or(Fact::TOP, |fact| self.facts.fact_value(fact));
		fact.narrowed = if ty == declared { TypeId::UNSET } else { ty };
		let id = self.facts.fact(fact);
		state.set_fact(place, id);
	}

	/// The narrowed type of a field read at `member` (type part, from the
	/// members of an access): the narrowing of `place`, with `NARROWED` on
	/// the member, when it is narrower than the field's type `ty`.
	pub(crate) fn narrowed_field(&mut self, member: NodeId, place: PlaceId, ty: TypeId) -> TypeId {
		if !self.is_union_like(ty) || self.s.types.is_recovering(ty) {
			return ty;
		}
		let current = self.narrowed_place_type(place, ty);
		if current == ty || !self.s.types.satisfies(current, ty) {
			return ty;
		}
		self.set_flag(member, NodeFlags::NARROWED);
		self.set_type(member, current);
		current
	}

	/// The flow part of the narrowed members of an access (decision A2): each
	/// stays valid, `E-OPT-1` / `E-NAR-1` otherwise.
	pub(crate) fn revalidate_fields(&mut self, target: NodeId, members: NodeList<'_>) {
		if !self.body.flow.reachable
			|| !members
				.iter()
				.any(|member| self.a.flags_of(member).contains(NodeFlags::NARROWED))
		{
			return;
		}
		let Some(mut place) = self.place_of(target) else {
			return;
		};
		let mut text = self.tree.text(target.index()).to_string();
		for member in members.iter() {
			let Some(next) = self.member_step(place, member) else {
				return;
			};
			place = next;
			text.push('.');
			text.push_str(self.tree.text(member.index()));
			if !self.a.flags_of(member).contains(NodeFlags::NARROWED) {
				continue;
			}
			let Resolution::Field(shape, index) = self.a.resolution(member) else {
				return;
			};
			let declared = self.s.types.shape_fields(shape)[index as usize].ty;
			let read = self.type_of(member);
			let current = self.narrowed_place_type(place, declared);
			if self.s.types.satisfies(current, read) {
				continue;
			}
			let (code, message) = if self.has_none(current) && !self.has_none(read) {
				(
					&E_OPT_1,
					format!(
						"`{text}` may be `none` here: test it with `if {text} is not none` first, or use `?.` or `??`"
					),
				)
			} else {
				(
					&E_NAR_1,
					format!(
						"`{text}` may be a `{}` here, not only a `{}`: test it with `is` first",
						self.display(current),
						self.display(read)
					),
				)
			};
			let diagnostic = self.diagnostic(code, member, message);
			self.report_flow(diagnostic);
			return;
		}
	}

	/// `E-CMP-7` when a test of `declared` against `tested` can never hold,
	/// `W-CMP-1` when it always does (type level, values brief D-28). Type part.
	pub(crate) fn type_level_verdict(
		&mut self,
		n: NodeId,
		declared: TypeId,
		tested: TypeId,
		positive: bool,
	) {
		if declared.is_unset()
			|| self.s.types.is_recovering(declared)
			|| self.s.types.is_recovering(tested)
		{
			return;
		}
		let members = self.members_of(declared);
		let matching = members
			.iter()
			.filter(|&&member| self.s.types.satisfies(member, tested))
			.count();
		let (never, always) = (matching == 0, matching == members.len());
		let text = self.tree.text(n.index()).to_string();
		if never == positive && (never || always) {
			let message = format!(
				"`{text}` is never true: a `{}` is never a `{}`",
				self.display(declared),
				self.display(tested)
			);
			let diagnostic = self.diagnostic(&E_CMP_7, n, message);
			self.report(diagnostic);
		} else if always || never {
			let message = format!(
				"`{text}` is always true: a `{}` is always {}a `{}`",
				self.display(declared),
				if positive { "" } else { "not " },
				self.display(tested)
			);
			let diagnostic = self.diagnostic(&W_CMP_1, n, message);
			self.report(diagnostic);
		}
	}
}

impl Walker<'_> {
	/// `E-OPT-1` at a sink: an optional where its base type is expected
	/// (from `sink`, before `E-TYPE-1`). Whether it reported.
	pub(crate) fn optional_sink(&mut self, n: NodeId, actual: TypeId, target: TypeId) -> bool {
		if !self.has_none(actual) || self.has_none(target) {
			return false;
		}
		let some = self.without_none(actual);
		if some == TypeId::NEVER || !self.s.types.satisfies(some, target) {
			return false;
		}
		let text = self.tree.text(self.strip_groups(n).index()).to_string();
		let message = format!(
			"`{text}` may be `none`, and a `{}` is expected: test it with `if {text} is not none` first, or use `??`",
			self.display(target)
		);
		let diagnostic = self.diagnostic(&E_OPT_1, n, message);
		self.report_flow(diagnostic);
		true
	}

	/// `E-OPT-1` / `E-NAR-1` for an operand of an operator that is an
	/// optional or a union (type part). Whether it reported.
	pub(crate) fn union_operand(&mut self, node: NodeId, ty: TypeId) -> bool {
		if !matches!(self.s.types.kind(ty), TypeKind::Union(_)) {
			return false;
		}
		let text = self.tree.text(node.index()).to_string();
		let (code, message) = if self.has_none(ty) {
			(
				&E_OPT_1,
				format!(
					"`{text}` may be `none`: test it with `if {text} is not none` first, or use `??`"
				),
			)
		} else {
			(
				&E_NAR_1,
				format!(
					"`{text}` may be any of `{}`: test it with `is` first",
					self.display(ty)
				),
			)
		};
		let diagnostic = self.diagnostic(code, node, message);
		self.report(diagnostic);
		true
	}
}

impl Walker<'_> {
	/// The flow of one arm of a `when` (from `patterns.rs`): the current state
	/// (what the previous arms left) becomes the arm's state, the subject
	/// narrowed to the pattern; `remaining` becomes the state where it
	/// doesn't match (values brief §14: "when with a plain local subject").
	pub(crate) fn when_arm(
		&mut self,
		n: NodeId,
		subject: NodeId,
		pattern: NodeId,
		remaining: &mut FlowState,
	) {
		if self.read_local(subject).is_none() || !self.body.flow.reachable {
			return;
		}
		// a pattern with an error (already reported): no narrowing
		let ty = self.type_of(pattern);
		if ty.is_unset() || self.s.types.is_recovering(ty) {
			return;
		}
		let (matching, others) = self.pattern_test(n, subject, pattern, None);
		self.body.flow = matching;
		*remaining = others;
	}

	/// Exhaustiveness over integers (values brief D-55, from `patterns.rs`
	/// at every visit): the literal and range patterns (with constant bounds)
	/// must cover the subject's values, its facts included. `E-WHEN-1` lists
	/// what is left.
	pub(crate) fn integer_coverage(&mut self, n: NodeId, subject: NodeId, patterns: &[NodeId]) {
		if !self.body.flow.reachable {
			return;
		}
		let subject_ty = self.type_of(self.strip_groups(subject));
		let int_member = self
			.members_of(subject_ty)
			.into_iter()
			.find(|&member| self.s.types.as_int(member).is_some());
		let Some(int_member) = int_member else {
			return;
		};
		let mut values = if self.s.types.as_int(subject_ty).is_some() {
			match self.value_set(subject) {
				Some(values) => values,
				None => return,
			}
		} else {
			let Some(range) = self.s.types.interval_of(int_member) else {
				return;
			};
			IntervalSet::of(range)
		};
		let mut intervals: SmallVec<[Interval; 8]> = SmallVec::new();
		for &pattern in patterns {
			if !self.pattern_intervals(pattern, int_member, &mut intervals) {
				return; // a pattern covering every integer, or not understood
			}
		}
		for interval in intervals {
			match values.exclude(interval) {
				Some(left) => values = left,
				None => return,
			}
		}
		let missing = if values.hull.lo <= -crate::types::int::INFINITY
			|| values.hull.hi >= crate::types::int::INFINITY
		{
			"the other integers".to_string()
		} else {
			let pieces: Vec<String> = values
				.pieces()
				.into_iter()
				.map(|piece| match piece.as_exact() {
					Some(value) => format!("`{value}`"),
					None => format!("`{}..={}`", piece.lo, piece.hi),
				})
				.collect();
			match pieces.as_slice() {
				[] => return,
				[single] => single.clone(),
				[rest @ .., last] => format!("{} and {last}", rest.join(", ")),
			}
		};
		let message = format!("this `when` doesn't handle {missing}: add an arm, or an `else` arm");
		let diagnostic = self.diagnostic(&E_WHEN_1, n, message);
		self.report_flow(diagnostic);
	}

	/// The integers a pattern covers (constant literals and ranges);
	/// `false` when it covers every integer (a type pattern matching the
	/// integer member) or isn't constant: then no diagnostic.
	fn pattern_intervals(
		&mut self,
		pattern: NodeId,
		int_member: TypeId,
		out: &mut SmallVec<[Interval; 8]>,
	) -> bool {
		let tree = std::sync::Arc::clone(&self.tree);
		let inner = self.strip_groups(pattern);
		match syntax::view(&tree, inner) {
			View::Union { operands } => operands
				.iter()
				.all(|operand| self.pattern_intervals(operand, int_member, out)),
			View::Integer(value) => {
				out.push(Interval::exact(value));
				true
			}
			View::Negate(_) => match self.value_set(inner).and_then(|set| set.as_exact()) {
				Some(value) => {
					out.push(Interval::exact(value));
					true
				}
				None => false,
			},
			View::Range {
				start,
				end,
				inclusive,
			} => {
				let bound = |w: &mut Self, node: Option<NodeId>| match node {
					Some(node) => w.value_set(node).and_then(|set| set.as_exact()).map(Some),
					None => Some(None),
				};
				let (Some(lo), Some(hi)) = (bound(self, start), bound(self, end)) else {
					return false;
				};
				let lo = lo.unwrap_or(-crate::types::int::INFINITY);
				let hi = match hi {
					Some(end) if inclusive => end,
					Some(end) => end - 1,
					None => crate::types::int::INFINITY,
				};
				if lo <= hi {
					out.push(Interval::new(lo, hi));
				}
				true
			}
			// other literals cover no integer
			View::String(_) | View::Boolean(_) | View::None | View::Float(_) => true,
			_ => {
				// a type pattern: covers the integers when it matches them
				let ty = self.type_of(pattern);
				!ty.is_unset()
					&& !self.s.types.is_recovering(ty)
					&& !self.s.types.satisfies(int_member, ty)
			}
		}
	}
}
