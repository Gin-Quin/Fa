//! Evaluated provenance: one transfer per value, before its flow is lost.
use crate::analysis::FieldValue;
use crate::check::Walker;
use crate::flow::values::{ClosureOrigin, ValueOrigins};
use crate::flow::{Identity, Step};
use crate::ids::{NodeId, PlaceId, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

impl Walker<'_> {
	pub(super) fn evaluated_origins(&self, node: NodeId) -> ValueOrigins {
		self.value_origins.get(&node).cloned().unwrap_or_default()
	}

	pub(super) fn record_value_origins(&mut self, node: NodeId) {
		let mut origins = ValueOrigins::default();
		let tree = std::sync::Arc::clone(&self.tree);
		match crate::syntax::view(&tree, node) {
			View::Group(inner)
			| View::Try(inner)
			| View::Do(inner)
			| View::Copy(inner)
			| View::Spread(inner) => origins = self.evaluated_origins(inner),
			View::None => {}
			View::Block { statements } => {
				if let Some(last) = statements.last() {
					origins = self.evaluated_origins(last);
				}
			}
			View::If {
				then_body,
				else_body,
				..
			} => {
				let taken = self.a.comptime_branches.get(&node).copied();
				if taken != Some(false) {
					origins.join(&self.evaluated_origins(then_body));
				}
				if taken != Some(true)
					&& let Some(body) = else_body
				{
					origins.join(&self.evaluated_origins(body));
				}
			}
			View::When { arms, .. } => {
				for arm in arms.iter() {
					origins.join(&self.evaluated_origins(arm.value));
				}
			}
			View::Catch {
				expression,
				handler,
				..
			} => {
				origins.join(&self.evaluated_origins(expression));
				origins.join(&self.evaluated_origins(handler));
			}
			View::Loop { .. } => origins = self.evaluated_origins(node),
			View::Lambda { .. } => {
				origins.owned = true;
				origins.closures.push(ClosureOrigin {
					projection: Vec::new(),
					lambda: node,
				});
			}
			View::Name(_) => {
				origins.places = self.places_of(node);
				if let Resolution::Local(symbol) = self.a.resolution(node) {
					if self.a.symbols.symbol(symbol).kind == SymbolKind::Function {
						if let Some(info) = self
							.a
							.functions
							.get(&self.a.symbols.symbol(symbol).decl_node)
						{
							origins.closures.push(ClosureOrigin {
								projection: Vec::new(),
								lambda: info.lambda,
							});
						}
					} else {
						for place in origins.places.clone() {
							origins.closures.extend(self.stored_origins(place).closures);
						}
					}
				}
				origins.owned = origins.places.is_empty();
			}
			View::Access { target, members } => {
				origins = self.evaluated_origins(target);
				for member in members.iter() {
					let View::Name(name) = self.view(member) else {
						continue;
					};
					let step = Step::Field(self.s.types.syms.intern(self.tree.name(name)));
					origins = Self::project_origins(origins, step);
				}
				origins.places = self.places_of(node);
				for place in origins.places.clone() {
					origins.closures.extend(self.stored_origins(place).closures);
				}
				origins.owned = origins.places.is_empty();
			}
			View::Index { target, index } => {
				let step = Step::Index(self.identity_of(index));
				origins = Self::project_origins(self.evaluated_origins(target), step);
				origins.places = self.places_of(node);
				for place in origins.places.clone() {
					origins.closures.extend(self.stored_origins(place).closures);
				}
				origins.owned = origins.places.is_empty();
			}
			View::Object { .. } | View::TypedObject { .. } => {
				origins.owned = true;
				let fields = self
					.a
					.objects
					.get(&node)
					.map(|info| info.fields.clone())
					.unwrap_or_default();
				for (name, field) in fields {
					match field {
						FieldValue::Given(value) | FieldValue::Default(value) => {
							for mut closure in self.evaluated_origins(value).closures {
								closure.projection.insert(0, Step::Field(name));
								origins.closures.push(closure);
							}
						}
						FieldValue::Spread(value) => origins
							.closures
							.extend(self.evaluated_origins(value).closures),
						_ => {}
					}
				}
			}
			View::Array { items } | View::Tuple { items } => {
				origins.owned = true;
				for (index, value) in items.iter().enumerate() {
					for mut closure in self.evaluated_origins(value).closures {
						closure
							.projection
							.insert(0, Step::Index(Identity::Exact(index as i64)));
						origins.closures.push(closure);
					}
				}
			}
			View::Assignment {
				value: Some(value), ..
			} => origins = self.evaluated_origins(value),
			_ => {
				self.ensure_builtin_reference(node);
				origins.places = self.places_of(node);
				origins.owned = origins.places.is_empty();
			}
		}
		if self.type_of(node) == TypeId::NEVER {
			origins = ValueOrigins::default();
		}
		origins.places.sort_unstable();
		origins.places.dedup();
		origins.closures.sort_unstable();
		origins.closures.dedup();
		self.value_origins.insert(node, origins);
	}

	fn project_origins(mut origins: ValueOrigins, step: Step) -> ValueOrigins {
		origins.closures.retain_mut(|closure| {
			let Some(first) = closure.projection.first() else {
				return false;
			};
			let matches = *first == step
				|| matches!(
					(first, step),
					(Step::Index(_), Step::Index(Identity::Unknown))
				);
			if matches {
				closure.projection.remove(0);
			}
			matches
		});
		origins
	}

	/// Read a projection and all contents stored below it.
	fn stored_origins(&self, place: PlaceId) -> ValueOrigins {
		let mut result = ValueOrigins::default();
		let steps = self.place_steps(place);
		for &(stored, id) in &self.body.flow.values {
			if self.places.get(stored).root != self.places.get(place).root {
				continue;
			}
			let stored_steps = self.place_steps(stored);
			if steps.starts_with(&stored_steps) {
				let mut value = self.facts.origins.get(id).clone();
				for &step in &steps[stored_steps.len()..] {
					value = Self::project_origins(value, step);
				}
				result.join(&value);
			} else if stored_steps.starts_with(&steps) {
				let mut value = self.facts.origins.get(id).clone();
				for closure in &mut value.closures {
					closure
						.projection
						.splice(0..0, stored_steps[steps.len()..].iter().copied());
				}
				result.join(&value);
			}
		}
		result
	}

	pub(super) fn store_value_origins(&mut self, places: Vec<PlaceId>, value: NodeId) {
		if !self.body.flow.reachable {
			return;
		}
		let origins = self.evaluated_origins(value).copied();
		let strong = places.len() == 1;
		for place in places {
			if strong {
				// Replace projected provenance as well as the complete local.
				let steps = self.place_steps(place);
				let root = self.places.get(place).root;
				let removed: Vec<_> = self
					.body
					.flow
					.values
					.iter()
					.filter_map(|&(stored, _)| {
						(self.places.get(stored).root == root
							&& self.place_steps(stored).starts_with(&steps))
						.then_some(stored)
					})
					.collect();
				self.body
					.flow
					.values
					.retain(|entry| !removed.contains(&entry.0));
				let ancestors: Vec<_> = self
					.body
					.flow
					.values
					.iter()
					.copied()
					.filter(|&(stored, _)| {
						self.places.get(stored).root == root
							&& steps.starts_with(&self.place_steps(stored))
					})
					.collect();
				for (stored, id) in ancestors {
					let suffix = &steps[self.place_steps(stored).len()..];
					let mut prior = self.facts.origins.get(id).clone();
					prior
						.closures
						.retain(|closure| !closure.projection.starts_with(suffix));
					let id = self.facts.origins.intern(prior);
					if let Ok(index) = self
						.body
						.flow
						.values
						.binary_search_by_key(&stored, |entry| entry.0)
					{
						self.body.flow.values[index].1 = id;
					}
				}
			}
			if origins.closures.is_empty() {
				continue;
			}
			let origins = if strong {
				origins.clone()
			} else {
				let mut old = self.stored_origins(place);
				old.join(&origins);
				old
			};
			let id = self.facts.origins.intern(origins);
			match self
				.body
				.flow
				.values
				.binary_search_by_key(&place, |entry| entry.0)
			{
				Ok(index) => self.body.flow.values[index].1 = id,
				Err(index) => self.body.flow.values.insert(index, (place, id)),
			}
		}
	}

	pub(super) fn bind_value_origins(&mut self, symbol: SymbolId, value: NodeId) {
		let place = self.places.symbol(symbol);
		self.store_value_origins(vec![place], value);
	}
}
