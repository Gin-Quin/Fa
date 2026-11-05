//! Type operations (types brief §3.10, §3.12.4): the difference `A - B`
//! (D10), spreads in object types (D58), and the built-in type functions
//! that transform object types: `Pick`, `Omit` (D15), `Partial`, `Required`
//! (D21) and `KeyType`.

use std::sync::Arc;

use ethnum::I256;

use crate::analysis::{FieldDecl, NodeFlags};
use crate::check::Walker;
use crate::diagnostics::codes::types::{TF010, TF021, TF022, TF024, TY100, TY101, TY110, TY111};
use crate::ids::{NodeId, Sym, TypeId};
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, FieldEntry, FieldFlags, TypeKind};

/// A key of a key set (`{ foo, nested { a, b } }`): a name and its children.
struct Key {
	name: Sym,
	node: NodeId,
	children: Vec<Key>,
}

impl Walker<'_> {
	// ── difference ───────────────────────────────────────────────────────

	/// `A - B - …` in a type: the members of `A` that satisfy none of the
	/// others are kept (types brief §3.10). A named union or an enumeration
	/// keeps its identity (a masked subset, D10); an integer range keeps the
	/// remaining ranges. `TY110` for a type that can't be split, `TY111` for a
	/// right side that removes nothing.
	pub(super) fn type_difference(&mut self, n: NodeId, operands: NodeList<'_>) -> TypeId {
		let types: Vec<TypeId> = operands
			.iter()
			.map(|operand| self.check_type_expr(operand))
			.collect();
		self.memo(n, |w| {
			if types.iter().any(|&ty| w.s.types.is_recovering(ty)) {
				return TypeId::UNKNOWN;
			}
			let mut current = types[0];
			for (index, &removed) in types.iter().enumerate().skip(1) {
				let node = operands.get(index).unwrap_or(n);
				current = match w.difference(n, node, current, removed) {
					Some(ty) => ty,
					None => return TypeId::UNKNOWN,
				};
			}
			current
		})
	}

	/// `left - removed`; `None` after a diagnostic. Type part.
	fn difference(
		&mut self,
		n: NodeId,
		node: NodeId,
		left: TypeId,
		removed: TypeId,
	) -> Option<TypeId> {
		let parts: Vec<TypeId> = match self.s.types.kind(removed) {
			TypeKind::Union(list) => self.s.types.list_items(list).to_vec(),
			TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
				let enumeration = matches!(self.s.types.kind(removed), TypeKind::Enum(..));
				let count = self
					.s
					.types
					.sym_list(self.s.types.nominal(nominal).members)
					.len();
				let indices: Vec<u16> = (0..count as u16)
					.filter(|&index| self.s.types.mask_has(mask, index))
					.collect();
				indices
					.into_iter()
					.map(|index| {
						if enumeration {
							let single = self.s.types.single_mask(index);
							self.s.types.enumeration(nominal, single)
						} else {
							self.s.types.member(nominal, index)
						}
					})
					.collect()
			}
			_ => vec![removed],
		};
		let mut current = left;
		for part in parts {
			let next = match self.remove_member(current, part) {
				Ok(next) => next,
				Err(()) => {
					let shown = self.display(self.s.types.widen(left));
					let message = format!(
						"`{shown}` can't be split: no type means every value of `{shown}` but some; list the values you want"
					);
					let diagnostic = self.diagnostic(&TY110, n, message);
					self.report(diagnostic);
					return None;
				}
			};
			if next == current {
				let message = format!(
					"`{}` is not a member of `{}`: nothing is removed",
					self.display(part),
					self.display(left)
				);
				let diagnostic = self.diagnostic(&TY111, node, message);
				self.report(diagnostic);
				return None;
			}
			current = next;
		}
		Some(current)
	}

	/// `ty` without the values of `part`: `Err` when `ty` can't be split.
	fn remove_member(&mut self, ty: TypeId, part: TypeId) -> Result<TypeId, ()> {
		match self.s.types.kind(ty) {
			TypeKind::Union(list) => {
				let members = self.s.types.list_items(list).to_vec();
				let mut kept = Vec::with_capacity(members.len());
				for member in members {
					// a member that only some values of `part` cover: split it too
					match self.remove_member(member, part) {
						Ok(rest) => kept.push(rest),
						Err(()) => kept.push(member),
					}
				}
				Ok(self.s.types.union(&kept))
			}
			TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
				let index = match self.s.types.kind(part) {
					TypeKind::Member(other, index) if other == nominal => Some(index),
					TypeKind::EnumMember(other, index) if other == nominal => Some(index),
					TypeKind::Enum(other, single) if other == nominal => {
						let count = self
							.s
							.types
							.sym_list(self.s.types.nominal(nominal).members)
							.len();
						(0..count as u16).find(|&index| self.s.types.mask_has(single, index))
					}
					_ => None,
				};
				let Some(index) = index.filter(|&index| self.s.types.mask_has(mask, index)) else {
					return Ok(ty);
				};
				let mut words = self.s.types.mask_bits(mask).to_vec();
				words[index as usize / 64] &= !(1 << (index % 64));
				if words.iter().all(|&word| word == 0) {
					return Ok(TypeId::NEVER);
				}
				let mask = self.s.types.mask(&words);
				Ok(match self.s.types.kind(ty) {
					TypeKind::Enum(..) => self.s.types.enumeration(nominal, mask),
					_ => self.s.types.named_union(nominal, mask),
				})
			}
			TypeKind::Int(id) if !self.s.types.is_literal(ty) => {
				let int = *self.s.types.int(id);
				let Some(cut) = self.s.types.as_int(part).copied() else {
					return Ok(ty);
				};
				if cut.max < int.min || cut.min > int.max {
					return Ok(ty);
				}
				let mut pieces = Vec::new();
				if cut.min > int.min {
					pieces.push(
						self.s
							.types
							.int_range(int.min, cut.min - I256::ONE, int.storage),
					);
				}
				if cut.max < int.max {
					pieces.push(
						self.s
							.types
							.int_range(cut.max + I256::ONE, int.max, int.storage),
					);
				}
				Ok(self.s.types.union(&pieces))
			}
			// a single literal, `none`, a member: removed when `part` covers it
			_ if self.s.types.is_literal(ty)
				|| matches!(
					self.s.types.kind(ty),
					TypeKind::None | TypeKind::Member(..) | TypeKind::EnumMember(..)
				) =>
			{
				if self.s.types.satisfies(ty, part) {
					Ok(TypeId::NEVER)
				} else {
					Ok(ty)
				}
			}
			// whole types inside a union (`Integer | String - String`)
			_ if ty == part => Ok(TypeId::NEVER),
			_ => Err(()),
		}
	}

	// ── spreads in object types ──────────────────────────────────────────

	/// `...Parent` in an object type: the public data fields of `Parent`,
	/// with their defaults and flags (not its functions, not its private
	/// fields). `TY100` for a type that is not an object; a field already
	/// given with another type is `TY101` (a type spread never overrides,
	/// D58). Type part, called by `object_type_parts`.
	pub(super) fn spread_type_fields(
		&mut self,
		item: NodeId,
		inner: NodeId,
		fields: &mut Vec<FieldDecl>,
	) -> bool {
		let ty = self.check_type_expr(inner);
		self.set_flag(item, NodeFlags::TYPE_EXPR);
		self.set_type(item, ty);
		if self.s.types.is_recovering(ty) {
			return false;
		}
		let object = match self.s.types.kind(self.s.types.unfold(ty)) {
			TypeKind::Object(_) => self.s.types.unfold(ty),
			TypeKind::Param(param) => {
				let constraint = self.s.types.param(param).constraint;
				if matches!(self.s.types.kind(constraint), TypeKind::Object(_)) {
					constraint
				} else {
					TypeId::UNSET
				}
			}
			_ => TypeId::UNSET,
		};
		if object.is_unset() {
			let message = format!(
				"only object types can be spread, not a `{}`",
				self.display(ty)
			);
			let diagnostic = self.diagnostic(&TY100, inner, message);
			self.report(diagnostic);
			return false;
		}
		let (_, declared) = self.object_declaration(object);
		for field in declared {
			if field.flags.contains(FieldFlags::PRIVATE) {
				continue;
			}
			self.add_field(item, field, fields);
		}
		true
	}

	/// Adds a field to an object type being built: once when it is already
	/// there with the same type, `TY101` with another type.
	pub(super) fn add_field(
		&mut self,
		node: NodeId,
		field: FieldDecl,
		fields: &mut Vec<FieldDecl>,
	) {
		match fields.iter().find(|existing| existing.name == field.name) {
			Some(existing) if existing.ty == field.ty => {}
			Some(existing) => {
				let name = self.s.types.syms.resolve(field.name).to_string();
				let message = format!(
					"the type of `{name}` is ambiguous: `{}` or `{}`; a spread never overrides a field (remove one, ex: with `Omit`)",
					self.display(existing.ty),
					self.display(field.ty)
				);
				let diagnostic = self.diagnostic(&TY101, node, message);
				self.report(diagnostic);
			}
			None => fields.push(field),
		}
	}

	// ── built-in type functions ──────────────────────────────────────────

	/// `Pick(T, keys)`, `Omit(T, keys)`, `Partial(T)`, `Required(T)`,
	/// `KeyType(M)` (types brief §3.12.4). `None` when `name` is not one of
	/// them.
	pub(super) fn builtin_type_function(
		&mut self,
		n: NodeId,
		callee: NodeId,
		name: &str,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		// `Keys`, `Values`, `Parameters`, `Uppercase`… (type_functions.rs)
		if let Some(ty) = self.key_or_function_type_function(n, callee, name, arguments) {
			return Some(ty);
		}
		if !matches!(name, "Pick" | "Omit" | "Partial" | "Required" | "KeyType") {
			return None;
		}
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let keyed = matches!(name, "Pick" | "Omit");
		let expected = if keyed { 2 } else { 1 };
		let subject = arguments.first().map(|first| self.check_type_expr(first));
		let keys_node = if keyed { arguments.get(1) } else { None };
		if let Some(keys_node) = keys_node {
			self.set_flag(keys_node, NodeFlags::TYPE_EXPR);
		}
		let ty = self.memo(n, |w| {
			let Some(subject) = subject.filter(|_| arguments.len() == expected) else {
				let message = if keyed {
					format!(
						"`{name}` takes an object type and a key set, ex: `{name}(User, {{ id, name }})`"
					)
				} else {
					format!("`{name}` takes one type, ex: `{name}(User)`")
				};
				let diagnostic = w.diagnostic(&crate::diagnostics::codes::types::TY003, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if w.s.types.is_recovering(subject) {
				return TypeId::UNKNOWN;
			}
			if name == "KeyType" {
				return match w.s.types.kind(subject) {
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, key) => key,
					_ => {
						let message =
							format!("`KeyType` takes a map type, not a `{}`", w.display(subject));
						let diagnostic = w.diagnostic(&TF022, n, message);
						w.report(diagnostic);
						TypeId::UNKNOWN
					}
				};
			}
			let object = w.s.types.unfold(subject);
			if !matches!(w.s.types.kind(object), TypeKind::Object(_)) {
				let message = format!(
					"`{name}` takes an object type, not a `{}`",
					w.display(subject)
				);
				let diagnostic = w.diagnostic(&TF021, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			match name {
				"Partial" | "Required" => w.partial_or_required(n, object, name == "Partial"),
				_ => {
					let Some(keys_node) = keys_node else {
						return TypeId::UNKNOWN;
					};
					let Some(keys) = w.key_set(keys_node) else {
						return TypeId::UNKNOWN;
					};
					w.pick_or_omit(object, &keys, name == "Pick")
						.unwrap_or(TypeId::UNKNOWN)
				}
			}
		});
		self.memo(callee, |_| ty);
		Some(ty)
	}

	/// `Partial(T)`: every field optional; `Required(T)`: `none` removed
	/// from every field (`TY024` for a field typed exactly `None`). Shallow,
	/// flags kept (D21).
	fn partial_or_required(&mut self, n: NodeId, object: TypeId, partial: bool) -> TypeId {
		let fields = self.s.types.fields_of(object).to_vec();
		let mut entries = Vec::with_capacity(fields.len());
		for field in fields {
			let ty = if partial {
				self.s.types.optional(field.ty)
			} else if field.ty == TypeId::NONE {
				let name = self.s.types.syms.resolve(field.name).to_string();
				let message = format!("`{name}` is always `none`: it can't be required");
				let diagnostic = self.diagnostic(&TF024, n, message);
				self.report(diagnostic);
				field.ty
			} else {
				match self.s.types.kind(field.ty) {
					TypeKind::Union(list) => {
						let members: Vec<TypeId> = self
							.s
							.types
							.list_items(list)
							.iter()
							.copied()
							.filter(|&member| member != TypeId::NONE)
							.collect();
						self.s.types.union(&members)
					}
					_ => field.ty,
				}
			};
			entries.push(FieldEntry { ty, ..field });
		}
		let nominal = match self.s.types.kind(object) {
			TypeKind::Object(shape) => self.s.types.shape(shape).nominal,
			_ => None,
		};
		self.s.types.object(&entries, nominal)
	}

	/// The keys of a key-set literal: `{ foo, nested { a, b } }`. `TF010` for
	/// an item that is not a key.
	fn key_set(&mut self, n: NodeId) -> Option<Vec<Key>> {
		let tree = Arc::clone(&self.tree);
		let items = match syntax::view(&tree, n) {
			View::Object { items } => items,
			_ => {
				let diagnostic = self.diagnostic(
					&TF010,
					n,
					"a key set is written in braces, ex: `{ id, name }`",
				);
				self.report(diagnostic);
				return None;
			}
		};
		self.keys_of(items)
	}

	fn keys_of(&mut self, items: NodeList<'_>) -> Option<Vec<Key>> {
		let tree = Arc::clone(&self.tree);
		let mut keys = Vec::with_capacity(items.len());
		for item in items.iter() {
			let key = match syntax::view(&tree, item) {
				View::Name(name) => Key {
					name: self.s.types.syms.intern(tree.name(name)),
					node: item,
					children: Vec::new(),
				},
				View::TypedObject { target, members } => {
					let View::Name(name) = syntax::view(&tree, target) else {
						return None;
					};
					Key {
						name: self.s.types.syms.intern(tree.name(name)),
						node: item,
						children: self.keys_of(members)?,
					}
				}
				_ => {
					let diagnostic = self.diagnostic(
						&TF010,
						item,
						"a key set lists field names, ex: `{ id, name }`",
					);
					self.report(diagnostic);
					return None;
				}
			};
			self.set_flag(item, NodeFlags::TYPE_EXPR);
			keys.push(key);
		}
		Some(keys)
	}

	/// `Pick(T, keys)` keeps the listed leaves, `Omit(T, keys)` removes them;
	/// a nested group applies to the field's object type, and a group that
	/// names every field of it is the whole field (D15). `TF010` for a key
	/// that is not a field.
	fn pick_or_omit(&mut self, object: TypeId, keys: &[Key], pick: bool) -> Option<TypeId> {
		let fields = self.s.types.fields_of(object).to_vec();
		for key in keys {
			if !fields.iter().any(|field| field.name == key.name) {
				let name = self.s.types.syms.resolve(key.name).to_string();
				let message = format!("`{}` has no field `{name}`", self.display(object));
				let diagnostic = self.diagnostic(&TF010, key.node, message);
				self.report(diagnostic);
				return None;
			}
		}
		let mut entries = Vec::with_capacity(fields.len());
		for field in fields {
			let key = keys.iter().find(|key| key.name == field.name);
			let nested = key.filter(|key| !key.children.is_empty()).map(|key| {
				let inner = self.s.types.unfold(field.ty);
				let complete = matches!(self.s.types.kind(inner), TypeKind::Object(_))
					&& self
						.s
						.types
						.fields_of(inner)
						.iter()
						.all(|child| key.children.iter().any(|k| k.name == child.name));
				(key, inner, complete)
			});
			match (pick, key, nested) {
				// a partial group: the nested object picked or omitted
				(_, Some(_), Some((key, inner, false))) => {
					if !matches!(self.s.types.kind(inner), TypeKind::Object(_)) {
						let name = self.s.types.syms.resolve(field.name).to_string();
						let message =
							format!("`{name}` is not an object: it has no fields to select");
						let diagnostic = self.diagnostic(&TF010, key.node, message);
						self.report(diagnostic);
						return None;
					}
					let ty = self.pick_or_omit(inner, &key.children, pick)?;
					entries.push(FieldEntry { ty, ..field });
				}
				(true, Some(_), _) | (false, None, _) => entries.push(field),
				_ => {}
			}
		}
		// a selection of the fields is a new structural type
		entries.retain(|entry| !entry.flags.contains(FieldFlags::PRIVATE));
		Some(self.s.types.object(&entries, None))
	}
}
