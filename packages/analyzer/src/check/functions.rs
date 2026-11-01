//! Functions: declarations, lambdas, signatures, parameters, summaries, SCC
//! rounds (design §3.4), composition and partial application, `provide`.

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{Name, NodeList, SignatureView};

impl Walker<'_> {
	/// `function name = …` (hoisted: runs once, see `Walker::hoisted`). Sets
	/// the symbol's type (`self.a.symbols.symbol_of_decl(n)`).
	pub(super) fn check_function_decl(
		&mut self,
		n: NodeId,
		_name: Name,
		_is_async: bool,
		_generics: NodeList<'_>,
		_annotation: Option<NodeId>,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "function declarations")
	}

	/// A lambda; in a type, a function type.
	pub(super) fn check_lambda(
		&mut self,
		n: NodeId,
		_signature: SignatureView<'_>,
		_body: NodeId,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "lambdas")
	}

	/// The parameters of a function (normally handled by their function).
	pub(super) fn check_parameters(&mut self, n: NodeId, _items: NodeList<'_>) -> TypeId {
		self.unsupported(n, "parameters")
	}

	/// `{ x, y }: Point` as a parameter.
	pub(super) fn check_destructured_parameter(
		&mut self,
		n: NodeId,
		_pattern: NodeId,
		_annotation: NodeId,
	) -> TypeId {
		self.unsupported(n, "destructured parameters")
	}

	/// `f then g`
	pub(super) fn check_compose(&mut self, n: NodeId, _operands: NodeList<'_>) -> TypeId {
		self.unsupported(n, "`then`")
	}

	/// `f with (y = 0)`
	pub(super) fn check_partial_application(
		&mut self,
		n: NodeId,
		_function: NodeId,
		_arguments: NodeList<'_>,
	) -> TypeId {
		self.unsupported(n, "`with`")
	}

	/// `provide Database = value` (M2 requirements).
	pub(super) fn check_provide(&mut self, n: NodeId, _ty: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "`provide`")
	}
}
