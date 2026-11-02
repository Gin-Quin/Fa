//! The `fa` binary: see the crate documentation of `fa` (`src/lib.rs`).

/// Counts the heap in use, so that compile-time evaluation stays within its
/// memory budget (`fa_interpreter::memory`).
#[global_allocator]
static ALLOCATOR: fa_interpreter::memory::CountingAllocator =
	fa_interpreter::memory::CountingAllocator;

fn main() {
	let code = fa::main(std::env::args_os().skip(1).collect());
	std::process::exit(code);
}
