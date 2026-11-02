//! The global layer of the type store (design/parallel-pipeline.md §1.2,
//! P1–P2): what every analysis unit shares, append-only and `Sync`.
//!
//! - [`AppendVec`]: a segmented vector (32 buckets of growing sizes, never
//!   reallocated), whose reads take no lock and return references that live
//!   as long as the vector.
//! - [`GlobalValues`] / [`GlobalSlices`]: hash-consed values and lists in an
//!   [`AppendVec`], with a sharded index whose lookups take no lock: the
//!   units look values up while the merges of finished units (concurrent
//!   too, [`crate::types::merge`]) insert, each insert under the lock of
//!   its shard only.
//!
//! A store id whose bit 31 is set is *local* to one view of the store (one
//! unit, `store.rs`); the global tables never hold one.

use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, AtomicU64, AtomicUsize, Ordering};

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

/// The number of shards of an [`Index`] (a power of two): the inserts of
/// concurrent merges rarely meet in one.
const SHARDS: usize = 32;

/// An empty slot of an index table.
const EMPTY: u64 = 0;

/// The bits of a slot holding the high half of the hash of its value.
const TAG: u64 = 0xffff_ffff_0000_0000;

/// An open-addressing table of an index shard: each slot holds the high
/// half of the hash of a value and its id plus one (0: empty). At most three
/// quarters full, so that a probe always meets an empty slot, soon.
struct Slots {
	slots: Box<[AtomicU64]>,
}

impl Slots {
	fn new(len: usize) -> Box<Self> {
		Box::new(Slots {
			slots: (0..len).map(|_| AtomicU64::new(EMPTY)).collect(),
		})
	}

	/// The slot of an id whose value has this hash.
	#[inline]
	fn entry(hash: u64, id: u32) -> u64 {
		(hash & TAG) | (u64::from(id) + 1)
	}

	/// Puts an entry in the first empty slot of its probe sequence (by the
	/// writer of the shard only).
	fn put(&self, hash: u64, entry: u64) {
		let mask = self.slots.len() - 1;
		let mut position = (hash as usize) & mask;
		loop {
			let slot = &self.slots[position];
			if slot.load(Ordering::Relaxed) == EMPTY {
				slot.store(entry, Ordering::Release);
				return;
			}
			position = (position + 1) & mask;
		}
	}
}

/// A shard of an [`Index`]: its current table, read without a lock, and
/// the state of its writer.
struct Shard {
	table: AtomicPtr<Slots>,
	writer: Mutex<Writer>,
}

/// What the writer of a shard owns: the number of entries, and the tables
/// replaced by larger ones (a lookup may still probe one: they are freed
/// with the index; all together, at most the size of the current one).
#[derive(Default)]
struct Writer {
	count: usize,
	retired: Vec<*mut Slots>,
}

/// An index of the ids of a global table by the hash of their values
/// (design/parallel-pipeline.md §1.2): a lookup takes no lock, so the views
/// of running units never wait for a merge; inserts are serialized per
/// shard. A lookup finds every id inserted before it (a merge whose
/// interfaces the unit read); it may miss one a concurrent merge is
/// inserting, as if it had looked just before.
struct Index {
	shards: [Shard; SHARDS],
}

// SAFETY: the retired tables are owned by the index and freed by its drop
// only; the tables are made of atomics.
unsafe impl Send for Index {}
// SAFETY: as above; every shared access goes through an atomic, or through
// the lock of the shard's writer.
unsafe impl Sync for Index {}

impl Default for Index {
	fn default() -> Self {
		Self {
			shards: std::array::from_fn(|_| Shard {
				table: AtomicPtr::new(std::ptr::null_mut()),
				writer: Mutex::new(Writer::default()),
			}),
		}
	}
}

impl Index {
	#[inline]
	fn shard(&self, hash: u64) -> &Shard {
		// the high bits (an Fx hash mixes them best); the low ones place
		// the entry in the table of the shard
		&self.shards[(hash >> 59) as usize % SHARDS]
	}

	/// The id whose value has this hash and `matches`, if any; no lock.
	#[inline]
	fn find(&self, hash: u64, matches: impl Fn(u32) -> bool) -> Option<u32> {
		let table = self.shard(hash).table.load(Ordering::Acquire);
		if table.is_null() {
			return None;
		}
		// SAFETY: a published table is freed only when the index drops
		let slots = unsafe { &(*table).slots };
		let mask = slots.len() - 1;
		let tag = hash & TAG;
		let mut position = (hash as usize) & mask;
		loop {
			// acquire: the value of the id was pushed before the slot was set
			let entry = slots[position].load(Ordering::Acquire);
			if entry == EMPTY {
				return None;
			}
			if entry & TAG == tag {
				let id = (entry as u32) - 1;
				if matches(id) {
					return Some(id);
				}
			}
			position = (position + 1) & mask;
		}
	}

	/// The writer of the shard of a hash: its lock held.
	fn writer(&self, hash: u64) -> ShardWriter<'_> {
		let shard = self.shard(hash);
		let writer = shard
			.writer
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		ShardWriter { shard, writer }
	}
}

/// The writer of one shard of an [`Index`], its lock held: an insert finds
/// the value first, then adds its id.
struct ShardWriter<'i> {
	shard: &'i Shard,
	writer: std::sync::MutexGuard<'i, Writer>,
}

impl ShardWriter<'_> {
	/// Adds the id of a value with this hash (`hash_of_id`: the hash of the
	/// value of an id, to grow the table).
	fn insert(&mut self, hash: u64, id: u32, hash_of_id: impl Fn(u32) -> u64) {
		let shard = self.shard;
		let mut table = shard.table.load(Ordering::Relaxed);
		// SAFETY: only the writer replaces the table, and never frees it
		let len = if table.is_null() {
			0
		} else {
			unsafe { &*table }.slots.len()
		};
		if (self.writer.count + 1) * 4 > len * 3 {
			let grown = Slots::new((len * 2).max(16));
			if !table.is_null() {
				// SAFETY: as above
				for slot in unsafe { &*(*table).slots } {
					let entry = slot.load(Ordering::Relaxed);
					if entry != EMPTY {
						grown.put(hash_of_id((entry as u32) - 1), entry);
					}
				}
				self.writer.retired.push(table);
			}
			table = Box::into_raw(grown);
			// release: a lookup that sees the new table sees its entries
			shard.table.store(table, Ordering::Release);
		}
		// SAFETY: as above
		unsafe { (*table).put(hash, Slots::entry(hash, id)) };
		self.writer.count += 1;
	}
}

impl Drop for Index {
	fn drop(&mut self) {
		for shard in &mut self.shards {
			let current = *shard.table.get_mut();
			let writer = shard
				.writer
				.get_mut()
				.unwrap_or_else(|poisoned| poisoned.into_inner());
			for table in writer.retired.drain(..).chain(Some(current)) {
				if !table.is_null() {
					// SAFETY: allocated by `Box::into_raw`, freed once
					drop(unsafe { Box::from_raw(table) });
				}
			}
		}
	}
}

/// Hash-consed values shared by every view: reads and lookups take no
/// lock; inserts (the merges of finished units, which may run at the same
/// time) are serialized per shard of the index.
pub struct GlobalValues<T> {
	items: AppendVec<T>,
	index: Index,
}

impl<T> Default for GlobalValues<T> {
	fn default() -> Self {
		Self {
			items: AppendVec::new(),
			index: Index::default(),
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
	#[inline]
	pub fn find(&self, hash: u64, value: &T) -> Option<u32> {
		self.index
			.find(hash, |id| self.items.get(id as usize) == value)
	}

	/// Adds a value that lookups never find (the placeholder at id 0).
	pub fn push_unindexed(&self, value: T) -> u32 {
		u32::try_from(self.items.push(value)).expect("global store overflow")
	}

	/// The id of a value, inserted when it is not global yet. Its ids
	/// must all be global.
	pub fn insert(&self, value: T) -> u32 {
		let hash = hash_of(&value);
		let mut writer = self.index.writer(hash);
		if let Some(id) = self.find(hash, &value) {
			return id;
		}
		let id = u32::try_from(self.items.push(value)).expect("global store overflow");
		assert!(!is_local(id), "global store overflow");
		let items = &self.items;
		writer.insert(hash, id, |other| hash_of(items.get(other as usize)));
		id
	}
}

/// Hash-consed lists shared by every view (see [`GlobalValues`]).
pub struct GlobalSlices<T> {
	items: AppendVec<Box<[T]>>,
	index: Index,
}

impl<T> Default for GlobalSlices<T> {
	fn default() -> Self {
		Self {
			items: AppendVec::new(),
			index: Index::default(),
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
	#[inline]
	pub fn find(&self, hash: u64, list: &[T]) -> Option<u32> {
		self.index
			.find(hash, |id| &**self.items.get(id as usize) == list)
	}

	/// The id of a list, inserted when it is not global yet. Its ids must
	/// all be global.
	pub fn insert(&self, list: &[T]) -> u32 {
		let hash = hash_of(list);
		let mut writer = self.index.writer(hash);
		if let Some(id) = self.find(hash, list) {
			return id;
		}
		let id = u32::try_from(self.items.push(list.into())).expect("global store overflow");
		assert!(!is_local(id), "global store overflow");
		let items = &self.items;
		writer.insert(hash, id, |other| hash_of(&**items.get(other as usize)));
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
