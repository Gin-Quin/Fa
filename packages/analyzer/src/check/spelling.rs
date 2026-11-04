//! The spelling of a type expression (`@typeToString`, decision D49): a
//! name as written with its arguments (`UserId`, `Point(Integer)`), an
//! anonymous type canonically (`{ x: Integer, y: Integer }`, fields in the
//! order they are written). A type built by an operation (a spread, a
//! type function whose body spreads its parameter) is anonymous: its name
//! spells its fields, the spread ones first where the spread is written
//! (`type B = { ...A, b: Integer }` is `{ a: String, b: Integer }`).
//! What isn't spelled here (`Type(value)`, functions, ranged integers) is
//! the type as displayed.

use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::check::Walker;
use crate::ids::{NodeId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};
use crate::types::display::type_application;

/// A spelled type: its text, or the fields of an object (`name: Type`, in
/// order, a field written again replacing the earlier one in place).
#[derive(Clone)]
enum Spelled {
	Text(String),
	Object(Vec<(String, String)>),
}

impl Spelled {
	fn text(self) -> String {
		match self {
			Spelled::Text(text) => text,
			Spelled::Object(fields) if fields.is_empty() => "{}".to_string(),
			Spelled::Object(fields) => {
				let fields: Vec<String> = fields
					.into_iter()
					.map(|(name, ty)| format!("{name}: {ty}"))
					.collect();
				format!("{{ {} }}", fields.join(", "))
			}
		}
	}
}

/// The type parameters of a type function, bound to the spelled arguments.
type Bindings = FxHashMap<String, Spelled>;

impl Walker<'_> {
	/// The spelling of the type expression `n` (checked already).
	pub(super) fn type_spelling(&mut self, n: NodeId) -> String {
		match self.spell(n, &Bindings::default(), 0) {
			Some(spelled) => spelled.text(),
			None => self.display(self.type_of(n)),
		}
	}

	/// A type expression spelled, `None` where the displayed type is used.
	fn spell(&mut self, n: NodeId, bindings: &Bindings, depth: u32) -> Option<Spelled> {
		if depth > 32 {
			return None;
		}
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Group(inner) => self.spell(inner, bindings, depth + 1),
			View::Name(name) => {
				let text = tree.name(name);
				if let Some(bound) = bindings.get(text) {
					return Some(bound.clone());
				}
				// a type built by an operation: its fields
				if let Some((parameters, value)) = self.type_declaration(n, text)
					&& parameters.is_empty()
					&& self.is_built_by_operation(value, depth)
				{
					return self.spell(value, &Bindings::default(), depth + 1);
				}
				Some(Spelled::Text(text.to_string()))
			}
			View::Call { callee, arguments } => {
				let View::Name(name) = syntax::view(&tree, callee) else {
					return None;
				};
				let text = tree.name(name);
				// a type of this file applied: `Point(Integer)`, or a type
				// function expanded; anything else is displayed
				let (parameters, value) = self.type_declaration(callee, text)?;
				let mut spelled = Vec::new();
				for argument in arguments.iter() {
					let argument = match self.spell(argument, bindings, depth + 1) {
						Some(argument) => argument,
						None => Spelled::Text(self.display(self.type_of(argument))),
					};
					spelled.push(argument);
				}
				if self.is_built_by_operation(value, depth) && parameters.len() == spelled.len() {
					let inner: Bindings = parameters.into_iter().zip(spelled).collect();
					return self.spell(value, &inner, depth + 1);
				}
				let arguments: Vec<String> = spelled.into_iter().map(Spelled::text).collect();
				Some(Spelled::Text(type_application(text, &arguments)))
			}
			View::Object { items } => {
				let mut fields: Vec<(String, String)> = Vec::new();
				for item in items.iter() {
					let (item, prefix) = match syntax::view(&tree, item) {
						View::Readonly(inner) => (inner, "readonly "),
						_ => (item, ""),
					};
					match syntax::view(&tree, item) {
						View::Spread(inner) => {
							let Some(Spelled::Object(spread)) =
								self.spell_fields(inner, bindings, depth)
							else {
								return None;
							};
							for field in spread {
								add_field(&mut fields, field);
							}
						}
						View::Assignment {
							target, annotation, ..
						} => {
							let View::Name(name) = syntax::view(&tree, target) else {
								return None;
							};
							let ty = match annotation {
								Some(annotation) => {
									self.spell(annotation, bindings, depth + 1).map_or_else(
										|| self.display(self.type_of(annotation)),
										Spelled::text,
									)
								}
								None => self.display(self.type_of(target)),
							};
							add_field(&mut fields, (format!("{prefix}{}", tree.name(name)), ty));
						}
						View::Declaration {
							name, annotation, ..
						} => {
							let annotation = annotation?;
							let ty = self.spell(annotation, bindings, depth + 1).map_or_else(
								|| self.display(self.type_of(annotation)),
								Spelled::text,
							);
							add_field(&mut fields, (format!("reactive {}", tree.name(name)), ty));
						}
						// the functions of a type are not part of its spelling
						View::Function { .. } | View::TypeDecl { .. } => {}
						_ => return None,
					}
				}
				Some(Spelled::Object(fields))
			}
			View::Union { operands } => {
				let mut members = Vec::new();
				for operand in operands.iter() {
					let member = match self.spell(operand, bindings, depth + 1) {
						Some(member) => member.text(),
						None => self.display(self.type_of(operand)),
					};
					members.push(member);
				}
				Some(Spelled::Text(members.join(" | ")))
			}
			View::Optional(inner) => {
				let spelled = self.spell(inner, bindings, depth + 1)?.text();
				let union = matches!(syntax::view(&tree, inner), View::Union { .. });
				Some(Spelled::Text(if union {
					format!("({spelled})?")
				} else {
					format!("{spelled}?")
				}))
			}
			_ => None,
		}
	}

	/// The fields a spread inserts (`...Entity`): a bound parameter, a type
	/// of this file (its definition's fields), an object type written there.
	fn spell_fields(&mut self, n: NodeId, bindings: &Bindings, depth: u32) -> Option<Spelled> {
		if depth > 32 {
			return None;
		}
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Group(inner) => self.spell_fields(inner, bindings, depth + 1),
			View::Name(name) => {
				let text = tree.name(name);
				if let Some(bound) = bindings.get(text) {
					return match bound {
						Spelled::Object(_) => Some(bound.clone()),
						Spelled::Text(_) => None,
					};
				}
				let (parameters, value) = self.type_declaration(n, text)?;
				if !parameters.is_empty() {
					return None;
				}
				self.spell_fields(value, &Bindings::default(), depth + 1)
			}
			View::Call { .. } | View::Object { .. } => match self.spell(n, bindings, depth + 1)? {
				Spelled::Object(fields) => Some(Spelled::Object(fields)),
				Spelled::Text(_) => None,
			},
			_ => None,
		}
	}

	/// The declaration of the type a name of this file designates: its
	/// parameters' names and its value.
	fn type_declaration(&mut self, n: NodeId, text: &str) -> Option<(Vec<String>, NodeId)> {
		let symbol = match self.a.resolution(n) {
			Resolution::Local(symbol) => symbol,
			_ => {
				let sym = self.s.types.syms.intern(text);
				self.a.symbols.lookup(self.body.scope, sym)?
			}
		};
		self.declaration_of_type(symbol)
	}

	fn declaration_of_type(&self, symbol: SymbolId) -> Option<(Vec<String>, NodeId)> {
		let data = self.a.symbols.symbol(symbol);
		if data.kind != SymbolKind::Type {
			return None;
		}
		let tree = &self.tree;
		let View::TypeDecl {
			parameters, value, ..
		} = syntax::view(tree, data.decl_node)
		else {
			return None;
		};
		let parameters = parameters
			.iter()
			.map(|parameter| {
				let parameter = match syntax::view(tree, parameter) {
					View::Assignment { target, .. } => target,
					_ => parameter,
				};
				tree.text(parameter.index())
					.split(':')
					.next()
					.unwrap_or_default()
					.trim()
					.to_string()
			})
			.collect();
		Some((parameters, value))
	}

	/// Whether a type's definition is built by an operation: an object type
	/// with a spread, or a type function of one applied.
	fn is_built_by_operation(&mut self, value: NodeId, depth: u32) -> bool {
		if depth > 32 {
			return false;
		}
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, value) {
			View::Group(inner) => self.is_built_by_operation(inner, depth + 1),
			View::Object { items } => items
				.iter()
				.any(|item| matches!(syntax::view(&tree, item), View::Spread(_))),
			View::Call { callee, .. } => {
				let View::Name(name) = syntax::view(&tree, callee) else {
					return false;
				};
				let text = tree.name(name).to_string();
				self.type_declaration(callee, &text)
					.is_some_and(|(_, inner)| self.is_built_by_operation(inner, depth + 1))
			}
			View::Name(name) => {
				let text = tree.name(name).to_string();
				self.type_declaration(value, &text)
					.is_some_and(|(parameters, inner)| {
						parameters.is_empty() && self.is_built_by_operation(inner, depth + 1)
					})
			}
			_ => false,
		}
	}
}

/// Adds a field, replacing one of the same name in place.
fn add_field(fields: &mut Vec<(String, String)>, field: (String, String)) {
	let bare = |name: &str| {
		name.trim_start_matches("readonly ")
			.trim_start_matches("reactive ")
			.to_string()
	};
	let name = bare(&field.0);
	match fields.iter_mut().find(|existing| bare(&existing.0) == name) {
		Some(existing) => *existing = field,
		None => fields.push(field),
	}
}
