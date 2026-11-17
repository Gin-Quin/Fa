//! Incoming owners stop being observed after a complete replacement.

use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::Graph;
use crate::analysis::OwnershipContext;
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

pub(super) type IncomingReads = FxHashMap<(OwnershipContext, SymbolId), FxHashSet<NodeId>>;

impl Walker<'_> {
	/// Closed replacement proofs. Aliases and transitive observations need
	/// their own storage versions; never identify them with a replaced slot.
	pub(super) fn versioned_source_roots(
		&self,
		defaults: &FxHashSet<NodeId>,
	) -> FxHashSet<(OwnershipContext, SymbolId)> {
		let mut roots = FxHashSet::default();
		let mut blocked = FxHashSet::default();
		for (&node, &place) in &self.move_writes {
			let Some(symbol) = self.place_symbol(place) else {
				continue;
			};
			if !self.places.steps(place).is_empty()
				|| self.ownership_body(self.a.symbols.symbol(symbol).decl_node)
					== self.ownership_body(node)
			{
				continue;
			}
			let key = (self.invocation_context(node, defaults), symbol);
			let direct = matches!(self.view(node), View::Assignment { target, .. }
				if matches!(self.view(target), View::Name(_)) && self.a.resolution(target) == Resolution::Local(symbol));
			let body = self.source_context_body(key.0);
			if direct
				&& self.a.symbols.symbol(symbol).kind == SymbolKind::Mutable
				&& matches!(self.view(body), View::Block { .. } | View::Module { .. })
			{
				roots.insert(key);
			} else {
				blocked.insert(key);
			}
		}
		for (&node, places) in &self.move_places {
			if self.access_roots.contains(&node) {
				continue;
			}
			let context = self.invocation_context(node, defaults);
			for &place in places {
				if let Some(symbol) = self.place_symbol(place)
					&& roots.contains(&(context, symbol))
					&& crate::lower::copies::place_root(&self.tree, &self.a, node) != Some(symbol)
				{
					blocked.insert((context, symbol));
				}
			}
		}
		roots.retain(|key| !blocked.contains(key));
		roots
	}

	pub(super) fn incoming_source_reads(
		&self,
		reads: &FxHashMap<NodeId, Vec<SymbolId>>,
		defaults: &FxHashSet<NodeId>,
	) -> IncomingReads {
		let mut incoming = IncomingReads::default();
		for (context, symbol) in self.versioned_source_roots(defaults) {
			let body = self.source_context_body(context);
			let graph = Graph::build(self, body, reads, &FxHashMap::default());
			let Some(before) = graph.before_definition(body, symbol) else {
				continue;
			};
			let nodes = graph
				.at
				.iter()
				.filter_map(|(&node, positions)| {
					positions
						.iter()
						.any(|&position| before[position])
						.then_some(node)
				})
				.collect();
			incoming.insert((context, symbol), nodes);
		}
		incoming
	}

	pub(super) fn observes_incoming_owner(
		&self,
		incoming: &IncomingReads,
		context: OwnershipContext,
		node: NodeId,
		symbol: SymbolId,
	) -> bool {
		crate::lower::copies::place_root(&self.tree, &self.a, node) != Some(symbol)
			|| incoming
				.get(&(context, symbol))
				.is_none_or(|nodes| nodes.contains(&node))
	}

	fn source_context_body(&self, context: OwnershipContext) -> NodeId {
		match context {
			OwnershipContext::Body(node) => self.a.function(node).map_or(node, |info| info.body),
			OwnershipContext::Default(node) => node,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::{analyzed, codes};

	#[test]
	fn replacement_captures_publish_both_whole_owner_transfers() {
		let source = include_str!("../../../tests/cases/ownership-capture-replacements.fa");
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		assert_eq!(
			analysis.capture_moves.len(),
			2,
			"{:?}",
			analysis.capture_moves
		);
		assert_eq!(analysis.capture_moves, analysis.clone().capture_moves);
	}

	#[test]
	fn a_read_before_replacement_or_on_a_branch_still_observes_the_incoming_owner() {
		for body in [
			"let before = items.length\nitems = [2]\nbefore + items.length",
			"if flag { items = [2] }\nitems.length",
		] {
			let source = format!(
				"type Reader = () => Integer\nfunction make = (flag: Boolean): Reader => {{\nmutable items = [1]\nfunction replace = (): Integer => {{ {body} }}\nfunction saved = (): Integer => items.length\nconsole.log(replace(), flag)\nsaved\n}}\n"
			);
			assert_eq!(codes(&source), ["MEM004"], "{source}");
		}
	}
}
