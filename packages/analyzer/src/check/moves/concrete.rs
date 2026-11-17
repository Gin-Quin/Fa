//! Concrete ownership decisions over runtime control flow. Generic-dependent
//! stores are left to instantiation conditions, not guessed from constraints.

use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::Graph;
use crate::analysis::{Argument, Callee, FieldValue, NodeFlags, OwnershipContext};
use crate::check::Walker;
use crate::ids::{NodeId, PlaceId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

impl Walker<'_> {
	pub(super) fn finish_concrete_moves(&mut self) -> FxHashMap<NodeId, NodeId> {
		let mut conflicts = FxHashMap::default();
		self.publish_callable_capture_facts();
		let borrowed_calls = self.borrowed_call_observations();
		self.collect_store_values();
		let reads = self.ownership_reads();
		self.finish_ownership_task_borrows(&reads);
		let mut commits: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
		for value in self.copy_sites.iter().copied() {
			commits.entry(value).or_default().push(value);
		}
		self.mark_reserved_module_reads(&reads, &commits);
		let root = NodeId::new(self.tree.root);
		let mut bodies = vec![root];
		bodies.extend(self.a.functions.values().map(|info| info.body));
		bodies.extend(
			self.a
				.functions
				.values()
				.flat_map(|info| info.params.iter().filter_map(|param| param.default)),
		);
		bodies.extend(self.a.starts.values().filter_map(|info| info.block));
		bodies.extend(self.a.watches.values().map(|info| info.body));
		for index in 0..self.tree.nodes.len() {
			if let View::Test { body, .. } = self.view(NodeId::new(index)) {
				bodies.push(body);
			}
		}
		bodies.sort_unstable();
		bodies.dedup();
		for body in bodies {
			let graph = Graph::build(self, body, &reads, &commits);
			let mut sources: FxHashMap<SymbolId, Vec<NodeId>> = FxHashMap::default();
			for &value in graph.stores.keys() {
				if matches!(self.view(value), View::Name(_))
					&& let Resolution::Local(symbol) = self.a.resolution(value)
					&& matches!(
						self.store_ownership(value),
						crate::types::Ownership::Structure | crate::types::Ownership::Unique
					) {
					sources.entry(symbol).or_default().push(value);
				}
			}
			for (symbol, values) in sources {
				let later = graph.later_uses(symbol);
				for value in values {
					if self.pending_ownership_task(value, symbol).is_some()
						|| self.module_owner_loans.contains_key(&(value, symbol))
					{
						self.clear_flag(value, NodeFlags::MOVE);
						self.a.moved_reads.remove(&value);
						continue;
					}
					let positions = &graph.stores[&value];
					let reachable = positions.iter().any(|&index| graph.reachable[index]);
					let conflict = positions
						.iter()
						.filter(|&&index| graph.reachable[index])
						.find_map(|&index| later[index]);
					if let Some(witness) = conflict {
						conflicts.insert(value, witness);
					}
					let actual = self.a.type_of(value);
					let subject = self.a.coercion(value).unwrap_or(actual);
					let projection = self.s.types.store_projection(actual, subject);
					let unique = projection.ownership == crate::types::Ownership::Unique;
					let complete = !self.s.types.has_lifetime_boundary(actual)
						|| self.s.types.preserves_lifetime_owner(actual, subject);
					if reachable
						&& complete && !projection.needs_field_moves
						&& self.ownership_source_owned(symbol, value)
						&& (unique || conflict.is_none())
					{
						self.set_flag(value, NodeFlags::MOVE);
						if unique {
							self.a.moved_reads.insert(value);
						}
					}
				}
			}
			self.check_structure_capture_stores(&graph, &borrowed_calls);
			self.check_move_validity(&graph);
		}
		conflicts
	}

	fn collect_store_values(&mut self) {
		let mut values: Vec<_> = self
			.a
			.objects
			.values()
			.flat_map(|object| {
				object.fields.iter().filter_map(|(_, value)| match *value {
					FieldValue::Given(value) | FieldValue::Default(value) => Some(value),
					_ => None,
				})
			})
			.collect();
		// Enclosing defaults borrow. A local constructed by the argument must
		// leave its inner scope, just like a written block-valued argument.
		for info in self.a.functions.values() {
			for param in &info.params {
				let Some(default) = param.default else {
					continue;
				};
				if self
					.s
					.types
					.parameter_can_be_consumed(self.a.symbols.symbol(param.symbol).ty)
				{
					values.push(default);
					continue;
				}
				let mut pending = vec![default];
				while let Some(value) = pending.pop() {
					let inputs = match self.view(value) {
						View::Loop { .. } => {
							Some(self.a.loop_results.get(&value).cloned().unwrap_or_default())
						}
						_ => crate::lower::copies::result_inputs(&self.tree, value),
					};
					if let Some(inputs) = inputs {
						pending.extend(inputs);
					} else if crate::lower::copies::argument_local_root(
						&self.tree, &self.a, default, value,
					)
					.is_some()
					{
						values.push(value);
					}
				}
			}
		}
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if self.a.has_flags(node, NodeFlags::TYPE_EXPR) {
				continue;
			}
			match self.view(node) {
				View::Array { items } | View::Tuple { items } => {
					values.extend(
						items
							.iter()
							.filter(|&item| !matches!(self.view(item), View::Spread(_))),
					);
				}
				View::Throw(value) | View::ExportValue { value, .. } => values.push(value),
				_ => {}
			}
		}
		for value in values {
			if !self.is_relation_link_store(value) {
				self.record_copy_sources(value);
			}
		}
		// Object metadata is complete now. A side's initializer may have
		// been visited before its containing object was recorded.
		let sites: Vec<_> = self.copy_sites.iter().copied().collect();
		for value in sites {
			if self.is_relation_link_store(value) {
				self.copy_sites.remove(&value);
			}
		}
	}

	/// The runtime body owning a declaration or read. A nested function
	/// borrows its nonlocal values, even if they are otherwise owned locals.
	pub(crate) fn ownership_body(&self, mut node: NodeId) -> NodeId {
		while let Some(parent) = self.parent(node) {
			if matches!(
				self.view(parent),
				View::Lambda { .. } | View::Test { .. } | View::Watch(_)
			) {
				return parent;
			}
			if matches!(self.view(node), View::Do(_)) && matches!(self.view(parent), View::Start(_))
			{
				return node;
			}
			node = parent;
		}
		node
	}

	pub(crate) fn ownership_source_owned(&self, symbol: SymbolId, read: NodeId) -> bool {
		let data = self.a.symbols.symbol(symbol);
		(matches!(
			data.kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::CatchBinding
		) || (matches!(data.kind, SymbolKind::Parameter { .. })
			&& self.s.types.parameter_can_be_consumed(data.ty)))
			&& self.ownership_body(data.decl_node) == self.ownership_body(read)
	}

	/// A local call observes its captures when it runs, after evaluating
	/// arguments. Include globals and transitive local calls in that read.
	fn ownership_reads(&mut self) -> FxHashMap<NodeId, Vec<SymbolId>> {
		let mut reads = self.move_reads.clone();
		// An owned named handle observes its declaration environment. Reads
		// of its spelling no longer keep the original payload alive.
		for (&node, symbols) in &mut reads {
			if !matches!(self.view(node), View::Name(_)) {
				continue;
			}
			if let Resolution::Local(symbol) = self.a.resolution(node)
				&& self.a.symbols.symbol(symbol).kind == SymbolKind::Function
				&& let Some(function) = self.direct_function(symbol)
				&& let Some(info) = self.a.function(function)
				&& self.a.escapes.contains(&info.lambda)
			{
				symbols.retain(|read| *read == symbol);
			}
		}
		// Alias construction can preserve more current alternatives than
		// the reference checker's single canonical walk-order path. Keep
		// each source live; storage-origin filtering belongs to the later
		// borrowed/owned invocation graph, not this conservative seed.
		for (node, sources) in self.ownership_read_origins.clone() {
			let function = self.ownership_body(node);
			// The current move graph observes borrowed source scopes. Owned
			// invocation summaries will select the environment boundary when
			// declaration bindings and capture transfer plans activate together.
			let sources = crate::flow::ownership::boundary_sources(
				&mut self.places,
				sources,
				function,
				false,
				&self.ownership_capture_origins,
			);
			self.observe_places(
				node,
				sources.into_iter().map(|source| source.place),
				&mut reads,
			);
		}
		self.capture_creation_reads(&mut reads);
		let defaults: FxHashSet<_> = self
			.a
			.functions
			.values()
			.flat_map(|info| info.params.iter().filter_map(|param| param.default))
			.collect();
		for (call, place) in self.lifetime_ends.clone() {
			self.observe_places(call, [place], &mut reads);
		}
		let calls: Vec<_> = self
			.a
			.calls
			.iter()
			.map(|(&call, info)| (call, info.clone()))
			.collect();
		// Arguments and method receivers borrow until the call completes,
		// including while later arguments are evaluated.
		for (call, info) in &calls {
			for argument in &info.arguments {
				let Argument::Given(node) = *argument else {
					continue;
				};
				if self.copy_sites.contains(&self.strip_groups(node))
					|| self
						.a
						.argument_ownership
						.get(&node)
						.copied()
						.unwrap_or_else(|| {
							self.s.types.parameter_can_be_consumed(self.a.type_of(node))
						}) {
					continue;
				}
				let places = self.read_places(node);
				self.observe_places(*call, places, &mut reads);
			}
			if let View::Call { callee, .. } = self.view(*call) {
				let consumes_receiver = self.builtin_path(*call, callee).is_some_and(|path| {
					matches!(
						path,
						"Subscription#stop"
							| "Worker#stop" | "WorkerPool#shutdown"
							| "TcpListener#close" | "TcpStream#close"
					)
				});
				let stores_receiver = matches!(self.view(callee), View::Access { target, .. } if self.copy_sites.contains(&self.strip_groups(target)));
				if stores_receiver {
					// Selecting a declared method borrows the same receiver
					// before its task store; it is not a later payload use.
					reads.remove(&callee);
				}
				if !consumes_receiver
					&& !stores_receiver
					&& matches!(self.view(callee), View::Access { .. })
				{
					let places = self.move_places.get(&callee).cloned().unwrap_or_default();
					self.observe_places(*call, places, &mut reads);
				}
			}
		}
		let incoming = self.incoming_source_reads(&reads, &defaults);
		let mut free: FxHashMap<OwnershipContext, FxHashSet<PlaceId>> = FxHashMap::default();
		for (&node, symbols) in &reads {
			if self.access_roots.contains(&node) {
				continue;
			}
			let body = self.invocation_context(node, &defaults);
			let entry = self.ownership_observation_entry(body);
			if let Some(data) = self.a.ownership_source_graph.entries.get(&entry)
				&& !data.pending
			{
				let sources = data.reads.get(&node).cloned().unwrap_or_default();
				for source in sources {
					if let crate::flow::PlaceRoot::Symbol(symbol) = source.root
						&& self
							.invocation_context(self.a.symbols.symbol(symbol).decl_node, &defaults)
							!= body && self.observes_incoming_owner(&incoming, body, node, symbol)
					{
						let paths: Vec<_> = self
							.move_places
							.get(&node)
							.into_iter()
							.flatten()
							.copied()
							.filter(|&place| self.place_symbol(place) == Some(symbol))
							.collect();
						if paths.is_empty() {
							free.entry(body)
								.or_default()
								.insert(self.places.symbol(symbol));
						} else {
							free.entry(body).or_default().extend(paths);
						}
					}
				}
				continue;
			}
			for &symbol in symbols {
				if !self.observes_incoming_owner(&incoming, body, node, symbol) {
					continue;
				}
				if self.invocation_context(self.a.symbols.symbol(symbol).decl_node, &defaults)
					== body
				{
					continue;
				}
				let paths: Vec<_> = self
					.move_places
					.get(&node)
					.into_iter()
					.flatten()
					.copied()
					.filter(|&place| self.place_symbol(place) == Some(symbol))
					.collect();
				if paths.is_empty() {
					free.entry(body)
						.or_default()
						.insert(self.places.symbol(symbol));
				} else {
					free.entry(body).or_default().extend(paths);
				}
			}
		}
		loop {
			let mut changed = false;
			for (call, info) in &calls {
				let functions = match info.callee {
					Callee::Function(function) => vec![function],
					Callee::Value => self.callable_borrowed_targets(*call),
					_ => Vec::new(),
				};
				for function in functions {
					let Some(callee) = self.a.functions.get(&function) else {
						continue;
					};
					let callee = callee.clone();
					let mut places = free
						.get(&OwnershipContext::Body(callee.lambda))
						.cloned()
						.unwrap_or_default();
					let mut bindings = FxHashMap::default();
					for (param, argument) in callee.params.iter().zip(&info.arguments) {
						let argument_places = match *argument {
							Argument::Given(value) | Argument::Receiver(value) => {
								self.ownership_result_places(value)
							}
							Argument::Default => {
								if let Some(default) = param.default {
									for &place in free
										.get(&OwnershipContext::Default(default))
										.into_iter()
										.flatten()
									{
										places.extend(
											self.substitute_argument_places(place, &bindings),
										);
									}
									self.ownership_result_places(default)
										.into_iter()
										.flat_map(|place| {
											self.substitute_argument_places(place, &bindings)
										})
										.collect()
								} else {
									Vec::new()
								}
							}
							_ => Vec::new(),
						};
						bindings.insert(param.symbol, argument_places);
					}
					self.expand_callable_capture_reads(*call, &free, &mut places);
					changed |= self
						.observe_call_ownership(*call, places, &defaults, &mut free, &mut reads);
				}
				// Passing a borrowed callback uses its captured references for
				// the duration of this call, even through a saved handle.
				let mut places = self.borrowed_callback_argument_reads(&info.arguments, &free);
				self.expand_callable_capture_reads(*call, &free, &mut places);
				changed |=
					self.observe_call_ownership(*call, places, &defaults, &mut free, &mut reads);
			}
			if !changed {
				break;
			}
		}
		self.reserve_exported_owner_sources(&free);
		reads
	}

	fn borrowed_callback_argument_reads(
		&self,
		arguments: &[Argument],
		free: &FxHashMap<OwnershipContext, FxHashSet<PlaceId>>,
	) -> FxHashSet<PlaceId> {
		let mut places = FxHashSet::default();
		for argument in arguments {
			let values: &[NodeId] = match argument {
				Argument::Given(value) => std::slice::from_ref(value),
				Argument::Variadic(values) => values,
				_ => continue,
			};
			for &value in values {
				if !self.callable_slot_type(self.a.type_of(value)) {
					continue;
				}
				for function in self.callable_targets(value) {
					if self.a.escapes.contains(&function) {
						continue;
					}
					if let Some(info) = self.a.function(function) {
						places.extend(
							free.get(&OwnershipContext::Body(info.lambda))
								.into_iter()
								.flatten()
								.copied(),
						);
					}
				}
			}
		}
		places
	}

	fn observe_call_ownership(
		&mut self,
		call: NodeId,
		places: FxHashSet<PlaceId>,
		defaults: &FxHashSet<NodeId>,
		free: &mut FxHashMap<OwnershipContext, FxHashSet<PlaceId>>,
		reads: &mut FxHashMap<NodeId, Vec<SymbolId>>,
	) -> bool {
		let body = self.invocation_context(call, defaults);
		let mut changed = false;
		for &place in &places {
			let Some(symbol) = self.place_symbol(place) else {
				continue;
			};
			if self.invocation_context(self.a.symbols.symbol(symbol).decl_node, defaults) != body {
				changed |= free.entry(body).or_default().insert(place);
			}
		}
		self.observe_places(call, places, reads);
		changed
	}

	/// A free callable place is a dependency on its value at this call.
	/// Expand recursively, using the same free-place summaries as ordinary
	/// calls; cycles visit each place once.
	fn expand_callable_capture_reads(
		&self,
		call: NodeId,
		free: &FxHashMap<OwnershipContext, FxHashSet<PlaceId>>,
		places: &mut FxHashSet<PlaceId>,
	) {
		let mut pending: Vec<_> = places.iter().copied().collect();
		while let Some(place) = pending.pop() {
			for function in self.callable_targets_at_call(call, place) {
				let Some(info) = self.a.function(function) else {
					continue;
				};
				for &source in free
					.get(&OwnershipContext::Body(info.lambda))
					.into_iter()
					.flatten()
				{
					if places.insert(source) {
						pending.push(source);
					}
				}
			}
		}
	}

	fn ownership_observation_entry(
		&self,
		context: OwnershipContext,
	) -> crate::analysis::OwnershipEntry {
		use crate::analysis::{OwnershipEntry, OwnershipEntryMode};
		let owner = match context {
			OwnershipContext::Default(default) => self
				.a
				.functions
				.values()
				.find(|info| {
					info.params
						.iter()
						.any(|param| param.default == Some(default))
				})
				.map_or(default, |info| info.lambda),
			OwnershipContext::Body(body) => body,
		};
		OwnershipEntry {
			context,
			mode: if self.a.escapes.contains(&owner) {
				OwnershipEntryMode::Owned
			} else {
				OwnershipEntryMode::Borrowed
			},
		}
	}

	/// A settled invocation graph proves that every possible original-source
	/// observation is present. Mixed readiness and unresolved inputs require
	/// the conditional consumer rather than an empty-observation assumption.
	pub(super) fn capture_observations_complete(&self, creation: NodeId) -> bool {
		let defaults = self
			.a
			.functions
			.values()
			.flat_map(|info| info.params.iter().filter_map(|param| param.default))
			.collect();
		let context = self.invocation_context(creation, &defaults);
		let mut pending = vec![self.ownership_observation_entry(context)];
		let mut seen = FxHashSet::default();
		let versioned = self.versioned_source_roots(&defaults);
		while let Some(entry) = pending.pop() {
			if !seen.insert(entry) {
				continue;
			}
			let Some(data) = self.a.ownership_source_graph.entries.get(&entry) else {
				return false;
			};
			if data.pending {
				return false;
			}
			// Direct whole-slot replacements have an incoming-owner CFG proof.
			// Alias and transitive versions remain separate unresolved inputs.
			if self.move_writes.iter().any(|(&node, &place)| {
				self.invocation_context(node, &defaults) == entry.context
					&& self.places.steps(place).is_empty()
					&& self.place_symbol(place).is_some_and(|symbol| {
						self.ownership_body(self.a.symbols.symbol(symbol).decl_node)
							!= self.ownership_body(node)
					}) && self
					.place_symbol(place)
					.is_some_and(|symbol| !versioned.contains(&(entry.context, symbol)))
			}) {
				return false;
			}
			for edge in &data.calls {
				if edge.guard.is_some_and(|guard| !guard.reached) {
					return false;
				}
				pending.push(edge.target);
			}
		}
		true
	}

	/// Defaults run only at calls which omit them. Keep their observations
	/// separate from the unconditional observations of the function body.
	pub(super) fn ownership_read_context(
		&self,
		node: NodeId,
		defaults: &FxHashSet<NodeId>,
	) -> NodeId {
		self.invocation_context(node, defaults).node()
	}

	fn ownership_result_places(&self, value: NodeId) -> Vec<PlaceId> {
		let mut pending = vec![value];
		let mut places = Vec::new();
		while let Some(value) = pending.pop() {
			if let Some(inputs) = crate::lower::copies::result_inputs(&self.tree, value) {
				pending.extend(inputs);
			} else {
				places.extend(self.read_places(value));
			}
		}
		places
	}

	fn substitute_argument_places(
		&mut self,
		place: PlaceId,
		bindings: &FxHashMap<SymbolId, Vec<PlaceId>>,
	) -> Vec<PlaceId> {
		let Some(actual) = self
			.place_symbol(place)
			.and_then(|symbol| bindings.get(&symbol))
		else {
			return vec![place];
		};
		let mut steps = Vec::new();
		let mut parent = place;
		while let Some(step) = self.places.get(parent).step {
			steps.push(step);
			parent = self
				.places
				.parent(parent)
				.expect("a projection has a parent");
		}
		actual
			.iter()
			.map(|&root| {
				steps
					.iter()
					.rev()
					.fold(root, |parent, &step| self.places.child(parent, step))
			})
			.collect()
	}

	fn read_places(&self, node: NodeId) -> Vec<PlaceId> {
		let node = self.strip_groups(node);
		self.move_places.get(&node).cloned().unwrap_or_default()
	}

	pub(super) fn observe_places(
		&mut self,
		node: NodeId,
		places: impl IntoIterator<Item = PlaceId>,
		reads: &mut FxHashMap<NodeId, Vec<SymbolId>>,
	) {
		for place in places {
			if let Some(symbol) = self.place_symbol(place) {
				let symbols = reads.entry(node).or_default();
				if !symbols.contains(&symbol) {
					symbols.push(symbol);
				}
				let paths = self.move_places.entry(node).or_default();
				if !paths.contains(&place) {
					paths.push(place);
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::analysis::NodeFlags;
	use crate::ids::NodeId;
	use crate::syntax::{self, View};

	fn transfers(source: &str) -> Vec<bool> {
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		(0..tree.nodes.len())
			.filter_map(|index| {
				let node = NodeId::new(index);
				(matches!(syntax::view(tree, node), View::Name(_))
					&& matches!(
						analysis.resolution(node),
						crate::symbols::Resolution::Local(_)
					) && tree.text(index) == "items")
					.then(|| analysis.has_flags(node, NodeFlags::MOVE))
			})
			.collect()
	}

	#[test]
	fn a_later_method_call_keeps_its_receiver_owner_live() {
		for body in ["original.push(2)", "console.log(original.sum())"] {
			let statements = format!(
				"mutable original: Array<Integer> = [1]\nfunction visit = () => {body}\nlet moved = original\nvisit()\nconsole.log(moved)\n"
			);
			for source in [
				statements.clone(),
				format!("function run = () => {{ {statements} }}\nrun()\n"),
			] {
				assert_eq!(
					crate::check::test_support::codes(&source),
					["MEM004"],
					"{source}"
				);
			}
		}
	}

	#[test]
	fn exported_callable_values_keep_their_module_sources_live() {
		for exported in [
			"read",
			"reader",
			"{ run = reader }",
			"[reader]",
			"(reader, 0)",
		] {
			let source = format!(
				"mutable original = [1]\nfunction read = (): Integer => original.sum()\nlet reader = read\nmutable target = original\ntarget.push(2)\nexport = {exported}\n"
			);
			assert_eq!(
				crate::check::test_support::codes(&source),
				["MEM004"],
				"{source}"
			);
		}
	}

	#[test]
	fn public_namespace_callables_keep_their_module_sources_live() {
		for member in [
			"function read = (): Integer => original.sum()",
			"let handle = reader",
			"namespace inner = { function read = (): Integer => original.sum() }",
		] {
			let source = format!(
				"mutable original = [1]\nfunction reader = (): Integer => original.sum()\nmutable target = original\ntarget.push(2)\nexport namespace = {{ {member} }}\n"
			);
			assert_eq!(
				crate::check::test_support::codes(&source),
				["MEM004"],
				"{source}"
			);
		}
	}

	#[test]
	fn an_exported_reader_observes_the_replacement_module_owner() {
		for exported in [
			"export function = (): Integer => original.sum()",
			"function read = (): Integer => original.sum()\nexport = read",
		] {
			let source = format!(
				"mutable original: Array<Integer> = [1]\nmutable target = original\noriginal = [2]\ntarget.push(3)\n{exported}\n"
			);
			crate::check::test_support::analyzed(&source);
		}
	}

	#[test]
	fn awaiting_a_replacement_task_keeps_the_moved_task_borrow_live() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet first = [1]\nlet second = [2]\nmutable task = start read(first)\nlet old = task\ntask = start read(second)\nconsole.log(await task)\nmutable moved = first\nmoved.push(3)\nconsole.log(await old, moved.length)\n";
		assert_eq!(crate::check::test_support::codes(source), ["MEM004"]);
	}

	#[test]
	fn awaiting_the_current_mutable_task_ends_its_current_borrow() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet first = [1]\nlet second = [2]\nmutable task = start read(first)\nconsole.log(await task)\ntask = start read(second)\nconsole.log(await task)\nmutable moved = second\nmoved.push(3)\nconsole.log(moved.length)\n";
		crate::check::test_support::analyzed(source);
	}

	#[test]
	fn task_field_transfers_preserve_their_borrow_endpoints() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet items = [1]\nlet held = { task = start read(items), other = 2 }\nlet task = held.task\nconsole.log(held.other, await task)\nmutable moved = items\nmoved.push(3)\nconsole.log(moved.length)\n";
		crate::check::test_support::analyzed(source);
	}

	#[test]
	fn a_task_in_a_returned_composite_cannot_borrow_a_local_owner() {
		for returned in [
			"{ task = start read(items) }",
			"return { task = start read(items) }",
		] {
			let source = format!(
				"async function read = (items: Array<Integer>): Integer => {{ await Task.yield()\nitems.length }}\nfunction make = (): {{ task: Task<Integer> }} => {{ let items = [1]\n{returned} }}\n"
			);
			assert_eq!(crate::check::test_support::codes(&source), ["CONC018"]);
		}
	}

	#[test]
	fn a_task_leaving_a_block_cannot_borrow_that_blocks_local_owner() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet held = do { let items = [1]\n{ task = start read(items) } }\nconsole.log(await held.task catch _error => 0)\n";
		assert_eq!(crate::check::test_support::codes(source), ["CONC018"]);
	}

	#[test]
	fn awaiting_tasks_stored_by_push_ends_their_borrows() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet items = [1]\nmutable tasks: Array<Task<Integer>> = []\ntasks.push(start read(items))\nconsole.log(await Task.all(tasks))\nmutable moved = items\nmoved.push(3)\nconsole.log(moved.length)\n";
		crate::check::test_support::analyzed(source);
	}

	#[test]
	fn recursive_task_owner_provenance_converges_at_a_loop_head() {
		let source = "type Pending = { task: Task<Integer>, next: Pending? }\nasync function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nfunction collect = (items: Array<Integer>, count: Integer<min = 0>): Pending? => { mutable list: Pending? = none\nfor index in 0..count { list = Pending { task = start read(items), next = list } }\nlist }\n";
		crate::check::test_support::analyzed(source);
	}

	#[test]
	fn an_exported_reader_keeps_its_module_owner_live_after_initialization() {
		let source = "let original = [1, 2]\nmutable target = original\ntarget.push(3)\nexport function = () => console.log(original.length, target.length)\n";
		assert_eq!(crate::check::test_support::codes(source), ["MEM004"]);
	}

	#[test]
	fn exported_readers_include_transitive_calls_and_omitted_defaults() {
		for reader in [
			"function read = (): Integer => original.length\nexport function = () => console.log(read(), target.length)",
			"function read = (value: Integer = original.length): Integer => value\nexport function = () => console.log(read(), target.length)",
		] {
			let source = format!(
				"let original = [1, 2]\nmutable target = original\ntarget.push(3)\n{reader}\n"
			);
			assert_eq!(crate::check::test_support::codes(&source), ["MEM004"]);
		}
	}

	#[test]
	fn a_pending_task_keeps_its_borrowed_owner_live() {
		for source in [
			"let items: Array<Integer> = [1, 2]\nasync function read = (): Integer => { await Task.yield()\nitems.length }\nlet pending = start read()\nmutable target = items\ntarget.push(3)\nconsole.log(await pending, target.length)\n",
			"async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet items = [1, 2]\nlet pending = start read(items)\nmutable target = items\ntarget.push(3)\nconsole.log(await pending, target.length)\n",
		] {
			let codes = crate::check::test_support::codes(source);
			assert_eq!(codes, ["MEM004"], "{source}");
		}
	}

	#[test]
	fn pending_task_diagnostic_offers_a_checked_independent_copy() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nlet items = [1, 2]\nlet pending = start read(items)\nmutable target = items\ntarget.push(3)\nconsole.log(await pending, target.length)\n";
		let (session, file) = crate::analyze_source(source);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
		let diagnostic = &diagnostics[0];
		assert_eq!(diagnostic.code.id, "MEM004");
		assert!(
			diagnostic
				.message
				.contains("pending task started on line 4")
		);
		assert!(!diagnostic.related.is_empty());
		let mut fixed = source.to_string();
		let mut edits = diagnostic.fixes[0].edits.clone();
		edits.sort_by_key(|edit| std::cmp::Reverse(edit.location.span.start));
		for edit in edits {
			fixed.replace_range(
				edit.location.span.start as usize..edit.location.span.end as usize,
				&edit.replacement,
			);
		}
		crate::check::test_support::analyzed(&fixed);
	}

	#[test]
	fn a_pending_task_prevents_an_owned_capture_from_transferring_its_source() {
		let source = "type Reader = () => Integer\nasync function count = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nasync function make = (): Reader => {\nlet items = [1, 2]\nlet pending = start count(items)\nfunction saved = (): Integer => items.length\nconsole.log(await pending)\nsaved\n}\n";
		assert_eq!(crate::check::test_support::codes(source), ["MEM004"]);
	}

	#[test]
	fn a_pending_task_borrow_of_a_resource_forbids_its_move() {
		let source = "type Held = { value: Integer, function destroy = (self) => {} }\nasync function read = (held: Held): Integer => { await Task.yield()\nheld.value }\nlet held = Held { value = 1 }\nlet pending = start read(held)\nlet moved = held\nconsole.log(await pending, moved.value)\n";
		let mut session = crate::Session::new();
		let file = session.set_source("pending.foreign", std::sync::Arc::from(source));
		session.analyze_file(file);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|d| d.severity == crate::Severity::Error)
			.collect();
		assert_eq!(errors.len(), 1, "{errors:#?}");
		assert_eq!(errors[0].code.id, "MEM002");
		assert!(errors[0].message.contains("pending task"));
	}

	#[test]
	fn awaiting_a_moved_task_ends_its_owner_borrow() {
		crate::check::test_support::analyzed(
			"async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nasync function run = (): Integer => {\nlet items = [1, 2]\nlet pending = start read(items)\nlet next = pending\nlet before = await next\nmutable target = items\ntarget.push(3)\nbefore + target.length\n}\n",
		);
	}

	#[test]
	fn an_ordinary_pending_borrow_prevents_last_use_optimizations() {
		let source = "async function read = (text: String): String => { await Task.yield()\ntext }\nasync function run = (count: Integer<min = 0>): Integer => {\nlet text = \"ab\".repeat(count)\nlet pending = start read(text)\nmutable other = text\nother += \"X\"\nlet received = await pending\nreceived.length + other.length\n}\n";
		let (session, file) = crate::check::test_support::analyzed(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let node = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| {
				tree.text(node.index()) == "text"
					&& analysis.has_flags(node, NodeFlags::BORROWED_OWNER)
			})
			.expect("the retained source read");
		assert!(!analysis.has_flags(node, NodeFlags::MOVE));
		assert_eq!(analysis.flags_of(node), analysis.clone().flags_of(node));
	}

	#[test]
	fn a_task_moved_out_of_a_block_keeps_its_source_borrow_live() {
		let source = "async function read = (items: Array<Integer>): Integer => { await Task.yield()\nitems.length }\nasync function run = (): Integer => {\nlet items = [1, 2]\nlet pending = do { let inner = start read(items)\nlet next = inner\nnext }\nmutable target = items\ntarget.push(3)\nlet before = await pending\nbefore + target.length\n}\n";
		assert_eq!(crate::check::test_support::codes(source), ["MEM004"]);
	}

	#[test]
	fn both_exclusive_branch_results_transfer_the_owner() {
		assert_eq!(
			transfers(
				"function choose = (flag: Boolean): Array<Integer> => { let items = [1]\nif flag { items } else { items } }\n"
			),
			[true, true]
		);
	}

	#[test]
	fn a_loop_back_edge_keeps_an_outer_owner_alive() {
		let flags = transfers(
			"function run = (count: Integer) => { let items = [1]\nfor index in 0..count { let held = items\nconsole.log(held) } }\n",
		);
		assert_eq!(flags, [false]);
	}

	#[test]
	fn a_return_can_move_before_a_textually_later_read() {
		assert_eq!(
			transfers(
				"function choose = (flag: Boolean): Array<Integer> => { let items = [1]\nif flag { return items }\nconsole.log(items)\nitems }\n"
			),
			[true, false, true]
		);
	}

	#[test]
	fn a_reference_and_a_later_local_call_keep_the_owner_alive() {
		assert_eq!(
			transfers(
				"function run = () => { let items = [1]\nuse alias = items\nlet held = items\nconsole.log(alias, held) }\n"
			),
			[false, false]
		);
		assert_eq!(
			transfers(
				"function run = () => { let items = [1]\nfunction read = () => items.length\nlet held = items\nconsole.log(read(), held) }\n"
			),
			[false, false]
		);
	}

	#[test]
	fn earlier_arguments_borrow_until_the_call_finishes() {
		assert_eq!(
			transfers(
				"function both = (left: Array<Integer>, right: { items: Array<Integer> }) => console.log(left, right.items)\nfunction run = () => { let items = [1]\nboth(items, { items = items }) }\n"
			),
			[false, false]
		);
	}

	#[test]
	fn an_alias_argument_borrows_the_owner_through_later_arguments() {
		assert_eq!(
			crate::check::test_support::codes(
				"function both = (left: Array<Integer>, right: { items: Array<Integer> }) => console.log(left, right.items)\nfunction run = () => { let items = [1]\nuse alias = items\nboth(alias, { items = items }) }\n"
			),
			["MEM004"]
		);
	}

	#[test]
	fn storing_a_literal_item_transfers_at_its_last_use() {
		assert_eq!(
			transfers("function run = (): Array<Array<Integer>> => { let items = [1]\n[items] }\n"),
			[true]
		);
		assert_eq!(
			transfers(
				"function run = (): { nested: Array<Integer> } => { let items = [1]\n{ nested = items } }\n"
			),
			[true]
		);
	}

	#[test]
	fn later_failure_edges_keep_the_original_owner_live() {
		assert_eq!(
			transfers(
				"function run = (text: String): Integer => { let items = [1]\nlet held = do { (items, Integer.parse(text)) } catch _error => { console.log(items)\n([], 0) }\nheld[1] }\n"
			),
			[false, false]
		);
	}

	#[test]
	fn a_closure_cannot_consume_a_captured_task() {
		let source = "async function work = (): Integer => 1\nasync function outer = (): Integer => { let task = start work()\nasync function finish = (): Integer => await task\nawait finish() }\n";
		let (session, file) = crate::analyze_source(source);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
		assert_eq!(diagnostics[0].code.id, "MEM002");
		assert!(diagnostics[0].message.contains("`task`"));
		assert!(diagnostics[0].message.contains("finish(task)"));
		assert!(diagnostics[0].message.contains("captures"));
		crate::check::test_support::analyzed(
			"async function work = (): Integer => 1\nasync function finish = (task: Task<Integer>): Integer => await task catch _error => -1\nasync function outer = (): Integer => { let task = start work()\nawait finish(task) }\n",
		);
	}

	#[test]
	fn a_reference_cannot_consume_its_task_owner() {
		let source = "async function work = (): Integer => 1\nlet task = start work()\nuse alias = task\nconsole.log(await alias catch _error => -1)\n";
		let codes = crate::check::test_support::codes(source);
		assert_eq!(codes, ["MEM002"]);
	}

	#[test]
	fn a_replacement_ends_the_previous_value_before_the_back_edge() {
		assert_eq!(
			transfers(
				"function run = (count: Integer) => { mutable items = [1]\nfor index in 0..count { let held = items\nitems = [index]\nconsole.log(held) } }\n"
			),
			[true, false]
		);
	}
}
