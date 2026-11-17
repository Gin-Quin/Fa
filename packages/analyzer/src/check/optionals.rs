//! Optionals (values brief §9, types brief §3.4.4): `?.` chains and `??`.
//! `ops.rs` dispatches the two operators here. A chain that starts with
//! `?.` short-circuits as a whole: `a?.b.c` is `none` when `a` is.

use crate::check::Walker;
use crate::diagnostics::codes::values::E_OPT_3;
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
		self.check_member_chain(n, target, members, true)
	}

	/// Validate an explicit optional receiver, retaining the declared type
	/// even when flow has narrowed the value to None.
	pub(super) fn optional_receiver(
		&mut self,
		n: NodeId,
		target: NodeId,
		receiver: TypeId,
	) -> TypeId {
		if self.s.types.is_recovering(receiver) {
			return TypeId::UNKNOWN;
		}
		let (mut current, optional) = self.strip_none(receiver);
		// narrowed to `none` (`x = none` then `x?.a`): the members of the
		// declared type, the chain is always `none`
		if current == TypeId::NEVER {
			let declared = self.declared_type_of(target);
			current = self.strip_none(declared).0;
		}
		if !optional && !self.is_optional_chain(target) {
			// optional by declaration, narrowed here: say what narrowed it
			let declared = self.declared_type_of(target);
			let site = self
				.has_none(declared)
				.then(|| self.narrowing_site(target))
				.flatten();
			let diagnostic = match site {
				Some(site) => {
					let text = self
						.tree
						.text(self.strip_groups(target).index())
						.to_string();
					let line = self.line_of(site);
					let message = format!(
						"`{text}` is never `none` here: it was narrowed at line {line}; use `.` instead of `?.`"
					);
					let location = crate::diagnostics::Location {
						file: self.file,
						span: self.span(site),
					};
					self.diagnostic(&E_OPT_3, n, message).with_related(
						location,
						format!(
							"`{text}` is narrowed to a `{}` here",
							self.display(receiver)
						),
					)
				}
				None if self.has_none(declared) => {
					let text = self
						.tree
						.text(self.strip_groups(target).index())
						.to_string();
					let message = format!(
						"`{text}` is never `none` here: it is narrowed to a `{}`; use `.` instead of `?.`",
						self.display(receiver)
					);
					self.diagnostic(&E_OPT_3, n, message)
				}
				None => {
					let message = format!(
						"a `{}` is never `none`: use `.` instead of `?.`",
						self.display(receiver)
					);
					self.diagnostic(&E_OPT_3, n, message)
				}
			};
			self.report(diagnostic);
		}
		current
	}
}
