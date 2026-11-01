//! Patterns: destructuring declarations, object, array and tuple patterns
//! (`when` patterns are handled by `when`, with exhaustiveness).

use crate::check::Walker;
use crate::ids::{NodeId, TypeId};
use crate::symbols::SymbolKind;
use crate::syntax::{BindingKind, Name, NodeList, PatternFields, View};

impl Walker<'_> {
	/// `let { a, b } = object`
	pub(super) fn check_destructuring(
		&mut self,
		n: NodeId,
		kind: BindingKind,
		pattern: NodeId,
		_value: NodeId,
	) -> TypeId {
		let mut names = Vec::new();
		self.pattern_names(pattern, &mut names);
		let kind = match kind {
			BindingKind::Mutable => SymbolKind::Mutable,
			BindingKind::Use => SymbolKind::Use,
			BindingKind::MutableUse => SymbolKind::MutableUse,
			_ => SymbolKind::Let,
		};
		self.unsupported_declaration(n, &names, kind, "destructuring")
	}

	/// The names a pattern binds, in source order.
	pub(super) fn pattern_names(&self, pattern: NodeId, names: &mut Vec<Name>) {
		match self.view(pattern) {
			View::Name(name) => names.push(name),
			View::ObjectPattern { fields, rest } => {
				for field in fields.iter() {
					match field.nested {
						Some(nested) => self.pattern_names(nested, names),
						None => names.push(field.alias.unwrap_or(field.name)),
					}
				}
				if let Some(rest) = rest {
					self.pattern_names(rest, names);
				}
			}
			View::ArrayPattern { items, rest } => {
				for item in items.iter() {
					self.pattern_names(item, names);
				}
				if let Some(rest) = rest {
					self.pattern_names(rest, names);
				}
			}
			View::TuplePattern { items } | View::Tuple { items } => {
				for item in items.iter() {
					self.pattern_names(item, names);
				}
			}
			View::Spread(inner) | View::Group(inner) => self.pattern_names(inner, names),
			_ => {}
		}
	}

	/// `{ a, b as c, d = 4, nested { e }, ...rest }`
	pub(super) fn check_object_pattern(
		&mut self,
		n: NodeId,
		_fields: PatternFields<'_>,
		_rest: Option<NodeId>,
	) -> TypeId {
		self.unsupported(n, "object patterns")
	}

	/// `[first, second, ...others]`
	pub(super) fn check_array_pattern(
		&mut self,
		n: NodeId,
		_items: NodeList<'_>,
		_rest: Option<NodeId>,
	) -> TypeId {
		self.unsupported(n, "array patterns")
	}

	/// `(number, name)`
	pub(super) fn check_tuple_pattern(&mut self, n: NodeId, _items: NodeList<'_>) -> TypeId {
		self.unsupported(n, "tuple patterns")
	}
}
