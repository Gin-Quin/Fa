//! Evaluated value provenance, shared by escapes and reference results.
use crate::flow::Step;
use crate::ids::{NodeId, PlaceId, SymbolId};
use crate::intern::Interner;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClosureOrigin {
	pub projection: Vec<Step>,
	pub lambda: NodeId,
}

/// An unknown returned place is somewhere below this owner. A formal root
/// preserves the identity of an explicitly borrowed parameter inside its body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReferenceBound {
	pub place: PlaceId,
	pub formal: Option<SymbolId>,
}

/// Empty is unreachable/absent; `owned` preserves a non-borrowed alternative
/// when joining it with a borrowed branch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ValueOrigins {
	pub places: Vec<PlaceId>,
	pub bounds: Vec<ReferenceBound>,
	pub readonly: bool,
	pub owned: bool,
	/// A user-call result whose physical projection is not known here.
	pub opaque: bool,
	pub closures: Vec<ClosureOrigin>,
}

impl ValueOrigins {
	pub fn join(&mut self, other: &Self) {
		self.owned |= other.owned;
		self.opaque |= other.opaque;
		self.readonly |= other.readonly;
		self.bounds.extend_from_slice(&other.bounds);
		self.bounds.sort_unstable();
		self.bounds.dedup();
		self.places.extend_from_slice(&other.places);
		self.places.sort_unstable();
		self.places.dedup();
		self.closures.extend_from_slice(&other.closures);
		self.closures.sort_unstable();
		self.closures.dedup();
	}
	pub fn copied(&self) -> Self {
		Self {
			places: Vec::new(),
			bounds: Vec::new(),
			readonly: false,
			owned: true,
			opaque: false,
			closures: self.closures.clone(),
		}
	}
}

#[derive(Default)]
pub struct OriginStore(Interner<ValueOrigins>);
impl OriginStore {
	pub fn intern(&mut self, value: ValueOrigins) -> u32 {
		self.0.intern(value)
	}
	pub fn get(&self, value: u32) -> &ValueOrigins {
		self.0.resolve(value)
	}
	pub fn join(&mut self, left: u32, right: u32) -> u32 {
		if left == right {
			return left;
		}
		let mut value = self.get(left).clone();
		value.join(self.get(right));
		self.intern(value)
	}
}
