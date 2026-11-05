//! A Fa package built as a C library (`build.cLibrary`, foreign-code.md,
//! "Exposing a library to C"): `fa build` of `tests/c-library/farm/` writes
//! `libfarm.a`, `libfarm.so` and `farm.h`; the C program `main.c`, compiled
//! against the header with `-std=c11 -Wall -Wextra -Werror -pedantic`, is
//! linked statically and dynamically, run (under Valgrind when it is
//! installed: no invalid access, no definite leak), and its output compared
//! with `expected.txt`. Per unit with LTO (the default), per unit without
//! LTO (`-O0`), and as one module (`FA_SPLIT=0`). The checks of the API
//! (`FFI012`, `FFI013`, `FFI014`, `REQ006`) are run by `fa check`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The fixture: the package and the C program.
fn fixture() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/c-library")
}

/// A scratch folder of this test binary.
fn scratch(name: &str) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	let _ = fs::remove_dir_all(&directory);
	fs::create_dir_all(&directory).unwrap();
	directory
}

/// Runs `fa` (`fa-native` for the native target) in a folder:
/// `(stdout + stderr, exit code)`.
fn fa(arguments: &[&str], directory: &Path, environment: &[(&str, &str)]) -> (String, i32) {
	let mut command = Command::new(env!("CARGO_BIN_EXE_fa-native"));
	command
		.args(arguments)
		.current_dir(directory)
		.env("XDG_CACHE_HOME", directory.join(".cache"))
		.env("FA_CACHE", directory.join(".cache/fa"));
	for (name, value) in environment {
		command.env(name, value);
	}
	let output = command.output().expect("`fa-native` runs");
	(
		format!(
			"{}{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		),
		output.status.code().unwrap_or(-1),
	)
}

/// The C compiler: the clang of the LLVM install the backend was built
/// with, else `cc`.
fn c_compiler() -> PathBuf {
	option_env!("LLVM_SYS_181_PREFIX")
		.map(|prefix| Path::new(prefix).join("bin/clang"))
		.filter(|clang| clang.is_file())
		.unwrap_or_else(|| PathBuf::from("cc"))
}

fn has_valgrind() -> bool {
	Command::new("valgrind")
		.arg("--version")
		.output()
		.is_ok_and(|output| output.status.success())
}

/// Compiles `main.c` against the library in `directory`, links it
/// statically and dynamically, runs both and compares their output.
fn check_c_program(directory: &Path) {
	let header = directory.join("farm.h");
	assert!(header.is_file(), "the header is written");
	let main = fixture().join("main.c");
	let compile = |output: &Path, library: &[&str]| {
		let status = Command::new(c_compiler())
			.args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
			.arg("-I")
			.arg(directory)
			.arg(&main)
			.args(library)
			.arg("-o")
			.arg(output)
			.status()
			.expect("the C compiler runs");
		assert!(
			status.success(),
			"main.c compiles against farm.h ({library:?})"
		);
	};
	let archive = directory.join("libfarm.a");
	let statically = directory.join("static");
	compile(&statically, &[archive.to_str().unwrap(), "-lm", "-pthread"]);
	let dynamically = directory.join("dynamic");
	let search = format!("-L{}", directory.display());
	let rpath = format!("-Wl,-rpath,{}", directory.display());
	compile(&dynamically, &[&search, "-lfarm", &rpath, "-pthread"]);
	let expected = fs::read_to_string(fixture().join("expected.txt")).unwrap();
	let valgrind = has_valgrind();
	for program in [&statically, &dynamically] {
		let output = if valgrind {
			Command::new("valgrind")
				.args([
					"-q",
					"--leak-check=full",
					"--errors-for-leak-kinds=definite",
					"--error-exitcode=3",
				])
				.arg(program)
				.output()
		} else {
			Command::new(program).output()
		}
		.expect("the C program runs");
		let stdout = String::from_utf8_lossy(&output.stdout);
		assert_eq!(
			output.status.code(),
			Some(0),
			"{} exits normally: {}",
			program.display(),
			String::from_utf8_lossy(&output.stderr)
		);
		assert_eq!(stdout, expected, "the output of {}", program.display());
	}
	// only the API is visible in the shared library
	let symbols = Command::new("nm")
		.args(["-D", "--defined-only"])
		.arg(directory.join("libfarm.so"))
		.output();
	if let Ok(symbols) = symbols
		&& symbols.status.success()
	{
		let text = String::from_utf8_lossy(&symbols.stdout);
		for line in text.lines() {
			let name = line.split_whitespace().last().unwrap_or("");
			assert!(
				name.starts_with("farm_"),
				"only the API is exported, not `{name}`"
			);
		}
		assert!(text.contains("farm_last_error_message"));
	}
	// loaded with `dlopen` after the program's start (as `ctypes` does)
	let loader = directory.join("loader");
	let status = Command::new(c_compiler())
		.args(["-std=c11", "-Wall", "-Werror"])
		.arg(fixture().join("dlopen.c"))
		.args(["-ldl", "-o"])
		.arg(&loader)
		.status()
		.expect("the C compiler runs");
	assert!(status.success(), "dlopen.c compiles");
	let output = Command::new(&loader)
		.arg(directory.join("libfarm.so"))
		.output()
		.expect("the loader runs");
	assert_eq!(
		String::from_utf8_lossy(&output.stdout),
		"30 1 MathError.DivisionByZero\ninternal: hidden\nunloaded\n"
	);
	assert_eq!(output.status.code(), Some(0));
}

/// Copies the fixture's package into a scratch folder.
fn package_in(directory: &Path) -> PathBuf {
	let package = directory.join("farm");
	copy_folder(&fixture().join("farm"), &package);
	package
}

fn copy_folder(from: &Path, to: &Path) {
	fs::create_dir_all(to).unwrap();
	for entry in fs::read_dir(from).unwrap() {
		let entry = entry.unwrap();
		let target = to.join(entry.file_name());
		if entry.file_type().unwrap().is_dir() {
			copy_folder(&entry.path(), &target);
		} else {
			fs::copy(entry.path(), &target).unwrap();
		}
	}
}

/// Builds the library with these options and checks the C program.
fn build_and_check(name: &str, options: &[&str], environment: &[(&str, &str)]) {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory = scratch(name);
	let package = package_in(&directory);
	let output = directory.join("out");
	let output_option = format!("--output={}", output.display());
	let mut arguments = vec!["build", "--target=native"];
	arguments.extend_from_slice(options);
	arguments.push(&output_option);
	arguments.push("farm");
	let (text, code) = fa(&arguments, &directory, environment);
	assert_eq!(code, 0, "fa build succeeds: {text}");
	assert!(
		text.contains("libfarm.so") && text.contains("libfarm.a") && text.contains("farm.h"),
		"{text}"
	);
	assert!(package.join("library/capi.foreign.fa").is_file());
	check_c_program(&output.join("native"));
}

#[test]
fn a_c_library_built_per_unit_with_lto_is_used_from_c() {
	build_and_check("c-library-lto", &[], &[]);
}

#[test]
fn a_c_library_built_per_unit_without_lto_is_used_from_c() {
	build_and_check("c-library-o0", &["-O0"], &[]);
}

#[test]
fn a_c_library_built_as_one_module_is_used_from_c() {
	build_and_check("c-library-module", &[], &[("FA_SPLIT", "0")]);
}

/// Writes a package whose API is `api`, and runs `fa check` on it for
/// `targets`: its output.
fn check_api(name: &str, targets: &str, api: &str) -> (String, i32) {
	let directory = scratch(name);
	let package = directory.join("lib");
	fs::create_dir_all(package.join("library")).unwrap();
	fs::write(
		package.join("package.fa"),
		format!(
			"export = {{\n\tname = \"lib\"\n\ttargets = [{targets}]\n\tbuild = {{\n\t\tcLibrary = {{ header = \"lib.h\", prefix = \"lib\", api = \"library/api.foreign.fa\" }}\n\t}}\n}}\n"
		),
	)
	.unwrap();
	fs::write(package.join("library/api.foreign.fa"), api).unwrap();
	let output = Command::new(env!("CARGO_BIN_EXE_fa"))
		.args(["check", "lib"])
		.current_dir(&directory)
		.env("XDG_CACHE_HOME", directory.join(".cache"))
		.output()
		.expect("`fa` runs");
	(
		format!(
			"{}{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		),
		output.status.code().unwrap_or(-1),
	)
}

#[test]
fn the_api_of_a_c_library_is_checked() {
	// C types only at the boundary
	let (text, code) = check_api(
		"c-library-types",
		"\"native\"",
		"export namespace = {\n\tfunction count = (names: Array<String>): Integer => names.length\n}\n",
	);
	assert_eq!(code, 1, "{text}");
	assert!(
		text.contains("FFI014") && text.contains("`names`"),
		"{text}"
	);
	assert!(text.contains("returns an `Integer`"), "{text}");
	// a namespace
	let (text, code) = check_api(
		"c-library-export",
		"\"native\"",
		"export function = (): Int32 => 1\n",
	);
	assert_eq!(code, 1, "{text}");
	assert!(
		text.contains("FFI014") && text.contains("namespace"),
		"{text}"
	);
	// no requirements
	let (text, code) = check_api(
		"c-library-requires",
		"\"native\"",
		"export namespace = {\n\tfunction scaled = (value: Int32): Int32 requires factor: Int32 => value * factor\n}\n",
	);
	assert_eq!(code, 1, "{text}");
	assert!(text.contains("REQ006"), "{text}");
	// no shared mutable state: C calls from any thread
	let (text, code) = check_api(
		"c-library-mutable",
		"\"native\"",
		"mutable calls: Int32 = 0\n\nfunction count = (): Int32 => {\n\tcalls += 1\n\tcalls\n}\n\nexport namespace = {\n\tfunction next = (): Int32 => count()\n}\n",
	);
	assert_eq!(code, 1, "{text}");
	assert!(
		text.contains("FFI012") && text.contains("`calls`") && text.contains("`count`"),
		"{text}"
	);
	// native only
	let (text, code) = check_api(
		"c-library-bun",
		"\"bun\", \"native\"",
		"export namespace = {\n\tfunction one = (): Int32 => 1\n}\n",
	);
	assert_eq!(code, 1, "{text}");
	assert!(text.contains("FFI013"), "{text}");
	// a valid API
	let (text, code) = check_api(
		"c-library-valid",
		"\"native\"",
		"type Pair = CStruct {\n\ta: Int32\n\tb: Float64\n}\n\ntype Failure = Errors {\n\tNegative\n}\n\nexport namespace = {\n\tfunction swap = (pair: Pair): Pair => Pair { a = 1, b = pair.b }\n\tfunction check = (value: Int64): Int64 throws Failure => {\n\t\tif value < 0 {\n\t\t\tthrow Failure.Negative\n\t\t}\n\t\tvalue\n\t}\n\tfunction text = (value: CString?): Boolean => value != none\n}\n",
	);
	assert_eq!(code, 0, "{text}");
}
