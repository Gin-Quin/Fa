//! Measure the throughput of the formatter on the ```fa blocks of the documentation:
//! `cargo run --release -p fa-formatter --example benchmark [repeat]`.
//!
//! Prints the throughput on the blocks one by one, then on files made of all of them, 1, 4 and
//! 16 times, to check that the time grows linearly with the size.

#[path = "../../parser/tests/support/snippets.rs"]
#[allow(dead_code)]
mod support;

use std::time::Instant;

use fa_formatter::format;
use fa_parser::parse;

fn main() {
	let repeat: usize = std::env::args()
		.nth(1)
		.and_then(|argument| argument.parse().ok())
		.unwrap_or(20);
	let blocks: Vec<String> = support::snippets()
		.into_iter()
		.map(|snippet| snippet.code)
		.filter(|code| !parse(code).has_errors())
		.collect();
	let bytes: usize = blocks.iter().map(String::len).sum();

	let started_at = Instant::now();
	for _ in 0..repeat {
		for block in &blocks {
			std::hint::black_box(format(block).ok());
		}
	}
	let seconds = started_at.elapsed().as_secs_f64();
	println!(
		"{} blocks, {:.1} KB: {:.1} MB/s (parsing and formatting)",
		blocks.len(),
		bytes as f64 / 1e3,
		(bytes * repeat) as f64 / seconds / 1e6
	);

	let started_at = Instant::now();
	for _ in 0..repeat {
		for block in &blocks {
			std::hint::black_box(parse(block));
		}
	}
	let parsing = started_at.elapsed().as_secs_f64();
	println!(
		"parsing alone: {:.1} MB/s, formatting alone: {:.1} MB/s",
		(bytes * repeat) as f64 / parsing / 1e6,
		(bytes * repeat) as f64 / (seconds - parsing).max(1e-9) / 1e6
	);

	let file: String = blocks.join("\n");
	for copies in [1, 4, 16] {
		let source = file.repeat(copies);
		let started_at = Instant::now();
		let formatted = format(&source).expect("the blocks together should parse");
		let seconds = started_at.elapsed().as_secs_f64();
		std::hint::black_box(formatted);
		println!(
			"one file of {:.1} MB: {:.1} ms, {:.1} MB/s",
			source.len() as f64 / 1e6,
			seconds * 1e3,
			source.len() as f64 / seconds / 1e6
		);
	}
}
