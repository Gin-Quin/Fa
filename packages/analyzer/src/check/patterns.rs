//! Patterns: destructuring declarations, object, array and tuple patterns
//! (`when` patterns are handled by `when`, with exhaustiveness).

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{BindingKind, NodeList, PatternFields};

impl Walker<'_> {
	/// `let { a, b } = object`
	pub(super) fn check_destructuring(
		&mut self,
		n: NodeId,
		_kind: BindingKind,
		_pattern: NodeId,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "destructuring")
	}

	/// `{ a, b as c, d = 4, nested { e }, ...rest }`
	pub(super) fn check_object_pattern(
		&mut self,
		n: NodeId,
		_fields: PatternFields<'_>,
		_rest: Option<NodeId>,
	) -> TypeId {
		self.unsupported(n, "object patterns")
	}

	/// `[first, second, ...others]`
	pub(super) fn check_array_pattern(
		&mut self,
		n: NodeId,
		_items: NodeList<'_>,
		_rest: Option<NodeId>,
	) -> TypeId {
		self.unsupported(n, "array patterns")
	}

	/// `(number, name)`
	pub(super) fn check_tuple_pattern(&mut self, n: NodeId, _items: NodeList<'_>) -> TypeId {
		self.unsupported(n, "tuple patterns")
	}
}
