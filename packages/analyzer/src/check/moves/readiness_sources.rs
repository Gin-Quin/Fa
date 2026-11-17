//! Portable per-entry source observations and guarded call dependencies.
//! This is a graph, not flattened combinations of nested declaration tests.

use rustc_hash::{FxHashMap, FxHashSet};

use super::{Context, EARLY, Entry, Propagation, READY};
use crate::analysis::{
	OwnershipDeclarationGuard, OwnershipEntry, OwnershipEntryMode, OwnershipEntrySources,
	OwnershipInvocationEdge, OwnershipSourceGraph, OwnershipSources,
};
use crate::check::Walker;
use crate::flow::ownership::{Sources, boundary_sources};
use crate::ids::NodeId;

impl Entry {
	fn published(self) -> OwnershipEntry {
		OwnershipEntry {
			context: self.context,
			mode: if self.owned {
				OwnershipEntryMode::Owned
			} else {
				OwnershipEntryMode::Borrowed
			},
		}
	}
}

impl Walker<'_> {
	pub(super) fn publish_ownership_source_graph(
		&mut self,
		propagation: &Propagation,
		bodies: &FxHashMap<Context, NodeId>,
		defaults: &FxHashSet<NodeId>,
	) {
		let mut graph = OwnershipSourceGraph::default();
		for &entry in propagation.inherited.keys() {
			let owner = propagation
				.owners
				.get(&entry.context)
				.copied()
				.unwrap_or(entry.context.node());
			let pending = propagation.unresolved.contains(&entry.context)
				|| !bodies.contains_key(&entry.context)
				|| self.a.ownership_sources.pending_functions.contains(&owner);
			graph.entries.insert(
				entry.published(),
				OwnershipEntrySources {
					pending,
					..Default::default()
				},
			);
		}
		for node in self
			.a
			.ownership_sources
			.reads
			.keys()
			.copied()
			.collect::<Vec<_>>()
		{
			let Some(sources) = self.ownership_read_origins.get(&node).cloned() else {
				continue;
			};
			let context = self.invocation_context(node, defaults);
			self.publish_entry_read(&mut graph, propagation, context, node, sources);
		}
		let creations: Vec<_> = self
			.a
			.escapes
			.iter()
			.filter_map(|&function| self.a.function(function))
			.map(|info| {
				(
					info.lambda,
					info.declaration.unwrap_or(info.lambda),
					info.captures.clone(),
				)
			})
			.collect();
		for (function, creation, captures) in creations {
			let context = self.invocation_context(creation, defaults);
			let mut sources = Sources::new();
			for symbol in captures {
				if let Some(inputs) = self.ownership_capture_origins.get(&(function, symbol)) {
					sources.extend(inputs.iter().copied());
				} else {
					for owned in [false, true] {
						if let Some(entry) =
							graph.entries.get_mut(&Entry { context, owned }.published())
						{
							entry.pending = true;
						}
					}
				}
			}
			self.publish_entry_read(&mut graph, propagation, context, creation, sources);
		}
		self.publish_source_edges(&mut graph, propagation);
		self.a.ownership_source_graph = graph;
	}

	fn publish_entry_read(
		&mut self,
		graph: &mut OwnershipSourceGraph,
		propagation: &Propagation,
		context: Context,
		node: NodeId,
		sources: Sources,
	) {
		let function = propagation
			.owners
			.get(&context)
			.copied()
			.unwrap_or(context.node());
		// Tests, watches and started blocks can retain reads before their
		// invocation entry has been modeled. Preserve them as pending inputs.
		graph
			.entries
			.entry(
				Entry {
					context,
					owned: false,
				}
				.published(),
			)
			.or_insert_with(|| OwnershipEntrySources {
				pending: true,
				..Default::default()
			});
		for owned in [false, true] {
			let entry = Entry { context, owned }.published();
			if !graph.entries.contains_key(&entry) {
				continue;
			}
			let sources = boundary_sources(
				&mut self.places,
				sources.clone(),
				function,
				owned,
				&self.ownership_capture_origins,
			);
			let sources = self.stable_ownership_sources(&sources);
			let stored = graph
				.entries
				.get_mut(&entry)
				.expect("published source entry")
				.reads
				.entry(node)
				.or_default();
			append_sources(stored, sources);
		}
	}

	fn publish_source_edges(&self, graph: &mut OwnershipSourceGraph, propagation: &Propagation) {
		for &entry in propagation.inherited.keys() {
			let owner = propagation
				.owners
				.get(&entry.context)
				.copied()
				.unwrap_or(entry.context.node());
			let stored = graph
				.entries
				.get_mut(&entry.published())
				.expect("published invocation entry");
			for edge in propagation
				.outgoing
				.get(&entry.context)
				.into_iter()
				.flatten()
			{
				let modes = self.readiness_invocation_modes(propagation, entry, edge);
				if modes == 0 {
					stored.pending = true;
				}
				let declaration =
					(edge.direct && self.a.escapes.contains(&edge.target) && edge.target != owner)
						.then(|| {
							propagation
								.declarations
								.iter()
								.find(|data| data.lambda == edge.target)
								.map(|data| data.node)
						})
						.flatten();
				for owned in [false, true] {
					if modes & if owned { READY } else { EARLY } == 0 {
						continue;
					}
					let guard = declaration.map(|declaration| OwnershipDeclarationGuard {
						declaration,
						reached: owned,
					});
					for context in std::iter::once(Context::Body(edge.target))
						.chain(edge.defaults.iter().copied().map(Context::Default))
					{
						let edge = OwnershipInvocationEdge {
							site: edge.site,
							target: Entry { context, owned }.published(),
							guard,
						};
						if !stored.calls.contains(&edge) {
							stored.calls.push(edge);
						}
					}
				}
			}
		}
	}
}

fn append_sources(target: &mut OwnershipSources, sources: OwnershipSources) {
	for source in sources {
		if !target.contains(&source) {
			target.push(source);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::analysis::{Analysis, FnInfo, OwnershipSourceOrigin};
	use crate::ids::FileId;
	use crate::session::Session;

	fn function<'a>(session: &Session, analysis: &'a Analysis, name: &str) -> &'a FnInfo {
		analysis
			.functions
			.values()
			.find(|info| {
				info.symbol.is_some_and(|symbol| {
					session
						.types()
						.syms
						.resolve(analysis.symbols.symbol(symbol).name)
						== name
				})
			})
			.unwrap()
	}

	fn analyze(source: &str) -> (Session, FileId) {
		let (session, file) = crate::analyze_source(source);
		assert_eq!(
			session.analysis(file).ownership_source_graph,
			session.analysis(file).clone().ownership_source_graph
		);
		(session, file)
	}

	fn entry(context: NodeId, owned: bool) -> OwnershipEntry {
		Entry {
			context: Context::Body(context),
			owned,
		}
		.published()
	}

	#[test]
	fn owned_capture_reads_stop_but_direct_reads_of_the_same_owner_remain() {
		let source = "type Reader = () => Integer\nmutable global = { value = 1 }\nfunction make = (): Reader => {\n use snapshot = global.value\n function read = (): Integer => {\n mutable use selected = snapshot\n if global.value > 0 { reuse selected = global.value }\n let first = selected\n reuse selected = snapshot\n let second = selected\n first + second\n }\n read\n}\n";
		let (session, file) = analyze(source);
		let analysis = session.analysis(file);
		let read = function(&session, analysis, "read");
		let graph = &analysis.ownership_source_graph;
		let owned = &graph.entries[&entry(read.lambda, true)];
		let borrowed = &graph.entries[&entry(read.lambda, false)];
		let tree = session.tree(file);
		let mut nodes: Vec<_> = analysis
			.ownership_sources
			.reads
			.keys()
			.filter(|node| tree.text(node.index()) == "selected")
			.copied()
			.collect();
		nodes.sort_unstable();
		assert_eq!(nodes.len(), 2);
		assert_eq!(owned.reads[&nodes[0]].len(), 1);
		assert_eq!(
			owned.reads[&nodes[0]][0].origin,
			OwnershipSourceOrigin::Direct
		);
		assert!(owned.reads[&nodes[1]].is_empty());
		assert!(!borrowed.reads[&nodes[1]].is_empty());
	}

	#[test]
	fn nested_creation_crosses_only_its_current_environment_boundary() {
		let source = "type Reader = () => Integer\ntype Maker = () => Reader\nfunction make = (n: Integer): Maker => {\n function outer = (): Reader => {\n function inner = (): Integer => n\n inner\n }\n outer\n}\n";
		let (session, file) = analyze(source);
		let analysis = session.analysis(file);
		let outer = function(&session, analysis, "outer");
		let inner = function(&session, analysis, "inner");
		let graph = &analysis.ownership_source_graph;
		let creation = inner.declaration.unwrap();
		assert!(graph.entries[&entry(outer.lambda, true)].pending);
		assert!(
			graph.entries.contains_key(&entry(outer.lambda, true)),
			"outer {:?}; inner {:?}; escapes {:?}; states {:?}",
			outer.lambda,
			inner.lambda,
			analysis.escapes,
			analysis.function_capture_states
		);
		assert!(
			graph.entries[&entry(outer.lambda, true)]
				.reads
				.contains_key(&creation),
			"creation {creation:?}, reads {:?}",
			graph.entries[&entry(outer.lambda, true)].reads
		);
		assert!(graph.entries[&entry(outer.lambda, true)].reads[&creation].is_empty());
		assert!(!graph.entries[&entry(outer.lambda, false)].reads[&creation].is_empty());
		let borrowed = &graph.entries[&entry(inner.lambda, false)];
		assert!(borrowed.reads.values().flatten().any(|source| matches!(source.origin, OwnershipSourceOrigin::Capture { function, .. } if function == outer.lambda)));
		assert!(
			graph.entries[&entry(inner.lambda, true)]
				.reads
				.values()
				.all(|sources| sources.is_empty())
		);
	}

	#[test]
	fn mixed_wrapper_calls_keep_opposite_guards_on_separate_edges() {
		let source = "type Reader = () => Integer\nfunction make = (n: Integer): (Integer, Reader) => {\n function invoke = (): Integer => read()\n let before = invoke()\n function read = (): Integer => n\n (before + invoke(), read)\n}\n";
		let (session, file) = analyze(source);
		let analysis = session.analysis(file);
		let invoke = function(&session, analysis, "invoke");
		let read = function(&session, analysis, "read");
		let calls: Vec<_> = analysis.ownership_source_graph.entries[&entry(invoke.lambda, false)]
			.calls
			.iter()
			.filter(|edge| edge.target.context == Context::Body(read.lambda))
			.collect();
		assert_eq!(calls.len(), 2);
		for edge in calls {
			assert_eq!(
				edge.guard,
				Some(OwnershipDeclarationGuard {
					declaration: read.declaration.unwrap(),
					reached: edge.target.mode == OwnershipEntryMode::Owned
				})
			);
		}
	}

	#[test]
	fn written_arguments_do_not_add_a_default_dependency() {
		let source = "function make = (n: Integer): Integer => {\n function read = (): Integer => n\n function invoke = (value: Integer = read()): Integer => value\n invoke() + invoke(0)\n}\n";
		let (session, file) = analyze(source);
		let analysis = session.analysis(file);
		let make = function(&session, analysis, "make");
		let invoke = function(&session, analysis, "invoke");
		let default = invoke.params[0].default.unwrap();
		let calls = &analysis.ownership_source_graph.entries[&entry(make.lambda, false)].calls;
		assert_eq!(
			calls
				.iter()
				.filter(|edge| edge.target.context == Context::Body(invoke.lambda))
				.count(),
			2
		);
		assert_eq!(
			calls
				.iter()
				.filter(|edge| edge.target.context == Context::Default(default))
				.count(),
			1
		);
	}

	#[test]
	fn an_unknown_callable_observation_stays_pending() {
		let (session, file) =
			analyze("function invoke = (callback: () => Integer): Integer => callback()\n");
		let analysis = session.analysis(file);
		let invoke = function(&session, analysis, "invoke");
		assert!(analysis.ownership_source_graph.entries[&entry(invoke.lambda, false)].pending);
	}
	#[test]
	fn anonymous_escaping_entries_do_not_need_a_named_declaration() {
		let (session, file) = analyze(
			"type Reader = () => Integer\nfunction make = (n: Integer): Reader => () => n\n",
		);
		let analysis = session.analysis(file);
		let anonymous = analysis
			.functions
			.values()
			.find(|info| info.declaration.is_none() && !info.captures.is_empty())
			.unwrap();
		let graph = &analysis.ownership_source_graph;
		assert!(
			graph.entries[&entry(anonymous.lambda, true)]
				.reads
				.values()
				.all(|sources| sources.is_empty())
		);
		assert!(
			graph.entries[&entry(anonymous.lambda, false)]
				.reads
				.values()
				.any(|sources| !sources.is_empty())
		);
	}
	#[test]
	fn a_lambda_default_has_distinct_body_and_default_contexts() {
		let source = "type Reader = () => Integer\nfunction make = (n: Integer): Reader => {\n function invoke = (callback: Reader = () => n): Reader => callback\n invoke()\n}\n";
		let (session, file) = analyze(source);
		let analysis = session.analysis(file);
		let invoke = function(&session, analysis, "invoke");
		let default = invoke.params[0].default.unwrap();
		let body = &analysis.ownership_source_graph.entries[&entry(default, false)];
		let default_key = Entry {
			context: Context::Default(default),
			owned: false,
		}
		.published();
		let construction = &analysis.ownership_source_graph.entries[&default_key];
		assert_ne!(entry(default, false), default_key);
		assert!(body.reads.values().any(|sources| !sources.is_empty()));
		assert!(construction.reads.contains_key(&default));
		let root = &analysis.ownership_source_graph.entries
			[&entry(function(&session, analysis, "make").lambda, false)];
		assert!(
			!root
				.calls
				.iter()
				.any(|edge| edge.target == entry(default, false))
		);
	}
	#[test]
	fn a_borrowed_default_lambda_calls_from_its_body_only() {
		let source = "type Reader = () => Integer\nfunction make = (n: Integer): Integer => {\n function read = (): Integer => n\n function invoke = (callback: Reader = () => read()): Integer => callback()\n invoke()\n}\n";
		let (session, file) = analyze(source);
		let analysis = session.analysis(file);
		let invoke = function(&session, analysis, "invoke");
		let read = function(&session, analysis, "read");
		let default = invoke.params[0].default.unwrap();
		let graph = &analysis.ownership_source_graph;
		let body = &graph.entries[&entry(default, false)];
		let construction = &graph.entries[&Entry {
			context: Context::Default(default),
			owned: false,
		}
		.published()];
		assert!(
			body.calls
				.iter()
				.any(|edge| edge.target.context == Context::Body(read.lambda))
		);
		assert!(construction.calls.is_empty());
	}
}
