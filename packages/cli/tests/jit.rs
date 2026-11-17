//! End-to-end tests of the JIT of the `native` target (design/decisions.md,
//! "Interpreter scope and JIT for native tests"): `fa run --target=native`
//! and `fa test --target=native` load the program in the process of
//! `fa-native` instead of linking an executable, and behave like the linked
//! executable that `FA_JIT=0` keeps.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A scratch folder of this test binary, with files.
fn scratch(name: &str, files: &[(&str, &str)]) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
		.join("jit")
		.join(name);
	let _ = fs::remove_dir_all(&directory);
	for (path, text) in files {
		let path = directory.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, text).unwrap();
	}
	directory
}

/// Runs `fa` in a folder, with the JIT (`jit`) or the linked executable,
/// and the trace of the JIT: `(stdout, stderr, exit code)`.
fn fa(arguments: &[&str], directory: &Path, jit: bool) -> (String, String, i32) {
	let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jit-cache");
	let mut command = Command::new(env!("CARGO_BIN_EXE_fa"));
	command
		.args(arguments)
		.current_dir(directory)
		.env("FA_CACHE", cache)
		.env("FA_JIT_TRACE", "1")
		.env_remove("CI")
		.env_remove("FA_TEST_TIMEOUT");
	if jit {
		command.env_remove("FA_JIT");
	} else {
		command.env("FA_JIT", "0");
	}
	let output = command.output().expect("`fa` runs");
	(
		String::from_utf8_lossy(&output.stdout).to_string(),
		String::from_utf8_lossy(&output.stderr).to_string(),
		output.status.code().unwrap_or(-1),
	)
}

/// The standard error without the trace of the JIT.
fn without_trace(stderr: &str) -> String {
	stderr
		.lines()
		.filter(|line| !line.starts_with("jit: "))
		.map(|line| format!("{line}\n"))
		.collect()
}

/// Whether the backend is there; else says why the test is skipped.
fn llvm() -> bool {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
	}
	fa_codegen_llvm::llvm_available()
}

#[test]
fn standalone_bulk_appends_keep_growing_header_caches_coherent_in_the_jit() {
	if !llvm() {
		return;
	}
	for (name, source, expected) in [
		(
			"fill",
			include_str!("programs/growing-nested-fill.fa"),
			"3\n4\n7\n8\ntrue\ntrue\ntrue\nfalse\ntrue\ntrue\ntrue\nfalse\n",
		),
		(
			"copy",
			include_str!("programs/growing-nested-copy.fa"),
			"3\n4\n7\n8\n10\n20\n30\n99\n10\n20\n30\n99\n",
		),
	] {
		let folder = scratch(&format!("bulk-growing-{name}"), &[("main.fa", source)]);
		for opt in ["-O0", "-O3"] {
			let arguments = [
				"run",
				"--target=native",
				"--asynchronous=false",
				opt,
				"main.fa",
			];
			let (stdout, stderr, code) = fa(&arguments, &folder, true);
			assert_eq!((stdout.as_str(), code), (expected, 0), "{name}: {stderr}");
			assert!(stderr.contains("jit: loaded 1 object in "), "{stderr}");
			assert_eq!(without_trace(&stderr), "");
			assert_eq!(
				fa(&arguments, &folder, false),
				(stdout, String::new(), code)
			);
		}
	}
}

#[test]
fn bulk_growing_appends_and_narrow_remainders_work_in_jit_and_cached_units() {
	if !llvm() {
		return;
	}
	let folder = scratch(
		"bulk-growing",
		&[
			("package.fa", "export = { name = \"bulkGrowing\" }\n"),
			(
				"library/nested.fa",
				include_str!(
					"../../../design/reviews/repros/perf-arrays/per-unit/library/nested.fa"
				),
			),
			(
				"library/remainder.fa",
				"export function = (value: UInt32): UInt32 => value %% 7\n",
			),
			(
				"application/main.fa",
				"export function = () => {\nlibrary.nested()\nassert library.remainder(UInt32.maxValue) == 3\nconsole.log(\"bulk jit ok\")\n}\n",
			),
		],
	);
	for opt in ["-O0", "-O3"] {
		for split in [false, true] {
			let mut arguments = vec!["run", "--target=native", opt];
			if !split {
				arguments.push("--asynchronous=false");
			}
			arguments.push(".");
			let (stdout, stderr, code) = fa(&arguments, &folder, true);
			assert_eq!((stdout.as_str(), code), ("bulk jit ok\n", 0), "{stderr}");
			let loaded = stderr
				.lines()
				.find_map(|line| {
					line.strip_prefix("jit: loaded ")?
						.split(' ')
						.next()?
						.parse::<usize>()
						.ok()
				})
				.expect("the JIT loads objects");
			assert!(if split { loaded > 2 } else { loaded == 1 }, "{stderr}");
			let cached = fa(&arguments, &folder, true);
			assert_eq!(
				(cached.0, without_trace(&cached.1), cached.2),
				(stdout.clone(), String::new(), code)
			);
			let linked = fa(&arguments, &folder, false);
			assert_eq!(linked, (stdout, String::new(), code));
		}
	}
}

#[test]
fn run_loads_a_file_in_memory_with_its_arguments() {
	if !llvm() {
		return;
	}
	let folder = scratch(
		"arguments",
		&[(
			"main.fa",
			"let first = try Integer.parse(arguments.get(0) ?? \"0\")\nconsole.log(first * 6)\nconsole.log(arguments.length)\n",
		)],
	);
	let arguments = ["run", "--target=native", "main.fa", "7", "seven"];
	let (stdout, stderr, code) = fa(&arguments, &folder, true);
	assert_eq!((stdout.as_str(), code), ("42\n2\n", 0), "{stderr}");
	assert!(stderr.contains("jit: loaded 1 object in "), "{stderr}");
	// `FA_JIT=0`: the linked executable, with the same result
	let linked = fa(&arguments, &folder, false);
	assert_eq!(linked, (stdout, String::new(), code));
}

#[test]
fn run_ends_with_the_code_and_the_message_of_a_stop() {
	if !llvm() {
		return;
	}
	let folder = scratch(
		"stop",
		&[(
			"main.fa",
			"let total = 2 + 3\nconsole.log(\"before\")\nassert total == 6\nconsole.log(\"after\")\n",
		)],
	);
	let arguments = ["run", "--target=native", "main.fa"];
	let (stdout, stderr, code) = fa(&arguments, &folder, true);
	assert_eq!((stdout.as_str(), code), ("before\n", 101), "{stderr}");
	assert!(stderr.contains("jit: loaded 1 object in "), "{stderr}");
	assert!(stderr.contains("stop: assertion failed"), "{stderr}");
	let linked = fa(&arguments, &folder, false);
	assert_eq!(linked, (stdout, without_trace(&stderr), code));
}

#[test]
fn run_loads_the_cached_objects_of_a_package() {
	if !llvm() {
		return;
	}
	let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/packages");
	let arguments = ["run", "--target=native", "-O0", "modules", "a", "b"];
	let (stdout, stderr, code) = fa(&arguments, &fixtures, true);
	// the units, the entry and the runtime
	let loaded = stderr.lines().find_map(|line| {
		line.strip_prefix("jit: loaded ")?
			.split(' ')
			.next()?
			.parse::<usize>()
			.ok()
	});
	assert!(loaded.is_some_and(|count| count > 2), "{stderr}");
	assert!(!stdout.is_empty(), "{stderr}");
	let linked = fa(&arguments, &fixtures, false);
	assert_eq!(linked, (stdout, without_trace(&stderr), code));
}

#[test]
fn tests_run_in_a_child_each() {
	if !llvm() {
		return;
	}
	let package = scratch(
		"tests",
		&[
			("package.fa", "export = { name = \"tests\" }\n"),
			(
				"library/counter.fa",
				"export namespace = {\n\tmutable count = 0\n\tfunction increment = () mutates count => {\n\t\tcount += 1\n\t}\n}\n",
			),
			(
				"library/counter.test.fa",
				concat!(
					"test \"counts\" {\n\tcounter.increment()\n\tcounter.increment()\n\tassert counter.count == 2\n}\n",
					"test \"starts again from zero\" {\n\tassert counter.count == 0\n}\n",
					"test \"fails\" {\n\tcounter.increment()\n\tassert counter.count == 5\n}\n",
					"test \"continues after a failed assert\" {\n\tassert counter.count == 0\n}\n",
				),
			),
			(
				"library/stops.test.fa",
				concat!(
					"test \"stops\" {\n\tmutable total: Int32 = 2147483647\n\ttotal += counter.count + 1\n\tassert total > 0\n}\n",
					"test \"is not run\" {\n\tassert 1 == 1\n}\n",
				),
			),
		],
	);
	let arguments = ["test", "--target=native"];
	let (stdout, stderr, code) = fa(&arguments, &package, true);
	assert!(
		stderr.contains("jit: loaded 1 object in "),
		"{stdout}{stderr}"
	);
	assert_eq!(code, 1, "{stdout}{stderr}");
	for expected in [
		"✓ counts\n",
		"✓ starts again from zero\n",
		"✗ fails (library/counter.test.fa:",
		"✓ continues after a failed assert\n",
		"✗ stops (library/stops.test.fa:",
		"- is not run (library/stops.test.fa:",
		"3 passed, 2 failed, 1 not run\n",
	] {
		assert!(
			stdout.contains(expected),
			"missing {expected:?} in:\n{stdout}{stderr}"
		);
	}
	let linked = fa(&arguments, &package, false);
	assert_eq!(linked, (stdout, without_trace(&stderr), code));
}

/// The JIT lives in `fa-native`: `fa` handles a loaded program as a trait
/// object, and must not load `libLLVM` for it (design/decisions.md, "`fa`
/// doesn't link LLVM").
#[test]
fn fa_itself_does_not_load_llvm() {
	if !llvm() {
		return;
	}
	let Ok(output) = Command::new("readelf")
		.arg("-d")
		.arg(env!("CARGO_BIN_EXE_fa"))
		.output()
	else {
		eprintln!("skipped: `readelf` is not installed");
		return;
	};
	if !output.status.success() {
		eprintln!("skipped: `readelf` can't read this executable");
		return;
	}
	let dynamic = String::from_utf8_lossy(&output.stdout);
	assert!(!dynamic.contains("LLVM"), "`fa` loads LLVM:\n{dynamic}");
}

/// Exercise the storage and return ABIs added by the merged native lowering.
#[test]
fn loaded_and_linked_storage_and_return_destinations_agree() {
	if !llvm() {
		return;
	}
	let programs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../codegen-llvm/tests/programs");
	for name in [
		"table-storage",
		"aggregate-return-destinations",
		"collection-return-destinations",
		"return-local-ownership",
	] {
		let source = fs::read_to_string(programs.join(format!("{name}.fa"))).unwrap();
		let folder = scratch(&format!("storage-{name}"), &[("main.fa", &source)]);
		for level in ["-O0", "-O3"] {
			let arguments = ["run", "--target=native", level, "main.fa"];
			let (stdout, stderr, code) = fa(&arguments, &folder, true);
			assert_eq!(code, 0, "{name} {level}: {stderr}");
			assert!(!stdout.is_empty(), "{name} {level}: {stderr}");
			assert!(stderr.contains("jit: loaded "), "{name} {level}: {stderr}");
			let linked = fa(&arguments, &folder, false);
			assert_eq!(
				linked,
				(stdout, without_trace(&stderr), code),
				"{name} {level}"
			);
		}
	}
}
