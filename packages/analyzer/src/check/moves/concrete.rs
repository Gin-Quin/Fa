//! Concrete ownership decisions over runtime control flow. Generic-dependent
//! stores are left to instantiation conditions, not guessed from constraints.

use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::Graph;
use crate::analysis::{Argument, Callee, FieldValue, NodeFlags};
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
		let mut commits: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
		for value in self.copy_sites.iter().copied() {
			commits.entry(value).or_default().push(value);
		}
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
		// An omitted argument borrows exactly like a written argument.
		// Only consuming parameters take ownership of their defaults.
		values.extend(self.a.functions.values().flat_map(|info| {
			info.params.iter().filter_map(|param| {
				self.s
					.types
					.parameter_can_be_consumed(self.a.symbols.symbol(param.symbol).ty)
					.then_some(param.default)
					.flatten()
			})
		}));
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
				if !consumes_receiver && matches!(self.view(callee), View::Access { .. }) {
					let places = self.move_places.get(&callee).cloned().unwrap_or_default();
					self.observe_places(*call, places, &mut reads);
				}
			}
		}
		let mut free: FxHashMap<NodeId, FxHashSet<PlaceId>> = FxHashMap::default();
		for (&node, symbols) in &reads {
			if self.access_roots.contains(&node) {
				continue;
			}
			let body = self.ownership_read_context(node, &defaults);
			for &symbol in symbols {
				if self.ownership_read_context(self.a.symbols.symbol(symbol).decl_node, &defaults)
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
					let mut places = free.get(&callee.lambda).cloned().unwrap_or_default();
					let mut bindings = FxHashMap::default();
					for (param, argument) in callee.params.iter().zip(&info.arguments) {
						let argument_places = match *argument {
							Argument::Given(value) | Argument::Receiver(value) => {
								self.ownership_result_places(value)
							}
							Argument::Default => {
								if let Some(default) = param.default {
									for &place in free.get(&default).into_iter().flatten() {
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
		reads
	}

	fn borrowed_callback_argument_reads(
		&self,
		arguments: &[Argument],
		free: &FxHashMap<NodeId, FxHashSet<PlaceId>>,
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
						places.extend(free.get(&info.lambda).into_iter().flatten().copied());
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
		free: &mut FxHashMap<NodeId, FxHashSet<PlaceId>>,
		reads: &mut FxHashMap<NodeId, Vec<SymbolId>>,
	) -> bool {
		let body = self.ownership_read_context(call, defaults);
		let mut changed = false;
		for &place in &places {
			let Some(symbol) = self.place_symbol(place) else {
				continue;
			};
			if self.ownership_read_context(self.a.symbols.symbol(symbol).decl_node, defaults)
				!= body
			{
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
		free: &FxHashMap<NodeId, FxHashSet<PlaceId>>,
		places: &mut FxHashSet<PlaceId>,
	) {
		let mut pending: Vec<_> = places.iter().copied().collect();
		while let Some(place) = pending.pop() {
			for function in self.callable_targets_at_call(call, place) {
				let Some(info) = self.a.function(function) else {
					continue;
				};
				for &source in free.get(&info.lambda).into_iter().flatten() {
					if places.insert(source) {
						pending.push(source);
					}
				}
			}
		}
	}

	/// Defaults run only at calls which omit them. Keep their observations
	/// separate from the unconditional observations of the function body.
	pub(super) fn ownership_read_context(
		&self,
		mut node: NodeId,
		defaults: &FxHashSet<NodeId>,
	) -> NodeId {
		loop {
			if defaults.contains(&node)
				|| matches!(
					self.view(node),
					View::Lambda { .. } | View::Test { .. } | View::Watch(_)
				) || (matches!(self.view(node), View::Do(_))
				&& self
					.parent(node)
					.is_some_and(|parent| matches!(self.view(parent), View::Start(_))))
			{
				return node;
			}
			let Some(parent) = self.parent(node) else {
				return node;
			};
			node = parent;
		}
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
