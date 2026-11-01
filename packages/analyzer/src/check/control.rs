//! Control flow: `if`, `when`, `while`, `for`, `loop`, `do`, `break`,
//! `continue`, `return`; forks, joins and the loop fixpoint (design §3.3).

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{Name, NodeList, WhenArms};

impl Walker<'_> {
	/// `if condition { … } else …`: a statement (`Void`) when
	/// `self.is_statement(n)`, otherwise a value.
	pub(super) fn check_if(
		&mut self,
		n: NodeId,
		_condition: NodeId,
		_then_body: NodeId,
		_else_body: Option<NodeId>,
		_compile_time: bool,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`if`")
	}

	/// `when subject { … }`
	pub(super) fn check_when(
		&mut self,
		n: NodeId,
		_subject: NodeId,
		_arms: WhenArms<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`when`")
	}

	/// `while condition { … }`
	pub(super) fn check_while(&mut self, n: NodeId, _condition: NodeId, _body: NodeId) -> TypeId {
		self.unsupported(n, "`while`")
	}

	/// `while let name = value { … }`
	pub(super) fn check_while_let(
		&mut self,
		n: NodeId,
		_pattern: NodeId,
		_value: NodeId,
		_body: NodeId,
	) -> TypeId {
		self.unsupported(n, "`while let`")
	}

	/// `for pattern, index in iterable { … }`
	#[allow(clippy::too_many_arguments)]
	pub(super) fn check_for(
		&mut self,
		n: NodeId,
		_pattern: Option<NodeId>,
		_index: Option<Name>,
		_iterable: NodeId,
		_body: NodeId,
		_parallel: Option<NodeList<'_>>,
		_compile_time: bool,
	) -> TypeId {
		self.unsupported(n, "`for`")
	}

	/// `loop { … }`
	pub(super) fn check_loop(
		&mut self,
		n: NodeId,
		_body: NodeId,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`loop`")
	}

	/// `do { … }`
	pub(super) fn check_do(
		&mut self,
		n: NodeId,
		_body: NodeId,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "`do`")
	}

	/// `break`, `break value`
	pub(super) fn check_break(&mut self, n: NodeId, _value: Option<NodeId>) -> TypeId {
		self.unsupported(n, "`break`")
	}

	/// `continue`
	pub(super) fn check_continue(&mut self, n: NodeId) -> TypeId {
		self.unsupported(n, "`continue`")
	}

	/// `return`, `return value`: implemented with the functions (E1).
	pub(super) fn check_return(&mut self, n: NodeId, value: Option<NodeId>) -> TypeId {
		self.check_return_value(n, value)
	}
}
