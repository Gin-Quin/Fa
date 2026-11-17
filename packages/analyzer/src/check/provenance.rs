//! Evaluated provenance: one transfer per value, before its flow is lost.
use crate::analysis::FieldValue;
use crate::check::Walker;
use crate::flow::values::{CallableOrigin, ClosureOrigin, ValueOrigins};
use crate::flow::{Identity, Step};
use crate::ids::{NodeId, PlaceId, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

/// Captures as evaluated when a function value is created.
#[derive(Clone)]
pub(crate) struct CapturedOrigins {
	pub symbol: SymbolId,
	pub places: Vec<PlaceId>,
	pub closures: Vec<NodeId>,
}

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
				self.record_capture_origins(node);
				origins.owned = true;
				origins.closures.push(ClosureOrigin {
					projection: Vec::new(),
					callable: CallableOrigin::Local(node),
				});
			}
			View::Name(_) => {
				origins.places = self.places_of(node);
				if let Resolution::Local(symbol) = self.a.resolution(node) {
					if let Some(state) = self.body.flow.ref_state(symbol) {
						let state = self.facts.refs.get(state);
						origins.opaque = state.opaque;
						origins.bounds = state.bounds.to_vec();
						origins.readonly = state.readonly;
					}
					origins.readonly |= self.readonly_elements.contains(&symbol);
					if self.a.symbols.symbol(symbol).kind == SymbolKind::Function {
						if let Some(info) = self
							.a
							.functions
							.get(&self.a.symbols.symbol(symbol).decl_node)
						{
							origins.closures.push(ClosureOrigin {
								projection: Vec::new(),
								callable: CallableOrigin::Local(info.lambda),
							});
						}
					} else {
						for place in origins.places.clone() {
							let stored = self.stored_origins(place);
							origins.closures.extend(stored.closures);
							origins.tasks.extend(stored.tasks);
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
					origins = self.project_origins(origins, step);
				}
				origins.places = self.places_of(node);
				origins.opaque = false;
				for place in origins.places.clone() {
					let stored = self.stored_origins(place);
					origins.closures.extend(stored.closures);
					origins.tasks.extend(stored.tasks);
				}
				origins.owned = origins.places.is_empty();
			}
			View::Index { target, index } => {
				let step =
					Step::Index(self.reference_index_identity(index, Some(self.type_of(target))));
				origins = self.project_origins(self.evaluated_origins(target), step);
				origins.places = self.places_of(node);
				origins.opaque = false;
				for place in origins.places.clone() {
					let stored = self.stored_origins(place);
					origins.closures.extend(stored.closures);
					origins.tasks.extend(stored.tasks);
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
							let child = self.evaluated_origins(value);
							for mut task in child.tasks {
								task.projection.insert(0, Step::Field(name));
								origins.tasks.push(task);
							}
							for mut closure in child.closures {
								closure.projection.insert(0, Step::Field(name));
								origins.closures.push(closure);
							}
						}
						FieldValue::Spread(value) => {
							let child = self.evaluated_origins(value);
							origins.closures.extend(child.closures);
							origins.tasks.extend(child.tasks);
						}
						_ => {}
					}
				}
			}
			View::Array { items } | View::Tuple { items } => {
				origins.owned = true;
				let mut shifted = false;
				for (index, value) in items.iter().enumerate() {
					let spread = matches!(self.view(value), View::Spread(_));
					let child = self.evaluated_origins(value);
					for mut task in child.tasks {
						prefix_item_projection(&mut task.projection, index, shifted, spread);
						origins.tasks.push(task);
					}
					for mut closure in child.closures {
						prefix_item_projection(&mut closure.projection, index, shifted, spread);
						origins.closures.push(closure);
					}
					shifted |= spread;
				}
			}
			View::Assignment {
				value: Some(value), ..
			} => origins = self.evaluated_origins(value),
			_ => {
				self.ensure_builtin_reference(node);
				origins = self
					.reference_results
					.get(&node)
					.cloned()
					.unwrap_or_else(|| ValueOrigins {
						places: self.places_of(node),
						..ValueOrigins::default()
					});
				origins.owned = origins.places.is_empty();
				origins.opaque = !origins.bounds.is_empty();
			}
		}
		if let crate::types::TypeKind::Function(sig) = self.s.types.kind(self.type_of(node))
			&& let Resolution::External(key) = self.reference_callee_resolution(node)
		{
			origins.closures.push(ClosureOrigin {
				projection: Vec::new(),
				callable: CallableOrigin::External {
					key,
					reference: self.s.types.sig(sig).reference,
				},
			});
		}
		self.record_produced_task_origins(node, &mut origins);
		if self.type_of(node) == TypeId::NEVER {
			origins = ValueOrigins::default();
		}
		origins.readonly |= origins
			.places
			.iter()
			.any(|&place| self.readonly_place(place));
		origins.places.sort_unstable();
		origins.places.dedup();
		origins.closures.sort_unstable();
		origins.closures.dedup();
		origins.normalize_tasks();
		self.value_origins.insert(node, origins);
	}

	fn project_origins(&self, mut origins: ValueOrigins, step: Step) -> ValueOrigins {
		origins.tasks.retain_mut(|task| {
			let Some(first) = task.projection.first() else {
				return task.descendants;
			};
			let matches = !self.steps_distinct(*first, step);
			if matches {
				task.projection.remove(0);
			}
			matches
		});
		origins.closures.retain_mut(|closure| {
			let Some(first) = closure.projection.first() else {
				return false;
			};
			let matches = !self.steps_distinct(*first, step);
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
			let overlap = steps
				.iter()
				.zip(&stored_steps)
				.all(|(a, b)| !self.steps_distinct(*a, *b));
			if overlap && steps.len() >= stored_steps.len() {
				let mut value = self.facts.origins.get(id).clone();
				for &step in &steps[stored_steps.len()..] {
					value = self.project_origins(value, step);
				}
				result.join(&value);
			} else if overlap && stored_steps.len() > steps.len() {
				let mut value = self.facts.origins.get(id).clone();
				for task in &mut value.tasks {
					task.projection
						.splice(0..0, stored_steps[steps.len()..].iter().copied());
				}
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
		let mut origins = self.evaluated_origins(value).copied();
		// A narrower ordinary projection does not retain omitted Task fields.
		if self
			.s
			.types
			.is_ordinary(self.a.coercion(value).unwrap_or(self.type_of(value)))
		{
			origins.tasks.clear();
		}
		let mut replaced_tasks = Vec::new();
		let strong = places.len() == 1
			&& self.place_steps(places[0]).iter().all(|step| {
				matches!(
					step,
					Step::Field(_) | Step::Index(Identity::Exact(_) | Identity::Key(_))
				)
			});
		for place in places {
			if strong {
				replaced_tasks.extend(
					self.stored_origins(place)
						.tasks
						.into_iter()
						.map(|task| task.start),
				);
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
						.tasks
						.retain(|task| !task.projection.starts_with(suffix));
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
			if origins.closures.is_empty() && origins.tasks.is_empty() {
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
		self.end_unowned_tasks(&replaced_tasks, &[]);
	}

	/// Removing an owning path transfers its Task payloads, without ending
	/// their borrows. The receiving store uses the value's frozen snapshot.
	pub(crate) fn forget_transferred_task_origins(&mut self, value: NodeId) {
		let places = self.places_of(value);
		self.forget_task_place_origins(&places);
	}

	pub(super) fn forget_task_place_origins(&mut self, places: &[PlaceId]) {
		let rows = std::mem::take(&mut self.body.flow.values);
		for (stored, id) in rows {
			let mut origins = self.facts.origins.get(id).clone();
			let stored_steps = self.place_steps(stored);
			for &place in places {
				if self.places.get(place).root != self.places.get(stored).root {
					continue;
				}
				let steps = self.place_steps(place);
				if stored_steps.starts_with(&steps) {
					origins.tasks.clear();
				} else if steps.starts_with(&stored_steps) {
					let suffix = &steps[stored_steps.len()..];
					origins
						.tasks
						.retain(|task| !task.projection.starts_with(suffix));
				}
			}
			if !origins.closures.is_empty() || !origins.tasks.is_empty() {
				let id = self.facts.origins.intern(origins);
				self.body.flow.values.push((stored, id));
			}
		}
	}

	pub(super) fn stored_task_starts(&self, symbol: SymbolId) -> Vec<NodeId> {
		let mut starts = Vec::new();
		for &(place, id) in &self.body.flow.values {
			if self.places.get(place).root == crate::flow::PlaceRoot::Symbol(symbol) {
				starts.extend(
					self.facts
						.origins
						.get(id)
						.tasks
						.iter()
						.map(|task| task.start),
				);
			}
		}
		starts.sort_unstable();
		starts.dedup();
		starts
	}

	pub(super) fn task_has_stored_owner(&self, start: NodeId) -> bool {
		self.body.flow.values.iter().any(|&(_, id)| {
			self.facts
				.origins
				.get(id)
				.tasks
				.iter()
				.any(|task| task.start == start)
		})
	}

	pub(super) fn record_capture_origins(&mut self, lambda: NodeId) {
		let captures = self
			.a
			.functions
			.get(&lambda)
			.map(|info| info.captures.clone())
			.unwrap_or_default();
		let mut origins = Vec::new();
		for symbol in captures {
			let places = self.symbol_places(symbol);
			let mut closures = Vec::new();
			for &place in &places {
				closures.extend(
					self.stored_origins(place)
						.closures
						.into_iter()
						.filter_map(|origin| origin.callable.local()),
				);
			}
			let declaration = self.a.symbols.symbol(symbol).decl_node;
			if self.a.symbols.symbol(symbol).kind == SymbolKind::Function
				&& let Some(info) = self.a.functions.get(&declaration)
			{
				closures.push(info.lambda);
			}
			origins.push(CapturedOrigins {
				symbol,
				places,
				closures,
			});
		}
		self.capture_origins.insert(lambda, origins);
	}

	pub(super) fn bind_value_origins(&mut self, symbol: SymbolId, value: NodeId) {
		let place = self.places.symbol(symbol);
		self.store_value_origins(vec![place], value);
	}
}

fn prefix_item_projection(projection: &mut Vec<Step>, index: usize, shifted: bool, spread: bool) {
	if spread && matches!(projection.first(), Some(Step::Index(_))) {
		projection[0] = Step::Index(Identity::Unknown);
	} else {
		projection.insert(
			0,
			Step::Index(if shifted || spread {
				Identity::Unknown
			} else {
				Identity::Exact(index as i64)
			}),
		);
	}
}
