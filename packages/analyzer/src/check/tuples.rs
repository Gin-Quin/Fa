//! Tuples (values brief §10, `values/tuples.md`): values `(a, b, …)`, types
//! `Tuple(A, B, …)` and `(A, B, …)`, indexing with a literal `t[0]`.
//!
//! - A tuple value has the widened types of its items (`(1, "one")` is a
//!   `Tuple(Integer, String)`); with an expected tuple type of the same
//!   arity, each item is checked against its element type (literals adapt)
//!   and the value has the expected type.
//! - `()`, `Tuple()` and `Tuple(A)` are `E-TUP-1`; a non-literal index is
//!   `E-TUP-2`, an index out of range or negative `E-TUP-3`. `t.0` is a
//!   parse error (`E-TUP-4`).
//! - Equality is element-wise (`ops.rs`, through assignability); tuples have
//!   no order.

use crate::check::Walker;
use crate::diagnostics::codes::values::{E_TUP_1, E_TUP_2, E_TUP_3};
use crate::ids::{NodeId, TypeId};
use crate::syntax::NodeList;
use crate::types::TypeKind;

impl Walker<'_> {
	/// `(a, b, …)` in value position.
	pub(super) fn check_tuple_value(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let expected_items = expected
			.and_then(|expected| self.tuple_items(expected))
			.filter(|elements| elements.len() == items.len());
		let mut types = Vec::with_capacity(items.len());
		for (index, item) in items.iter().enumerate() {
			let element = expected_items.as_ref().map(|elements| elements[index]);
			let ty = self.check(item, element);
			self.store_closures(item, n);
			if let Some(element) = element {
				self.sink(item, ty, element);
			}
			types.push(ty);
		}
		self.memo(n, |w| {
			if items.len() < 2 {
				return w.unit_tuple(n);
			}
			if let (Some(_), Some(expected)) = (&expected_items, expected) {
				return expected;
			}
			if types
				.iter()
				.any(|&ty| w.s.types.is_recovering(ty) || ty.is_unset())
			{
				return TypeId::UNKNOWN;
			}
			if let Some(index) = types.iter().position(|&ty| ty == TypeId::VOID) {
				let item = items.get(index).unwrap_or(n);
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::values::E_DECL_9,
					item,
					"this expression has no value (`Void`): a tuple can't hold it",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let widened: Vec<TypeId> = types.iter().map(|&ty| w.s.types.widen(ty)).collect();
			w.s.types.tuple(&widened)
		})
	}

	/// `Tuple(A, B)` or `(A, B)` in a type: at least two elements (`E-TUP-1`).
	pub(super) fn tuple_type(&mut self, n: NodeId, items: NodeList<'_>) -> TypeId {
		let elements: Vec<TypeId> = items
			.iter()
			.map(|item| self.check_type_expr(item))
			.collect();
		self.memo(n, |w| {
			if elements.len() < 2 {
				return w.unit_tuple(n);
			}
			w.s.types.tuple(&elements)
		})
	}

	/// `E-TUP-1`: `()`, `Tuple()`, `Tuple(A)`. Type part.
	pub(super) fn unit_tuple(&mut self, n: NodeId) -> TypeId {
		let diagnostic = self.diagnostic(
			&E_TUP_1,
			n,
			"a tuple has at least two elements: there is no empty or one-element tuple (`(value)` is a parenthesized expression)",
		);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// The element types of a tuple type.
	pub(crate) fn tuple_items(&self, ty: TypeId) -> Option<Vec<TypeId>> {
		match self.s.types.kind(ty) {
			TypeKind::Tuple(list) => Some(self.s.types.list_items(list).to_vec()),
			_ => None,
		}
	}

	/// `tuple[index]`: the index is a literal between 0 and the arity,
	/// excluded (`E-TUP-2`, `E-TUP-3`). Type part.
	pub(super) fn tuple_index_type(
		&mut self,
		index: NodeId,
		elements: &[TypeId],
		index_ty: TypeId,
	) -> TypeId {
		if self.s.types.is_recovering(index_ty) {
			return TypeId::UNKNOWN;
		}
		let value = self
			.s
			.types
			.as_int(index_ty)
			.and_then(|int| int.literal())
			.filter(|_| self.is_number_literal(index));
		let Some(value) = value else {
			let diagnostic = self.diagnostic(
				&E_TUP_2,
				index,
				"a tuple index must be a literal, ex: `pair[0]`: the type of the element depends on it",
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		};
		match usize::try_from(value.as_i128())
			.ok()
			.and_then(|at| elements.get(at))
		{
			Some(&element) => element,
			None => {
				let message = format!(
					"this tuple has {} elements: its indices go from 0 to {}",
					elements.len(),
					elements.len().saturating_sub(1)
				);
				let diagnostic = self.diagnostic(&E_TUP_3, index, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
		}
	}
}
