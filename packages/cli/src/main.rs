//! The `fa` binary: see the crate documentation of `fa` (`src/lib.rs`).

fn main() {
	let code = fa::main(std::env::args_os().skip(1).collect());
	std::process::exit(code);
}
