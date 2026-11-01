// The Rust program the Fibonacci proof of concept is compared with, built by
// the `report` example with `rustc -O -C overflow-checks=on` (Fa checks every
// addition that the range analysis could not prove, Rust's release profile
// checks none: both are measured).

use std::hint::black_box;

fn fibonacci(n: i64) -> i64 {
	if n < 2 {
		n
	} else {
		fibonacci(n - 1) + fibonacci(n - 2)
	}
}

fn main() {
	let n = std::env::args().nth(1).map_or(30, |n| n.parse().expect("an integer"));
	println!("{}", fibonacci(black_box(n)));
}
