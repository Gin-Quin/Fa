//! End-to-end tests of `fa test` (advanced brief §18.2): a focused test
//! (`test only`) on CI or on the default branch (`TEST004`), and a test
//! file that never ends (`FA_TEST_TIMEOUT`).
//!
//! Bun is found on the `PATH` or in `~/.bun/bin`; without it, the tests
//! print a message and pass.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// A scratch folder of this test binary, with files.
fn scratch(name: &str, files: &[(&str, &str)]) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
		.join("testing")
		.join(name);
	let _ = fs::remove_dir_all(&directory);
	for (path, text) in files {
		let path = directory.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, text).unwrap();
	}
	directory
}

/// A command run in a folder, away from the Git repository of the tests
/// (`GIT_CEILING_DIRECTORIES`) and from the user's Git configuration.
fn command(program: &str, arguments: &[&str], directory: &Path) -> Command {
	let mut command = Command::new(program);
	let scratch = directory.parent().unwrap();
	let empty = scratch.join("gitconfig");
	fs::write(&empty, "").unwrap();
	command
		.args(arguments)
		.current_dir(directory)
		.env("XDG_CACHE_HOME", scratch.join("cache"))
		.env("GIT_CEILING_DIRECTORIES", scratch)
		.env("GIT_CONFIG_GLOBAL", &empty)
		.env("GIT_CONFIG_NOSYSTEM", "1")
		.env("GIT_AUTHOR_NAME", "Fa")
		.env("GIT_AUTHOR_EMAIL", "fa@example.com")
		.env("GIT_COMMITTER_NAME", "Fa")
		.env("GIT_COMMITTER_EMAIL", "fa@example.com")
		.env_remove("CI")
		.env_remove("FA_TEST_TIMEOUT");
	command
}

/// Runs `fa` with environment variables: `(stdout, stderr, exit code)`.
fn fa(arguments: &[&str], directory: &Path, variables: &[(&str, &str)]) -> (String, String, i32) {
	let output = command(env!("CARGO_BIN_EXE_fa"), arguments, directory)
		.envs(variables.iter().copied())
		.output()
		.expect("`fa` runs");
	(
		String::from_utf8_lossy(&output.stdout).to_string(),
		String::from_utf8_lossy(&output.stderr).to_string(),
		output.status.code().unwrap_or(-1),
	)
}

/// Runs `git` in a folder: whether it succeeded.
fn git(arguments: &[&str], directory: &Path) -> bool {
	command("git", arguments, directory)
		.output()
		.is_ok_and(|output| output.status.success())
}

fn has_bun() -> bool {
	let name = if cfg!(windows) { "bun.exe" } else { "bun" };
	let found = std::env::var_os("PATH").is_some_and(|path| {
		std::env::split_paths(&path).any(|directory| directory.join(name).is_file())
	}) || std::env::var_os("HOME")
		.is_some_and(|home| PathBuf::from(home).join(".bun/bin").join(name).is_file());
	if !found {
		eprintln!("Bun is not installed: the test is skipped");
	}
	found
}

const FOCUSED: &[(&str, &str)] = &[
	("focus/package.fa", "export = { name = \"focus\" }\n"),
	(
		"focus/library/math.test.fa",
		"test only \"adds\" {\n\tassert 1 + 1 == 2\n}\n\ntest \"subtracts\" {\n\tassert 2 - 1 == 1\n}\n",
	),
];

/// `TEST004`: a `test only` fails the run on CI and on the default
/// branch, and only runs the focused test elsewhere.
#[test]
fn a_focused_test_fails_on_ci_and_on_the_default_branch() {
	if !has_bun() {
		return;
	}
	let package = scratch("focus", FOCUSED).join("focus");
	let (stdout, stderr, code) = fa(&["test"], &package, &[]);
	assert_eq!(code, 0, "{stdout}{stderr}");
	assert!(stdout.contains("1 passed, 0 failed, 1 skipped"), "{stdout}");
	let (stdout, stderr, code) = fa(&["test"], &package, &[("CI", "true")]);
	assert_eq!(code, 1, "{stdout}{stderr}");
	assert!(
		stderr.contains(
			"error[TEST004]: a `test only` would hide the other tests, and the `CI` variable is set: remove `only`"
		),
		"{stderr}"
	);
	assert!(!stdout.contains("passed"), "{stdout}");
	// the default branch of the repository (no remote: `main`)
	if !git(&["init", "--quiet"], &package) {
		eprintln!("`git` is not available: the default branch is not checked");
		return;
	}
	assert!(git(&["checkout", "--quiet", "-b", "main"], &package));
	assert!(git(
		&["commit", "--quiet", "--allow-empty", "-m", "first"],
		&package
	));
	let (_, stderr, code) = fa(&["test"], &package, &[]);
	assert_eq!(code, 1, "{stderr}");
	assert!(
		stderr.contains("error[TEST004]") && stderr.contains("`main` is the default branch"),
		"{stderr}"
	);
	// another branch
	assert!(git(
		&["checkout", "--quiet", "-b", "feature/focus"],
		&package
	));
	let (stdout, stderr, code) = fa(&["test"], &package, &[]);
	assert_eq!(code, 0, "{stdout}{stderr}");
}

/// A test file that never ends is stopped after `FA_TEST_TIMEOUT` seconds
/// and reported as failed; the other files run.
#[test]
fn a_test_file_that_never_ends_is_stopped() {
	if !has_bun() {
		return;
	}
	let hangs = "use { sleep } = std.time\n\ntest \"waits forever\" {\n\tawait sleep(600000)\n}\n";
	let folder = scratch(
		"timeout",
		&[
			("both/package.fa", "export = { name = \"both\" }\n"),
			("both/library/hangs.test.fa", hangs),
			(
				"both/library/quick.test.fa",
				"test \"is quick\" {\n\tassert 1 == 1\n}\n",
			),
			("alone/package.fa", "export = { name = \"alone\" }\n"),
			("alone/library/hangs.test.fa", hangs),
		],
	);
	for (package, sequential) in [("both", "1"), ("both", "0"), ("alone", "1")] {
		let start = Instant::now();
		let (stdout, stderr, code) = fa(
			&["test"],
			&folder.join(package),
			&[("FA_TEST_TIMEOUT", "2"), ("FA_PIPELINE", sequential)],
		);
		assert!(
			start.elapsed() < Duration::from_secs(60),
			"{stdout}{stderr}"
		);
		assert_eq!(code, 1, "{stdout}{stderr}");
		assert!(
			stdout.contains(
				"✗ library/hangs.test.fa: its tests didn't finish in 2 s, so they were stopped (set `FA_TEST_TIMEOUT` to give them more seconds)"
			),
			"{package}: {stdout}{stderr}"
		);
		if package == "both" {
			assert!(stdout.contains("✓ is quick"), "{stdout}");
			assert!(stdout.contains("1 passed, 1 failed"), "{stdout}");
		}
	}
}

#[test]
fn wasm_tests_select_each_body_and_reinitialize_module_values() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let package = scratch(
		"wasm-tests",
		&[
			(
				"package.fa",
				"export = { name = \"tests\", targets = [\"wasm\"] }\n",
			),
			(
				"library/check.test.fa",
				r#"use { sleep } = std.time
mutable count = 0
test "first" {
	count += 1
	assert count == 1
}
test "second" {
	count += 1
	assert count == 1
}
test "fails" { assert 1 == 2 }
test skip "skipped" { assert false }
test "awaits" {
	await sleep(1)
	assert count == 0
}
type E = Errors { Bad }
function fail = (): Never throws E.Bad => { throw E.Bad }
test "catches" { assert (fail() catch _error => true) }
"#,
			),
		],
	);
	let (stdout, stderr, code) = fa(&["test", "--target=wasm"], &package, &[]);
	assert_eq!(code, 1, "{stdout}{stderr}");
	assert!(
		stdout.contains("4 passed, 1 failed, 1 skipped"),
		"{stdout}{stderr}"
	);
	assert!(
		stdout.contains("✗ fails") && stdout.contains("✓ catches"),
		"{stdout}{stderr}"
	);
}

#[test]
fn llvm_tests_time_out_and_continue_with_the_other_files() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	for target in ["native", "wasm"] {
		let package = scratch(
			&format!("{target}-timeout"),
			&[
				("package.fa", "export = { name = \"tests\" }\n"),
				(
					"library/slow.test.fa",
					"test \"waits\" { await std.time.sleep(600000) }\n",
				),
				(
					"library/quick.test.fa",
					"test \"continues\" { assert 1 == 1 }\n",
				),
			],
		);
		let start = Instant::now();
		let (stdout, stderr, code) = fa(
			&["test", &format!("--target={target}")],
			&package,
			&[("FA_TEST_TIMEOUT", "1")],
		);
		assert!(
			start.elapsed() < Duration::from_secs(60),
			"{stdout}{stderr}"
		);
		assert_eq!(code, 1, "{stdout}{stderr}");
		assert!(
			stdout.contains("its test didn't finish in 1 s") && stdout.contains("✓ continues"),
			"{stdout}{stderr}"
		);
		assert!(stdout.contains("1 passed, 1 failed"), "{stdout}{stderr}");
	}
}
