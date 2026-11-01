//! Literals, names, access to built-in namespaces, groups, arrays, tuples,
//! indexing, ranges, string templates.

use std::sync::Arc;

use ethnum::I256;

use crate::builtins;
use crate::check::Walker;
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::values::E_DECL_1;
use crate::ids::{BuiltinId, NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, Name, NodeList, TemplateParts, View};

impl Walker<'_> {
	/// An integer literal: its literal type, adapted to the expected type when
	/// it fits (values brief §2.2); a sink reports when it doesn't.
	pub(super) fn check_integer(
		&mut self,
		n: NodeId,
		value: I256,
		expected: Option<TypeId>,
	) -> TypeId {
		self.memo(n, |w| {
			let literal = w.s.types.int_literal(value);
			expected
				.and_then(|expected| w.s.types.adapt_literal(literal, expected))
				.unwrap_or(literal)
		})
	}

	/// `true` / `false`.
	pub(super) fn check_boolean(&mut self, n: NodeId, value: bool) -> TypeId {
		self.memo(n, |w| w.s.types.bool_literal(value))
	}

	/// `none`.
	pub(super) fn check_none(&mut self, n: NodeId) -> TypeId {
		self.memo(n, |_| TypeId::NONE)
	}

	/// A string literal (E3).
	pub(super) fn check_string(
		&mut self,
		n: NodeId,
		_text: &str,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "strings")
	}

	/// A string template (E3).
	pub(super) fn check_template(&mut self, n: NodeId, _parts: TemplateParts<'_>) -> TypeId {
		self.unsupported(n, "string templates")
	}

	/// A name in value position: a local, then the prelude (`E-DECL-1` otherwise).
	pub(super) fn check_name(&mut self, n: NodeId, name: Name) -> TypeId {
		self.memo(n, |w| {
			if name.is_empty() {
				return TypeId::UNKNOWN;
			}
			let tree = Arc::clone(&w.tree);
			let text = tree.name(name);
			let sym = w.s.types.syms.intern(text);
			if let Some(symbol) = w.a.symbols.lookup(w.body.scope, sym) {
				w.set_resolution(n, Resolution::Local(symbol));
				return w.symbol_type(symbol);
			}
			if let Some(builtin) = builtins::lookup(text) {
				w.set_resolution(n, Resolution::Builtin(builtin));
				return match builtins::value_type(&mut w.s.types, builtin) {
					Some(ty) => ty,
					None => {
						let message = format!("`{text}` used as a value");
						w.unsupported_message(n, &message)
					}
				};
			}
			let diagnostic = w.diagnostic(
				&E_DECL_1,
				n,
				format!("`{text}` is not declared: declare it with `let` or `mutable` first"),
			);
			w.report(diagnostic);
			TypeId::UNKNOWN
		})
	}

	/// Reports `AN001` for a construct with no children to walk.
	fn unsupported_message(&mut self, n: NodeId, what: &str) -> TypeId {
		let message = format!("{what} is not supported yet by the analyzer");
		let diagnostic = self.diagnostic(&AN001, n, message);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// `.red` where a `Color` is expected (M1b, enumerations).
	pub(super) fn check_implicit_member(
		&mut self,
		n: NodeId,
		_name: Name,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "implicit members")
	}

	/// `(expression)`: the type of the expression.
	pub(super) fn check_group(
		&mut self,
		_n: NodeId,
		inner: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		self.check(inner, expected)
	}

	/// `()`
	pub(super) fn check_empty_group(&mut self, n: NodeId) -> TypeId {
		self.unsupported(n, "`()`")
	}

	/// `[a, b, c]` (M1b).
	pub(super) fn check_array(
		&mut self,
		n: NodeId,
		_items: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "arrays")
	}

	/// `a, b, c` (M2).
	pub(super) fn check_tuple(
		&mut self,
		n: NodeId,
		_items: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "tuples")
	}

	/// `...value` (in arrays, objects and calls; handled by the parent).
	pub(super) fn check_spread(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "spreads")
	}

	/// `a..b` (M2).
	pub(super) fn check_range(
		&mut self,
		n: NodeId,
		_start: Option<NodeId>,
		_end: Option<NodeId>,
		_inclusive: bool,
	) -> TypeId {
		self.unsupported(n, "ranges")
	}

	/// `a.b.c`: members of built-in namespaces (`console.log`) for now.
	pub(super) fn check_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		match self.builtin_namespace(target) {
			Some(namespace) => self.memo(n, |w| w.check_builtin_members(namespace, members)),
			None => self.unsupported(n, "field access"),
		}
	}

	/// The built-in namespace a name refers to, unless a local hides it.
	fn builtin_namespace(&mut self, target: NodeId) -> Option<BuiltinId> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, target) else {
			return None;
		};
		let text = tree.name(name);
		let sym = self.s.types.syms.intern(text);
		if self.a.symbols.lookup(self.body.scope, sym).is_some() {
			return None;
		}
		let builtin = builtins::lookup(text)?;
		builtins::has_members(builtin).then(|| {
			self.set_resolution(target, Resolution::Builtin(builtin));
			builtin
		})
	}

	/// Resolves `namespace.member.member…` in the prelude. Type part.
	fn check_builtin_members(&mut self, namespace: BuiltinId, members: NodeList<'_>) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let mut current = namespace;
		for member in members.iter() {
			let View::Name(name) = syntax::view(&tree, member) else {
				return TypeId::UNKNOWN;
			};
			let text = tree.name(name);
			let Some(found) = builtins::member(current, text) else {
				let message = format!("`{}.{text}`", builtins::get(current).path);
				return self.unsupported_message(member, &message);
			};
			self.set_resolution(member, Resolution::Builtin(found));
			current = found;
		}
		builtins::value_type(&mut self.s.types, current).unwrap_or(TypeId::UNKNOWN)
	}

	/// `items[index]` (M1b).
	pub(super) fn check_index(&mut self, n: NodeId, _target: NodeId, _index: NodeId) -> TypeId {
		self.unsupported(n, "indexing")
	}
}
