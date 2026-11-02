//! Objects (values brief §11, types brief §3.2): untyped and typed literals,
//! defaults, field access, field places (`readonly`, `#private`, methods),
//! `.=`; members of named unions and enumerations as values; the members of
//! arrays (`length`, `push`); method values.

use std::sync::Arc;

use crate::analysis::{FieldDecl, FieldValue, ObjectInfo};
use crate::builtins;
use crate::check::{PlaceInfo, Walker};
use crate::diagnostics::Code;
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::types::{FN042, TY021, TY053, TY054, TY055, TY056, TY084};
use crate::diagnostics::codes::values::{
	E_DECL_4, E_OBJ_1, E_OBJ_2, E_OBJ_3, E_OBJ_4, E_OBJ_5, E_OBJ_10, E_OBJ_11, E_OBJ_13, E_OPT_1,
	E_TYPE_1,
};
use crate::ids::{NodeId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, Name, NodeList, View};
use crate::types::{CollKind, FieldEntry, FieldFlags, FnSig, TypeKind};

/// A field written in a literal: its name, the item node, the value node.
#[derive(Copy, Clone, Debug)]
struct LiteralField {
	name: Name,
	item: NodeId,
	value: NodeId,
	annotation: Option<NodeId>,
}

/// What a member of a value is.
#[derive(Copy, Clone, Debug)]
pub(super) enum MemberKind {
	/// A data field: its flags.
	Field(FieldFlags),
	/// A function of the type (method or associated function).
	Function,
	/// A member of a built-in type (`length`, `push`, a string method).
	Builtin,
}

impl Walker<'_> {
	// ── literals ─────────────────────────────────────────────────────────

	/// `{ a = 1, b, … }`: checked against the expected object type when there
	/// is one; otherwise an anonymous object of the widened field types.
	pub(super) fn check_object(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		if let Some(function) = items
			.iter()
			.find(|&item| matches!(self.view(item), View::Function { .. }))
		{
			self.quiet += 1;
			self.walk_children(n);
			self.quiet -= 1;
			return self.memo(n, |w| {
				let diagnostic = w.diagnostic(
					&FN042,
					function,
					"functions are declared in a `type`: declare one, or use a field holding a lambda",
				);
				w.report(diagnostic);
				TypeId::UNKNOWN
			});
		}
		// `{ ...value, field = … }` (spreads.rs)
		if self.has_spread(items) {
			let target = expected.and_then(|expected| self.literal_target(expected));
			return self.check_spread_literal(n, items, target);
		}
		let Some(fields) = self.literal_fields(items) else {
			return self.unsupported(n, "this object literal");
		};
		if let Some(expected) = expected
			&& let Some((code, message)) = self.unnamed_member(expected)
		{
			for field in &fields {
				self.check(field.value, None);
			}
			return self.memo(n, |w| {
				let diagnostic = w.diagnostic(code, n, message);
				w.report(diagnostic);
				TypeId::UNKNOWN
			});
		}
		let target = match expected.map(|expected| (expected, self.s.types.kind(expected))) {
			Some((_, TypeKind::Member(..))) => None,
			Some((expected, _)) => self.literal_target_for(expected, &fields),
			None => None,
		};
		match target {
			Some((object, member)) => {
				self.check_literal_against(n, &fields, object, member);
				self.type_of(n)
			}
			None => self.anonymous_literal(n, &fields),
		}
	}

	/// `TY055` / `TY056`: a literal without the member name where a named
	/// union, or one of its members with object data, is expected.
	fn unnamed_member(&mut self, expected: TypeId) -> Option<(Code, String)> {
		match self.s.types.kind(expected) {
			TypeKind::NamedUnion(nominal, mask) => {
				let count = self
					.s
					.types
					.sym_list(self.s.types.nominal(nominal).members)
					.len() as u16;
				let object = (0..count).find(|&index| {
					self.s.types.mask_has(mask, index)
						&& matches!(
							self.s.types.kind(self.s.types.payload(nominal, index)),
							TypeKind::Object(_)
						)
				})?;
				let name = self.display(expected);
				let member = self.s.types.member_name(nominal, object).to_string();
				Some((
					&TY055,
					format!("a `{name}` is built by naming its member: `{name}.{member} {{ … }}`"),
				))
			}
			TypeKind::Member(nominal, index)
				if matches!(
					self.s.types.kind(self.s.types.payload(nominal, index)),
					TypeKind::Object(_)
				) =>
			{
				let name = self.display(expected);
				Some((
					&TY056,
					format!("a plain object is never a `{name}`: write `{name} {{ … }}`"),
				))
			}
			_ => None,
		}
	}

	/// The object type an untyped literal builds where `expected` is
	/// expected; in a union of several object types, the one whose fields the
	/// literal gives (`TY041` when several fit).
	fn literal_target_for(
		&mut self,
		expected: TypeId,
		fields: &[LiteralField],
	) -> Option<(TypeId, Option<TypeId>)> {
		let expected = self.s.types.unfold(expected);
		let TypeKind::Union(list) = self.s.types.kind(expected) else {
			return self.literal_target(expected);
		};
		let objects: Vec<TypeId> = self
			.s
			.types
			.list_items(list)
			.iter()
			.map(|&member| self.s.types.unfold(member))
			.filter(|&member| matches!(self.s.types.kind(member), TypeKind::Object(_)))
			.collect();
		if objects.len() < 2 {
			return self.literal_target(expected);
		}
		let fitting: Vec<TypeId> = objects
			.iter()
			.copied()
			.filter(|&object| self.literal_fits(object, fields))
			.collect();
		match fitting.as_slice() {
			[object] => Some((*object, None)),
			[] => None,
			_ => None,
		}
	}

	/// Whether a literal gives exactly the fields of an object type: every
	/// written field exists, every required one is written, and a field whose
	/// type is a literal type gets that literal.
	fn literal_fits(&mut self, object: TypeId, fields: &[LiteralField]) -> bool {
		let declared: Vec<FieldEntry> = self.s.types.fields_of(object).to_vec();
		for field in fields {
			let text = self.tree.name(field.name);
			let Some(entry) = declared
				.iter()
				.find(|entry| self.s.types.syms.resolve(entry.name) == text)
			else {
				return false;
			};
			if self.s.types.is_literal(entry.ty) {
				let tree = Arc::clone(&self.tree);
				let written = match syntax::view(&tree, field.value) {
					View::String(text) => Some(self.s.types.string_literal(text)),
					View::Integer(value) => Some(self.s.types.int_literal(value)),
					View::Boolean(value) => Some(self.s.types.bool_literal(value)),
					_ => None,
				};
				if written.is_some_and(|written| written != entry.ty) {
					return false;
				}
			}
		}
		declared.iter().all(|entry| {
			fields
				.iter()
				.any(|field| self.tree.name(field.name) == self.s.types.syms.resolve(entry.name))
				|| self.s.types.satisfies(TypeId::NONE, entry.ty)
		})
	}

	/// `Point { x = 1 }`, `Shape.Circle { radius = 1.0 }`.
	pub(super) fn check_typed_object(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		let ty = self.literal_target_type(target);
		if self.has_spread(members)
			&& self.free_params(ty).is_empty()
			&& let Some(built) = self.literal_target(ty)
		{
			return self.check_spread_literal(n, members, Some(built));
		}
		let Some(fields) = self.literal_fields(members) else {
			return self.unsupported(n, "this object literal");
		};
		// a generic type without arguments: inferred from the fields (GN014, GN015)
		let free = self.free_params(ty);
		let ty = if free.is_empty() {
			ty
		} else {
			self.infer_literal(n, ty, &free, &fields)
		};
		if self.s.types.is_recovering(ty) {
			self.quiet += 1;
			for field in &fields {
				self.check(field.value, None);
			}
			self.quiet -= 1;
			return self.memo(n, |_| TypeId::UNKNOWN);
		}
		match self.literal_target(ty) {
			Some((object, member)) => {
				self.check_literal_against(n, &fields, object, member);
				self.type_of(n)
			}
			None => {
				for field in &fields {
					self.check(field.value, None);
				}
				self.memo(n, |w| {
					let shown = w.display(ty);
					let (code, message): (Code, String) = match w.s.types.kind(ty) {
						TypeKind::Member(..) if w.member_data(ty).is_some() => (
							&TY053,
							format!("`{shown}` holds a value, not fields: write `{shown}(value)`"),
						),
						TypeKind::Member(..) | TypeKind::EnumMember(..) => (
							&TY054,
							format!("`{shown}` holds no data: write it without braces"),
						),
						_ => (
							&E_TYPE_1,
							format!(
								"`{shown}` is not an object type: its values are not built with braces"
							),
						),
					};
					let diagnostic = w.diagnostic(code, target, message);
					w.report(diagnostic);
					TypeId::UNKNOWN
				})
			}
		}
	}

	/// The parameters of a generic type built by a literal, from the types of
	/// its field values (types brief §3.11.4); the instance type.
	fn infer_literal(
		&mut self,
		n: NodeId,
		generic: TypeId,
		free: &[crate::ids::ParamId],
		fields: &[LiteralField],
	) -> TypeId {
		let object = match self.s.types.kind(generic) {
			TypeKind::Member(nominal, index) => self.s.types.payload(nominal, index),
			_ => generic,
		};
		let declared: Vec<FieldEntry> = self.s.types.fields_of(object).to_vec();
		let mut bindings = crate::types::Bindings::new(free);
		let mut candidates = crate::types::Candidates::default();
		for field in fields {
			let text = self.tree.name(field.name);
			let Some(entry) = declared
				.iter()
				.find(|entry| self.s.types.syms.resolve(entry.name) == text)
			else {
				continue;
			};
			if !self.has_free(entry.ty, free) {
				continue;
			}
			let actual = self.check(field.value, None);
			self.s
				.types
				.unify(entry.ty, actual, &bindings, &mut candidates);
		}
		let mut problems = Vec::new();
		self.resolve_candidates(&mut bindings, &candidates, &mut problems);
		self.finish_bindings(&mut bindings, &mut problems);
		self.check_constraints(&mut bindings, &mut problems, false);
		let failed = bindings.types.contains(&TypeId::UNKNOWN);
		let instance = if failed {
			TypeId::UNKNOWN
		} else {
			self.instantiate(generic, &bindings)
		};
		if self.type_of(n).is_unset() {
			for (code, message) in problems {
				let diagnostic = self.diagnostic(code, n, message);
				self.report_once_on(n, diagnostic);
			}
			self.record_instantiation(n, &bindings);
		}
		instance
	}

	/// Reports a type diagnostic once per node and code.
	fn report_once_on(&mut self, n: NodeId, diagnostic: crate::diagnostics::Diagnostic) {
		if self.reported_once.insert((n, diagnostic.code.id)) {
			self.report(diagnostic);
		}
	}

	/// The fields written in a literal; `None` when one of them is a form the
	/// analyzer doesn't handle yet (a spread, M2).
	fn literal_fields(&self, items: NodeList<'_>) -> Option<Vec<LiteralField>> {
		let mut fields = Vec::with_capacity(items.len());
		for item in items.iter() {
			match self.view(item) {
				View::Assignment {
					target,
					annotation,
					value: Some(value),
				} => match self.view(target) {
					View::Name(name) => fields.push(LiteralField {
						name,
						item,
						value,
						annotation,
					}),
					_ => return None,
				},
				// `{ id }` is `{ id = id }`
				View::Name(name) => fields.push(LiteralField {
					name,
					item,
					value: item,
					annotation: None,
				}),
				View::Error => {}
				_ => return None,
			}
		}
		Some(fields)
	}

	/// The object type a literal builds where `ty` is expected: an object
	/// type, a member of a named union with object data, or the object
	/// member of an optional.
	fn literal_target(&mut self, ty: TypeId) -> Option<(TypeId, Option<TypeId>)> {
		let ty = self.s.types.unfold(ty);
		match self.s.types.kind(ty) {
			TypeKind::Object(_) => Some((ty, None)),
			TypeKind::Member(nominal, index) => {
				let payload = self.s.types.payload(nominal, index);
				matches!(self.s.types.kind(payload), TypeKind::Object(_))
					.then_some((payload, Some(ty)))
			}
			TypeKind::Union(list) => {
				let objects: Vec<TypeId> = self
					.s
					.types
					.list_items(list)
					.iter()
					.map(|&member| self.s.types.unfold(member))
					.filter(|&member| matches!(self.s.types.kind(member), TypeKind::Object(_)))
					.collect();
				match objects.as_slice() {
					[object] => Some((*object, None)),
					_ => None,
				}
			}
			_ => None,
		}
	}

	/// An anonymous object literal: the widened types of its values, or their
	/// annotations (`{ count: Int8 = 1 }`).
	fn anonymous_literal(&mut self, n: NodeId, fields: &[LiteralField]) -> TypeId {
		let mut types = Vec::with_capacity(fields.len());
		for field in fields {
			let declared = field
				.annotation
				.map(|annotation| self.check_type_expr(annotation));
			let actual = self.check(field.value, declared);
			if let Some(declared) = declared {
				self.sink(field.value, actual, declared);
			}
			types.push(declared.unwrap_or(actual));
		}
		self.memo(n, |w| {
			let mut entries: Vec<FieldEntry> = Vec::with_capacity(fields.len());
			let mut values = Vec::with_capacity(fields.len());
			for (field, &ty) in fields.iter().zip(&types) {
				let text = w.tree.name(field.name).to_string();
				let sym = w.s.types.syms.intern(&text);
				if entries.iter().any(|entry| entry.name == sym) {
					let diagnostic = w.diagnostic(
						&E_OBJ_3,
						field.item,
						format!("the field `{text}` is given twice"),
					);
					w.report(diagnostic);
					continue;
				}
				if matches!(ty, TypeId::VOID) {
					let diagnostic = w.diagnostic(
						&crate::diagnostics::codes::values::E_DECL_9,
						field.value,
						format!(
							"this expression has no value (`Void`): the field `{text}` can't hold it"
						),
					);
					w.report(diagnostic);
				}
				if ty == TypeId::NONE && field.annotation.is_none() {
					let diagnostic = w.diagnostic(
						&crate::diagnostics::codes::values::E_DECL_11,
						field.value,
						format!(
							"`none` alone doesn't give `{text}` a type: annotate it, ex: `{text}: Integer? = none`"
						),
					);
					w.report(diagnostic);
				}
				let ty = if field.annotation.is_some() {
					ty
				} else {
					w.s.types.widen(ty)
				};
				w.set_type(field.item, ty);
				entries.push(FieldEntry {
					name: sym,
					ty,
					flags: FieldFlags::empty(),
				});
				values.push((sym, FieldValue::Given(field.value)));
			}
			if entries
				.iter()
				.any(|entry| w.s.types.is_recovering(entry.ty) || entry.ty == TypeId::VOID)
			{
				return TypeId::UNKNOWN;
			}
			let ty = w.s.types.object(&entries, None);
			w.a.objects.insert(
				n,
				ObjectInfo {
					ty,
					decl: None,
					member: None,
					fields: values,
				},
			);
			ty
		})
	}

	/// A literal checked against an object type: every field written must
	/// exist (`E-OBJ-1`), once (`E-OBJ-3`), not be private outside the type
	/// (`E-OBJ-4`) nor a function (`E-OBJ-11`); every field without default
	/// that doesn't accept `none` must be written (`E-OBJ-2`). The literal's
	/// type is the member when one is built, the object type otherwise.
	fn check_literal_against(
		&mut self,
		n: NodeId,
		fields: &[LiteralField],
		object: TypeId,
		member: Option<TypeId>,
	) {
		let (decl, declared) = self.object_declaration(object);
		let inside = self.is_inside_type(object);
		// the values: every visit
		for field in fields {
			let text = self.tree.name(field.name);
			let expected = declared
				.iter()
				.find(|declared| self.s.types.syms.resolve(declared.name) == text)
				.map(|declared| declared.ty);
			let actual = self.check(field.value, expected);
			if let Some(expected) = expected {
				self.sink(field.value, actual, expected);
			}
		}
		let result = member.unwrap_or(object);
		self.memo(n, |w| {
			let type_name = w.display(object);
			let mut given: Vec<Sym> = Vec::new();
			for field in fields {
				let text = w.tree.name(field.name).to_string();
				let sym = w.s.types.syms.intern(&text);
				let code_message: Option<(Code, String)> = if given.contains(&sym) {
					Some((&E_OBJ_3, format!("the field `{text}` is given twice")))
				} else {
					match declared.iter().find(|declared| declared.name == sym) {
						None if w.type_function(object, &text).is_some() => Some((
							&E_OBJ_11,
							format!("`{text}` is a function of `{type_name}`, not a field: it can't be given"),
						)),
						None => Some((
							&E_OBJ_1,
							format!("`{text}` is not a field of `{type_name}`"),
						)),
						Some(declared) if declared.flags.contains(FieldFlags::PRIVATE) && !inside => {
							Some((
								&E_OBJ_4,
								format!("`{text}` is private: only the functions of `{type_name}` can give it"),
							))
						}
						Some(declared) => {
							w.set_type(field.item, declared.ty);
							None
						}
					}
				};
				given.push(sym);
				if let Some((code, message)) = code_message {
					let diagnostic = w.diagnostic(code, field.item, message);
					w.report(diagnostic);
				}
			}
			let mut values = Vec::with_capacity(declared.len());
			let mut missing = Vec::new();
			for declared in &declared {
				let value = match fields.iter().find(|field| {
					w.s.types.syms.resolve(declared.name) == w.tree.name(field.name)
				}) {
					Some(field) => FieldValue::Given(field.value),
					None => match declared.default {
						Some(default) => FieldValue::Default(default),
						None if w.s.types.satisfies(TypeId::NONE, declared.ty)
							&& !w.s.types.is_recovering(declared.ty) =>
						{
							FieldValue::None
						}
						None => {
							let name = w.s.types.syms.resolve(declared.name).to_string();
							if declared.flags.contains(FieldFlags::PRIVATE) && !inside {
								let diagnostic = w.diagnostic(
									&E_OBJ_4,
									n,
									format!("`{type_name}` has the private field `{name}` without default: only its functions can build it"),
								);
								w.report(diagnostic);
							} else if !w.s.types.is_recovering(declared.ty) {
								missing.push(format!("`{name}`"));
							}
							FieldValue::None
						}
					},
				};
				values.push((declared.name, value));
			}
			if !missing.is_empty() {
				let plural = if missing.len() == 1 { "" } else { "s" };
				let diagnostic = w.diagnostic(
					&E_OBJ_2,
					n,
					format!("missing field{plural} of `{type_name}`: {}", missing.join(", ")),
				);
				w.report(diagnostic);
			}
			w.a.objects.insert(
				n,
				ObjectInfo {
					ty: object,
					decl,
					member,
					fields: values,
				},
			);
			result
		});
	}

	/// The fields of an object type, as declared (the declaration of its
	/// spelling, with defaults and declaration order), or from its shape.
	pub(super) fn object_declaration(
		&mut self,
		object: TypeId,
	) -> (Option<NodeId>, Vec<FieldDecl>) {
		if let Some((decl, mut fields)) = self.declared_fields(object)
			&& !fields.is_empty()
		{
			// an instance of a generic type: its fields substituted
			if let Some(bindings) = self.instance_bindings.get(&object).cloned() {
				for field in &mut fields {
					field.ty = self.s.types.substitute(field.ty, &bindings);
				}
			}
			return (Some(decl), fields);
		}
		let fields = self
			.s
			.types
			.fields_of(object)
			.iter()
			.map(|field| FieldDecl {
				node: NodeId::new(0),
				name: field.name,
				ty: field.ty,
				default: None,
				flags: field.flags,
			})
			.collect();
		(None, fields)
	}

	/// Whether the code being walked is inside the functions of the type `ty`
	/// (private fields, `readonly` writes).
	pub(super) fn is_inside_type(&mut self, ty: TypeId) -> bool {
		let Some((own, _)) = self.self_type(self.body.scope) else {
			return false;
		};
		if own == ty {
			return true;
		}
		let nominal_of = |w: &Self, ty: TypeId| match w.s.types.kind(ty) {
			TypeKind::Object(shape) => w.s.types.shape(shape).nominal,
			_ => None,
		};
		nominal_of(self, own).is_some() && nominal_of(self, own) == nominal_of(self, ty)
	}

	/// `.red` where an enumeration or a named union is expected: its member
	/// (types brief §3.8).
	pub(super) fn check_implicit_member_value(
		&mut self,
		n: NodeId,
		name: Name,
		expected: Option<TypeId>,
	) -> TypeId {
		self.memo(n, |w| {
			let text = w.tree.name(name).to_string();
			let candidates: Vec<TypeId> = match expected {
				Some(expected) => w.s.types.union_members(&expected).to_vec(),
				None => Vec::new(),
			};
			if candidates.iter().any(|&ty| w.s.types.is_recovering(ty)) {
				return TypeId::UNKNOWN;
			}
			for candidate in candidates {
				let whole = match w.s.types.kind(candidate) {
					TypeKind::Enum(nominal, _)
					| TypeKind::EnumMember(nominal, _)
					| TypeKind::NamedUnion(nominal, _)
					| TypeKind::Member(nominal, _) => w.s.types.nominal_type(nominal),
					_ => continue,
				};
				if let Some(member) = w.nominal_member(whole, &text)
					&& w.member_data(member).is_none()
				{
					w.set_member_resolution(n, member);
					return member;
				}
			}
			let (code, message): (Code, String) = match expected {
				Some(expected)
					if !matches!(
						w.s.types.kind(expected),
						TypeKind::Enum(..) | TypeKind::NamedUnion(..)
					) =>
				{
					(
						&TY084,
						format!("write `Type.{text}`: the type of `.{text}` is not known here"),
					)
				}
				Some(expected) => (
					&E_OBJ_5,
					format!("`{}` has no member `{text}`", w.display(expected)),
				),
				None => (
					&TY084,
					format!("write `Type.{text}`: the type of `.{text}` is not known here"),
				),
			};
			let diagnostic = w.diagnostic(code, n, message);
			w.report(diagnostic);
			TypeId::UNKNOWN
		})
	}

	// ── reading members ─────────────────────────────────────────────────

	/// `a.b.c` on a value (not a built-in namespace): fields, functions of
	/// the type, members of strings and arrays; or, after a type name,
	/// members of a named union or an enumeration and associated functions.
	pub(super) fn check_member_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		if let Some(ty) = self.type_target(target) {
			return self.memo(n, |w| w.type_member_chain(target, ty, members));
		}
		let receiver = self.check(target, None);
		// `a?.b.c`: the chain short-circuits with its `?.`
		let chained = self.is_optional_chain(target);
		self.memo(n, |w| {
			let mut current = if chained {
				w.strip_none(receiver).0
			} else {
				receiver
			};
			// the methods of the type written for the receiver (structural.rs)
			let mut written = w.written_decl(target);
			for member in members.iter() {
				w.receiver_decl = written.take();
				let found = w.member_of(member, current);
				w.receiver_decl = None;
				match found {
					Some((ty, _)) => current = ty,
					None => return TypeId::UNKNOWN,
				}
			}
			if chained {
				w.s.types.optional(current)
			} else {
				current
			}
		})
	}

	/// The type a name denotes when it is a type used as the target of an
	/// access (`Shape.Circle`, `Point.origin`, `Self.create`).
	fn type_target(&mut self, target: NodeId) -> Option<TypeId> {
		self.type_of_target(target)
	}

	/// The type a type name denotes as the target of an access or of a typed
	/// literal: a type of this file (a generic one without its arguments,
	/// which the construction infers), `Self`, or a generic application
	/// `Result(Integer, String)`. `None` when it is not a type.
	pub(super) fn type_of_target(&mut self, target: NodeId) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, target) {
			View::Name(name) => {
				let text = tree.name(name);
				let sym = self.s.types.syms.intern(text);
				match self.a.symbols.lookup(self.body.scope, sym) {
					Some(symbol) if self.a.symbols.symbol(symbol).kind == SymbolKind::Type => {
						self.set_resolution(target, Resolution::Local(symbol));
						self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
						let ty = self.symbol_type(symbol, target);
						self.memo(target, |_| ty);
						Some(ty)
					}
					None if text == "Self" => self
						.self_type(self.body.scope)
						.map(|_| self.check_type_expr(target)),
					_ => None,
				}
			}
			View::Call { callee, .. } => {
				let View::Name(name) = syntax::view(&tree, callee) else {
					return None;
				};
				let sym = self.s.types.syms.intern(tree.name(name));
				let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
				(self.a.symbols.symbol(symbol).kind == SymbolKind::Type)
					.then(|| self.check_type_expr(target))
			}
			_ => None,
		}
	}

	/// The type of a typed literal's target (`Point`, `Shape.Circle`,
	/// `Point(Float)`): a generic type without arguments stays generic, its
	/// parameters inferred from the fields.
	fn literal_target_type(&mut self, target: NodeId) -> TypeId {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, target) {
			View::Name(_) => match self.type_of_target(target) {
				Some(ty) => ty,
				None => self.check_type_expr(target),
			},
			View::Access {
				target: base,
				members,
			} if members.len() == 1 => {
				let Some(base_ty) = self.type_of_target(base) else {
					return self.check_type_expr(target);
				};
				let member = members.first().expect("one member");
				let View::Name(name) = syntax::view(&tree, member) else {
					return TypeId::UNKNOWN;
				};
				let text = tree.name(name).to_string();
				self.memo(target, |w| match w.nominal_member(base_ty, &text) {
					Some(ty) => {
						w.set_member_resolution(member, ty);
						w.set_type(member, ty);
						ty
					}
					None if w.s.types.is_recovering(base_ty) => TypeId::UNKNOWN,
					None => {
						let message = format!("`{}` has no member `{text}`", w.display(base_ty));
						let diagnostic = w.diagnostic(&E_OBJ_5, member, message);
						w.report(diagnostic);
						TypeId::UNKNOWN
					}
				})
			}
			_ => self.check_type_expr(target),
		}
	}

	/// `Type.member…`: a member of a named union or enumeration (a value), or
	/// a function of the type; the rest of the chain reads that value. Type part.
	fn type_member_chain(&mut self, target: NodeId, ty: TypeId, members: NodeList<'_>) -> TypeId {
		if self.s.types.is_recovering(ty) {
			return TypeId::UNKNOWN;
		}
		let Some(first) = members.first() else {
			return TypeId::UNKNOWN;
		};
		let View::Name(name) = self.view(first) else {
			return TypeId::UNKNOWN;
		};
		let text = self.tree.name(name).to_string();
		let mut current = if let Some(member) = self.nominal_member(ty, &text) {
			self.set_member_resolution(first, member);
			self.set_type(first, member);
			match self.member_data(member) {
				Some(data) => {
					let message = format!(
						"`{}` holds data: write `{}.{text}({})` or `{}.{text} {{ … }}`",
						self.display(member),
						self.tree.text(target.index()),
						self.display(data),
						self.tree.text(target.index()),
					);
					let diagnostic = self.diagnostic(&E_TYPE_1, first, message);
					self.report(diagnostic);
					return TypeId::UNKNOWN;
				}
				None => member,
			}
		} else if text == "members" && matches!(self.s.types.kind(ty), TypeKind::Enum(..)) {
			if let Some(builtin) = builtins::by_path("Enum.members") {
				self.set_resolution(first, Resolution::Builtin(builtin));
			}
			self.s.types.array(ty)
		} else if let Some(function) = self.type_function(ty, &text) {
			self.set_resolution(first, Resolution::Local(function));
			let function_ty = self.symbol_type(function, first);
			self.set_type(first, function_ty);
			function_ty
		} else {
			let message = format!("`{}` has no member or function `{text}`", self.display(ty));
			let diagnostic = self.diagnostic(&E_OBJ_5, first, message);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		};
		for member in members.iter().skip(1) {
			match self.member_of(member, current) {
				Some((ty, _)) => current = ty,
				None => return TypeId::UNKNOWN,
			}
		}
		current
	}

	/// The data a member of a named union holds, when it holds data that is
	/// not a literal (a literal member is its own value).
	pub(super) fn member_data(&self, member: TypeId) -> Option<TypeId> {
		let TypeKind::Member(nominal, index) = self.s.types.kind(member) else {
			return None;
		};
		let payload = self.s.types.payload(nominal, index);
		(!payload.is_unset() && !self.s.types.is_literal(payload)).then_some(payload)
	}

	/// Records that a node names a member of a nominal type.
	fn set_member_resolution(&mut self, node: NodeId, member: TypeId) {
		if let TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) =
			self.s.types.kind(member)
		{
			self.set_resolution(node, Resolution::Member(nominal, index));
		}
	}

	/// The member `member` (a name node) of a value of type `receiver`: its
	/// type and what it is, with its resolution set. Reports `E-OBJ-5`,
	/// `E-OBJ-4`, `E-OPT-1`, `TY021` and returns `None` (already reported, or
	/// recovering). Type part.
	pub(super) fn member_of(
		&mut self,
		member: NodeId,
		receiver: TypeId,
	) -> Option<(TypeId, MemberKind)> {
		if self.s.types.is_recovering(receiver) || receiver.is_unset() {
			return None;
		}
		let View::Name(name) = self.view(member) else {
			return None;
		};
		let text = self.tree.name(name).to_string();
		// a recursive reference reads like the type it stands for
		let receiver = self.s.types.unfold(receiver);
		let found = self.find_member(member, receiver, &text);
		if let Some((ty, _)) = found {
			self.set_type(member, ty);
		}
		found
	}

	fn find_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		text: &str,
	) -> Option<(TypeId, MemberKind)> {
		let types = &self.s.types;
		if crate::builtins::strings::is_string_like(types, receiver) {
			return match self.string_member(member, receiver, text) {
				Some(ty) => Some((ty, MemberKind::Builtin)),
				None => {
					self.no_member(member, receiver, text);
					None
				}
			};
		}
		// the members of numbers (`x.sqrt()`, `i.toString()`, builtins/numbers.rs)
		// and the fields of ranges (`r.start`, builtins/ranges.rs)
		let range = receiver == TypeId::RANGE;
		if range || builtins::numbers::number_owner(types, receiver).is_some() {
			let Some(builtin) = builtins::instance_member(types, receiver, text) else {
				self.no_member(member, receiver, text);
				return None;
			};
			self.set_resolution(member, Resolution::Builtin(builtin));
			let ty = builtins::member_type(&mut self.s.types, builtin, receiver)?;
			let kind = if range {
				MemberKind::Field(FieldFlags::empty())
			} else {
				MemberKind::Builtin
			};
			return Some((ty, kind));
		}
		// a function of the type's declaration (methods of objects and unions)
		if let Some(function) = self.type_function(receiver, text) {
			self.set_resolution(member, Resolution::Local(function));
			let function_ty = self.symbol_type(function, member);
			let bound = self.bound_method_type(function, function_ty);
			let bound = match self.instance_bindings.get(&receiver).cloned() {
				Some(bindings) => self.s.types.substitute(bound, &bindings),
				None => bound,
			};
			return Some((bound, MemberKind::Function));
		}
		if let TypeKind::Param(param) = self.s.types.kind(receiver) {
			let constraint = self.s.types.param(param).constraint;
			if constraint == TypeId::UNKNOWN || constraint.is_unset() {
				let message = format!(
					"`{}` has no constraint giving it a member `{text}`: constrain it, ex: `{}: {{ {text}: … }}`",
					self.display(receiver),
					self.display(receiver)
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::types::GN016, member, message);
				self.report(diagnostic);
				return None;
			}
			return self.find_member(member, constraint, text);
		}
		match self.s.types.kind(receiver) {
			TypeKind::Union(list) if self.s.types.list_items(list).contains(&TypeId::NONE) => {
				let message = format!(
					"this value may be `none` (`{}`): use `?.{text}`, or check it first",
					self.display(receiver)
				);
				let diagnostic = self.diagnostic(&E_OPT_1, member, message);
				self.report(diagnostic);
				None
			}
			TypeKind::Object(_) | TypeKind::Member(..) => {
				let object = match self.s.types.kind(receiver) {
					TypeKind::Member(nominal, index) => {
						let payload = self.s.types.payload(nominal, index);
						if !matches!(self.s.types.kind(payload), TypeKind::Object(_)) {
							self.no_member(member, receiver, text);
							return None;
						}
						payload
					}
					_ => receiver,
				};
				let TypeKind::Object(shape) = self.s.types.kind(object) else {
					return None;
				};
				let fields = self.s.types.shape_fields(shape);
				let Some(index) = fields
					.iter()
					.position(|field| self.s.types.syms.resolve(field.name) == text)
				else {
					self.no_member(member, receiver, text);
					return None;
				};
				let field = fields[index];
				if field.flags.contains(FieldFlags::PRIVATE) && !self.is_inside_type(object) {
					let message = format!(
						"`{text}` is private: only the functions of `{}` can use it",
						self.display(object)
					);
					let diagnostic = self.diagnostic(&E_OBJ_4, member, message);
					self.report(diagnostic);
					return None;
				}
				self.set_resolution(member, Resolution::Field(shape, index as u16));
				Some((field.ty, MemberKind::Field(field.flags)))
			}
			TypeKind::Union(list) => {
				let members = self.s.types.list_items(list).to_vec();
				let mut types = Vec::with_capacity(members.len());
				for member_ty in members {
					let field = self
						.s
						.types
						.fields_of(member_ty)
						.iter()
						.find(|field| self.s.types.syms.resolve(field.name) == text)
						.copied();
					match field {
						Some(field) if !field.flags.contains(FieldFlags::PRIVATE) => {
							types.push(field.ty)
						}
						_ => {
							let message = format!(
								"`{text}` is not a field of every member of `{}`: narrow the value first",
								self.display(receiver)
							);
							let diagnostic = self.diagnostic(&E_OBJ_5, member, message);
							self.report(diagnostic);
							return None;
						}
					}
				}
				let ty = self.s.types.union(&types);
				Some((ty, MemberKind::Field(FieldFlags::empty())))
			}
			TypeKind::NamedUnion(..) => {
				let message = format!(
					"a `{}` may hold any of its members: narrow it to a member first, with `when` or `is`",
					self.display(receiver)
				);
				let diagnostic = self.diagnostic(&TY021, member, message);
				self.report(diagnostic);
				None
			}
			TypeKind::Collection(kind @ (CollKind::Array | CollKind::GrowingArray), item, _) => {
				self.array_member(member, kind, item, text)
			}
			TypeKind::Collection(..) => self.collection_member(member, receiver, text),
			TypeKind::Iterator(item, _) => self.iterator_member(member, receiver, item, text),
			TypeKind::Enum(..) | TypeKind::EnumMember(..) if text == "toString" => {
				let builtin = builtins::by_path("Enum#toString")?;
				self.set_resolution(member, Resolution::Builtin(builtin));
				let ty = builtins::value_type(&mut self.s.types, builtin)?;
				Some((ty, MemberKind::Builtin))
			}
			_ => {
				let message = format!(
					"the members of a `{}` are not supported yet by the analyzer",
					self.display(receiver)
				);
				let diagnostic = self.diagnostic(&AN001, member, message);
				self.report(diagnostic);
				None
			}
		}
	}

	/// `E-OBJ-5`: no such field or method.
	pub(super) fn no_member(&mut self, member: NodeId, receiver: TypeId, text: &str) {
		let shown = self.display(self.s.types.widen(receiver));
		let message = if crate::builtins::strings::is_string_like(&self.s.types, receiver) {
			format!(
				"a `{shown}` has no field or method `{text}`: see the string methods (`length`, `includes`, `split`…); no method changes a string"
			)
		} else {
			format!("a `{shown}` has no field or method `{text}`")
		};
		let diagnostic = self.diagnostic(&E_OBJ_5, member, message);
		self.report(diagnostic);
	}

	/// The type of a method read from a value: without its `self` parameter.
	pub(super) fn bound_method_type(&mut self, function: SymbolId, ty: TypeId) -> TypeId {
		if !self.is_method(function) {
			return ty;
		}
		let Some(sig) = self.signature_of(ty) else {
			return ty;
		};
		let params = self.s.types.params(sig.params).to_vec();
		let list = self.s.types.param_list(params.get(1..).unwrap_or(&[]));
		self.s.types.function(FnSig {
			params: list,
			..sig
		})
	}

	/// Whether a function declared in a type takes `self`.
	pub(super) fn is_method(&self, function: SymbolId) -> bool {
		// a function of a type of another file (modules.rs)
		if let Some(&method) = self.modules.methods.get(&function) {
			return method;
		}
		let decl = self.a.symbols.symbol(function).decl_node;
		self.a.function(decl).is_some_and(|info| {
			info.params.first().is_some_and(|param| {
				self.a.symbols.symbol(param.symbol).kind == SymbolKind::SelfParameter
			})
		})
	}

	/// The members of an array or a growing array (types brief §5.1, §5.2):
	/// `length` and the methods. `CL007` for `size`, `CL040` for a removal on
	/// a growing array.
	fn array_member(
		&mut self,
		member: NodeId,
		kind: CollKind,
		item: TypeId,
		text: &str,
	) -> Option<(TypeId, MemberKind)> {
		if text == "size" {
			let message = format!(
				"a `{}` has a `length`, not a `size`: its elements have positions",
				kind.name()
			);
			let diagnostic =
				self.diagnostic(&crate::diagnostics::codes::types::CL007, member, message);
			self.report(diagnostic);
			return None;
		}
		if self.growing_lacks(member, kind, text) {
			return None;
		}
		let path = format!("Array#{text}");
		let builtin = builtins::by_path(&path);
		let found = builtin
			.and_then(|_| self.array_method_type(text, item))
			.map(|ty| self.with_effects(ty, kind, text));
		let ty = match found {
			Some(ty) => {
				self.set_resolution(member, Resolution::Builtin(builtin.expect("found")));
				ty
			}
			None if builtin.is_some() && matches!(text, "sum" | "join") => {
				let shown = self.display(item);
				let (code, message) = if text == "sum" {
					(
						&crate::diagnostics::codes::types::CL024,
						format!(
							"`sum` adds numbers, and the items are `{shown}`: use `reduce` to combine them"
						),
					)
				} else {
					(
						&E_OBJ_5,
						format!(
							"`join` writes strings, and the items are `{shown}`: convert them first, ex: `items.map((item) => item.toString()).join(\", \")`"
						),
					)
				};
				let diagnostic = self.diagnostic(code, member, message);
				self.report(diagnostic);
				return None;
			}
			None => {
				let message = format!("`{text}` of arrays is not supported yet by the analyzer");
				let diagnostic = self.diagnostic(&AN001, member, message);
				self.report(diagnostic);
				return None;
			}
		};
		Some((ty, MemberKind::Builtin))
	}

	// ── writing members ─────────────────────────────────────────────────

	/// `a.b.c` as an assignment target: the root must be mutable (`E-DECL-4`,
	/// reported by `check_assignable`), the field writable from here
	/// (`E-OBJ-10` readonly, `E-OBJ-11` function, `E-OBJ-4` private).
	pub(super) fn check_field_place(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> PlaceInfo {
		// `library.counter.count = 10`: through namespaces (modules.rs)
		if let Some(place) = self.module_place(n, target, members) {
			return place;
		}
		let root = self.check_root_place(target);
		self.memo(n, |w| {
			let mut current = root.ty;
			let count = members.len();
			for (position, member) in members.iter().enumerate() {
				let Some((ty, kind)) = w.member_of(member, current) else {
					return TypeId::UNKNOWN;
				};
				if position + 1 == count {
					let text = w.tree.text(member.index()).to_string();
					let owner = w.display(current);
					let inside = w.is_inside_type(current);
					let problem: Option<(Code, String)> = match kind {
						MemberKind::Field(flags)
							if flags.contains(FieldFlags::READONLY) && !inside =>
						{
							Some((
								&E_OBJ_10,
								format!(
									"`{text}` is `readonly`: only the functions of `{owner}` can change it"
								),
							))
						}
						MemberKind::Function => Some((
							&E_OBJ_11,
							format!("`{text}` is a function of `{owner}`: it can't be replaced"),
						)),
						MemberKind::Builtin => {
							Some((&E_OBJ_11, format!("`{text}` can't be assigned")))
						}
						MemberKind::Field(_) => None,
					};
					if let Some((code, message)) = problem {
						let diagnostic = w.diagnostic(code, member, message);
						w.report(diagnostic);
					}
				}
				current = ty;
			}
			current
		});
		PlaceInfo {
			symbol: root.symbol,
			place: None,
			ty: self.type_of(n),
			assignable: root.assignable,
			below: true,
		}
	}

	/// `items[index]` as an assignment target.
	pub(super) fn check_index_place(
		&mut self,
		n: NodeId,
		target: NodeId,
		index: NodeId,
	) -> PlaceInfo {
		let root = self.check_root_place(target);
		let ty = match self.map_key(root.ty) {
			// `map[key] = value` adds or replaces: it never fails
			Some(key) => {
				let index_ty = self.check(index, Some(key));
				self.sink(index, index_ty, key);
				self.memo(n, |w| w.index_type(n, target, root.ty, index, index_ty))
			}
			None => {
				let index_ty = self.check(index, Some(TypeId::INTEGER));
				let ty = self.memo(n, |w| w.index_type(n, target, root.ty, index, index_ty));
				self.check_index_bounds(n, target, index);
				ty
			}
		};
		PlaceInfo {
			symbol: root.symbol,
			place: None,
			ty,
			assignable: root.assignable,
			below: true,
		}
	}

	/// The place a field or element is written into: a place itself when it
	/// is a name, a field or an element; a temporary value otherwise.
	fn check_root_place(&mut self, target: NodeId) -> PlaceInfo {
		match self.view(target) {
			View::Name(_) | View::Access { .. } | View::Index { .. } => {
				let mut place = self.check_place(target);
				// a parameter is changed through `mutates` (M2: effects)
				if let Some(symbol) = place.symbol
					&& matches!(
						self.a.symbols.symbol(symbol).kind,
						SymbolKind::Parameter { .. }
							| SymbolKind::SelfParameter
							| SymbolKind::Requirement { .. }
					) {
					place.assignable = true;
				}
				place
			}
			_ => {
				let ty = self.check(target, None);
				PlaceInfo {
					ty,
					..PlaceInfo::unknown()
				}
			}
		}
	}

	/// The type of `target[index]` (type part): the item of an array, the
	/// value of a map (whose key the caller sinks), a `Character` of a string.
	pub(super) fn index_type(
		&mut self,
		n: NodeId,
		target: NodeId,
		target_ty: TypeId,
		index: NodeId,
		index_ty: TypeId,
	) -> TypeId {
		if self.s.types.is_recovering(target_ty) || target_ty.is_unset() {
			return TypeId::UNKNOWN;
		}
		if let TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, value, _) =
			self.s.types.kind(target_ty)
		{
			return value;
		}
		// `pair[0]`: tuples.rs
		if let Some(elements) = self.tuple_items(target_ty) {
			return self.tuple_index_type(index, &elements, index_ty);
		}
		// `items[1..3]`, `text[..5]`: a slice (values brief §12)
		if index_ty == TypeId::RANGE {
			return match self.s.types.kind(target_ty) {
				TypeKind::Collection(CollKind::Array, ..) => target_ty,
				TypeKind::String | TypeKind::StringLit(_) => TypeId::STRING,
				_ => {
					let message = format!("a `{}` can't be sliced", self.display(target_ty));
					let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
					self.report(diagnostic);
					TypeId::UNKNOWN
				}
			};
		}
		if !self.s.types.is_recovering(index_ty) && self.s.types.as_int(index_ty).is_none() {
			let message = format!("an index is an integer, not a `{}`", self.display(index_ty));
			let diagnostic = self.diagnostic(&E_TYPE_1, index, message);
			self.report(diagnostic);
		}
		match self.s.types.kind(target_ty) {
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => item,
			TypeKind::Collection(kind @ (CollKind::Set | CollKind::GrowingSet), ..) => {
				let message = format!(
					"a `{}` has no positions: it can't be indexed; test an element with `includes`",
					kind.name()
				);
				let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			TypeKind::String | TypeKind::StringLit(_) => TypeId::CHARACTER,
			TypeKind::Union(list) if self.s.types.list_items(list).contains(&TypeId::NONE) => {
				let message = format!(
					"this value may be `none` (`{}`): check it first",
					self.display(target_ty)
				);
				let diagnostic = self.diagnostic(&E_OPT_1, target, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			TypeKind::Collection(..) => {
				let message = format!(
					"indexing a `{}` is not supported yet by the analyzer",
					self.display(target_ty)
				);
				let diagnostic = self.diagnostic(&AN001, n, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			_ => {
				let message = format!("a `{}` can't be indexed", self.display(target_ty));
				let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
		}
	}

	/// `target .= { field = value, … }`: a list of field assignments (values
	/// brief §11). The right side is a literal written in place (`E-OBJ-13`).
	pub(super) fn check_update_fields(
		&mut self,
		n: NodeId,
		target: NodeId,
		value: NodeId,
	) -> TypeId {
		let place = self.check_root_place(target);
		let tree = Arc::clone(&self.tree);
		let fields = match syntax::view(&tree, value) {
			View::Object { items } => self.literal_fields(items),
			_ => None,
		};
		let Some(fields) = fields else {
			self.check(value, None);
			return self.memo(n, |w| {
				let diagnostic = w.diagnostic(
					&E_OBJ_13,
					value,
					"`.=` takes an object literal written in place, ex: `settings .= { port = 8080 }`",
				);
				w.report(diagnostic);
				TypeId::VOID
			});
		};
		let object = self.literal_target(place.ty).map(|(object, _)| object);
		let declared = object
			.map(|object| self.object_declaration(object).1)
			.unwrap_or_default();
		for field in &fields {
			let text = self.tree.name(field.name);
			let expected = declared
				.iter()
				.find(|declared| self.s.types.syms.resolve(declared.name) == text)
				.map(|declared| declared.ty);
			let actual = self.check(field.value, expected);
			if let Some(expected) = expected {
				self.sink(field.value, actual, expected);
			}
		}
		// the write: an effect on the place (effects.rs)
		if place.assignable {
			self.write_effect(target, n);
		}
		self.memo(n, |w| {
			w.check_assignable(
				target,
				&PlaceInfo {
					below: true,
					..place
				},
			);
			let Some(object) = object else {
				if !w.s.types.is_recovering(place.ty) {
					let message = format!(
						"`.=` updates the fields of an object, not of a `{}`",
						w.display(place.ty)
					);
					let diagnostic = w.diagnostic(&E_TYPE_1, target, message);
					w.report(diagnostic);
				}
				return TypeId::VOID;
			};
			let inside = w.is_inside_type(object);
			let owner = w.display(object);
			let mut given = Vec::new();
			for field in &fields {
				let text = w.tree.name(field.name).to_string();
				let problem: Option<(Code, String)> = if given.contains(&text) {
					Some((&E_OBJ_3, format!("the field `{text}` is given twice")))
				} else {
					match declared
						.iter()
						.find(|declared| w.s.types.syms.resolve(declared.name) == text)
					{
						None => Some((&E_OBJ_5, format!("a `{owner}` has no field `{text}`"))),
						Some(declared)
							if declared.flags.contains(FieldFlags::PRIVATE) && !inside =>
						{
							Some((
								&E_OBJ_4,
								format!(
									"`{text}` is private: only the functions of `{owner}` can change it"
								),
							))
						}
						Some(declared)
							if declared.flags.contains(FieldFlags::READONLY) && !inside =>
						{
							Some((
								&E_OBJ_10,
								format!(
									"`{text}` is `readonly`: only the functions of `{owner}` can change it"
								),
							))
						}
						Some(_) => None,
					}
				};
				given.push(text);
				if let Some((code, message)) = problem {
					let diagnostic = w.diagnostic(code, field.item, message);
					w.report(diagnostic);
				}
			}
			TypeId::VOID
		})
	}

	/// `readonly field: T` outside a type.
	pub(super) fn check_readonly(&mut self, n: NodeId, _field: NodeId) -> TypeId {
		self.unsupported(n, "`readonly` outside a type")
	}

	/// `E-DECL-4`: a field or an element written below an immutable place.
	pub(super) fn report_immutable_root(&mut self, target: NodeId, place: &PlaceInfo) {
		let name = place.symbol.map_or_else(
			|| self.tree.text(target.index()).to_string(),
			|symbol| {
				let name = self.a.symbols.symbol(symbol).name;
				self.s.types.syms.resolve(name).to_string()
			},
		);
		let message = format!(
			"`{name}` is immutable: declare it with `mutable` to change its fields or elements"
		);
		let diagnostic = self.diagnostic(&E_DECL_4, target, message);
		self.report(diagnostic);
	}
}
