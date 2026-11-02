//! The memory an evaluation may use (decision D26 bounds the steps; this
//! bounds the memory, so that the compiler stays below its memory budget).
//!
//! Two checks, both reported as [`Error::Memory`]:
//!
//! - every value an operation is about to build in one go (a string
//!   concatenation, `repeat`, the items of a range…) is checked before it is
//!   allocated: no value may be larger than [`Limits::memory`];
//! - the heap of the whole process is checked every few steps, when the
//!   program installed the [`CountingAllocator`] (`fa`, the language
//!   server): an evaluation may not grow it by more than [`Limits::memory`].
//!
//! [`Error::Memory`]: crate::Error::Memory
//! [`Limits::memory`]: crate::Limits::memory

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The bytes allocated through the [`CountingAllocator`] and not freed yet.
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, counting the bytes in use. A program installs it
/// with `#[global_allocator]`; the counting costs an atomic addition per
/// allocation.
pub struct CountingAllocator;

// SAFETY: every method forwards to `System` with the same arguments; the
// counter is only statistics.
unsafe impl GlobalAlloc for CountingAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// SAFETY: forwarded as is (same contract).
		let pointer = unsafe { System.alloc(layout) };
		if !pointer.is_null() {
			ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
		}
		pointer
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		// SAFETY: forwarded as is (same contract).
		let pointer = unsafe { System.alloc_zeroed(layout) };
		if !pointer.is_null() {
			ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
		}
		pointer
	}

	unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
		// SAFETY: forwarded as is (same contract).
		unsafe { System.dealloc(pointer, layout) };
		ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
	}

	unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		// SAFETY: forwarded as is (same contract).
		let new = unsafe { System.realloc(pointer, layout, new_size) };
		if !new.is_null() {
			if new_size >= layout.size() {
				ALLOCATED.fetch_add(new_size - layout.size(), Ordering::Relaxed);
			} else {
				ALLOCATED.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
			}
		}
		new
	}
}

/// The bytes of heap in use, when the program installed the
/// [`CountingAllocator`] (otherwise 0).
pub fn allocated() -> usize {
	ALLOCATED.load(Ordering::Relaxed)
}

/// The limit of the address space of the process (`ulimit -v`), if any
/// (read from `/proc/self/limits` on Linux; `None` elsewhere).
pub fn address_space_limit() -> Option<usize> {
	static LIMIT: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
	*LIMIT.get_or_init(|| {
		let limits = std::fs::read_to_string("/proc/self/limits").ok()?;
		let line = limits
			.lines()
			.find(|line| line.starts_with("Max address space"))?;
		line.split_whitespace().nth(3)?.parse().ok()
	})
}

/// The default memory budget of an evaluation: 256 MB, or a quarter of the
/// address space when it is limited to less than 1 GB.
pub fn default_budget() -> usize {
	let budget = crate::eval::DEFAULT_MEMORY;
	address_space_limit().map_or(budget, |limit| budget.min(limit / 4))
}

/// The bytes of a collection of `count` values.
pub fn values_size(count: usize) -> usize {
	count.saturating_mul(std::mem::size_of::<crate::Value>())
}
