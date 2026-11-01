//! Type expressions: the type an expression node denotes in type position
//! (annotations, `type` declarations, `is`). Its own small dispatch over
//! `View`, separate from value-mode `check`. Each node gets `TYPE_EXPR` and,
//! as its type, the type it denotes.

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::builtins;
use crate::check::Walker;
use crate::diagnostics::codes::types::TY007;
use crate::ids::{NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, Name, View};

impl Walker<'_> {
	/// The type a type expression denotes.
	pub fn check_type_expr(&mut self, n: NodeId) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let ty = match syntax::view(&tree, n) {
			View::Name(name) => self.memo(n, |w| w.type_name(n, name)),
			View::Group(inner) => {
				let ty = self.check_type_expr(inner);
				self.memo(n, |_| ty)
			}
			View::Error => TypeId::UNKNOWN,
			_ => self.unsupported(n, "this type expression"),
		};
		self.set_flag(n, NodeFlags::TYPE_EXPR);
		ty
	}

	/// A type name: a local `type`, then the prelude (`TY007` otherwise).
	fn type_name(&mut self, n: NodeId, name: Name) -> TypeId {
		if name.is_empty() {
			return TypeId::UNKNOWN;
		}
		let tree = Arc::clone(&self.tree);
		let text = tree.name(name);
		let sym = self.s.types.syms.intern(text);
		if let Some(symbol) = self.a.symbols.lookup(self.body.scope, sym) {
			self.set_resolution(n, Resolution::Local(symbol));
			if self.a.symbols.symbol(symbol).kind == SymbolKind::Type {
				return self.symbol_type(symbol);
			}
		} else if let Some(builtin) = builtins::lookup(text) {
			self.set_resolution(n, Resolution::Builtin(builtin));
			if let Some(ty) = builtins::denoted_type(&mut self.s.types, builtin) {
				return ty;
			}
		}
		let diagnostic = self.diagnostic(&TY007, n, format!("`{text}` is not a type"));
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// A construct that only exists in types (`A | B`, `T?`, `Error`, …) in
	/// value position.
	pub(super) fn check_type_in_value_position(&mut self, n: NodeId) -> TypeId {
		self.unsupported(n, "type expressions in value position")
	}
}
