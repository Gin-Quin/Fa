// The Rust version of `collatz.fa`, built by the `report` example.

fn steps(start: i64) -> i64 {
	let mut value = start;
	let mut count = 0;
	while value != 1 {
		if value % 2 == 0 {
			value /= 2;
		} else {
			value = 3 * value + 1;
		}
		count += 1;
	}
	count
}

fn main() {
	let limit = std::hint::black_box(3_000_000);
	let (mut longest, mut best) = (0, 1);
	for start in 1..limit {
		let length = steps(start);
		if length > longest {
			longest = length;
			best = start;
		}
	}
	println!("{best} takes {longest} steps");
}
