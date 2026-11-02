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
	FN042, TY010, TY013, TY050, TY051, TY080, TY081, TY082, TY083,
};
use crate::diagnostics::codes::values::{E_DECL_9, E_OBJ_12};
use crate::ids::{DeclKey, NodeId, ScopeId, Sym, SymbolId, TypeId};
use crate::symbols::{DeclState, ScopeKind, SymbolKind};
use crate::syntax::{self, Name, NodeList, View};
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
		let text = tree.name(name).to_string();
		match syntax::view(&tree, value) {
			View::Object { items } => self.object_type_decl(n, symbol, &text, value, items),
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
				let ty = self.check_type_expr(value);
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
			.collect();
		self.a.symbols.symbol_mut(symbol).ty = ty;
		self.decls_by_type.entry(ty).or_default().push(n);
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
	) {
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(body, Some(parent), ScopeKind::Namespace);
		self.body.scope = scope;
		let parts = self.object_type_parts(items, true);
		let private = parts
			.fields
			.iter()
			.any(|field| field.flags.contains(FieldFlags::PRIVATE));
		let nominal = private.then(|| {
			let key = self.nominal_key(name);
			let args = self.s.types.list(&[]);
			let members = self.s.types.intern_sym_list(&[]);
			self.s.types.intern_nominal(NominalInfo {
				key,
				args,
				kind: NominalKind::Private,
				members,
			})
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
			let ty = self.s.types.object(&entries, nominal);
			self.s.types.name_type(ty, name);
			ty
		};
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
		for item in items.iter() {
			let (field, readonly) = match syntax::view(&tree, item) {
				View::Readonly(inner) => (inner, true),
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
				View::Error => continue,
				_ => (item, false),
			};
			let View::Assignment {
				target,
				annotation,
				value: default,
			} = syntax::view(&tree, field)
			else {
				self.unsupported(item, "this member of a type");
				parts.incomplete = true;
				continue;
			};
			let View::Name(name) = syntax::view(&tree, target) else {
				self.unsupported(item, "this member of a type");
				parts.incomplete = true;
				continue;
			};
			let text = tree.name(name);
			let declared = annotation.map(|annotation| self.check_type_expr(annotation));
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
			if ty == TypeId::VOID {
				let diagnostic = self.diagnostic(
					&E_DECL_9,
					annotation.unwrap_or(field),
					"`Void` is not a value type: a field can't hold it",
				);
				self.report(diagnostic);
			}
			let sym = self.s.types.syms.intern(text);
			if parts.fields.iter().any(|field| field.name == sym) {
				let diagnostic = self.diagnostic(
					&TY010,
					target,
					format!("the field `{text}` is declared twice"),
				);
				self.report(diagnostic);
				continue;
			}
			let mut flags = FieldFlags::empty();
			flags.set(FieldFlags::READONLY, readonly);
			flags.set(FieldFlags::PRIVATE, text.starts_with('#'));
			self.set_type(target, ty);
			self.set_type(field, ty);
			parts.fields.push(FieldDecl {
				node: field,
				name: sym,
				ty,
				default,
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
		let key = self.nominal_key(name);
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
						}
						_ => {
							let diagnostic = self.diagnostic(
								&TY083,
								value,
								"the value of a member is a non-negative integer literal",
							);
							self.report(diagnostic);
						}
					}
					TypeId::UNSET
				}
				(_, Some(definition)) => self.check_type_expr(definition),
			};
			if self.s.types.is_literal(payload) && payloads.contains(&payload) {
				let message = format!(
					"two members accept `{}`: a literal belongs to one member",
					self.display(payload)
				);
				let diagnostic = self.diagnostic(&TY051, target, message);
				self.report(diagnostic);
			}
			payloads.push(payload);
		}
		self.s.types.set_payloads(nominal, payloads);
		self.finish_type(n, symbol, ty, ObjectTypeParts::default(), None);
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
		for decl in self.declarations_of(ty) {
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
		None
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
