//! Sorting: an array of objects with a three-key comparator (integer, string, integer), then an array of integers.

use std::env;

const DEFAULT_SIZE: i64 = 1_000_000;
const MODULUS: i64 = 1_000_000_007;
const SYLLABLES: [&str; 16] = [
	"ka", "lo", "mi", "nu", "pe", "ra", "si", "to", "va", "ze", "bo", "di", "fu", "ga", "he", "jo",
];

/// The "minimal standard" Park-Miller generator: deterministic and below 2^53 on every target.
struct Random {
	state: i64,
}

impl Random {
	fn next(&mut self, bound: i64) -> i64 {
		self.state = self.state * 48_271 % 2_147_483_647;
		self.state % bound
	}

	fn syllable(&mut self) -> &'static str {
		SYLLABLES[self.next(SYLLABLES.len() as i64) as usize]
	}
}

struct Record {
	id: i64,
	name: String,
	score: i64,
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
	let mut random = Random { state: 7 };

	let mut records = Vec::new();
	for id in 0..n {
		let first = random.syllable();
		let second = random.syllable();
		let name = format!("{first}{second}");
		let score = random.next(1_000);
		records.push(Record { id, name, score });
	}
	// Highest score first, then by name, then by id.
	records.sort_by(|a, b| {
		b.score
			.cmp(&a.score)
			.then_with(|| a.name.cmp(&b.name))
			.then(a.id.cmp(&b.id))
	});

	let mut record_checksum = 0;
	for record in &records {
		record_checksum = (record_checksum * 31 + record.id) % MODULUS;
	}

	let mut numbers = Vec::new();
	for _ in 0..n {
		numbers.push(random.next(1_000_000_000));
	}
	numbers.sort();

	let mut number_checksum = 0;
	for number in &numbers {
		number_checksum = (number_checksum * 31 + number) % MODULUS;
	}

	if let (Some(first), Some(last)) = (records.first(), records.last()) {
		println!("first: {} {} {}", first.id, first.name, first.score);
		println!("last: {} {} {}", last.id, last.name, last.score);
	}
	println!("record checksum: {record_checksum}");
	println!("number checksum: {number_checksum}");
}
