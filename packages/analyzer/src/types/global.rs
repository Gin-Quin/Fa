//! The global layer of the type store (design/parallel-pipeline.md §1.2,
//! P1–P2): what every analysis unit shares, append-only and `Sync`.
//!
//! - [`AppendVec`]: a segmented vector (32 buckets of growing sizes, never
//!   reallocated), whose reads take no lock and return references that live
//!   as long as the vector.
//! - [`GlobalValues`] / [`GlobalSlices`]: hash-consed values and lists in an
//!   [`AppendVec`], with an index under a read-write lock: units look values
//!   up concurrently; only the merge of a finished unit inserts (one merge
//!   at a time, [`crate::types::merge`]).
//!
//! A store id whose bit 31 is set is *local* to one view of the store (one
//! unit, `store.rs`); the global tables never hold one.

use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use hashbrown::HashTable;
use rustc_hash::FxBuildHasher;

/// The bit of a store id that marks it local to a view.
pub const LOCAL: u32 = 1 << 31;

/// Whether a store id is local to a view.
#[inline]
pub fn is_local(id: u32) -> bool {
	id & LOCAL != 0
}

/// The hash the store indexes values by.
#[inline]
pub fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
	FxBuildHasher.hash_one(value)
}

/// The size of the first bucket of an [`AppendVec`] (a power of two).
const FIRST_BITS: u32 = 5;
/// The number of buckets: room for about 2^36 items.
const BUCKETS: usize = 32;

/// A vector that only grows, whose items never move: `get` takes no lock.
/// Pushes are serialized by an internal lock.
pub struct AppendVec<T> {
	buckets: [AtomicPtr<T>; BUCKETS],
	len: AtomicUsize,
	push: Mutex<()>,
	_items: PhantomData<T>,
}

// SAFETY: the items are only shared by reference (`get`), and moved in by
// `push` under the push lock; a `T: Send + Sync` can be shared that way.
unsafe impl<T: Send + Sync> Sync for AppendVec<T> {}
// SAFETY: the vector owns its items.
unsafe impl<T: Send> Send for AppendVec<T> {}

impl<T> Default for AppendVec<T> {
	fn default() -> Self {
		Self {
			buckets: std::array::from_fn(|_| AtomicPtr::new(std::ptr::null_mut())),
			len: AtomicUsize::new(0),
			push: Mutex::new(()),
			_items: PhantomData,
		}
	}
}

/// The bucket of an index and the position in it.
#[inline]
fn locate(index: usize) -> (usize, usize) {
	let shifted = index + (1 << FIRST_BITS);
	let bucket = (usize::BITS - 1 - shifted.leading_zeros()) - FIRST_BITS;
	(bucket as usize, shifted - (1 << (bucket + FIRST_BITS)))
}

/// The number of items of a bucket.
#[inline]
fn bucket_len(bucket: usize) -> usize {
	1 << (bucket as u32 + FIRST_BITS)
}

impl<T> AppendVec<T> {
	/// An empty vector.
	pub fn new() -> Self {
		Self::default()
	}

	/// The number of items.
	#[inline]
	pub fn len(&self) -> usize {
		self.len.load(Ordering::Acquire)
	}

	/// Whether there is no item.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// The item at `index`, which must have been pushed before (by this
	/// thread, or by another one whose push this thread saw happen).
	#[inline]
	pub fn get(&self, index: usize) -> &T {
		assert!(index < self.len(), "store id {index} out of range");
		let (bucket, position) = locate(index);
		let items = self.buckets[bucket].load(Ordering::Acquire);
		// SAFETY: `index < len`, so its bucket is allocated and the item was
		// written before `len` was published (release in `push`).
		unsafe { &*items.add(position) }
	}

	/// Adds an item; returns its index.
	pub fn push(&self, value: T) -> usize {
		let _guard = self
			.push
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		let index = self.len.load(Ordering::Relaxed);
		let (bucket, position) = locate(index);
		let mut items = self.buckets[bucket].load(Ordering::Acquire);
		if items.is_null() {
			let fresh: Box<[MaybeUninit<T>]> = Box::new_uninit_slice(bucket_len(bucket));
			items = Box::into_raw(fresh).cast::<T>();
			self.buckets[bucket].store(items, Ordering::Release);
		}
		// SAFETY: the slot is in the bucket and not initialized yet (only
		// pushes write, under the lock).
		unsafe { items.add(position).write(value) };
		self.len.store(index + 1, Ordering::Release);
		index
	}
}

impl<T> Drop for AppendVec<T> {
	fn drop(&mut self) {
		let len = *self.len.get_mut();
		for bucket in 0..BUCKETS {
			let items = *self.buckets[bucket].get_mut();
			if items.is_null() {
				continue;
			}
			let size = bucket_len(bucket);
			let start = size - (1 << FIRST_BITS);
			let initialized = len.saturating_sub(start).min(size);
			// SAFETY: the first `initialized` items of the bucket were
			// written; the bucket was allocated as a boxed slice of `size`.
			unsafe {
				for position in 0..initialized {
					items.add(position).drop_in_place();
				}
				let slice =
					std::ptr::slice_from_raw_parts_mut(items.cast::<MaybeUninit<T>>(), size);
				drop(Box::from_raw(slice));
			}
		}
	}
}

/// Hash-consed values shared by every view: lock-free reads, an index
/// under a read-write lock.
pub struct GlobalValues<T> {
	items: AppendVec<T>,
	index: RwLock<HashTable<u32>>,
}

impl<T> Default for GlobalValues<T> {
	fn default() -> Self {
		Self {
			items: AppendVec::new(),
			index: RwLock::new(HashTable::new()),
		}
	}
}

impl<T: Hash + Eq> GlobalValues<T> {
	/// The value of a global id.
	#[inline]
	pub fn get(&self, id: u32) -> &T {
		self.items.get(id as usize)
	}

	/// The number of values.
	pub fn len(&self) -> usize {
		self.items.len()
	}

	/// Whether there is no value.
	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	/// The id of a value, if it is global (`hash`: [`hash_of`] it).
	pub fn find(&self, hash: u64, value: &T) -> Option<u32> {
		let index = self
			.index
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		index
			.find(hash, |&id| self.items.get(id as usize) == value)
			.copied()
	}

	/// Adds a value that lookups never find (the placeholder at id 0).
	pub fn push_unindexed(&self, value: T) -> u32 {
		u32::try_from(self.items.push(value)).expect("global store overflow")
	}

	/// The id of a value, inserted when it is not global yet. Its ids
	/// must all be global.
	pub fn insert(&self, value: T) -> u32 {
		let hash = hash_of(&value);
		let mut index = self
			.index
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		if let Some(&id) = index.find(hash, |&id| *self.items.get(id as usize) == value) {
			return id;
		}
		let id = u32::try_from(self.items.push(value)).expect("global store overflow");
		assert!(!is_local(id), "global store overflow");
		let items = &self.items;
		index.insert_unique(hash, id, |&other| hash_of(items.get(other as usize)));
		id
	}
}

/// Hash-consed lists shared by every view (see [`GlobalValues`]).
pub struct GlobalSlices<T> {
	items: AppendVec<Box<[T]>>,
	index: RwLock<HashTable<u32>>,
}

impl<T> Default for GlobalSlices<T> {
	fn default() -> Self {
		Self {
			items: AppendVec::new(),
			index: RwLock::new(HashTable::new()),
		}
	}
}

impl<T: Hash + Eq + Clone> GlobalSlices<T> {
	/// The list of a global id.
	#[inline]
	pub fn get(&self, id: u32) -> &[T] {
		self.items.get(id as usize)
	}

	/// The number of lists.
	pub fn len(&self) -> usize {
		self.items.len()
	}

	/// Whether there is no list.
	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	/// The id of a list, if it is global (`hash`: [`hash_of`] it).
	pub fn find(&self, hash: u64, list: &[T]) -> Option<u32> {
		let index = self
			.index
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		index
			.find(hash, |&id| &**self.items.get(id as usize) == list)
			.copied()
	}

	/// The id of a list, inserted when it is not global yet. Its ids must
	/// all be global.
	pub fn insert(&self, list: &[T]) -> u32 {
		let hash = hash_of(list);
		let mut index = self
			.index
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		if let Some(&id) = index.find(hash, |&id| &**self.items.get(id as usize) == list) {
			return id;
		}
		let id = u32::try_from(self.items.push(list.into())).expect("global store overflow");
		assert!(!is_local(id), "global store overflow");
		let items = &self.items;
		index.insert_unique(hash, id, |&other| hash_of(&**items.get(other as usize)));
		id
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use super::*;

	#[test]
	fn items_never_move() {
		let vector = AppendVec::new();
		let first = vector.push(String::from("first"));
		let reference: *const String = vector.get(first);
		for index in 0..10_000 {
			vector.push(format!("{index}"));
		}
		assert_eq!(vector.get(first), "first");
		assert!(std::ptr::eq(reference, vector.get(first)));
		assert_eq!(vector.get(10_000), "9999");
		assert_eq!(vector.len(), 10_001);
	}

	#[test]
	fn buckets_are_located() {
		assert_eq!(locate(0), (0, 0));
		assert_eq!(locate(31), (0, 31));
		assert_eq!(locate(32), (1, 0));
		assert_eq!(locate(95), (1, 63));
		assert_eq!(locate(96), (2, 0));
	}

	/// 16 threads intern overlapping values: one id per value.
	#[test]
	fn concurrent_inserts_give_one_id_per_value() {
		let values: Arc<GlobalValues<(u32, u32)>> = Arc::new(GlobalValues::default());
		let lists: Arc<GlobalSlices<u32>> = Arc::new(GlobalSlices::default());
		let found: Vec<Vec<(u32, u32)>> = std::thread::scope(|scope| {
			let handles: Vec<_> = (0..16u32)
				.map(|thread| {
					let values = Arc::clone(&values);
					let lists = Arc::clone(&lists);
					scope.spawn(move || {
						(0..2_000u32)
							.map(|step| {
								// overlapping: every thread meets every value
								let value = (step * 7 + thread) % 1_000;
								let id = values.insert((value, value * 3));
								let list = lists.insert(&[value, value + 1]);
								assert_eq!(
									values.find(hash_of(&(value, value * 3)), &(value, value * 3)),
									Some(id)
								);
								(id, list)
							})
							.collect()
					})
				})
				.collect();
			handles
				.into_iter()
				.map(|handle| handle.join().unwrap())
				.collect()
		});
		assert_eq!(values.len(), 1_000);
		assert_eq!(lists.len(), 1_000);
		for ids in &found {
			for &(id, list) in ids {
				let (value, triple) = *values.get(id);
				assert_eq!(triple, value * 3);
				assert_eq!(lists.get(list), &[value, value + 1]);
			}
		}
	}
}
