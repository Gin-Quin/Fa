//! `Array.from(source, capacity = n)` and `Array(T).from(…)`
//! (`collections/arrays.md`, types brief §3.13): a new array holding a copy
//! of the items of a collection. With `Array(T)`, the source is checked with
//! `Array(T)` as its expected type (a literal adapts, `[]` is typed) and its
//! items must be `T`s; without it, the items keep the source's item type.
//! `capacity` is a hint, a natural integer.
//!
//! The call is recorded with `Callee::Builtin(Array.from)` and the arguments
//! `[source, capacity]` (`Argument::Default` when `capacity` is omitted).

use std::sync::Arc;

use crate::analysis::{Argument, CallInfo, Callee, NodeFlags};
use crate::builtins::{self, Ty};
use crate::check::Walker;
use crate::diagnostics::Code;
use crate::diagnostics::codes::types::{FN022, FN023, FN024, FN025};
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, TypeKind};

impl Walker<'_> {
	/// `Array.from(…)` / `Array(T).from(…)`, unless `Array` is a local. `None`
	/// for any other callee.
	pub(super) fn check_collection_from(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let View::Access { target, members } = syntax::view(&tree, callee) else {
			return None;
		};
		if members.len() != 1 || tree.text(members.first()?.index()) != "from" {
			return None;
		}
		let base = match syntax::view(&tree, target) {
			View::Name(name) => name,
			View::Call { callee: inner, .. } => match syntax::view(&tree, inner) {
				View::Name(name) => name,
				_ => return None,
			},
			_ => return None,
		};
		if tree.name(base) != "Array" {
			return None;
		}
		let sym = self.s.types.syms.intern("Array");
		if self.a.symbols.lookup(self.body.scope, sym).is_some() {
			return None;
		}
		let from = builtins::by_path("Array.from")?;
		let member = members.first()?;
		// the item type, when written
		let given_item = match syntax::view(&tree, target) {
			View::Call { .. } => {
				let ty = self.check_type_expr(target);
				Some(match self.s.types.kind(ty) {
					TypeKind::Collection(CollKind::Array, item, _) => item,
					_ => TypeId::UNKNOWN,
				})
			}
			_ => None,
		};
		self.set_flag(target, NodeFlags::TYPE_EXPR);
		self.memo(member, |w| {
			w.set_resolution(member, Resolution::Builtin(from));
			builtins::value_type(&mut w.s.types, from).unwrap_or(TypeId::UNKNOWN)
		});
		// the arguments: `source`, then `capacity = n`
		let mut source = None;
		let mut capacity = None;
		let mut errors: Vec<(Code, NodeId, String)> = Vec::new();
		for argument in arguments.iter() {
			match syntax::view(&tree, argument) {
				View::Assignment {
					target: name,
					annotation: None,
					value: Some(value),
				} if matches!(syntax::view(&tree, name), View::Name(_)) => {
					let text = tree.text(name.index());
					if text != "capacity" {
						errors.push((
							&FN023,
							name,
							format!(
								"`Array.from` has no option `{text}`: its only option is `capacity`"
							),
						));
						self.check(value, None);
					} else if capacity.is_some() {
						errors.push((&FN022, name, "`capacity` is already given".to_string()));
						self.check(value, None);
					} else {
						capacity = Some((argument, name, value));
					}
				}
				_ if source.is_none() && capacity.is_none() => source = Some(argument),
				_ => {
					errors.push((
						&FN025,
						argument,
						"`Array.from` takes a collection, and the option `capacity = n`"
							.to_string(),
					));
					self.check(argument, None);
				}
			}
		}
		let expected = given_item.map(|item| self.s.types.array(item));
		let source_ty = source.map(|source| self.check(source, expected));
		let natural = Ty::Natural.build(&mut self.s.types);
		if let Some((_, _, value)) = capacity {
			let actual = self.check(value, Some(TypeId::INTEGER));
			self.sink(value, actual, natural);
		}
		let source_item = source_ty.map(|ty| self.collection_items(ty));
		if let (Some(source), Some(actual), Some(expected)) = (source, source_ty, expected)
			&& matches!(
				self.s.types.kind(actual),
				TypeKind::Collection(CollKind::Array, ..)
			) {
			self.sink(source, actual, expected);
		}
		let ty = self.memo(n, |w| {
			for (code, node, message) in errors {
				let diagnostic = w.diagnostic(code, node, message);
				w.report(diagnostic);
			}
			if let Some((argument, name, value)) = capacity {
				let ty = w.type_of(value);
				w.set_type(name, natural);
				w.set_type(argument, ty);
			}
			let item = match (source, source_item) {
				(None, _) => {
					let diagnostic = w.diagnostic(
						&FN024,
						n,
						"`from` takes the collection to copy, ex: `Array<Integer>.from([])`",
					);
					w.report(diagnostic);
					given_item.unwrap_or(TypeId::UNKNOWN)
				}
				(Some(source), Some(None)) => {
					let actual = w.type_of(source);
					if !w.s.types.is_recovering(actual) {
						let message = format!(
							"`Array.from` copies a collection, not a `{}`",
							w.display(actual)
						);
						let diagnostic = w.diagnostic(&E_TYPE_1, source, message);
						w.report(diagnostic);
					}
					given_item.unwrap_or(TypeId::UNKNOWN)
				}
				(_, Some(Some(item))) => given_item.unwrap_or_else(|| w.s.types.widen(item)),
				(Some(_), None) => given_item.unwrap_or(TypeId::UNKNOWN),
			};
			w.a.calls.insert(
				n,
				CallInfo {
					callee: Callee::Builtin(from),
					arguments: vec![
						source.map_or(Argument::Missing, Argument::Given),
						capacity.map_or(Argument::Default, |(_, _, value)| Argument::Given(value)),
					],
				},
			);
			w.s.types.array(item)
		});
		let function = self.type_of(member);
		self.memo(callee, |_| function);
		if given_item.is_none() {
			self.memo(target, |_| ty);
		}
		Some(ty)
	}

	/// The item type of a collection that `from` copies (`None`: not a
	/// collection; `UNKNOWN` while recovering).
	fn collection_items(&self, ty: TypeId) -> Option<TypeId> {
		if self.s.types.is_recovering(ty) {
			return Some(TypeId::UNKNOWN);
		}
		match self.s.types.kind(ty) {
			TypeKind::Collection(CollKind::Map, ..) => None,
			TypeKind::Collection(_, item, _) => Some(item),
			_ => None,
		}
	}
}
