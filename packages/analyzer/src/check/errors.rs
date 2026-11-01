//! Errors: `try`, `throw`, `catch` and `throws` coverage (values brief §15).

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::Name;

impl Walker<'_> {
	/// `try expression`
	pub(super) fn check_try(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`try`")
	}

	/// `throw error`
	pub(super) fn check_throw(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "`throw`")
	}

	/// `expression catch error => handler`
	pub(super) fn check_catch(
		&mut self,
		n: NodeId,
		_expression: NodeId,
		_error_name: Name,
		_handler: NodeId,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`catch`")
	}
}
