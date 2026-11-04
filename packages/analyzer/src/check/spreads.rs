//! Spreads in object literals (values brief §11, types brief D9, D58):
//! `{ ...defaults, port = 8080 }`. A later field replaces a copied one of
//! the same type (`E-OBJ-7` otherwise); a field written before a spread that
//! provides it is `E-OBJ-8`; a spread of a non-object is `E-OBJ-9`. Against
//! an expected type, a spread can't bring a field the type lacks (`E-OBJ-1`,
//! D9: narrowing is a copy to an annotated binding, not a literal).

use std::sync::Arc;

use crate::analysis::{FieldValue, ObjectInfo};
use crate::check::Walker;
use crate::diagnostics::Code;
use crate::diagnostics::codes::values::{
	E_OBJ_1, E_OBJ_2, E_OBJ_3, E_OBJ_4, E_OBJ_7, E_OBJ_8, E_OBJ_9, E_TYPE_1,
};
use crate::ids::{NodeId, Sym, TypeId};
use crate::syntax::{self, NodeList, View};
use crate::types::{FieldEntry, FieldFlags, Mode, TypeKind};

/// An item of a literal with spreads, in source order.
#[derive(Copy, Clone, Debug)]
enum Part {
	/// `name = value` or `name`: the item, the name, the value.
	Field(NodeId, Sym, NodeId),
	/// `...value`: the item and the value.
	Spread(NodeId, NodeId),
}

impl Walker<'_> {
	/// Whether a literal has a spread (handled here rather than by `objects.rs`).
	pub(super) fn has_spread(&self, items: NodeList<'_>) -> bool {
		items
			.iter()
			.any(|item| matches!(self.view(item), View::Spread(_)))
	}

	/// An object literal with spreads. `target` is the object type it
	/// builds and the named-union member, when one is expected or written.
	pub(super) fn check_spread_literal(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		target: Option<(TypeId, Option<TypeId>)>,
	) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let mut parts = Vec::with_capacity(items.len());
		for item in items.iter() {
			let part = match syntax::view(&tree, item) {
				View::Spread(value) => Part::Spread(item, value),
				View::Assignment {
					target: name,
					annotation: None,
					value: Some(value),
				} => match syntax::view(&tree, name) {
					View::Name(name) => {
						Part::Field(item, self.s.types.syms.intern(tree.name(name)), value)
					}
					_ => return self.unsupported(n, "this object literal"),
				},
				View::Name(name) => {
					Part::Field(item, self.s.types.syms.intern(tree.name(name)), item)
				}
				View::Error => continue,
				_ => return self.unsupported(n, "this object literal"),
			};
			parts.push(part);
		}
		let declared = target.map(|(object, _)| self.object_declaration(object).1);
		let declared_type = |name: Sym| {
			declared.as_ref().and_then(|declared| {
				declared
					.iter()
					.find(|field| field.name == name)
					.map(|field| field.ty)
			})
		};
		// the values, in source order: every visit
		let mut spread_types: Vec<(NodeId, TypeId)> = Vec::new();
		let mut provided: Vec<(Sym, TypeId)> = Vec::new();
		for &part in &parts {
			match part {
				Part::Spread(_, value) => {
					let ty = self.check(value, None);
					spread_types.push((value, ty));
					for field in self.spread_source_fields(ty) {
						provided.retain(|(name, _)| *name != field.name);
						provided.push((field.name, field.ty));
					}
				}
				Part::Field(_, name, value) => {
					let expected = declared_type(name);
					let replaced = provided
						.iter()
						.find(|(other, _)| *other == name)
						.map(|&(_, ty)| ty);
					let actual = self.check(value, expected.or(replaced));
					match (replaced, expected) {
						// replacing a copied field: the same type (`E-OBJ-7`)
						(Some(copied), _) => {
							let wanted = expected.unwrap_or(copied);
							if !self.s.types.is_recovering(actual)
								&& self.s.types.assignable(actual, wanted, Mode::Copy).is_err()
							{
								let text = self.s.types.syms.resolve(name).to_string();
								let message = format!(
									"`{text}` replaces a field copied by the spread: it keeps its type `{}`, not `{}`",
									self.display(wanted),
									self.display(self.s.types.widen(actual))
								);
								let diagnostic = self.diagnostic(&E_OBJ_7, value, message);
								self.report_flow(diagnostic);
							} else {
								self.sink(value, actual, wanted);
							}
						}
						(None, Some(expected)) => self.sink(value, actual, expected),
						(None, None) => {}
					}
				}
			}
		}
		self.memo(n, |w| {
			w.spread_literal_type(n, &parts, target, declared.as_deref())
		})
	}

	/// The public data fields a spread value brings (an object, or a member
	/// with object data); none for anything else (`E-OBJ-9`, reported later).
	fn spread_source_fields(&self, ty: TypeId) -> Vec<FieldEntry> {
		let object = match self.s.types.kind(self.s.types.unfold(ty)) {
			TypeKind::Member(nominal, index) => self.s.types.payload(nominal, index),
			_ => self.s.types.unfold(ty),
		};
		self.s
			.types
			.fields_of(object)
			.iter()
			.copied()
			.filter(|field| !field.flags.contains(FieldFlags::PRIVATE))
			.collect()
	}

	/// The type part of a literal with spreads: its diagnostics, its type and
	/// its `ObjectInfo`.
	fn spread_literal_type(
		&mut self,
		n: NodeId,
		parts: &[Part],
		target: Option<(TypeId, Option<TypeId>)>,
		declared: Option<&[crate::analysis::FieldDecl]>,
	) -> TypeId {
		let mut problems: Vec<(Code, NodeId, String)> = Vec::new();
		// the source of each field, the last one wins: (name, type, value)
		let mut sources: Vec<(Sym, TypeId, FieldValue)> = Vec::new();
		let mut written: Vec<Sym> = Vec::new();
		let owner = target.map(|(object, _)| self.display(object));
		let inside = target.is_some_and(|(object, _)| self.is_inside_type(object));
		for &part in parts {
			match part {
				Part::Spread(item, value) => {
					let ty = self.type_of(value);
					if self.s.types.is_recovering(ty) {
						continue;
					}
					let object_like = match self.s.types.kind(self.s.types.unfold(ty)) {
						TypeKind::Object(_) => true,
						TypeKind::Member(nominal, index) => matches!(
							self.s.types.kind(self.s.types.payload(nominal, index)),
							TypeKind::Object(_)
						),
						_ => false,
					};
					if !object_like {
						let message = format!(
							"only an object can be spread, not a `{}`{}",
							self.display(ty),
							if self.s.types.satisfies(TypeId::NONE, ty) {
								": check it is not `none` first"
							} else {
								""
							}
						);
						problems.push((&E_OBJ_9, value, message));
						continue;
					}
					for field in self.spread_source_fields(ty) {
						let text = self.s.types.syms.resolve(field.name).to_string();
						if written.contains(&field.name) {
							problems.push((
								&E_OBJ_8,
								item,
								format!(
									"`{text}` is written before this spread, which replaces it: write it after the spread"
								),
							));
						}
						if let Some(declared) = declared {
							match declared.iter().find(|declared| declared.name == field.name) {
								None => {
									problems.push((
										&E_OBJ_1,
										item,
										format!(
											"the spread brings `{text}`, which is not a field of `{}`: copy the value into a `{}` first",
											owner.as_deref().unwrap_or(""),
											owner.as_deref().unwrap_or("")
										),
									));
									continue;
								}
								Some(declared)
									if self
										.s
										.types
										.assignable(field.ty, declared.ty, Mode::Copy)
										.is_err() =>
								{
									problems.push((
										&E_TYPE_1,
										item,
										format!(
											"the spread's `{text}` is a `{}`, but `{}` expects a `{}`",
											self.display(field.ty),
											owner.as_deref().unwrap_or(""),
											self.display(declared.ty)
										),
									));
								}
								Some(_) => {}
							}
						}
						sources.retain(|(name, _, _)| *name != field.name);
						sources.push((field.name, field.ty, FieldValue::Spread(value)));
					}
				}
				Part::Field(item, name, value) => {
					let text = self.s.types.syms.resolve(name).to_string();
					if written.contains(&name) {
						problems.push((
							&E_OBJ_3,
							item,
							format!("the field `{text}` is given twice"),
						));
						continue;
					}
					written.push(name);
					if let Some(declared) = declared {
						match declared.iter().find(|declared| declared.name == name) {
							None => {
								problems.push((
									&E_OBJ_1,
									item,
									format!(
										"`{text}` is not a field of `{}`",
										owner.as_deref().unwrap_or("")
									),
								));
								continue;
							}
							Some(declared)
								if declared.flags.contains(FieldFlags::PRIVATE) && !inside =>
							{
								problems.push((
									&E_OBJ_4,
									item,
									format!("`{text}` is private: it can't be given here"),
								));
							}
							Some(_) => {}
						}
					}
					let ty = declared
						.and_then(|declared| declared.iter().find(|declared| declared.name == name))
						.map_or_else(
							|| self.s.types.widen(self.type_of(value)),
							|declared| declared.ty,
						);
					self.set_type(item, ty);
					sources.retain(|(other, _, _)| *other != name);
					sources.push((name, ty, FieldValue::Given(value)));
				}
			}
		}
		for (code, node, message) in problems {
			let diagnostic = self.diagnostic(code, node, message);
			self.report(diagnostic);
		}
		let Some((object, member)) = target else {
			// an anonymous object: the fields in the order they come
			if sources
				.iter()
				.any(|(_, ty, _)| self.s.types.is_recovering(*ty) || *ty == TypeId::VOID)
			{
				return TypeId::UNKNOWN;
			}
			let entries: Vec<FieldEntry> = sources
				.iter()
				.map(|&(name, ty, _)| FieldEntry {
					name,
					ty,
					flags: FieldFlags::empty(),
				})
				.collect();
			let ty = self.s.types.object(&entries, None);
			let fields = sources
				.iter()
				.map(|&(name, _, value)| (name, value))
				.collect();
			self.a.objects.insert(
				n,
				ObjectInfo {
					ty,
					decl: None,
					member: None,
					fields,
				},
			);
			return ty;
		};
		let (decl, declared) = self.object_declaration(object);
		let mut values = Vec::with_capacity(declared.len());
		let mut missing = Vec::new();
		for field in &declared {
			let value = match sources.iter().find(|(name, _, _)| *name == field.name) {
				Some(&(_, _, value)) => value,
				None => match field.default {
					Some(default) => FieldValue::Default(default),
					None if let Some(constant) = field.constant => FieldValue::Constant(constant),
					None => {
						if !self.s.types.satisfies(TypeId::NONE, field.ty)
							&& !self.s.types.is_recovering(field.ty)
						{
							missing.push(format!("`{}`", self.s.types.syms.resolve(field.name)));
						}
						FieldValue::None
					}
				},
			};
			values.push((field.name, value));
		}
		if !missing.is_empty() {
			let plural = if missing.len() == 1 { "" } else { "s" };
			let message = format!(
				"missing field{plural} of `{}`: {}",
				self.display(object),
				missing.join(", ")
			);
			let diagnostic = self.diagnostic(&E_OBJ_2, n, message);
			self.report(diagnostic);
		}
		self.a.objects.insert(
			n,
			ObjectInfo {
				ty: object,
				decl,
				member,
				fields: values,
			},
		);
		member.unwrap_or(object)
	}
}
