//! The language server resolves the dependencies of a package like `fa
//! check` (`fa::editor_session`): a Git dependency served by a local bare
//! repository (`git+file://`, no network), fetched into the cache of the
//! test (`FA_CACHE`); go-to-definition and hover into it, the diagnostics
//! of its errors, and its configuration read again when saved.
//!
//! One test only: it sets `FA_CACHE` for the whole process. Without `git`,
//! it prints a message and passes.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use fa_language_server::{AnalysisBackend, AnalyzerBackend};
use tower_lsp::lsp_types::Url;

/// Runs `git` in a folder.
fn git(arguments: &[&str], directory: &Path) {
	let output = Command::new("git")
		.args(arguments)
		.current_dir(directory)
		.env("GIT_AUTHOR_NAME", "Fa")
		.env("GIT_AUTHOR_EMAIL", "fa@example.com")
		.env("GIT_COMMITTER_NAME", "Fa")
		.env("GIT_COMMITTER_EMAIL", "fa@example.com")
		.output()
		.expect("git runs");
	assert!(
		output.status.success(),
		"git {arguments:?}: {}",
		String::from_utf8_lossy(&output.stderr)
	);
}

/// Writes files below a folder.
fn write_files(root: &Path, files: &[(&str, &str)]) {
	for (path, text) in files {
		let path = root.join(path);
		std::fs::create_dir_all(path.parent().unwrap()).unwrap();
		std::fs::write(path, text).unwrap();
	}
}

/// How long a background resolution may take.
const WAIT: Duration = Duration::from_secs(30);

fn codes(backend: &AnalyzerBackend, uri: &Url) -> Vec<String> {
	backend
		.diagnostics(uri)
		.into_iter()
		.map(|diagnostic| diagnostic.code.unwrap_or_default())
		.collect()
}

#[test]
fn the_dependencies_of_a_package_are_resolved_like_fa_check() {
	if Command::new("git").arg("--version").output().is_err() {
		eprintln!("`git` is not installed: the test is skipped");
		return;
	}
	let directory = tempfile::TempDir::new().unwrap();
	let root = directory.path().canonicalize().unwrap();
	let cache = root.join("cache");
	// SAFETY: the only test of this binary, before any thread reads it
	unsafe { std::env::set_var("FA_CACHE", &cache) };
	// the library, released as 1.0.0
	let bare = root.join("remote/farm.git");
	std::fs::create_dir_all(&bare).unwrap();
	git(&["init", "--quiet", "--bare"], &bare);
	let work = root.join("work");
	std::fs::create_dir_all(&work).unwrap();
	git(&["init", "--quiet"], &work);
	write_files(
		&work,
		&[
			(
				"package.fa",
				"export = {\n\tname = \"farm\"\n\tversion = \"1.0.0\"\n}\n",
			),
			(
				"library/cows.fa",
				"-- the cows of the farm\nexport function = (): Integer => 10\n",
			),
		],
	);
	git(&["add", "-A"], &work);
	git(&["commit", "--quiet", "-m", "1.0.0"], &work);
	git(&["tag", "v1.0.0"], &work);
	git(
		&[
			"push",
			"--quiet",
			&bare.display().to_string(),
			"HEAD:refs/heads/main",
			"--tags",
		],
		&work,
	);
	let source = format!("git+file://{}", bare.display());
	let dependencies = format!("export = {{\n\tfarm = \"{source}@1\"\n}}\n");
	let main_text =
		"export function = () => {\n\tlet count: Integer = farm.cows()\n\tconsole.log(count)\n}\n";
	let package = root.join("app");
	write_files(
		&package,
		&[
			("package.fa", "export = { name = \"app\" }\n"),
			("dependencies.fa", &dependencies),
			("application/main.fa", main_text),
		],
	);
	let main = Url::from_file_path(package.join("application/main.fa")).unwrap();
	let mut backend = AnalyzerBackend::new();
	backend.set_document(&main, Arc::from(main_text));
	// the repository is cloned in the background: meanwhile, a note only
	assert!(backend.has_background_work());
	assert_eq!(codes(&backend, &main), [""]);
	assert_eq!(backend.wait_for_background(WAIT), vec![main.clone()]);
	assert!(
		backend.diagnostics(&main).is_empty(),
		"{:?}",
		backend.diagnostics(&main)
	);
	// in the cache now: a new load needs no network (a save that changes
	// nothing on the disk loads nothing)
	let dependencies_url = Url::from_file_path(package.join("dependencies.fa")).unwrap();
	assert!(backend.saved(&dependencies_url).is_empty());
	std::fs::write(
		package.join("dependencies.fa"),
		format!("{dependencies}-- changed\n"),
	)
	.unwrap();
	let dependencies_url = Url::from_file_path(package.join("dependencies.fa")).unwrap();
	assert_eq!(backend.saved(&dependencies_url), vec![main.clone()]);
	assert!(!backend.has_background_work());
	assert!(backend.diagnostics(&main).is_empty());
	assert!(
		!package.join("dependencies.lock").exists(),
		"the lockfile is never written"
	);
	// hover and go-to-definition into the dependency, in the cache
	let offset = main_text.find("cows").unwrap() + 1;
	let hover = backend.hover(&main, offset).expect("a hover");
	assert!(hover.markdown.contains("Integer"), "{}", hover.markdown);
	let definition = backend.definition(&main, offset).expect("a definition");
	let path = definition.uri.to_file_path().unwrap();
	assert!(
		path.starts_with(&cache) && path.ends_with("library/cows.fa"),
		"{path:?}"
	);
	// a type error with the dependency, as `fa check` reports it
	let wrong = main_text.replace("let count: Integer", "let count: String");
	backend.set_document(&main, Arc::from(wrong.as_str()));
	assert_eq!(codes(&backend, &main), ["E-TYPE-1"]);
	backend.set_document(&main, Arc::from(main_text));
	assert!(backend.diagnostics(&main).is_empty());
	// a range no version satisfies, saved: the error of `fa check`, alone
	let dependencies_path = package.join("dependencies.fa");
	std::fs::write(
		&dependencies_path,
		format!("export = {{\n\tfarm = \"{source}@2\"\n}}\n"),
	)
	.unwrap();
	let saved = Url::from_file_path(&dependencies_path).unwrap();
	assert_eq!(backend.saved(&saved), vec![main.clone()]);
	// a range that isn't locked: the repository is fetched again
	assert_eq!(backend.wait_for_background(WAIT), vec![main.clone()]);
	let diagnostics = backend.diagnostics(&main);
	assert_eq!(codes(&backend, &main), ["DEP002"], "{diagnostics:?}");
	assert!(
		diagnostics[0]
			.message
			.starts_with("no version 2.x of `git+file://"),
		"{}",
		diagnostics[0].message
	);
	// an unversioned remote file: its warning on `dependencies.fa`
	write_files(
		&root,
		&[("served/types.fa", "export function = (): Integer => 21\n")],
	);
	let fixed = format!(
		"export = {{\n\tfarm = \"{source}@1\"\n\tweather = \"file://{}/served/types.fa\"\n}}\n",
		root.display()
	);
	std::fs::write(&dependencies_path, &fixed).unwrap();
	backend.saved(&saved);
	backend.wait_for_background(WAIT);
	assert!(backend.diagnostics(&main).is_empty());
	backend.set_document(&saved, Arc::from(fixed.as_str()));
	// the remote file isn't locked (the lockfile is never written): fetched again
	backend.wait_for_background(WAIT);
	assert_eq!(codes(&backend, &saved), ["DEP004"]);
	assert!(backend.diagnostics(&main).is_empty());
}
