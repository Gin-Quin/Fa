//! Type declarations (types brief §3.1, §3.2, §3.5, §3.8): aliases, object
//! types (fields, defaults, `readonly`, `#private`, methods and associated
//! functions), named unions, `Errors` types and enumerations.
//!
//! An object type is structural: its `TypeId` is its shape, so two
//! declarations with the same fields are the same type, and the methods of a
//! value are found through the declarations of its type (`decls_by_type`).
//! A type with `#private` fields, a named union, an `Errors` type and an
//! enumeration are nominal: one identity per declaration.

use std::sync::Arc;

use crate::analysis::{FieldDecl, TypeDecl};
use crate::check::Walker;
use crate::diagnostics::codes::types::{
	FN042, TY010, TY013, TY050, TY051, TY060, TY080, TY081, TY082, TY083,
};
use crate::diagnostics::codes::values::{E_DECL_9, E_OBJ_12};
use crate::ids::{DeclKey, NodeId, ScopeId, Sym, SymbolId, TypeId};
use crate::symbols::{DeclState, ScopeKind, SymbolKind};
use crate::syntax::{self, BindingKind, Name, NodeList, View};
use crate::types::{FieldEntry, FieldFlags, NominalInfo, NominalKind};

/// The nominal type constructors written `Name { … }` in a type declaration.
fn constructor_kind(name: &str) -> Option<NominalKind> {
	match name {
		"Union" => Some(NominalKind::Union),
		"Errors" => Some(NominalKind::Errors),
		"Enum" => Some(NominalKind::Enum),
		_ => None,
	}
}

/// An object type being built: its fields in declaration order and its functions.
#[derive(Default)]
pub(super) struct ObjectTypeParts {
	/// The fields, in declaration order.
	pub fields: Vec<FieldDecl>,
	/// The `function` declarations of the body.
	pub functions: Vec<NodeId>,
	/// Whether a member couldn't be analyzed (the type is then `UNKNOWN`).
	pub incomplete: bool,
}

impl Walker<'_> {
	/// `type Name = …` (hoisted, analyzed once): an alias, an object type, or
	/// a nominal constructor. Sets the symbol's type and records the
	/// declaration in `Analysis::type_decls`.
	pub(super) fn declare_type(&mut self, n: NodeId, name: Name, value: NodeId) -> TypeId {
		let Some(symbol) = self.a.symbols.symbol_of_decl(n) else {
			return TypeId::VOID;
		};
		let tree = Arc::clone(&self.tree);
		let text = self.decl_text(n, name);
		match syntax::view(&tree, value) {
			View::Object { items } => self.object_type_decl(n, symbol, &text, value, items, false),
			// `CStruct { … }`: a nominal object type with the C layout (foreign.rs)
			View::TypedObject { target, members }
				if matches!(syntax::view(&tree, target), View::Name(constructor)
					if tree.name(constructor) == "CStruct" && !self.is_declared(constructor)) =>
			{
				self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
				self.object_type_decl(n, symbol, &text, value, members, true);
			}
			View::TypedObject { target, members }
				if matches!(syntax::view(&tree, target), View::Name(constructor)
					if constructor_kind(tree.name(constructor)).is_some()
						&& !self.is_declared(constructor)) =>
			{
				let View::Name(constructor) = syntax::view(&tree, target) else {
					unreachable!("checked by the guard");
				};
				let kind = constructor_kind(tree.name(constructor)).expect("checked by the guard");
				self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
				self.nominal_decl(n, symbol, &text, kind, members);
			}
			_ => {
				let ty = self.check_owned_type_expr(value);
				// `type Permissions = Fields { … }`: named in messages (fields.rs)
				if matches!(self.s.types.kind(ty), crate::types::TypeKind::Fields(_)) {
					self.s.types.name_type(ty, &text);
				}
				self.finish_type(n, symbol, ty, ObjectTypeParts::default(), None);
			}
		}
		TypeId::VOID
	}

	/// Records a declared type.
	fn finish_type(
		&mut self,
		n: NodeId,
		symbol: SymbolId,
		ty: TypeId,
		parts: ObjectTypeParts,
		scope: Option<ScopeId>,
	) {
		let functions = parts
			.functions
			.iter()
			.filter_map(|&function| self.a.symbols.symbol_of_decl(function))
			.filter(|&symbol| self.a.symbols.symbol(symbol).kind == SymbolKind::Function)
			.collect();
		self.finish_recursive(n, symbol, ty);
		self.a.symbols.symbol_mut(symbol).ty = ty;
		self.decls_by_type.entry(ty).or_default().push(n);
		// its methods are found from other files too (modules.rs)
		let key = self.symbol_key(symbol);
		self.s.record_type_origin(ty, key);
		self.a.type_decls.insert(
			n,
			TypeDecl {
				symbol,
				ty,
				fields: parts.fields,
				functions,
				scope,
				generics: self.generic_params.get(&n).cloned().unwrap_or_default(),
			},
		);
	}

	/// `type Point = { x: Integer, … }`: the fields, then the functions, once
	/// the type is known (they may use it).
	fn object_type_decl(
		&mut self,
		n: NodeId,
		symbol: SymbolId,
		name: &str,
		body: NodeId,
		items: NodeList<'_>,
		c_struct: bool,
	) {
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(body, Some(parent), ScopeKind::Namespace);
		self.body.scope = scope;
		self.relations.declaring.push(n);
		let parts = self.object_type_parts(items, true);
		self.relations.declaring.pop();
		let private = parts
			.fields
			.iter()
			.any(|field| field.flags.contains(FieldFlags::PRIVATE));
		if c_struct {
			self.check_c_struct_fields(&parts.fields);
		}
		// `destroy`: a resource, move-only (foreign.rs)
		let tree = Arc::clone(&self.tree);
		let destroy = parts.functions.iter().copied().find(|&function| {
			matches!(syntax::view(&tree, function), View::Function { name, .. } if tree.name(name) == "destroy")
		});
		let resource = destroy.is_some_and(|function| self.check_destroy_declaration(function));
		let kind = if c_struct {
			Some(NominalKind::CStruct)
		} else if resource {
			Some(NominalKind::Resource)
		} else if private {
			Some(NominalKind::Private)
		} else {
			None
		};
		let nominal = kind.map(|kind| {
			let key = self.type_nominal_key(n, name);
			let parameters = self.generic_params.get(&n).cloned().unwrap_or_default();
			let arguments: Vec<_> = parameters
				.iter()
				.map(|&parameter| {
					self.s
						.types
						.intern(crate::types::TypeKind::Param(parameter))
				})
				.collect();
			let args = self.s.types.list(&arguments);
			let members = self.s.types.intern_sym_list(&[]);
			let nominal = self.s.types.intern_nominal(NominalInfo {
				key,
				args,
				kind,
				members,
			});
			if !parameters.is_empty() {
				self.s.types.set_generic_nominal(key, nominal, parameters);
			}
			nominal
		});
		let ty = if parts.incomplete {
			TypeId::UNKNOWN
		} else {
			let entries: Vec<FieldEntry> = parts
				.fields
				.iter()
				.map(|field| FieldEntry {
					name: field.name,
					ty: field.ty,
					flags: field.flags,
				})
				.collect();
			let ty = match nominal {
				Some(nominal) if c_struct => self.s.types.object_in_order(&entries, nominal),
				_ => self.s.types.object(&entries, nominal),
			};
			self.s.types.name_type(ty, name);
			// a generic type: its substitutions are named after their
			// arguments (`Pair<Integer, String>`)
			if let Some(params) = self.generic_params.get(&n).cloned()
				&& !params.is_empty()
			{
				let params: Vec<TypeId> = params
					.into_iter()
					.map(|param| self.s.types.intern(crate::types::TypeKind::Param(param)))
					.collect();
				self.s.types.record_application(ty, name, &params);
			}
			ty
		};
		// its relation fields, found by type (relations.rs)
		self.register_relation_fields(n, ty);
		self.type_scopes.insert(scope, (ty, n));
		// the functions see the type: the declaration is no longer a cycle
		let frame = self.decl_stack.len().saturating_sub(1) as u32;
		let symbol_data = self.a.symbols.symbol_mut(symbol);
		symbol_data.ty = ty;
		symbol_data.state = DeclState::Body { frame };
		let functions = parts.functions.clone();
		self.finish_type(n, symbol, ty, parts, Some(scope));
		for function in functions {
			self.check_stmt(function);
			self.a.function_owners.insert(function, n);
			if let Some(info) = self.a.function(function) {
				self.a.function_owners.insert(info.lambda, n);
			}
		}
		self.body.scope = parent;
	}

	/// The members of an object type: fields (with their defaults), and, in a
	/// declaration, functions declared `Pending` in the current scope (`FN042`
	/// in an anonymous type). `TY010` for a field declared twice, `E-OBJ-12`
	/// for a field and a function with one name.
	pub(super) fn object_type_parts(
		&mut self,
		items: NodeList<'_>,
		declaration: bool,
	) -> ObjectTypeParts {
		let tree = Arc::clone(&self.tree);
		let mut parts = ObjectTypeParts::default();
		let mut function_names: Vec<(Sym, NodeId)> = Vec::new();
		// the fields copied by spreads (types brief §3.10)
		let mut spread_names: Vec<Sym> = Vec::new();
		// the types declared inside (`type Town = { type Person = … }`) are
		// hoisted: the fields may name them (relations.md)
		if declaration {
			for item in items.iter() {
				if let View::TypeDecl { name, .. } = syntax::view(&tree, item)
					&& !name.is_empty()
					&& self.a.symbols.symbol_of_decl(item).is_none()
				{
					let symbol = self.declare(item, name, SymbolKind::Type);
					self.a.symbols.symbol_mut(symbol).state = DeclState::Pending;
				}
			}
		}
		for item in items.iter() {
			let mut reactive = false;
			let (field, readonly) = match syntax::view(&tree, item) {
				View::Readonly(inner) => (inner, true),
				View::Link(_) => {
					self.unsupported_with_message(item, "stored `link` fields are not implemented yet: use a relation with its reverse side until then");
					parts.incomplete = true;
					continue;
				}
				// `reactive count: Integer` (advanced brief §11.2)
				View::Declaration {
					kind: BindingKind::Reactive,
					..
				} => {
					reactive = true;
					(item, false)
				}
				View::Spread(inner) => {
					let before = parts.fields.len();
					if !self.spread_type_fields(item, inner, &mut parts.fields) {
						parts.incomplete = true;
					}
					spread_names.extend(parts.fields[before..].iter().map(|field| field.name));
					continue;
				}
				View::Function { name, .. } => {
					if !declaration {
						let diagnostic = self.diagnostic(
							&FN042,
							item,
							"functions can only be declared in a `type` declaration",
						);
						self.report(diagnostic);
						continue;
					}
					if name.is_empty() {
						continue;
					}
					if self.a.symbols.symbol_of_decl(item).is_none() {
						let symbol = self.declare(item, name, SymbolKind::Function);
						self.a.symbols.symbol_mut(symbol).state = DeclState::Pending;
					}
					let sym = self.s.types.syms.intern(tree.name(name));
					function_names.push((sym, item));
					parts.functions.push(item);
					continue;
				}
				// a nested type: analyzed with the functions (or on demand)
				View::TypeDecl { .. } if declaration => {
					parts.functions.push(item);
					continue;
				}
				View::Error => continue,
				_ => (item, false),
			};
			let (target, name, annotation, default) = match syntax::view(&tree, field) {
				View::Assignment {
					target,
					annotation,
					value: default,
				} => {
					let View::Name(name) = syntax::view(&tree, target) else {
						self.unsupported(item, "this member of a type");
						parts.incomplete = true;
						continue;
					};
					(target, name, annotation, default)
				}
				View::Declaration {
					kind: BindingKind::Reactive,
					name,
					annotation,
					value,
				} if !name.is_empty() => (field, name, annotation, value),
				_ => {
					self.unsupported(item, "this member of a type");
					parts.incomplete = true;
					continue;
				}
			};
			let text = tree.name(name);
			// a side of a relation (relations.rs)
			let relation = match annotation {
				Some(annotation) if declaration => {
					let sym = self.s.types.syms.intern(text);
					self.relation_field(field, sym, annotation, default, items)
				}
				_ => None,
			};
			let declared = match relation {
				Some(ty) => Some(ty),
				None => annotation.map(|annotation| self.check_type_expr(annotation)),
			};
			if relation.is_none()
				&& let Some(annotation) = annotation
			{
				self.check_reference_annotation(annotation);
			}
			// a relation side has no initializer (REL013, reported)
			let default = if relation.is_some() { None } else { default };
			let field_names: Vec<&str> = items
				.iter()
				.filter_map(|other| {
					let other = match syntax::view(&tree, other) {
						View::Readonly(inner) => inner,
						_ => other,
					};
					match syntax::view(&tree, other) {
						View::Assignment { target, .. } => match syntax::view(&tree, target) {
							View::Name(other_name) => Some(tree.name(other_name)),
							_ => None,
						},
						View::Declaration {
							kind: BindingKind::Reactive,
							name,
							..
						} => Some(tree.name(name)),
						_ => None,
					}
				})
				.chain(["self"])
				.collect();
			let field_use = default.and_then(|default| self.find_name_use(default, &field_names));
			if let Some((use_node, used)) = &field_use {
				let message = format!(
					"a default value can't use `{used}`: it is computed before the other fields exist"
				);
				let diagnostic = self.diagnostic(&TY013, *use_node, message);
				self.report(diagnostic);
			}
			self.quiet += u32::from(field_use.is_some());
			let ty = match (declared, default) {
				(Some(declared), Some(default)) => {
					let actual = self.check(default, Some(declared));
					self.sink(default, actual, declared);
					declared
				}
				(Some(declared), None) => declared,
				(None, Some(default)) => {
					let actual = self.check(default, None);
					self.s.types.widen(actual)
				}
				(None, None) => TypeId::UNKNOWN,
			};
			self.quiet -= u32::from(field_use.is_some());
			if let Some(written) = annotation
				.or(default)
				.and_then(|source| self.written_origin(source))
			{
				self.a.written_types.insert(field, written);
			}
			self.check_stored_iterator_type(annotation.unwrap_or(field), ty, "stored in a field");
			if ty == TypeId::VOID {
				let diagnostic = self.diagnostic(
					&E_DECL_9,
					annotation.unwrap_or(field),
					"`Void` is not a value type: a field can't hold it",
				);
				self.report(diagnostic);
			}
			let sym = self.s.types.syms.intern(text);
			let mut flags = FieldFlags::empty();
			flags.set(FieldFlags::READONLY, readonly);
			flags.set(FieldFlags::REACTIVE, reactive);
			flags.set(FieldFlags::RELATION, relation.is_some());
			flags.set(FieldFlags::PRIVATE, text.starts_with('#'));
			flags.set(
				FieldFlags::METHOD,
				annotation
					.is_some_and(|annotation| super::type_expr::is_method_type(&tree, annotation)),
			);
			if spread_names.contains(&sym) {
				// a field a spread already gave: once if the same, `TY101` otherwise
				self.set_type(target, ty);
				self.set_type(field, ty);
				let decl = FieldDecl {
					node: field,
					name: sym,
					ty,
					default,
					constant: None,
					flags,
				};
				self.add_field(target, decl, &mut parts.fields);
				continue;
			}
			if parts.fields.iter().any(|field| field.name == sym) {
				let diagnostic = self.diagnostic(
					&TY010,
					target,
					format!("the field `{text}` is declared twice"),
				);
				self.report(diagnostic);
				continue;
			}
			self.set_type(target, ty);
			self.set_type(field, ty);
			parts.fields.push(FieldDecl {
				node: field,
				name: sym,
				ty,
				default,
				constant: None,
				flags,
			});
		}
		for (sym, function) in function_names {
			if parts.fields.iter().any(|field| field.name == sym) {
				let text = self.s.types.syms.resolve(sym).to_string();
				let diagnostic = self.diagnostic(
					&E_OBJ_12,
					function,
					format!("`{text}` is both a field and a function of this type: rename one"),
				);
				self.report(diagnostic);
			}
		}
		parts
	}

	/// `type Shape = Union { … }`, `Errors { … }`, `Enum { … }`: a new
	/// nominal identity, then the data of each member (which may name the
	/// type itself).
	fn nominal_decl(
		&mut self,
		n: NodeId,
		symbol: SymbolId,
		name: &str,
		kind: NominalKind,
		members: NodeList<'_>,
	) {
		let tree = Arc::clone(&self.tree);
		let mut names: Vec<Sym> = Vec::new();
		let mut definitions: Vec<(NodeId, Option<NodeId>)> = Vec::new();
		for member in members.iter() {
			let (target, definition) = match syntax::view(&tree, member) {
				View::Name(_) => (member, None),
				View::Assignment {
					target,
					annotation: None,
					value: Some(value),
				} => (target, Some(value)),
				View::Error => continue,
				_ => {
					self.unsupported(member, "this member");
					continue;
				}
			};
			let View::Name(member_name) = syntax::view(&tree, target) else {
				self.unsupported(member, "this member");
				continue;
			};
			let sym = self.s.types.syms.intern(tree.name(member_name));
			if names.contains(&sym) {
				let code = if kind == NominalKind::Enum {
					&TY080
				} else {
					&TY050
				};
				let diagnostic = self.diagnostic(
					code,
					target,
					format!("the member `{}` is declared twice", tree.name(member_name)),
				);
				self.report(diagnostic);
				continue;
			}
			names.push(sym);
			definitions.push((target, definition));
		}
		let key = self.type_nominal_key(n, name);
		// a generic union: its declaration has its parameters as arguments
		let params = self.generic_params.get(&n).cloned().unwrap_or_default();
		let param_types: Vec<TypeId> = params
			.iter()
			.map(|&param| self.s.types.intern(crate::types::TypeKind::Param(param)))
			.collect();
		let args = self.s.types.list(&param_types);
		let member_list = self.s.types.intern_sym_list(&names);
		let nominal = self.s.types.intern_nominal(NominalInfo {
			key,
			args,
			kind,
			members: member_list,
		});
		if !params.is_empty() {
			self.s.types.set_generic_nominal(key, nominal, params);
		}
		let ty = self.s.types.nominal_type(nominal);
		let frame = self.decl_stack.len().saturating_sub(1) as u32;
		let symbol_data = self.a.symbols.symbol_mut(symbol);
		symbol_data.ty = ty;
		symbol_data.state = DeclState::Body { frame };
		if kind == NominalKind::Enum {
			let pinned = definitions
				.iter()
				.filter(|(_, value)| value.is_some())
				.count();
			if pinned != 0 && pinned != definitions.len() {
				let diagnostic = self.diagnostic(
					&TY081,
					n,
					"either every member of an enumeration has a value, or none",
				);
				self.report(diagnostic);
			}
		}
		let mut pinned_values: Vec<ethnum::I256> = Vec::new();
		let mut payloads = Vec::with_capacity(definitions.len());
		for (index, (target, definition)) in definitions.into_iter().enumerate() {
			let member_ty = if kind == NominalKind::Enum {
				let mask = self.s.types.single_mask(index as u16);
				self.s.types.enumeration(nominal, mask)
			} else {
				self.s.types.member(nominal, index as u16)
			};
			self.set_type(target, member_ty);
			self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
			let payload = match (kind, definition) {
				(_, None) => TypeId::UNSET,
				// pinned values of an enumeration are not types
				(NominalKind::Enum, Some(value)) => {
					self.check(value, Some(TypeId::INTEGER));
					match self.constant_integer(value) {
						Some(pinned) if pinned >= 0 => {
							if pinned_values.contains(&pinned) {
								let diagnostic = self.diagnostic(
									&TY082,
									value,
									format!("{pinned} is already the value of another member"),
								);
								self.report(diagnostic);
							}
							pinned_values.push(pinned);
							// the value, as the member's literal (the backends,
							// `@typeToSchema`, other files read it)
							let storage = crate::types::IntStorage::Default;
							self.s.types.int_range(pinned, pinned, storage)
						}
						_ => {
							let diagnostic = self.diagnostic(
								&TY083,
								value,
								"the value of a member is a non-negative integer literal",
							);
							self.report(diagnostic);
							TypeId::UNSET
						}
					}
				}
				(_, Some(definition)) => {
					let payload = self.check_type_expr(definition);
					// an object written inline: the order of its fields
					let tree = std::sync::Arc::clone(&self.tree);
					if let View::Object { items } = syntax::view(&tree, definition)
						&& matches!(
							self.s.types.kind(payload),
							crate::types::TypeKind::Object(_)
						) {
						let names: Vec<String> = items
							.iter()
							.filter_map(|item| match syntax::view(&tree, item) {
								View::Assignment { target, .. } => {
									match syntax::view(&tree, target) {
										View::Name(name) => Some(tree.name(name).to_string()),
										_ => None,
									}
								}
								View::Name(name) => Some(tree.name(name).to_string()),
								_ => None,
							})
							.collect();
						let names: Vec<crate::ids::Sym> = names
							.iter()
							.map(|name| self.s.types.syms.intern(name))
							.collect();
						self.a.inline_field_orders.entry(payload).or_insert(names);
					}
					payload
				}
			};
			// (a pinned value of an enumeration is `TY082` above)
			if kind != NominalKind::Enum
				&& self.s.types.is_literal(payload)
				&& payloads.contains(&payload)
			{
				let message = format!(
					"two members accept `{}`: a literal belongs to one member",
					self.display(payload)
				);
				let diagnostic = self.diagnostic(&TY051, target, message);
				self.report(diagnostic);
			}
			if kind == NominalKind::Errors
				&& let Some(definition) = definition
			{
				self.error_payloads.push((definition, payload));
			}
			payloads.push(payload);
		}
		self.s.types.set_payloads(nominal, payloads);
		self.finish_type(n, symbol, ty, ObjectTypeParts::default(), None);
	}

	// ── recursive types (types brief §3.6) ───────────────────────────────

	/// The type of a `type` declaration named inside its own type (or inside
	/// a declaration it depends on): a recursive reference, `TypeKind::Rec`,
	/// completed when the declaration is built ([`Walker::finish_recursive`]).
	/// A generic declaration refers to itself with its own parameters.
	pub(super) fn recursive_reference(&mut self, symbol: SymbolId) -> TypeId {
		if let Some(&ty) = self.rec_references.get(&symbol) {
			return ty;
		}
		let data = self.a.symbols.symbol(symbol);
		let enclosing_scope = data.scope;
		let name = self.s.types.syms.resolve(data.name).to_string();
		let object = matches!(
			self.view(data.decl_node),
			View::TypeDecl { value, .. } if matches!(self.view(value), View::Object { .. })
		);
		let mut params = self.type_generics(symbol).unwrap_or_default();
		// A nested recursive type also captures its enclosing type parameters.
		// They must be arguments of the recursive edge, so instantiating the
		// owner substitutes every traversal back into that nested type.
		let mut scope = Some(enclosing_scope);
		while let Some(current) = scope {
			let current = self.a.symbols.scope(current);
			for &symbol in &current.symbols {
				let symbol = self.a.symbols.symbol(symbol);
				if symbol.kind == SymbolKind::TypeParameter
					&& let crate::types::TypeKind::Param(param) = self.s.types.kind(symbol.ty)
					&& !params.contains(&param)
				{
					params.push(param);
				}
			}
			scope = current.parent;
		}
		let args: Vec<TypeId> = params
			.iter()
			.map(|&param| self.s.types.intern(crate::types::TypeKind::Param(param)))
			.collect();
		let key = self.nominal_key(&name);
		let ty = self.s.types.recursive_reference(key, &name, &args, object);
		if !params.is_empty() {
			self.s.types.set_rec_params(key, ty, params);
		}
		self.rec_references.insert(symbol, ty);
		ty
	}

	/// Completes the recursive reference of a declaration that named itself:
	/// its target is the declared type. A cycle made only of object fields
	/// and tuple items (no union, optional or collection on the way) has no
	/// finite value: `TY060`.
	fn finish_recursive(&mut self, n: NodeId, symbol: SymbolId, ty: TypeId) {
		let Some(&reference) = self.rec_references.get(&symbol) else {
			return;
		};
		if self.s.types.is_recovering(ty) {
			return;
		}
		self.s.types.set_rec_target(reference, ty);
		let mut seen = Vec::new();
		if self.contains_inline(ty, reference, &mut seen) {
			let name = self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name);
			let message = format!(
				"`{name}` contains itself: a value would never end; make the field optional (`{name}?`), or hold it in a collection"
			);
			let name_node = match self.view(n) {
				View::TypeDecl { value, .. } => value,
				_ => n,
			};
			let diagnostic = self.diagnostic(&TY060, name_node, message);
			self.report(diagnostic);
		}
	}

	/// Whether `ty` holds `reference` inline: through object fields and tuple
	/// items only (a union member, an optional or a collection element is an
	/// indirection, which the native backend boxes).
	fn contains_inline(&self, ty: TypeId, reference: TypeId, seen: &mut Vec<TypeId>) -> bool {
		if ty == reference {
			return true;
		}
		if seen.contains(&ty) {
			return false;
		}
		seen.push(ty);
		match self.s.types.kind(ty) {
			crate::types::TypeKind::Object(shape) => self
				.s
				.types
				.shape_fields(shape)
				.iter()
				.any(|field| self.contains_inline(field.ty, reference, seen)),
			crate::types::TypeKind::Tuple(list) => self
				.s
				.types
				.list_items(list)
				.iter()
				.any(|&item| self.contains_inline(item, reference, seen)),
			crate::types::TypeKind::Rec(_) => self
				.s
				.types
				.rec_target(ty)
				.is_some_and(|target| self.contains_inline(target, reference, seen)),
			_ => false,
		}
	}

	/// The identity of a nominal type declared in this file: its module, its
	/// name, and how many of that name came before.
	pub(super) fn nominal_key(&mut self, name: &str) -> DeclKey {
		let count = self.nominal_names.entry(name.to_string()).or_insert(0);
		let disambiguator = *count;
		*count += 1;
		let module_name = self.s.module(self.file).to_string();
		let module = self.s.types.module(&module_name);
		self.s.types.decl_key(module, &[name], disambiguator)
	}

	/// The key of the nominal type a `type` declaration (node `n`) declares:
	/// a type declared inside types is keyed under their names
	/// (`Farm.Error`), as its symbol is.
	fn type_nominal_key(&mut self, n: NodeId, name: &str) -> DeclKey {
		let mut path: Vec<String> = vec![name.to_string()];
		if let Some(symbol) = self.a.symbols.symbol_of_decl(n) {
			let mut scope = Some(self.a.symbols.symbol(symbol).scope);
			while let Some(current) = scope {
				let info = self.a.symbols.scope(current);
				let (owner, parent) = (info.owner, info.parent);
				let Some(outer) = self.type_body_name(owner, parent) else {
					break;
				};
				path.push(self.s.types.syms.resolve(outer).to_string());
				scope = parent;
			}
		}
		path.reverse();
		let count = self.nominal_names.entry(path.join(".")).or_insert(0);
		let disambiguator = *count;
		*count += 1;
		let module_name = self.s.module(self.file).to_string();
		let module = self.s.types.module(&module_name);
		let names: Vec<&str> = path.iter().map(String::as_str).collect();
		self.s.types.decl_key(module, &names, disambiguator)
	}

	/// The `type` declarations whose type is `ty` (a member: its union's).
	pub(super) fn declarations_of(&mut self, ty: TypeId) -> Vec<NodeId> {
		let ty = match self.s.types.kind(ty) {
			crate::types::TypeKind::Member(nominal, _)
			| crate::types::TypeKind::EnumMember(nominal, _) => self.s.types.nominal_type(nominal),
			_ => ty,
		};
		self.decls_by_type.get(&ty).cloned().unwrap_or_default()
	}

	/// The function named `name` declared in a type whose type is `ty`.
	pub(super) fn type_function(&mut self, ty: TypeId, name: &str) -> Option<SymbolId> {
		if let Some(origin) = self.receiver_origin.clone()
			&& self.written_origin_matches(&origin, ty)
		{
			let origin = self.resolve_written_type(origin);
			self.receiver_origin = Some(origin.clone());
			return self.written_type_function(origin.declaration, name);
		}
		let mut decls = self.declarations_of(ty);
		// the type written for the receiver first (advanced brief §10.1)
		if let Some(written) = self.receiver_decl
			&& let Some(position) = decls.iter().position(|&decl| decl == written)
		{
			decls.swap(0, position);
		}
		for decl in decls {
			let Some(info) = self.a.type_decls.get(&decl) else {
				continue;
			};
			let found = info.functions.iter().copied().find(|&function| {
				self.s
					.types
					.syms
					.resolve(self.a.symbols.symbol(function).name)
					== name
			});
			if found.is_some() {
				return found;
			}
		}
		// a type declared in another file (modules.rs)
		self.external_type_function(ty, name)
	}

	fn written_origin_matches(
		&mut self,
		origin: &crate::analysis::WrittenType,
		ty: TypeId,
	) -> bool {
		let module = self.s.types.decl_key_data(origin.declaration).module;
		let declared = if module == self.s.module_id(self.file) {
			self.modules
				.keyed
				.get(&origin.declaration)
				.map(|&symbol| self.a.symbols.symbol(symbol).ty)
		} else {
			None
		};
		let declared = declared.map(|declared| match self.written_bindings(origin) {
			Some(bindings) => self.s.types.substitute(declared, &bindings),
			None => declared,
		});
		declared.is_some_and(|declared| self.s.types.unfold(declared) == self.s.types.unfold(ty))
			|| self
				.s
				.type_origins(ty, |_| true)
				.any(|known| known == origin.declaration)
	}

	fn written_type_function(
		&mut self,
		mut origin: crate::ids::DeclKey,
		name: &str,
	) -> Option<SymbolId> {
		let mut seen = rustc_hash::FxHashSet::default();
		while seen.insert(origin) {
			if self.s.types.decl_key_data(origin).module != self.s.module_id(self.file) {
				return self.external_type_function_at(origin, name);
			}
			let symbol = *self.modules.keyed.get(&origin)?;
			let declaration = self.a.symbols.symbol(symbol).decl_node;
			let info = self.a.type_decls.get(&declaration)?;
			if let Some(function) = info.functions.iter().copied().find(|&function| {
				self.s
					.types
					.syms
					.resolve(self.a.symbols.symbol(function).name)
					== name
			}) {
				return Some(function);
			}
			// A named alias inherits the methods of its explicitly named
			// target, never those of an unrelated equal structure.
			let (View::TypeDecl { value, .. } | View::ExportType { value, .. }) =
				self.view(declaration)
			else {
				return None;
			};
			origin = self.written_origin(value)?.declaration;
		}
		None
	}

	/// The type named `name` declared inside a type whose type is `ty`
	/// (`type Database = { type OpenError = Errors { … } }`): what
	/// `Self.OpenError` and `Database.OpenError` designate, in a type or as
	/// the start of a value (`Database.OpenError.Failed { code }`).
	pub(super) fn nested_type(
		&mut self,
		ty: TypeId,
		name: &str,
		use_node: NodeId,
	) -> Option<TypeId> {
		let sym = self.s.types.syms.get(name)?;
		for decl in self.declarations_of(ty) {
			let Some(scope) = self.a.type_decls.get(&decl).and_then(|info| info.scope) else {
				continue;
			};
			let found = self
				.a
				.symbols
				.scope(scope)
				.symbols
				.iter()
				.copied()
				.find(|&symbol| {
					let symbol = self.a.symbols.symbol(symbol);
					symbol.kind == SymbolKind::Type && symbol.name == sym
				});
			if let Some(symbol) = found {
				return Some(self.symbol_type(symbol, use_node));
			}
		}
		// a type declared in another file (modules.rs)
		self.external_nested_type(ty, sym)
	}

	/// The fields as declared of a type whose type is `ty`, when it has a declaration.
	pub(super) fn declared_fields(&mut self, ty: TypeId) -> Option<(NodeId, Vec<FieldDecl>)> {
		let decl = *self.declarations_of(ty).first()?;
		let info = self.a.type_decls.get(&decl)?;
		Some((decl, info.fields.clone()))
	}

	/// The type of `self` (and `Self`) in the scope `scope` or an enclosing one.
	pub(super) fn self_type(&self, scope: ScopeId) -> Option<(TypeId, NodeId)> {
		let mut current = Some(scope);
		while let Some(scope) = current {
			if let Some(&found) = self.type_scopes.get(&scope) {
				return Some(found);
			}
			current = self.a.symbols.scope(scope).parent;
		}
		None
	}
}
