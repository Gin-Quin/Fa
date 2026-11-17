//! Storage provenance for aliases, separate from reference validity.
//! Missing metadata and an explicitly empty designation are distinct.

use smallvec::SmallVec;

use super::{FlowState, Places, Step, refs::RefStore};
use crate::ids::{NodeId, PlaceId, SymbolId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Origin {
	Direct,
	/// A logical capture input; `input` retains its checked relative path.
	Capture {
		function: NodeId,
		input: PlaceId,
	},
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Source {
	pub place: PlaceId,
	pub origin: Origin,
}

impl Source {
	pub fn direct(place: PlaceId) -> Self {
		Self {
			place,
			origin: Origin::Direct,
		}
	}
}

pub(crate) type Sources = SmallVec<[Source; 2]>;

/// Project canonical designations and their correlated logical capture inputs.
/// A new lexical capture supersedes the inherited origin, never the source path.
pub(crate) fn project_sources(
	places: &mut Places,
	sources: Sources,
	suffix: &[Step],
	capture: Option<Origin>,
) -> Sources {
	sources
		.into_iter()
		.map(|source| {
			let mut place = source.place;
			for &step in suffix {
				place = places.child(place, step);
			}
			let origin = match (capture, source.origin) {
				(Some(origin), _) => origin,
				(
					None,
					Origin::Capture {
						function,
						mut input,
					},
				) => {
					for &step in suffix {
						input = places.child(input, step);
					}
					Origin::Capture { function, input }
				}
				(None, Origin::Direct) => Origin::Direct,
			};
			Source { place, origin }
		})
		.collect()
}

/// Sorted sparse bindings. A reseat replaces the whole correlated set;
/// forks clone it and joins union complete (place, origin) alternatives.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AliasOrigins(Vec<(SymbolId, Sources)>);

impl AliasOrigins {
	pub fn get(&self, symbol: SymbolId) -> Option<&Sources> {
		self.0
			.binary_search_by_key(&symbol, |(symbol, _)| *symbol)
			.ok()
			.map(|index| &self.0[index].1)
	}

	pub fn set(&mut self, symbol: SymbolId, mut sources: Sources) {
		sources.sort_unstable();
		sources.dedup();
		match self.0.binary_search_by_key(&symbol, |(symbol, _)| *symbol) {
			Ok(index) => self.0[index].1 = sources,
			Err(index) => self.0.insert(index, (symbol, sources)),
		}
	}

	pub fn join(left: &FlowState, right: &FlowState, refs: &RefStore) -> Self {
		let mut symbols: Vec<_> = left
			.ownership_aliases
			.0
			.iter()
			.chain(&right.ownership_aliases.0)
			.map(|(symbol, _)| *symbol)
			.collect();
		symbols.sort_unstable();
		symbols.dedup();
		let mut joined = Self::default();
		for symbol in symbols {
			let mut sources = Self::incoming(left, symbol, refs);
			sources.extend(Self::incoming(right, symbol, refs));
			joined.set(symbol, sources);
		}
		joined
	}

	fn incoming(flow: &FlowState, symbol: SymbolId, refs: &RefStore) -> Sources {
		match flow.ownership_aliases.get(symbol) {
			Some(sources) => sources.clone(),
			None => flow
				.ref_state(symbol)
				.into_iter()
				.flat_map(|state| refs.get(state).places.iter().copied())
				.map(Source::direct)
				.collect(),
		}
	}
}

#[cfg(test)]
mod tests {
	use smallvec::smallvec;

	use super::*;
	use crate::flow::{FactStore, refs::RefState};
	use crate::types::TypeStore;

	fn captured(place: PlaceId) -> Source {
		Source {
			place,
			origin: Origin::Capture {
				function: NodeId(7),
				input: PlaceId(9),
			},
		}
	}

	#[test]
	fn equal_places_retain_distinct_storage_origins() {
		let mut facts = FactStore::default();
		let mut types = TypeStore::new();
		let alias = SymbolId(0);
		let place = PlaceId(1);
		let mut left = FlowState::new();
		left.ownership_aliases
			.set(alias, smallvec![captured(place)]);
		let mut right = left.fork();
		right
			.ownership_aliases
			.set(alias, smallvec![Source::direct(place)]);
		assert_ne!(left, right);
		let joined = left.join(&right, &mut facts, &mut types);
		assert_eq!(joined.ownership_aliases.get(alias).unwrap().len(), 2);
		assert_eq!(
			left.ownership_aliases.get(alias).unwrap().as_slice(),
			&[captured(place)]
		);
	}

	#[test]
	fn missing_metadata_keeps_direct_predecessor_places() {
		let mut facts = FactStore::default();
		let mut types = TypeStore::new();
		let alias = SymbolId(0);
		let place = PlaceId(1);
		let mut left = FlowState::new();
		left.set_ref_state(alias, facts.refs.state(RefState::valid(place)));
		let mut right = left.fork();
		right
			.ownership_aliases
			.set(alias, smallvec![captured(place)]);
		let joined = left.join(&right, &mut facts, &mut types);
		let sources = joined.ownership_aliases.get(alias).unwrap();
		assert!(sources.contains(&Source::direct(place)));
		assert!(sources.contains(&captured(place)));
	}

	#[test]
	fn explicit_clearing_does_not_restore_a_remembered_owner() {
		let mut facts = FactStore::default();
		let mut types = TypeStore::new();
		let alias = SymbolId(0);
		let place = PlaceId(1);
		let mut left = FlowState::new();
		left.set_ref_state(alias, facts.refs.state(RefState::valid(place)));
		left.ownership_aliases
			.set(alias, smallvec![captured(place)]);
		let mut right = left.fork();
		right.ownership_aliases.set(alias, Sources::new());
		let joined = left.join(&right, &mut facts, &mut types);
		assert_eq!(
			joined.ownership_aliases.get(alias).unwrap().as_slice(),
			&[captured(place)]
		);
		assert!(right.ownership_aliases.get(alias).unwrap().is_empty());
	}

	#[test]
	fn a_reseat_replaces_the_join_without_rewriting_earlier_states() {
		let mut facts = FactStore::default();
		let mut types = TypeStore::new();
		let alias = SymbolId(0);
		let place = PlaceId(1);
		let mut captured_state = FlowState::new();
		captured_state
			.ownership_aliases
			.set(alias, smallvec![captured(place)]);
		let mut direct = captured_state.fork();
		direct
			.ownership_aliases
			.set(alias, smallvec![Source::direct(place)]);
		let first = captured_state.join(&direct, &mut facts, &mut types);
		let mut later = first.clone();
		later
			.ownership_aliases
			.set(alias, smallvec![captured(place)]);
		assert_eq!(first.ownership_aliases.get(alias).unwrap().len(), 2);
		assert_eq!(later, captured_state);
	}
	#[test]
	fn a_projection_rebases_source_and_capture_paths_together() {
		let mut places = Places::new();
		let place = places.symbol(SymbolId(0));
		let input = places.symbol(SymbolId(1));
		let origin = Origin::Capture {
			function: NodeId(2),
			input,
		};
		let suffix = [Step::Length];
		let result = project_sources(
			&mut places,
			smallvec![Source { place, origin }],
			&suffix,
			None,
		);
		assert_eq!(result[0].place, places.child(place, Step::Length));
		assert_eq!(
			result[0].origin,
			Origin::Capture {
				function: NodeId(2),
				input: places.child(input, Step::Length),
			}
		);
	}

	#[test]
	fn a_nested_capture_uses_its_lexical_input_without_changing_the_source() {
		let mut places = Places::new();
		let place = places.symbol(SymbolId(0));
		let old = places.symbol(SymbolId(1));
		let input = places.symbol(SymbolId(2));
		let origin = Origin::Capture {
			function: NodeId(3),
			input,
		};
		let sources = smallvec![
			Source {
				place,
				origin: Origin::Capture {
					function: NodeId(1),
					input: old
				},
			},
			Source::direct(place)
		];
		let result = project_sources(&mut places, sources, &[Step::Length], Some(origin));
		assert!(result.iter().all(|source| source.origin == origin));
		assert!(
			result
				.iter()
				.all(|source| source.place == places.child(place, Step::Length))
		);
	}
}
