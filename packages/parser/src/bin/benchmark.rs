//! Measure the throughput of the tokenizer and the parser.
//!
//! `cargo run --release -p fa-parser --bin benchmark -- [file] [repeat]`
//!
//! Without a file, the input is every ```fa block of the documentation (`documentation/` and
//! `README.md` at the root of the repository), concatenated. The input is repeated `repeat`
//! times (30 by default) to get a few megabytes.

use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use fa_parser::parse;
use fa_parser::tokenize::tokenize;

const RUNS: usize = 5;

fn main() {
	let mut args = env::args().skip(1);
	let path = args.next().filter(|argument| argument != "-");
	let repeat: usize = args
		.next()
		.and_then(|repeat| repeat.parse().ok())
		.unwrap_or(30);

	let text = match path {
		Some(path) => fs::read_to_string(&path).expect("the input file should be readable"),
		None => documentation_snippets(),
	};
	let mut source = String::with_capacity((text.len() + 1) * repeat);
	for _ in 0..repeat {
		source.push_str(&text);
		source.push('\n');
	}
	let megabytes = source.len() as f64 / 1_000_000.0;

	let mut best_tokenize = f64::MAX;
	let mut best_parse = f64::MAX;
	let mut nodes = 0;
	for _ in 0..RUNS {
		let started = Instant::now();
		let tokens = tokenize(&source);
		best_tokenize = best_tokenize.min(started.elapsed().as_secs_f64());
		std::hint::black_box(tokens);

		let started = Instant::now();
		let tree = parse(&source);
		best_parse = best_parse.min(started.elapsed().as_secs_f64());
		nodes = tree.nodes.len();
		std::hint::black_box(tree);
	}

	println!("input: {megabytes:.2} MB, {nodes} nodes");
	println!("tokenize: {:.1} MB/s", megabytes / best_tokenize);
	println!(
		"parse (tokenize + parse): {:.1} MB/s",
		megabytes / best_parse
	);
}

/// Every ```fa block of the documentation, concatenated.
fn documentation_snippets() -> String {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
	let mut files = vec![root.join("README.md")];
	collect_markdown_files(&root.join("documentation"), &mut files);
	files.sort();

	let mut snippets = String::new();
	for file in files {
		let Ok(markdown) = fs::read_to_string(&file) else {
			continue;
		};
		let mut in_block = false;
		for line in markdown.lines() {
			let trimmed = line.trim();
			if in_block {
				if trimmed == "```" {
					in_block = false;
					snippets.push('\n');
				} else {
					snippets.push_str(line);
					snippets.push('\n');
				}
			} else if trimmed == "```fa" {
				in_block = true;
			}
		}
	}
	snippets
}

fn collect_markdown_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_markdown_files(&path, files);
		} else if path.extension().is_some_and(|extension| extension == "md") {
			files.push(path);
		}
	}
}
