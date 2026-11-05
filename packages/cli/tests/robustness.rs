//! The command line on unusual files and concurrent runs (review of the
//! tooling, 2026-11-05).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A new empty folder of the tests.
fn scratch(name: &str) -> PathBuf {
	let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
		.join("robustness")
		.join(name);
	let _ = fs::remove_dir_all(&directory);
	fs::create_dir_all(&directory).unwrap();
	directory
}

/// Writes files into a folder.
fn write(folder: &Path, files: &[(&str, &[u8])]) {
	for (path, bytes) in files {
		let path = folder.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, bytes).unwrap();
	}
}

/// A `fa` command in a folder, with a cache of its own.
fn fa(arguments: &[&str], directory: &Path) -> Command {
	let mut command = Command::new(env!("CARGO_BIN_EXE_fa"));
	command
		.args(arguments)
		.current_dir(directory)
		.env("FA_CACHE", directory.join(".cache"))
		.env_remove("CI");
	command
}

/// Runs `fa`: `(stdout, stderr, exit code)`.
fn run(arguments: &[&str], directory: &Path) -> (String, String, i32) {
	let output = fa(arguments, directory).output().expect("`fa` runs");
	(
		String::from_utf8_lossy(&output.stdout).to_string(),
		String::from_utf8_lossy(&output.stderr).to_string(),
		output.status.code().unwrap_or(-1),
	)
}

fn has_bun() -> bool {
	let home = std::env::var_os("HOME").map(PathBuf::from);
	std::env::var_os("PATH").is_some_and(|path| {
		std::env::split_paths(&path).any(|directory| directory.join("bun").is_file())
	}) || home.is_some_and(|home| home.join(".bun/bin/bun").is_file())
}

#[test]
fn a_file_named_x_fa_fa_is_checked() {
	let directory = scratch("double-extension");
	write(&directory, &[("one/x.fa.fa", b"console.log(1)\n")]);
	let (_, stderr, code) = run(&["check", "one/x.fa.fa"], &directory);
	assert_eq!(code, 0, "{stderr}");
}

#[test]
fn a_file_that_is_not_utf8_is_named_as_such() {
	let directory = scratch("not-utf8");
	write(&directory, &[("bad.fa", b"let x = \"\xff\xfe\"\n")]);
	let (_, stderr, code) = run(&["check", "bad.fa"], &directory);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("bad.fa` is not UTF-8 text: save it as UTF-8"),
		"{stderr}"
	);
	let (_, stderr, code) = run(&["format", "bad.fa"], &directory);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("bad.fa: is not UTF-8 text: save it as UTF-8"),
		"{stderr}"
	);
}

#[test]
fn the_errors_of_several_files_end_with_a_total() {
	let directory = scratch("total");
	write(
		&directory,
		&[
			("package.fa", b"export = { name = \"total\" }\n"),
			("library/a.fa", b"export = missingA\n"),
			("library/b.fa", b"export = missingB\n"),
			(
				"application/main.fa",
				b"export function = () => {\n\tconsole.log(library.a, library.b)\n}\n",
			),
		],
	);
	let (_, stderr, code) = run(&["check"], &directory);
	assert_eq!(code, 1);
	assert!(
		stderr.trim_end().ends_with("2 errors in 2 files"),
		"{stderr}"
	);
	assert!(!stderr.contains("\n1 error\n"), "{stderr}");
	// one file: its own summary
	fs::write(directory.join("library/b.fa"), "export = 2\n").unwrap();
	let (_, stderr, _) = run(&["check"], &directory);
	assert!(stderr.trim_end().ends_with("\n1 error"), "{stderr}");
}

#[test]
fn concurrent_runs_of_a_package_all_succeed() {
	if !has_bun() {
		return;
	}
	let directory = scratch("concurrent");
	write(
		&directory,
		&[
			("package.fa", b"export = { name = \"concurrent\" }\n"),
			("library/value.fa", b"export function = (): Integer => 42\n"),
			(
				"application/main.fa",
				b"export function = () => {\n\tconsole.log(library.value())\n}\n",
			),
		],
	);
	let children: Vec<_> = (0..6)
		.map(|_| {
			fa(&["run", "."], &directory)
				.env(
					"PATH",
					format!(
						"{}/.bun/bin:{}",
						std::env::var("HOME").unwrap_or_default(),
						std::env::var("PATH").unwrap_or_default()
					),
				)
				.stdout(std::process::Stdio::piped())
				.stderr(std::process::Stdio::piped())
				.spawn()
				.expect("`fa` starts")
		})
		.collect();
	for child in children {
		let output = child.wait_with_output().unwrap();
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert_eq!(output.status.code(), Some(0), "{stderr}");
		assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n", "{stderr}");
	}
	// the folders of the runs are removed
	let run = directory.join(".cache/run");
	let left: Vec<_> = fs::read_dir(&run)
		.unwrap()
		.flatten()
		.flat_map(|entry| fs::read_dir(entry.path()).unwrap().flatten())
		.map(|entry| entry.file_name())
		.collect();
	assert!(left.is_empty(), "{left:?}");
}
