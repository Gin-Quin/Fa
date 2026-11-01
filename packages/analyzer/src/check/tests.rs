//! `test` blocks and `assert` (advanced brief §18).

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::TestModifier;

impl Walker<'_> {
	/// `test "name" { … }`
	pub(super) fn check_test(
		&mut self,
		n: NodeId,
		_name: NodeId,
		_modifier: Option<TestModifier>,
		_body: NodeId,
		_compile_time: bool,
	) -> TypeId {
		self.unsupported(n, "tests")
	}

	/// `assert condition`
	pub(super) fn check_assert(&mut self, n: NodeId, _condition: NodeId) -> TypeId {
		self.unsupported(n, "`assert`")
	}
}
