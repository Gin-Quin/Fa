//! Positive source-use witnesses for borrowed calls and callback arguments.
//! Conditional or incomplete dependencies do not prove a witness here; this
//! query never supplies a complete liveness or ownership-transfer proof.

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{
	Argument, Callee, OwnershipContext, OwnershipEntry, OwnershipEntryMode, OwnershipSourceOrigin,
};
use crate::check::Walker;
use crate::flow::PlaceRoot;
use crate::ids::{NodeId, SymbolId};

type Roots = FxHashSet<SymbolId>;
type Summaries = FxHashMap<OwnershipEntry, Roots>;

impl Walker<'_> {
	pub(super) fn borrowed_call_observations(&mut self) -> FxHashMap<NodeId, Roots> {
		let summaries = self.unconditional_borrowed_sources();
		let mut observed = FxHashMap::default();
		let calls: Vec<_> = self
			.a
			.calls
			.iter()
			.map(|(&node, call)| (node, call.clone()))
			.collect();
		for (node, call) in calls {
			let functions = match call.callee {
				Callee::Function(function) => vec![function],
				Callee::Value => self.callable_borrowed_targets(node),
				_ => Vec::new(),
			};
			let roots = observed.entry(node).or_insert_with(Roots::default);
			for function in functions {
				if let Some(info) = self.a.function(function) {
					if self.a.escapes.contains(&info.lambda) {
						continue;
					}
					extend_context(roots, &summaries, OwnershipContext::Body(info.lambda));
					for (param, argument) in info.params.iter().zip(&call.arguments) {
						if matches!(argument, Argument::Default)
							&& let Some(default) = param.default
						{
							extend_context(roots, &summaries, OwnershipContext::Default(default));
						}
					}
				}
			}
			for argument in &call.arguments {
				let values: &[NodeId] = match argument {
					Argument::Given(value) => std::slice::from_ref(value),
					Argument::Variadic(values) => values,
					_ => continue,
				};
				for &value in values {
					for function in self.callable_targets(value) {
						let Some(info) = self.a.function(function) else {
							continue;
						};
						if self.a.escapes.contains(&info.lambda) {
							continue;
						}
						// Even an ignored borrowed callback keeps its captures usable for
						// the whole call, including inputs used only by its defaults.
						extend_context(roots, &summaries, OwnershipContext::Body(info.lambda));
						for default in info.params.iter().filter_map(|param| param.default) {
							extend_context(roots, &summaries, OwnershipContext::Default(default));
						}
					}
				}
			}
		}
		observed
	}

	fn unconditional_borrowed_sources(&self) -> Summaries {
		let graph = &self.a.ownership_source_graph;
		let default_owners: FxHashMap<_, _> = self
			.a
			.functions
			.values()
			.flat_map(|info| {
				info.params
					.iter()
					.filter_map(|param| param.default.map(|default| (default, info.lambda)))
			})
			.collect();
		let mut sources = Summaries::default();
		let mut reverse: FxHashMap<OwnershipEntry, Vec<OwnershipEntry>> = FxHashMap::default();
		for (&entry, data) in &graph.entries {
			if data.pending {
				continue;
			}
			let owner = entry_owner(entry, &default_owners);
			let roots = sources.entry(entry).or_default();
			for source in data.reads.values().flatten() {
				if source.origin == OwnershipSourceOrigin::Direct
					&& let PlaceRoot::Symbol(symbol) = source.root
					&& self.ownership_body(self.a.symbols.symbol(symbol).decl_node) != owner
				{
					roots.insert(symbol);
				}
			}
			for edge in &data.calls {
				if edge.guard.is_none() {
					reverse.entry(edge.target).or_default().push(entry);
				}
			}
		}
		// This positive-witness fast path is limited to read-only captured
		// bindings. A replacement can make later reads refer to a new value;
		// leave such calls to the complete lifetime analysis.
		let writers: FxHashSet<_> = self
			.move_writes
			.iter()
			.filter_map(|(&node, &place)| {
				let symbol = self.place_symbol(place)?;
				let owner = self.ownership_body(node);
				(self.places.steps(place).is_empty()
					&& self.ownership_body(self.a.symbols.symbol(symbol).decl_node) != owner)
					.then_some(owner)
			})
			.collect();
		let mut blocked: FxHashSet<_> = graph
			.entries
			.iter()
			.filter_map(|(&entry, data)| {
				(data.pending || writers.contains(&entry_owner(entry, &default_owners)))
					.then_some(entry)
			})
			.collect();
		let mut pending: Vec<_> = blocked.iter().copied().collect();
		while let Some(entry) = pending.pop() {
			for &caller in reverse.get(&entry).into_iter().flatten() {
				if blocked.insert(caller) {
					pending.push(caller);
				}
			}
		}
		sources.retain(|entry, _| !blocked.contains(entry));
		let mut queue: VecDeque<_> = sources.keys().copied().collect();
		let mut queued: FxHashSet<_> = queue.iter().copied().collect();
		while let Some(entry) = queue.pop_front() {
			queued.remove(&entry);
			let incoming = sources.get(&entry).cloned().unwrap_or_default();
			for &caller in reverse.get(&entry).into_iter().flatten() {
				if blocked.contains(&caller) {
					continue;
				}
				let roots = sources.entry(caller).or_default();
				let previous = roots.len();
				let owner = entry_owner(caller, &default_owners);
				roots.extend(incoming.iter().copied().filter(|&symbol| {
					self.ownership_body(self.a.symbols.symbol(symbol).decl_node) != owner
				}));
				if roots.len() != previous && queued.insert(caller) {
					queue.push_back(caller);
				}
			}
		}
		sources
	}
}

fn extend_context(roots: &mut Roots, summaries: &Summaries, context: OwnershipContext) {
	let entry = OwnershipEntry {
		context,
		mode: OwnershipEntryMode::Borrowed,
	};
	roots.extend(summaries.get(&entry).into_iter().flatten().copied());
}

fn entry_owner(entry: OwnershipEntry, defaults: &FxHashMap<NodeId, NodeId>) -> NodeId {
	match entry.context {
		OwnershipContext::Body(node) => node,
		OwnershipContext::Default(node) => defaults.get(&node).copied().unwrap_or(node),
	}
}
