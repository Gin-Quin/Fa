//! Pools and columnars (types brief §5.5, §5.6, `collections/pools.md`,
//! `collections/columnars.md`, `collections/growing-collections.md`):
//! the members of `Pool`, `GrowingPool`, `Columnar`, `GrowingColumnar` and
//! `OrderedColumnar`, `REF020` (alias `CL050`: deleting an element of another collection)
//! and `WCL002` (a copy of what `add` returns).
//!
//! - `Pool` and `Columnar` (and their growing variants) have the pool API:
//!   `add(value)` returns a fresh reference to the stored element
//!   (`mutates self.size`), `delete(element)` and `take(element)` take a
//!   reference into the same collection (`invalidates element`), callbacks
//!   receive `(item)`. Paths `Pool#…` and `Columnar#…`.
//! - `OrderedColumnar` has the positional API of an `Array` (`length`,
//!   indices, `delete(index)`, `pop`) with `add` appending (`mutates self`),
//!   callbacks receive `(item, index)`. Paths `OrderedColumnar#…`.
//!
//! The effects are the reference table (`builtins::array::effects`).

use crate::builtins;
use crate::check::Walker;
use crate::check::objects::MemberKind;
use crate::diagnostics::codes::advanced::REF020;
use crate::diagnostics::codes::types::{CL007, WCL002};
use crate::flow::Step;
use crate::ids::{NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::View;
use crate::types::{CollKind, IntStorage};

/// The builtin family of a pool or columnar kind: the owner of its paths.
pub(crate) fn pool_owner(kind: CollKind) -> Option<&'static str> {
	Some(match kind {
		CollKind::Pool | CollKind::GrowingPool => "Pool",
		CollKind::Columnar | CollKind::GrowingColumnar => "Columnar",
		CollKind::OrderedColumnar => "OrderedColumnar",
		_ => return None,
	})
}

/// Whether a kind is a stable-address collection: elements never move
/// (`Pool`, `GrowingPool`, `Columnar`, `GrowingColumnar`; relations need
/// one, advanced brief §8.2, D16).
pub(crate) fn has_stable_addresses(kind: CollKind) -> bool {
	matches!(
		kind,
		CollKind::Pool | CollKind::GrowingPool | CollKind::Columnar | CollKind::GrowingColumnar
	)
}

impl Walker<'_> {
	/// A member of a pool or a columnar.
	pub(super) fn pool_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		kind: CollKind,
		item: TypeId,
		text: &str,
	) -> Option<(TypeId, MemberKind)> {
		let owner = pool_owner(kind)?;
		let positional = kind == CollKind::OrderedColumnar;
		let (wrong, right) = if positional {
			("size", "length")
		} else {
			("length", "size")
		};
		if text == wrong {
			let message = if positional {
				format!(
					"an `{}` has a `length`, not a `size`: its elements have positions",
					kind.name()
				)
			} else {
				format!(
					"a `{}` has a `size`, not a `length`: its elements have no positions",
					kind.name()
				)
			};
			let diagnostic = self.diagnostic(&CL007, member, message);
			self.report(diagnostic);
			return None;
		}
		if self.growing_lacks(member, kind, text) {
			return None;
		}
		let builtin = builtins::by_path(&format!("{owner}#{text}"));
		let ty = self.pool_method_type(kind, text, item, right);
		match (builtin, ty) {
			(Some(builtin), Some(ty)) => {
				self.set_resolution(member, Resolution::Builtin(builtin));
				Some((self.with_effects(ty, kind, text), MemberKind::Builtin))
			}
			_ => {
				self.no_member(member, receiver, text);
				None
			}
		}
	}

	/// The type of a method of a pool or a columnar (types brief §5.5, §5.6).
	fn pool_method_type(
		&mut self,
		kind: CollKind,
		name: &str,
		item: TypeId,
		count: &str,
	) -> Option<TypeId> {
		let positional = kind == CollKind::OrderedColumnar;
		let natural = {
			let storage = IntStorage::Default;
			self.s
				.types
				.int_range(ethnum::I256::ZERO, storage.range().hi, storage)
		};
		// `filter` keeps the kind; a growing collection gives its counterpart (D34)
		let standard = match kind {
			CollKind::GrowingPool => CollKind::Pool,
			CollKind::GrowingColumnar => CollKind::Columnar,
			other => other,
		};
		let same = self.s.types.collection(standard, item, TypeId::NONE);
		let optional_item = self.s.types.optional(item);
		let callback_params: &[TypeId] = if positional {
			&[item, TypeId::INTEGER]
		} else {
			&[item]
		};
		let ignorable = u8::from(positional);
		let predicate = self.callback_with(callback_params, TypeId::BOOLEAN, ignorable);
		Some(match name {
			_ if name == count => return Some(natural),
			"capacity" => return Some(natural),
			"reserve" => self.method(&[natural], TypeId::VOID),
			"isEmpty" => self.method(&[], TypeId::BOOLEAN),
			"add" => self.method(&[item], item),
			"delete" if positional => self.method(&[TypeId::INTEGER], TypeId::VOID),
			"take" if positional => self.method(&[TypeId::INTEGER], item),
			// `delete(element)`, `take(element)`: a reference into the collection (REF020)
			"delete" => self.method(&[item], TypeId::VOID),
			"take" => self.method(&[item], item),
			"pop" if positional => self.method(&[], optional_item),
			"get" if positional => self.method(&[TypeId::INTEGER], optional_item),
			"indexOf" if positional => {
				let index = self.s.types.optional(natural);
				self.method(&[item], index)
			}
			"deleteWhere" => {
				let predicate = self.callback_with(&[item], TypeId::BOOLEAN, 0);
				self.method(&[predicate], TypeId::VOID)
			}
			"clear" => self.method(&[], TypeId::VOID),
			"includes" => self.method(&[item], TypeId::BOOLEAN),
			"find" => self.method(&[predicate], optional_item),
			"filter" => self.method(&[predicate], same),
			"every" | "some" => self.method(&[predicate], TypeId::BOOLEAN),
			"map" => {
				let result = self.builtin_param("Pool.map", "Result", TypeId::UNKNOWN);
				let mut params = callback_params.to_vec();
				params[0] = item;
				let transform = self.callback_with(&params, result, ignorable);
				let mapped = self.s.types.array(result);
				self.method(&[transform], mapped)
			}
			"reduce" => {
				let accumulator = self.builtin_param("Pool.reduce", "Accumulator", TypeId::UNKNOWN);
				let mut params = vec![accumulator, item];
				if positional {
					params.push(TypeId::INTEGER);
				}
				let combine = self.callback_with(&params, accumulator, ignorable);
				self.method(&[combine, accumulator], accumulator)
			}
			"sum" => {
				let sum = self.array_method_type("sum", item)?;
				let ret = self.signature_of(sum)?.ret;
				self.method(&[], ret)
			}
			"values" => self.values_method(item),
			_ => return None,
		})
	}

	/// The indexing of a pool or a columnar: only an `OrderedColumnar` has
	/// positions.
	pub(super) fn pool_index_type(&mut self, n: NodeId, kind: CollKind, item: TypeId) -> TypeId {
		if kind == CollKind::OrderedColumnar {
			return item;
		}
		let message = format!(
			"a `{}` has no positions: it can't be indexed; keep the reference `add` returns (`use element = collection.add(…)`), or `find` the element",
			kind.name()
		);
		let diagnostic = self.diagnostic(&crate::diagnostics::codes::values::E_TYPE_1, n, message);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// `REF020` (`CL050`): `delete(element)` and `take(element)` of a pool or a
	/// columnar take a reference into that very collection: not a copy, not
	/// an element of another collection. Flow part of the call.
	pub(crate) fn check_element_of(&mut self, callee: NodeId, argument: NodeId, method: &str) {
		if !self.body.flow.reachable {
			return;
		}
		let collections = self.callee_receiver_places(callee);
		if collections.is_empty() {
			return;
		}
		let elements: Vec<_> = self
			.reference_origins(argument)
			.into_iter()
			.map(|place| self.canonical_reference_place(place))
			.collect();
		let inside = !elements.is_empty()
			&& collections.iter().all(|&collection| {
				elements.iter().all(|&element| {
					let entry = self.places.get(element);
					entry.parent == Some(collection) && matches!(entry.step, Some(Step::Index(_)))
				})
			});
		if inside || self.s.types.is_recovering(self.type_of(argument)) {
			return;
		}
		let text = self.tree.text(argument.index()).to_string();
		let collection_text = collections
			.into_iter()
			.map(|place| self.place_text(place))
			.collect::<Vec<_>>()
			.join(" or ");
		let message = format!(
			"`{text}` is not an element of `{collection_text}`: `{method}` takes a reference into it, from `use {text} = {collection_text}.add(…)`, `find` or a loop"
		);
		let diagnostic = self.diagnostic(&REF020, argument, message);
		self.report_flow(diagnostic);
	}

	pub(crate) fn callee_receiver_places(&mut self, callee: NodeId) -> Vec<crate::ids::PlaceId> {
		let callee = self.strip_groups(callee);
		let View::Access { target, members } = self.view(callee) else {
			return Vec::new();
		};
		let members: Vec<_> = members.iter().collect();
		self.access_places(target, &members[..members.len().saturating_sub(1)])
	}

	/// The receiver's last node (`text` in `self.text.method()`), whose type
	/// is the receiver type even when the parser flattens the member chain.
	pub(crate) fn callee_receiver_node(&self, callee: NodeId) -> Option<NodeId> {
		let callee = self.strip_groups(callee);
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let members: Vec<NodeId> = members.iter().collect();
		match members.len() {
			0 => None,
			1 => Some(target),
			length => Some(members[length - 2]),
		}
	}

	/// The type of the receiver of a method call `a.b.method(…)`: `a.b`'s.
	pub(crate) fn callee_receiver_type(&self, callee: NodeId) -> Option<TypeId> {
		self.callee_receiver_node(callee)
			.map(|node| self.type_of(node))
	}

	/// `WCL002`: `let duplicate = pool.add(…)` copies the stored element; the
	/// reference is kept with `use`. Type part of a declaration.
	pub(crate) fn warn_copied_add(&mut self, value: NodeId) {
		let value = self.strip_groups(value);
		let View::Call { callee, .. } = self.view(value) else {
			return;
		};
		let View::Access { target, members } = self.view(self.strip_groups(callee)) else {
			return;
		};
		let Some(member) = members.last() else {
			return;
		};
		let Resolution::Builtin(builtin) = self.a.resolution(member) else {
			return;
		};
		let path = builtins::get(builtin).path;
		if !matches!(path, "Pool#add" | "Columnar#add" | "OrderedColumnar#add") {
			return;
		}
		let collection = self.tree.text(target.index()).to_string();
		let message = format!(
			"bad practice: this is a copy of the stored element; keep the reference with `use … = {collection}.add(…)`"
		);
		let diagnostic = self.diagnostic(&WCL002, value, message);
		self.report(diagnostic);
	}
}
