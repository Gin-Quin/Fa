//! The memory bound of `fa check` (design/parallel-pipeline.md, step 5):
//! a generated package of 2,000 files of about 500 lines is checked in this
//! process, whose peak resident memory must stay under 512 MB. Ignored by
//! default (it takes a minute): `cargo test --release -p fa --test memory --
//! --ignored`.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Writes a package of `count` files into `root`: namespaces with types,
/// loops, `when`, errors, generics, statics, and calls to earlier files.
fn generate(root: &Path, count: usize) {
	fs::create_dir_all(root.join("library")).unwrap();
	fs::create_dir_all(root.join("application")).unwrap();
	fs::write(
		root.join("package.fa"),
		"export = {\n\tname = \"memory\"\n\tversion = \"0.1.0\"\n\ttargets = [\"bun\", \"native\"]\n}\n",
	)
	.unwrap();
	for index in 0..count {
		let mut text = String::from("export namespace = {\n");
		text.push_str("\ttype Item = {\n\t\tid: Integer\n\t\tweight: Integer\n\t\tlabel: String\n");
		let _ = writeln!(
			text,
			"\t\tfunction score = (self): Integer => self.id * {} + self.weight\n\t}}",
			index % 7 + 2
		);
		text.push_str("\ttype Shape = Union {\n\t\tCircle = { radius: Integer }\n\t\tSquare = { side: Integer }\n\t\tEmpty\n\t}\n");
		text.push_str("\ttype Level = Enum { low, middle, high }\n");
		text.push_str("\ttype Failure = Errors { Missing = { key: String }, Invalid }\n");
		if index % 25 == 3 {
			text.push_str("\tstatic base = 3\n\tfunction square = (x: Integer): Integer => x * x\n\tstatic table = @square(base)\n");
		}
		let _ = writeln!(
			text,
			"\tfunction make = (seed: Integer): Item => Item {{ id = seed, weight = seed %% {}, label = \"m{index}-{{seed}}\" }}",
			index % 11 + 7
		);
		for block in 0..11 {
			let _ = write!(
				text,
				"\tfunction area{block} = (shape: Shape): Integer => when shape is {{\n\t\tShape.Circle as {{ radius }} => radius * radius * 3\n\t\tShape.Square as {{ side }} => side * side\n\t\tShape.Empty => 0\n\t}}\n\
				\tfunction total{block} = (items: Array(Item)): Integer => {{\n\t\tmutable sum = 0\n\t\tfor index in 0..items.length {{\n\t\t\tsum += items[index].weight %% 13\n\t\t}}\n\t\tsum\n\t}}\n\
				\tfunction lookup{block} = (key: String, items: Array(Item)): Item throws Failure => {{\n\t\tfor item in items {{\n\t\t\tif item.label == key {{\n\t\t\t\treturn item\n\t\t\t}}\n\t\t}}\n\t\tthrow Failure.Missing {{ key }}\n\t}}\n\
				\tfunction safe{block} = (key: String, items: Array(Item)): Integer => lookup{block}(key, items).weight catch error => -1\n\
				\tfunction firstOr{block}(Value) = (values: Array(Value), fallback: Value): Value => values.get(0) ?? fallback\n\
				\tfunction level{block} = (value: Integer): Level => if value < 5 {{ .low }} else if value < 50 {{ .middle }} else {{ .high }}\n\
				\tfunction compute{block} = (x: Integer): Integer => {{\n\t\tlet items = [make(x), make(x + 2)]\n\t\tlet shape = if x %% 2 == 0 {{ Shape.Circle {{ radius = x %% 7 }} }} else {{ Shape.Square {{ side = x %% 5 }} }}\n\t\tmutable result = total{block}(items) + area{block}(shape) + safe{block}(\"m{index}-{{x}}\", items) + firstOr{block}([x], 0)\n"
			);
			for used in [index / 2, index / 3]
				.into_iter()
				.filter(|&used| used < index)
			{
				let _ = writeln!(
					text,
					"\t\tresult += m{used:04}.compute{}(x %% 100) %% 1000 + m{used:04}.make(x).score() %% 100",
					block % 11
				);
			}
			text.push_str("\t\twhen level0(result) is {\n\t\t\tLevel.low => result += 1\n\t\t\tLevel.middle => result += 2\n\t\t\tLevel.high => result -= 3\n\t\t}\n\t\tresult %% 50000\n\t}\n");
		}
		text.push_str("}\n");
		fs::write(root.join(format!("library/m{index:04}.fa")), text).unwrap();
	}
	fs::write(
		root.join("application/main.fa"),
		format!(
			"export function = (arguments: Array(String)): Integer => {{\n\tconsole.log(library.m{:04}.compute0(1))\n\t0\n}}\n",
			count - 1
		),
	)
	.unwrap();
}

/// The peak resident set size of this process, in bytes (Linux).
fn peak_rss() -> Option<usize> {
	let status = fs::read_to_string("/proc/self/status").ok()?;
	let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
	let kilobytes: usize = line
		.trim_start_matches("VmHWM:")
		.trim()
		.trim_end_matches("kB")
		.trim()
		.parse()
		.ok()?;
	Some(kilobytes * 1024)
}

#[test]
#[ignore = "generates and checks 2,000 files: run with --ignored, in release"]
fn checking_2000_files_stays_below_512_mb() {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("memory-2000");
	let _ = fs::remove_dir_all(&directory);
	generate(&directory, 2_000);
	let code = fa::main(vec!["check".into(), directory.as_os_str().to_owned()]);
	let _ = fs::remove_dir_all(&directory);
	assert_eq!(code, 0, "the generated package checks without errors");
	let peak = peak_rss().expect("the peak resident memory (Linux)");
	assert!(
		peak < 512 << 20,
		"`fa check` of 2,000 files peaked at {} MB",
		peak >> 20
	);
}
