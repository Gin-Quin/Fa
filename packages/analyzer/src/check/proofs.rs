//! Length facts and index proofs (values brief §6.5 rule 3, decisions.md
//! "Index proofs").
//!
//! - The length of a collection literal, or the UTF-16 code unit count of a string
//!   literal, is a fact of the place it is bound to (`Fact::length`), kept
//!   while the place can't change: immutable collections only (`let`,
//!   parameters), strings until they are assigned.
//! - A relation `i + k < c.length` (`Fact::below`) comes from `for i in
//!   a..c.length - k` and from a test `i < c.length - k`, for an immutable
//!   collection `c`; it ends when `i` is assigned.
//! - `c[i]` needs no check when `0 <= i < length` (or `-length <= i < 0`)
//!   follows from the facts; a certain miss is `E-NUM-17`; otherwise
//!   `INDEX_CHECK` and the `IndexError` path (`can_fail`).

use std::sync::Arc;

use ethnum::I256;

use crate::analysis::NodeFlags;
use crate::builtins::strings::utf16_length;
use crate::check::Walker;
use crate::diagnostics::codes::values::E_NUM_17;
use crate::ids::{IntervalId, NodeId, PlaceId, SymbolId, TypeId};
use crate::symbols::SymbolKind;
use crate::syntax::{self, ArithOp, View};
use crate::types::{CollKind, Interval, TypeKind};

impl Walker<'_> {
	/// The length a value has when it is bound: the number of items of an
	/// array literal without spreads, the UTF-16 code unit count of a string literal.
	pub(crate) fn literal_length(&self, value: NodeId) -> Option<Interval> {
		let value = self.strip_groups(value);
		match syntax::view(&self.tree, value) {
			View::Array { items } => {
				let spreads = items
					.iter()
					.any(|item| matches!(syntax::view(&self.tree, item), View::Spread(_)));
				(!spreads).then(|| Interval::exact(I256::from(items.len() as u64)))
			}
			_ => match self.s.types.kind(self.type_of(value)) {
				TypeKind::StringLit(text) => {
					let count = utf16_length(self.s.types.syms.resolve(text));
					Some(Interval::exact(I256::from(count as u64)))
				}
				_ => None,
			},
		}
	}

	/// Drops the relations `i < c.length` to a collection whose length may
	/// have changed (a method changing it, an assignment).
	pub(crate) fn drop_relations_to(&mut self, collection: PlaceId) {
		let related: Vec<(PlaceId, crate::flow::Fact)> = self
			.body
			.flow
			.facts
			.iter()
			.map(|(place, fact)| (place, self.facts.fact_value(fact)))
			.filter(|(_, fact)| fact.below.is_some_and(|(place, _)| place == collection))
			.collect();
		for (place, mut fact) in related {
			fact.below = None;
			let id = self.facts.fact(fact);
			self.body.flow.set_fact(place, id);
		}
	}

	/// Whether the length of a local can't change while it is in scope: an
	/// immutable collection or string (`let`, a parameter).
	fn fixed_length(&self, symbol: SymbolId) -> bool {
		matches!(
			self.a.symbols.symbol(symbol).kind,
			SymbolKind::Let | SymbolKind::Parameter { .. }
		)
	}

	/// The length fact of the value of a binding (`None`: nothing known).
	pub(crate) fn binding_length(&mut self, symbol: SymbolId, value: NodeId) -> IntervalId {
		let ty = self.a.symbols.symbol(symbol).ty;
		// a mutable collection keeps its length fact until a method changes
		// it (`mutated_receiver`) or it is assigned
		let _ = ty;
		// a copy of a local has its length (`mutable results = numbers`)
		let copied = match self.view(self.strip_groups(value)) {
			View::Copy(inner) => self.expression_length(inner),
			_ => self
				.read_local(value)
				.filter(|&source| source != symbol)
				.and_then(|source| self.length_of(source)),
		};
		let length = copied.or_else(|| {
			if self.type_of(self.strip_groups(value)) == TypeId::BYTES
				|| self.a.flags[self.strip_groups(value).index()]
					.contains(NodeFlags::STRING_CODE_UNIT)
			{
				self.expression_length(value)
			} else {
				self.literal_length(value)
			}
		});
		match length {
			Some(length) => self.facts.interval(length),
			None => IntervalId::NONE,
		}
	}

	/// The length of an expression here: a local's length fact, a literal's
	/// length, the length of the collection a `map` is called on (values
	/// brief §6.5 rule 3: "equal lengths for … `map`").
	pub(crate) fn expression_length(&mut self, n: NodeId) -> Option<Interval> {
		let n = self.strip_groups(n);
		if let View::Copy(inner) = self.view(n) {
			return self.expression_length(inner);
		}
		if let Some(symbol) = self.read_local(n) {
			return self.length_of(symbol);
		}
		if let Some(call) = self.a.calls.get(&n).cloned()
			&& let crate::analysis::Callee::Builtin(builtin) = call.callee
		{
			let given = call.arguments.iter().find_map(|arg| match arg {
				crate::analysis::Argument::Given(value) => Some(*value),
				_ => None,
			});
			match (crate::builtins::get(builtin).path, given) {
				("Bytes.from", Some(items)) => return self.expression_length(items),
				("Bytes.filled", Some(length)) => {
					return self.value_set(length).map(|set| set.hull);
				}
				_ => {}
			}
			if crate::builtins::get(builtin).path == "String#toUtf8"
				&& let View::Call { callee, .. } = self.view(n)
				&& let Some(receiver) = self.callee_receiver_node(callee)
				&& let TypeKind::StringLit(text) =
					self.s.types.kind(self.type_of(self.strip_groups(receiver)))
			{
				return Some(Interval::exact(I256::from(
					self.s.types.syms.resolve(text).len() as u64,
				)));
			}
		}
		if self.a.flags[n.index()].contains(NodeFlags::STRING_CODE_UNIT) {
			return Some(Interval::exact(I256::ONE));
		}
		if let View::Call { callee, .. } = syntax::view(&self.tree, n) {
			let callee = self.strip_groups(callee);
			if let View::Access { target, members } = syntax::view(&self.tree, callee)
				&& members.len() == 1
				&& let Some(member) = members.first()
				&& let View::Name(name) = syntax::view(&self.tree, member)
				&& self.tree.name(name) == "map"
			{
				return self.expression_length(target);
			}
			return None;
		}
		self.literal_length(n).or_else(|| {
			self.s
				.types
				.inline_array(self.type_of(n))
				.map(|(_, capacity)| Interval::new(I256::ZERO, I256::from(capacity)))
		})
	}

	/// The length of a local collection or string here, when known.
	pub(crate) fn length_of(&mut self, symbol: SymbolId) -> Option<Interval> {
		let fact = self.symbol_fact(symbol).unwrap_or(crate::flow::Fact::TOP);
		if fact.length != IntervalId::NONE {
			return Some(self.facts.interval_value(fact.length));
		}
		self.s
			.types
			.inline_array(self.a.symbols.symbol(symbol).ty)
			.map(|(_, capacity)| Interval::new(I256::ZERO, I256::from(capacity)))
	}

	/// Narrows the length fact of a local collection here (`if c.length >= 2`).
	pub(crate) fn refine_length(&mut self, symbol: SymbolId, length: Interval) {
		let mut fact = self.symbol_fact(symbol).unwrap_or(crate::flow::Fact::TOP);
		let length = match fact.length {
			IntervalId::NONE => Some(length),
			current => self.facts.interval_value(current).intersect(length),
		};
		let Some(length) = length else {
			return;
		};
		fact.length = self.facts.interval(length);
		let id = self.facts.fact(fact);
		let place = self.symbol_place(symbol);
		self.body.flow.set_fact(place, id);
	}

	/// Prove normalized endpoints, or reject a bound certainly beyond the
	/// longest possible length. An inclusive end must name an existing item.
	fn slice_proven(&mut self, n: NodeId, target: NodeId, index: NodeId) -> bool {
		let index = self.strip_groups(index);
		let mut bounds = Vec::with_capacity(2);
		let mut known = true;
		match self.view(index) {
			View::Range {
				start,
				end,
				inclusive,
			} => {
				for (bound, item) in [(start, false), (end, inclusive)] {
					if let Some(bound) = bound {
						match self.value_set(bound) {
							Some(set) => bounds.push((set.hull, item)),
							None => known = false,
						}
					}
				}
			}
			_ => match self.a.constants.get(&index) {
				Some(fa_interpreter::Value::Range {
					start,
					end,
					inclusive,
				}) => {
					bounds.extend(start.iter().map(|&v| (Interval::exact(v), false)));
					bounds.extend(end.iter().map(|&v| (Interval::exact(v), *inclusive)));
				}
				_ => return false,
			},
		}
		if bounds.is_empty() {
			return known;
		}
		let Some(length) = self.expression_length(target) else {
			return false;
		};
		let mut proven = known;
		for (bound, item) in bounds {
			let adjustment = if item { I256::ONE } else { I256::ZERO };
			if bound.lo > length.hi - adjustment || bound.hi < -length.hi {
				let fix = if item {
					"an inclusive end must designate an existing item; use `..length` for an exclusive end, or `..=length - 1` for the last item"
				} else {
					"use bounds from `-length` to `length`"
				};
				let diagnostic =
					self.diagnostic(&E_NUM_17, n, format!("this slice is out of bounds: {fix}"));
				self.report_flow(diagnostic);
				return true;
			}
			proven &= bound.lo >= -length.lo && bound.hi <= length.lo - adjustment;
		}
		proven
	}

	/// `c.length` on a local (`Some(c)`).
	pub(crate) fn length_access(&self, n: NodeId) -> Option<SymbolId> {
		let n = self.strip_groups(n);
		let View::Access { target, members } = syntax::view(&self.tree, n) else {
			return None;
		};
		let member = members.first().filter(|_| members.len() == 1)?;
		let View::Name(name) = syntax::view(&self.tree, member) else {
			return None;
		};
		if self.tree.name(name) != "length" {
			return None;
		}
		self.read_local(target)
	}

	/// The relation an upper bound gives (`exclusive`: `i < bound`): for
	/// `c.length - k` with an immutable collection `c`, `(c, k)` (meaning
	/// `i + k < c.length`), adjusted for an inclusive bound.
	pub(crate) fn length_relation(
		&mut self,
		bound: NodeId,
		exclusive: bool,
	) -> Option<(PlaceId, i32)> {
		self.length_relation_in(bound, exclusive, None)
	}

	/// [`Walker::length_relation`] for the bound of a `for` over a range: a
	/// `mutable` collection also gives one when the loop body only reads its
	/// items, writes them and reads its length (`isPrime[multiple] = false`):
	/// its length can't change during the loop (decisions.md, "Index proofs").
	pub(crate) fn length_relation_in(
		&mut self,
		bound: NodeId,
		exclusive: bool,
		body: Option<NodeId>,
	) -> Option<(PlaceId, i32)> {
		let bound = self.strip_groups(bound);
		let (collection, k) = match syntax::view(&self.tree, bound) {
			View::Arithmetic {
				op: ArithOp::Subtract,
				operands,
			} if operands.len() == 2 => {
				let collection = self.length_access(operands.first()?)?;
				let k = self.value_set(operands.get(1)?)?.as_exact()?;
				(collection, i32::try_from(k).ok()?)
			}
			_ => (self.length_access(bound)?, 0),
		};
		let kept = self.a.symbols.symbol(collection).kind == SymbolKind::Mutable
			&& match body {
				Some(body) => self.length_stable_in(collection, body),
				None => self.length_stable_in_loop(collection),
			};
		if !kept && !self.fixed_length(collection) {
			return None;
		}
		let place = self.symbol_place(collection);
		Some((place, if exclusive { k } else { k - 1 }))
	}

	/// Whether the length of a `mutable` collection can't change in `n`: its
	/// name is only used for items and `length` there.
	fn length_stable_in(&self, collection: SymbolId, n: NodeId) -> bool {
		let name = self
			.s
			.types
			.syms
			.resolve(self.a.symbols.symbol(collection).name);
		self.only_items_used(n, name)
	}

	/// Whether the innermost loop (of this function) leaves the length of a
	/// `mutable` collection unchanged: a relation found in its condition or
	/// its body holds until the end of the iteration (`while i < c.length`).
	fn length_stable_in_loop(&self, collection: SymbolId) -> bool {
		self.body.loops.last().is_some_and(|cx| {
			cx.function_depth == self.body.functions.len()
				&& self.length_stable_in(collection, cx.node)
		})
	}

	/// Whether a `for` loop iterates over the collection here: its length
	/// can't change until the loop ends (the iteration rule, `REF012`).
	fn is_iterated(&mut self, collection: SymbolId) -> bool {
		let place = self.symbol_place(collection);
		self.body.flow.iterating.contains(&place)
	}

	/// Whether every use of the name `name` below `n` is an item access
	/// `name[…]` or `name.length`: nothing there can change its length. A
	/// local of the same name hiding it only makes the answer `false`.
	fn only_items_used(&self, n: NodeId, name: &str) -> bool {
		let is_name = |node: NodeId| matches!(syntax::view(&self.tree, node), View::Name(found) if self.tree.name(found) == name);
		match syntax::view(&self.tree, n) {
			View::Name(_) => !is_name(n),
			View::Index { target, index } if is_name(target) => self.only_items_used(index, name),
			View::Access { target, members }
				if is_name(target)
					&& members.len() == 1
					&& members
						.first()
						.is_some_and(|member| self.tree.text(member.index()) == "length") =>
			{
				true
			}
			// `name.push(value)`, `name.insert(…)`: the length only grows
			View::Call { callee, arguments }
				if matches!(syntax::view(&self.tree, callee), View::Access { target, members }
				if is_name(target)
					&& members.len() == 1
					&& members.first().is_some_and(|member| {
						matches!(self.tree.text(member.index()), "push" | "insert")
					})) =>
			{
				arguments
					.iter()
					.all(|argument| self.only_items_used(argument, name))
			}
			_ => syntax::children(&self.tree, n).all(|child| self.only_items_used(child, name)),
		}
	}

	/// Records a relation on an integer local in the current state.
	pub(crate) fn relate_below(&mut self, symbol: SymbolId, relation: (PlaceId, i32)) {
		let place = self.symbol_place(symbol);
		let mut fact = self
			.body
			.flow
			.fact(place)
			.map_or(crate::flow::Fact::TOP, |fact| self.facts.fact_value(fact));
		fact.below = Some(relation);
		let id = self.facts.fact(fact);
		self.body.flow.set_fact(place, id);
	}

	/// The relation of an index to a collection: `Some(c)` when
	/// `index < c.length` follows from a fact (`i` or `i + c` with `c <= k`).
	fn index_below(&mut self, index: NodeId, collection: PlaceId) -> bool {
		let index = self.strip_groups(index);
		let (local, added) = match syntax::view(&self.tree, index) {
			View::Arithmetic {
				op: ArithOp::Add,
				operands,
			} if operands.len() == 2 => {
				let (Some(left), Some(right)) = (operands.first(), operands.get(1)) else {
					return false;
				};
				match (self.read_local(left), self.read_local(right)) {
					(Some(local), None) => {
						(local, self.value_set(right).and_then(|set| set.as_exact()))
					}
					(None, Some(local)) => {
						(local, self.value_set(left).and_then(|set| set.as_exact()))
					}
					_ => return false,
				}
			}
			_ => match self.read_local(index) {
				Some(local) => (local, Some(I256::ZERO)),
				None => return false,
			},
		};
		let Some(added) = added.and_then(|added| i32::try_from(added).ok()) else {
			return false;
		};
		let Some(fact) = self.symbol_fact(local) else {
			return false;
		};
		matches!(fact.below, Some((place, k)) if place == collection && added <= k)
	}

	/// The flow part of `target[index]` (read or written, called by
	/// `expr.rs` and `objects.rs`): `INDEX_CHECK` and the error path unless
	/// the facts prove the index; `E-NUM-17` when it is certainly out.
	pub(crate) fn check_index_bounds(&mut self, n: NodeId, target: NodeId, index: NodeId) {
		let target_ty = self.type_of(self.strip_groups(target));
		self.check_index_bounds_of(n, target, index, target_ty);
	}

	pub(super) fn check_index_bounds_of(
		&mut self,
		n: NodeId,
		target: NodeId,
		index: NodeId,
		target_ty: TypeId,
	) {
		self.clear_flag(n, NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL);
		let error = match self.s.types.kind(target_ty) {
			TypeKind::Collection(
				CollKind::Array
				| CollKind::InlineArray
				| CollKind::GrowingArray
				| CollKind::OrderedColumnar,
				..,
			)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Bytes => "IndexError",
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..) => "KeyNotFound",
			// a `CArray`: an index within its count is proven (from the end too)
			TypeKind::CArray(_, count) => {
				let index_ty = self.type_of(self.strip_groups(index));
				if let Some(int) = self.s.types.as_int(index_ty) {
					let count = ethnum::I256::from(count);
					if int.min >= -count && int.max < count {
						return;
					}
				}
				"IndexError"
			}
			_ => return,
		};
		if !self.body.flow.reachable {
			return;
		}
		// a slice `items[1..3]`: its bounds between `-length` and `length`
		if error == "IndexError" && self.type_of(self.strip_groups(index)) == TypeId::RANGE {
			if !self.slice_proven(n, target, index) {
				self.set_flag(n, NodeFlags::INDEX_CHECK);
				let error = self.builtin_error(error);
				self.can_fail(n, error);
			}
			return;
		}
		// an index that is not an integer: already reported
		if error == "IndexError"
			&& self
				.s
				.types
				.as_int(self.type_of(self.strip_groups(index)))
				.is_none()
		{
			return;
		}
		if error == "IndexError" && self.index_proven(n, target, index) {
			return;
		}
		self.set_flag(n, NodeFlags::INDEX_CHECK);
		let error = self.builtin_error(error);
		self.can_fail(n, error);
	}

	/// Whether the facts prove `target[index]` in bounds (`E-NUM-17` when
	/// they prove it out: the result is then "proven", nothing to check).
	fn index_proven(&mut self, n: NodeId, target: NodeId, index: NodeId) -> bool {
		let Some(values) = self.value_set(index) else {
			return false;
		};
		let collection = self.read_local(target);
		let length = self.expression_length(target);
		let (lo, hi) = (values.hull.lo, values.hull.hi);
		if let Some(length) = length {
			// Every index in [-length, length) is valid, including an
			// interval spanning the negative and non-negative representations.
			let shortest = length.lo;
			if lo >= -shortest && hi < shortest {
				return true;
			}
			// certainly out: beyond the longest possible length
			let longest = length.hi;
			if lo >= longest || hi < -longest {
				let tree = Arc::clone(&self.tree);
				let message = format!(
					"`{}` is out of bounds: the index is {}, and the length {}; use an index from `-length` to `length - 1`, or `get(index)` for an optional result",
					tree.text(n.index()),
					values
						.hull
						.as_exact()
						.map_or_else(|| format!("at least {lo}"), |value| value.to_string()),
					length.as_exact().map_or_else(
						|| format!("at most {longest}"),
						|value| format!("is {value}")
					)
				);
				let diagnostic = self.diagnostic(&E_NUM_17, n, message);
				self.report_flow(diagnostic);
				return true;
			}
		}
		// a relation `index < c.length` with a non-negative index (a `mutable`
		// collection: inside a loop that can't change its length)
		if lo >= I256::ZERO
			&& let Some(symbol) = collection
			&& (self.fixed_length(symbol)
				|| self.length_stable_in_loop(symbol)
				|| self.is_iterated(symbol))
		{
			let place = self.symbol_place(symbol);
			if self.index_below(index, place) {
				return true;
			}
			// `index < c.length` with `c` no longer than the target
			// (`mutable results = numbers`, then `results[index]`)
			if let Some(length) = length
				&& let Some(local) = self.read_local(index)
				&& let Some(fact) = self.symbol_fact(local)
				&& let Some((other, 0)) = fact.below
				&& let Some(other) = self.place_symbol(other)
				&& let Some(other_length) = self.length_of(other)
				&& other_length.hi <= length.lo
			{
				return true;
			}
		}
		false
	}
}
