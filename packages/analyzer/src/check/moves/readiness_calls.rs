//! Invocation-site propagation of declaration readiness. There are at most
//! two entries per body/default: borrowed and owned, never one per caller.

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::Graph;
use crate::analysis::{
	Argument, Callee, DeclarationReadiness as Ready, FunctionCaptureState, NodeFlags,
};
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

#[path = "readiness_sources.rs"]
mod source_graph;

const EARLY: u8 = 1;
const READY: u8 = 2;

#[derive(Clone, Copy)]
struct Declaration {
	lambda: NodeId,
	node: NodeId,
	symbol: SymbolId,
	owner: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct Entry {
	context: NodeId,
	owned: bool,
}

struct Invocation {
	site: NodeId,
	caller: NodeId,
	target: NodeId,
	direct: bool,
	defaults: Vec<NodeId>,
}

struct Propagation {
	declarations: Vec<Declaration>,
	local: FxHashMap<(NodeId, NodeId), u8>,
	inherited: FxHashMap<Entry, Vec<u8>>,
	owners: FxHashMap<NodeId, NodeId>,
	outgoing: FxHashMap<NodeId, Vec<Invocation>>,
	unresolved: FxHashSet<NodeId>,
}

impl Propagation {
	fn phase(&self, entry: Entry, declaration: usize, site: NodeId) -> u8 {
		let data = self.declarations[declaration];
		let owner = self
			.owners
			.get(&entry.context)
			.copied()
			.unwrap_or(entry.context);
		if data.owner == owner {
			// Defaults run before declarations in the function body.
			if entry.context != owner {
				return EARLY;
			}
			return self.local.get(&(site, data.node)).copied().unwrap_or(0);
		}
		if entry.owned {
			return READY;
		}
		self.inherited.get(&entry).map_or(0, |row| row[declaration])
	}

	fn transfer(&mut self, source: Entry, site: NodeId, target: Entry) -> bool {
		let phases: Vec<_> = (0..self.declarations.len())
			.map(|index| self.phase(source, index, site))
			.collect();
		let row = self
			.inherited
			.entry(target)
			.or_insert_with(|| vec![0; phases.len()]);
		let mut changed = false;
		for (value, phase) in row.iter_mut().zip(phases) {
			changed |= *value | phase != *value;
			*value |= phase;
		}
		changed
	}
}

impl Walker<'_> {
	pub(super) fn propagate_function_readiness(&mut self) {
		let defaults: FxHashSet<_> = self
			.a
			.functions
			.values()
			.flat_map(|info| info.params.iter().filter_map(|param| param.default))
			.collect();
		let mut declarations: Vec<_> = self
			.a
			.functions
			.values()
			.filter_map(|info| {
				let node = info.declaration?;
				let symbol = info.symbol?;
				(self.a.function_capture_states.get(&info.lambda)
					== Some(&FunctionCaptureState::Capturing))
				.then_some(Declaration {
					lambda: info.lambda,
					node,
					symbol,
					owner: self.ownership_body(node),
				})
			})
			.collect();
		declarations.sort_unstable_by_key(|data| data.node);
		declarations.dedup_by_key(|data| data.node);
		let marker = declarations.first().map(|data| data.symbol);
		let mut propagation = Propagation {
			declarations,
			local: FxHashMap::default(),
			inherited: FxHashMap::default(),
			owners: FxHashMap::default(),
			outgoing: FxHashMap::default(),
			unresolved: FxHashSet::default(),
		};
		let mut bodies = FxHashMap::default();
		bodies.insert(NodeId::new(self.tree.root), NodeId::new(self.tree.root));
		for info in self.a.functions.values() {
			bodies.insert(info.lambda, info.body);
			for default in info.params.iter().filter_map(|param| param.default) {
				bodies.insert(default, default);
				propagation.owners.insert(default, info.lambda);
			}
		}
		(propagation.outgoing, propagation.unresolved) = self.readiness_invocations(&defaults);
		if let Some(marker) = marker {
			self.readiness_barriers(&mut propagation, &bodies, &defaults, marker);
		}
		self.solve_readiness(&mut propagation, &bodies);
		self.publish_propagated_readiness(&propagation, &defaults);
		self.publish_ownership_source_graph(&propagation, &bodies, &defaults);
	}

	fn solve_readiness(&self, propagation: &mut Propagation, bodies: &FxHashMap<NodeId, NodeId>) {
		let mut queue = VecDeque::new();
		let mut queued = FxHashSet::default();
		for &context in bodies.keys() {
			for owned in [false, true] {
				let owner = propagation.owners.get(&context).copied().unwrap_or(context);
				if owned
					&& (!self.a.escapes.contains(&owner)
						|| self.a.function_capture_states.get(&owner)
							!= Some(&FunctionCaptureState::Capturing))
				{
					continue;
				}
				let entry = Entry { context, owned };
				propagation
					.inherited
					.insert(entry, vec![0; propagation.declarations.len()]);
				queue.push_back(entry);
				queued.insert(entry);
			}
		}
		while let Some(entry) = queue.pop_front() {
			queued.remove(&entry);
			let edges = propagation
				.outgoing
				.remove(&entry.context)
				.unwrap_or_default();
			for edge in &edges {
				debug_assert_eq!(edge.caller, entry.context);
				let modes = self.readiness_invocation_modes(propagation, entry, edge);
				for owned in [false, true] {
					if modes & if owned { READY } else { EARLY } == 0 {
						continue;
					}
					for context in std::iter::once(edge.target).chain(edge.defaults.iter().copied())
					{
						let target = Entry { context, owned };
						if propagation.transfer(entry, edge.site, target) && queued.insert(target) {
							queue.push_back(target);
						}
					}
				}
			}
			propagation.outgoing.insert(entry.context, edges);
		}
	}

	fn readiness_invocations(
		&mut self,
		defaults: &FxHashSet<NodeId>,
	) -> (FxHashMap<NodeId, Vec<Invocation>>, FxHashSet<NodeId>) {
		let mut outgoing: FxHashMap<NodeId, Vec<Invocation>> = FxHashMap::default();
		let mut unresolved = FxHashSet::default();
		for index in 0..self.tree.nodes.len() {
			let site = NodeId::new(index);
			let View::Call {
				callee, arguments, ..
			} = self.view(site)
			else {
				continue;
			};
			if self.a.flags_of(site).intersects(
				NodeFlags::TYPE_EXPR | NodeFlags::GENERIC_APPLICATION | NodeFlags::UNREACHABLE,
			) {
				continue;
			}
			let arguments: Vec<_> = arguments.iter().collect();
			let caller = self.ownership_read_context(site, defaults);
			let info = self.a.calls.get(&site).cloned();
			let direct = info
				.as_ref()
				.and_then(|info| match info.callee {
					Callee::Function(function) => Some(function),
					_ => None,
				})
				.or_else(|| {
					let node = self.strip_groups(callee);
					let Resolution::Local(symbol) = self.a.resolution(node) else {
						return None;
					};
					let data = self.a.symbols.symbol(symbol);
					(data.kind == SymbolKind::Function).then_some(data.decl_node)
				});
			let targets = direct.map_or_else(
				|| self.callable_borrowed_targets(site),
				|function| vec![function],
			);
			if targets.is_empty()
				&& info
					.as_ref()
					.is_some_and(|info| info.callee == Callee::Value)
			{
				unresolved.insert(caller);
			}

			for function in targets {
				let Some(function) = self.a.function(function) else {
					continue;
				};
				let omitted = info.as_ref().map_or_else(Vec::new, |call| {
					function
						.params
						.iter()
						.zip(&call.arguments)
						.filter_map(|(param, argument)| {
							matches!(argument, Argument::Default)
								.then_some(param.default)
								.flatten()
						})
						.collect()
				});
				outgoing.entry(caller).or_default().push(Invocation {
					site,
					caller,
					target: function.lambda,
					direct: direct.is_some(),
					defaults: omitted,
				});
			}
			// A borrowed callback may run throughout the receiving call. Known
			// handle origins are used even when the callee ignores its parameter.
			for argument in arguments {
				let value = match self.view(argument) {
					View::Assignment {
						value: Some(value), ..
					} => value,
					_ => argument,
				};
				for target in self.callable_targets(value) {
					if let Some(function) = self.a.function(target) {
						outgoing.entry(caller).or_default().push(Invocation {
							site,
							caller,
							target: function.lambda,
							direct: false,
							defaults: Vec::new(),
						});
					}
				}
			}
		}

		(outgoing, unresolved)
	}
	fn publish_propagated_readiness(
		&mut self,
		propagation: &Propagation,
		defaults: &FxHashSet<NodeId>,
	) {
		let mut published = std::mem::take(&mut self.a.named_function_readiness);
		for (&node, state) in &mut published {
			if state.borrowed != Ready::Pending
				|| state.owned == Ready::Pending
				|| self.a.type_of(node) == TypeId::UNSET
				|| self
					.a
					.ownership_sources
					.pending_functions
					.contains(&self.ownership_body(node))
			{
				continue;
			}
			let Some(index) = propagation
				.declarations
				.iter()
				.position(|data| data.node == state.declaration)
			else {
				continue;
			};
			let context = self.ownership_read_context(node, defaults);
			let phase = propagation.phase(
				Entry {
					context,
					owned: false,
				},
				index,
				node,
			);
			state.borrowed = match phase {
				EARLY => Ready::Early,
				READY => Ready::Ready,
				3 => Ready::Mixed,
				_ => Ready::Pending,
			};
		}
		self.a.named_function_readiness = published;
	}

	fn readiness_barriers(
		&self,
		propagation: &mut Propagation,
		bodies: &FxHashMap<NodeId, NodeId>,
		defaults: &FxHashSet<NodeId>,
		marker: SymbolId,
	) {
		let mut reads = self.move_reads.clone();
		for &node in self.a.named_function_readiness.keys() {
			reads.entry(node).or_default().push(marker);
		}
		for edge in propagation.outgoing.values().flatten() {
			reads.entry(edge.site).or_default().push(marker);
		}
		let mut sites: Vec<_> = self.a.named_function_readiness.keys().copied().collect();
		sites.extend(
			propagation
				.outgoing
				.values()
				.flatten()
				.map(|edge| edge.site),
		);
		let mut graphs = FxHashMap::default();
		let mut barriers = FxHashMap::default();
		let commits = FxHashMap::default();
		for site in sites {
			let context = self.ownership_read_context(site, defaults);
			let Some(&body) = bodies.get(&context) else {
				continue;
			};
			let graph = graphs
				.entry(context)
				.or_insert_with(|| Graph::build(self, body, &reads, &commits));
			for declaration in &propagation.declarations {
				if declaration.owner != context {
					continue;
				}
				let block = self.function_declaration_block(declaration.node);
				let positions = barriers
					.entry((context, declaration.symbol))
					.or_insert_with(|| graph.before_definition(block, declaration.symbol));
				let phase = positions
					.as_ref()
					.and_then(|positions| graph.definition_reached(site, positions))
					.map_or(0, |ready| if ready { READY } else { EARLY });
				propagation.local.insert((site, declaration.node), phase);
			}
		}
	}
	fn readiness_invocation_modes(
		&self,
		propagation: &Propagation,
		entry: Entry,
		edge: &Invocation,
	) -> u8 {
		let escapes = self.a.escapes.contains(&edge.target)
			&& self.a.function_capture_states.get(&edge.target)
				== Some(&FunctionCaptureState::Capturing);
		let owner = propagation
			.owners
			.get(&entry.context)
			.copied()
			.unwrap_or(entry.context);
		if !escapes {
			EARLY
		} else if edge.target == owner {
			if entry.owned { READY } else { EARLY }
		} else if !edge.direct {
			READY
		} else {
			propagation
				.declarations
				.iter()
				.position(|data| data.lambda == edge.target)
				.map_or(0, |index| propagation.phase(entry, index, edge.site))
		}
	}
}
