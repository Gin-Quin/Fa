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
use crate::diagnostics::codes::types::{FN028, TY003, TY007, TY008};
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
				return self.symbol_type(symbol, n);
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
				&E_DECL_9,
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
			View::Name(_) => self.check_type_expr(callee),
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
