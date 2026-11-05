//! The built-in type functions on keys, collections, functions and string
//! literals (types brief §3.12.4, `types/type-functions`): `Keys`,
//! `DeepKeys`, `Values`, `Record`, `Parameters`, `Return`, `Uppercase`,
//! `Lowercase`, `Capitalize`. `Pick`, `Omit`, `Partial`, `Required` and
//! `KeyType` are in `type_ops.rs`.
//!
//! A key set is a `Fields` type (`TypeKind::Fields`): `Keys<T>` gives the
//! public data fields of an object type, `DeepKeys<T>` their nested object
//! fields too (stopping at a recursive field). `Parameters` reads the names
//! of the parameters where they are written: a function type expression, a
//! function declaration (`Type<f>`), the annotation of a value or of a field
//! (`Type<names.get>`).

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::types::{TF021, TF026, TF028, TY003};
use crate::ids::{NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, FieldEntry, FieldFlags, TypeKind};

/// The type functions of this module.
pub(super) const KEY_AND_FUNCTION_TYPE_FUNCTIONS: &[&str] = &[
	"Keys",
	"DeepKeys",
	"Values",
	"Record",
	"Parameters",
	"Return",
	"ReturnType",
	"Uppercase",
	"Lowercase",
	"Capitalize",
];

impl Walker<'_> {
	/// One of [`KEY_AND_FUNCTION_TYPE_FUNCTIONS`] applied: its result (the
	/// errors reported at `n`), `None` for another name.
	pub(super) fn key_or_function_type_function(
		&mut self,
		n: NodeId,
		callee: NodeId,
		name: &str,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		if !KEY_AND_FUNCTION_TYPE_FUNCTIONS.contains(&name) {
			return None;
		}
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let expected = if name == "Record" { 2 } else { 1 };
		if arguments.len() != expected {
			let message = if expected == 2 {
				format!("`{name}` takes a key set and a type, ex: `{name}<{{ id, name }}, String>`")
			} else {
				format!("`{name}` takes one type, ex: `{name}<User>`")
			};
			let ty = self.memo(n, |w| {
				let diagnostic = w.diagnostic(&TY003, n, message);
				w.report(diagnostic);
				TypeId::UNKNOWN
			});
			self.memo(callee, |_| ty);
			return Some(ty);
		}
		let first = arguments.first().expect("an argument");
		let ty = match name {
			"Record" => {
				self.set_flag(first, NodeFlags::TYPE_EXPR);
				let item = self.check_type_expr(arguments.get(1).expect("a type"));
				self.memo(n, |w| w.record_type(first, item))
			}
			"Parameters" => {
				let function = self.check_type_expr(first);
				let names = self.parameter_names(first, 0);
				self.memo(n, |w| w.parameters_type(n, function, names))
			}
			_ => {
				let subject = self.check_type_expr(first);
				self.memo(n, |w| {
					if w.s.types.is_recovering(subject) {
						return TypeId::UNKNOWN;
					}
					match name {
						"Keys" | "DeepKeys" => w.keys_type(n, subject, name == "DeepKeys"),
						"Values" => w.values_type(n, subject),
						"Return" | "ReturnType" => match w.s.types.kind(subject) {
							TypeKind::Function(sig) => w.s.types.sig(sig).ret,
							_ => w.misuse(n, name, "a function type", subject),
						},
						_ => w.string_type_function(n, name, subject),
					}
				})
			}
		};
		self.memo(callee, |_| ty);
		Some(ty)
	}

	/// `TF021`: a type function given a type it doesn't take.
	fn misuse(&mut self, n: NodeId, name: &str, wanted: &str, given: TypeId) -> TypeId {
		let message = format!("`{name}` takes {wanted}, not a `{}`", self.display(given));
		let diagnostic = self.diagnostic(&TF021, n, message);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// `Keys<T>`, `DeepKeys<T>`: the key set of an object type.
	pub(super) fn keys_type(&mut self, n: NodeId, subject: TypeId, deep: bool) -> TypeId {
		let object = self.s.types.unfold(subject);
		if !matches!(self.s.types.kind(object), TypeKind::Object(_)) {
			let name = if deep { "DeepKeys" } else { "Keys" };
			let wanted = match self.s.types.kind(subject) {
				TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..) => {
					"an object type (the keys of a map are `KeyType<…>`)"
				}
				_ => "an object type",
			};
			return self.misuse(n, name, wanted, subject);
		}
		self.key_set_of(object, deep, 0)
	}

	fn key_set_of(&mut self, object: TypeId, deep: bool, depth: u32) -> TypeId {
		// in declaration order (fields.rs)
		let fields = self.ordered_fields(object);
		let mut keys = Vec::with_capacity(fields.len());
		for field in fields {
			if field
				.flags
				.intersects(FieldFlags::PRIVATE | FieldFlags::METHOD)
			{
				continue;
			}
			let name = self.s.types.syms.resolve(field.name).to_string();
			// a nested object's own keys; a recursive field is a key alone
			let children = match self.s.types.kind(field.ty) {
				TypeKind::Object(_) if deep && depth < 32 => {
					self.key_set_of(field.ty, true, depth + 1)
				}
				_ => TypeId::UNSET,
			};
			keys.push((name, children));
		}
		self.s.types.fields_type(&keys)
	}

	/// `Values<C>`: the type of the values of a collection.
	fn values_type(&mut self, n: NodeId, subject: TypeId) -> TypeId {
		match self.s.types.kind(subject) {
			TypeKind::Collection(_, item, _) => item,
			_ => self.misuse(n, "Values", "a collection type", subject),
		}
	}

	/// `Record<keys, T>`: an object with the keys of a key set, nested groups
	/// as nested objects, every leaf of type `T`.
	fn record_type(&mut self, keys: NodeId, item: TypeId) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let View::Object { items } = syntax::view(&tree, keys) else {
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::types::TF010,
				keys,
				"a key set is written in braces, ex: `{ id, name }`",
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		};
		self.record_of(items, item).unwrap_or(TypeId::UNKNOWN)
	}

	fn record_of(&mut self, items: NodeList<'_>, item: TypeId) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let mut fields = Vec::with_capacity(items.len());
		for key in items.iter() {
			self.set_flag(key, NodeFlags::TYPE_EXPR);
			let (name, ty) = match syntax::view(&tree, key) {
				View::Name(name) => (tree.name(name), item),
				View::TypedObject { target, members } => {
					let View::Name(name) = syntax::view(&tree, target) else {
						return None;
					};
					(tree.name(name), self.record_of(members, item)?)
				}
				_ => {
					let diagnostic = self.diagnostic(
						&crate::diagnostics::codes::types::TF010,
						key,
						"a key set lists field names, ex: `{ id, name }`",
					);
					self.report(diagnostic);
					return None;
				}
			};
			fields.push(FieldEntry {
				name: self.s.types.syms.intern(name),
				ty,
				flags: FieldFlags::empty(),
			});
		}
		Some(self.s.types.object(&fields, None))
	}

	/// `Parameters<F>`: the parameters of a function type as an object
	/// (`TF026` when one has no name where it is written).
	fn parameters_type(
		&mut self,
		n: NodeId,
		function: TypeId,
		names: Option<Vec<Option<String>>>,
	) -> TypeId {
		if self.s.types.is_recovering(function) {
			return TypeId::UNKNOWN;
		}
		let TypeKind::Function(sig) = self.s.types.kind(function) else {
			return self.misuse(n, "Parameters", "a function type", function);
		};
		let params = self.s.types.params(self.s.types.sig(sig).params).to_vec();
		let names = names.unwrap_or_default();
		let mut fields = Vec::with_capacity(params.len());
		for (index, param) in params.iter().enumerate() {
			let Some(Some(name)) = names.get(index) else {
				let message = format!(
					"the parameter {} of `{}` has no name: `Parameters` needs named parameters, ex: `(id: Integer) => …`",
					index + 1,
					self.display(function)
				);
				let diagnostic = self.diagnostic(&TF026, n, message);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			fields.push(FieldEntry {
				name: self.s.types.syms.intern(name),
				ty: param.ty,
				flags: FieldFlags::empty(),
			});
		}
		self.s.types.object(&fields, None)
	}

	/// The names of the parameters of the function type a type expression
	/// designates, where they are written: `None` for each unnamed one, and
	/// `None` overall when they can't be found.
	pub(super) fn parameter_names(&mut self, n: NodeId, depth: u32) -> Option<Vec<Option<String>>> {
		if depth > 16 {
			return None;
		}
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Group(inner) => self.parameter_names(inner, depth + 1),
			// `(id: Integer, Integer) => …`
			View::Lambda { signature, .. } => Some(
				signature
					.parameters
					.iter()
					.map(|parameter| match syntax::view(&tree, parameter) {
						View::Assignment {
							target,
							annotation: Some(_),
							..
						} => match syntax::view(&tree, target) {
							View::Name(name) => Some(tree.name(name).to_string()),
							View::Spread(inner) => match syntax::view(&tree, inner) {
								View::Name(name) => Some(tree.name(name).to_string()),
								_ => None,
							},
							_ => None,
						},
						_ => None,
					})
					.collect(),
			),
			// a type of this file naming a function type
			View::Name(name) => {
				let symbol = match self.a.resolution(n) {
					Resolution::Local(symbol) => symbol,
					_ => {
						let sym = self.s.types.syms.intern(tree.name(name));
						self.a.symbols.lookup(self.body.scope, sym)?
					}
				};
				let declaration = self.a.symbols.symbol(symbol).decl_node;
				match syntax::view(&tree, declaration) {
					View::TypeDecl { value, .. } => self.parameter_names(value, depth + 1),
					_ => None,
				}
			}
			// `Type<value>`
			View::Call { callee, arguments } => {
				let View::Name(name) = syntax::view(&tree, callee) else {
					return None;
				};
				if tree.name(name) != "Type" || arguments.len() != 1 {
					return None;
				}
				self.value_parameter_names(arguments.first()?, depth + 1)
			}
			_ => None,
		}
	}

	/// The names of the parameters of the function a value is: a function
	/// declaration, a value annotated with a function type, a field of an
	/// object type declared with one.
	pub(super) fn value_parameter_names(
		&mut self,
		n: NodeId,
		depth: u32,
	) -> Option<Vec<Option<String>>> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Group(inner) => self.value_parameter_names(inner, depth + 1),
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(n) else {
					return None;
				};
				let data = self.a.symbols.symbol(symbol);
				if data.kind == SymbolKind::Function
					&& let Some(declaration) = self.direct_function(symbol)
				{
					let info = self.a.function(declaration)?;
					return Some(
						info.params
							.iter()
							.map(|param| {
								let name = self.a.symbols.symbol(param.symbol).name;
								Some(self.s.types.syms.resolve(name).to_string())
							})
							.collect(),
					);
				}
				match syntax::view(&tree, data.decl_node) {
					View::Assignment {
						annotation: Some(annotation),
						..
					}
					| View::Declaration {
						annotation: Some(annotation),
						..
					} => self.parameter_names(annotation, depth + 1),
					_ => None,
				}
			}
			// `names.get`: the field's declaration in its object type
			View::Access { target, members } => {
				let [member] = members.iter().collect::<Vec<_>>()[..] else {
					return None;
				};
				let View::Name(field) = syntax::view(&tree, member) else {
					return None;
				};
				let field = self.s.types.syms.intern(tree.name(field));
				let object = self.type_of(target);
				let mut declarations = self.declarations_of(object);
				if declarations.is_empty()
					&& let Resolution::Local(symbol) = self.a.resolution(target)
					&& let View::Assignment {
						annotation: Some(annotation),
						..
					}
					| View::Declaration {
						annotation: Some(annotation),
						..
					} = syntax::view(&tree, self.a.symbols.symbol(symbol).decl_node)
				{
					declarations = self.declarations_of(self.type_of(annotation));
				}
				for declaration in declarations {
					let Some(decl) = self.a.type_decls.get(&declaration) else {
						continue;
					};
					let Some(field) = decl.fields.iter().find(|decl| decl.name == field) else {
						continue;
					};
					let node = field.node;
					if let View::Assignment {
						annotation: Some(annotation),
						..
					} = syntax::view(&tree, node)
					{
						return self.parameter_names(annotation, depth + 1);
					}
				}
				None
			}
			_ => None,
		}
	}

	/// `Uppercase`, `Lowercase`, `Capitalize`: applied to each string
	/// literal; `String` stays `String` (`TF028` for another type).
	fn string_type_function(&mut self, n: NodeId, name: &str, subject: TypeId) -> TypeId {
		let members: Vec<TypeId> = match self.s.types.kind(subject) {
			TypeKind::Union(list) => self.s.types.list_items(list).to_vec(),
			_ => vec![subject],
		};
		let mut results = Vec::with_capacity(members.len());
		for member in members {
			match self.s.types.kind(member) {
				TypeKind::String => results.push(TypeId::STRING),
				TypeKind::StringLit(text) => {
					let text = self.s.types.syms.resolve(text).to_string();
					let changed = match name {
						"Uppercase" => text.to_uppercase(),
						"Lowercase" => text.to_lowercase(),
						_ => {
							let mut characters = text.chars();
							match characters.next() {
								Some(first) => first.to_uppercase().chain(characters).collect(),
								None => text,
							}
						}
					};
					results.push(self.s.types.string_literal(&changed));
				}
				_ => {
					let message = format!(
						"`{name}` takes a string type or string literals, not a `{}`",
						self.display(subject)
					);
					let diagnostic = self.diagnostic(&TF028, n, message);
					self.report(diagnostic);
					return TypeId::UNKNOWN;
				}
			}
		}
		self.s.types.union(&results)
	}
}
