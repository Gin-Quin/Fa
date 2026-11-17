//! Literals, names, access to built-in namespaces, groups, arrays, tuples,
//! indexing, ranges, string templates.

use std::sync::Arc;

use ethnum::I256;

use crate::builtins::{self, BuiltinKind};
use crate::check::Walker;
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::types::{FN043, TY008};
use crate::diagnostics::codes::values::E_DECL_1;
use crate::ids::{BuiltinId, NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
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

	/// A string literal keeps its literal type; bindings widen it to `String`.
	pub(super) fn check_string(
		&mut self,
		n: NodeId,
		text: &str,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.memo(n, |w| w.s.types.string_literal(text))
	}

	/// A string template `"Hello {name}"`: a `String` (strings.rs).
	pub(super) fn check_template(&mut self, n: NodeId, parts: TemplateParts<'_>) -> TypeId {
		self.template_type(n, parts)
	}

	/// A name in value position: a local, then the prelude (`E-DECL-1` otherwise).
	pub(super) fn check_name(&mut self, n: NodeId, name: Name) -> TypeId {
		let ty = self.memo(n, |w| {
			if name.is_empty() {
				return TypeId::UNKNOWN;
			}
			let tree = Arc::clone(&w.tree);
			let text = tree.name(name);
			// `@add` in `@add(2, 4)` (comptime.rs)
			if let Some(ty) = w.check_at_name(n, text) {
				return ty;
			}
			let sym = w.s.types.syms.intern(text);
			if let Some(symbol) = w.a.symbols.lookup(w.body.scope, sym) {
				w.set_resolution(n, Resolution::Local(symbol));
				// a key-set parameter: the iterable of an `@for` (fields.rs)
				w.check_key_set_read(n, symbol);
				if w.a.symbols.symbol(symbol).kind == SymbolKind::Type {
					return w.type_as_value(n, text);
				}
				// a folder or a namespace (modules.rs)
				if w.is_namespace(symbol) {
					return w.namespace_as_value(n, text);
				}
				w.check_unqualified_type_function(n, symbol, text);
				// the narrowed type of the local here (narrowing.rs)
				let declared = w.symbol_type(symbol, n);
				return w.narrowed_read(n, symbol, declared);
			}
			if let Some(builtin) = w.lookup_builtin(text) {
				w.set_resolution(n, Resolution::Builtin(builtin));
				if builtins::denoted_type(&mut w.s.types, builtin).is_some() {
					return w.type_as_value(n, text);
				}
				return match builtins::value_type(&mut w.s.types, builtin) {
					Some(ty) => ty,
					None => {
						let message = format!("`{text}` used as a value");
						w.unsupported_message(n, &message)
					}
				};
			}
			// `target`, `configuration` (comptime.rs)
			if let Some(ty) = w.check_provided_static(n, text) {
				return ty;
			}
			if super::type_expr::is_later_prelude_name(text) || text == "Array" {
				return w.unsupported_message(n, &format!("`{text}`"));
			}
			// `now` without `use { now } = std.time` (std_library.rs)
			if w.report_std_name(n, text) {
				return TypeId::UNKNOWN;
			}
			let diagnostic = w.diagnostic(
				&E_DECL_1,
				n,
				if text == "Character" { "`Character` is no longer a type: use `String` for text, including a single code unit or code point".to_string() } else if w.object_hint_names.contains(&n) { format!("`{text}` is not declared: declare it with `let` or `mutable` first; or, to return an object, write its type before the braces: `=> TypeName {{ … }}`") } else { format!("`{text}` is not declared: declare it with `let` or `mutable` first") },
			);
			w.report(diagnostic);
			TypeId::UNKNOWN
		});
		// the read of a `static` is its value (comptime.rs)
		self.fold_static_read(n);
		// a narrowed read stays valid at every visit (narrowing.rs)
		self.revalidate_read(n);
		// a local reference must be valid here (references.rs)
		self.use_reference(n);
		// a stopped subscription is gone (reactivity.rs)
		self.check_consumed(n);
		ty
	}

	/// `FN043`: inside a type, its functions are named through the type
	/// (`Self.make()`, `self.describe()`), never alone. Type part.
	fn check_unqualified_type_function(
		&mut self,
		n: NodeId,
		symbol: crate::ids::SymbolId,
		text: &str,
	) {
		let symbol = self.a.symbols.symbol(symbol);
		if symbol.kind != SymbolKind::Function || !self.type_scopes.contains_key(&symbol.scope) {
			return;
		}
		let diagnostic = self.diagnostic(
			&FN043,
			n,
			format!(
				"`{text}` is a function of the type: write `Self.{text}(…)`, or `self.{text}(…)` for a method"
			),
		);
		self.report(diagnostic);
	}

	/// `TY008`: a type name used as a value (`let x = Integer`). Type part.
	fn type_as_value(&mut self, n: NodeId, text: &str) -> TypeId {
		let diagnostic = self.diagnostic(
			&TY008,
			n,
			format!("`{text}` is a type, not a value: to convert a value, write `{text}(value)`"),
		);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// Reports `AN001` for a construct with no children to walk.
	pub(super) fn unsupported_message(&mut self, n: NodeId, what: &str) -> TypeId {
		let message = format!("{what} is not supported yet by the analyzer");
		let diagnostic = self.diagnostic(&AN001, n, message);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// `.red` where a `Color` is expected (M1b, enumerations).
	pub(super) fn check_implicit_member(
		&mut self,
		n: NodeId,
		name: Name,
		expected: Option<TypeId>,
	) -> TypeId {
		self.check_implicit_member_value(n, name, expected)
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

	/// `copy value` produces a value, never a place. Check the projected
	/// result type: an ordinary data projection need not copy a resource.
	pub(super) fn check_copy(&mut self, inner: NodeId, expected: Option<TypeId>) -> TypeId {
		let ty = self.check(inner, expected);
		let target = if let Some(expected) = expected
			&& self
				.s
				.types
				.assignable(ty, expected, crate::types::relate::Mode::Copy)
				.is_ok()
		{
			self.sink(inner, ty, expected);
			expected
		} else {
			ty
		};
		if self.s.types.owns_unique(target) {
			let diagnostic = self.diagnostic(&crate::diagnostics::codes::advanced::MEM003, inner,
				"this value owns a Task or a resource and cannot be copied: borrow it, move the owner, or copy a data projection without its unique fields");
			self.report(diagnostic);
		}
		target
	}

	/// `()`: not a value (`E-TUP-1`, tuples.rs).
	pub(super) fn check_empty_group(&mut self, n: NodeId) -> TypeId {
		self.memo(n, |w| w.unit_tuple(n))
	}

	/// `[a, b, c]` (M1b).
	pub(super) fn check_array(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		self.check_array_literal(n, items, expected)
	}

	/// `(a, b, c)` (tuples.rs).
	pub(super) fn check_tuple(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		self.check_tuple_value(n, items, expected)
	}

	/// `...value` (in arrays, objects and calls; handled by the parent).
	pub(super) fn check_spread(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "spreads")
	}

	/// `a..b` (M2).
	pub(super) fn check_range(
		&mut self,
		n: NodeId,
		start: Option<NodeId>,
		end: Option<NodeId>,
		_inclusive: bool,
	) -> TypeId {
		// the bounds are integers that fit `Integer` (values brief D-41; numbers.rs)
		self.check_range_bounds(n, start, end)
	}

	/// `a.b.c`: members of built-in namespaces (`console.log`) and of string
	/// values (strings.rs) for now.
	pub(super) fn check_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		// `library.math.add`, `counter.count`: through namespaces (modules.rs)
		if let Some(ty) = self.check_module_access(n, target, members) {
			return ty;
		}
		let ty = match self.builtin_namespace(target) {
			Some(namespace) => self.memo(n, |w| w.check_builtin_members(namespace, members)),
			None => self.check_member_access(n, target, members),
		};
		self.note_code_point_count(n, target, members);
		// `settings.port` of a static value: its value (comptime.rs)
		self.fold_static_fields(n, target, members);
		ty
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
		let builtin = self.lookup_builtin(text)?;
		builtins::has_members(builtin).then(|| {
			self.set_resolution(target, Resolution::Builtin(builtin));
			builtin
		})
	}

	/// Resolves `namespace.member.member…` in the prelude. Type part.
	fn check_builtin_members(&mut self, namespace: BuiltinId, members: NodeList<'_>) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let mut current = namespace;
		for (index, member) in members.iter().enumerate() {
			// the members of a built-in value (`Float.nan.isNaN`): its type's
			if index > 0 && matches!(builtins::get(current).kind, BuiltinKind::Value(_)) {
				let mut ty =
					builtins::value_type(&mut self.s.types, current).unwrap_or(TypeId::UNKNOWN);
				for member in members.iter().skip(index) {
					ty = match self.member_of(member, ty) {
						Some((ty, _)) => ty,
						None => return TypeId::UNKNOWN,
					};
				}
				return ty;
			}
			let View::Name(name) = syntax::view(&tree, member) else {
				return TypeId::UNKNOWN;
			};
			let text = tree.name(name);
			let Some(found) = builtins::member(current, text) else {
				if builtins::get(current).path == "File" && text == "readBytes" {
					let diagnostic = self.diagnostic(
						&crate::diagnostics::codes::analyzer::AN001,
						member,
						"`File.readBytes` was replaced by `File.read(path)`, which returns `Bytes`; use `File.readText(path)` to read UTF-8 text",
					);
					self.report(diagnostic);
					return TypeId::UNKNOWN;
				}
				let message = format!("`{}.{text}`", builtins::get(current).path);
				return self.unsupported_message(member, &message);
			};
			self.set_resolution(member, Resolution::Builtin(found));
			current = found;
		}
		builtins::value_type(&mut self.s.types, current).unwrap_or(TypeId::UNKNOWN)
	}

	/// `items[index]` (M1b).
	pub(super) fn check_index(&mut self, n: NodeId, target: NodeId, index: NodeId) -> TypeId {
		// `value[key]` in an `@for` over a key set (fields.rs)
		if let Some(ty) = self.check_key_read(n, target, index) {
			return ty;
		}
		// `chain[1..]`: some steps of a composition (compose.rs)
		if let Some(ty) = self.check_composition_slice(n, target, index) {
			return ty;
		}
		let target_ty = self.check(target, None);
		if let Some(key) = self.map_key(target_ty) {
			// `map[key]`: `KeyNotFound` unless the key is proven present (D32)
			let key_ty = self.check(index, Some(key));
			self.sink(index, key_ty, key);
			let ty = self.memo(n, |w| w.index_type(n, target, target_ty, index, key_ty));
			self.check_key_presence(n, target, index);
			return ty;
		}
		let index_ty = self.check(index, Some(TypeId::INTEGER));
		let ty = self.memo(n, |w| w.index_type(n, target, target_ty, index, index_ty));
		self.check_index_bounds(n, target, index);
		if crate::builtins::strings::is_string_like(&self.s.types, target_ty)
			&& self.type_of(index) != TypeId::RANGE
		{
			self.set_flag(n, crate::analysis::NodeFlags::STRING_CODE_UNIT);
		}
		ty
	}
}
