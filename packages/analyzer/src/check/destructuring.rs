//! Destructuring (values brief §16, `operations/destructuring.md`): object,
//! array and tuple patterns in `let`/`mutable` declarations, in `for` loops
//! and in parameters, nested in any order.
//!
//! - **Object patterns**: the value is an object (`E-OPT-1` when optional,
//!   `E-DST-2` otherwise); a field must exist unless it has a default
//!   (`E-DST-3`); a default applies when the field holds `none` or is absent
//!   from the type, so the binding is not optional; `as` renames. A rest is
//!   an object of the other fields.
//! - **Array patterns**: the value is an `Array` (`E-DST-2`); its items are
//!   read by index, so the declaration can fail with `IndexError` unless the
//!   length is proven (a literal, a length fact): `try let [a, b] = values`.
//!   A rest is an `Array` of the other items.
//! - **Tuple patterns**: a tuple of exactly the same arity (`E-DST-5`).
//! - A name bound twice in one pattern is `E-DST-7`. `use` patterns (aliases
//!   of the fields) come with references: `mutable use` is `E-DST-1`, a
//!   default in a `use` pattern `E-DST-4`.
//!
//! Every visit walks the pattern: the type part of each pattern node is
//! memoized (its diagnostics, its symbols, [`PatternInfo`]); defaults are
//! checked at every visit (they carry flow), and the bound names lose the
//! facts of a previous loop round. The items of a tuple literal give their
//! facts to the names they are bound to (`let (a, b) = (1, 2)`).

use std::sync::Arc;

use ethnum::I256;

use crate::analysis::{NodeFlags, PatternBinding, PatternElement, PatternInfo};
use crate::check::Walker;
use crate::diagnostics::codes::values::{
	E_DST_1, E_DST_2, E_DST_3, E_DST_4, E_DST_5, E_DST_7, E_OPT_1,
};
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{DeclState, SymbolKind};
use crate::syntax::{self, BindingKind, Name, View};
use crate::types::{CollKind, FieldEntry, TypeKind};

/// The names bound so far by the pattern being declared (`E-DST-7`).
#[derive(Default)]
pub(super) struct PatternNames {
	seen: Vec<String>,
}

impl Walker<'_> {
	/// `let pattern = value`, `mutable pattern = value`, `use pattern = place`.
	pub(super) fn check_destructuring_declaration(
		&mut self,
		n: NodeId,
		kind: BindingKind,
		pattern: NodeId,
		value: NodeId,
	) -> TypeId {
		let symbol_kind = match kind {
			BindingKind::Let => SymbolKind::Let,
			BindingKind::Mutable => SymbolKind::Mutable,
			BindingKind::Use | BindingKind::MutableUse => {
				return self.check_use_pattern(n, kind, pattern, value);
			}
			_ => {
				let mut names = Vec::new();
				self.pattern_names(pattern, &mut names);
				let what = format!("`{}` destructuring", kind.keyword());
				return self.unsupported_declaration(n, &names, SymbolKind::Let, &what);
			}
		};
		let actual = self.check(value, None);
		let mut names = PatternNames::default();
		self.destructure(pattern, actual, symbol_kind, Some(value), &mut names);
		self.memo(n, |_| TypeId::VOID)
	}

	/// `use { a, b } = object`: aliases of places come with the reference
	/// checker; `E-DST-1` and `E-DST-4` are reported here.
	fn check_use_pattern(
		&mut self,
		n: NodeId,
		kind: BindingKind,
		pattern: NodeId,
		_value: NodeId,
	) -> TypeId {
		let mut names = Vec::new();
		self.pattern_names(pattern, &mut names);
		let symbol_kind = if kind == BindingKind::MutableUse {
			SymbolKind::MutableUse
		} else {
			SymbolKind::Use
		};
		let mut problems = Vec::new();
		if kind == BindingKind::MutableUse {
			problems.push((
				&E_DST_1,
				n,
				"`mutable use` can't destructure: write one `mutable use` per field, or `use { a, b } = place`"
					.to_string(),
			));
		}
		if let Some(default) = self.pattern_default(pattern) {
			problems.push((
				&E_DST_4,
				default,
				"a `use` pattern designates fields of a place: a default value is not a place, use `let`"
					.to_string(),
			));
		}
		if problems.is_empty() {
			return self.unsupported_declaration(n, &names, symbol_kind, "`use` destructuring");
		}
		self.memo(n, |w| {
			for (code, node, message) in problems {
				let diagnostic = w.diagnostic(code, node, message);
				w.report(diagnostic);
			}
			w.quiet += 1;
			for &name in names.iter().filter(|name| !name.is_empty()) {
				let symbol = w.declare(n, name, symbol_kind);
				w.a.symbols.symbol_mut(symbol).ty = TypeId::UNKNOWN;
			}
			w.quiet -= 1;
			TypeId::VOID
		})
	}

	/// The first default value of an object pattern (nested included).
	fn pattern_default(&self, pattern: NodeId) -> Option<NodeId> {
		match self.view(pattern) {
			View::ObjectPattern { fields, .. } => fields.iter().find_map(|field| {
				field
					.default
					.or_else(|| field.nested.and_then(|nested| self.pattern_default(nested)))
			}),
			View::ArrayPattern { items, .. } | View::TuplePattern { items } => {
				items.iter().find_map(|item| self.pattern_default(item))
			}
			_ => None,
		}
	}

	/// Binds a pattern (or a name) to a value of type `ty`, declaring its
	/// names with `kind` in the current scope. `value`: the value node, when
	/// its parts can give facts (a tuple literal). Every visit.
	pub(super) fn destructure(
		&mut self,
		pattern: NodeId,
		ty: TypeId,
		kind: SymbolKind,
		value: Option<NodeId>,
		names: &mut PatternNames,
	) {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, pattern) {
			View::Name(name) => self.bind_name(pattern, name, ty, kind, value, names),
			View::Group(inner) => self.destructure(inner, ty, kind, value, names),
			View::TuplePattern { items } | View::Tuple { items } => {
				let items: Vec<NodeId> = items.iter().collect();
				self.bind_tuple_pattern(pattern, &items, ty, kind, value, names);
			}
			View::ArrayPattern { items, rest } => {
				let items: Vec<NodeId> = items.iter().collect();
				self.bind_array_pattern(pattern, (&items, rest), ty, kind, value, names);
			}
			View::ObjectPattern { .. } => self.bind_object_pattern(pattern, ty, kind, names),
			View::Error => {}
			_ => {
				self.unsupported(pattern, "this pattern");
			}
		}
	}

	/// A name of a pattern: a symbol of type `ty` declared on the name node.
	fn bind_name(
		&mut self,
		node: NodeId,
		name: Name,
		ty: TypeId,
		kind: SymbolKind,
		value: Option<NodeId>,
		names: &mut PatternNames,
	) {
		self.memo(node, |w| {
			w.declare_pattern_name(node, name, ty, kind, names);
			ty
		});
		if let Some(symbol) = self.a.symbols.symbol_of_decl(node) {
			self.bind_pattern_value(symbol, value);
		}
	}

	/// Declares one name of a pattern (type part): `E-DST-7` when the pattern
	/// already binds it.
	fn declare_pattern_name(
		&mut self,
		node: NodeId,
		name: Name,
		ty: TypeId,
		kind: SymbolKind,
		names: &mut PatternNames,
	) -> Option<SymbolId> {
		if name.is_empty() {
			return None;
		}
		let text = self.tree.name(name).to_string();
		if names.seen.contains(&text) {
			let diagnostic = self.diagnostic_at(
				&E_DST_7,
				syntax::name_span(name),
				format!("`{text}` is bound twice by this pattern: rename one with `as`"),
			);
			self.report(diagnostic);
			return None;
		}
		names.seen.push(text);
		let symbol = self.declare(node, name, kind);
		let entry = self.a.symbols.symbol_mut(symbol);
		entry.ty = ty;
		entry.state = DeclState::Sequential;
		Some(symbol)
	}

	/// The flow of a bound name: the value's facts (an item of a tuple
	/// literal), or none (a previous loop round's are dropped).
	fn bind_pattern_value(&mut self, symbol: SymbolId, value: Option<NodeId>) {
		match value {
			Some(value) => self.bind_value(symbol, value),
			None => {
				let place = self.symbol_place(symbol);
				self.body.flow.remove_fact(place);
			}
		}
	}

	/// `(a, b)`: a tuple of the same arity.
	fn bind_tuple_pattern(
		&mut self,
		pattern: NodeId,
		items: &[NodeId],
		ty: TypeId,
		kind: SymbolKind,
		value: Option<NodeId>,
		names: &mut PatternNames,
	) {
		let recovering = self.s.types.is_recovering(ty);
		let elements = self.tuple_items(ty);
		let fits = elements
			.as_ref()
			.is_some_and(|elements| elements.len() == items.len());
		// the items of a tuple literal give their facts
		let literal_items: Option<Vec<NodeId>> = value
			.map(|value| self.strip_groups(value))
			.and_then(|value| match self.view(value) {
				View::Tuple { items: literal } if literal.len() == items.len() => {
					Some(literal.iter().collect())
				}
				_ => None,
			});
		for (index, &item) in items.iter().enumerate() {
			let element = match &elements {
				Some(elements) if fits => elements[index],
				_ => TypeId::UNKNOWN,
			};
			let item_value = literal_items.as_ref().map(|literal| literal[index]);
			self.destructure(item, element, kind, item_value, names);
		}
		self.memo(pattern, |w| {
			if !recovering {
				match &elements {
					None => w.pattern_mismatch(pattern, ty, "a tuple pattern `(a, b)`"),
					Some(elements) if elements.len() != items.len() => {
						let message = format!(
							"this value has {} elements (`{}`), the pattern {}: bind every element",
							elements.len(),
							w.display(ty),
							items.len()
						);
						let diagnostic = w.diagnostic(&E_DST_5, pattern, message);
						w.report(diagnostic);
					}
					Some(_) => {}
				}
			}
			let elements = items
				.iter()
				.map(|&item| PatternElement {
					field: None,
					binding: w.item_binding(item),
					ty: w.type_of(item),
					default: None,
					absent: false,
				})
				.collect();
			w.a.patterns.insert(
				pattern,
				PatternInfo {
					ty,
					elements,
					rest: None,
				},
			);
			ty
		});
	}

	/// What an item of an array or tuple pattern binds.
	fn item_binding(&self, item: NodeId) -> PatternBinding {
		match self.view(item) {
			View::Name(_) => self
				.a
				.symbols
				.symbol_of_decl(item)
				.map_or(PatternBinding::None, PatternBinding::Symbol),
			View::Group(inner) => self.item_binding(inner),
			View::Error => PatternBinding::None,
			_ => PatternBinding::Nested(item),
		}
	}

	/// `[a, b, ...rest]`: an array, read by index.
	fn bind_array_pattern(
		&mut self,
		pattern: NodeId,
		(items, rest): (&[NodeId], Option<NodeId>),
		ty: TypeId,
		kind: SymbolKind,
		value: Option<NodeId>,
		names: &mut PatternNames,
	) {
		let recovering = self.s.types.is_recovering(ty);
		let item = match self.s.types.kind(ty) {
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => Some(item),
			_ => None,
		};
		let item_ty = item.unwrap_or(TypeId::UNKNOWN);
		for &node in items {
			self.destructure(node, item_ty, kind, None, names);
		}
		let rest_ty = self.s.types.array(item_ty);
		if let Some(rest) = rest {
			self.destructure(rest, rest_ty, kind, None, names);
		}
		self.memo(pattern, |w| {
			if item.is_none() && !recovering {
				w.pattern_mismatch(pattern, ty, "an array pattern `[a, b]`");
			}
			let elements = items
				.iter()
				.map(|&node| PatternElement {
					field: None,
					binding: w.item_binding(node),
					ty: item_ty,
					default: None,
					absent: false,
				})
				.collect();
			let rest = rest
				.and_then(|rest| w.a.symbols.symbol_of_decl(rest))
				.map(|symbol| (symbol, rest_ty));
			w.a.patterns
				.insert(pattern, PatternInfo { ty, elements, rest });
			ty
		});
		// the length: proven, or `IndexError` (flow)
		self.clear_flag(pattern, NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL);
		if item.is_some() && !items.is_empty() && self.body.flow.reachable {
			let needed = I256::from(items.len() as u64);
			let proven = value
				.and_then(|value| self.expression_length(value))
				.is_some_and(|length| length.lo >= needed);
			if !proven {
				self.set_flag(pattern, NodeFlags::INDEX_CHECK);
				let error = self.builtin_error("IndexError");
				self.can_fail(pattern, error);
			}
		}
	}

	/// `{ a, b as c, d = 4, nested { e }, ...rest }`: an object.
	fn bind_object_pattern(
		&mut self,
		pattern: NodeId,
		ty: TypeId,
		kind: SymbolKind,
		names: &mut PatternNames,
	) {
		let tree = Arc::clone(&self.tree);
		let View::ObjectPattern { fields, rest } = syntax::view(&tree, pattern) else {
			return;
		};
		let recovering = self.s.types.is_recovering(ty);
		let object = self.pattern_object(ty);
		let object_fields: Vec<FieldEntry> = object
			.map(|object| self.s.types.fields_of(object).to_vec())
			.unwrap_or_default();
		// each field: its type, its default checked against it (every visit)
		let mut elements = Vec::new();
		for field in fields.iter() {
			let text = tree.name(field.name);
			let found = object_fields
				.iter()
				.find(|entry| self.s.types.syms.resolve(entry.name) == text)
				.copied();
			let field_ty = found.map_or(TypeId::UNKNOWN, |entry| entry.ty);
			let bound_ty = match (found, field.default) {
				(Some(_), Some(default)) => {
					let expected = self.without_none(field_ty);
					let actual = self.check(default, Some(expected));
					self.sink(default, actual, expected);
					expected
				}
				(None, Some(default)) => {
					let actual = self.check(default, None);
					self.sink_unannotated(default, actual);
					self.s.types.widen(actual)
				}
				(Some(_), None) => field_ty,
				(None, None) => TypeId::UNKNOWN,
			};
			if let Some(nested) = field.nested {
				self.destructure(nested, bound_ty, kind, None, names);
			}
			elements.push((field, found, field_ty, bound_ty));
		}
		let rest_ty = match object {
			Some(_) => {
				let named: Vec<&str> = fields.iter().map(|field| tree.name(field.name)).collect();
				let remaining: Vec<FieldEntry> = object_fields
					.iter()
					.filter(|entry| !named.contains(&self.s.types.syms.resolve(entry.name)))
					.copied()
					.collect();
				self.s.types.object(&remaining, None)
			}
			None => TypeId::UNKNOWN,
		};
		if let Some(rest) = rest {
			self.destructure(rest, rest_ty, kind, None, names);
		}
		self.memo(pattern, |w| {
			if object.is_none() && !recovering {
				if w.has_none(ty) {
					let message = format!(
						"this value may be `none` (`{}`): check it first, ex: `if value is not none`",
						w.display(ty)
					);
					let diagnostic = w.diagnostic(&E_OPT_1, pattern, message);
					w.report(diagnostic);
				} else {
					w.pattern_mismatch(pattern, ty, "an object pattern `{ a, b }`");
				}
			}
			let mut infos = Vec::with_capacity(elements.len());
			for &(field, found, field_ty, bound_ty) in &elements {
				let text = w.tree.name(field.name).to_string();
				if found.is_none() && field.default.is_none() && object.is_some() {
					let message = format!(
						"`{text}` is not a field of `{}`: give it a default, ex: `{text} = …`",
						w.display(ty)
					);
					let diagnostic =
						w.diagnostic_at(&E_DST_3, syntax::name_span(field.name), message);
					w.report(diagnostic);
				}
				let binding = match field.nested {
					Some(nested) => PatternBinding::Nested(nested),
					None => {
						let name = field.alias.unwrap_or(field.name);
						w.declare_pattern_name(pattern, name, bound_ty, kind, names)
							.map_or(PatternBinding::None, PatternBinding::Symbol)
					}
				};
				let field_sym = w.s.types.syms.intern(&text);
				infos.push(PatternElement {
					field: Some(field_sym),
					binding,
					ty: field_ty,
					default: field.default,
					absent: found.is_none(),
				});
			}
			let rest = rest
				.and_then(|rest| w.a.symbols.symbol_of_decl(rest))
				.map(|symbol| (symbol, rest_ty));
			w.a.patterns.insert(
				pattern,
				PatternInfo {
					ty,
					elements: infos,
					rest,
				},
			);
			ty
		});
		// the names of the fields lose the facts of a previous round
		if let Some(info) = self.a.patterns.get(&pattern) {
			let symbols: Vec<SymbolId> = info
				.elements
				.iter()
				.filter_map(|element| match element.binding {
					PatternBinding::Symbol(symbol) => Some(symbol),
					_ => None,
				})
				.collect();
			for symbol in symbols {
				self.bind_pattern_value(symbol, None);
			}
		}
	}

	/// The object type an object pattern reads: an object, or a member of a
	/// named union whose data is an object.
	fn pattern_object(&self, ty: TypeId) -> Option<TypeId> {
		match self.s.types.kind(ty) {
			TypeKind::Object(_) => Some(ty),
			TypeKind::Member(nominal, index) => {
				let payload = self.s.types.payload(nominal, index);
				matches!(self.s.types.kind(payload), TypeKind::Object(_)).then_some(payload)
			}
			_ => None,
		}
	}

	/// `E-DST-2`: the pattern doesn't match the kind of the value.
	fn pattern_mismatch(&mut self, pattern: NodeId, ty: TypeId, what: &str) {
		let message = format!(
			"{what} can't destructure a `{}`",
			self.display(self.s.types.widen(ty))
		);
		let diagnostic = self.diagnostic(&E_DST_2, pattern, message);
		self.report(diagnostic);
	}
}
