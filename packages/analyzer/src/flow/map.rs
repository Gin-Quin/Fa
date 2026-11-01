//! [`FactMap`]: the facts of a flow state, place → fact, in two levels.
//!
//! A flow state is forked at every branch and joined where branches meet, and
//! a loop compares its head between rounds. With a plain sorted vector (the
//! first version of decision A9), each of these costs the number of facts,
//! which grows with the module-level values of a file. Here a state is a
//! shared, sorted **base** (`Arc`, never mutated) plus a small sorted
//! **overlay** of the changes made since the base was frozen:
//!
//! - a fork copies the `Arc` and the overlay (at most [`OVERLAY_LIMIT`] entries);
//! - two states with the same base are compared, joined and widened on their
//!   overlays only: the places that neither changed have the same fact;
//! - the overlay is normalized (an entry equal to the base's value is not
//!   kept; a removal is a [`FactId::TOP`]-valued tombstone only when the base
//!   has the place), so that, with the same base, equal maps have equal overlays.
//!
//! When the overlay grows past the limit, it is merged into a new base.

use std::sync::Arc;

use smallvec::SmallVec;

use crate::ids::{FactId, PlaceId};

/// The overlay size beyond which a map is flattened into a new base.
pub const OVERLAY_LIMIT: usize = 24;

/// The removal marker in an overlay.
const TOMBSTONE: FactId = FactId(0);

type Entries = SmallVec<[(PlaceId, FactId); 8]>;

/// A sorted map from places to facts, with a shared base (module docs).
#[derive(Clone, Debug, Default)]
pub struct FactMap {
	base: Arc<Vec<(PlaceId, FactId)>>,
	overlay: Entries,
}

impl PartialEq for FactMap {
	fn eq(&self, other: &Self) -> bool {
		if Arc::ptr_eq(&self.base, &other.base) {
			return self.overlay == other.overlay;
		}
		self.iter().eq(other.iter())
	}
}

impl Eq for FactMap {}

fn find(entries: &[(PlaceId, FactId)], place: PlaceId) -> Result<usize, usize> {
	entries.binary_search_by_key(&place, |&(id, _)| id)
}

impl FactMap {
	/// An empty map.
	pub fn new() -> Self {
		Self::default()
	}

	/// A map holding these entries (sorted by place, no `TOP`).
	pub fn from_sorted(entries: Vec<(PlaceId, FactId)>) -> Self {
		debug_assert!(entries.windows(2).all(|pair| pair[0].0 < pair[1].0));
		Self {
			base: Arc::new(entries),
			overlay: SmallVec::new(),
		}
	}

	fn base_fact(&self, place: PlaceId) -> Option<FactId> {
		find(&self.base, place).ok().map(|index| self.base[index].1)
	}

	/// The fact of a place.
	pub fn get(&self, place: PlaceId) -> Option<FactId> {
		match find(&self.overlay, place) {
			Ok(index) => Some(self.overlay[index].1).filter(|&fact| fact != TOMBSTONE),
			Err(_) => self.base_fact(place),
		}
	}

	/// Sets the fact of a place; `TOP` removes it.
	pub fn set(&mut self, place: PlaceId, fact: FactId) {
		let base = self.base_fact(place);
		// normalized: an overlay entry only where the map differs from the base
		let entry = match (base, fact) {
			(Some(base), fact) if base == fact => None,
			(None, TOMBSTONE) => None,
			(_, fact) => Some(fact),
		};
		match (find(&self.overlay, place), entry) {
			(Ok(index), Some(fact)) => self.overlay[index].1 = fact,
			(Ok(index), None) => {
				self.overlay.remove(index);
			}
			(Err(index), Some(fact)) => self.overlay.insert(index, (place, fact)),
			(Err(_), None) => {}
		}
		if self.overlay.len() > OVERLAY_LIMIT {
			self.flatten();
		}
	}

	/// Merges the overlay into a new base.
	pub fn flatten(&mut self) {
		if self.overlay.is_empty() {
			return;
		}
		let entries: Vec<_> = self.iter().collect();
		*self = Self::from_sorted(entries);
	}

	/// The entries, sorted by place.
	pub fn iter(&self) -> impl Iterator<Item = (PlaceId, FactId)> + '_ {
		let (base, overlay) = (&self.base[..], &self.overlay[..]);
		let (mut i, mut j) = (0, 0);
		std::iter::from_fn(move || {
			loop {
				let next = match (base.get(i), overlay.get(j)) {
					(None, None) => return None,
					(Some(&a), None) => {
						i += 1;
						a
					}
					(None, Some(&b)) => {
						j += 1;
						b
					}
					(Some(&a), Some(&b)) => match a.0.cmp(&b.0) {
						std::cmp::Ordering::Less => {
							i += 1;
							a
						}
						std::cmp::Ordering::Greater => {
							j += 1;
							b
						}
						std::cmp::Ordering::Equal => {
							i += 1;
							j += 1;
							b
						}
					},
				};
				if next.1 != TOMBSTONE {
					return Some(next);
				}
			}
		})
	}

	/// The number of entries.
	pub fn len(&self) -> usize {
		self.iter().count()
	}

	/// Whether there is no entry.
	pub fn is_empty(&self) -> bool {
		self.iter().next().is_none()
	}

	/// Keeps the entries for which `keep` holds (a full pass: rare operations).
	pub fn retain(&mut self, mut keep: impl FnMut(PlaceId, FactId) -> bool) {
		let entries: Vec<_> = self
			.iter()
			.filter(|&(place, fact)| keep(place, fact))
			.collect();
		*self = Self::from_sorted(entries);
	}

	/// The places that may differ between two maps of the same base (their
	/// overlays' places, sorted, deduplicated); `None` when the bases differ.
	pub fn changed_places(&self, other: &Self) -> Option<SmallVec<[PlaceId; 16]>> {
		if !Arc::ptr_eq(&self.base, &other.base) {
			return None;
		}
		let mut places: SmallVec<[PlaceId; 16]> = self
			.overlay
			.iter()
			.chain(other.overlay.iter())
			.map(|&(place, _)| place)
			.collect();
		places.sort_unstable();
		places.dedup();
		Some(places)
	}

	/// The places of the overlay: those that differ from the base.
	pub fn overlay_places(&self) -> SmallVec<[PlaceId; 16]> {
		self.overlay.iter().map(|&(place, _)| place).collect()
	}

	/// Whether two maps share their base.
	pub fn same_base(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.base, &other.base)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn p(index: u32) -> PlaceId {
		PlaceId(index)
	}

	fn f(index: u32) -> FactId {
		FactId(index)
	}

	#[test]
	fn set_get_and_tombstones() {
		let mut map = FactMap::from_sorted(vec![(p(1), f(10)), (p(3), f(30))]);
		assert_eq!(map.get(p(1)), Some(f(10)));
		map.set(p(2), f(20));
		map.set(p(1), TOMBSTONE);
		assert_eq!(map.get(p(1)), None);
		assert_eq!(
			map.iter().collect::<Vec<_>>(),
			[(p(2), f(20)), (p(3), f(30))]
		);
		assert_eq!(map.len(), 2);
		// setting the base value back removes the overlay entry
		map.set(p(1), f(10));
		map.set(p(2), TOMBSTONE);
		assert!(map.overlay.is_empty());
		assert_eq!(
			map,
			FactMap::from_sorted(vec![(p(1), f(10)), (p(3), f(30))])
		);
	}

	#[test]
	fn forks_share_their_base_and_compare_by_overlay() {
		let mut map = FactMap::from_sorted(vec![(p(1), f(10))]);
		map.set(p(5), f(50));
		let mut fork = map.clone();
		assert!(fork.same_base(&map));
		assert_eq!(fork, map);
		fork.set(p(6), f(60));
		assert_ne!(fork, map);
		assert_eq!(map.changed_places(&fork).unwrap().as_slice(), &[p(5), p(6)]);
		// equal contents with different bases are equal
		let mut flat = fork.clone();
		flat.flatten();
		assert!(!flat.same_base(&fork));
		assert_eq!(flat, fork);
	}

	#[test]
	fn a_large_overlay_is_flattened() {
		let mut map = FactMap::new();
		for index in 0..(OVERLAY_LIMIT as u32 + 5) {
			map.set(p(index), f(index + 1));
		}
		assert!(map.overlay.len() <= OVERLAY_LIMIT);
		assert_eq!(map.len(), OVERLAY_LIMIT + 5);
		map.retain(|place, _| place.0 % 2 == 0);
		assert!(map.iter().all(|(place, _)| place.0 % 2 == 0));
	}
}
