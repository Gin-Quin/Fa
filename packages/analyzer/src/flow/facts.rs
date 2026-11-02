//! Flow facts: hash-consed, so that fact equality (and the fixpoint test of
//! loops) is an id compare.
//!
//! The integer fact of a place is an [`IntervalSet`]: an interval and at most
//! [`MAX_EXCLUDED`] excluded intervals, the oldest dropped first (values
//! brief D-95). Joins take the hull and the intersection of the exclusions;
//! widening sends a bound that moved outward to ±∞, then clips it to the
//! type (D-96).

use ethnum::I256;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::ids::{
	ConstId, ConstListId, FactId, IntervalId, IntervalSetId, NodeId, PlaceId, SymbolId, TypeId,
};
use crate::intern::{Interner, SliceInterner};
use crate::types::TypeStore;
use crate::types::int::{INFINITY, Interval};

/// The most excluded intervals an integer fact keeps (values brief D-95).
pub const MAX_EXCLUDED: usize = 8;

/// An interval minus excluded intervals.
///
/// Normalized: the exclusions lie strictly inside the hull and don't touch its
/// ends (an excluded end shrinks the hull), and are kept in insertion order.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct IntervalSet {
	/// The hull.
	pub hull: Interval,
	/// The excluded intervals, oldest first.
	pub excluded: Vec<Interval>,
}

impl IntervalSet {
	/// The set of every value of `interval`.
	pub fn of(interval: Interval) -> Self {
		Self {
			hull: interval,
			excluded: Vec::new(),
		}
	}

	/// Normalizes `hull` minus `excluded`; `None` when no value is left.
	pub fn new(hull: Interval, excluded: impl IntoIterator<Item = Interval>) -> Option<Self> {
		let mut exclusions: Vec<Interval> = excluded.into_iter().collect();
		let mut hull = hull;
		// shrink the ends while an exclusion covers them
		loop {
			let mut changed = false;
			for exclusion in &exclusions {
				if exclusion.contains(hull.lo) {
					if exclusion.hi >= hull.hi {
						return None;
					}
					hull = Interval::new(exclusion.hi + 1, hull.hi);
					changed = true;
				}
				if exclusion.contains(hull.hi) {
					if exclusion.lo <= hull.lo {
						return None;
					}
					hull = Interval::new(hull.lo, exclusion.lo - 1);
					changed = true;
				}
			}
			if !changed {
				break;
			}
		}
		exclusions.retain_mut(|exclusion| {
			if let Some(inner) = exclusion.intersect(hull) {
				*exclusion = inner;
				true
			} else {
				false
			}
		});
		while exclusions.len() > MAX_EXCLUDED {
			exclusions.remove(0);
		}
		Some(Self {
			hull,
			excluded: exclusions,
		})
	}

	/// Whether `value` is in the set.
	pub fn contains(&self, value: I256) -> bool {
		self.hull.contains(value)
			&& !self
				.excluded
				.iter()
				.any(|exclusion| exclusion.contains(value))
	}

	/// Whether the set has exactly one value.
	pub fn as_exact(&self) -> Option<I256> {
		self.hull.as_exact()
	}

	/// The set without `interval` (`x != literal`, the false branch of `x is a..b`).
	pub fn exclude(&self, interval: Interval) -> Option<Self> {
		if interval.is_disjoint(self.hull) {
			return Some(self.clone());
		}
		Self::new(self.hull, self.excluded.iter().copied().chain([interval]))
	}

	/// The values also in `interval` (comparisons, `x is a..b`, `x == literal`).
	pub fn restrict(&self, interval: Interval) -> Option<Self> {
		let hull = self.hull.intersect(interval)?;
		Self::new(hull, self.excluded.iter().copied())
	}

	/// Join: the hull of the hulls and the intersection of the excluded
	/// values (values brief §2.1), in this set's order.
	pub fn join(&self, other: &Self) -> Self {
		let hull = self.hull.hull(other.hull);
		let excluded = self.excluded.iter().flat_map(|mine| {
			other
				.excluded
				.iter()
				.filter_map(move |theirs| mine.intersect(*theirs))
		});
		Self::new(hull, excluded).expect("a join is never empty")
	}

	/// Widening against the previous loop head (D-96): a bound that moved
	/// outward becomes infinite, then is clipped to `limit` (the type's range).
	pub fn widen(&self, previous: &Self, limit: Interval) -> Self {
		let lo = if self.hull.lo < previous.hull.lo {
			limit.lo
		} else {
			self.hull.lo
		};
		let hi = if self.hull.hi > previous.hull.hi {
			limit.hi
		} else {
			self.hull.hi
		};
		let hull = Interval::new(lo.min(self.hull.lo), hi.max(self.hull.hi));
		Self::new(hull, self.excluded.iter().copied()).expect("widening never empties a set")
	}
}

/// What the flow walk knows about a place beyond its declared type.
///
/// Every part is optional: `UNSET` / `NONE` / `EMPTY` mean "nothing known".
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Fact {
	/// The member set the place was narrowed to (`UNSET`: no narrowing).
	pub narrowed: TypeId,
	/// The integer values it may hold (`NONE`: the range of its type).
	pub int: IntervalSetId,
	/// Its exact value (strings, booleans, enumeration members, numbers).
	pub literal: ConstId,
	/// The length of a collection or the grapheme count of a string.
	pub length: IntervalId,
	/// Map keys proven present (types brief D32).
	pub key_present: ConstListId,
	/// An integer place below the length of a collection: `(c, k)` means
	/// `value + k < c.length` (decisions.md, "Index proofs").
	pub below: Option<(PlaceId, i32)>,
}

impl Fact {
	/// The fact that knows nothing.
	pub const TOP: Fact = Fact {
		narrowed: TypeId::UNSET,
		int: IntervalSetId::NONE,
		literal: ConstId::NONE,
		length: IntervalId::NONE,
		key_present: ConstListId::EMPTY,
		below: None,
	};

	/// Whether nothing is known.
	pub fn is_top(&self) -> bool {
		*self == Self::TOP
	}
}

/// The per-file store of hash-consed facts.
pub struct FactStore {
	intervals: Interner<Interval>,
	sets: Interner<IntervalSet>,
	facts: Interner<Fact>,
	const_lists: SliceInterner<ConstId>,
	/// The facts of immutable bindings (`let`), which hold for the whole
	/// scope of the binding: kept out of the flow states, so that a file
	/// with thousands of constants doesn't make every fork and join copy them.
	bindings: FxHashMap<SymbolId, FactId>,
	/// The stable head of each loop walked so far: walked again (inside an
	/// outer loop), a loop starts from it joined with its new entry state,
	/// which the fixpoint would reach anyway since facts only widen.
	loop_heads: FxHashMap<NodeId, crate::flow::FlowState>,
}

impl Default for FactStore {
	fn default() -> Self {
		Self::new()
	}
}

impl FactStore {
	/// A store whose id 0 of each table means "nothing known".
	pub fn new() -> Self {
		let mut store = Self {
			intervals: Interner::new(),
			sets: Interner::new(),
			facts: Interner::new(),
			const_lists: SliceInterner::new(),
			bindings: FxHashMap::default(),
			loop_heads: FxHashMap::default(),
		};
		store.intervals.intern(Interval::full());
		store.sets.intern(IntervalSet::of(Interval::full()));
		store.facts.intern(Fact::TOP);
		store
	}

	/// Records the fact of an immutable binding ([`Fact::TOP`]: nothing known).
	pub fn bind(&mut self, symbol: SymbolId, fact: FactId) {
		if fact == FactId(0) {
			self.bindings.remove(&symbol);
		} else {
			self.bindings.insert(symbol, fact);
		}
	}

	/// The stable head of a loop, from its previous walk.
	pub fn loop_head(&self, node: NodeId) -> Option<&crate::flow::FlowState> {
		self.loop_heads.get(&node)
	}

	/// Records the stable head of a loop.
	pub fn set_loop_head(&mut self, node: NodeId, head: crate::flow::FlowState) {
		self.loop_heads.insert(node, head);
	}

	/// The fact of an immutable binding.
	pub fn binding(&self, symbol: SymbolId) -> Option<FactId> {
		self.bindings.get(&symbol).copied()
	}

	/// Interns an interval.
	pub fn interval(&mut self, interval: Interval) -> IntervalId {
		IntervalId(self.intervals.intern(interval))
	}

	/// The interval of an id (`NONE` is `[−∞, +∞]`).
	pub fn interval_value(&self, id: IntervalId) -> Interval {
		*self.intervals.resolve(id.0)
	}

	/// Interns an interval set.
	pub fn interval_set(&mut self, set: IntervalSet) -> IntervalSetId {
		IntervalSetId(self.sets.intern(set))
	}

	/// The interval set of an id (`NONE` is `[−∞, +∞]`: clip with the type's range).
	pub fn interval_set_value(&self, id: IntervalSetId) -> &IntervalSet {
		self.sets.resolve(id.0)
	}

	/// Interns a fact. [`Fact::TOP`] is `FactId(0)`.
	pub fn fact(&mut self, fact: Fact) -> FactId {
		FactId(self.facts.intern(fact))
	}

	/// The fact of an id.
	pub fn fact_value(&self, id: FactId) -> Fact {
		*self.facts.resolve(id.0)
	}

	/// The fact that a place holds integers of `set`.
	pub fn int_fact(&mut self, set: IntervalSet) -> FactId {
		let int = self.interval_set(set);
		self.fact(Fact { int, ..Fact::TOP })
	}

	/// The fact that a place holds exactly `value` (an integer: also its interval).
	pub fn exact_int_fact(&mut self, value: I256, constant: ConstId) -> FactId {
		let int = self.interval_set(IntervalSet::of(Interval::exact(value)));
		self.fact(Fact {
			int,
			literal: constant,
			..Fact::TOP
		})
	}

	/// Interns a list of constants (sorted, deduplicated).
	pub fn const_list(&mut self, items: &[ConstId]) -> ConstListId {
		let mut sorted: SmallVec<[ConstId; 4]> = items.into();
		sorted.sort_unstable();
		sorted.dedup();
		ConstListId(self.const_lists.intern(&sorted))
	}

	/// The constants of a list.
	pub fn const_list_items(&self, list: ConstListId) -> &[ConstId] {
		self.const_lists.resolve(list.0)
	}

	/// The join of two facts of a place, at the meeting point of two branches.
	pub fn join(&mut self, a: FactId, b: FactId, types: &mut TypeStore) -> FactId {
		if a == b {
			return a;
		}
		let (x, y) = (self.fact_value(a), self.fact_value(b));
		let narrowed = if x.narrowed.is_unset() || y.narrowed.is_unset() {
			TypeId::UNSET
		} else {
			types.union(&[x.narrowed, y.narrowed])
		};
		let int = if x.int == IntervalSetId::NONE || y.int == IntervalSetId::NONE {
			IntervalSetId::NONE
		} else {
			let joined = self
				.interval_set_value(x.int)
				.join(self.interval_set_value(y.int));
			self.interval_set(joined)
		};
		let literal = if x.literal == y.literal {
			x.literal
		} else {
			ConstId::NONE
		};
		let length = if x.length == IntervalId::NONE || y.length == IntervalId::NONE {
			IntervalId::NONE
		} else {
			let hull = self
				.interval_value(x.length)
				.hull(self.interval_value(y.length));
			self.interval(hull)
		};
		let key_present = {
			let theirs = self.const_list_items(y.key_present);
			let common: SmallVec<[ConstId; 4]> = self
				.const_list_items(x.key_present)
				.iter()
				.copied()
				.filter(|key| theirs.contains(key))
				.collect();
			self.const_list(&common)
		};
		// a relation holds after the join only when both sides have it
		let below = if x.below == y.below { x.below } else { None };
		self.fact(Fact {
			narrowed,
			int,
			literal,
			length,
			key_present,
			below,
		})
	}

	/// Widens `next` (the join at a loop head) against `previous` (the previous
	/// head). `limit` is the range of the place's type, for integer places.
	pub fn widen(&mut self, previous: FactId, next: FactId, limit: Option<Interval>) -> FactId {
		if previous == next {
			return next;
		}
		let (old, mut new) = (self.fact_value(previous), self.fact_value(next));
		if new.int != IntervalSetId::NONE && old.int != IntervalSetId::NONE && new.int != old.int {
			let limit = limit.unwrap_or_else(Interval::full);
			let widened = self
				.interval_set_value(new.int)
				.widen(self.interval_set_value(old.int), limit);
			new.int = if widened.hull == limit && widened.excluded.is_empty() {
				IntervalSetId::NONE
			} else {
				self.interval_set(widened)
			};
		}
		if new.length != IntervalId::NONE
			&& old.length != IntervalId::NONE
			&& new.length != old.length
		{
			let (old_length, new_length) = (
				self.interval_value(old.length),
				self.interval_value(new.length),
			);
			let lo = if new_length.lo < old_length.lo {
				I256::ZERO
			} else {
				new_length.lo
			};
			let hi = if new_length.hi > old_length.hi {
				INFINITY
			} else {
				new_length.hi
			};
			new.length = if lo == I256::ZERO && hi == INFINITY {
				IntervalId::NONE
			} else {
				self.interval(Interval::new(lo, hi))
			};
		}
		self.fact(new)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn i(lo: i128, hi: i128) -> Interval {
		Interval::from_i128(lo, hi)
	}

	fn n(value: i128) -> I256 {
		I256::from(value)
	}

	#[test]
	fn excluded_ends_shrink_the_hull() {
		let set = IntervalSet::of(i(0, 10)).exclude(i(0, 0)).unwrap();
		assert_eq!(set.hull, i(1, 10));
		assert!(set.excluded.is_empty());
		let set = set.exclude(i(10, 10)).unwrap();
		assert_eq!(set.hull, i(1, 9));
		// a middle exclusion stays
		let set = set.exclude(i(5, 5)).unwrap();
		assert_eq!(set.hull, i(1, 9));
		assert_eq!(set.excluded.as_slice(), &[i(5, 5)]);
		assert!(!set.contains(n(5)) && set.contains(n(4)));
		// excluding then shrinking to the exclusion's edge
		let set = set.restrict(i(5, 9)).unwrap();
		assert_eq!(set.hull, i(6, 9));
		assert!(set.excluded.is_empty());
		// everything excluded
		assert_eq!(IntervalSet::of(i(3, 3)).exclude(i(3, 3)), None);
		assert_eq!(IntervalSet::of(i(0, 5)).exclude(i(-1, 9)), None);
		assert_eq!(IntervalSet::of(i(0, 5)).restrict(i(7, 9)), None);
	}

	#[test]
	fn at_most_eight_exclusions_oldest_dropped() {
		let mut set = IntervalSet::of(i(0, 100));
		for value in 1..=10 {
			set = set.exclude(i(value * 2, value * 2)).unwrap();
		}
		assert_eq!(set.excluded.len(), MAX_EXCLUDED);
		assert_eq!(set.excluded[0], i(6, 6));
		assert!(set.contains(n(2)) && set.contains(n(4)));
		assert!(!set.contains(n(20)));
	}

	#[test]
	fn excluding_zero_proves_a_divisor() {
		let set = IntervalSet::of(crate::types::IntStorage::Default.range())
			.exclude(i(0, 0))
			.unwrap();
		assert!(!set.contains(n(0)));
		assert!(set.contains(n(-1)) && set.contains(n(1)));
	}

	#[test]
	fn joins_take_hulls_and_common_exclusions() {
		let a = IntervalSet::of(i(0, 10)).exclude(i(5, 5)).unwrap();
		let b = IntervalSet::of(i(0, 10)).exclude(i(5, 6)).unwrap();
		let joined = a.join(&b);
		assert_eq!(joined.hull, i(0, 10));
		assert_eq!(joined.excluded.as_slice(), &[i(5, 5)]);
		// exclusions only one side has are lost
		let c = IntervalSet::of(i(0, 10));
		assert!(a.join(&c).excluded.is_empty());
		// only exclusions both sides have survive: 5 is lost with a far set
		let far = IntervalSet::of(i(20, 30));
		let joined = a.join(&far);
		assert_eq!(joined.hull, i(0, 30));
		assert!(joined.contains(n(5)) && joined.contains(n(15)));
	}

	#[test]
	fn widening_moves_changed_bounds_to_the_limit() {
		let int8 = crate::types::IntStorage::I8.range();
		let previous = IntervalSet::of(i(0, 0));
		let next = IntervalSet::of(i(0, 1));
		assert_eq!(next.widen(&previous, int8).hull, i(0, 127));
		let next = IntervalSet::of(i(-1, 0));
		assert_eq!(next.widen(&previous, int8).hull, i(-128, 0));
		let stable = IntervalSet::of(i(0, 5));
		assert_eq!(stable.widen(&stable, int8), stable);
		// unbounded type: ±∞
		let widened = IntervalSet::of(i(0, 1)).widen(&previous, Interval::full());
		assert!(widened.hull.is_unbounded_above());
		assert_eq!(widened.hull.lo, n(0));
	}

	#[test]
	fn facts_are_hash_consed() {
		let mut store = FactStore::new();
		assert_eq!(store.fact(Fact::TOP), FactId(0));
		let a = store.int_fact(IntervalSet::of(i(0, 5)));
		let b = store.int_fact(IntervalSet::of(i(0, 5)));
		let c = store.int_fact(IntervalSet::of(i(0, 6)));
		assert_eq!(a, b);
		assert_ne!(a, c);
		assert_eq!(
			store.interval_set_value(store.fact_value(a).int).hull,
			i(0, 5)
		);
		assert_eq!(
			store.const_list(&[ConstId(3), ConstId(1), ConstId(3)]),
			store.const_list(&[ConstId(1), ConstId(3)])
		);
	}

	#[test]
	fn fact_joins() {
		let mut types = TypeStore::new();
		let mut store = FactStore::new();
		let low = store.int_fact(IntervalSet::of(i(0, 5)));
		let high = store.int_fact(IntervalSet::of(i(10, 20)));
		let joined = store.join(low, high, &mut types);
		assert_eq!(
			store.interval_set_value(store.fact_value(joined).int).hull,
			i(0, 20)
		);
		// an unknown side makes the join unknown
		assert_eq!(store.join(low, FactId(0), &mut types), FactId(0));
		// literals are kept only when equal (values brief D-30)
		let one = types.constant(crate::types::ConstValue::Int(n(1)));
		let two = types.constant(crate::types::ConstValue::Int(n(2)));
		let exact_one = store.exact_int_fact(n(1), one);
		let exact_two = store.exact_int_fact(n(2), two);
		let joined = store.join(exact_one, exact_two, &mut types);
		let joined = store.fact_value(joined);
		assert_eq!(joined.literal, ConstId::NONE);
		assert_eq!(store.interval_set_value(joined.int).hull, i(1, 2));
		assert_eq!(store.join(exact_one, exact_one, &mut types), exact_one);
		// narrowings: union of the members
		let string = store.fact(Fact {
			narrowed: TypeId::STRING,
			..Fact::TOP
		});
		let none = store.fact(Fact {
			narrowed: TypeId::NONE,
			..Fact::TOP
		});
		let joined = store.join(string, none, &mut types);
		let joined = store.fact_value(joined);
		assert_eq!(joined.narrowed, types.optional(TypeId::STRING));
		// key proofs: intersection
		let ab = store.const_list(&[one, two]);
		let a = store.const_list(&[one]);
		let with_ab = store.fact(Fact {
			key_present: ab,
			..Fact::TOP
		});
		let with_a = store.fact(Fact {
			key_present: a,
			..Fact::TOP
		});
		let joined = store.join(with_ab, with_a, &mut types);
		assert_eq!(store.fact_value(joined).key_present, a);
	}

	#[test]
	fn fact_widening() {
		let mut store = FactStore::new();
		let int8 = crate::types::IntStorage::I8.range();
		let zero = store.int_fact(IntervalSet::of(i(0, 0)));
		let grown = store.int_fact(IntervalSet::of(i(0, 1)));
		let widened = store.widen(zero, grown, Some(int8));
		assert_eq!(
			store.interval_set_value(store.fact_value(widened).int).hull,
			i(0, 127)
		);
		// reaching the whole type: no fact
		let both = store.int_fact(IntervalSet::of(i(-1, 1)));
		assert_eq!(store.widen(zero, both, Some(int8)), FactId(0));
		// lengths widen to [0, ∞)
		let three = store.interval(i(3, 3));
		let four = store.interval(i(3, 4));
		let old = store.fact(Fact {
			length: three,
			..Fact::TOP
		});
		let new = store.fact(Fact {
			length: four,
			..Fact::TOP
		});
		let widened = store.widen(old, new, None);
		let widened = store.fact_value(widened);
		assert_eq!(
			store.interval_value(widened.length),
			Interval::new(n(3), INFINITY)
		);
	}
}
