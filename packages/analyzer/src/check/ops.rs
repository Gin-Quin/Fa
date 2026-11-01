//! Comparisons, equality, `is`, `is in`, logic, `??`, `?.`.

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{CompareOps, LogicOp, NodeList};

impl Walker<'_> {
	/// `a == b`, `a != b`
	pub(super) fn check_equality(
		&mut self,
		n: NodeId,
		_left: NodeId,
		_right: NodeId,
		_negated: bool,
	) -> TypeId {
		self.unsupported(n, "equality")
	}

	/// `a < b <= c`: `a < b and b <= c`, each middle operand evaluated once.
	pub(super) fn check_comparison(
		&mut self,
		n: NodeId,
		_operands: NodeList<'_>,
		_operators: CompareOps<'_>,
	) -> TypeId {
		self.unsupported(n, "comparisons")
	}

	/// `and` / `or`, folding left.
	pub(super) fn check_logic(
		&mut self,
		n: NodeId,
		_op: LogicOp,
		_operands: NodeList<'_>,
	) -> TypeId {
		self.unsupported(n, "`and` and `or`")
	}

	/// `not x`
	pub(super) fn check_not(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "`not`")
	}

	/// `a ?? b ?? c`, folding left.
	pub(super) fn check_fallback(
		&mut self,
		n: NodeId,
		_operands: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`??`")
	}

	/// `a is T`, `a is not T`
	pub(super) fn check_is(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_ty: NodeId,
		_negated: bool,
	) -> TypeId {
		self.unsupported(n, "`is`")
	}

	/// `a is in b`, `a is not in b`
	pub(super) fn check_is_in(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_collection: NodeId,
		_negated: bool,
	) -> TypeId {
		self.unsupported(n, "`is in`")
	}

	/// `a?.b?.c`
	pub(super) fn check_optional_access(
		&mut self,
		n: NodeId,
		_target: NodeId,
		_members: NodeList<'_>,
	) -> TypeId {
		self.unsupported(n, "`?.`")
	}
}
