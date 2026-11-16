//! Reference validity (advanced brief §5.1): the state of each local
//! reference (`use`, `mutable use`) in the flow state, hash-consed so that
//! the flow state stays a small vector of ids.

use smallvec::SmallVec;

use crate::ids::{NodeId, PlaceId, RefStateId};
use crate::intern::Interner;

/// The state of a local reference at a program point.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct RefState {
	/// The places it may designate, sorted: one, or several for a `mutable
	/// use` moved differently in two branches.
	pub places: SmallVec<[PlaceId; 2]>,
	/// Its physical projection cannot be compressed to an owner-relative key.
	pub opaque: bool,
	/// `None` while valid; else the operation that invalidated it, and
	/// whether only some paths did (`maybe`: `REF002`, else `REF001`).
	pub invalidated: Option<(NodeId, bool)>,
	/// The invalidation was reported at a use on this path: the later uses
	/// are not reported again (a join forgets it: each path reports).
	pub reported: bool,
}

impl RefState {
	/// A valid reference to `place`.
	pub fn valid(place: PlaceId) -> Self {
		Self {
			places: SmallVec::from_slice(&[place]),
			opaque: false,
			invalidated: None,
			reported: false,
		}
	}
}

/// The per-file store of reference states.
#[derive(Default)]
pub struct RefStore {
	states: Interner<RefState>,
}

impl RefStore {
	/// An empty store.
	pub fn new() -> Self {
		Self::default()
	}

	/// Interns a state.
	pub fn state(&mut self, state: RefState) -> RefStateId {
		RefStateId(self.states.intern(state))
	}

	/// A state.
	pub fn get(&self, id: RefStateId) -> &RefState {
		self.states.resolve(id.0)
	}

	/// The state where two branches meet (advanced brief §5.6): valid only
	/// if valid on both sides; definitely invalidated only if on both; the
	/// places of both.
	pub fn join(&mut self, a: RefStateId, b: RefStateId) -> RefStateId {
		if a == b {
			return a;
		}
		let (left, right) = (self.get(a).clone(), self.get(b).clone());
		let mut places = left.places.clone();
		for place in right.places {
			if !places.contains(&place) {
				places.push(place);
			}
		}
		places.sort();
		let invalidated = match (left.invalidated, right.invalidated) {
			(None, None) => None,
			(Some((by, _)), None) | (None, Some((by, _))) => Some((by, true)),
			(Some((by, maybe)), Some((_, other))) => Some((by, maybe || other)),
		};
		self.state(RefState {
			places,
			opaque: left.opaque || right.opaque,
			invalidated,
			reported: false,
		})
	}
}

/// Joins two sorted lists of reference states: the symbols of both, joined.
pub fn join_refs(
	store: &mut RefStore,
	a: &[(crate::ids::SymbolId, RefStateId)],
	b: &[(crate::ids::SymbolId, RefStateId)],
) -> SmallVec<[(crate::ids::SymbolId, RefStateId); 4]> {
	let mut joined = SmallVec::new();
	let (mut i, mut j) = (0, 0);
	while i < a.len() && j < b.len() {
		match a[i].0.cmp(&b[j].0) {
			std::cmp::Ordering::Less => i += 1,
			std::cmp::Ordering::Greater => j += 1,
			std::cmp::Ordering::Equal => {
				joined.push((a[i].0, store.join(a[i].1, b[j].1)));
				i += 1;
				j += 1;
			}
		}
	}
	joined
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ids::SymbolId;

	#[test]
	fn joins_follow_the_validity_lattice() {
		let mut store = RefStore::new();
		let valid = store.state(RefState::valid(PlaceId(1)));
		let gone = store.state(RefState {
			invalidated: Some((NodeId(7), false)),
			..RefState::valid(PlaceId(1))
		});
		let maybe = store.join(valid, gone);
		assert_eq!(store.get(maybe).invalidated, Some((NodeId(7), true)));
		assert_eq!(store.join(gone, gone), gone);
		assert_eq!(store.join(valid, valid), valid);
		let other = store.state(RefState::valid(PlaceId(2)));
		let both = store.join(valid, other);
		assert_eq!(store.get(both).places.as_slice(), &[PlaceId(1), PlaceId(2)]);
		let a = [(SymbolId(1), valid), (SymbolId(2), valid)];
		let b = [(SymbolId(2), gone), (SymbolId(3), valid)];
		let joined = join_refs(&mut store, &a, &b);
		assert_eq!(joined.len(), 1);
		assert_eq!(store.get(joined[0].1).invalidated, Some((NodeId(7), true)));
	}
}
