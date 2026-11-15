//! Compile-time value parameters (types brief §3.11.1, §3.11.2): `type
//! Matrix(Element, rows: Integer, columns: Integer)`, `function
//! repeatText(times: Integer(min = 0))`. A value parameter is bound to the
//! literal type of a constant (`Matrix(Float, 2 + 2, 4)` binds `rows` to
//! `4`, so equal values give one type); in the body it is a constant of its
//! declared type. A callee with a value parameter always takes its first
//! argument list as the generic arguments.

use std::sync::Arc;

use ethnum::I256;

use crate::check::Walker;
use crate::diagnostics::codes::types::{GN006, GN008};
use crate::ids::{NodeId, ParamId, TypeId};
use crate::symbols::{DeclState, Resolution, SymbolKind};
use crate::syntax::{self, ArithOp, Name, NodeList, View};
use crate::types::{CollKind, TypeKind};

impl Walker<'_> {
	/// Declares a value parameter: the type of its value is required
	/// (`GN001`) and must have an exact equality (`GN006`: not `Float`, not
	/// a function…); its default is a constant.
	pub(super) fn declare_value_param(
		&mut self,
		param: ParamId,
		generic: NodeId,
		target: NodeId,
		name: Name,
		annotation: Option<NodeId>,
		default: Option<NodeId>,
	) {
		// `keys: DeepKeys<Value>` (fields.rs)
		if let Some(annotation) = annotation
			&& self.declare_key_set_param(param, generic, target, name, annotation)
		{
			return;
		}
		let ty = match annotation {
			Some(annotation) => self.check_type_expr(annotation),
			// without a type: `GN001`, reported by the parser
			None => TypeId::UNKNOWN,
		};
		if !self.s.types.is_recovering(ty) && !self.exact_value_type(ty, 0) {
			let message = format!(
				"a compile-time value is compared exactly: a `{}` can't be one (use an integer, a string, a boolean, an enumeration…)",
				self.display(ty)
			);
			let diagnostic = self.diagnostic(&GN006, annotation.unwrap_or(generic), message);
			self.report(diagnostic);
		}
		let default_ty = default.map_or(TypeId::UNSET, |default| {
			self.constant_type(default).unwrap_or(TypeId::UNKNOWN)
		});
		self.s.types.set_param_bounds(param, ty, default_ty);
		let symbol = self.declare(generic, name, SymbolKind::Static);
		let symbol = self.a.symbols.symbol_mut(symbol);
		symbol.ty = ty;
		symbol.state = DeclState::Done;
		self.set_type(target, ty);
	}

	/// Whether values of `ty` have an exact equality (types brief §3.11.1).
	fn exact_value_type(&self, ty: TypeId, depth: u32) -> bool {
		if depth > 8 {
			return false;
		}
		match self.s.types.kind(ty) {
			TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..)
			| TypeKind::Fields(_) => true,
			TypeKind::Union(list) | TypeKind::Tuple(list) => self
				.s
				.types
				.list_items(list)
				.iter()
				.all(|&item| self.exact_value_type(item, depth + 1)),
			TypeKind::Collection(CollKind::Array, item, _) => {
				self.exact_value_type(item, depth + 1)
			}
			TypeKind::Object(shape) => self
				.s
				.types
				.shape_fields(shape)
				.iter()
				.all(|field| self.exact_value_type(field.ty, depth + 1)),
			_ => false,
		}
	}

	/// The literal type of a compile-time constant: a literal, a negation, an
	/// integer `+`, `-`, `*` of constants, a value parameter or `static`
	/// with a literal type. `None` for anything else.
	pub(super) fn constant_type(&mut self, n: NodeId) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::String(text) => Some(self.s.types.string_literal(text)),
			View::Boolean(value) => Some(self.s.types.bool_literal(value)),
			View::Group(inner) => self.constant_type(inner),
			View::Integer(_) | View::Negate(_) | View::Arithmetic { .. } => {
				let value = self.constant_int(n)?;
				Some(self.s.types.int_literal(value))
			}
			View::Name(name) => {
				let sym = self.s.types.syms.intern(tree.name(name));
				let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
				let data = self.a.symbols.symbol(symbol);
				let (kind, ty) = (data.kind, data.ty);
				if kind != SymbolKind::Static || !self.s.types.is_literal(ty) {
					return None;
				}
				self.set_resolution(n, Resolution::Local(symbol));
				Some(ty)
			}
			_ => None,
		}
	}

	/// The value of a constant integer expression.
	fn constant_int(&self, n: NodeId) -> Option<I256> {
		match self.view(n) {
			View::Integer(value) => Some(value),
			View::Negate(inner) => self.constant_int(inner).map(|value| -value),
			View::Group(inner) => self.constant_int(inner),
			View::Arithmetic { op, operands } => {
				let mut values = operands.iter().map(|operand| self.constant_int(operand));
				let first = values.next()??;
				values.try_fold(first, |acc, value| {
					let value = value?;
					match op {
						ArithOp::Add => acc.checked_add(value),
						ArithOp::Subtract => acc.checked_sub(value),
						ArithOp::Multiply => acc.checked_mul(value),
						_ => None,
					}
				})
			}
			_ => None,
		}
	}

	/// Whether a callee names a function declared with a value parameter
	/// (its first argument list is then always the generic one).
	pub(super) fn has_value_params(&mut self, callee: NodeId) -> bool {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, callee) else {
			return false;
		};
		let sym = self.s.types.syms.intern(tree.name(name));
		let Some(symbol) = self.a.symbols.lookup(self.body.scope, sym) else {
			return false;
		};
		let decl = self.a.symbols.symbol(symbol).decl_node;
		let View::Function { generics, .. } = syntax::view(&tree, decl) else {
			return false;
		};
		generics.iter().any(|generic| {
			let target = match syntax::view(&tree, generic) {
				View::Assignment { target, .. } => target,
				_ => generic,
			};
			matches!(syntax::view(&tree, target), View::Name(name)
				if tree.name(name).chars().next().is_some_and(char::is_lowercase))
		})
	}

	/// The generic arguments of an application: a constant for a value
	/// parameter (`GN008` for a type), a type otherwise.
	pub(super) fn generic_arguments(
		&mut self,
		params: &[ParamId],
		arguments: NodeList<'_>,
	) -> Vec<TypeId> {
		let mut types = Vec::with_capacity(arguments.len());
		for (index, argument) in arguments.iter().enumerate() {
			let value = params
				.get(index)
				.is_some_and(|&param| self.s.types.param(param).value);
			if !value {
				types.push(self.check_type_expr(argument));
				continue;
			}
			// a key set (fields.rs)
			if let Some(&param) = params.get(index)
				&& let Some(ty) = self.key_set_argument(param, argument, params, &types)
			{
				types.push(ty);
				continue;
			}
			self.set_flag(argument, crate::analysis::NodeFlags::TYPE_EXPR);
			let ty = match self.constant_type(argument) {
				Some(ty) => ty,
				None => {
					self.memo(argument, |w| {
						let diagnostic = w.diagnostic(
							&GN008,
							argument,
							"a compile-time value is expected here: a literal or a constant expression",
						);
						w.report(diagnostic);
						TypeId::UNKNOWN
					});
					TypeId::UNKNOWN
				}
			};
			self.memo(argument, |_| ty);
			types.push(ty);
		}
		types
	}
}
