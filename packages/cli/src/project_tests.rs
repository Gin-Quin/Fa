//! The unit tests of `project.rs`: the project of a path (`locate`), how
//! its folder is shown (`shown_base`), and the default branch of
//! `focus_forbidden` (`TEST004`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

/// An empty scratch folder of a test.
fn scratch(name: &str) -> PathBuf {
	let directory = std::env::temp_dir().join(format!("fa-project-{name}-{}", std::process::id()));
	let _ = fs::remove_dir_all(&directory);
	fs::create_dir_all(&directory).unwrap();
	fs::canonicalize(&directory).unwrap()
}

/// Writes files into a folder.
fn write(folder: &Path, files: &[(&str, &str)]) {
	for (path, text) in files {
		let path = folder.join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, text).unwrap();
	}
}

#[test]
fn the_default_branch_keeps_its_slashes() {
	assert_eq!(remote_branch("origin/main"), "main");
	assert_eq!(remote_branch("origin/release/1.0"), "release/1.0");
	assert_eq!(remote_branch("upstream/main"), "upstream/main");
}

#[test]
fn shown_base_removes_the_relative_path_of_the_file() {
	let shown = |given: &str, relative: &str| {
		shown_base(Path::new(given), Path::new(relative))
			.display()
			.to_string()
	};
	assert_eq!(shown("farm/scripts/seed.fa", "scripts/seed.fa"), "farm");
	assert_eq!(shown("../farm/scripts/a/b.fa", "scripts/a/b.fa"), "../farm");
	assert_eq!(shown("scripts/seed.fa", "scripts/seed.fa"), "");
	// given from inside the scripts folder
	assert_eq!(shown("seed.fa", "scripts/seed.fa"), "");
}

#[test]
fn a_script_is_located_in_its_package() {
	let folder = scratch("locate");
	write(
		&folder,
		&[
			("farm/package.fa", "export = { name = \"farm\" }\n"),
			("farm/scripts/seed.fa", "console.log(1)\n"),
			("farm/scripts/tools/x.fa.fa", "console.log(2)\n"),
			("farm/library/greet.fa", "export = 1\n"),
			("loose/hello.fa", "console.log(3)\n"),
		],
	);
	let package = folder.join("farm");
	let project = locate(Some(&package.join("scripts/seed.fa"))).unwrap();
	assert!(project.package);
	assert_eq!(project.base, package);
	assert_eq!(project.entry.as_deref(), Some("scripts/seed"));
	assert_eq!(project.shown, package);
	let project = locate(Some(&package.join("scripts/tools/x.fa.fa"))).unwrap();
	assert_eq!(project.entry.as_deref(), Some("scripts/tools/x.fa"));
	assert_eq!(project.shown, package);
	// a file of the library is not a script: a standalone file
	let project = locate(Some(&package.join("library/greet.fa"))).unwrap();
	assert!(!project.package);
	assert_eq!(project.base, package.join("library"));
	assert_eq!(project.entry.as_deref(), Some("greet"));
	// a file outside any package
	let project = locate(Some(&folder.join("loose/hello.fa"))).unwrap();
	assert!(!project.package);
	assert_eq!(project.base, folder.join("loose"));
	assert_eq!(project.entry.as_deref(), Some("hello"));
	// a package folder
	let project = locate(Some(&package)).unwrap();
	assert!(project.package && project.entry.is_none());
	assert_eq!(project.shown, package);
	// neither
	let missing = locate(Some(&folder.join("missing"))).unwrap_err();
	assert!(
		missing.contains("neither a .fa file nor a folder"),
		"{missing}"
	);
	let _ = fs::remove_dir_all(folder);
}

/// Runs `git` in a folder: whether it succeeded.
fn git(arguments: &[&str], directory: &Path) -> bool {
	Command::new("git")
		.args(arguments)
		.current_dir(directory)
		.env("GIT_CONFIG_NOSYSTEM", "1")
		.env("GIT_AUTHOR_NAME", "Fa")
		.env("GIT_AUTHOR_EMAIL", "fa@example.com")
		.env("GIT_COMMITTER_NAME", "Fa")
		.env("GIT_COMMITTER_EMAIL", "fa@example.com")
		.output()
		.is_ok_and(|output| output.status.success())
}

/// Item 16 of the tooling review: a default branch with a slash.
#[test]
fn a_focused_test_is_forbidden_on_a_default_branch_with_a_slash() {
	if std::env::var_os("CI").is_some_and(|value| !value.is_empty()) {
		eprintln!("`CI` is set: the default branch is not checked");
		return;
	}
	let repository = scratch("focus");
	if !git(&["init", "--quiet"], &repository) {
		eprintln!("`git` is not available: the test is skipped");
		return;
	}
	assert!(git(
		&["checkout", "--quiet", "-b", "release/1.0"],
		&repository
	));
	assert!(git(
		&["commit", "--quiet", "--allow-empty", "-m", "first"],
		&repository
	));
	assert!(git(
		&[
			"symbolic-ref",
			"refs/remotes/origin/HEAD",
			"refs/remotes/origin/release/1.0"
		],
		&repository
	));
	assert_eq!(
		focus_forbidden(&repository).as_deref(),
		Some("`release/1.0` is the default branch")
	);
	assert!(git(
		&["checkout", "--quiet", "-b", "feature/x"],
		&repository
	));
	assert_eq!(focus_forbidden(&repository), None);
	let _ = fs::remove_dir_all(repository);
}
