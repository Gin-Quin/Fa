use std::env;
use std::fs;
use std::time::Instant;

use fa_parser::parsing::parse::parse;

fn main() {
	let mut args = env::args();
	let program = args.next().unwrap_or_else(|| "fa-parser".to_string());
	let path = match args.next() {
		Some(path) => path,
		None => {
			eprintln!("Usage: {program} <path>");
			std::process::exit(1);
		}
	};

	let source = fs::read_to_string(&path).expect("failed to read file");
	let source = Box::leak(source.into_boxed_str());
	let started_at = Instant::now();
	let tree = parse(source);
	let elapsed = started_at.elapsed();
	let tree_ptr = &tree as *const _;

	println!("\n{:#?}\n", tree_ptr);
	println!("[TIMER] parse: {}μs", elapsed.as_micros());
	// println!("\n[RESULT]\n{}", tree.to_string());
}
