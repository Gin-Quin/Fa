//! Stable whole-owner provenance for analysis and key-specialization clones.
//! Temporary checked place IDs never leave the walk through this table.

use crate::analysis::{
	FunctionCaptureState, NodeFlags, OwnershipRead, OwnershipSource, OwnershipSourceFacts,
	OwnershipSourceOrigin, OwnershipSources,
};
use crate::check::Walker;
use crate::flow::{
	PlaceRoot,
	ownership::{Origin, Sources},
};
use crate::ids::NodeId;
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

impl Walker<'_> {
	pub(crate) fn publish_ownership_source_facts(&mut self) {
		let defaults = self
			.a
			.functions
			.values()
			.flat_map(|info| info.params.iter().filter_map(|param| param.default))
			.collect();
		let mut facts = OwnershipSourceFacts::default();
		for (&node, sources) in &self.ownership_read_origins {
			if self.access_roots.contains(&node) || self.a.has_flags(node, NodeFlags::TYPE_EXPR) {
				continue;
			}
			facts.reads.insert(
				node,
				OwnershipRead {
					context: self.ownership_read_context(node, &defaults),
					sources: self.stable_ownership_sources(sources),
				},
			);
		}
		for (&key, sources) in &self.ownership_capture_origins {
			facts
				.captures
				.insert(key, self.stable_ownership_sources(sources));
		}
		facts
			.pending_functions
			.extend(
				self.a
					.function_capture_states
					.iter()
					.filter_map(|(&function, state)| {
						(*state == FunctionCaptureState::Pending).then_some(function)
					}),
			);
		// A retained key-loop function body can have resolved reads without
		// current-pass observations. Preserve that incompleteness explicitly,
		// rather than treating absent reads as a proof that the source died.
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if !matches!(self.view(node), View::Name(_))
				|| self.a.has_flags(node, NodeFlags::TYPE_EXPR)
				|| self.source_observation_covers(node)
			{
				continue;
			}
			let Resolution::Local(symbol) = self.a.resolution(node) else {
				continue;
			};
			if matches!(
				self.a.symbols.symbol(symbol).kind,
				SymbolKind::Function
					| SymbolKind::Type
					| SymbolKind::Namespace
					| SymbolKind::TypeParameter
					| SymbolKind::Implicit
					| SymbolKind::Static
			) {
				continue;
			}
			if let Some(context) = self.runtime_source_context(node) {
				facts.pending_functions.insert(context);
			}
		}
		self.a.ownership_sources = facts;
	}

	/// Root prefixes are covered by the checked projection. A whole-name
	/// assignment is a definition, whereas writing through a field/index
	/// still observes its owner. Index/key expressions are separate reads.
	fn source_observation_covers(&self, mut node: NodeId) -> bool {
		let mut projected = false;
		loop {
			if self.ownership_read_origins.contains_key(&node) {
				return true;
			}
			let Some(parent) = self.parent(node) else {
				return false;
			};
			match self.view(parent) {
				View::Group(inner) if inner == node => {}
				View::Access { target, .. } | View::Index { target, .. } if target == node => {
					projected = true
				}
				View::Assignment { target, .. } if target == node && !projected => return true,
				_ => return false,
			}
			node = parent;
		}
	}

	/// Parameter names and signature effects are not runtime reads. Only
	/// a body or a default can be missing a source observation.
	fn runtime_source_context(&self, mut node: NodeId) -> Option<NodeId> {
		let context = self.ownership_body(node);
		let info = self.a.function(context)?;
		while node != context {
			if node == info.body || info.params.iter().any(|param| param.default == Some(node)) {
				return Some(context);
			}
			node = self.parent(node)?;
		}
		None
	}

	fn stable_ownership_sources(&self, sources: &Sources) -> OwnershipSources {
		let mut result = OwnershipSources::new();
		for source in sources {
			let origin = match source.origin {
				Origin::Direct => OwnershipSourceOrigin::Direct,
				Origin::Capture { function, input } => match self.places.get(input).root {
					PlaceRoot::Symbol(symbol) => {
						OwnershipSourceOrigin::Capture { function, symbol }
					}
					_ => OwnershipSourceOrigin::Direct,
				},
			};
			let source = OwnershipSource {
				root: self.places.get(source.place).root,
				origin,
			};
			if !result.contains(&source) {
				result.push(source);
			}
		}
		result
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::check::test_support::analyzed;

	#[test]
	fn reseating_replaces_origins_even_when_the_canonical_owner_is_the_same() {
		let source = "type Reader = () => Integer\nmutable global = { value = 1 }\nfunction make = (): Reader => {\n use snapshot = global.value\n function read = (): Integer => {\n mutable use selected = snapshot\n if global.value > 0 { reuse selected = global.value }\n let first = selected\n reuse selected = snapshot\n let second = selected\n first + second\n }\n read\n}\n";
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let mut reads: Vec<_> = analysis
			.ownership_sources
			.reads
			.iter()
			.filter(|(node, _)| tree.text(node.index()) == "selected")
			.collect();
		reads.sort_by_key(|(node, _)| **node);
		assert_eq!(reads.len(), 2);
		assert_eq!(reads[0].1.sources.len(), 2);
		assert_eq!(reads[1].1.sources.len(), 1);
		assert_eq!(reads[0].1.sources[0].root, reads[0].1.sources[1].root);
		assert!(
			reads[0]
				.1
				.sources
				.iter()
				.any(|source| source.origin == OwnershipSourceOrigin::Direct)
		);
		assert!(matches!(
			reads[1].1.sources[0].origin,
			OwnershipSourceOrigin::Capture { .. }
		));
		assert!(
			!analysis
				.ownership_sources
				.pending_functions
				.contains(&reads[1].1.context)
		);
		assert_eq!(
			analysis.ownership_sources,
			analysis.clone().ownership_sources
		);
	}

	#[test]
	fn capture_creation_keeps_the_enclosing_logical_input() {
		let source = "type Reader = () => Integer\ntype Maker = () => Reader\nfunction make = (n: Integer): Maker => {\n function outer = (): Reader => {\n function inner = (): Integer => n\n inner\n }\n outer\n}\n";
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		let inner = analysis
			.functions
			.values()
			.find(|info| {
				info.symbol.is_some_and(|symbol| {
					session
						.types()
						.syms
						.resolve(analysis.symbols.symbol(symbol).name)
						== "inner"
				})
			})
			.unwrap();
		let sources: Vec<_> = analysis
			.ownership_sources
			.captures
			.iter()
			.filter(|((function, _), _)| *function == inner.lambda)
			.flat_map(|(_, sources)| sources)
			.collect();
		assert_eq!(sources.len(), 1);
		assert!(
			matches!(sources[0].origin, OwnershipSourceOrigin::Capture { function, .. } if function != inner.lambda)
		);
	}

	#[test]
	fn key_snapshots_preserve_creation_inputs_and_mark_missing_reads_pending() {
		let source = "type Row = { count: Integer, label: String }\nstatic keys: Keys<Row> = { count, label }\nfunction show = (row: Row) => {\n@for key in keys {\nfunction field = () => row[key]\nfunction invoke = () => field()\nconsole.log(invoke())\n}\n}\nshow(Row { count = 7, label = \"seven\" })\n";
		let (session, file) = analyzed(source);
		let snapshots: Vec<_> = session
			.analysis(file)
			.key_loop_bodies
			.values()
			.flatten()
			.flat_map(|instance| &instance.analyses)
			.collect();
		assert_eq!(snapshots.len(), 2);
		for snapshot in snapshots {
			assert_eq!(
				snapshot.ownership_sources,
				snapshot.clone().ownership_sources
			);
			let field = snapshot
				.functions
				.values()
				.find(|info| {
					info.symbol.is_some_and(|symbol| {
						session
							.types()
							.syms
							.resolve(snapshot.symbols.symbol(symbol).name)
							== "field"
					})
				})
				.unwrap();
			assert!(
				snapshot
					.ownership_sources
					.captures
					.iter()
					.any(|((function, _), sources)| {
						*function == field.lambda && !sources.is_empty()
					})
			);
			let observed = snapshot.ownership_sources.reads.values().any(|read| {
				read.context == field.lambda && read.sources.iter().any(|source| {
					matches!(source.origin, OwnershipSourceOrigin::Capture { function, .. } if function == field.lambda)
				})
			});
			assert!(
				observed
					|| snapshot
						.ownership_sources
						.pending_functions
						.contains(&field.lambda),
				"a missing body observation cannot claim completeness"
			);
		}
	}
	#[test]
	fn key_snapshots_keep_unfinished_enclosing_sources_pending() {
		let source = "type Row = { count: Integer, label: String }\nstatic keys: Keys<Row> = { count, label }\ntype Reader = () => Integer\nfunction make = (seed: Integer): Reader => {\nfunction outer = (): Integer => {\n@for _key in keys {\nfunction inner = (): Integer => outer()\ninner()\n}\nseed\n}\nouter\n}\nmake(1)\n";
		let (session, file) = analyzed(source);
		let snapshots: Vec<_> = session
			.analysis(file)
			.key_loop_bodies
			.values()
			.flatten()
			.flat_map(|instance| &instance.analyses)
			.collect();
		assert_eq!(snapshots.len(), 2);
		for snapshot in snapshots {
			let outer = snapshot
				.functions
				.values()
				.find(|info| {
					info.symbol.is_some_and(|symbol| {
						session
							.types()
							.syms
							.resolve(snapshot.symbols.symbol(symbol).name)
							== "outer"
					})
				})
				.unwrap();
			assert!(
				snapshot
					.ownership_sources
					.pending_functions
					.contains(&outer.lambda)
			);
		}
	}

	#[test]
	fn final_publication_keeps_the_last_read_and_complete_mutating_bodies() {
		let source =
			"function next = (value: Integer): Integer => {\nmutable n = value\nn = 2\nn\n}\n";
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		let info = analysis
			.functions
			.values()
			.find(|info| info.symbol.is_some())
			.unwrap();
		assert!(
			!analysis
				.ownership_sources
				.pending_functions
				.contains(&info.lambda),
			"facts {:?}; missing {:?}",
			analysis.ownership_sources,
			(0..session.tree(file).nodes.len())
				.filter_map(|index| {
					let node = NodeId::new(index);
					if analysis.ownership_sources.reads.contains_key(&node) {
						return None;
					}
					let Resolution::Local(symbol) = analysis.resolution(node) else {
						return None;
					};
					Some((
						node,
						session.tree(file).text(index),
						analysis.symbols.symbol(symbol).kind,
						analysis.flags_of(node),
					))
				})
				.collect::<Vec<_>>()
		);
		assert!(analysis.ownership_sources.reads.iter().any(|(node, read)| {
			read.context == info.lambda && session.tree(file).text(node.index()) == "n"
		}));
	}
	#[test]
	fn defaults_have_their_own_source_context() {
		let source = "let shared = [1]\nfunction size = (items: Array<Integer> = shared): Integer => items.length\nconsole.log(size(), shared.length)\n";
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		let default = analysis
			.functions
			.values()
			.flat_map(|info| &info.params)
			.find_map(|param| param.default)
			.unwrap();
		assert_eq!(analysis.ownership_sources.reads[&default].context, default);
		assert_eq!(analysis.ownership_sources.reads[&default].sources.len(), 1);
		assert_eq!(
			analysis.ownership_sources.reads[&default].sources[0].origin,
			OwnershipSourceOrigin::Direct
		);
	}

	#[test]
	fn signature_effects_are_not_missing_runtime_reads() {
		let source = "function append = (items: Array<Integer>): Integer mutates items => {\nitems.push(2)\nitems.length\n}\n";
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		let info = analysis
			.functions
			.values()
			.find(|info| info.symbol.is_some())
			.unwrap();
		assert!(
			!analysis
				.ownership_sources
				.pending_functions
				.contains(&info.lambda)
		);
	}
}
