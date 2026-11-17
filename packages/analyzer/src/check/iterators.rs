//! Iterators (types brief §5.0.6, `control-flow/loops.md`, "Element by
//! element"): `values()` of every collection and `keys()` of maps give an
//! `Iterator(T)`, lazy and element by element; its methods `filter`, `map`
//! and `firstValues(count = 1)` give iterators, `find`, `reduce`, `every`,
//! `some` and `toArray()` consume it, and so does a `for` loop.
//!
//! An iterator is a local reference into its collection: a local holding
//! one is invalidated like a `use` by a change of the collection
//! (`REF001`), a loop over one iterates the collection (`REF012`), and it
//! can't be stored in a field or a collection, returned, or captured by an
//! escaping closure (`CL030`).
//!
//! The backends lower the methods by their paths (`Iterator#filter`…) and
//! the collection methods `Array#values`, `Set#values`, `Map#values`,
//! `Map#keys`, `Pool#values`, `Columnar#values`.

use std::sync::Arc;

use crate::check::Walker;
use crate::check::objects::MemberKind;
use crate::diagnostics::codes::types::CL030;
use crate::ids::{NodeId, PlaceId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, View};
use crate::types::{IntStorage, TypeFlags, TypeKind};

impl Walker<'_> {
	/// The members of an `Iterator(item)`.
	pub(super) fn iterator_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		item: TypeId,
		text: &str,
	) -> Option<(TypeId, MemberKind)> {
		let iterator = self.s.types.iterator(item);
		let predicate = self.callback_with(&[item], TypeId::BOOLEAN, 0);
		let ty = match text {
			"filter" => self.method(&[predicate], iterator),
			"map" => {
				let result = self.builtin_param("Iterator.map", "Result", TypeId::UNKNOWN);
				let transform = self.callback_with(&[item], result, 0);
				let mapped = self.s.types.iterator(result);
				self.method(&[transform], mapped)
			}
			"find" => {
				let found = self.s.types.optional(item);
				self.method(&[predicate], found)
			}
			"every" | "some" => self.method(&[predicate], TypeId::BOOLEAN),
			"reduce" => {
				let accumulator =
					self.builtin_param("Iterator.reduce", "Accumulator", TypeId::UNKNOWN);
				let combine = self.callback_with(&[accumulator, item], accumulator, 0);
				self.method(&[combine, accumulator], accumulator)
			}
			"firstValues" => {
				let natural = {
					let storage = IntStorage::Default;
					self.s
						.types
						.int_range(ethnum::I256::ZERO, storage.range().hi, storage)
				};
				self.method(&[natural], iterator)
			}
			"toArray" => {
				let array = self.s.types.array(item);
				self.method(&[], array)
			}
			_ => {
				self.no_member(member, receiver, text);
				return None;
			}
		};
		let builtin = crate::builtins::by_path(&format!("Iterator#{text}"))?;
		self.set_resolution(member, Resolution::Builtin(builtin));
		Some((ty, MemberKind::Builtin))
	}

	/// `values()` (and `keys()` of a map): `() => Iterator(item)`.
	pub(super) fn values_method(&mut self, item: TypeId) -> TypeId {
		let iterator = self.s.types.iterator(item);
		self.method(&[], iterator)
	}

	/// The collection an iterator expression reads, and whether its items
	/// are still the collection's elements (no `map` in the chain):
	/// `people.values().filter(…)` reads `people`. `None` when unknown.
	pub(crate) fn iterator_source(&mut self, n: NodeId) -> Option<(Vec<PlaceId>, bool)> {
		let n = self.strip_groups(n);
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(n) else {
					return None;
				};
				self.iterator_locals.get(&symbol).cloned()
			}
			View::Call { callee, .. } => {
				let callee = self.strip_groups(callee);
				let View::Access { target, members } = syntax::view(&tree, callee) else {
					return None;
				};
				let member = members.last()?;
				let Resolution::Builtin(builtin) = self.a.resolution(member) else {
					return None;
				};
				let path = crate::builtins::get(builtin).path;
				let (owner, method) = path.split_once('#')?;
				// the target of the last member
				let receiver = if members.len() == 1 {
					target
				} else {
					return None;
				};
				match (owner, method) {
					("Iterator", "filter" | "firstValues") => self.iterator_source(receiver),
					("Iterator", "map") => self
						.iterator_source(receiver)
						.map(|(place, _)| (place, false)),
					(_, "values" | "keys") => Some((self.evaluated_origins(receiver).places, true)),
					_ => None,
				}
			}
			_ => None,
		}
	}

	/// `CL030`: an iterator stored in a field or a collection, returned, or
	/// captured by an escaping closure. Type part.
	fn report_stored_iterator(&mut self, n: NodeId, how: &str) {
		if !self.reported_once.insert((n, CL030.id)) {
			return;
		}
		let message = format!(
			"an iterator reads its collection while it runs: it can't be {how}. Consume it where it is written, ex: `.toArray()`"
		);
		let diagnostic = self.diagnostic(&CL030, n, message);
		self.report(diagnostic);
	}

	/// `CL030` for a written type that stores an iterator: a collection of
	/// iterators, an object field, a return type.
	pub(super) fn check_stored_iterator_type(&mut self, n: NodeId, ty: TypeId, how: &str) {
		if self.s.types.flags(ty).contains(TypeFlags::HAS_UNKNOWN) {
			return;
		}
		if self.s.types.contains_iterator(ty) {
			self.report_stored_iterator(n, how);
		}
	}

	/// Declares a local holding an iterator: a reference into its
	/// collection, invalidated by a change of it (like a `use`).
	pub(crate) fn declare_iterator_local(&mut self, symbol: crate::ids::SymbolId, value: NodeId) {
		if !matches!(
			self.s.types.kind(self.a.symbols.symbol(symbol).ty),
			TypeKind::Iterator(..)
		) {
			return;
		}
		let Some((collection, elements)) = self.iterator_source(value) else {
			return;
		};
		self.iterator_locals
			.insert(symbol, (collection.clone(), elements));
		if self.body.flow.reachable {
			let places: Vec<_> = collection
				.into_iter()
				.map(|collection| {
					self.places.child(
						collection,
						crate::flow::Step::Index(crate::flow::Identity::Unknown),
					)
				})
				.collect();
			self.record_alias_origins(symbol, value, places.first().copied());
			let state = self.facts.refs.state(crate::flow::refs::RefState {
				places: places.into(),
				opaque: false,
				bounds: Default::default(),
				readonly: false,
				invalidated: None,
				reported: false,
			});
			self.body.flow.set_ref_state(symbol, state);
			self.body.flow.bind_reference_alias(symbol, None);
		}
	}
}
