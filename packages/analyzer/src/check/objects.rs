//! Objects: untyped and typed literals, fields, spreads, `.=`, `readonly`.

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::NodeList;

impl Walker<'_> {
	/// `{ a = 1, b, ...c }`
	pub(super) fn check_object(
		&mut self,
		n: NodeId,
		_items: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "objects")
	}

	/// `Point { x = 1 }`
	pub(super) fn check_typed_object(
		&mut self,
		n: NodeId,
		_target: NodeId,
		_members: NodeList<'_>,
	) -> TypeId {
		self.unsupported(n, "typed object literals")
	}

	/// `target .= { field = value }`
	pub(super) fn check_update_fields(
		&mut self,
		n: NodeId,
		_target: NodeId,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "`.=`")
	}

	/// `readonly field: T` (in a type; handled by the type).
	pub(super) fn check_readonly(&mut self, n: NodeId, _field: NodeId) -> TypeId {
		self.unsupported(n, "`readonly`")
	}
}
