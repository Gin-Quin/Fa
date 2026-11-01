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

	/// `assert condition`: a `Boolean` condition (`E-BOOL-1`, alias
	/// `TEST001`, reported by `check_cond`). A false `assert` stops the
	/// program, or fails the test: the code after it runs only when the
	/// condition is true, so it continues from the true state of the
	/// condition, with its narrowings (values brief D-27).
	pub(super) fn check_assert(&mut self, n: NodeId, condition: NodeId) -> TypeId {
		let (when_true, _) = self.check_cond(condition);
		self.body.flow = when_true;
		self.memo(n, |_| TypeId::VOID)
	}
}
