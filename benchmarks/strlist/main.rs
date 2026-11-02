//! A list of strings: build a text of random words, split it into a list of owned words, scan
//! the list four times (length, first and last character of each word).
//! From the Goose suite's `bench/rust/strlist_owned.rs` (the "rust Vec<String>" row: the list
//! owns its words, like the Fa and Goose versions).

use std::env;

const DEFAULT_SIZE: i64 = 2_000_000;
const PASSES: i64 = 4;

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
	let mut text = String::new();
	let mut r: u64 = 12345;
	for _ in 0..n {
		r = xs_next(r);
		let length = 3 + xs_mod(r, 18);
		for _ in 0..length {
			r = xs_next(r);
			text.push((b'a' + xs_mod(r, 26) as u8) as char);
		}
		text.push(' ');
	}
	let words: Vec<String> = text.split_whitespace().map(String::from).collect();
	let mut total: i64 = 0;
	for _ in 0..PASSES {
		for word in &words {
			let bytes = word.as_bytes();
			total += bytes.len() as i64 + bytes[0] as i64 + bytes[bytes.len() - 1] as i64;
		}
	}
	println!("{}", words.len());
	println!("{total}");
}
