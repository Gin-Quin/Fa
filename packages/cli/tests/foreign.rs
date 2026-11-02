//! Foreign code through the `fa` command: a package binding a small C
//! library (`packages/codegen-llvm/tests/foreign/farm.c`, compiled here into
//! a shared library with `cc`) through `build.link` and
//! `build.libraryPaths`, run on Bun (`bun:ffi`); `fa build --target=node`
//! refuses C calls (`FFI008`); `fa bind` writes a foreign file from a header.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs `fa` with arguments in a folder: `(stdout, stderr, exit code)`.
fn fa(arguments: &[&str], directory: &Path, cache: &Path) -> (String, String, i32) {
	let output = Command::new(env!("CARGO_BIN_EXE_fa"))
		.args(arguments)
		.current_dir(directory)
		.env("XDG_CACHE_HOME", cache)
		.output()
		.expect("`fa` runs");
	(
		String::from_utf8_lossy(&output.stdout).to_string(),
		String::from_utf8_lossy(&output.stderr).to_string(),
		output.status.code().unwrap_or(-1),
	)
}

fn has_bun() -> bool {
	std::env::var_os("PATH").is_some_and(|path| {
		std::env::split_paths(&path).any(|directory| directory.join("bun").is_file())
	}) || std::env::var_os("HOME")
		.is_some_and(|home| PathBuf::from(home).join(".bun/bin/bun").is_file())
}

/// A scratch folder of this test binary.
fn scratch(name: &str) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	let _ = fs::remove_dir_all(&directory);
	fs::create_dir_all(&directory).unwrap();
	directory
}

/// The package: `library/Farm.foreign.fa` binds `libfarm.so`, the
/// application only sees a safe `Farm` type.
fn write_package(directory: &Path, targets: &str) {
	let files = [
		(
			"package.fa",
			format!(
				"export = {{\n\tname = \"farm\"\n\ttargets = [{targets}]\n\tbuild = {{ link = [\"farm\"], libraryPaths = [\"native\"] }}\n}}\n"
			),
		),
		(
			"library/Farm.foreign.fa",
			"use c = std.c

type RawFarm = CStruct { }

function openFarm: (name: CString) => Pointer(RawFarm) = @extern(\"farm_open\")
function addAnimals: (farm: Pointer(RawFarm), count: Int32) => Void = @extern(\"farm_add\")
function animals: (farm: Pointer(RawFarm)) => Int32 = @extern(\"farm_animals\")
function farmName: (farm: Pointer(RawFarm)) => CString = @extern(\"farm_name\")
function closeFarm: (farm: Pointer(RawFarm)) => Void = @extern(\"farm_close\")
function increment: (counter: Pointer(Int64), step: Int64) => Void mutates counter = @extern(\"counter_increment\")

export type = {
	#raw: Pointer(RawFarm)

	function open = (name: String): Self => Self { #raw = openFarm(name) }

	function add = (self, count: Int32) => {
		addAnimals(self.#raw, count)
	}

	function count = (self): Int32 => animals(self.#raw)

	function name = (self): String => farmName(self.#raw).toString()

	function counted = (start: Int64): Int64 => {
		mutable counter = start
		increment(counter, 2)
		counter
	}

	function destroy = (self) => {
		closeFarm(self.#raw)
	}
}
"
			.to_string(),
		),
		(
			"application/main.fa",
			"export function = () => {
	let farm = library.Farm.open(\"Green Acres\")
	farm.add(3)
	farm.add(4)
	console.log(farm.count())
	console.log(farm.name())
	console.log(library.Farm.counted(40))
}
"
			.to_string(),
		),
	];
	for (path, text) in files {
		let path = directory.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, text).unwrap();
	}
}

/// Compiles `farm.c` into `native/libfarm.so`: whether a C compiler did it.
fn compile_library(directory: &Path) -> bool {
	let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../codegen-llvm/tests/foreign/farm.c");
	let native = directory.join("native");
	fs::create_dir_all(&native).unwrap();
	Command::new(std::env::var_os("CC").unwrap_or_else(|| "cc".into()))
		.args(["-shared", "-fPIC", "-O1", "-o"])
		.arg(native.join("libfarm.so"))
		.arg(source)
		.status()
		.is_ok_and(|status| status.success())
}

#[test]
fn a_package_calls_its_c_library_on_bun() {
	if !cfg!(target_os = "linux") || !has_bun() {
		eprintln!("skipped: needs Linux and Bun");
		return;
	}
	let directory = scratch("foreign-package");
	write_package(&directory, "\"bun\", \"native\"");
	if !compile_library(&directory) {
		eprintln!("skipped: no C compiler (`cc`)");
		return;
	}
	let cache = scratch("foreign-cache");
	let (stdout, stderr, code) = fa(&["run", "."], &directory, &cache);
	assert_eq!(code, 0, "stderr: {stderr}");
	assert_eq!(stdout, "7\nGreen Acres\n42\n");
	let (_, stderr, code) = fa(&["check"], &directory, &cache);
	assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn node_and_the_web_cant_call_c() {
	let directory = scratch("foreign-node");
	write_package(&directory, "\"node\"");
	let cache = scratch("foreign-node-cache");
	let (_, stderr, code) = fa(&["build", "--target=node"], &directory, &cache);
	assert_eq!(code, 1);
	assert!(stderr.contains("FFI008"), "{stderr}");
	assert!(stderr.contains("target `node`"), "{stderr}");
}

#[test]
fn bind_writes_a_foreign_file() {
	let directory = scratch("foreign-bind");
	fs::write(
		directory.join("counter.h"),
		"#include <stdint.h>\n#define COUNTER_MAX 10\ntypedef struct counter counter;\ncounter *counter_new(const char *name);\nvoid counter_add(counter *c, int64_t step);\nint64_t counter_value(const counter *c);\n",
	)
	.unwrap();
	let cache = scratch("foreign-bind-cache");
	let (_, stderr, code) = fa(
		&["bind", "counter.h", "--output", "counter.foreign.fa"],
		&directory,
		&cache,
	);
	assert_eq!(code, 0, "{stderr}");
	let written = fs::read_to_string(directory.join("counter.foreign.fa")).unwrap();
	assert!(written.contains("@extern(\"counter_new\")"), "{written}");
	// the generated file is checked by the analyzer
	let (_, stderr, code) = fa(&["check", "counter.foreign.fa"], &directory, &cache);
	assert_eq!(code, 0, "{stderr}\n{written}");
}
