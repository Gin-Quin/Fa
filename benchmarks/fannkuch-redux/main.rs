//! Fannkuch-redux: permutations and pancake flips on small integer arrays with data-dependent indices.

use std::env;

const DEFAULT_SIZE: i64 = 10;

struct Fannkuch {
	checksum: i64,
	max_flips: i64,
}

fn fannkuch(n: usize) -> Fannkuch {
	let mut permutation: Vec<usize> = (0..n).collect();
	let mut current = vec![0; n];
	let mut count = vec![0; n];
	let mut max_flips = 0;
	let mut checksum = 0;
	let mut permutation_index = 0;
	let mut r = n;
	loop {
		while r != 1 {
			count[r - 1] = r;
			r -= 1;
		}

		current.copy_from_slice(&permutation);
		let mut flips = 0;
		let mut first = current[0];
		while first != 0 {
			current[..=first].reverse();
			flips += 1;
			first = current[0];
		}
		max_flips = max_flips.max(flips);
		checksum += if permutation_index % 2 == 0 {
			flips
		} else {
			-flips
		};

		// The next permutation, in the order of the benchmark.
		loop {
			if r == n {
				return Fannkuch {
					checksum,
					max_flips,
				};
			}
			permutation[..=r].rotate_left(1);
			count[r] -= 1;
			if count[r] > 0 {
				break;
			}
			r += 1;
		}
		permutation_index += 1;
	}
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
	let size = usize::try_from(n)
		.ok()
		.filter(|&size| size >= 1)
		.expect("the size must be at least 1");
	let result = fannkuch(size);
	println!("{}", result.checksum);
	println!("Pfannkuchen({n}) = {}", result.max_flips);
}
