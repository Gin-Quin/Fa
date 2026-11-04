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
		let receiver = self.check(target, None);
		self.memo(n, |w| {
			if w.s.types.is_recovering(receiver) {
				return TypeId::UNKNOWN;
			}
			let (mut current, optional) = w.strip_none(receiver);
			// narrowed to `none` (`x = none` then `x?.a`): the members of the
			// declared type, the chain is always `none`
			if current == TypeId::NEVER {
				let declared = w.declared_type_of(target);
				current = w.strip_none(declared).0;
			}
			if !optional && !w.is_optional_chain(target) {
				// optional by declaration, narrowed here: say what narrowed it
				let declared = w.declared_type_of(target);
				let site = w
					.has_none(declared)
					.then(|| w.narrowing_site(target))
					.flatten();
				let diagnostic = match site {
					Some(site) => {
						let text = w.tree.text(w.strip_groups(target).index()).to_string();
						let line = w.line_of(site);
						let message = format!(
							"`{text}` is never `none` here: it was narrowed at line {line}; use `.` instead of `?.`"
						);
						let location = crate::diagnostics::Location {
							file: w.file,
							span: w.span(site),
						};
						w.diagnostic(&E_OPT_3, n, message).with_related(
							location,
							format!("`{text}` is narrowed to a `{}` here", w.display(receiver)),
						)
					}
					None if w.has_none(declared) => {
						let text = w.tree.text(w.strip_groups(target).index()).to_string();
						let message = format!(
							"`{text}` is never `none` here: it is narrowed to a `{}`; use `.` instead of `?.`",
							w.display(receiver)
						);
						w.diagnostic(&E_OPT_3, n, message)
					}
					None => {
						let message = format!(
							"a `{}` is never `none`: use `.` instead of `?.`",
							w.display(receiver)
						);
						w.diagnostic(&E_OPT_3, n, message)
					}
				};
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
}
