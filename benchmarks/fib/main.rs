//! Recursive Fibonacci: function calls and checked integer additions.

use std::env;

const DEFAULT_SIZE: i64 = 42;

fn fib(n: i64) -> i64 {
	if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
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
	println!("fib({n}) = {}", fib(n));
}
