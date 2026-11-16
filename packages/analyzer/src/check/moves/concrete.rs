//! Concrete ownership decisions over runtime control flow. Generic-dependent
//! stores are left to instantiation conditions, not guessed from constraints.

use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::Graph;
use crate::analysis::{Argument, Callee, FieldValue, NodeFlags};
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

impl Walker<'_> {
	pub(super) fn finish_concrete_moves(&mut self) {
		self.collect_store_values();
		let reads = self.ownership_reads();
		let mut commits: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
		for value in self.copy_sites.iter().copied() {
			commits.entry(value).or_default().push(value);
		}
		let root = NodeId::new(self.tree.root);
		let mut bodies = vec![root];
		bodies.extend(self.a.functions.values().map(|info| info.body));
		bodies.sort_unstable();
		bodies.dedup();
		for body in bodies {
			let graph = Graph::build(self, body, &reads, &commits);
			let mut sources: FxHashMap<SymbolId, Vec<NodeId>> = FxHashMap::default();
			for &value in graph.stores.keys() {
				if matches!(self.view(value), View::Name(_))
					&& let Resolution::Local(symbol) = self.a.resolution(value)
					&& self.s.types.is_structure(self.a.type_of(value))
					&& self.ownership_source_owned(symbol, value)
				{
					sources.entry(symbol).or_default().push(value);
				}
			}
			for (symbol, values) in sources {
				let later = graph.later_uses(symbol);
				for value in values {
					let positions = &graph.stores[&value];
					if positions.iter().any(|&index| graph.reachable[index])
						&& positions
							.iter()
							.all(|&index| !graph.reachable[index] || later[index].is_none())
					{
						self.set_flag(value, NodeFlags::MOVE);
					}
				}
			}
		}
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
		values.extend(
			self.a
				.functions
				.values()
				.flat_map(|info| info.params.iter().filter_map(|param| param.default)),
		);
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
			self.record_copy_sources(value);
		}
	}

	/// The runtime body owning a declaration or read. A nested function
	/// borrows its nonlocal values, even if they are otherwise owned locals.
	pub(crate) fn ownership_body(&self, mut node: NodeId) -> NodeId {
		while let Some(parent) = self.parent(node) {
			if matches!(self.view(parent), View::Lambda { .. } | View::Test { .. }) {
				return parent;
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
	fn ownership_reads(&self) -> FxHashMap<NodeId, Vec<SymbolId>> {
		let mut reads = self.move_reads.clone();
		// A borrowed argument stays live through the call, not just until
		// its expression was evaluated (later arguments may store an owner).
		for (&call, info) in &self.a.calls {
			for argument in &info.arguments {
				let node = match *argument {
					Argument::Given(node) => node,
					_ => continue,
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
				if let Some(symbol) = crate::lower::copies::place_root(&self.tree, &self.a, node) {
					let output = reads.entry(call).or_default();
					if !output.contains(&symbol) {
						output.push(symbol);
					}
				}
			}
		}
		let mut free: FxHashMap<NodeId, FxHashSet<SymbolId>> = FxHashMap::default();
		for (&node, symbols) in &reads {
			let body = self.ownership_body(node);
			for &symbol in symbols {
				if self.ownership_body(self.a.symbols.symbol(symbol).decl_node) != body {
					free.entry(body).or_default().insert(symbol);
				}
			}
		}
		loop {
			let mut changed = false;
			for (&call, info) in &self.a.calls {
				let Callee::Function(function) = info.callee else {
					continue;
				};
				let Some(callee) = self.a.functions.get(&function) else {
					continue;
				};
				let symbols = free.get(&callee.lambda).cloned().unwrap_or_default();
				let body = self.ownership_body(call);
				let output = reads.entry(call).or_default();
				for symbol in symbols {
					if !output.contains(&symbol) {
						output.push(symbol);
					}
					if self.ownership_body(self.a.symbols.symbol(symbol).decl_node) != body {
						changed |= free.entry(body).or_default().insert(symbol);
					}
				}
			}
			if !changed {
				break;
			}
		}
		reads
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
