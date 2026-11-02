//! Type expressions: the type an expression node denotes in type position
//! (annotations, `type` declarations, `is`). Its own small dispatch over
//! `View`, separate from value-mode `check`. Each node gets `TYPE_EXPR` and,
//! as its type, the type it denotes.

use std::sync::Arc;

use ethnum::I256;

use crate::analysis::NodeFlags;
use crate::builtins;
use crate::check::Walker;
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::types::{FN028, TY003, TY007, TY008, TY040};
use crate::diagnostics::codes::values::{E_DECL_9, E_NUM_13};
use crate::ids::{NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, Name, NodeList, SignatureView, View};
use crate::types::{FnSig, IntType, ParamSig, TypeKind};

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
			View::Optional(inner) => {
				let inner_ty = self.check_type_expr(inner);
				self.memo(n, |w| w.optional_type(inner, inner_ty))
			}
			View::Call { callee, arguments } => self.type_application(n, callee, arguments),
			View::Lambda { signature, body } => self.function_type_expr(n, signature, body),
			// literal types (types brief §3.1.2)
			View::Integer(value) => self.memo(n, |w| w.s.types.int_literal(value)),
			View::String(text) => self.memo(n, |w| w.s.types.string_literal(text)),
			View::Boolean(value) => self.memo(n, |w| w.s.types.bool_literal(value)),
			View::Float(value) => self.memo(n, |w| {
				w.s.types.float_literal(crate::types::FloatBits::F64, value)
			}),
			View::None => self.memo(n, |_| TypeId::NONE),
			View::ErrorType => self.memo(n, |_| TypeId::ERROR),
			View::Union { operands } => self.union_type_expr(n, operands),
			View::Object { items } => self.memo(n, |w| w.anonymous_object_type(items)),
			View::Access { target, members } => self.type_path(n, target, members),
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
			match self.a.symbols.symbol(symbol).kind {
				SymbolKind::Type => {
					let ty = self.symbol_type(symbol, n);
					if self.missing_type_arguments(n, symbol) {
						return TypeId::UNKNOWN;
					}
					return ty;
				}
				SymbolKind::TypeParameter => return self.symbol_type(symbol, n),
				// a `static` value is its literal type (M3: `static` is not analyzed yet)
				SymbolKind::Static => {
					let ty = self.symbol_type(symbol, n);
					return if self.s.types.is_literal(ty) {
						ty
					} else {
						TypeId::UNKNOWN
					};
				}
				_ => {}
			}
			let diagnostic = self.diagnostic(
				&TY008,
				n,
				format!("`{text}` is a value, not a type: write its type, or `Type({text})`"),
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		if let Some(builtin) = builtins::lookup(text) {
			self.set_resolution(n, Resolution::Builtin(builtin));
			if let Some(ty) = builtins::denoted_type(&mut self.s.types, builtin) {
				return ty;
			}
		}
		if text == "Self"
			&& let Some((ty, _)) = self.self_type(self.body.scope)
		{
			return ty;
		}
		if text == "None" {
			return TypeId::NONE;
		}
		if text == "Array" {
			let diagnostic = self.diagnostic(
				&TY003,
				n,
				"`Array` needs the type of its items, ex: `Array(Integer)`",
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		if is_later_prelude_name(text) {
			let message = format!("`{text}` is not supported yet by the analyzer");
			let diagnostic = self.diagnostic(&AN001, n, message);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		let diagnostic = self.diagnostic(&TY007, n, format!("`{text}` is not a type"));
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// `Array(Item)`: the built-in collection (types brief §5.1).
	fn array_type(&mut self, n: NodeId, callee: NodeId, arguments: NodeList<'_>) -> TypeId {
		let item = match (arguments.len(), arguments.first()) {
			(1, Some(item)) => self.check_type_expr(item),
			_ => {
				for argument in arguments.iter() {
					self.check_type_expr(argument);
				}
				TypeId::UNSET
			}
		};
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let ty = self.memo(n, |w| {
			if item.is_unset() {
				let diagnostic = w.diagnostic(
					&TY003,
					n,
					"`Array` takes the type of its items, ex: `Array(Integer)`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			if item == TypeId::VOID {
				let diagnostic = w.diagnostic(
					&E_DECL_9,
					n,
					"`Void` is not a value type: an array can't hold it",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			w.s.types.array(item)
		});
		self.memo(callee, |_| ty)
	}

	/// `T?`. `Void?` is `E-DECL-9` (`Void` is not a value).
	fn optional_type(&mut self, inner: NodeId, inner_ty: TypeId) -> TypeId {
		if inner_ty == TypeId::VOID {
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::values::E_OPT_9,
				inner,
				"`Void` is not a value type: it can't be optional",
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		self.s.types.optional(inner_ty)
	}

	/// `Name(arguments)` in a type: a ranged integer type,
	/// `Integer(min = 0, max = 10)`, `Int8(min = 0)` (values brief §6.2). Generic
	/// applications (`Array(Integer)`) come with collections and generics.
	fn type_application(&mut self, n: NodeId, callee: NodeId, arguments: NodeList<'_>) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let base = match syntax::view(&tree, callee) {
			View::Name(name) if tree.name(name) == "Array" && !self.is_declared(name) => {
				return self.array_type(n, callee, arguments);
			}
			View::Name(name)
				if tree.name(name) == "Optional"
					&& !self.is_declared(name)
					&& arguments.len() == 1 =>
			{
				// `Optional(T)` is `T?`
				self.set_flag(callee, NodeFlags::TYPE_EXPR);
				let inner = arguments.first().expect("one argument");
				let inner_ty = self.check_type_expr(inner);
				return self.memo(n, |w| w.optional_type(inner, inner_ty));
			}
			View::Name(name) => {
				// a generic type of this file: `Point(Float)`
				let sym = self.s.types.syms.intern(tree.name(name));
				if let Some(symbol) = self.a.symbols.lookup(self.body.scope, sym)
					&& self.a.symbols.symbol(symbol).kind == SymbolKind::Type
					&& {
						// analyzed first: its parameters are known then
						self.symbol_type(symbol, callee);
						self.type_generics(symbol).is_some()
					} {
					self.set_resolution(callee, crate::symbols::Resolution::Local(symbol));
					self.set_flag(callee, NodeFlags::TYPE_EXPR);
					return self.generic_type_application(n, symbol, arguments);
				}
				self.check_type_expr(callee)
			}
			_ => return self.unsupported(n, "this type expression"),
		};
		if self.s.types.is_recovering(base) {
			return self.memo(n, |_| TypeId::UNKNOWN);
		}
		match self.s.types.as_int(base).copied() {
			Some(int) if int.is_full() => self.memo(n, |w| w.ranged_integer(n, int, arguments)),
			_ => self.unsupported(n, "this generic type"),
		}
	}

	/// `Integer(min = …, max = …)`: the bounds must be constant integers within
	/// the storage's range, with `min <= max` (`E-NUM-13` otherwise). Type part.
	fn ranged_integer(&mut self, n: NodeId, full: IntType, arguments: NodeList<'_>) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let storage = full.storage.name();
		let (mut min, mut max) = (full.min, full.max);
		let mut valid = true;
		for argument in arguments.iter() {
			self.set_flag(argument, NodeFlags::TYPE_EXPR);
			let bound = match syntax::view(&tree, argument) {
				View::Assignment {
					target,
					annotation: None,
					value: Some(value),
				} => match syntax::view(&tree, target) {
					View::Name(name) if matches!(tree.name(name), "min" | "max") => {
						let constant = self.constant_integer(value);
						if constant.is_none() {
							let diagnostic = self.diagnostic(
								&E_NUM_13,
								value,
								"a bound of an integer type is a constant integer, ex: `Integer(min = 0, max = 10)`",
							);
							self.report(diagnostic);
						}
						constant.map(|value| (tree.name(name) == "min", value))
					}
					_ => None,
				},
				_ => None,
			};
			let Some((is_min, value)) = bound else {
				valid = false;
				let diagnostic = self.diagnostic(
					&E_NUM_13,
					argument,
					format!(
						"an integer type takes the named bounds `min` and `max`, ex: `{storage}(min = 0, max = 10)`"
					),
				);
				self.report(diagnostic);
				continue;
			};
			if !full.interval().contains(value) {
				valid = false;
				let diagnostic = self.diagnostic(
					&E_NUM_13,
					argument,
					format!(
						"{value} is out of the range of `{storage}` ({}..={})",
						full.min, full.max
					),
				);
				self.report(diagnostic);
			} else if is_min {
				min = value;
			} else {
				max = value;
			}
		}
		if valid && min > max {
			valid = false;
			let diagnostic = self.diagnostic(
				&E_NUM_13,
				n,
				format!("`min` ({min}) is greater than `max` ({max}): this type has no value"),
			);
			self.report(diagnostic);
		}
		if !valid {
			return TypeId::UNKNOWN;
		}
		self.s.types.int_range(min, max, full.storage)
	}

	/// The value of a constant integer expression: a literal, possibly negated
	/// or in parentheses.
	pub(super) fn constant_integer(&self, n: NodeId) -> Option<I256> {
		match self.view(n) {
			View::Integer(value) => Some(value),
			View::Negate(inner) => self.constant_integer(inner).map(|value| -value),
			View::Group(inner) => self.constant_integer(inner),
			_ => None,
		}
	}

	/// A function type `(a: Integer, b: Integer) => Integer`: the body is the
	/// return type. Parameter names are not part of the type. `throws`,
	/// `requires` and effects in function types come with effects.
	fn function_type_expr(
		&mut self,
		n: NodeId,
		signature: SignatureView<'_>,
		body: NodeId,
	) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let mut params = Vec::with_capacity(signature.parameters.len());
		for (index, parameter) in signature.parameters.iter().enumerate() {
			self.set_flag(parameter, NodeFlags::TYPE_EXPR);
			let (variadic, ty) = match syntax::view(&tree, parameter) {
				View::Assignment {
					target,
					annotation: Some(annotation),
					..
				} => (
					matches!(syntax::view(&tree, target), View::Spread(_)),
					self.check_type_expr(annotation),
				),
				// `(Integer) => Integer`: an unnamed parameter is its type
				_ => (false, self.check_type_expr(parameter)),
			};
			if variadic && index + 1 != signature.parameters.len() {
				self.memo(parameter, |w| {
					let diagnostic =
						w.diagnostic(&FN028, parameter, "a variadic parameter is the last one");
					w.report(diagnostic);
					TypeId::UNKNOWN
				});
			}
			let ty = if variadic { self.s.types.array(ty) } else { ty };
			params.push(ParamSig { ty, variadic });
		}
		let ret = self.check_type_expr(body);
		let unsupported = signature.throws.is_some()
			|| !signature.requires.is_empty()
			|| !signature.mutates.is_empty()
			|| !signature.invalidates.is_empty();
		if unsupported {
			return self.unsupported(n, "effects in function types");
		}
		self.memo(n, |w| {
			let base = w.s.types.simple_sig(&[], ret, TypeId::NEVER);
			let params = w.s.types.param_list(&params);
			w.s.types.function(FnSig {
				params,
				is_async: signature.is_async,
				..base
			})
		})
	}

	/// `A | B | …`: the normalized union; `TY040` when two members can't be
	/// told apart at runtime (types brief §3.4.2).
	fn union_type_expr(&mut self, n: NodeId, operands: NodeList<'_>) -> TypeId {
		let members: Vec<TypeId> = operands
			.iter()
			.map(|operand| self.check_type_expr(operand))
			.collect();
		self.memo(n, |w| {
			if members
				.iter()
				.any(|&member| w.s.types.is_recovering(member))
			{
				return TypeId::UNKNOWN;
			}
			if members.contains(&TypeId::VOID) {
				let diagnostic = w.diagnostic(
					&E_DECL_9,
					n,
					"`Void` is not a value type: it can't be a member of a union",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let ty = w.s.types.union(&members);
			// a constraint is "one of these types", not a runtime union
			if w.constraint_depth == 0 {
				w.check_distinguishable(n, ty);
			}
			ty
		})
	}

	/// `TY040` when two members of a union written by the user, or joined
	/// without an expected type, can't be told apart at runtime. Type part.
	pub fn check_distinguishable(&mut self, n: NodeId, ty: TypeId) {
		if let Some((a, b)) = self.s.types.indistinguishable_pair(ty) {
			let message = format!(
				"`{}` and `{}` can't be told apart at runtime: declare a named union, ex: `Union {{ A = …, B = … }}`",
				self.display(a),
				self.display(b)
			);
			let diagnostic = self.diagnostic(&TY040, n, message);
			self.report(diagnostic);
		}
	}

	/// `{ name: String, greeting: String = "Hello" }` in a type. Type part.
	fn anonymous_object_type(&mut self, items: NodeList<'_>) -> TypeId {
		let parts = self.object_type_parts(items, false);
		if parts.incomplete {
			return TypeId::UNKNOWN;
		}
		let entries: Vec<crate::types::FieldEntry> = parts
			.fields
			.iter()
			.map(|field| crate::types::FieldEntry {
				name: field.name,
				ty: field.ty,
				flags: field.flags,
			})
			.collect();
		self.s.types.object(&entries, None)
	}

	/// `Shape.Circle`, `Color.red`: a member of a named union or of an
	/// enumeration, as a type.
	fn type_path(&mut self, n: NodeId, target: NodeId, members: NodeList<'_>) -> TypeId {
		let base = match self.view(target) {
			View::Name(_) | View::Call { .. } => match self.type_of_target(target) {
				Some(ty) => ty,
				None => self.check_type_expr(target),
			},
			_ => return self.unsupported(n, "this type expression"),
		};
		self.memo(n, |w| {
			let mut current = base;
			for member in members.iter() {
				if w.s.types.is_recovering(current) {
					return TypeId::UNKNOWN;
				}
				let View::Name(name) = w.view(member) else {
					return TypeId::UNKNOWN;
				};
				let text = w.tree.name(name).to_string();
				match w.nominal_member(current, &text) {
					Some(ty) => {
						w.set_type(member, ty);
						w.set_flag(member, NodeFlags::TYPE_EXPR);
						current = ty;
					}
					None => {
						let message = format!("`{}` has no member `{text}`", w.display(current));
						let diagnostic = w.diagnostic(&TY007, member, message);
						w.report(diagnostic);
						return TypeId::UNKNOWN;
					}
				}
			}
			current
		})
	}

	/// The member `name` of a named union, `Errors` type or enumeration, as a type.
	pub(super) fn nominal_member(&mut self, ty: TypeId, name: &str) -> Option<TypeId> {
		let (nominal, is_enum) = match self.s.types.kind(ty) {
			TypeKind::NamedUnion(nominal, _) => (nominal, false),
			TypeKind::Enum(nominal, _) => (nominal, true),
			_ => return None,
		};
		let index = self.s.types.member_index(nominal, name)?;
		Some(if is_enum {
			let mask = self.s.types.single_mask(index);
			self.s.types.enumeration(nominal, mask)
		} else {
			self.s.types.member(nominal, index)
		})
	}

	/// The type of a function signature, when it is a function type.
	pub(super) fn signature_of(&self, ty: TypeId) -> Option<FnSig> {
		match self.s.types.kind(ty) {
			TypeKind::Function(sig) => Some(self.s.types.sig(sig)),
			_ => None,
		}
	}

	/// A construct that only exists in types (`A | B`, `T?`, `Error`, …) in
	/// value position.
	pub(super) fn check_type_in_value_position(&mut self, n: NodeId) -> TypeId {
		self.unsupported(n, "type expressions in value position")
	}

	/// Whether a name is declared in the current scope or an enclosing one.
	pub(super) fn is_declared(&mut self, name: Name) -> bool {
		let sym = self.s.types.syms.intern(self.tree.name(name));
		self.a.symbols.lookup(self.body.scope, sym).is_some()
	}
}

/// The names of the prelude that the analyzer doesn't know yet (collections,
/// type functions, built-in errors): `AN001` rather than `TY007`/`E-DECL-1`.
pub(super) fn is_later_prelude_name(text: &str) -> bool {
	text.starts_with('@')
		|| matches!(
			text,
			"GrowingArray"
				| "Set" | "GrowingSet"
				| "Map" | "GrowingMap"
				| "Pool" | "GrowingPool"
				| "Columnar" | "GrowingColumnar"
				| "OrderedColumnar"
				| "Tuple" | "Type"
				| "Fields" | "Partial"
				| "Required" | "Pick"
				| "Omit" | "Parameters"
				| "ReturnType"
				| "Uppercase"
				| "Lowercase"
				| "Capitalize"
				| "Iterator" | "Task"
				| "Schema" | "TypeFromSchema"
				| "IndexError"
				| "ParseError"
				| "ArithmeticError"
				| "DivisionByZero"
				| "KeyError" | "Math"
				| "arguments"
				| "target" | "configuration"
		)
}
