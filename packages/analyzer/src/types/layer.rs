//! The tables of a view of the type store (design/parallel-pipeline.md
//! §1.3): a global table shared by every view ([`crate::types::global`]),
//! and a local one owned by this view.
//!
//! Interning looks in the local index first, then in the global table: a
//! global hit is recorded in the local index (the global index is asked at
//! most once per structure and view), a miss is interned locally, its id
//! marked with [`LOCAL`]. A structure has one id in a view: the local index
//! is asked first and what it records is final, and every global id the
//! view holds was global when it got it.
//!
//! The local ids die with the view; [`crate::types::merge`] copies what the
//! interface of a finished unit reaches into the global tables.

use std::hash::Hash;
use std::sync::Arc;

use hashbrown::HashTable;

use crate::ids::Sym;
use crate::types::global::{GlobalSlices, GlobalValues, LOCAL, hash_of, is_local};

/// Values interned in a view.
pub struct ValueLayer<T> {
	pub(crate) global: Arc<GlobalValues<T>>,
	local: Vec<T>,
	/// Local and global ids, by hash of their value.
	index: HashTable<u32>,
}

impl<T: Hash + Eq + Clone> ValueLayer<T> {
	/// A view of a global table, with no local value.
	pub fn new(global: Arc<GlobalValues<T>>) -> Self {
		Self {
			global,
			local: Vec::new(),
			index: HashTable::new(),
		}
	}

	/// The value of an id.
	#[inline]
	pub fn resolve(&self, id: u32) -> &T {
		if is_local(id) {
			&self.local[(id & !LOCAL) as usize]
		} else {
			self.global.get(id)
		}
	}

	/// The value of a local id, to update it (late-bound data of a local
	/// structure: never part of its identity).
	pub fn local_mut(&mut self, id: u32) -> Option<&mut T> {
		is_local(id).then(|| &mut self.local[(id & !LOCAL) as usize])
	}

	/// The id of a value, and whether it was new (interned locally).
	pub fn intern_new(&mut self, value: T) -> (u32, bool) {
		let hash = hash_of(&value);
		if let Some(id) = self.find_hashed(hash, &value) {
			return (id, false);
		}
		let id = self.local.len() as u32 | LOCAL;
		assert!(self.local.len() < LOCAL as usize, "local store overflow");
		self.local.push(value);
		self.insert_index(hash, id);
		(id, true)
	}

	/// The id of a value.
	#[inline]
	pub fn intern(&mut self, value: T) -> u32 {
		self.intern_new(value).0
	}

	/// The id of a value if it is interned (in the view or globally); a
	/// global hit is recorded in the view.
	pub fn find(&mut self, value: &T) -> Option<u32> {
		self.find_hashed(hash_of(value), value)
	}

	/// The id of a value if it is interned (in the view or globally).
	pub fn get(&self, value: &T) -> Option<u32> {
		let hash = hash_of(value);
		self.index
			.find(hash, |&id| self.resolve(id) == value)
			.copied()
			.or_else(|| self.global.find(hash, value))
	}

	/// Finds a value: the local index, then the global table (a hit is
	/// recorded locally).
	fn find_hashed(&mut self, hash: u64, value: &T) -> Option<u32> {
		let (local, global) = (&self.local, &self.global);
		let resolve = |id: u32| {
			if is_local(id) {
				&local[(id & !LOCAL) as usize]
			} else {
				global.get(id)
			}
		};
		if let Some(&id) = self.index.find(hash, |&id| resolve(id) == value) {
			return Some(id);
		}
		let id = self.global.find(hash, value)?;
		self.insert_index(hash, id);
		Some(id)
	}

	fn insert_index(&mut self, hash: u64, id: u32) {
		let (local, global) = (&self.local, &self.global);
		self.index.insert_unique(hash, id, |&other| {
			if is_local(other) {
				hash_of(&local[(other & !LOCAL) as usize])
			} else {
				hash_of(global.get(other))
			}
		});
	}

	/// The number of local values.
	pub fn local_len(&self) -> usize {
		self.local.len()
	}

	/// The number of values, global ones included.
	pub fn len(&self) -> usize {
		self.global.len() + self.local.len()
	}

	/// Always false: a table has its predefined values.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

/// Lists interned in a view.
pub struct SliceLayer<T> {
	pub(crate) global: Arc<GlobalSlices<T>>,
	items: Vec<T>,
	ranges: Vec<(u32, u32)>,
	index: HashTable<u32>,
}

impl<T: Hash + Eq + Clone> SliceLayer<T> {
	/// A view of a global table, with no local list.
	pub fn new(global: Arc<GlobalSlices<T>>) -> Self {
		Self {
			global,
			items: Vec::new(),
			ranges: Vec::new(),
			index: HashTable::new(),
		}
	}

	/// The list of an id.
	#[inline]
	pub fn resolve(&self, id: u32) -> &[T] {
		if is_local(id) {
			let (start, end) = self.ranges[(id & !LOCAL) as usize];
			&self.items[start as usize..end as usize]
		} else {
			self.global.get(id)
		}
	}

	/// The id of a list.
	pub fn intern(&mut self, list: &[T]) -> u32 {
		let hash = hash_of(list);
		let (items, ranges, global) = (&self.items, &self.ranges, &self.global);
		let resolve = |id: u32| -> &[T] {
			if is_local(id) {
				let (start, end) = ranges[(id & !LOCAL) as usize];
				&items[start as usize..end as usize]
			} else {
				global.get(id)
			}
		};
		if let Some(&id) = self.index.find(hash, |&id| resolve(id) == list) {
			return id;
		}
		let id = match self.global.find(hash, list) {
			Some(id) => id,
			None => {
				let start = self.items.len() as u32;
				self.items.extend_from_slice(list);
				let id = self.ranges.len() as u32 | LOCAL;
				assert!(self.ranges.len() < LOCAL as usize, "local store overflow");
				self.ranges.push((start, self.items.len() as u32));
				id
			}
		};
		let (items, ranges, global) = (&self.items, &self.ranges, &self.global);
		self.index.insert_unique(hash, id, |&other| {
			if is_local(other) {
				let (start, end) = ranges[(other & !LOCAL) as usize];
				hash_of(&items[start as usize..end as usize])
			} else {
				hash_of(global.get(other))
			}
		});
		id
	}

	/// The number of local lists.
	pub fn local_len(&self) -> usize {
		self.ranges.len()
	}
}

/// Identifiers and strings interned in a view.
pub struct Syms {
	pub(crate) global: Arc<GlobalSlices<u8>>,
	bytes: String,
	spans: Vec<(u32, u32)>,
	index: HashTable<u32>,
}

impl Syms {
	/// A view of the global symbols.
	pub fn new(global: Arc<GlobalSlices<u8>>) -> Self {
		Self {
			global,
			bytes: String::new(),
			spans: Vec::new(),
			index: HashTable::new(),
		}
	}

	/// The text of a symbol.
	#[inline]
	pub fn resolve(&self, sym: Sym) -> &str {
		if is_local(sym.0) {
			let (start, end) = self.spans[(sym.0 & !LOCAL) as usize];
			&self.bytes[start as usize..end as usize]
		} else {
			// SAFETY: global symbols are interned from `&str`s only
			unsafe { std::str::from_utf8_unchecked(self.global.get(sym.0)) }
		}
	}

	/// The symbol of `text`, interned on first use.
	pub fn intern(&mut self, text: &str) -> Sym {
		let hash = hash_of(text.as_bytes());
		if let Some(sym) = self.find_hashed(hash, text) {
			return sym;
		}
		let start = self.bytes.len() as u32;
		self.bytes.push_str(text);
		let id = self.spans.len() as u32 | LOCAL;
		assert!(self.spans.len() < LOCAL as usize, "local store overflow");
		self.spans.push((start, self.bytes.len() as u32));
		self.insert_index(hash, id);
		Sym(id)
	}

	/// The symbol of `text` if it was interned.
	pub fn get(&self, text: &str) -> Option<Sym> {
		let hash = hash_of(text.as_bytes());
		self.index
			.find(hash, |&id| self.resolve(Sym(id)) == text)
			.map(|&id| Sym(id))
			.or_else(|| self.global.find(hash, text.as_bytes()).map(Sym))
	}

	fn find_hashed(&mut self, hash: u64, text: &str) -> Option<Sym> {
		if let Some(&id) = self.index.find(hash, |&id| self.resolve(Sym(id)) == text) {
			return Some(Sym(id));
		}
		let id = self.global.find(hash, text.as_bytes())?;
		self.insert_index(hash, id);
		Some(Sym(id))
	}

	fn insert_index(&mut self, hash: u64, id: u32) {
		let (bytes, spans, global) = (&self.bytes, &self.spans, &self.global);
		self.index.insert_unique(hash, id, |&other| {
			if is_local(other) {
				let (start, end) = spans[(other & !LOCAL) as usize];
				hash_of(&bytes.as_bytes()[start as usize..end as usize])
			} else {
				hash_of(global.get(other))
			}
		});
	}

	/// The number of symbols of the view, global ones included.
	pub fn len(&self) -> usize {
		self.global.len() + self.spans.len()
	}

	/// Whether no symbol was interned.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

/// Records with an id, created in a view (type parameters, recursive
/// references): global ones are read-only, local ones can be updated.
pub struct RecordLayer<T> {
	pub(crate) global: Arc<crate::types::global::AppendVec<std::sync::OnceLock<T>>>,
	local: Vec<T>,
}

impl<T> RecordLayer<T> {
	/// A view of a global table.
	pub fn new(global: Arc<crate::types::global::AppendVec<std::sync::OnceLock<T>>>) -> Self {
		Self {
			global,
			local: Vec::new(),
		}
	}

	/// The record of an id.
	#[inline]
	pub fn get(&self, id: u32) -> &T {
		if is_local(id) {
			&self.local[(id & !LOCAL) as usize]
		} else {
			self.global
				.get(id as usize)
				.get()
				.expect("a global record is set before it is published")
		}
	}

	/// The record of a local id, to update it; `None` for a global one.
	pub fn local_mut(&mut self, id: u32) -> Option<&mut T> {
		is_local(id).then(|| &mut self.local[(id & !LOCAL) as usize])
	}

	/// Adds a local record; returns its id.
	pub fn push(&mut self, record: T) -> u32 {
		assert!(self.local.len() < LOCAL as usize, "local store overflow");
		self.local.push(record);
		(self.local.len() - 1) as u32 | LOCAL
	}

	/// The number of local records.
	pub fn local_len(&self) -> usize {
		self.local.len()
	}
}
