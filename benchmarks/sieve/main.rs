//! Sieve of Eratosthenes: a large array of booleans, strided writes, a linear scan.

use std::env;

const DEFAULT_SIZE: i64 = 100_000_000;

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let limit = size_argument(DEFAULT_SIZE);
	let length = usize::try_from(limit + 1).expect("the size must not be negative");
	let mut is_prime = vec![true; length];

	let mut count = 0;
	let mut sum = 0;
	let mut largest = 0;
	for i in 2..is_prime.len() {
		if is_prime[i] {
			count += 1;
			sum += i;
			largest = i;
			// Checked before computing `i * i`, which would exceed 2^53 on JavaScript targets.
			if i <= is_prime.len() / i {
				let mut multiple = i * i;
				while multiple < is_prime.len() {
					is_prime[multiple] = false;
					multiple += i;
				}
			}
		}
	}
	println!("primes up to {limit}: {count}, sum: {sum}, largest: {largest}");
}
