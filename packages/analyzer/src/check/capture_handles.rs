//! Named callable bindings needed by declaration environments. These are
//! handles, distinct from the data read by the named callee's body.

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use super::Walker;
use crate::analysis::FunctionCaptureState;
use crate::ids::{NodeId, SymbolId};
use crate::syntax::View;

impl Walker<'_> {
	pub(super) fn publish_callable_capture_facts(&mut self) {
		let root = NodeId::new(self.tree.root);
		let mut functions: Vec<_> = self.a.functions.values().map(|info| info.lambda).collect();
		functions.sort_unstable();
		functions.dedup();
		let mut dependencies: FxHashMap<NodeId, FxHashSet<NodeId>> = functions
			.iter()
			.map(|&lambda| {
				(
					lambda,
					self.callable_handle_captures(lambda).into_iter().collect(),
				)
			})
			.collect();
		// A nested declaration must obtain a nonlocal handle through each
		// intervening environment, stopping at the binding's owning frame or
		// at the running function itself (which provides its self handle).
		let edges: Vec<_> = dependencies
			.iter()
			.flat_map(|(&caller, callees)| callees.iter().map(move |&callee| (caller, callee)))
			.collect();
		for (caller, callee) in edges {
			let binding_owner = self.function_binding_owner(callee);
			let mut parent = self.function_binding_owner(caller);
			while parent != root && parent != binding_owner && parent != callee {
				dependencies.entry(parent).or_default().insert(callee);
				let next = self.function_binding_owner(parent);
				if next == parent {
					break;
				}
				parent = next;
			}
		}
		// A chain of captureless local calls needs no environment. Compute
		// capturefulness from data roots, not from the presence of call edges;
		// this also handles cycles without making empty cycles capturing.
		let mut reverse: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
		for (&caller, callees) in &dependencies {
			for &callee in callees {
				reverse.entry(callee).or_default().push(caller);
			}
		}
		let capturing = propagate(
			functions
				.iter()
				.copied()
				.filter(|&lambda| {
					self.a
						.function(lambda)
						.is_some_and(|info| !info.captures.is_empty())
				})
				.chain(
					self.body
						.functions
						.iter()
						.filter(|cx| !cx.captures.is_empty())
						.map(|cx| cx.node),
				),
			&reverse,
		);
		let pending = propagate(self.body.functions.iter().map(|cx| cx.node), &reverse);
		functions.extend(dependencies.keys().copied());
		functions.sort_unstable();
		functions.dedup();
		self.a.function_capture_states = functions
			.into_iter()
			.map(|lambda| {
				let state = if capturing.contains(&lambda) {
					FunctionCaptureState::Capturing
				} else if pending.contains(&lambda) {
					FunctionCaptureState::Pending
				} else {
					FunctionCaptureState::Empty
				};
				(lambda, state)
			})
			.collect();
		let handles = dependencies
			.into_iter()
			.filter_map(|(lambda, dependencies)| {
				let mut symbols: Vec<SymbolId> = dependencies
					.into_iter()
					.filter_map(|callee| self.a.function(callee).and_then(|info| info.symbol))
					.collect();
				symbols.sort_unstable();
				symbols.dedup();
				(!symbols.is_empty()).then_some((lambda, symbols))
			})
			.collect();
		self.a.named_function_dependencies = handles;
		self.publish_ownership_source_facts();
		self.publish_local_function_readiness();
	}

	fn function_binding_owner(&self, lambda: NodeId) -> NodeId {
		let node = self
			.a
			.function(lambda)
			.map_or(lambda, |info| info.declaration.unwrap_or(info.lambda));
		let node = if matches!(self.view(node), View::Do(_))
			&& let Some(parent) = self.parent(node)
			&& matches!(self.view(parent), View::Start(_))
		{
			parent
		} else {
			node
		};
		self.ownership_body(node)
	}
}

fn propagate(
	seeds: impl IntoIterator<Item = NodeId>,
	reverse: &FxHashMap<NodeId, Vec<NodeId>>,
) -> FxHashSet<NodeId> {
	let mut found: FxHashSet<_> = seeds.into_iter().collect();
	let mut pending: VecDeque<_> = found.iter().copied().collect();
	while let Some(callee) = pending.pop_front() {
		for &caller in reverse.get(&callee).into_iter().flatten() {
			if found.insert(caller) {
				pending.push_back(caller);
			}
		}
	}
	found
}

#[cfg(test)]
mod tests {
	use crate::analysis::FunctionCaptureState;
	use crate::ids::NodeId;

	fn captures(source: &str, name: &str) -> Vec<String> {
		let (session, file) = crate::analyze_source(source);
		assert!(
			session.diagnostics(file).is_empty(),
			"{:?}",
			session.diagnostics(file)
		);
		let analysis = session.analysis(file);
		assert!(
			analysis
				.function_capture_states
				.values()
				.all(|state| *state != FunctionCaptureState::Pending)
		);
		let lambda = analysis
			.functions
			.values()
			.find_map(|info| {
				let symbol = info.symbol?;
				(session
					.types()
					.syms
					.resolve(analysis.symbols.symbol(symbol).name)
					== name)
					.then_some(info.lambda)
			})
			.expect("the named function");
		let cloned = analysis.clone();
		assert_eq!(
			analysis.named_function_dependencies,
			cloned.named_function_dependencies
		);
		assert_eq!(
			analysis.function_capture_states,
			cloned.function_capture_states
		);
		handle_names(&session, file, lambda)
	}

	fn handle_names(
		session: &crate::Session,
		file: crate::ids::FileId,
		lambda: NodeId,
	) -> Vec<String> {
		let analysis = session.analysis(file);
		let mut names: Vec<_> = analysis
			.named_function_dependencies
			.get(&lambda)
			.into_iter()
			.flatten()
			.filter(|&&symbol| {
				let node = analysis.symbols.symbol(symbol).decl_node;
				analysis.function(node).is_some_and(|info| {
					analysis.function_capture_states.get(&info.lambda)
						== Some(&FunctionCaptureState::Capturing)
				})
			})
			.map(|&symbol| {
				session
					.types()
					.syms
					.resolve(analysis.symbols.symbol(symbol).name)
					.to_string()
			})
			.collect();
		names.sort();
		names
	}

	#[test]
	fn a_named_callee_is_a_handle_capture() {
		let source = "type Reader = () => Integer\nfunction make = (seed: Integer): Reader => {\nfunction read = (): Integer => seed\nfunction invoke = (): Integer => read()\ninvoke\n}\nconsole.log(make(7)())\n";
		assert_eq!(captures(source, "invoke"), ["read"]);
		assert!(captures(source, "make").is_empty());
		assert!(captures(source, "read").is_empty());
	}

	#[test]
	fn a_callee_used_only_by_a_default_is_a_handle_capture() {
		let source = "type Reader = (Integer) => Integer\nfunction make = (seed: Integer): Reader => {\nfunction read = (): Integer => seed\nfunction invoke = (value: Integer = read()): Integer => value\ninvoke\n}\nconsole.log(make(7)(2))\n";
		assert_eq!(captures(source, "invoke"), ["read"]);
	}

	#[test]
	fn a_nested_handle_is_transported_through_its_enclosing_environment() {
		let source = "type Reader = () => Integer\ntype Maker = () => Reader\nfunction make = (seed: Integer): Maker => {\nfunction read = (): Integer => seed\nfunction layer = (): Reader => {\nfunction nested = (): Integer => read()\nnested\n}\nlayer\n}\nconsole.log(make(7)()())\n";
		assert_eq!(captures(source, "nested"), ["read"]);
		assert_eq!(captures(source, "layer"), ["read"]);
		assert!(captures(source, "make").is_empty());
	}

	#[test]
	fn captureless_callees_need_no_environment() {
		let source = "type Reader = () => Integer\nfunction make = (): Reader => {\nfunction answer = (remaining: Integer): Integer => if remaining == 0 { 42 } else { answer(remaining - 1) }\nfunction invoke = (): Integer => answer(2)\ninvoke\n}\nconsole.log(make()())\n";
		assert!(captures(source, "invoke").is_empty());
		assert!(captures(source, "answer").is_empty());
	}

	#[test]
	fn key_loop_snapshots_keep_their_named_handle_bindings() {
		let source = "type Row = { count: Integer, label: String }\nstatic keys: Keys<Row> = { count, label }\nfunction show = (row: Row) => {\n@for key in keys {\nfunction field = () => row[key]\nfunction invoke = () => field()\nconsole.log(invoke())\n}\n}\nshow(Row { count = 7, label = \"seven\" })\n";
		assert_key_loop_handles(source, "invoke", "field", FunctionCaptureState::Capturing);
	}

	#[test]
	fn a_key_loop_keeps_unfinished_enclosing_dependencies() {
		let source = "type Row = { count: Integer, label: String }\nstatic keys: Keys<Row> = { count, label }\ntype Reader = () => Integer\nfunction make = (seed: Integer): Reader => {\nfunction outer = (): Integer => {\n@for _key in keys {\nfunction inner = (): Integer => outer()\ninner()\n}\nseed\n}\nouter\n}\nmake(1)\n";
		assert_key_loop_handles(source, "inner", "outer", FunctionCaptureState::Pending);
	}

	fn assert_key_loop_handles(
		source: &str,
		caller: &str,
		callee: &str,
		state: FunctionCaptureState,
	) {
		let (session, file) = crate::analyze_source(source);
		assert!(
			session.diagnostics(file).is_empty(),
			"{:?}",
			session.diagnostics(file)
		);
		let snapshots: Vec<_> = session
			.analysis(file)
			.key_loop_bodies
			.values()
			.flatten()
			.flat_map(|instance| &instance.analyses)
			.collect();
		assert_eq!(snapshots.len(), 2);
		for snapshot in snapshots {
			let lambda = snapshot
				.functions
				.values()
				.find_map(|info| {
					let symbol = info.symbol?;
					(session
						.types()
						.syms
						.resolve(snapshot.symbols.symbol(symbol).name)
						== caller)
						.then_some(info.lambda)
				})
				.unwrap();
			let names: Vec<_> = snapshot
				.named_function_dependencies
				.get(&lambda)
				.into_iter()
				.flatten()
				.map(|&symbol| {
					session
						.types()
						.syms
						.resolve(snapshot.symbols.symbol(symbol).name)
				})
				.collect();
			assert_eq!(names, [callee]);
			assert_eq!(snapshot.function_capture_states.get(&lambda), Some(&state));
			let uses: Vec<_> = snapshot
				.named_function_readiness
				.iter()
				.filter(|(_, read)| {
					snapshot
						.symbols
						.symbol_of_decl(read.declaration)
						.is_some_and(|symbol| {
							session
								.types()
								.syms
								.resolve(snapshot.symbols.symbol(symbol).name)
								== callee
						})
				})
				.collect();
			assert!(
				!uses.is_empty(),
				"the key snapshot lost {callee}: {:?}; reads {:?}",
				snapshot.named_function_readiness,
				(0..session.tree(file).nodes.len())
					.filter_map(|index| {
						let node = NodeId::new(index);
						(session.tree(file).text(index) == callee).then_some((
							node,
							snapshot.type_of(node),
							snapshot.flags_of(node),
							snapshot.resolution(node),
						))
					})
					.collect::<Vec<_>>()
			);
			let expected_owned = if state == FunctionCaptureState::Pending {
				crate::analysis::DeclarationReadiness::Pending
			} else {
				crate::analysis::DeclarationReadiness::Ready
			};
			// Each key snapshot can be at a different completion stage.
			// Require the exact state its current expression/source facts prove.
			assert!(
				uses.iter().copied().all(|(&node, read)| {
					let incomplete = snapshot.type_of(node) == crate::ids::TypeId::UNSET
						|| snapshot
							.ownership_sources
							.pending_functions
							.contains(&lambda);
					let expected_borrowed = if incomplete {
						crate::analysis::DeclarationReadiness::Pending
					} else {
						expected_owned
					};
					read.borrowed == expected_borrowed && read.owned == expected_owned
				}),
				"readiness {uses:?}; pending {:?}",
				snapshot.ownership_sources.pending_functions
			);

			assert_eq!(
				snapshot.named_function_readiness,
				snapshot.clone().named_function_readiness
			);
		}
	}
}
