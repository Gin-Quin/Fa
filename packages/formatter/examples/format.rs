//! Format the standard input: `cargo run -p fa-formatter --example format < file.fa`.
//! `--no-rewrites` disables the canonical rewrites.

use std::io::Read;

use fa_formatter::{Options, format_with};

fn main() {
	let mut source = String::new();
	std::io::stdin()
		.read_to_string(&mut source)
		.expect("the standard input should be readable");
	let options = Options {
		rewrites: !std::env::args().any(|argument| argument == "--no-rewrites"),
		..Options::default()
	};
	match format_with(&source, &options) {
		Ok(formatted) => print!("{formatted}"),
		Err(diagnostics) => {
			for diagnostic in diagnostics {
				eprintln!("{}", diagnostic.message);
			}
			std::process::exit(1);
		}
	}
}
