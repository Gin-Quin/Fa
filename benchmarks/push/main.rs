//! Pushing records while keeping a way back to every 64th one.
//! From the Goose suite's `bench/rust/push.rs` (the "rust Vec+reserve, indices" row): safe Rust
//! can't keep references across `push`, so the marks are indices.

use std::env;

const DEFAULT_SIZE: i64 = 32_000_000;

struct Item {
	key: i32,
	val: i32,
}

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
	let mut items: Vec<Item> = Vec::with_capacity(n as usize);
	let mut marks: Vec<usize> = Vec::new();
	let mut r: u64 = 12345;
	for i in 0..n {
		r = xs_next(r);
		items.push(Item {
			key: xs_mod(r, 1_000_000) as i32,
			val: (i % 97) as i32,
		});
		if i % 64 == 0 {
			marks.push(items.len() - 1);
		}
	}
	let mut total: i64 = 0;
	for item in &items {
		total += item.key as i64;
	}
	let mut marked_total: i64 = 0;
	for &mark in &marks {
		marked_total += items[mark].val as i64;
	}
	println!("{}", items.len());
	println!("{}", marks.len());
	println!("{total}");
	println!("{marked_total}");
}
