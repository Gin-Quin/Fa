//! Builds a standalone Fa file with the LLVM backend, until the `fa` command
//! line wires it:
//!
//! ```bash
//! cargo run -p fa-codegen-llvm --example build -- file.fa --target=native
//! cargo run -p fa-codegen-llvm --example build -- file.fa --target=wasm --output=build/wasm
//! cargo run -p fa-codegen-llvm --example build -- file.fa --size --emit-ir --run -- arguments…
//! ```
//!
//! Options: `--target=native|wasm|native-freestanding` (default `native`),
//! `--output=<directory>` (default `build/<target>/`), `-O0` to `-O3`
//! (default `-O3`), `--size`, `--overflow-checks=false`, `--emit-ir`, `--leak-check`, `--asynchronous=true|false`, `--run` (runs the program with the
//! arguments after `--`, and exits with its code).

#[cfg(not(fa_llvm))]
fn main() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
	std::process::exit(2);
}

#[cfg(fa_llvm)]
fn main() {
	if let Err(message) = with_llvm::main() {
		eprintln!("{message}");
		std::process::exit(1);
	}
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::io::Write;
	use std::path::PathBuf;

	use fa_codegen_llvm::{OptLevel, Options, Target, compile_file, run};

	pub fn main() -> Result<(), String> {
		let mut file = None;
		let mut target = Target::Native;
		let mut output = None;
		let mut options = Options::default();
		let mut execute = false;
		let mut program_arguments = Vec::new();
		let mut arguments = std::env::args().skip(1);
		while let Some(argument) = arguments.next() {
			match argument.as_str() {
				"--" => {
					program_arguments.extend(arguments.by_ref());
				}
				"--size" => options.size = true,
				"--overflow-checks=false" => options.overflow_checks = false,
				"--overflow-checks=true" => options.overflow_checks = true,
				"--emit-ir" => options.emit_ir = true,
				"--leak-check" => options.leak_check = true,
				"--asynchronous=true" => options.asynchronous = Some(true),
				"--asynchronous=false" => options.asynchronous = Some(false),
				"--run" => execute = true,
				"-O0" => options.opt_level = OptLevel::O0,
				"-O1" => options.opt_level = OptLevel::O1,
				"-O2" => options.opt_level = OptLevel::O2,
				"-O3" => options.opt_level = OptLevel::O3,
				_ => {
					if let Some(name) = argument.strip_prefix("--target=") {
						target = Target::from_name(name).ok_or_else(|| {
							format!("unknown target `{name}`: native, wasm or native-freestanding")
						})?;
					} else if let Some(directory) = argument.strip_prefix("--output=") {
						output = Some(PathBuf::from(directory));
					} else if argument.starts_with('-') {
						return Err(format!("unknown option `{argument}`"));
					} else {
						file = Some(PathBuf::from(argument));
					}
				}
			}
		}
		let file = file.ok_or("usage: build <file.fa> [--target=native|wasm] [--output=<directory>] [-O0..-O3] [--size] [--emit-ir] [--run [-- arguments…]]")?;
		let directory = output.unwrap_or_else(|| PathBuf::from("build").join(target.name()));
		let stem = file
			.file_stem()
			.map(|stem| stem.to_string_lossy().into_owned())
			.unwrap_or_else(|| "main".to_string());
		let artifact = compile_file(&file, target, &options, &directory.join(stem))
			.map_err(|error| error.to_string())?;
		if !execute {
			println!(
				"{} ({} bytes, {:.1} ms)",
				artifact
					.loader
					.as_ref()
					.unwrap_or(&artifact.output)
					.display(),
				artifact.output_size,
				artifact.timings.total().as_secs_f64() * 1000.0
			);
			return Ok(());
		}
		let arguments: Vec<&str> = program_arguments.iter().map(String::as_str).collect();
		let result = run(&artifact, &arguments).map_err(|error| error.to_string())?;
		std::io::stdout().write_all(&result.stdout).ok();
		std::io::stderr().write_all(&result.stderr).ok();
		std::process::exit(result.status.code().unwrap_or(1));
	}
}
