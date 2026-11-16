//! The flow state carried along the walk (design §2.4).
//!
//! - [`Places`] is a trie of places: a root (a symbol, a module-level value, a
//!   temporary) and steps below it. Equal paths get equal [`PlaceId`]s.
//! - [`FlowState`] maps places to hash-consed facts in a sorted inline vector
//!   (decision A9): a fork is a copy of about 8 entries, a join a merge, the
//!   fixpoint test of a loop a slice compare.
//!
//! An absent place has no fact beyond its declared type.

pub mod facts;
pub mod map;
pub mod refs;

use smallvec::SmallVec;

use crate::ids::{DeclKey, FactId, NodeId, PlaceId, RefStateId, Sym, SymbolId, TypeId};
use crate::intern::Interner;
use crate::types::TypeStore;
use crate::types::int::Interval;

pub use facts::{Fact, FactStore, IntervalSet, MAX_EXCLUDED};
pub use map::FactMap;

/// A slot's incoming expression sources; None means an unknown value.
pub type CallableSources = SmallVec<[Option<NodeId>; 1]>;
pub type CallableBindings = SmallVec<[(PlaceId, CallableSources); 2]>;

/// Join evaluation-time callable facts, preserving unknown incoming paths.
pub fn join_callable_bindings(left: &mut CallableBindings, right: &CallableBindings) {
	for (place, sources) in right {
		if let Some((_, known)) = left.iter_mut().find(|(p, _)| p == place) {
			for source in sources {
				if !known.contains(source) {
					known.push(*source);
				}
			}
		} else {
			let mut sources = sources.clone();
			sources.push(None);
			left.push((*place, sources));
		}
	}
	for (place, sources) in left.iter_mut() {
		if !right.iter().any(|(p, _)| p == place) && !sources.contains(&None) {
			sources.push(None);
		}
		sources.sort_unstable();
		sources.dedup();
	}
	left.sort_unstable_by_key(|(place, _)| *place);
}

/// Where a place starts.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum PlaceRoot {
	/// A local value, a parameter, a loop variable.
	Symbol(SymbolId),
	/// A module-level value.
	ModuleValue(DeclKey),
	/// The value of an expression (a call result being narrowed, a `use` of a temporary).
	Temporary(NodeId),
}

/// What an index designates (advanced brief §5.4): taken when the place is
/// built, so that two places can be told apart (`distinct`).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Identity {
	/// A known value: a literal, or the exact fact of the index.
	Exact(i64),
	/// An immutable binding (`let`, parameter, range loop variable) plus an
	/// offset: its interval is its current fact plus the offset.
	Symbol(SymbolId, i32),
	/// `c.length + k` while the length of `c` hasn't changed.
	LengthOf(PlaceId, i32),
	/// A snapshot of the interval of a value that may change (a `mutable`).
	Interval(crate::ids::IntervalId),
	/// A reference returned by a call that returns fresh ones, at this time.
	Fresh(u32),
	/// An element that existed at this time (`find`): distinct from every
	/// fresh one returned later.
	Existing(u32),
	/// A literal key that is not an integer (`scores["ana"]`).
	Key(crate::ids::ConstId),
	/// Nothing known: any index.
	Unknown,
}

/// A step from a place to a place below it.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Step {
	/// `.field`.
	Field(Sym),
	/// `[index]`.
	Index(Identity),
	/// `[a..b]` or `[a..=b]` (the flag: end included).
	Range(Identity, Identity, bool),
	/// `[..]`: every element.
	All,
	/// `.length`.
	Length,
	/// `.size`.
	Size,
	/// `.capacity`: the room of a collection or a string, which only
	/// `reserve` (and the operations that grow it) changes.
	Capacity,
	/// The payload of a union member.
	Payload(TypeId),
	/// The characters of a string.
	Characters,
	/// A generic child (the element of an iteration).
	Child,
}

/// A node of the place trie.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Place {
	/// The root of the path.
	pub root: PlaceRoot,
	/// The place one step up (`None` for a root).
	pub parent: Option<PlaceId>,
	/// The step from the parent (`None` for a root).
	pub step: Option<Step>,
}

/// The per-file trie of places.
#[derive(Default)]
pub struct Places {
	places: Interner<Place>,
	/// Per place: whether a place below it exists (`drop_below` skips the scan otherwise).
	parents: Vec<bool>,
	/// The root place of each local symbol, by symbol index (a cache of [`Places::root`]).
	symbols: Vec<Option<PlaceId>>,
	/// Per place: the root of an immutable binding with a fact (see `FactStore::bind`).
	bound: Vec<bool>,
}

impl Places {
	/// An empty trie.
	pub fn new() -> Self {
		Self::default()
	}

	/// The place of a root.
	pub fn root(&mut self, root: PlaceRoot) -> PlaceId {
		let id = self.places.intern(Place {
			root,
			parent: None,
			step: None,
		});
		self.track(id);
		PlaceId(id)
	}

	/// The root place of a local symbol.
	pub fn symbol(&mut self, symbol: SymbolId) -> PlaceId {
		let index = symbol.index();
		if let Some(Some(place)) = self.symbols.get(index) {
			return *place;
		}
		let place = self.root(PlaceRoot::Symbol(symbol));
		if self.symbols.len() <= index {
			self.symbols.resize(index + 1, None);
		}
		self.symbols[index] = Some(place);
		place
	}

	/// The root place of a local symbol, when it was built.
	pub fn symbol_place(&self, symbol: SymbolId) -> Option<PlaceId> {
		self.symbols.get(symbol.index()).copied().flatten()
	}

	fn track(&mut self, id: u32) {
		if self.parents.len() <= id as usize {
			self.parents.resize(id as usize + 1, false);
			self.bound.resize(id as usize + 1, false);
		}
	}

	/// Marks a place as the root of an immutable binding with a fact.
	pub fn mark_bound(&mut self, place: PlaceId) {
		self.track(place.0);
		self.bound[place.0 as usize] = true;
	}

	/// Whether a place is the root of an immutable binding with a fact.
	pub fn is_bound(&self, place: PlaceId) -> bool {
		self.bound.get(place.0 as usize).copied().unwrap_or(false)
	}

	/// Whether a place below `place` was ever created.
	pub fn has_children(&self, place: PlaceId) -> bool {
		self.parents.get(place.0 as usize).copied().unwrap_or(false)
	}

	/// The place one `step` below `parent`.
	pub fn child(&mut self, parent: PlaceId, step: Step) -> PlaceId {
		let root = self.get(parent).root;
		let id = self.places.intern(Place {
			root,
			parent: Some(parent),
			step: Some(step),
		});
		self.track(id);
		self.track(parent.0);
		self.parents[parent.0 as usize] = true;
		PlaceId(id)
	}

	/// A place.
	pub fn get(&self, place: PlaceId) -> Place {
		*self.places.resolve(place.0)
	}

	/// The place one step up.
	pub fn parent(&self, place: PlaceId) -> Option<PlaceId> {
		self.get(place).parent
	}

	/// Whether `place` is `ancestor` or below it (one parent walk).
	pub fn is_at_or_below(&self, place: PlaceId, ancestor: PlaceId) -> bool {
		let mut current = Some(place);
		while let Some(id) = current {
			if id == ancestor {
				return true;
			}
			current = self.parent(id);
		}
		false
	}

	/// The number of steps from the root.
	pub fn depth(&self, place: PlaceId) -> usize {
		let mut depth = 0;
		let mut current = self.parent(place);
		while let Some(id) = current {
			depth += 1;
			current = self.parent(id);
		}
		depth
	}
}

/// The flow state at a program point.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FlowState {
	/// False after a `Never` expression: code here is unreachable.
	pub reachable: bool,
	/// Unreachable only because the facts rule this path out (a branch
	/// whose condition can't hold): not "unreachable code" (`W-DO-2`).
	pub vacuous: bool,
	/// Reference validity per symbol (M2, advanced brief §5.1).
	pub refs: SmallVec<[(SymbolId, RefStateId); 4]>,
	/// The places being iterated (M2, advanced brief §5.8).
	pub iterating: SmallVec<[PlaceId; 2]>,
	/// The freshness clock (M2).
	pub time: u32,
	/// The move-only locals consumed on some path (`subscription.stop()`,
	/// advanced brief §11.3), with the consuming node: a later use is `MEM002`.
	pub consumed: SmallVec<[(SymbolId, NodeId); 1]>,
	/// The fields moved out of an owned move-only local on some path (a
	/// task, a resource), with the moving node: the local can't be used as
	/// a whole, nor the field, until the field is assigned again (`MEM002`).
	pub moved_fields: SmallVec<[(PlaceId, NodeId); 1]>,
	/// Evaluation-time function-slot sources, joined with the other flow facts.
	pub callable_values: CallableBindings,
	/// The kept tasks that may still run (their `start` nodes): they
	/// borrow what they use until they end (`check/tasks.rs`).
	pub tasks: SmallVec<[NodeId; 1]>,
	/// The local tasks `cancel()` may have been called on (A11): awaiting
	/// one may fail with `Cancelled`.
	pub cancelled: SmallVec<[SymbolId; 1]>,
	/// Facts per place; an absent place has no fact (see [`FactMap`]).
	/// Last: the derived comparison looks at the cheaper fields first.
	pub facts: FactMap,
}

impl Default for FlowState {
	fn default() -> Self {
		Self::new()
	}
}

impl FlowState {
	/// A reachable state with no fact.
	pub fn new() -> Self {
		Self {
			reachable: true,
			vacuous: false,
			facts: FactMap::new(),
			refs: SmallVec::new(),
			iterating: SmallVec::new(),
			time: 0,
			consumed: SmallVec::new(),
			moved_fields: SmallVec::new(),
			callable_values: SmallVec::new(),
			tasks: SmallVec::new(),
			cancelled: SmallVec::new(),
		}
	}

	/// The state after a `Never` expression: the identity of joins.
	pub fn unreachable() -> Self {
		Self {
			reachable: false,
			..Self::new()
		}
	}

	/// Marks the state unreachable and forgets its facts.
	pub fn set_unreachable(&mut self) {
		*self = Self::unreachable();
	}

	/// A state no execution reaches because of the facts (a refinement
	/// that leaves no value): unreachable, but vacuously.
	pub fn impossible() -> Self {
		Self {
			reachable: false,
			vacuous: true,
			..Self::new()
		}
	}

	/// Marks the state impossible (see [`FlowState::impossible`]).
	pub fn set_impossible(&mut self) {
		*self = Self::impossible();
	}

	/// A copy for a branch: the lower levels of the facts are shared, and
	/// the changes made so far are frozen first (see [`FactMap::freeze`]) so
	/// that the branches compare and join on their own changes only.
	pub fn fork(&mut self) -> Self {
		self.facts.freeze();
		self.clone()
	}

	/// The fact of a place.
	pub fn fact(&self, place: PlaceId) -> Option<FactId> {
		self.facts.get(place)
	}

	/// Sets the fact of a place, without touching the places below it.
	/// [`Fact::TOP`] (id 0) removes the entry.
	pub fn set_fact(&mut self, place: PlaceId, fact: FactId) {
		self.facts.set(place, fact);
	}

	/// The state of a local reference.
	pub fn ref_state(&self, symbol: SymbolId) -> Option<RefStateId> {
		self.refs
			.binary_search_by_key(&symbol, |entry| entry.0)
			.ok()
			.map(|index| self.refs[index].1)
	}

	/// Sets the state of a local reference.
	pub fn set_ref_state(&mut self, symbol: SymbolId, state: RefStateId) {
		match self.refs.binary_search_by_key(&symbol, |entry| entry.0) {
			Ok(index) => self.refs[index].1 = state,
			Err(index) => self.refs.insert(index, (symbol, state)),
		}
	}

	/// Removes the fact of a place.
	pub fn remove_fact(&mut self, place: PlaceId) {
		self.set_fact(place, FactId(0));
	}

	/// An assignment to `place`: its facts are replaced by `fact`, and the
	/// facts below it are dropped.
	pub fn assign(&mut self, places: &Places, place: PlaceId, fact: FactId) {
		if places.has_children(place) {
			self.drop_below(places, place);
		}
		self.set_fact(place, fact);
	}

	/// Drops the facts of `place` and of every place below it.
	pub fn drop_below(&mut self, places: &Places, place: PlaceId) {
		if !places.has_children(place) {
			self.remove_fact(place);
			return;
		}
		self.facts.retain(|id, _| !places.is_at_or_below(id, place));
	}

	/// A `mutates place` effect (advanced brief §5.5): drops the facts at
	/// and below `place`, the lengths of the collections at or below it
	/// included. A write below a collection never changes its length: the
	/// places above keep their facts.
	pub fn mutate(&mut self, places: &Places, place: PlaceId) {
		self.drop_below(places, place);
	}

	/// The state where two branches meet. Unreachable states are the
	/// identity; otherwise only places known on both sides keep a fact, the
	/// pointwise join of the two. With a shared base, only the places either
	/// side changed are visited.
	pub fn join(
		&self,
		other: &FlowState,
		facts: &mut FactStore,
		types: &mut TypeStore,
	) -> FlowState {
		if !self.reachable && !other.reachable {
			let mut joined = FlowState::unreachable();
			joined.vacuous = self.vacuous || other.vacuous;
			return joined;
		}
		if !self.reachable {
			return other.clone();
		}
		if !other.reachable || self == other {
			return self.clone();
		}
		let mut joined = FlowState::new();
		match self.facts.changed_places(&other.facts) {
			Some(changed) => {
				joined.facts = self.facts.clone();
				for place in changed {
					let fact = match (self.fact(place), other.fact(place)) {
						(Some(a), Some(b)) => facts.join(a, b, types),
						_ => FactId(0),
					};
					joined.facts.set(place, fact);
				}
			}
			None => {
				let mut entries = Vec::new();
				let (mut left, mut right) =
					(self.facts.iter().peekable(), other.facts.iter().peekable());
				while let (Some(&a), Some(&b)) = (left.peek(), right.peek()) {
					match a.0.cmp(&b.0) {
						std::cmp::Ordering::Less => {
							left.next();
						}
						std::cmp::Ordering::Greater => {
							right.next();
						}
						std::cmp::Ordering::Equal => {
							let fact = facts.join(a.1, b.1, types);
							if fact != FactId(0) {
								entries.push((a.0, fact));
							}
							left.next();
							right.next();
						}
					}
				}
				joined.facts = FactMap::from_sorted(entries);
			}
		}
		joined.refs = refs::join_refs(&mut facts.refs, &self.refs, &other.refs);
		joined.iterating = self
			.iterating
			.iter()
			.copied()
			.filter(|place| other.iterating.contains(place))
			.collect();
		joined.time = self.time.max(other.time);
		joined.consumed = self.consumed.clone();
		for &(symbol, node) in &other.consumed {
			if !joined.consumed.iter().any(|&(other, _)| other == symbol) {
				joined.consumed.push((symbol, node));
			}
		}
		joined.moved_fields = self.moved_fields.clone();
		for &(place, node) in &other.moved_fields {
			if !joined.moved_fields.iter().any(|&(other, _)| other == place) {
				joined.moved_fields.push((place, node));
			}
		}
		joined.tasks = self.tasks.clone();
		joined.callable_values = self.callable_values.clone();
		join_callable_bindings(&mut joined.callable_values, &other.callable_values);
		for &task in &other.tasks {
			if !joined.tasks.contains(&task) {
				joined.tasks.push(task);
			}
		}
		joined.cancelled = self.cancelled.clone();
		for &task in &other.cancelled {
			if !joined.cancelled.contains(&task) {
				joined.cancelled.push(task);
			}
		}
		joined
	}

	/// The new head of a loop: `next` (the join of the head and the ends of
	/// the body) widened against `self`, the previous head (values brief
	/// D-96). `limit` gives the range of an integer place's type.
	pub fn widen(
		&self,
		next: &FlowState,
		facts: &mut FactStore,
		limit: impl Fn(PlaceId) -> Option<Interval>,
	) -> FlowState {
		if !self.reachable || !next.reachable || self == next {
			return next.clone();
		}
		let mut widened = next.clone();
		let places: SmallVec<[PlaceId; 16]> = match self.facts.changed_places(&next.facts) {
			Some(changed) => changed,
			None => next.facts.iter().map(|(place, _)| place).collect(),
		};
		for place in places {
			if let (Some(previous), Some(current)) = (self.fact(place), next.fact(place))
				&& previous != current
			{
				let fact = facts.widen(previous, current, limit(place));
				widened.facts.set(place, fact);
			}
		}
		widened
	}

	/// The hard cap of the fixpoint (design §3.3, round 8): keeps only the
	/// facts that didn't change between `self` and `next`.
	pub fn forget_changed(&self, next: &FlowState) -> FlowState {
		let mut kept = next.clone();
		kept.facts
			.retain(|place, fact| self.fact(place) == Some(fact));
		// reference states are finite: they reach their fixpoint unwidened
		kept
	}
}

#[cfg(test)]
mod tests {
	use ethnum::I256;

	use super::*;
	use crate::types::IntStorage;

	fn i(lo: i128, hi: i128) -> Interval {
		Interval::from_i128(lo, hi)
	}

	struct Fixture {
		places: Places,
		facts: FactStore,
		types: TypeStore,
	}

	impl Fixture {
		fn new() -> Self {
			Self {
				places: Places::new(),
				facts: FactStore::new(),
				types: TypeStore::new(),
			}
		}

		fn symbol(&mut self, index: u32) -> PlaceId {
			self.places.root(PlaceRoot::Symbol(SymbolId(index)))
		}

		fn field(&mut self, parent: PlaceId, name: &str) -> PlaceId {
			let sym = self.types.syms.intern(name);
			self.places.child(parent, Step::Field(sym))
		}

		fn int(&mut self, lo: i128, hi: i128) -> FactId {
			self.facts.int_fact(IntervalSet::of(i(lo, hi)))
		}

		fn hull(&self, state: &FlowState, place: PlaceId) -> Option<Interval> {
			let fact = self.facts.fact_value(state.fact(place)?);
			Some(self.facts.interval_set_value(fact.int).hull)
		}
	}

	#[test]
	fn places_form_a_trie() {
		let mut fx = Fixture::new();
		let a = fx.symbol(0);
		let ab = fx.field(a, "b");
		let abc = fx.field(ab, "c");
		assert_eq!(fx.field(a, "b"), ab);
		assert_eq!(fx.symbol(0), a);
		assert_ne!(fx.symbol(1), a);
		assert!(fx.places.is_at_or_below(abc, a));
		assert!(fx.places.is_at_or_below(ab, ab));
		assert!(!fx.places.is_at_or_below(a, ab));
		assert_eq!(fx.places.depth(abc), 2);
		assert_eq!(fx.places.get(abc).root, PlaceRoot::Symbol(SymbolId(0)));
		assert_eq!(fx.places.parent(abc), Some(ab));
	}

	#[test]
	fn facts_are_set_sorted_and_removed() {
		let mut fx = Fixture::new();
		let (a, b, c) = (fx.symbol(0), fx.symbol(1), fx.symbol(2));
		let five = fx.int(5, 5);
		let mut state = FlowState::new();
		state.set_fact(c, five);
		state.set_fact(a, five);
		state.set_fact(b, five);
		let entries: Vec<_> = state.facts.iter().collect();
		assert!(entries.windows(2).all(|pair| pair[0].0 < pair[1].0));
		assert_eq!(state.fact(b), Some(five));
		state.remove_fact(b);
		assert_eq!(state.fact(b), None);
		state.set_fact(a, FactId(0));
		assert_eq!(state.fact(a), None);
		assert_eq!(state.facts.len(), 1);
	}

	#[test]
	fn assignment_and_drop_below() {
		let mut fx = Fixture::new();
		let a = fx.symbol(0);
		let ab = fx.field(a, "b");
		let abc = fx.field(ab, "c");
		let other = fx.symbol(1);
		let fact = fx.int(1, 2);
		let mut state = FlowState::new();
		for place in [a, ab, abc, other] {
			state.set_fact(place, fact);
		}
		state.drop_below(&fx.places, ab);
		assert_eq!(state.fact(a), Some(fact));
		assert_eq!(state.fact(ab), None);
		assert_eq!(state.fact(abc), None);
		assert_eq!(state.fact(other), Some(fact));
		let new = fx.int(7, 7);
		state.set_fact(abc, fact);
		state.assign(&fx.places, a, new);
		assert_eq!(state.fact(a), Some(new));
		assert_eq!(state.fact(abc), None);
	}

	#[test]
	fn mutation_drops_facts_below_only() {
		let mut fx = Fixture::new();
		let list = fx.symbol(0);
		let element = fx.places.child(list, Step::Index(Identity::Exact(0)));
		let name = fx.field(element, "name");
		let length_place = fx.places.child(list, Step::Length);
		let three = fx.facts.interval(i(3, 3));
		let with_length = fx.facts.fact(Fact {
			length: three,
			..Fact::TOP
		});
		let int = fx.int(0, 1);
		let mut state = FlowState::new();
		state.set_fact(list, with_length);
		state.set_fact(name, int);
		state.set_fact(length_place, int);
		state.mutate(&fx.places, element);
		assert_eq!(state.fact(name), None);
		assert_eq!(
			state.fact(list),
			Some(with_length),
			"an element keeps the length"
		);
		assert_eq!(state.fact(length_place), Some(int));
		state.mutate(&fx.places, list);
		assert_eq!(state.fact(list), None);
		assert_eq!(state.fact(length_place), None);
	}

	#[test]
	fn join_keeps_common_places() {
		let mut fx = Fixture::new();
		let (a, b) = (fx.symbol(0), fx.symbol(1));
		let low = fx.int(0, 5);
		let high = fx.int(10, 20);
		let mut left = FlowState::new();
		left.set_fact(a, low);
		left.set_fact(b, low);
		let mut right = FlowState::new();
		right.set_fact(a, high);
		let joined = left.join(&right, &mut fx.facts, &mut fx.types);
		assert_eq!(fx.hull(&joined, a), Some(i(0, 20)));
		assert_eq!(joined.fact(b), None);
		assert!(joined.reachable);
	}

	#[test]
	fn unreachable_states_are_the_identity_of_join() {
		let mut fx = Fixture::new();
		let a = fx.symbol(0);
		let fact = fx.int(1, 1);
		let mut state = FlowState::new();
		state.set_fact(a, fact);
		let dead = FlowState::unreachable();
		assert_eq!(state.join(&dead, &mut fx.facts, &mut fx.types), state);
		assert_eq!(dead.join(&state, &mut fx.facts, &mut fx.types), state);
		assert!(!dead.join(&dead, &mut fx.facts, &mut fx.types).reachable);
		let mut killed = state.fork();
		killed.set_unreachable();
		assert!(!killed.reachable && killed.facts.is_empty());
	}

	/// `mutable count = 0` then a loop body `count += 1`: the head reaches `[0, max]`.
	#[test]
	fn loop_fixpoint_with_widening() {
		let mut fx = Fixture::new();
		let count = fx.symbol(0);
		let limit = IntStorage::Default.range();
		let zero = fx.int(0, 0);
		let mut head = FlowState::new();
		head.set_fact(count, zero);
		let mut rounds = 0;
		loop {
			rounds += 1;
			// body: count += 1
			let mut end = head.fork();
			let current = fx.hull(&end, count).unwrap_or(limit);
			let next = (current + i(1, 1)).intersect(limit).unwrap();
			let fact = fx.facts.int_fact(IntervalSet::of(next));
			end.assign(&fx.places, count, fact);
			let joined = head.join(&end, &mut fx.facts, &mut fx.types);
			let widened = head.widen(&joined, &mut fx.facts, |_| Some(limit));
			if widened == head {
				break;
			}
			head = widened;
			assert!(rounds < 8, "no fixpoint");
		}
		assert_eq!(
			fx.hull(&head, count),
			Some(Interval::new(I256::ZERO, limit.hi))
		);
		assert!(rounds <= 3);
	}

	#[test]
	fn widening_leaves_stable_facts() {
		let mut fx = Fixture::new();
		let (a, b) = (fx.symbol(0), fx.symbol(1));
		let stable = fx.int(1, 4);
		let start = fx.int(0, 0);
		let grown = fx.int(-1, 0);
		let mut previous = FlowState::new();
		previous.set_fact(a, stable);
		previous.set_fact(b, start);
		let mut next = previous.clone();
		next.set_fact(b, grown);
		let widened = previous.widen(&next, &mut fx.facts, |_| Some(IntStorage::I8.range()));
		assert_eq!(widened.fact(a), Some(stable));
		assert_eq!(fx.hull(&widened, b), Some(i(-128, 0)));
		let capped = previous.forget_changed(&next);
		assert_eq!(capped.fact(a), Some(stable));
		assert_eq!(capped.fact(b), None);
	}
}
