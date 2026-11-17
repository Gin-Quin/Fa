//! Allocation-heavy loops under a memory cap, on every target: each program
//! allocates gigabytes over its run, and must run in 128 MiB of address
//! space (`ulimit -v`, native and freestanding) or of linear memory (the
//! loader's `FA_MEMORY_LIMIT`, wasm). A leak makes it stop with `out of
//! memory` (exit 101) or the loader report it (exit 103).

#[cfg(not(fa_llvm))]
#[test]
fn memory_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::PathBuf;
	use std::process::Command;

	use fa_codegen_llvm::{Options, Target, compile_file, javascript_runtime};

	/// The cap, in bytes.
	const LIMIT: u64 = 128 << 20;

	/// Strings: built, split, mapped, joined (about 1 GB allocated).
	const STRINGS: &str = r#"
mutable total = 0
for index in 0..100_000 {
	let line = "item {index} " + "x".repeat(1000)
	mutable parts = line.split(" ")
	parts.push(line.toUpperCase())
	let lengths = parts.map((part) => part.length)
	total += lengths.sum()
	if index %% 2 == 0 {
		continue
	}
	let duplicate = parts
	total += duplicate.length
}
console.log(total)
"#;

	/// Objects holding arrays, named unions with data, optionals, errors.
	const VALUES: &str = r#"
type Shape = Union {
	Circle = { radius: Float, label: String }
	Polygon = { points: Array<Integer>, name: String }
	Point
}
type Box = { name: String, items: Array<String>, best: String? }

function make = (index: Integer): Shape => when index %% 3 is {
	0 => Shape.Circle { radius = 1.0, label = "circle {index}" }
	1 => Shape.Polygon { points = [index, index + 1, index + 2], name = "p" + "{index}" }
	else => Shape.Point
}

function check = (text: String): Integer throws ParseError => {
	let trimmed = text.trim()
	try Integer.parse(trimmed)
}

mutable count = 0
mutable keep: Array<Shape> = []
for index in 0..200_000 {
	let shape = make(index)
	mutable holder: Box = { name = "box {index}", items = ["a" + "b", "c".repeat(200)], best = none }
	holder.best = holder.items.get(1)
	let copied = copy holder
	holder.items = []
	count += copied.items.length
	let parsed = check(if index %% 2 == 0 { " {index} " } else { "x{index}" }) catch error => -1
	if parsed >= 0 {
		count += 1
	}
	keep.push(shape)
	if keep.length > 100 {
		keep = []
	}
}
console.log(count)
"#;

	fn compile(name: &str, source: &str, target: Target) -> fa_codegen_llvm::Artifact {
		let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-memory")
			.join(target.name());
		std::fs::create_dir_all(&directory).expect("a directory");
		let path = directory.join(format!("{name}.fa"));
		std::fs::write(&path, source).expect("a program");
		// the loader checks the memory cap of a leak-checked build
		let options = Options {
			leak_check: target == Target::Wasm,
			..Options::default()
		};
		compile_file(&path, target, &options, &directory.join(name))
			.unwrap_or_else(|error| panic!("{name} does not compile: {error}"))
	}

	fn run_capped(name: &str, source: &str, target: Target) -> String {
		let artifact = compile(name, source, target);
		let output = match target {
			Target::Wasm => {
				let Ok(runtime) = javascript_runtime() else {
					return String::new();
				};
				Command::new(runtime)
					.arg(artifact.loader.as_ref().expect("a loader"))
					.env("FA_MEMORY_LIMIT", LIMIT.to_string())
					.output()
			}
			_ => Command::new("sh")
				.arg("-c")
				.arg(format!("ulimit -v {}; exec \"$0\"", LIMIT / 1024))
				.arg(&artifact.output)
				.output(),
		}
		.expect("the program runs");
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(
			output.status.success(),
			"{name} on {}: {:?}\n{stderr}",
			target.name(),
			output.status
		);
		String::from_utf8_lossy(&output.stdout).trim().to_string()
	}

	fn all_targets(name: &str, source: &str, expected: &str) {
		let mut targets = vec![Target::Native, Target::Wasm];
		if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
			targets.push(Target::NativeFreestanding);
		}
		for target in targets {
			let printed = run_capped(name, source, target);
			if printed.is_empty() && target == Target::Wasm {
				eprintln!("skipped wasm: neither node nor bun was found");
				continue;
			}
			assert_eq!(printed, expected, "{name} on {}", target.name());
		}
	}

	#[test]
	fn strings_in_a_loop_are_freed() {
		all_targets("strings", STRINGS, "202177780");
	}

	#[test]
	fn values_in_a_loop_are_freed() {
		all_targets("values", VALUES, "500000");
	}

	#[test]
	fn inline_arrays_in_a_loop_are_freed() {
		all_targets(
			"inline-array",
			r#"
function make = (label: String): InlineArray<String, 4> => [label, label.repeat(1000)]
mutable total = 0
for index in 0..50_000 {
	mutable values = make("item {index}")
	let duplicate = copy values
	let last = values.pop()
	values.clear()
	total += duplicate.length
	if index %% 2 == 0 { continue }
	assert last is not none
}
console.log(total)
"#,
			"100000",
		);
	}
}
