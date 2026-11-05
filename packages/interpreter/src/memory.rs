//! The memory an evaluation may use (decision D26 bounds the steps; this
//! bounds the memory, so that the compiler stays below its memory budget).
//!
//! Two checks, both reported as [`Error::Memory`]:
//!
//! - every value an operation is about to build in one go (a string
//!   concatenation, `repeat`, the items of a range…) is checked before it is
//!   allocated: no value may be larger than [`Limits::memory`];
//! - the heap is checked every few steps, when the program installed the
//!   [`CountingAllocator`] (`fa`, the language server): an evaluation may
//!   not grow it by more than [`Limits::memory`]. The allocator counts per
//!   thread, and an evaluation runs on one thread: what the threads
//!   running other analyses and evaluations allocate meanwhile is not
//!   counted (the result doesn't depend on them), and the threads don't
//!   share a counter (no contended atomic per allocation).
//!
//! [`Error::Memory`]: crate::Error::Memory
//! [`Limits::memory`]: crate::Limits::memory

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

std::thread_local! {
	/// The bytes this thread allocated through the [`CountingAllocator`],
	/// minus the bytes it freed (wrapping: a thread may free what another
	/// one allocated).
	static ALLOCATED: Cell<usize> = const { Cell::new(0) };
}

/// Adds `bytes` to the counter of this thread (none while it is torn down).
#[inline]
fn count(bytes: usize, freed: bool) {
	let _ = ALLOCATED.try_with(|allocated| {
		allocated.set(if freed {
			allocated.get().wrapping_sub(bytes)
		} else {
			allocated.get().wrapping_add(bytes)
		});
	});
}

/// An allocator (the system's by default; mimalloc in the `fa` binaries,
/// design/parallel-pipeline.md P8), counting the bytes in use per thread. A
/// program installs it with `#[global_allocator]`; the counting costs an
/// addition to a thread-local counter per allocation.
pub struct CountingAllocator<A = System>(pub A);

impl CountingAllocator {
	/// The system allocator, counted.
	pub const SYSTEM: Self = CountingAllocator(System);
}

// SAFETY: every method forwards to the inner allocator with the same
// arguments; the counter is only statistics.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// SAFETY: forwarded as is (same contract).
		let pointer = unsafe { self.0.alloc(layout) };
		if !pointer.is_null() {
			count(layout.size(), false);
		}
		pointer
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		// SAFETY: forwarded as is (same contract).
		let pointer = unsafe { self.0.alloc_zeroed(layout) };
		if !pointer.is_null() {
			count(layout.size(), false);
		}
		pointer
	}

	unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
		// SAFETY: forwarded as is (same contract).
		unsafe { self.0.dealloc(pointer, layout) };
		count(layout.size(), true);
	}

	unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		// SAFETY: forwarded as is (same contract).
		let new = unsafe { self.0.realloc(pointer, layout, new_size) };
		if !new.is_null() {
			if new_size >= layout.size() {
				count(new_size - layout.size(), false);
			} else {
				count(layout.size() - new_size, true);
			}
		}
		new
	}
}

/// The bytes of heap this thread allocated and didn't free, when the
/// program installed the [`CountingAllocator`] (otherwise 0); wrapping: the
/// difference of two readings is what the thread allocated in between.
pub fn allocated() -> usize {
	ALLOCATED.try_with(Cell::get).unwrap_or(0)
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
