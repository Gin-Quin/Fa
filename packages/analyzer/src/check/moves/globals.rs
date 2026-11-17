//! Module initialization must preserve owners used by its exported entry.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{NodeFlags, OwnershipContext};
use crate::check::Walker;
use crate::ids::{NodeId, PlaceId};
use crate::symbols::{Resolution, ScopeKind};
use crate::syntax::View;

impl Walker<'_> {
	pub(super) fn reserve_exported_owner_sources(
		&mut self,
		free: &FxHashMap<OwnershipContext, FxHashSet<PlaceId>>,
	) {
		self.module_owner_borrows.clear();
		for info in self.a.functions.values() {
			let Some(declaration) = info.declaration else {
				continue;
			};
			if !matches!(self.view(declaration), View::ExportFunction { .. }) {
				continue;
			}
			let contexts = std::iter::once(OwnershipContext::Body(info.lambda)).chain(
				info.params
					.iter()
					.filter_map(|param| param.default)
					.map(OwnershipContext::Default),
			);
			for context in contexts {
				for &place in free.get(&context).into_iter().flatten() {
					if let Some(symbol) = self.place_symbol(place)
						&& self
							.a
							.symbols
							.scope(self.a.symbols.symbol(symbol).scope)
							.kind == ScopeKind::Module
					{
						self.module_owner_borrows
							.entry(symbol)
							.or_insert(declaration);
					}
				}
			}
		}
	}

	pub(super) fn mark_reserved_module_reads(&mut self) {
		let reads: Vec<_> = self
			.a
			.resolutions
			.iter()
			.enumerate()
			.filter_map(|(index, resolution)| {
				if let Resolution::Local(symbol) = resolution
					&& self.module_owner_borrows.contains_key(symbol)
				{
					Some(NodeId::new(index))
				} else {
					None
				}
			})
			.collect();
		for node in reads {
			self.clear_flag(node, NodeFlags::MOVE);
			self.set_flag(node, NodeFlags::BORROWED_OWNER);
		}
	}
}
