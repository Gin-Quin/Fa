//! Length facts and index proofs (values brief §6.5 rule 3, decisions.md
//! "Index proofs").
//!
//! - The length of a collection literal, or the grapheme count of a string
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
use crate::builtins::strings::grapheme_count;
use crate::check::Walker;
use crate::diagnostics::codes::values::E_NUM_17;
use crate::ids::{IntervalId, NodeId, PlaceId, SymbolId};
use crate::symbols::SymbolKind;
use crate::syntax::{self, ArithOp, View};
use crate::types::{CollKind, Interval, TypeKind};

impl Walker<'_> {
	/// The length a value has when it is bound: the number of items of an
	/// array literal without spreads, the grapheme count of a string literal.
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
					let count = grapheme_count(self.s.types.syms.resolve(text));
					Some(Interval::exact(I256::from(count as u64)))
				}
				_ => None,
			},
		}
	}

	/// `receiver.method(…)` with a method that changes its receiver (`push`):
	/// the facts of the receiver's place go (its length).
	pub(crate) fn mutated_receiver(&mut self, callee: NodeId) {
		let callee = self.strip_groups(callee);
		let View::Access { target, .. } = syntax::view(&self.tree, callee) else {
			return;
		};
		if let Some(symbol) = self.read_local(target) {
			let place = self.symbol_place(symbol);
			self.body.flow.mutate(&self.places, &mut self.facts, place);
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
		match self.literal_length(value) {
			Some(length) => self.facts.interval(length),
			None => IntervalId::NONE,
		}
	}

	/// The length of an expression here: a local's length fact, a literal's
	/// length, the length of the collection a `map` is called on (values
	/// brief §6.5 rule 3: "equal lengths for … `map`").
	pub(crate) fn expression_length(&mut self, n: NodeId) -> Option<Interval> {
		let n = self.strip_groups(n);
		if let Some(symbol) = self.read_local(n) {
			return self.length_of(symbol);
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
		self.literal_length(n)
	}

	/// The length of a local collection or string here, when known.
	pub(crate) fn length_of(&mut self, symbol: SymbolId) -> Option<Interval> {
		let fact = self.symbol_fact(symbol)?;
		(fact.length != IntervalId::NONE).then(|| self.facts.interval_value(fact.length))
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
		if !self.fixed_length(collection) {
			return None;
		}
		let place = self.symbol_place(collection);
		Some((place, if exclusive { k } else { k - 1 }))
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
		self.clear_flag(n, NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL);
		let target_ty = self.type_of(self.strip_groups(target));
		let error = match self.s.types.kind(target_ty) {
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, ..)
			| TypeKind::String
			| TypeKind::StringLit(_) => "IndexError",
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..) => "KeyNotFound",
			_ => return,
		};
		if !self.body.flow.reachable {
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
			// within [0, length) or [-length, 0) for every possible length
			let shortest = length.lo;
			if (lo >= I256::ZERO && hi < shortest) || (lo >= -shortest && hi < I256::ZERO) {
				return true;
			}
			// certainly out: beyond the longest possible length
			let longest = length.hi;
			if lo >= longest || hi < -longest {
				let tree = Arc::clone(&self.tree);
				let message = format!(
					"`{}` is out of bounds: the index is {}, and the length {}",
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
		// a relation `index < c.length` with a non-negative index
		if lo >= I256::ZERO
			&& let Some(symbol) = collection
			&& self.fixed_length(symbol)
		{
			let place = self.symbol_place(symbol);
			return self.index_below(index, place);
		}
		false
	}
}
