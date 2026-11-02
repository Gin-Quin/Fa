//! Errors: `try`, `throw`, `catch` and the coverage of failing expressions
//! (values brief §15).
//!
//! An expression that can fail (a call to a function that `throws`, a
//! division by a divisor not proven non-zero, a conversion not proven to fit,
//! an index not proven…) calls [`Walker::can_fail`] from its flow part, at
//! every visit: the node gets `CAN_FAIL`, and its errors go to the innermost
//! `try` or `catch` of the same function body whose operand contains it
//! (`BodyCx::handlers`); without one, `E-ERR-3`.
//!
//! What a `try` (or a `throw`) propagates must be declared by the function
//! (`throws`): `E-ERR-4` (`E-ERR-2`) otherwise. It is inferred, and joins the
//! function type, for a single-expression body without `throws`, and for a
//! test body; at the top level of a file, an escaped error ends the program
//! (values brief D-59).

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::builtins::errors::error_type;
use crate::check::Walker;
use crate::diagnostics::codes::values::{
	E_ERR_1, E_ERR_2, E_ERR_3, E_ERR_4, E_ERR_6, E_ERR_9, W_ERR_1, W_ERR_2, W_ERR_3,
};
use crate::ids::{NodeId, TypeId};
use crate::symbols::{ScopeKind, SymbolKind};
use crate::syntax::{self, Name, View};

/// What covers the failing expressions of its operand.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum HandlerKind {
	/// `try`: propagates them.
	Try,
	/// `catch`: handles them.
	Catch,
}

/// An open `try` or `catch` (its operand is being walked).
#[derive(Copy, Clone, Debug)]
pub struct Handler {
	/// `try` or `catch`.
	pub kind: HandlerKind,
	/// The `try` or `catch` node.
	pub node: NodeId,
	/// `functions.len()` when it opened: failing expressions of a nested
	/// function body (a lambda) are not covered by it.
	pub depth: usize,
	/// The errors of the failing expressions met so far (`NEVER`: none).
	pub errors: TypeId,
}

impl Walker<'_> {
	/// A built-in error type (`DivisionByZero`, `ArithmeticError`, `IndexError`…).
	pub(crate) fn builtin_error(&mut self, name: &str) -> TypeId {
		error_type(&mut self.s.types, name).unwrap_or(TypeId::ERROR)
	}

	/// The flow part of an expression that can fail with `errors`: `CAN_FAIL`
	/// on `n`, and the errors go to the innermost handler of the body
	/// (`E-ERR-3` without one).
	pub(crate) fn can_fail(&mut self, n: NodeId, errors: TypeId) {
		// in a `static` value, an error is a compile error (comptime.rs)
		if errors == TypeId::NEVER || errors.is_unset() || self.comptime_values > 0 {
			return;
		}
		self.set_flag(n, NodeFlags::CAN_FAIL);
		let depth = self.body.functions.len();
		let handler = self
			.body
			.handlers
			.iter()
			.rposition(|handler| handler.depth == depth);
		match handler {
			Some(index) => {
				let current = self.body.handlers[index].errors;
				let joined = if current == TypeId::NEVER {
					errors
				} else {
					self.s.types.union(&[current, errors])
				};
				self.body.handlers[index].errors = joined;
			}
			None => {
				if !self.body.flow.reachable || self.s.types.is_recovering(errors) {
					return;
				}
				let text = self.tree.text(n.index()).to_string();
				let what = if text.len() <= 40 && !text.contains('\n') {
					format!("`{text}`")
				} else {
					"this expression".to_string()
				};
				let message = format!(
					"{what} can fail with `{}`: use `try` to propagate the error, or `catch` to handle it",
					self.display(errors)
				);
				let diagnostic = self.diagnostic(&E_ERR_3, n, message);
				self.report_flow(diagnostic);
			}
		}
	}

	/// Opens a handler for the operand about to be walked.
	pub(crate) fn open_handler(&mut self, kind: HandlerKind, node: NodeId) {
		let depth = self.body.functions.len();
		self.body.handlers.push(Handler {
			kind,
			node,
			depth,
			errors: TypeId::NEVER,
		});
	}

	/// Closes the innermost handler and returns what it collected.
	pub(crate) fn close_handler(&mut self) -> TypeId {
		self.body
			.handlers
			.pop()
			.map_or(TypeId::NEVER, |handler| handler.errors)
	}

	/// Whether a `catch` of the current body is open around this point.
	fn inside_catch(&self) -> bool {
		let depth = self.body.functions.len();
		self.body
			.handlers
			.iter()
			.any(|handler| handler.depth == depth && handler.kind == HandlerKind::Catch)
	}

	/// The errors the current body may propagate: `Some(declared)` (possibly
	/// `NEVER`), or `None` when they are inferred or unchecked (a
	/// single-expression function without `throws`, a test body, the top level).
	fn declared_throws(&self) -> Option<TypeId> {
		let cx = self.body.functions.last()?;
		let info = self.a.functions.get(&cx.node)?;
		if info.throws == TypeId::NEVER && !info.block_body {
			return None;
		}
		Some(info.throws)
	}

	/// Errors leaving the current body (`try` or `throw` at `n`): checked
	/// against its declared `throws` (`code` otherwise), accumulated in the
	/// function context.
	fn propagate(&mut self, n: NodeId, errors: TypeId, code: crate::diagnostics::Code, verb: &str) {
		if errors == TypeId::NEVER || self.s.types.is_recovering(errors) {
			return;
		}
		if let Some(cx) = self.body.functions.last_mut() {
			cx.throws = if cx.throws == TypeId::NEVER {
				errors
			} else {
				self.s.types.union(&[cx.throws, errors])
			};
		}
		let Some(declared) = self.declared_throws() else {
			return;
		};
		if declared != TypeId::NEVER && self.s.types.satisfies(errors, declared) {
			return;
		}
		let name = self.current_function_name();
		let message = if declared == TypeId::NEVER {
			format!(
				"{verb} `{}`, which {name} doesn't declare: add `throws {}`, or handle it with `catch`",
				self.display(errors),
				self.display(errors)
			)
		} else {
			format!(
				"{verb} `{}`, which {name} doesn't declare (it declares `{}`): declare it, or handle it with `catch`",
				self.display(errors),
				self.display(declared)
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
	}

	/// The current function, for messages: "`name`" or "this function".
	fn current_function_name(&self) -> String {
		self.body.functions.last().and_then(|cx| cx.symbol).map_or(
			"this function".to_string(),
			|symbol| {
				let name = self.a.symbols.symbol(symbol).name;
				format!("`{}`", self.s.types.syms.resolve(name))
			},
		)
	}

	/// `W-ERR-3`: a `throws` declared by a block body that never throws (type
	/// part, at the end of the body; `thrown` is what the body propagated).
	pub(crate) fn unused_throws(&mut self, n: NodeId, declared: TypeId, thrown: TypeId) {
		if declared == TypeId::NEVER
			|| thrown != TypeId::NEVER
			|| self.s.types.is_recovering(declared)
		{
			return;
		}
		let message = format!(
			"this function declares `throws {}` but never throws: remove it",
			self.display(declared)
		);
		let diagnostic = self.diagnostic(&W_ERR_3, n, message);
		self.report(diagnostic);
	}

	/// The node inside the parentheses around `n`, if any.
	fn parenthesized(&self, n: NodeId) -> NodeId {
		let mut current = n;
		while let View::Group(inner) = syntax::view(&self.tree, current) {
			current = inner;
		}
		current
	}

	/// `try expression`: propagates the errors of its operand.
	pub(super) fn check_try(
		&mut self,
		n: NodeId,
		value: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let nested_in_catch = self.inside_catch();
		// `(try f()) catch …`: the `catch` reports the pair, once
		let catch_operand = self.body.handlers.last().is_some_and(|handler| {
			handler.kind == HandlerKind::Catch
				&& matches!(syntax::view(&self.tree, handler.node), View::Catch { expression, .. }
					if self.parenthesized(expression) == n)
		});
		self.open_handler(HandlerKind::Try, n);
		let ty = self.check(value, expected);
		let errors = self.close_handler();
		let ty = self.memo(n, |w| {
			if nested_in_catch && !catch_operand {
				let diagnostic = w.diagnostic(
					&E_ERR_6,
					n,
					"this `try` is inside the left side of a `catch`: the `catch` handles its errors, remove the `try`",
				);
				w.report(diagnostic);
			}
			ty
		});
		if errors == TypeId::NEVER {
			if self.body.flow.reachable && !self.s.types.is_recovering(ty) {
				let diagnostic =
					self.diagnostic(&W_ERR_1, n, "this expression can't fail: remove the `try`");
				self.report_flow(diagnostic);
			}
		} else if nested_in_catch {
			// `E-ERR-6`: the `catch` around handles them
			let depth = self.body.functions.len();
			if let Some(index) = self
				.body
				.handlers
				.iter()
				.rposition(|handler| handler.depth == depth)
			{
				let current = self.body.handlers[index].errors;
				self.body.handlers[index].errors = if current == TypeId::NEVER {
					errors
				} else {
					self.s.types.union(&[current, errors])
				};
			}
		} else {
			self.propagate(n, errors, &E_ERR_4, "`try` propagates");
		}
		ty
	}

	/// `throw error`: leaves the function with an error value (`Never`).
	pub(super) fn check_throw(&mut self, n: NodeId, value: NodeId) -> TypeId {
		let ty = self.check(value, None);
		self.memo(n, |w| {
			if !w.s.types.is_recovering(ty) && !w.s.types.satisfies(ty, TypeId::ERROR) {
				let message = format!(
					"`throw` needs an error value, not a `{}`: declare an `Errors` type and throw one of its members",
					w.display(w.s.types.widen(ty))
				);
				let diagnostic = w.diagnostic(&E_ERR_1, value, message);
				w.report(diagnostic);
			}
			TypeId::NEVER
		});
		if self.s.types.satisfies(ty, TypeId::ERROR) && ty != TypeId::NEVER {
			let errors = self.s.types.widen(ty);
			self.propagate(n, errors, &E_ERR_2, "this throws");
		}
		self.body.flow.set_unreachable();
		TypeId::NEVER
	}

	/// `expression catch error => handler`: the handler runs instead of the
	/// failed value; typed like the two branches of an `if` (D-98).
	pub(super) fn check_catch(
		&mut self,
		n: NodeId,
		expression: NodeId,
		error_name: Name,
		handler: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let statement = self.is_statement(n);
		let try_operand = matches!(
			syntax::view(&tree, self.parenthesized(expression)),
			View::Try(_)
		);
		let entry = self.body.flow.fork();
		self.open_handler(HandlerKind::Catch, n);
		let left = if statement {
			self.check_stmt(expression);
			self.type_of(expression)
		} else {
			self.check(expression, expected)
		};
		let errors = self.close_handler();
		if !statement
			&& let Some(expected) = expected
			&& left != TypeId::NEVER
		{
			self.sink(expression, left, expected);
		}
		let end_left = std::mem::take(&mut self.body.flow);
		// the handler: from the state before the failing expression, with the error bound
		self.body.flow = self.join_states(&entry, &end_left);
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		// the binding: declared once (its type: the errors of the first visit)
		if !error_name.is_empty() && self.a.symbols.symbol_of_decl(n).is_none() {
			let symbol = self.declare(n, error_name, SymbolKind::CatchBinding);
			self.a.symbols.symbol_mut(symbol).ty = if errors == TypeId::NEVER {
				TypeId::ERROR
			} else {
				errors
			};
		}
		let handler_expected = expected.or_else(|| {
			(!statement && left != TypeId::NEVER && !self.s.types.is_recovering(left))
				.then(|| self.s.types.widen(left))
		});
		let handler_ty = if statement {
			self.check_stmt(handler);
			TypeId::VOID
		} else {
			self.check_block_value(handler, handler_expected).0
		};
		// a sink only for an outer expected type: the left operand's type is a hint (D-98)
		if !statement
			&& let Some(expected) = expected
			&& handler_ty != TypeId::VOID
			&& handler_ty != TypeId::NEVER
		{
			let value = self.handler_value_node(handler);
			self.sink(value, handler_ty, expected);
		}
		self.body.scope = parent;
		let end_handler = std::mem::take(&mut self.body.flow);
		self.body.flow = self.join_states(&end_left, &end_handler);
		if errors == TypeId::NEVER && self.body.flow.reachable && !self.s.types.is_recovering(left)
		{
			let diagnostic = self.diagnostic(
				&W_ERR_2,
				n,
				"this expression can't fail: remove the `catch`",
			);
			self.report_flow(diagnostic);
		}
		self.memo(n, |w| {
			if error_name.is_empty() {
				let diagnostic = w.diagnostic(
					&E_ERR_9,
					n,
					"a `catch` names the error, ex: `catch error => …`",
				);
				w.report(diagnostic);
			}
			if try_operand {
				let diagnostic = w.diagnostic(
					&E_ERR_6,
					n,
					"`try` and `catch` on the same expression: keep `catch` to handle the error, or `try` to propagate it",
				);
				w.report(diagnostic);
			}
			if statement {
				return TypeId::VOID;
			}
			w.branches_type(n, &[(expression, left), (handler, handler_ty)], expected)
		})
	}

	/// The node giving a handler its value: the last statement of a block.
	fn handler_value_node(&self, handler: NodeId) -> NodeId {
		match syntax::view(&self.tree, handler) {
			View::Block { statements } => statements.last().unwrap_or(handler),
			_ => handler,
		}
	}
}
