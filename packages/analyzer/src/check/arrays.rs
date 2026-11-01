//! Arrays (types brief §5.1): literals, the item type of an iteration. The
//! members (`length`, `push`) and indexing are in `objects.rs`.

use crate::check::Walker;
use crate::diagnostics::codes::types::TY006;
use crate::diagnostics::codes::values::E_DECL_9;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{NodeList, View};
use crate::types::{CollKind, TypeKind};

impl Walker<'_> {
	/// `[a, b, c]`: an `Array` of the expected item type, or of the join of
	/// the widened item types (D52). `[]` without context is an
	/// `Array(Never)`, which a declaration can't hold (`E-DECL-11`).
	pub(super) fn check_array_literal(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let expected_item = expected.and_then(|expected| self.expected_item(expected));
		let mut types = Vec::with_capacity(items.len());
		for item in items.iter() {
			if let View::Spread(_) = self.view(item) {
				self.unsupported(item, "spreads in arrays");
				types.push((item, TypeId::UNKNOWN));
				continue;
			}
			let ty = self.check(item, expected_item);
			if let Some(expected_item) = expected_item {
				self.sink(item, ty, expected_item);
			}
			types.push((item, ty));
		}
		self.memo(n, |w| {
			if let Some(item) = expected_item {
				return w.s.types.array(item);
			}
			if types.is_empty() {
				return w.s.types.array(TypeId::NEVER);
			}
			if types
				.iter()
				.any(|&(_, ty)| w.s.types.is_recovering(ty) || ty.is_unset())
			{
				return TypeId::UNKNOWN;
			}
			if let Some(&(item, _)) = types.iter().find(|&&(_, ty)| ty == TypeId::VOID) {
				let diagnostic = w.diagnostic(
					&E_DECL_9,
					item,
					"this expression has no value (`Void`): an array can't hold it",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let integer = types
				.iter()
				.any(|&(_, ty)| w.s.types.is_literal(ty) && w.s.types.as_int(ty).is_some());
			let float = types
				.iter()
				.any(|&(_, ty)| matches!(w.s.types.kind(ty), TypeKind::FloatLit(..)));
			if integer && float {
				let diagnostic = w.diagnostic(
					&TY006,
					n,
					"integers and floats are mixed: write the integers as floats (`1.0`), or annotate the array",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let widened: Vec<TypeId> = types.iter().map(|&(_, ty)| w.join_member(ty)).collect();
			let item = w.s.types.union(&widened);
			w.check_distinguishable(n, item);
			w.s.types.array(item)
		})
	}

	/// A type as a member of a join: widened, and a member of a named union
	/// or an enumeration widens to the whole type (D53).
	pub(super) fn join_member(&mut self, ty: TypeId) -> TypeId {
		match self.s.types.kind(ty) {
			TypeKind::Member(nominal, _) | TypeKind::EnumMember(nominal, _) => {
				self.s.types.nominal_type(nominal)
			}
			_ => self.s.types.widen(ty),
		}
	}

	/// The item type an expected type gives to the items of a list literal.
	fn expected_item(&self, expected: TypeId) -> Option<TypeId> {
		match self.s.types.kind(expected) {
			TypeKind::Collection(CollKind::Array, item, _) => Some(item),
			TypeKind::Union(list) => self.s.types.list_items(list).iter().find_map(|&member| {
				match self.s.types.kind(member) {
					TypeKind::Collection(CollKind::Array, item, _) => Some(item),
					_ => None,
				}
			}),
			_ => None,
		}
	}

	/// The type of the values a `for` loop over a value of type `ty` gives
	/// (`for item in items`): the items of an array, the characters of a
	/// string. `None` for a value that can't be iterated (yet).
	pub fn iteration_item(&self, ty: TypeId) -> Option<TypeId> {
		match self.s.types.kind(ty) {
			TypeKind::Collection(CollKind::Array, item, _) => Some(item),
			TypeKind::String | TypeKind::StringLit(_) => Some(TypeId::CHARACTER),
			TypeKind::Unknown => Some(TypeId::UNKNOWN),
			_ => None,
		}
	}
}
