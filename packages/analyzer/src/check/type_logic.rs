//! Type functions with logic (`types/type-functions`): `if` on a
//! compile-time value, `when` on a type, `do` blocks.
//!
//! ```fa
//! type Storage(size: Integer(min = 0)) = if size <= 255 { UInt8 } else { UInt64 }
//! type Optionalize(Value) = when Value is { None => None, else => Value? }
//! ```
//!
//! The declaration is checked once, generically: the conditions as values
//! (a value parameter is a constant of its declared type), the subjects,
//! patterns and results as types with rigid parameters. Each application
//! then *selects* a result: the condition evaluated on the interpreter with
//! the value parameters bound (`static_values`), the subject substituted and
//! tested against each pattern (`is` on types: the whole type satisfies the
//! pattern), and the selected result substituted. More than 256 nested
//! applications is `META004` (D26).

use std::sync::Arc;

use fa_interpreter::Value;

use crate::check::Walker;
use crate::diagnostics::codes::advanced::{META004, META006};
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::SymbolKind;
use crate::syntax::{self, View};
use crate::types::{Bindings, TypeKind};

/// The nesting bound of type-function applications (advanced brief §12.3).
const MAX_NESTING: u32 = 256;

impl Walker<'_> {
	/// Whether a node is the logic of a type function (`if`, `when`, `do`).
	pub(super) fn is_type_logic(&self, n: NodeId) -> bool {
		match self.view(n) {
			View::If { .. } | View::When { .. } | View::Do(_) => true,
			// `TypeFromSchema<@f(@typeToSchema(Input))>`: evaluated again for
			// each application
			View::Call { callee, .. } => {
				matches!(self.view(callee), View::Name(name) if self.tree.name(name) == "TypeFromSchema")
			}
			_ => false,
		}
	}

	/// `TypeFromSchema<…>` of a generic type function, for an application:
	/// its argument evaluated with the `@typeToSchema` of the parameters
	/// bound (the declaration evaluated it on the parameters themselves).
	fn applied_type_from_schema(
		&mut self,
		n: NodeId,
		argument: NodeId,
		bindings: &Bindings,
	) -> TypeId {
		let mut nodes = vec![argument];
		let mut index = 0;
		while index < nodes.len() && nodes.len() < 4096 {
			let node = nodes[index];
			nodes.extend(syntax::children(&self.tree, node));
			index += 1;
		}
		let mut saved: Vec<(NodeId, Option<fa_interpreter::Value>)> = Vec::new();
		for &node in &nodes {
			let View::Call { callee, arguments } = self.view(node) else {
				continue;
			};
			let View::Name(name) = self.view(callee) else {
				continue;
			};
			if self.tree.name(name) == "@typeToSchema"
				&& let Some(first) = arguments.first()
			{
				let ty = self.s.types.substitute(self.type_of(first), bindings);
				let schema = crate::comptime::schema::schema(&self.s.types, ty);
				let previous = self.a.constants.insert(node, schema);
				saved.push((node, previous));
			} else if self.tree.name(name).starts_with('@') {
				// a compile-time call: evaluated again
				let previous = self.a.constants.remove(&node);
				saved.push((node, previous));
			}
		}
		for &node in &nodes {
			self.comptime_evaluated.remove(&node);
		}
		let ty = match self.evaluate_at(argument, n) {
			Some(value) => {
				let mut named = Vec::new();
				self.schema_type(n, &value, &mut named, 0)
			}
			None => TypeId::UNKNOWN,
		};
		for (node, previous) in saved {
			match previous {
				Some(previous) => self.a.constants.insert(node, previous),
				None => self.a.constants.remove(&node),
			};
			self.comptime_evaluated.remove(&node);
		}
		ty
	}

	/// The logic of a type declaration: its value node, when it is one.
	pub(super) fn type_logic_of(&self, symbol: SymbolId) -> Option<NodeId> {
		let decl = self.a.symbols.symbol(symbol).decl_node;
		match self.view(decl) {
			View::TypeDecl { value, .. } if self.is_type_logic(value) => Some(value),
			_ => None,
		}
	}

	/// The generic check of the logic of a type function (at its
	/// declaration): every part resolved, no result (`UNKNOWN`).
	pub(super) fn check_type_logic(&mut self, n: NodeId) -> TypeId {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::If {
				condition,
				then_body,
				else_body,
				..
			} => {
				let ty = self.check(condition, Some(TypeId::BOOLEAN));
				self.sink(condition, ty, TypeId::BOOLEAN);
				self.check_type_logic(then_body);
				if let Some(else_body) = else_body {
					self.check_type_logic(else_body);
				}
			}
			View::When { subject, arms } => {
				self.check_type_expr(subject);
				for arm in arms.iter() {
					if let Some(pattern) = arm.pattern {
						self.check_type_expr(pattern);
					}
					self.check_type_logic(arm.value);
				}
			}
			View::Do(body) => {
				self.check_type_logic(body);
			}
			View::Block { statements } => {
				if let Some(last) = statements.last() {
					self.check_type_logic(last);
				}
			}
			_ => {
				self.check_type_expr(n);
			}
		}
		self.memo(n, |_| TypeId::UNKNOWN)
	}

	/// The result of a type function with logic for its bindings, at the
	/// application `at`.
	pub(super) fn apply_type_logic(
		&mut self,
		at: NodeId,
		symbol: SymbolId,
		value: NodeId,
		bindings: &Bindings,
	) -> TypeId {
		if self.type_function_depth >= MAX_NESTING {
			let diagnostic = self.diagnostic(
				&META004,
				at,
				format!(
					"more than {MAX_NESTING} nested applications of type functions: it may never end"
				),
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		// the value parameters, as constants of the conditions
		let decl = self.a.symbols.symbol(symbol).decl_node;
		let scope = self
			.a
			.symbols
			.scope_for(decl, None, crate::symbols::ScopeKind::Block);
		let mut bound = Vec::new();
		for (index, &param) in bindings.params.iter().enumerate() {
			let info = self.s.types.param(param);
			if !info.value {
				continue;
			}
			let Some(param_symbol) = self.a.symbols.lookup_in(scope, info.name) else {
				continue;
			};
			if self.a.symbols.symbol(param_symbol).kind != SymbolKind::Static {
				continue;
			}
			if let Some(value) = self.literal_value(bindings.types[index]) {
				let previous = self.a.static_values.insert(param_symbol, value);
				bound.push((param_symbol, previous));
			}
		}
		self.type_function_depth += 1;
		let ty = self.select_type(at, value, bindings);
		self.type_function_depth -= 1;
		for (param_symbol, previous) in bound {
			match previous {
				Some(previous) => self.a.static_values.insert(param_symbol, previous),
				None => self.a.static_values.remove(&param_symbol),
			};
		}
		ty
	}

	/// The value of a literal type (a value parameter's binding).
	fn literal_value(&self, ty: TypeId) -> Option<Value> {
		match self.s.types.kind(ty) {
			TypeKind::Int(_) => self
				.s
				.types
				.as_int(ty)
				.filter(|int| int.min == int.max)
				.map(|int| Value::Integer(int.min)),
			TypeKind::StringLit(text) => Some(Value::string(self.s.types.syms.resolve(text))),
			TypeKind::BoolLit(value) => Some(Value::Boolean(value)),
			_ => None,
		}
	}

	fn select_type(&mut self, at: NodeId, n: NodeId, bindings: &Bindings) -> TypeId {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::If {
				condition,
				then_body,
				else_body,
				..
			} => {
				// evaluated again for every application
				self.comptime_evaluated.remove(&condition);
				match self.evaluate_at(condition, at) {
					Some(Value::Boolean(true)) => self.select_type(at, then_body, bindings),
					Some(Value::Boolean(false)) => match else_body {
						Some(else_body) => self.select_type(at, else_body, bindings),
						None => TypeId::NONE,
					},
					Some(other) => {
						let message = format!(
							"the condition of a type function is a `Boolean` known at compile time, not `{}`",
							other.to_text()
						);
						let diagnostic = self.diagnostic(&META006, condition, message);
						self.report(diagnostic);
						TypeId::UNKNOWN
					}
					None => TypeId::UNKNOWN,
				}
			}
			View::When { subject, arms } => {
				let subject_ty = self.type_of(subject);
				let subject_ty = self.s.types.substitute(subject_ty, bindings);
				for arm in arms.iter() {
					let matches = match arm.pattern {
						None => true,
						Some(pattern) => {
							let pattern_ty = self.type_of(pattern);
							let pattern_ty = self.s.types.substitute(pattern_ty, bindings);
							self.s.types.satisfies(subject_ty, pattern_ty)
						}
					};
					if matches {
						return self.select_type(at, arm.value, bindings);
					}
				}
				// no branch for the subject: an error at the application
				let message = format!(
					"no branch of this `when` matches `{}`: add a branch for it, or an `else`",
					self.display(subject_ty)
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::types::TF007, at, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			View::Do(body) => self.select_type(at, body, bindings),
			View::Call { arguments, .. } if self.is_type_logic(n) && arguments.len() == 1 => {
				let argument = arguments.first().expect("an argument");
				self.applied_type_from_schema(at, argument, bindings)
			}
			View::Block { statements } => match statements.last() {
				Some(last) => self.select_type(at, last, bindings),
				None => TypeId::NONE,
			},
			_ => {
				let ty = self.type_of(n);
				if ty.is_unset() {
					return TypeId::UNKNOWN;
				}
				self.s.types.substitute(ty, bindings)
			}
		}
	}
}
