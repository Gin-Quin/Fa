//! Control: build a flat `i32` array from a random generator, then scan it eight times.
//! From the Goose suite's `bench/rust/sum.rs` (the "rust Vec, collect" row).

use std::env;

const DEFAULT_SIZE: i64 = 32_000_000;
const PASSES: i64 = 8;

/// xorshift64, as `rng.goose`.
fn xs_next(x: u64) -> u64 {
	let mut v = x;
	v ^= v << 13;
	v ^= v >> 7;
	v ^= v << 17;
	v
}

/// Reduction into `[0, k)`, as C's `uint64_t` modulo; the result fits an `i64`.
fn xs_mod(x: u64, k: i64) -> i64 {
	(x % k as u64) as i64
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let n = size_argument(DEFAULT_SIZE);
	let mut r: u64 = 12345;
	let data: Vec<i32> = (0..n)
		.map(|_| {
			r = xs_next(r);
			xs_mod(r, 1000) as i32
		})
		.collect();
	let mut total: i64 = 0;
	for p in 0..PASSES {
		// The `^ p` keeps the passes from collapsing into one multiply.
		for &x in &data {
			total += (x as i64) ^ p;
		}
	}
	println!("{}", data.len());
	println!("{total}");
}
