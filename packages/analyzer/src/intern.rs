//! Hash-consing.
//!
//! - [`SymInterner`] interns identifiers and strings into [`Sym`]s. It owns
//!   its bytes: it never relies on the parser's `&'static str` input.
//! - [`Interner`] hash-conses fixed-size values (integer types, shapes,
//!   signatures, facts…) into `u32` ids.
//! - [`SliceInterner`] hash-conses variable-length lists into `u32` ids, all
//!   stored in one flat `Vec`.
//!
//! None of them stores a value twice: the index maps a hash to the ids that
//! have it, and equality is checked against the stored value.

use std::hash::{BuildHasher, Hash};

use rustc_hash::{FxBuildHasher, FxHashMap};
use smallvec::SmallVec;

use crate::ids::Sym;

/// Maps a 64-bit hash to the ids of the stored values that have it.
#[derive(Default, Clone, Debug)]
struct HashIndex {
	buckets: FxHashMap<u64, SmallVec<[u32; 1]>>,
}

impl HashIndex {
	fn find(&self, hash: u64, mut equal: impl FnMut(u32) -> bool) -> Option<u32> {
		self.buckets
			.get(&hash)?
			.iter()
			.copied()
			.find(|&id| equal(id))
	}

	fn insert(&mut self, hash: u64, id: u32) {
		self.buckets.entry(hash).or_default().push(id);
	}
}

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
	FxBuildHasher.hash_one(value)
}

fn to_u32(index: usize) -> u32 {
	u32::try_from(index).expect("interner overflow")
}

/// Interns identifiers and strings.
#[derive(Default, Clone, Debug)]
pub struct SymInterner {
	bytes: String,
	spans: Vec<(u32, u32)>,
	index: HashIndex,
}

impl SymInterner {
	/// An empty interner.
	pub fn new() -> Self {
		Self::default()
	}

	/// The symbol of `text`, interned on first use.
	pub fn intern(&mut self, text: &str) -> Sym {
		let hash = hash_of(text);
		let (bytes, spans) = (&self.bytes, &self.spans);
		let found = self.index.find(hash, |id| {
			let (start, end) = spans[id as usize];
			&bytes[start as usize..end as usize] == text
		});
		if let Some(id) = found {
			return Sym(id);
		}
		let start = to_u32(self.bytes.len());
		self.bytes.push_str(text);
		let id = to_u32(self.spans.len());
		self.spans.push((start, to_u32(self.bytes.len())));
		self.index.insert(hash, id);
		Sym(id)
	}

	/// The symbol of `text` if it was interned.
	pub fn get(&self, text: &str) -> Option<Sym> {
		self.index
			.find(hash_of(text), |id| self.resolve(Sym(id)) == text)
			.map(Sym)
	}

	/// The text of a symbol.
	pub fn resolve(&self, sym: Sym) -> &str {
		let (start, end) = self.spans[sym.index()];
		&self.bytes[start as usize..end as usize]
	}

	/// The number of interned symbols.
	pub fn len(&self) -> usize {
		self.spans.len()
	}

	/// Whether no symbol was interned.
	pub fn is_empty(&self) -> bool {
		self.spans.is_empty()
	}
}

/// Hash-conses fixed-size values: equal values get equal ids.
#[derive(Clone, Debug)]
pub struct Interner<T> {
	items: Vec<T>,
	index: HashIndex,
}

impl<T> Default for Interner<T> {
	fn default() -> Self {
		Self {
			items: Vec::new(),
			index: HashIndex::default(),
		}
	}
}

impl<T: Hash + Eq> Interner<T> {
	/// An empty interner.
	pub fn new() -> Self {
		Self::default()
	}

	/// The id of `value`, and whether it was new.
	pub fn intern_new(&mut self, value: T) -> (u32, bool) {
		let hash = hash_of(&value);
		let items = &self.items;
		if let Some(id) = self.index.find(hash, |id| items[id as usize] == value) {
			return (id, false);
		}
		let id = to_u32(self.items.len());
		self.items.push(value);
		self.index.insert(hash, id);
		(id, true)
	}

	/// The id of `value`.
	pub fn intern(&mut self, value: T) -> u32 {
		self.intern_new(value).0
	}

	/// The id of `value` if it was interned.
	pub fn get(&self, value: &T) -> Option<u32> {
		self.index
			.find(hash_of(value), |id| self.items[id as usize] == *value)
	}

	/// The value of an id.
	#[inline]
	pub fn resolve(&self, id: u32) -> &T {
		&self.items[id as usize]
	}

	/// The number of interned values.
	pub fn len(&self) -> usize {
		self.items.len()
	}

	/// Whether no value was interned.
	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}
}

/// Hash-conses lists into one flat `Vec`: equal lists get equal ids.
#[derive(Clone, Debug)]
pub struct SliceInterner<T> {
	items: Vec<T>,
	ranges: Vec<(u32, u32)>,
	index: HashIndex,
}

impl<T> Default for SliceInterner<T> {
	fn default() -> Self {
		Self {
			items: Vec::new(),
			ranges: Vec::new(),
			index: HashIndex::default(),
		}
	}
}

impl<T: Hash + Eq + Clone> SliceInterner<T> {
	/// An interner whose id 0 is the empty list.
	pub fn new() -> Self {
		let mut interner = Self::default();
		interner.intern(&[]);
		interner
	}

	/// The id of `list`.
	pub fn intern(&mut self, list: &[T]) -> u32 {
		let hash = hash_of(list);
		let (items, ranges) = (&self.items, &self.ranges);
		let found = self.index.find(hash, |id| {
			let (start, end) = ranges[id as usize];
			&items[start as usize..end as usize] == list
		});
		if let Some(id) = found {
			return id;
		}
		let start = to_u32(self.items.len());
		self.items.extend_from_slice(list);
		let id = to_u32(self.ranges.len());
		self.ranges.push((start, to_u32(self.items.len())));
		self.index.insert(hash, id);
		id
	}

	/// The list of an id.
	#[inline]
	pub fn resolve(&self, id: u32) -> &[T] {
		let (start, end) = self.ranges[id as usize];
		&self.items[start as usize..end as usize]
	}

	/// The number of interned lists.
	pub fn len(&self) -> usize {
		self.ranges.len()
	}

	/// Whether no list was interned (never true after [`SliceInterner::new`]).
	pub fn is_empty(&self) -> bool {
		self.ranges.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn symbols_are_interned_once() {
		let mut symbols = SymInterner::new();
		let a = symbols.intern("count");
		let b = symbols.intern("total");
		assert_ne!(a, b);
		assert_eq!(symbols.intern("count"), a);
		assert_eq!(symbols.resolve(a), "count");
		assert_eq!(symbols.resolve(b), "total");
		assert_eq!(symbols.get("total"), Some(b));
		assert_eq!(symbols.get("missing"), None);
		assert_eq!(symbols.len(), 2);
	}

	#[test]
	fn empty_and_unicode_symbols() {
		let mut symbols = SymInterner::new();
		let empty = symbols.intern("");
		let unicode = symbols.intern("été");
		assert_eq!(symbols.resolve(empty), "");
		assert_eq!(symbols.resolve(unicode), "été");
		assert_eq!(symbols.intern(""), empty);
	}

	#[test]
	fn values_are_hash_consed() {
		let mut interner = Interner::new();
		let a = interner.intern((1u8, 2u32));
		let b = interner.intern((3u8, 4u32));
		assert_eq!(interner.intern((1u8, 2u32)), a);
		assert_eq!(interner.intern_new((3u8, 4u32)), (b, false));
		assert_eq!(*interner.resolve(b), (3, 4));
		assert_eq!(interner.get(&(9, 9)), None);
	}

	#[test]
	fn slices_are_hash_consed() {
		let mut lists = SliceInterner::<u32>::new();
		assert_eq!(lists.intern(&[]), 0);
		let a = lists.intern(&[1, 2, 3]);
		let b = lists.intern(&[1, 2]);
		assert_ne!(a, b);
		assert_eq!(lists.intern(&[1, 2, 3]), a);
		assert_eq!(lists.resolve(a), &[1, 2, 3]);
		assert_eq!(lists.resolve(b), &[1, 2]);
		assert_eq!(lists.resolve(0), &[] as &[u32]);
	}
}
