//! Optionals (values brief §9, types brief §3.4.4): `?.` chains and `??`.
//! `ops.rs` dispatches the two operators here. A chain that starts with
//! `?.` short-circuits as a whole: `a?.b.c` is `none` when `a` is.

use crate::check::Walker;
use crate::diagnostics::codes::values::{E_OPT_3, E_OPT_6};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{NodeList, View};
use crate::types::TypeKind;

impl Walker<'_> {
	/// The type without its `None` member (`T?` → `T`), and whether it had one.
	pub(super) fn strip_none(&mut self, ty: TypeId) -> (TypeId, bool) {
		match self.s.types.kind(ty) {
			TypeKind::Union(list) if self.s.types.list_items(list).contains(&TypeId::NONE) => {
				let rest: Vec<TypeId> = self
					.s
					.types
					.list_items(list)
					.iter()
					.copied()
					.filter(|&member| member != TypeId::NONE)
					.collect();
				(self.s.types.union(&rest), true)
			}
			TypeKind::None => (TypeId::NEVER, true),
			_ => (ty, false),
		}
	}

	/// Whether a node is a `?.` chain: `a?.b`, or an access or a call whose
	/// target is one (`a?.b.c`, `a?.f()`), which short-circuits with it.
	pub(super) fn is_optional_chain(&self, n: NodeId) -> bool {
		match self.view(n) {
			View::OptionalAccess { .. } => true,
			View::Access { target, .. } => self.is_optional_chain(target),
			View::Call { callee, .. } => self.is_optional_chain(callee),
			View::Index { target, .. } => self.is_optional_chain(target),
			_ => false,
		}
	}

	/// `a?.b?.c`: the members of the value when it is not `none`; `none`
	/// otherwise. `E-OPT-3` when the value can't be `none`.
	pub(super) fn check_optional_chain(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		let receiver = self.check(target, None);
		self.memo(n, |w| {
			if w.s.types.is_recovering(receiver) {
				return TypeId::UNKNOWN;
			}
			let (mut current, optional) = w.strip_none(receiver);
			if !optional && !w.is_optional_chain(target) {
				let message = format!(
					"a `{}` is never `none`: use `.` instead of `?.`",
					w.display(receiver)
				);
				let diagnostic = w.diagnostic(&E_OPT_3, n, message);
				w.report(diagnostic);
			}
			for member in members.iter() {
				let stripped = w.strip_none(current).0;
				match w.member_of(member, stripped) {
					Some((ty, _)) => current = ty,
					None => return TypeId::UNKNOWN,
				}
			}
			w.s.types.optional(current)
		})
	}

	/// `a ?? b ?? c`, folding left: the value of `a` when it is not `none`,
	/// otherwise `b`. Each left side must be able to be `none` (`E-OPT-6`).
	pub(super) fn check_fallback_value(
		&mut self,
		n: NodeId,
		operands: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let Some(first) = operands.first() else {
			return TypeId::UNKNOWN;
		};
		let first_expected = expected.map(|expected| self.s.types.optional(expected));
		let mut left = self.check(first, first_expected);
		let mut problems: Vec<(NodeId, TypeId)> = Vec::new();
		for operand in operands.iter().skip(1) {
			let (inner, optional) = if self.s.types.is_recovering(left) {
				(TypeId::UNKNOWN, true)
			} else {
				self.strip_none(left)
			};
			if !optional {
				problems.push((operand, left));
			}
			// the right side runs only when the left one is `none`
			let before = self.body.flow.fork();
			let hint = expected.or((!inner.is_unset() && inner != TypeId::NEVER).then_some(inner));
			let right = self.check(operand, hint);
			let after = self.body.flow.fork();
			self.body.flow = if after.reachable {
				self.join_states(&before, &after)
			} else {
				before
			};
			left = if self.s.types.is_recovering(right) || self.s.types.is_recovering(inner) {
				TypeId::UNKNOWN
			} else if right == TypeId::NEVER {
				inner
			} else if inner == TypeId::NEVER {
				self.s.types.widen(right)
			} else {
				let right = if self.s.types.satisfies(right, inner) {
					inner
				} else {
					self.s.types.widen(right)
				};
				self.s.types.union(&[inner, right])
			};
		}
		self.memo(n, |w| {
			for (operand, left) in problems {
				let message = format!(
					"the left side of `??` is a `{}`, which is never `none`: drop `??`, or use `catch` for a failing call",
					w.display(left)
				);
				let diagnostic = w.diagnostic(&E_OPT_6, operand, message);
				w.report(diagnostic);
			}
			left
		})
	}
}
