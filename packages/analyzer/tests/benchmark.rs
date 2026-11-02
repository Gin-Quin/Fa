//! The speed of the analysis (design §4: under 5 ms for 5 000 lines of
//! typical code). Ignored by default; run it in release mode:
//!
//! ```sh
//! cargo test --release -p fa-analyzer --test benchmark -- --ignored --nocapture
//! ```
//!
//! It times the programs of `benchmarks/` (typical code), and generated
//! files that stress one part each: flat declarations, loops nested three
//! deep, and many module-level mutable values (large flow states).

use std::path::Path;
use std::time::{Duration, Instant};

/// Analyzes `text` `repeat` times; returns the time of one analysis (parse included).
fn time(text: &str, repeat: u32) -> Duration {
	let start = Instant::now();
	for _ in 0..repeat {
		let (session, file) = fa_analyzer::analyze_source(text);
		std::hint::black_box(session.diagnostics(file).len());
	}
	start.elapsed() / repeat
}

fn report(name: &str, text: &str) {
	let lines = text.lines().count();
	let parse = {
		let start = Instant::now();
		for _ in 0..10 {
			std::hint::black_box(fa_parser::parse(text));
		}
		start.elapsed() / 10
	};
	let total = time(text, 10);
	let per_5000 = total.as_secs_f64() * 1000.0 * 5000.0 / lines.max(1) as f64;
	println!(
		"{name:28} {lines:6} lines  parse {parse:>10.2?}  parse + analysis {total:>10.2?}  ({per_5000:.2} ms per 5 000 lines)"
	);
}

/// `count` blocks of three nested loops with arithmetic, conditions and a `loop` value.
fn nested_loops(count: usize) -> String {
	let mut text = String::new();
	for k in 0..count {
		text += &format!(
			"mutable total{k} = 0\n\
			 for i in 0..100 {{\n\
			 \tmutable j = 0\n\
			 \twhile j < i {{\n\
			 \t\tlet a = Int8(j)\n\
			 \t\tlet w: Int16 = a + a * 2\n\
			 \t\tif a > 3 and j != 7 {{ total{k} += a }} else {{ total{k} -= 1 }}\n\
			 \t\tfor m in 0..j {{ let q = 10 // (m + 1) }}\n\
			 \t\tj += 1\n\
			 \t}}\n\
			 }}\n\
			 let r{k} = loop {{\n\
			 \tif total{k} > 10 {{ break total{k} }}\n\
			 \ttotal{k} += 3\n\
			 }}\n\
			 let v{k}: Integer = if r{k} > 4 {{ r{k} * 2 }} else {{ 0 }}\n\
			 console.log(v{k})\n\
			 let c{k} = 1 < 2 <= 3\n\
			 let d{k} = Float(3) * 0.5\n\
			 let e{k} = (v{k} + 1) %% 7\n"
		);
	}
	text
}

/// `count` blocks of straight-line declarations, conditions and calls.
fn flat(count: usize) -> String {
	let mut text = String::new();
	for k in 0..count {
		text += &format!(
			"let a{k} = {} + 3\n\
			 let b{k}: Integer = a{k} * 2 - 1\n\
			 let c{k} = if b{k} > 4 {{ b{k} }} else {{ 0 }}\n\
			 let d{k} = 1 < c{k} and c{k} <= 3\n\
			 console.log(a{k}, d{k})\n",
			k % 10
		);
	}
	text
}

/// `count` loops, each over its own module-level mutable value.
fn module_values(count: usize) -> String {
	let mut text = String::new();
	for k in 0..count {
		text += &format!(
			"mutable t{k} = 0\n\
			 for i in 0..100 {{\n\
			 \tlet a = Int8(i)\n\
			 \tif a > 3 {{ t{k} += a }} else {{ t{k} -= 1 }}\n\
			 \tlet w: Int16 = a * 2\n\
			 }}\n\
			 console.log(t{k})\n"
		);
	}
	text
}

#[test]
#[ignore]
fn analysis_speed() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks");
	let mut programs = String::new();
	let mut names: Vec<_> = std::fs::read_dir(&root)
		.map(|entries| entries.flatten().map(|entry| entry.path()).collect())
		.unwrap_or_default();
	names.sort();
	for directory in names {
		let Ok(text) = std::fs::read_to_string(directory.join("main.fa")) else {
			continue;
		};
		let name = directory.file_name().unwrap().to_string_lossy().to_string();
		report(&format!("benchmarks/{name}"), &text);
		programs += &format!("do {{\n{text}\n}}\n");
	}
	// the benchmark programs repeated to about 5 000 lines, each in its own scope
	let lines = programs.lines().count().max(1);
	report("benchmarks, repeated", &programs.repeat(5000 / lines + 1));
	report("flat declarations", &flat(1000));
	report("module-level mutable values", &module_values(700));
	report("loops nested three deep", &nested_loops(250));
}
