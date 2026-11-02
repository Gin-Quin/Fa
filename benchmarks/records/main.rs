//! Variant records: build a log of events of four kinds (one with a string), aggregate it four
//! times with a `match`.
//! From the Goose suite's `bench/rust/records.rs` (the "rust enum + String" row).

use std::env;

const DEFAULT_SIZE: i64 = 4_000_000;
const PASSES: i64 = 4;

enum Event {
	Tick,
	Move { id: i32, dx: i16, dy: i16 },
	Say { id: i32, text: String },
	Quit { id: i32, code: u8 },
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

/// A word of 4 to 15 letters from its own chain, so the caller's stream advances once per event
/// whatever the variant is.
fn word(seed: u64) -> String {
	let mut word = String::new();
	let mut r = xs_next(seed);
	let length = 4 + xs_mod(r, 12);
	for _ in 0..length {
		r = xs_next(r);
		word.push((b'a' + xs_mod(r, 26) as u8) as char);
	}
	word
}

fn main() {
	let n = size_argument(DEFAULT_SIZE);
	let mut log: Vec<Event> = Vec::new();
	let mut r: u64 = 12345;
	for _ in 0..n {
		r = xs_next(r);
		let kind = xs_mod(r, 8);
		let id = xs_mod(r >> 3, 100_000) as i32;
		if kind < 4 {
			log.push(Event::Move {
				id,
				dx: (xs_mod(r >> 20, 201) - 100) as i16,
				dy: (xs_mod(r >> 40, 201) - 100) as i16,
			});
		} else if kind < 6 {
			log.push(Event::Say { id, text: word(r) });
		} else if kind < 7 {
			log.push(Event::Quit {
				id,
				code: xs_mod(r >> 20, 256) as u8,
			});
		} else {
			log.push(Event::Tick);
		}
	}
	let mut total: i64 = 0;
	for _ in 0..PASSES {
		for event in &log {
			total += match event {
				Event::Tick => 1,
				Event::Move { id, dx, dy } => *id as i64 + *dx as i64 + *dy as i64,
				Event::Say { id, text } => {
					*id as i64 + text.len() as i64 + text.as_bytes()[0] as i64
				}
				Event::Quit { id, code } => *id as i64 + *code as i64,
			};
		}
	}
	println!("{}", log.len());
	println!("{total}");
}
