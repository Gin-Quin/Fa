//! Successful module initialization must preserve its future readers' owners.

use rustc_hash::{FxHashMap, FxHashSet};

use super::graph::Graph;
use crate::analysis::{NodeFlags, OwnershipContext};
use crate::check::Walker;
use crate::ids::{NodeId, PlaceId, SymbolId};
use crate::symbols::ScopeKind;
use crate::syntax::View;

impl Walker<'_> {
	pub(super) fn reserve_exported_owner_sources(
		&mut self,
		free: &FxHashMap<OwnershipContext, FxHashSet<PlaceId>>,
	) {
		self.module_owner_borrows.clear();
		let mut readers = Vec::new();
		for info in self.a.functions.values() {
			if let Some(declaration) = info.declaration {
				let exported = if matches!(self.view(declaration), View::ExportFunction { .. }) {
					Some(declaration)
				} else {
					self.exported_namespace_site(declaration)
				};
				if let Some(exported) = exported {
					readers.push((info.lambda, exported));
				}
			}
		}
		for index in 0..self.tree.nodes.len() {
			let declaration = NodeId::new(index);
			let exported = match self.view(declaration) {
				View::ExportValue { value, .. } => Some((value, declaration)),
				View::Declaration {
					value: Some(value), ..
				} => self
					.exported_namespace_site(declaration)
					.map(|site| (value, site)),
				_ => None,
			};
			if let Some((value, declaration)) = exported {
				readers.extend(
					self.callable_targets(value)
						.into_iter()
						.map(|function| (function, declaration)),
				);
			}
		}
		readers.sort_unstable();
		readers.dedup();
		for (function, declaration) in readers {
			let Some(info) = self.a.function(function) else {
				continue;
			};
			let contexts = std::iter::once(OwnershipContext::Body(info.lambda)).chain(
				info.params
					.iter()
					.filter_map(|param| param.default)
					.map(OwnershipContext::Default),
			);
			for context in contexts {
				for &place in free.get(&context).into_iter().flatten() {
					if let Some(symbol) = self.place_symbol(place)
						&& matches!(
							self.a
								.symbols
								.scope(self.a.symbols.symbol(symbol).scope)
								.kind,
							ScopeKind::Module | ScopeKind::Namespace
						) {
						self.module_owner_borrows
							.entry(symbol)
							.or_insert(declaration);
					}
				}
			}
		}
	}

	/// Public namespace members live with the module. A declaration inside
	/// one of their function bodies is instead an ordinary local capture.
	fn exported_namespace_site(&self, mut node: NodeId) -> Option<NodeId> {
		if self.ownership_body(node).index() != self.tree.root {
			return None;
		}
		while let Some(parent) = self.parent(node) {
			if matches!(self.view(parent), View::ExportNamespace(_)) {
				return Some(parent);
			}
			node = parent;
		}
		None
	}

	pub(super) fn mark_reserved_module_reads(
		&mut self,
		reads: &FxHashMap<NodeId, Vec<SymbolId>>,
		commits: &FxHashMap<NodeId, Vec<NodeId>>,
	) {
		self.module_owner_loans.clear();
		if self.module_owner_borrows.is_empty() {
			return;
		}
		let root = NodeId::new(self.tree.root);
		let graph = Graph::build(self, root, reads, commits);
		for (&symbol, &reader) in &self.module_owner_borrows {
			let later = graph.later_uses_where(symbol, |node| node == root);
			for (&node, positions) in &graph.at {
				if positions.iter().any(|&index| {
					graph.reachable[index]
						&& graph.events[index].reads.contains(&symbol)
						&& later[index].is_some()
				}) {
					self.module_owner_loans.insert((node, symbol), reader);
				}
			}
		}
		let nodes: Vec<_> = self
			.module_owner_loans
			.keys()
			.map(|&(node, _)| node)
			.collect();
		for node in nodes {
			self.clear_flag(node, NodeFlags::MOVE);
			self.set_flag(node, NodeFlags::BORROWED_OWNER);
		}
	}
}
