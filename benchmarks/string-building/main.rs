//! String building: interpolation, appending to a growing string, padding, splitting and joining.

use std::env;
use std::fmt::Write;

const DEFAULT_SIZE: i64 = 3_000_000;
const BLOCK_SIZE: i64 = 1_000;
const LABELS: [&str; 5] = ["alpha", "beta", "gamma", "delta", "epsilon"];

#[derive(Default)]
struct Totals {
	lines: usize,
	characters: usize,
	joined_characters: usize,
}

impl Totals {
	fn add(&mut self, text: &str) {
		let parts: Vec<&str> = text.split(';').collect();
		self.lines += parts.len();
		self.characters += text.len();
		self.joined_characters += parts.join("\n").len();
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
	let mut totals = Totals::default();
	let mut output = String::new();
	let mut last_entry = String::new();
	for i in 0..n {
		let label = LABELS[(i % 5) as usize];
		last_entry = format!("{i:08} {label} x{} = {}", i % 97, i * 3 + 1);
		write!(output, "{last_entry};").expect("writing to a String never fails");
		if i % BLOCK_SIZE == BLOCK_SIZE - 1 {
			totals.add(&output);
			output.clear();
		}
	}
	if !output.is_empty() {
		totals.add(&output);
	}
	println!("last entry: {last_entry}");
	println!(
		"lines: {}, characters: {}, joined characters: {}",
		totals.lines, totals.characters, totals.joined_characters
	);
}
