//! The worker threads of the toolchain (design/parallel-pipeline.md, P8):
//! one Rayon pool whose threads have 64 MiB stacks, since the walks of the
//! analyzer and of the backends recurse deeply (Rayon's default is 2 MiB).
//!
//! `RAYON_NUM_THREADS` sets the number of threads, as for Rayon's own pool.

use std::sync::OnceLock;

/// The stack of a worker thread.
pub const STACK_SIZE: usize = 64 << 20;

/// The pool of worker threads.
pub fn pool() -> &'static rayon::ThreadPool {
	static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
	POOL.get_or_init(|| {
		rayon::ThreadPoolBuilder::new()
			.stack_size(STACK_SIZE)
			.thread_name(|index| format!("fa-worker-{index}"))
			.build()
			.expect("the worker threads can be started")
	})
}

/// Runs `work` on the pool: the parallel iterators it uses run on its
/// threads. Called from one of them, it runs `work` in place.
pub fn install<T: Send>(work: impl FnOnce() -> T + Send) -> T {
	pool().install(work)
}
