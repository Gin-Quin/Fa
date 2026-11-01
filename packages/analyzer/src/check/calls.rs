//! Calls: argument binding, generic inference, effect application (design §3.5).

use std::sync::Arc;

use crate::check::Walker;
use crate::diagnostics::codes::types::{FN024, FN025};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, ParamSig, TypeKind};

impl Walker<'_> {
	/// `callee(arguments)`: positional arguments against a function type.
	/// Named arguments, generics and effects come with the function work.
	pub(super) fn check_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		let tree = Arc::clone(&self.tree);
		if arguments
			.iter()
			.any(|argument| matches!(syntax::view(&tree, argument), View::Assignment { .. }))
		{
			return self.unsupported(n, "named arguments");
		}
		let callee_ty = self.check(callee, None);
		let sig = match self.s.types.kind(callee_ty) {
			TypeKind::Function(sig) => self.s.types.sig(sig),
			TypeKind::Unknown => {
				for argument in arguments.iter() {
					self.check(argument, None);
				}
				return TypeId::UNKNOWN;
			}
			_ => return self.unsupported(n, "calling this value"),
		};
		let params: Vec<ParamSig> = self.s.types.params(sig.params).to_vec();
		let variadic = params.last().filter(|param| param.variadic).copied();
		for (index, argument) in arguments.iter().enumerate() {
			let param = params.get(index).copied().or(variadic);
			let expected = param.map(|param| self.parameter_type(param));
			let expected = expected.filter(|&ty| ty != TypeId::UNKNOWN);
			let actual = self.check(argument, expected);
			if let Some(expected) = expected {
				self.sink(argument, actual, expected);
			}
		}
		self.memo(n, |w| {
			let required = params.len() - usize::from(variadic.is_some());
			if arguments.len() < required {
				let diagnostic = w.diagnostic(
					&FN024,
					n,
					format!("{} arguments are missing", required - arguments.len()),
				);
				w.report(diagnostic);
			} else if variadic.is_none() && arguments.len() > params.len() {
				let first_extra = arguments.get(params.len()).unwrap_or(n);
				let diagnostic = w.diagnostic(
					&FN025,
					first_extra,
					format!("this function takes {} arguments", params.len()),
				);
				w.report(diagnostic);
			}
			sig.ret
		})
	}

	/// The type of one argument for a parameter: the item type of a variadic one.
	fn parameter_type(&self, param: ParamSig) -> TypeId {
		match (param.variadic, self.s.types.kind(param.ty)) {
			(true, TypeKind::Collection(CollKind::Array, item, _)) => item,
			_ => param.ty,
		}
	}
}
