//! Ownership observations retain storage origin before aliases are expanded.

use crate::check::Walker;
use crate::flow::ownership::{Origin, Source, Sources, project_sources};
use crate::ids::{NodeId, PlaceId, SymbolId};

impl Walker<'_> {
	/// Sources of a lexical root in the current flow. An explicit empty row
	/// is authoritative even when reference validity remembers an old owner.
	fn ownership_root_sources(&mut self, symbol: SymbolId) -> Sources {
		if let Some(sources) = self.body.flow.ownership_aliases.get(symbol) {
			return sources.clone();
		}
		if let Some(state) = self.body.flow.ref_state(symbol) {
			return self
				.facts
				.refs
				.get(state)
				.places
				.iter()
				.copied()
				.map(Source::direct)
				.collect();
		}
		let place = self
			.aliases
			.get(&symbol)
			.copied()
			.unwrap_or_else(|| self.places.symbol(symbol));
		std::iter::once(Source::direct(place)).collect()
	}

	/// Rebase the already checked projection onto each current designation.
	/// `aliases` supplies a path basis only, never the alternatives of a
	/// reseatable binding: those come from the flow above.
	fn ownership_sources(&mut self, symbol: SymbolId, place: PlaceId) -> Sources {
		let sources = self.ownership_root_sources(symbol);
		let basis = self
			.aliases
			.get(&symbol)
			.copied()
			.unwrap_or_else(|| self.places.symbol(symbol));
		let basis = std::iter::once(basis)
			.chain(sources.iter().map(|source| source.place))
			.find(|&basis| self.places.is_at_or_below(place, basis));
		let Some(basis) = basis else {
			return std::iter::once(Source::direct(place)).collect();
		};
		let steps = self.place_steps(place);
		let suffix = &steps[self.place_steps(basis).len()..];
		// Recognize the lexical capture before following any alias origin.
		let capture = self.is_captured(symbol).then(|| {
			let function = self
				.body
				.functions
				.last()
				.expect("a captured symbol has a function")
				.node;
			let mut input = self.places.symbol(symbol);
			for &step in suffix {
				input = self.places.child(input, step);
			}
			Origin::Capture { function, input }
		});
		project_sources(&mut self.places, sources, suffix, capture)
	}

	pub(crate) fn record_alias_origins(
		&mut self,
		alias: SymbolId,
		value: NodeId,
		place: Option<PlaceId>,
	) {
		if !self.body.flow.reachable {
			return;
		}
		let sources =
			place.map_or_else(
				Sources::new,
				|place| match crate::lower::copies::value_place_root(&self.tree, &self.a, value) {
					Some(symbol) => self.ownership_sources(symbol, place),
					None => std::iter::once(Source::direct(place)).collect(),
				},
			);
		self.body.flow.ownership_aliases.set(alias, sources);
	}

	pub(crate) fn record_ownership_read(&mut self, node: NodeId, places: &[PlaceId]) {
		if !self.body.flow.reachable {
			return;
		}
		let root = crate::lower::copies::value_place_root(&self.tree, &self.a, node);
		let mut sources = Sources::new();
		for &place in places {
			match root {
				Some(symbol) => sources.extend(self.ownership_sources(symbol, place)),
				None => sources.push(Source::direct(place)),
			}
		}
		sources.sort_unstable();
		sources.dedup();
		// Replace this pass's snapshot; speculative loop rounds must not
		// leave origins which a subsequent unconditional reseat removed.
		self.ownership_read_origins.insert(node, sources);
	}
}
