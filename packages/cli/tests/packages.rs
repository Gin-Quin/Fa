//! End-to-end tests of multi-file programs (advanced brief §15, §18): the
//! package fixtures of `tests/packages/` run through `fa run <package>` and
//! `fa test <package>`, `fa new package`, and the multi-file cases of the
//! analyzer (`-- file:` sections) that say what they print, written into a
//! folder and run through `fa run`.
//!
//! Bun is found on the `PATH` or in `~/.bun/bin`; without it, the tests
//! print a message and pass.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs `fa` with arguments in a folder: `(stdout, stderr, exit code)`.
fn fa(arguments: &[&str], directory: &Path, cache: &Path) -> (String, String, i32) {
	fa_with(arguments, directory, cache, &[])
}

/// `fa` with environment variables.
fn fa_with(
	arguments: &[&str],
	directory: &Path,
	cache: &Path,
	variables: &[(&str, &str)],
) -> (String, String, i32) {
	let output = Command::new(env!("CARGO_BIN_EXE_fa"))
		.args(arguments)
		.envs(variables.iter().copied())
		.current_dir(directory)
		.env("XDG_CACHE_HOME", cache)
		.env_remove("CI")
		.output()
		.expect("`fa` runs");
	(
		String::from_utf8_lossy(&output.stdout).to_string(),
		String::from_utf8_lossy(&output.stderr).to_string(),
		output.status.code().unwrap_or(-1),
	)
}

/// Whether Bun is installed (the tests pass without it).
fn has_bun() -> bool {
	let found = fa_bun();
	if !found {
		eprintln!("Bun is not installed: the multi-file programs are not run");
	}
	found
}

fn fa_bun() -> bool {
	let name = if cfg!(windows) { "bun.exe" } else { "bun" };
	std::env::var_os("PATH").is_some_and(|path| {
		std::env::split_paths(&path).any(|directory| directory.join(name).is_file())
	}) || std::env::var_os("HOME")
		.is_some_and(|home| PathBuf::from(home).join(".bun/bin").join(name).is_file())
}

/// A scratch folder of this test binary.
fn scratch(name: &str) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	let _ = fs::remove_dir_all(&directory);
	fs::create_dir_all(&directory).unwrap();
	directory
}

fn fixtures() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/packages")
}

#[test]
fn a_package_runs_its_application() {
	if !has_bun() {
		return;
	}
	let cache = scratch("packages-run");
	let (stdout, stderr, code) = fa(&["run", "modules", "a", "b"], &fixtures(), &cache);
	assert_eq!(
		stdout, "1 2 3\nsibling x root y bar [a]\n2 11\n11\n0 [hello Ada]\n5 11 3 8\n2\n",
		"stderr: {stderr}"
	);
	assert_eq!(code, 3, "the entry function's exit code; stderr: {stderr}");
}

/// An async function of another module (design/async.md §4): awaited,
/// started and called blocking, on the streaming pipeline and on the
/// sequential one (an asynchronous mode given).
#[test]
fn a_package_awaits_the_functions_of_its_modules() {
	if !has_bun() {
		return;
	}
	let cache = scratch("packages-tasks");
	for arguments in [
		&["run", "tasks"][..],
		&["run", "--asynchronous=true", "tasks"],
	] {
		let (stdout, stderr, code) = fa(arguments, &fixtures(), &cache);
		assert_eq!(
			stdout, "hello awaited\nhello fast\nhello slow\nhello blocking\n",
			"{arguments:?}, stderr: {stderr}"
		);
		assert_eq!(code, 2, "{arguments:?}, stderr: {stderr}");
	}
}

/// Tests that await run as tasks (`faTestTask`): a started task runs while
/// the test waits; on the streaming pipeline and on the sequential one.
#[test]
fn a_package_runs_its_async_tests() {
	if !has_bun() {
		return;
	}
	let cache = scratch("packages-async-tests");
	for arguments in [
		&["test", "tasks"][..],
		&["test", "--asynchronous=true", "tasks"],
	] {
		let (stdout, stderr, code) = fa(arguments, &fixtures(), &cache);
		assert_eq!(
			stdout,
			"before\ntick\nafter\n✓ a started task runs while the test awaits\n✓ awaits a function of another module\n✓ a blocking test\n\n3 passed, 0 failed\n",
			"{arguments:?}, stderr: {stderr}"
		);
		assert_eq!(code, 0, "{arguments:?}, stderr: {stderr}");
	}
}

/// The same package compiled by the LLVM backend: natively, and to wasm
/// when Node or Bun runs it.
#[test]
fn a_package_runs_natively() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let cache = scratch("packages-native");
	for target in ["native", "wasm"] {
		if target == "wasm" && fa_codegen_llvm::javascript_runtime().is_err() {
			continue;
		}
		let option = format!("--target={target}");
		let (stdout, stderr, code) =
			fa(&["run", &option, "modules", "a", "b"], &fixtures(), &cache);
		assert_eq!(
			stdout, "1 2 3\nsibling x root y bar [a]\n2 11\n11\n0 [hello Ada]\n5 11 3 8\n2\n",
			"{target}, stderr: {stderr}"
		);
		assert_eq!(
			code, 3,
			"{target}: the entry function's exit code; stderr: {stderr}"
		);
	}
}

/// Maps whose layout depends on the whole program (dense entries, entries in
/// the probe array): every unit of a build per unit takes it from the facts
/// merged from all units (`fa-codegen-llvm`, `facts.rs`), so that a map made
/// in one unit and changed in another agrees with Bun.
#[test]
fn units_agree_on_the_layout_of_maps() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let cache = scratch("packages-maps");
	for target in ["native", "bun"] {
		if target == "bun" && !fa_bun() {
			continue;
		}
		let option = format!("--target={target}");
		let (stdout, stderr, code) = fa(&["run", &option, "maps"], &fixtures(), &cache);
		assert_eq!(
			stdout, "2 405 3 4 49 2442\n30 529\n",
			"{target}, stderr: {stderr}"
		);
		assert_eq!(code, 0, "{target}, stderr: {stderr}");
	}
}

/// Never-negative fields in a build per unit (`fa-codegen-llvm`,
/// `proofs.rs`): a field counted up from zero in its own unit is proven, one
/// stored negative in another unit is not, and `get` counts it from the end.
#[test]
fn units_agree_on_never_negative_fields() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let cache = scratch("packages-cursors");
	for target in ["native", "bun"] {
		if target == "bun" && !fa_bun() {
			continue;
		}
		let option = format!("--target={target}");
		let (stdout, stderr, code) = fa(&["run", &option, "cursors"], &fixtures(), &cache);
		assert_eq!(stdout, "294 122\n", "{target}, stderr: {stderr}");
		assert_eq!(code, 0, "{target}, stderr: {stderr}");
	}
}

/// A package compiled per unit (design/parallel-pipeline.md §3.3): the
/// objects of its units (the cache of `fa run`) and its executable
/// (`fa build`, ThinLTO) are the same whatever the threads and the order
/// of the units.
#[test]
fn native_objects_are_the_same_whatever_the_threads() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let mut objects = Vec::new();
	let mut executables = Vec::new();
	for (threads, seed) in [("1", "0"), ("4", "7")] {
		let cache = scratch(&format!("packages-determinism-{threads}"));
		let variables = [("RAYON_NUM_THREADS", threads), ("FA_SCHEDULE_SEED", seed)];
		let (stdout, stderr, code) = fa_with(
			&["run", "--target=native", "modules", "a", "b"],
			&fixtures(),
			&cache,
			&variables,
		);
		assert_eq!(code, 3, "stdout: {stdout}, stderr: {stderr}");
		let directory = cache.join("fa/native/objects");
		let mut files: Vec<(String, Vec<u8>)> = fs::read_dir(&directory)
			.unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
			.map(|entry| {
				let path = entry.unwrap().path();
				let name = path.file_name().unwrap().to_string_lossy().into_owned();
				(name, fs::read(&path).unwrap())
			})
			.collect();
		files.sort();
		assert!(!files.is_empty(), "no object in {}", directory.display());
		objects.push(files);
		let output = cache.join("out");
		let (_, stderr, code) = fa_with(
			&[
				"build",
				"--target=native",
				&format!("--output={}", output.display()),
				"modules",
			],
			&fixtures(),
			&cache,
			&variables,
		);
		assert_eq!(code, 0, "stderr: {stderr}");
		executables.push(fs::read(output.join("native/main")).unwrap());
	}
	assert!(objects[0] == objects[1], "the objects differ");
	assert!(executables[0] == executables[1], "the executables differ");
}

/// A release build of a package links with full LTO when its estimated
/// memory fits `FA_LLVM_MEMORY`, with ThinLTO above it; `--verbose` says
/// which (design/decisions.md, "Full LTO when it fits the memory budget").
#[test]
fn a_native_build_chooses_its_lto_by_the_memory_budget() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let cache = scratch("packages-lto");
	for (budget, expected) in [
		("512", "linked with full LTO"),
		("64", "linked with ThinLTO"),
	] {
		let output = cache.join(format!("out-{budget}"));
		let (stdout, stderr, code) = fa_with(
			&[
				"build",
				"--target=native",
				"--verbose",
				&format!("--output={}", output.display()),
				"modules",
			],
			&fixtures(),
			&cache,
			&[("FA_LLVM_MEMORY", budget)],
		);
		assert_eq!(code, 0, "stderr: {stderr}");
		assert!(stderr.contains(expected), "{budget} MB: {stderr}");
		let run = std::process::Command::new(output.join("native/main"))
			.args(["a", "b"])
			.output()
			.expect("the program runs");
		assert_eq!(
			String::from_utf8_lossy(&run.stdout),
			"1 2 3\nsibling x root y bar [a]\n2 11\n11\n0 [hello Ada]\n5 11 3 8\n2\n",
			"{budget} MB, stdout of the build: {stdout}"
		);
	}
}

/// `fa build --target=native` of a package writes into
/// `<package>/<build.output>/native/`, wherever it runs from.
#[test]
fn a_native_build_writes_into_the_output_of_the_package() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let cache = scratch("packages-native-output");
	let root = scratch("packages-native-output-package");
	let (_, stderr, code) = fa(&["new", "package", "farm"], &root, &cache);
	assert_eq!(code, 0, "{stderr}");
	let package = root.join("farm");
	fs::write(
		package.join("package.fa"),
		"export = {\n\tname = \"farm\"\n\tbuild = { output = \"out\" }\n}\n",
	)
	.unwrap();
	let (_, stderr, code) = fa(
		&["build", "--target=native"],
		&package.join("library"),
		&cache,
	);
	assert_eq!(code, 0, "{stderr}");
	assert!(package.join("out/native/main").is_file(), "{stderr}");
	assert!(!package.join("library/build").exists(), "{stderr}");
	let (_, stderr, code) = fa(&["build", "farm", "--target=native"], &root, &cache);
	assert_eq!(code, 0, "{stderr}");
	assert!(!root.join("build").exists(), "{stderr}");
}

/// `fa build` of a package without `--target` builds every target of its
/// configuration, and reports each one.
#[test]
fn a_package_builds_for_each_of_its_targets() {
	let cache = scratch("packages-targets");
	let root = scratch("packages-targets-package");
	let (_, stderr, code) = fa(&["new", "package", "farm"], &root, &cache);
	assert_eq!(code, 0, "{stderr}");
	let package = root.join("farm");
	fs::write(
		package.join("package.fa"),
		"export = {\n\tname = \"farm\"\n\ttargets = [\"web\", \"node\"]\n}\n",
	)
	.unwrap();
	let output = scratch("packages-targets-output");
	let option = format!("--output={}", output.display());
	let (_, stderr, code) = fa(&["build", &option, "--minify=false"], &package, &cache);
	assert_eq!(code, 0, "{stderr}");
	assert!(output.join("web/main.js").is_file(), "{stderr}");
	assert!(output.join("node/main.js").is_file(), "{stderr}");
	assert!(!output.join("bun").exists(), "{stderr}");
	assert!(
		stderr.contains("fa build: `web` built, `node` built"),
		"{stderr}"
	);
	// without `--output`: `<package>/build/<target>/`
	let (_, stderr, code) = fa(&["build", "--minify=false"], &package, &cache);
	assert_eq!(code, 0, "{stderr}");
	assert!(package.join("build/web/main.js").is_file(), "{stderr}");
	assert!(package.join("build/node/main.js").is_file(), "{stderr}");
}

/// `fa build` of a standalone file without `--target` builds the default
/// targets, like a package without `targets`.
#[test]
fn a_standalone_file_builds_for_the_default_targets() {
	let cache = scratch("packages-standalone-targets");
	let root = scratch("packages-standalone-targets-root");
	fs::write(root.join("main.fa"), "console.log(\"hi\")\n").unwrap();
	let (_, stderr, _) = fa(&["build", "main.fa", "--minify=false"], &root, &cache);
	assert!(stderr.contains("fa build: target `web`"), "{stderr}");
	assert!(stderr.contains("fa build: target `native`"), "{stderr}");
	assert!(root.join("build/web/main.js").is_file(), "{stderr}");
	assert!(root.join("build/bun/main.js").is_file(), "{stderr}");
	if fa_codegen_llvm::llvm_available() {
		assert!(
			stderr.contains("fa build: `web` built, `bun` built, `native` built"),
			"{stderr}"
		);
		assert!(root.join("build/native/main").is_file(), "{stderr}");
	} else {
		assert!(stderr.contains("`native` failed"), "{stderr}");
	}
}

/// A script of a package built without `--target` builds its package's
/// `targets`.
#[test]
fn a_script_builds_for_the_targets_of_its_package() {
	let cache = scratch("packages-script-targets");
	let root = scratch("packages-script-targets-root");
	let (_, stderr, code) = fa(&["new", "package", "farm"], &root, &cache);
	assert_eq!(code, 0, "{stderr}");
	let package = root.join("farm");
	fs::write(
		package.join("package.fa"),
		"export = {\n\tname = \"farm\"\n\ttargets = [\"web\", \"node\"]\n}\n",
	)
	.unwrap();
	fs::create_dir_all(package.join("scripts")).unwrap();
	fs::write(package.join("scripts/seed.fa"), "console.log(\"seed\")\n").unwrap();
	let output = scratch("packages-script-targets-output");
	let option = format!("--output={}", output.display());
	let arguments = ["build", "scripts/seed.fa", &option, "--minify=false"];
	let (_, stderr, code) = fa(&arguments, &package, &cache);
	assert_eq!(code, 0, "{stderr}");
	assert!(
		stderr.contains("fa build: `web` built, `node` built"),
		"{stderr}"
	);
	assert!(output.join("web/main.js").is_file(), "{stderr}");
	assert!(output.join("node/main.js").is_file(), "{stderr}");
	assert!(!output.join("bun").exists(), "{stderr}");
}

/// A standalone program using a foreign file is built by default for the
/// targets that can call C only.
#[test]
fn a_standalone_program_calling_c_leaves_out_the_web() {
	let cache = scratch("packages-standalone-foreign");
	let root = scratch("packages-standalone-foreign-root");
	fs::write(
		root.join("clib.foreign.fa"),
		"function absolute: (value: Int32) => Int32 = @extern(\"abs\")\n\nexport function = (value: Int32): Int32 => absolute(value)\n",
	)
	.unwrap();
	fs::write(root.join("main.fa"), "console.log(clib(-3))\n").unwrap();
	let (_, stderr, _) = fa(&["build", "main.fa", "--minify=false"], &root, &cache);
	assert!(stderr.contains("fa build: target `bun`"), "{stderr}");
	assert!(stderr.contains("fa build: target `native`"), "{stderr}");
	assert!(!stderr.contains("target `web`"), "{stderr}");
	assert!(!root.join("build/web").exists(), "{stderr}");
}

/// `fa new package` refuses a name that isn't an identifier (it would
/// break `package.fa`).
#[test]
fn a_new_package_needs_an_identifier() {
	let cache = scratch("packages-new-name");
	let root = scratch("packages-new-name-root");
	for name in ["my\"farm", "farm{x}", "2farm", "my-farm"] {
		let (_, stderr, code) = fa(&["new", "package", name], &root, &cache);
		assert_eq!(code, 1, "{name}: {stderr}");
		assert!(stderr.contains("error[CLI008]"), "{name}: {stderr}");
		assert!(!root.join(name).exists(), "{name}");
	}
	let (_, stderr, code) = fa(&["new", "package", "nested/my_farm"], &root, &cache);
	assert_eq!(code, 0, "{stderr}");
	assert!(root.join("nested/my_farm/package.fa").is_file());
}

/// `fa format` without a path formats the closest package, else the
/// workspace.
#[test]
fn format_without_a_path_formats_the_closest_package() {
	let cache = scratch("packages-format");
	let root = scratch("packages-format-workspace");
	for (path, text) in [
		("workspace.fa", "export = { name = \"w\" }\n"),
		("packages/a/package.fa", "export = { name = \"a\" }\n"),
		("packages/a/library/x.fa", "export = 1+2\n"),
		("packages/b/package.fa", "export = { name = \"b\" }\n"),
		("packages/b/library/y.fa", "export = 1+2\n"),
	] {
		let path = root.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, text).unwrap();
	}
	let (stdout, stderr, code) = fa(
		&["format", "--check"],
		&root.join("packages/a/library"),
		&cache,
	);
	assert_eq!(code, 1, "{stderr}");
	assert!(stdout.contains("x.fa"), "{stdout}");
	assert!(
		!stdout.contains("y.fa"),
		"only the closest package: {stdout}"
	);
	let (stdout, _, code) = fa(&["format", "--check"], &root.join("packages"), &cache);
	assert_eq!(code, 1);
	assert!(
		stdout.contains("x.fa") && stdout.contains("y.fa"),
		"the workspace: {stdout}"
	);
}

/// `fa check --format=json`: one JSON object per diagnostic on stdout.
#[test]
fn check_prints_json_diagnostics() {
	let cache = scratch("packages-json");
	let root = scratch("packages-json-root");
	fs::write(root.join("main.fa"), "let a = 1\nlet b: Int8 = 300\n").unwrap();
	let (stdout, stderr, code) = fa(&["check", "main.fa", "--format=json"], &root, &cache);
	assert_eq!(code, 1, "{stderr}");
	assert_eq!(stderr, "");
	let lines: Vec<&str> = stdout.lines().collect();
	assert_eq!(lines.len(), 1, "{stdout}");
	assert!(
		lines[0].starts_with(
			"{\"file\":\"main.fa\",\"range\":{\"start\":{\"line\":2,\"column\":15},\"end\":{\"line\":2,\"column\":18}},\"severity\":\"error\",\"code\":\"E-NUM-6\",\"message\":\""
		),
		"{stdout}"
	);
	let (_, stderr, code) = fa(&["check", "main.fa", "--format=xml"], &root, &cache);
	assert_eq!(code, 1);
	assert!(stderr.contains("--format=json"), "{stderr}");
}

/// A computed `package.fa`, `static` values read from a file and from a
/// library function, `configuration` and `target` (advanced brief §12, §16.3).
#[test]
fn a_package_computes_its_configuration_and_compile_time_values() {
	let cache = scratch("packages-comptime");
	let (_, stderr, code) = fa(&["check", "comptime"], &fixtures(), &cache);
	assert_eq!(code, 0, "{stderr}");
	if !has_bun() {
		return;
	}
	let (stdout, stderr, code) = fa(&["run", "comptime"], &fixtures(), &cache);
	assert_eq!(
		stdout, "Hello\n[0, 1, 4, 9, 16]\nreadable\n1.2.0\nbun\n",
		"stderr: {stderr}"
	);
	assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn a_package_checks_and_builds() {
	let cache = scratch("packages-check");
	let (_, stderr, code) = fa(&["check", "modules"], &fixtures(), &cache);
	assert_eq!(code, 0, "{stderr}");
	let output = scratch("packages-build");
	let target = format!("--output={}", output.display());
	let arguments = ["build", "modules", &target, "--target=bun"];
	let (_, stderr, code) = fa(&arguments, &fixtures(), &cache);
	assert_eq!(code, 0, "{stderr}");
	// `--output=<directory>` writes into `<directory>/<target>/`
	let output = output.join("bun");
	for module in [
		"main.js",
		"$runtime.js",
		"application/main.js",
		"library/counter.js",
		"library/(utils)/bracket.js",
	] {
		assert!(output.join(module).is_file(), "{module} is written");
	}
	assert!(
		!output.join("library/counter.test.js").exists(),
		"tests aren't built"
	);
}

#[test]
fn timings_report_the_phases_and_sizes() {
	let cache = scratch("packages-timings");
	let (_, stderr, code) = fa(&["check", "modules", "--timings"], &fixtures(), &cache);
	assert_eq!(code, 0, "{stderr}");
	for line in [
		"timings: fa check",
		"phase parse",
		"phase analysis",
		"phase total",
		"units ",
		"types ",
		"trees ",
	] {
		assert!(stderr.contains(line), "`{line}` in:\n{stderr}");
	}
	let output = scratch("packages-timings-build");
	let target = format!("--output={}", output.display());
	let arguments = [
		"build",
		"modules",
		&target,
		"--target=bun",
		"--minify=false",
		"--timings",
	];
	let (_, stderr, code) = fa(&arguments, &fixtures(), &cache);
	assert_eq!(code, 0, "{stderr}");
	for line in ["timings: fa build", "phase generation", "phase write"] {
		assert!(stderr.contains(line), "`{line}` in:\n{stderr}");
	}
	let (_, stderr, _) = fa(&["check", "modules"], &fixtures(), &cache);
	assert!(!stderr.contains("timings:"), "{stderr}");
}

#[test]
fn a_package_runs_its_tests() {
	if !has_bun() {
		return;
	}
	let cache = scratch("packages-test");
	let (stdout, stderr, code) = fa(&["test", "modules"], &fixtures(), &cache);
	assert_eq!(
		stdout,
		concat!(
			"✓ increments\n",
			"✓ starts again from zero\n",
			"✓ runs the watch blocks after each statement\n",
			"✓ counts a visit\n",
			"✓ fresh values\n",
			"✗ fails (library/visits.test.fa:9)\n",
			"  assert visits == 2\n",
			"         ↳ left: 0, right: 2\n",
			"\n",
			"5 passed, 1 failed, 1 skipped\n",
		),
		"stderr: {stderr}"
	);
	assert_eq!(code, 1);
	// the tests of a package run on the parallel pipeline: the sequential
	// analysis gives the same
	let sequential = fa_with(
		&["test", "modules"],
		&fixtures(),
		&cache,
		&[("FA_PIPELINE", "0")],
	);
	assert_eq!(sequential, (stdout, stderr, code));
}

/// `fa test --target=native`: the same tests compiled by the LLVM backend,
/// one run per test, reported as on Bun; tests that await run as tasks.
#[test]
fn a_package_runs_its_tests_natively() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let cache = scratch("packages-test-native");
	let (stdout, stderr, code) = fa(&["test", "--target=native", "modules"], &fixtures(), &cache);
	assert_eq!(
		stdout,
		concat!(
			"✓ increments\n",
			"✓ starts again from zero\n",
			"✓ runs the watch blocks after each statement\n",
			"✓ counts a visit\n",
			"✓ fresh values\n",
			"✗ fails (library/visits.test.fa:9)\n",
			"  assert visits == 2\n",
			"         ↳ left: 0, right: 2\n",
			"\n",
			"5 passed, 1 failed, 1 skipped\n",
		),
		"stderr: {stderr}"
	);
	assert_eq!(code, 1);
	let (stdout, stderr, code) = fa(&["test", "--target=native", "tasks"], &fixtures(), &cache);
	assert_eq!(
		stdout,
		"before\ntick\nafter\n✓ a started task runs while the test awaits\n✓ awaits a function of another module\n✓ a blocking test\n\n3 passed, 0 failed\n",
		"stderr: {stderr}"
	);
	assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn errors_name_their_files() {
	let directory = scratch("packages-errors");
	for (path, text) in [
		(
			"package.fa",
			"export = { name = \"broken\", target = [\"web\"] }\n",
		),
		("library/a.fa", "export = b + 1\n"),
		("library/b.fa", "export = a + 1\n"),
		("library/my-file.fa", "export = 1\n"),
		(
			"application/main.fa",
			"export function = (count: Integer) => {}\n",
		),
	] {
		let path = directory.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, text).unwrap();
	}
	let (_, stderr, code) = fa(&["check"], &directory, &directory.join("cache"));
	assert_eq!(code, 1);
	for expected in [
		"package.fa:1:29: error[E-OBJ-1]: `target` is not a field of `PackageConfiguration`: did you mean `targets`?",
		"library/a.fa:1:1: error[MOD007]",
		"library/b.fa:1:1: error[MOD007]",
		"library/my-file.fa:1:1: error[MOD001]",
		"application/main.fa:1:1: error[MOD015]",
	] {
		assert!(stderr.contains(expected), "`{expected}` in:\n{stderr}");
	}
	let (_, stderr, code) = fa(
		&["check"],
		&scratch("packages-none"),
		&directory.join("cache"),
	);
	assert_eq!(code, 1);
	assert!(stderr.contains("CLI001"), "{stderr}");
}

#[test]
fn new_package_scaffolds_a_tested_package() {
	let directory = scratch("packages-new");
	let cache = directory.join("cache");
	let (_, stderr, code) = fa(&["new", "package", "hello"], &directory, &cache);
	assert_eq!(code, 0, "{stderr}");
	let (_, stderr, code) = fa(&["new", "package", "hello"], &directory, &cache);
	assert_eq!(code, 1);
	assert!(stderr.contains("CLI007"), "{stderr}");
	let package = directory.join("hello");
	let (_, stderr, code) = fa(&["check"], &package, &cache);
	assert_eq!(code, 0, "{stderr}");
	if !has_bun() {
		return;
	}
	let (stdout, stderr, code) = fa(&["run", ".", "Ada"], &package, &cache);
	assert_eq!((stdout.as_str(), code), ("Hello, Ada!\n", 0), "{stderr}");
	let (stdout, stderr, code) = fa(&["test"], &package, &cache);
	assert_eq!(code, 0, "{stdout}{stderr}");
	assert!(stdout.contains("1 passed, 0 failed"), "{stdout}");
}

/// A multi-file case of the analyzer: its files, what it prints, its exit
/// code, its arguments.
struct Case {
	files: Vec<(String, String)>,
	stdout: Vec<String>,
	exit: i32,
	pending: bool,
}

/// Reads the `-- file:` sections and the `prints:` / `exit:` annotations.
fn parse_case(text: &str) -> Option<Case> {
	let mut case = Case {
		files: Vec::new(),
		stdout: Vec::new(),
		exit: 0,
		pending: false,
	};
	for line in text.lines() {
		let trimmed = line.trim_start();
		if let Some(path) = trimmed.strip_prefix("-- file:") {
			case.files.push((path.trim().to_string(), String::new()));
			continue;
		}
		if trimmed.starts_with("-- pending") || trimmed.starts_with("-- error") {
			case.pending = true;
		}
		if let Some(position) = line.find("-- prints:") {
			case.stdout
				.push(line[position + "-- prints:".len()..].trim().to_string());
		}
		if let Some(position) = line.find("-- exit:") {
			case.exit = line[position + "-- exit:".len()..].trim().parse().ok()?;
		}
		if line.contains("-- error") || line.contains("-- warning") {
			case.pending = true;
		}
		if let Some((_, text)) = case.files.last_mut() {
			text.push_str(line);
			text.push('\n');
		}
	}
	(case.files.len() > 1 && !case.stdout.is_empty()).then_some(case)
}

#[test]
fn multi_file_cases_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("../analyzer/tests/cases");
	let mut paths = Vec::new();
	let mut pending = vec![cases.clone()];
	while let Some(directory) = pending.pop() {
		for entry in fs::read_dir(&directory).unwrap() {
			let path = entry.unwrap().path();
			if path.is_dir() {
				pending.push(path);
			} else if path.extension().is_some_and(|extension| extension == "fa") {
				paths.push(path);
			}
		}
	}
	paths.sort();
	let mut failures = Vec::new();
	let mut ran = 0;
	for (index, path) in paths.iter().enumerate() {
		let Some(case) = parse_case(&fs::read_to_string(path).unwrap()) else {
			continue;
		};
		if case.pending {
			continue;
		}
		let directory = scratch(&format!("packages-case-{index}"));
		let is_package = case
			.files
			.iter()
			.any(|(path, _)| path.starts_with("library/") || path.starts_with("application/"));
		if is_package {
			fs::write(
				directory.join("package.fa"),
				"export = { name = \"case\" }\n",
			)
			.unwrap();
		}
		for (file, text) in &case.files {
			let file = directory.join(file);
			fs::create_dir_all(file.parent().unwrap()).unwrap();
			fs::write(file, text).unwrap();
		}
		let target = if is_package {
			".".to_string()
		} else {
			case.files[0].0.clone()
		};
		let (stdout, stderr, code) = fa(&["run", &target], &directory, &directory.join("cache"));
		ran += 1;
		let printed: Vec<String> = stdout.lines().map(str::to_string).collect();
		if printed != case.stdout || code != case.exit {
			failures.push(format!(
				"{}: printed {printed:?} (exit {code}), expected {:?} (exit {})\n{stderr}",
				path.strip_prefix(&cases).unwrap_or(path).display(),
				case.stdout,
				case.exit
			));
		}
	}
	eprintln!("{ran} multi-file cases ran");
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}
