//! Word and number counting: hash maps with string and integer keys, insert-or-update, iteration.

use std::collections::HashMap;
use std::env;

const DEFAULT_SIZE: i64 = 5_000_000;
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
	let mut random = Random { state: 42 };

	let mut word_counts: HashMap<String, i64> = HashMap::new();
	for _ in 0..n {
		let mut word = String::new();
		for _ in 0..1 + random.next(3) {
			word += SYLLABLES[random.next(SYLLABLES.len() as i64) as usize];
		}
		*word_counts.entry(word).or_insert(0) += 1;
	}

	let mut number_counts: HashMap<i64, i64> = HashMap::new();
	for _ in 0..n {
		*number_counts.entry(random.next(100_000)).or_insert(0) += 1;
	}

	let mut best_word = "";
	let mut best_count = 0;
	let mut sum_of_squares = 0;
	for (word, &count) in &word_counts {
		sum_of_squares += count * count;
		if count > best_count || (count == best_count && word.as_str() < best_word) {
			best_word = word;
			best_count = count;
		}
	}

	let mut weighted_sum = 0;
	for (&number, &count) in &number_counts {
		weighted_sum += number * count;
	}

	println!(
		"distinct words: {}, sum of squares: {sum_of_squares}",
		word_counts.len()
	);
	println!("most frequent: {best_word} ({best_count} times)");
	println!(
		"distinct numbers: {}, weighted sum: {weighted_sum}",
		number_counts.len()
	);
}
