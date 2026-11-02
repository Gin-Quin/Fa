//! Print the ```fa blocks of the documentation that the formatter changes, before and after:
//! `cargo run -p fa-formatter --example documentation_changes`.

#[path = "../../parser/tests/support/snippets.rs"]
#[allow(dead_code)]
mod support;

use fa_formatter::format;

fn main() {
	let mut changed = 0;
	let snippets = support::snippets();
	for snippet in &snippets {
		let Ok(formatted) = format(&snippet.code) else {
			continue;
		};
		if formatted != snippet.code {
			changed += 1;
			println!("===== {}", snippet.location());
			println!("{}", snippet.code);
			println!("----- formatted");
			println!("{formatted}");
		}
	}
	println!("{changed} of {} blocks change", snippets.len());
}
