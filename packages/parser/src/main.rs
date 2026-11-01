use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use fa_parser::parse;

/// Parse a file, print its tree, and print its diagnostics on stderr.
/// Exits with 1 if the file has an error, 2 if it can't be read.
fn main() -> ExitCode {
	let mut args = env::args();
	let program = args.next().unwrap_or_else(|| "fa-parser".to_string());
	let Some(path) = args.next() else {
		eprintln!("Usage: {program} <path>");
		return ExitCode::from(2);
	};

	let source = match fs::read_to_string(&path) {
		Ok(source) => source,
		Err(error) => {
			eprintln!("{path}: {error}");
			return ExitCode::from(2);
		}
	};

	let started_at = Instant::now();
	let tree = parse(&source);
	let elapsed = started_at.elapsed();

	println!("{tree}");
	println!("\n[TIMER] parse: {}μs", elapsed.as_micros());

	for diagnostic in &tree.diagnostics {
		eprintln!("{path}:{}", diagnostic.display(&tree.source_map));
	}

	if tree.has_errors() {
		ExitCode::from(1)
	} else {
		ExitCode::SUCCESS
	}
}
