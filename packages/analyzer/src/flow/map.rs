//! [`FactMap`]: the facts of a flow state, place → fact, in three levels.
//!
//! A flow state is forked at every branch and joined where branches meet, and
//! a loop compares its head between rounds. With a plain sorted vector (the
//! first version of decision A9), each of these costs the number of facts,
//! which grows with the module-level values of a file. Here a state is
//!
//! - a shared, sorted **base** (`Arc`, never mutated),
//! - a shared, sorted **middle** (`Arc`, never mutated): the changes made
//!   since the base was frozen, at most [`MIDDLE_LIMIT`] entries,
//! - a small sorted **overlay**: the changes made since the middle was
//!   frozen, at most [`OVERLAY_LIMIT`] entries.
//!
//! A fork copies two `Arc`s and the overlay. Two states with the same base
//! and middle are compared, joined and widened on their overlays only; with
//! the same base only, on their changes since the base (middle and overlay).
//! Every level is normalized against the levels below it (an entry equal to
//! the value below is not kept; a removal is a [`FactId::TOP`]-valued
//! tombstone only when a level below has the place), so that, with the same
//! lower levels, equal maps have equal upper levels.
//!
//! The overlay stays small even when many places changed since the base (a
//! file whose loops each update their own module-level value): when it grows
//! past its limit it is merged into a new middle, and when the middle grows
//! past its own, everything is merged into a new base. A change costs the
//! overlay's size, plus the middle's size every [`OVERLAY_LIMIT`] changes,
//! plus the map's size every [`MIDDLE_LIMIT`] changes or so.

use std::sync::Arc;

use smallvec::SmallVec;

use crate::ids::{FactId, PlaceId};

/// The overlay size beyond which the overlay is merged into a new middle.
pub const OVERLAY_LIMIT: usize = 8;

/// The middle size beyond which a map is flattened into a new base.
pub const MIDDLE_LIMIT: usize = 64;

/// The removal marker in an overlay.
const TOMBSTONE: FactId = FactId(0);

type Entries = SmallVec<[(PlaceId, FactId); 8]>;

std::thread_local! {
	/// The empty level every new map starts from: a new map allocates nothing.
	static EMPTY: Arc<Vec<(PlaceId, FactId)>> = Arc::default();
}

/// A sorted map from places to facts, with shared lower levels (module docs).
#[derive(Clone, Debug)]
pub struct FactMap {
	base: Arc<Vec<(PlaceId, FactId)>>,
	middle: Arc<Vec<(PlaceId, FactId)>>,
	overlay: Entries,
}

impl Default for FactMap {
	fn default() -> Self {
		let empty = EMPTY.with(Arc::clone);
		Self {
			base: Arc::clone(&empty),
			middle: empty,
			overlay: SmallVec::new(),
		}
	}
}

impl PartialEq for FactMap {
	fn eq(&self, other: &Self) -> bool {
		if Arc::ptr_eq(&self.base, &other.base) {
			if Arc::ptr_eq(&self.middle, &other.middle) {
				return self.overlay == other.overlay;
			}
			return self.delta().eq(other.delta());
		}
		self.iter().eq(other.iter())
	}
}

impl Eq for FactMap {}

fn find(entries: &[(PlaceId, FactId)], place: PlaceId) -> Result<usize, usize> {
	entries.binary_search_by_key(&place, |&(id, _)| id)
}

/// The entries of `lower` overridden by those of `upper` (both sorted),
/// tombstones included; the flag tells an entry of `upper`.
fn layered<'a>(
	lower: &'a [(PlaceId, FactId)],
	upper: &'a [(PlaceId, FactId)],
) -> impl Iterator<Item = ((PlaceId, FactId), bool)> + 'a {
	let (mut i, mut j) = (0, 0);
	std::iter::from_fn(move || match (lower.get(i), upper.get(j)) {
		(None, None) => None,
		(Some(&a), None) => {
			i += 1;
			Some((a, false))
		}
		(None, Some(&b)) => {
			j += 1;
			Some((b, true))
		}
		(Some(&a), Some(&b)) => match a.0.cmp(&b.0) {
			std::cmp::Ordering::Less => {
				i += 1;
				Some((a, false))
			}
			std::cmp::Ordering::Greater => {
				j += 1;
				Some((b, true))
			}
			std::cmp::Ordering::Equal => {
				i += 1;
				j += 1;
				Some((b, true))
			}
		},
	})
}

/// The places whose entries differ between two sorted entry lists that are
/// normalized against the same lower levels.
fn differing(
	a: impl Iterator<Item = (PlaceId, FactId)>,
	b: impl Iterator<Item = (PlaceId, FactId)>,
) -> SmallVec<[PlaceId; 16]> {
	let (mut a, mut b) = (a.peekable(), b.peekable());
	let mut places = SmallVec::new();
	loop {
		match (a.peek().copied(), b.peek().copied()) {
			(None, None) => return places,
			(Some(x), Some(y)) if x.0 == y.0 => {
				if x.1 != y.1 {
					places.push(x.0);
				}
				a.next();
				b.next();
			}
			(Some(x), Some(y)) if x.0 < y.0 => {
				places.push(x.0);
				a.next();
			}
			(Some(x), None) => {
				places.push(x.0);
				a.next();
			}
			(_, Some(y)) => {
				places.push(y.0);
				b.next();
			}
		}
	}
}

/// The entry that records `fact` above a level where the place has `below`:
/// `None` when the level below already says it.
fn normalized(below: Option<FactId>, fact: FactId) -> Option<FactId> {
	match (below, fact) {
		(Some(below), fact) if below == fact => None,
		(None, TOMBSTONE) => None,
		(_, fact) => Some(fact),
	}
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
			middle: EMPTY.with(Arc::clone),
			overlay: SmallVec::new(),
		}
	}

	fn base_fact(&self, place: PlaceId) -> Option<FactId> {
		find(&self.base, place).ok().map(|index| self.base[index].1)
	}

	/// The fact of a place in the base and the middle.
	fn below_overlay(&self, place: PlaceId) -> Option<FactId> {
		match find(&self.middle, place) {
			Ok(index) => Some(self.middle[index].1).filter(|&fact| fact != TOMBSTONE),
			Err(_) => self.base_fact(place),
		}
	}

	/// The fact of a place.
	pub fn get(&self, place: PlaceId) -> Option<FactId> {
		match find(&self.overlay, place) {
			Ok(index) => Some(self.overlay[index].1).filter(|&fact| fact != TOMBSTONE),
			Err(_) => self.below_overlay(place),
		}
	}

	/// Sets the fact of a place; `TOP` removes it.
	pub fn set(&mut self, place: PlaceId, fact: FactId) {
		let entry = normalized(self.below_overlay(place), fact);
		match (find(&self.overlay, place), entry) {
			(Ok(index), Some(fact)) => self.overlay[index].1 = fact,
			(Ok(index), None) => {
				self.overlay.remove(index);
			}
			(Err(index), Some(fact)) => self.overlay.insert(index, (place, fact)),
			(Err(_), None) => {}
		}
		if self.overlay.len() > OVERLAY_LIMIT {
			self.push_down();
		}
	}

	/// The changes since the base, normalized against it: the middle
	/// overridden by the overlay (sorted, computed as it is read).
	fn delta(&self) -> impl Iterator<Item = (PlaceId, FactId)> + '_ {
		layered(&self.middle, &self.overlay).filter_map(|((place, fact), from_overlay)| {
			if !from_overlay {
				return Some((place, fact));
			}
			normalized(self.base_fact(place), fact).map(|fact| (place, fact))
		})
	}

	/// Merges the overlay into a new middle, or everything into a new base
	/// when the middle grows past its limit.
	fn push_down(&mut self) {
		let delta: Vec<_> = self.delta().collect();
		if delta.len() > MIDDLE_LIMIT {
			self.flatten();
			return;
		}
		self.middle = Arc::new(delta);
		self.overlay.clear();
	}

	/// Before a fork: moves an overlay more than half full into a new
	/// middle, which the branches then share. Otherwise a branch would soon
	/// push the changes made before the fork down on its own, and comparing
	/// or joining it with its sibling would visit all of them.
	pub fn freeze(&mut self) {
		if self.overlay.len() > OVERLAY_LIMIT / 2 {
			self.push_down();
		}
	}

	/// Merges the middle and the overlay into a new base.
	pub fn flatten(&mut self) {
		if self.overlay.is_empty() && self.middle.is_empty() {
			return;
		}
		let entries: Vec<_> = self.iter().collect();
		*self = Self::from_sorted(entries);
	}

	/// The entries, sorted by place.
	pub fn iter(&self) -> impl Iterator<Item = (PlaceId, FactId)> + '_ {
		let (base, middle, overlay) = (&self.base[..], &self.middle[..], &self.overlay[..]);
		let mut upper = layered(middle, overlay).map(|(entry, _)| entry).peekable();
		let mut i = 0;
		std::iter::from_fn(move || {
			loop {
				let next = match (base.get(i), upper.peek()) {
					(None, None) => return None,
					(Some(&a), None) => {
						i += 1;
						a
					}
					(None, Some(_)) => upper.next()?,
					(Some(&a), Some(&b)) => match a.0.cmp(&b.0) {
						std::cmp::Ordering::Less => {
							i += 1;
							a
						}
						std::cmp::Ordering::Greater => upper.next()?,
						std::cmp::Ordering::Equal => {
							i += 1;
							upper.next()?
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

	/// The places whose facts differ between two maps of the same base,
	/// sorted; `None` when the bases differ. With the same middle too, one
	/// merge of the two overlays; otherwise of their changes since the base.
	/// An entry both share (a module-level value set before a branch) is
	/// not visited by the joins.
	pub fn changed_places(&self, other: &Self) -> Option<SmallVec<[PlaceId; 16]>> {
		if !Arc::ptr_eq(&self.base, &other.base) {
			return None;
		}
		if Arc::ptr_eq(&self.middle, &other.middle) {
			return Some(differing(
				self.overlay.iter().copied(),
				other.overlay.iter().copied(),
			));
		}
		Some(differing(self.delta(), other.delta()))
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
		assert_eq!(fork, map);
		fork.set(p(6), f(60));
		assert_ne!(fork, map);
		// p(5) is in both overlays with the same fact: only p(6) differs
		assert_eq!(map.changed_places(&fork).unwrap().as_slice(), &[p(6)]);
		let mut other = map.clone();
		other.set(p(5), f(51));
		other.set(p(1), TOMBSTONE);
		assert_eq!(
			fork.changed_places(&other).unwrap().as_slice(),
			&[p(1), p(5), p(6)]
		);
		// equal contents with different bases are equal
		let mut flat = fork.clone();
		flat.flatten();
		assert!(!Arc::ptr_eq(&flat.base, &fork.base));
		assert_eq!(flat, fork);
		assert_eq!(flat.changed_places(&fork), None);
	}

	#[test]
	fn a_large_overlay_moves_to_the_middle_then_to_the_base() {
		let mut map = FactMap::from_sorted(vec![(p(1000), f(1))]);
		let base = Arc::clone(&map.base);
		for index in 0..(OVERLAY_LIMIT as u32 + 3) {
			map.set(p(index), f(index + 1));
		}
		assert!(map.overlay.len() <= OVERLAY_LIMIT);
		assert!(!map.middle.is_empty());
		assert!(Arc::ptr_eq(&map.base, &base), "the base is kept");
		assert_eq!(map.len(), OVERLAY_LIMIT + 4);
		for index in 0..(MIDDLE_LIMIT as u32 + 10) {
			map.set(p(index), f(index + 2));
			assert_eq!(map.get(p(index)), Some(f(index + 2)));
		}
		assert!(map.middle.len() <= MIDDLE_LIMIT);
		assert!(!Arc::ptr_eq(&map.base, &base), "flattened");
		assert_eq!(map.len(), MIDDLE_LIMIT + 11);
		map.retain(|place, _| place.0 % 2 == 0);
		assert!(map.iter().all(|(place, _)| place.0 % 2 == 0));
	}

	#[test]
	fn maps_with_different_middles_compare_by_their_changes() {
		let mut map = FactMap::from_sorted(vec![(p(0), f(1)), (p(100), f(1))]);
		for index in 1..=(OVERLAY_LIMIT as u32 + 1) {
			map.set(p(index), f(7));
		}
		let mut fork = map.clone();
		let mut other = map.clone();
		// the fork pushes its overlay down; the other keeps it
		for index in 20..=(20 + OVERLAY_LIMIT as u32) {
			fork.set(p(index), f(9));
			other.set(p(index), f(9));
		}
		other.set(p(50), f(9));
		other.set(p(50), TOMBSTONE);
		assert_eq!(fork, other);
		other.set(p(100), TOMBSTONE);
		other.set(p(3), f(1));
		assert_ne!(fork, other);
		assert_eq!(
			fork.changed_places(&other).unwrap().as_slice(),
			&[p(3), p(100)]
		);
		// a value set back to the base's, above a middle that changed it
		let mut back = fork.clone();
		back.set(p(0), f(2));
		for index in 40..=(40 + OVERLAY_LIMIT as u32) {
			back.set(p(index), f(3));
		}
		back.set(p(0), f(1));
		let mut expected = fork.clone();
		for index in 40..=(40 + OVERLAY_LIMIT as u32) {
			expected.set(p(index), f(3));
		}
		assert_eq!(back, expected);
		assert_eq!(
			back.iter().collect::<Vec<_>>(),
			expected.iter().collect::<Vec<_>>()
		);
	}
}
