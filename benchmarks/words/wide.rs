//! Word frequencies: build a vocabulary and a skewed text of N words, split it, count the words
//! in a hash map keyed by strings.
//! From the Goose suite's `bench/rust/words_hashmap.rs` (the "rust HashMap<&str>" row: the
//! standard map with its default hasher, like the Fa `Map(String, Integer)`).

use std::collections::HashMap;
use std::env;

const DEFAULT_SIZE: i64 = 500_000;
const MAXW: usize = 20;

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
	let vocabulary_size = n / 10 + 16;
	let mut r: u64 = 12345;
	// A vocabulary of words, indexed randomly to build the text.
	let mut vocabulary: Vec<String> = Vec::with_capacity(vocabulary_size as usize);
	for _ in 0..vocabulary_size {
		let mut word = String::with_capacity(MAXW);
		r = xs_next(r);
		let length = 3 + xs_mod(r, 12);
		for _ in 0..length {
			r = xs_next(r);
			word.push((b'a' + xs_mod(r, 26) as u8) as char);
		}
		vocabulary.push(word);
	}
	// A text of N words drawn with a skew towards the front of the vocabulary.
	let mut text = String::new();
	for _ in 0..n {
		r = xs_next(r);
		let a = xs_mod(r, vocabulary_size);
		text.push_str(&vocabulary[((a * a) / vocabulary_size) as usize]);
		text.push(' ');
	}
	let words: Vec<&str> = text.split_whitespace().collect();

	let mut counts: HashMap<&str, i64> = HashMap::new();
	for word in &words {
		*counts.entry(word).or_insert(0) += 1;
	}
	let mut total: i64 = 0;
	for (word, count) in &counts {
		total += count * (word.len() as i64 + word.as_bytes()[0] as i64);
	}
	println!("{}", words.len());
	println!("{}", counts.len());
	println!("{total}");
}
