//! End-to-end tests of workspaces, dependencies and the development commands
//! (advanced brief §16, §17, §19, §20): the workspace of
//! `tests/workspaces/demo` (two packages and a library, a sibling dependency),
//! Git dependencies served by local bare repositories (`FA_GITHUB_BASE` and
//! `git+file://` URLs, so no network is needed), the lockfile and
//! `fa update`, remote files (`file://`), `fa develop` and `fa test --watch`
//! with a scripted change, `fa preview`, `fa new workspace`, `fa help`.
//!
//! The tests that run programs need Bun, the Git tests need `git`; without
//! them, they print a message and pass.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The environment of a test: its scratch folder and its caches.
struct Scratch {
	directory: PathBuf,
}

impl Scratch {
	fn new(name: &str) -> Self {
		let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
			.join("workspaces")
			.join(name);
		let _ = fs::remove_dir_all(&directory);
		fs::create_dir_all(&directory).unwrap();
		Self { directory }
	}

	/// A `fa` command run in a folder of the scratch, with its caches.
	fn command(&self, arguments: &[&str], directory: &Path) -> Command {
		let mut command = Command::new(env!("CARGO_BIN_EXE_fa"));
		command
			.args(arguments)
			.current_dir(directory)
			.env("FA_CACHE", self.directory.join("cache"))
			.env("XDG_CACHE_HOME", self.directory.join("xdg"))
			.env(
				"FA_GITHUB_BASE",
				format!("file://{}", self.directory.join("remote").display()),
			)
			.env_remove("CI")
			.env_remove("FA_DEVELOP_RUNS");
		command
	}

	/// Runs `fa`: `(stdout, stderr, exit code)`.
	fn fa(&self, arguments: &[&str], directory: &Path) -> (String, String, i32) {
		let output = self
			.command(arguments, directory)
			.output()
			.expect("`fa` runs");
		(
			String::from_utf8_lossy(&output.stdout).to_string(),
			String::from_utf8_lossy(&output.stderr).to_string(),
			output.status.code().unwrap_or(-1),
		)
	}

	/// Writes files into a folder of the scratch.
	fn write(&self, folder: &str, files: &[(&str, &str)]) -> PathBuf {
		let folder = self.directory.join(folder);
		for (path, text) in files {
			let path = folder.join(path);
			fs::create_dir_all(path.parent().unwrap()).unwrap();
			fs::write(path, text).unwrap();
		}
		folder
	}
}

fn has_tool(name: &str) -> bool {
	let found = std::env::var_os("PATH").is_some_and(|path| {
		std::env::split_paths(&path).any(|directory| directory.join(name).is_file())
	}) || (name == "bun"
		&& std::env::var_os("HOME")
			.is_some_and(|home| PathBuf::from(home).join(".bun/bin/bun").is_file()));
	if !found {
		eprintln!("`{name}` is not installed: the test is skipped");
	}
	found
}

/// Copies a folder.
fn copy(from: &Path, to: &Path) {
	fs::create_dir_all(to).unwrap();
	for entry in fs::read_dir(from).unwrap() {
		let entry = entry.unwrap();
		let target = to.join(entry.file_name());
		if entry.file_type().unwrap().is_dir() {
			copy(&entry.path(), &target);
		} else {
			fs::copy(entry.path(), target).unwrap();
		}
	}
}

/// The demo workspace, copied into the scratch.
fn demo(scratch: &Scratch) -> PathBuf {
	let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/workspaces/demo");
	let root = scratch.directory.join("demo");
	copy(&fixture, &root);
	root
}

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

/// A Git repository served from the scratch (`remote/<owner>/<name>.git`),
/// and its working copy, to publish versions of a library.
struct Repository {
	work: PathBuf,
}

impl Repository {
	fn new(scratch: &Scratch, owner: &str, name: &str) -> Self {
		let bare = scratch
			.directory
			.join("remote")
			.join(owner)
			.join(format!("{name}.git"));
		fs::create_dir_all(&bare).unwrap();
		git(&["init", "--quiet", "--bare"], &bare);
		let work = scratch.directory.join("work").join(owner).join(name);
		fs::create_dir_all(&work).unwrap();
		git(&["init", "--quiet"], &work);
		git(
			&["remote", "add", "origin", &bare.display().to_string()],
			&work,
		);
		Self { work }
	}

	/// Commits files with `package.fa` at `version`, tags it (`tag`), and
	/// pushes it.
	fn release(&self, name: &str, version: &str, tag: &str, files: &[(&str, &str)]) {
		let package = format!("export = {{\n\tname = \"{name}\"\n\tversion = \"{version}\"\n}}\n");
		fs::write(self.work.join("package.fa"), package).unwrap();
		for (path, text) in files {
			let path = self.work.join(path);
			fs::create_dir_all(path.parent().unwrap()).unwrap();
			fs::write(path, text).unwrap();
		}
		git(&["add", "-A"], &self.work);
		git(&["commit", "--quiet", "-m", version], &self.work);
		git(&["tag", tag], &self.work);
		git(
			&[
				"push",
				"--quiet",
				"origin",
				"HEAD:refs/heads/main",
				"--tags",
			],
			&self.work,
		);
	}
}

fn lockfile(folder: &Path) -> String {
	fs::read_to_string(folder.join("dependencies.lock")).unwrap_or_default()
}

#[test]
fn a_workspace_runs_each_command_on_the_packages_that_implement_it() {
	let scratch = Scratch::new("demo-commands");
	let root = demo(&scratch);
	let (_, stderr, code) = scratch.fa(&["check"], &root);
	assert_eq!(code, 0, "{stderr}");
	for package in ["api", "docs", "shared"] {
		assert!(
			stderr.contains(&format!("fa check: package `{package}`")),
			"{stderr}"
		);
	}
	// inside a package, only that package
	let api = root.join("packages/applications/api");
	let (_, stderr, code) = scratch.fa(&["check"], &api);
	assert_eq!((code, stderr.as_str()), (0, ""));
	let (_, stderr, code) = scratch.fa(&["check", "farm"], &root);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("error[CLI004]: the workspace has no package named `farm`: its packages are `api`, `docs`, `shared`"),
		"{stderr}"
	);
	// no lockfile for sibling packages
	assert!(!root.join("dependencies.lock").exists());
	if !has_tool("bun") {
		return;
	}
	let (stdout, stderr, code) = scratch.fa(&["test"], &root);
	assert_eq!(code, 0, "{stdout}{stderr}");
	assert!(stderr.contains("fa test: package `api`"), "{stderr}");
	assert!(stderr.contains("fa test: package `shared`"), "{stderr}");
	assert!(
		!stderr.contains("package `docs`"),
		"docs has no tests: {stderr}"
	);
	assert!(
		stdout.contains("✓ welcomes with the shared greeting"),
		"{stdout}"
	);
	assert!(stdout.contains("✓ greets a farmer"), "{stdout}");
	let (stdout, stderr, code) = scratch.fa(&["test", "shared"], &root);
	assert_eq!(code, 0, "{stderr}");
	assert_eq!(stdout, "✓ greets a farmer\n\n1 passed, 0 failed\n");
	let (stdout, stderr, code) = scratch.fa(&["run", "api", "Cy"], &root);
	assert_eq!(
		(stdout.as_str(), code),
		("Hello, Cy! (1 cows)\nHello, Bo! (3 cows)\n", 0),
		"{stderr}"
	);
	// `build.output` of the workspace is a default of its packages
	let (_, stderr, code) = scratch.fa(&["build", "--minify=false"], &root);
	assert_eq!(code, 0, "{stderr}");
	let output = api.join("out/bun");
	assert!(output.join("main.js").is_file(), "{stderr}");
	assert!(
		output
			.join("$dependencies/shared/library/greet.js")
			.is_file(),
		"the sibling package is built from source"
	);
	assert!(
		!root.join("packages/docs/out").exists(),
		"docs has no application"
	);
	let (stdout, stderr, code) = scratch.fa(&["preview", "api"], &root);
	assert_eq!(
		(stdout.as_str(), code),
		("Hello, Ada! (1 cows)\nHello, Bo! (3 cows)\n", 0),
		"{stderr}"
	);
	let (_, stderr, code) = scratch.fa(&["preview", "api", "--target=node"], &root);
	assert_eq!(code, 1);
	assert!(stderr.contains("CLI006"), "{stderr}");
}

#[test]
fn the_dependencies_of_a_dependency_are_not_visible() {
	let scratch = Scratch::new("hidden");
	let root = scratch.write(
		"hidden",
		&[
			("workspace.fa", "export = { name = \"hidden\" }\n"),
			("packages/util/package.fa", "export = { name = \"util\" }\n"),
			(
				"packages/util/library/twice.fa",
				"export function = (x: Integer): Integer => x * 2\n",
			),
			("packages/core/package.fa", "export = { name = \"core\" }\n"),
			(
				"packages/core/dependencies.fa",
				"export = { util = \"../util\" }\n",
			),
			(
				"packages/core/library/four.fa",
				"export function = (): Integer => util.twice(2)\n",
			),
			("packages/app/package.fa", "export = { name = \"app\" }\n"),
			(
				"packages/app/dependencies.fa",
				"export = { core = \"../core\" }\n",
			),
			(
				"packages/app/application/main.fa",
				"export function = () => {\n\tconsole.log(core.four(), util.twice(1))\n}\n",
			),
		],
	);
	let (_, stderr, code) = scratch.fa(&["check", "app"], &root);
	assert_eq!(code, 1, "{stderr}");
	assert!(
		stderr.contains("packages/app/application/main.fa:2:") && stderr.contains("`util`"),
		"`util` is a dependency of `core`, not of `app`: {stderr}"
	);
	assert!(!stderr.contains("core/library"), "{stderr}");
	// a path that isn't a package of the workspace
	fs::write(
		root.join("packages/app/dependencies.fa"),
		"export = { core = \"../core\", other = \"../../elsewhere\" }\n",
	)
	.unwrap();
	let (_, stderr, code) = scratch.fa(&["check", "app"], &root);
	assert_eq!(code, 1);
	assert!(stderr.contains("error[DEP008]"), "{stderr}");
	// a key that is a root entry of the package
	fs::create_dir_all(root.join("packages/app/library")).unwrap();
	fs::write(root.join("packages/app/library/core.fa"), "export = 1\n").unwrap();
	fs::write(
		root.join("packages/app/dependencies.fa"),
		"export = { core = \"../core\" }\n",
	)
	.unwrap();
	let (_, stderr, code) = scratch.fa(&["check", "app"], &root);
	assert_eq!(code, 1);
	assert!(stderr.contains("error[DEP005]"), "{stderr}");
}

#[test]
fn git_dependencies_are_resolved_locked_and_updated() {
	if !has_tool("git") {
		return;
	}
	let scratch = Scratch::new("git");
	let farm = Repository::new(&scratch, "farmers", "farm");
	let cows = |count: &str| format!("export function = (): Integer => {count}\n");
	farm.release(
		"farm",
		"1.0.0",
		"v1.0.0",
		&[("library/cows.fa", &cows("10"))],
	);
	farm.release(
		"farm",
		"1.1.0",
		"1.1.0",
		&[("library/cows.fa", &cows("11"))],
	);
	farm.release(
		"farm",
		"2.0.0",
		"v2.0.0",
		&[("library/cows.fa", &cows("20"))],
	);
	farm.release(
		"farm",
		"2.1.0-beta.1",
		"v2.1.0-beta.1",
		&[("library/cows.fa", &cows("21"))],
	);
	let package = scratch.write(
		"app",
		&[
			("package.fa", "export = { name = \"app\" }\n"),
			(
				"dependencies.fa",
				"export = {\n\tfarm = \"github:farmers/farm@1\"\n\tnext = \"github:farmers/farm@2\"\n}\n",
			),
			(
				"application/main.fa",
				"export function = () => {\n\tlet total: Integer = farm.cows() + next.cows()\n\tconsole.log(total)\n}\n",
			),
		],
	);
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	// both majors are locked, the latest release of each (not the beta)
	let locked = lockfile(&package);
	assert!(
		locked.starts_with("-- dependencies.lock: generated by `fa`, do not edit\nexport = [\n\t{\n\t\tsource = \"github:farmers/farm\"\n\t\tversion = \"1.1.0\"\n\t\tcommit = \""),
		"{locked}"
	);
	assert!(locked.contains("\t\tversion = \"2.0.0\"\n"), "{locked}");
	assert_eq!(locked.matches("hash = \"sha256:").count(), 2, "{locked}");
	if has_tool("bun") {
		let (stdout, stderr, code) = scratch.fa(&["run", "."], &package);
		assert_eq!((stdout.as_str(), code), ("31\n", 0), "{stderr}");
	}
	// a new release doesn't change a locked version
	farm.release(
		"farm",
		"1.2.0",
		"v1.2.0",
		&[("library/cows.fa", &cows("12"))],
	);
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	assert_eq!(lockfile(&package), locked);
	// locked and cached: no network needed
	let remote = scratch.directory.join("remote");
	let away = scratch.directory.join("remote-away");
	fs::rename(&remote, &away).unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "offline: {stderr}");
	fs::rename(&away, &remote).unwrap();
	// `fa update farm` accepts the new version
	let (_, stderr, code) = scratch.fa(&["update", "farm"], &package);
	assert_eq!(code, 0, "{stderr}");
	assert!(
		stderr.contains("fa update: github:farmers/farm 1.1.0 -> 1.2.0"),
		"{stderr}"
	);
	let updated = lockfile(&package);
	assert!(updated.contains("version = \"1.2.0\""), "{updated}");
	assert!(updated.contains("version = \"2.0.0\""), "{updated}");
	let (_, stderr, code) = scratch.fa(&["update", "weather"], &package);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("no dependency is named `weather`"),
		"{stderr}"
	);
	// a content that doesn't match its hash (an empty cache, a changed hash)
	fs::remove_dir_all(scratch.directory.join("cache/sources")).unwrap();
	let tampered = updated.replacen("hash = \"sha256:", "hash = \"sha256:0", 1);
	fs::write(package.join("dependencies.lock"), &tampered).unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("error[DEP001]: the content of `farm` changed since it was locked")
			&& stderr.contains("help: run `fa update farm` to accept the new version"),
		"{stderr}"
	);
	let (_, stderr, code) = scratch.fa(&["update"], &package);
	assert_eq!(code, 0, "{stderr}");
	assert_eq!(lockfile(&package), updated);
	// an exact pre-release
	fs::write(
		package.join("dependencies.fa"),
		"export = {\n\tfarm = \"github:farmers/farm@1.0\"\n\tnext = \"github:farmers/farm@2.1.0-beta.1\"\n}\n",
	)
	.unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	let exact = lockfile(&package);
	assert!(exact.contains("version = \"1.0.0\""), "{exact}");
	assert!(exact.contains("version = \"2.1.0-beta.1\""), "{exact}");
	assert!(
		!exact.contains("version = \"1.2.0\""),
		"unused entries are removed: {exact}"
	);
}

#[test]
fn versions_are_shared_by_the_workspace_and_transitive() {
	if !has_tool("git") {
		return;
	}
	let scratch = Scratch::new("git-workspace");
	let soil = Repository::new(&scratch, "farmers", "soil");
	soil.release(
		"soil",
		"0.3.0",
		"v0.3.0",
		&[("library/ph.fa", "export function = (): Integer => 7\n")],
	);
	let farm = Repository::new(&scratch, "farmers", "farm");
	let soil_url = format!(
		"git+file://{}@0",
		scratch.directory.join("remote/farmers/soil.git").display()
	);
	farm.release(
		"farm",
		"4.2.1",
		"v4.2.1",
		&[
			(
				"dependencies.fa",
				&format!("export = {{ soil = \"{soil_url}\" }}\n"),
			),
			(
				"library/cows.fa",
				"export function = (): Integer => soil.ph() * 6\n",
			),
		],
	);
	farm.release(
		"farm",
		"4.3.0",
		"v4.3.0",
		&[(
			"library/cows.fa",
			"export function = (): Integer => soil.ph() * 7\n",
		)],
	);
	let root = scratch.write(
		"farmland",
		&[
			("workspace.fa", "export = { name = \"farmland\" }\n"),
			(
				"dependencies.fa",
				"export = { farm = \"github:farmers/farm@4\" }\n",
			),
			("packages/a/package.fa", "export = { name = \"a\" }\n"),
			(
				"packages/a/application/main.fa",
				"export function = () => {\n\tconsole.log(farm.cows())\n}\n",
			),
			("packages/b/package.fa", "export = { name = \"b\" }\n"),
			(
				"packages/b/dependencies.fa",
				"export = { farm = \"github:farmers/farm@4.2\" }\n",
			),
			(
				"packages/b/application/main.fa",
				"export function = () => {\n\tconsole.log(farm.cows(), soil.ph())\n}\n",
			),
		],
	);
	// one version for both packages: 4.2.x satisfies `@4` and `@4.2`
	let (_, stderr, code) = scratch.fa(&["check", "a"], &root);
	assert_eq!(code, 0, "{stderr}");
	let locked = lockfile(&root);
	assert!(locked.contains("version = \"4.2.1\""), "{locked}");
	assert!(!locked.contains("version = \"4.3.0\""), "{locked}");
	assert!(
		locked.contains("source = \"git+file://"),
		"the dependency of a dependency is locked: {locked}"
	);
	assert!(locked.contains("version = \"0.3.0\""), "{locked}");
	assert!(!root.join("packages/a/dependencies.lock").exists());
	// `soil` is a dependency of `farm`, not of `b`
	let (_, stderr, code) = scratch.fa(&["check", "b"], &root);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("packages/b/application/main.fa:2:") && stderr.contains("`soil`"),
		"{stderr}"
	);
	// no version satisfies both ranges
	fs::write(
		root.join("packages/b/dependencies.fa"),
		"export = { farm = \"github:farmers/farm@4.2.1\" }\n",
	)
	.unwrap();
	fs::write(
		root.join("packages/a/dependencies.fa"),
		"export = { farm = \"github:farmers/farm@4.3\" }\n",
	)
	.unwrap();
	let (_, stderr, code) = scratch.fa(&["check", "a"], &root);
	assert_eq!(code, 1);
	assert!(
		stderr.contains(
			"error[DEP002]: no version 4.x of `github:farmers/farm` satisfies every range"
		) && stderr.contains("@4.3 is requested by `a`")
			&& stderr.contains("@4.2.1 is requested by `b`"),
		"{stderr}"
	);
	if !has_tool("bun") {
		return;
	}
	fs::write(root.join("packages/a/dependencies.fa"), "export = {}\n").unwrap();
	fs::write(root.join("packages/b/dependencies.fa"), "export = {}\n").unwrap();
	let (_, stderr, code) = scratch.fa(&["update"], &root);
	assert_eq!(code, 0, "{stderr}");
	let (stdout, stderr, code) = scratch.fa(&["run", "a"], &root);
	assert_eq!((stdout.as_str(), code), ("49\n", 0), "{stderr}");
}

/// A package outside a workspace depends on a local folder holding a
/// package, locked by its content (design/decisions.md, "Path dependencies
/// outside a workspace").
#[test]
fn path_dependencies_outside_a_workspace_are_locked_by_their_content() {
	let scratch = Scratch::new("paths");
	let shared = scratch.write(
		"shared",
		&[
			("package.fa", "export = { name = \"shared\" }\n"),
			("dependencies.fa", "export = {\n\tunits = \"../units\"\n}\n"),
			(
				"library/greet.fa",
				"export function = (name: String): String => \"Hello, {name}{units.mark()}\"\n",
			),
		],
	);
	scratch.write(
		"units",
		&[
			("package.fa", "export = { name = \"units\" }\n"),
			("library/mark.fa", "export function = (): String => \"!\"\n"),
		],
	);
	let package = scratch.write(
		"app",
		&[
			("package.fa", "export = { name = \"app\" }\n"),
			(
				"dependencies.fa",
				"export = {\n\tshared = \"../shared\"\n}\n",
			),
			(
				"application/main.fa",
				"export function = () => {\n\tconsole.log(shared.greet(\"Ada\"))\n}\n",
			),
		],
	);
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	let locked = lockfile(&package);
	assert!(
		locked.contains("source = \"../shared\"\n\t\thash = \"sha256:"),
		"{locked}"
	);
	assert!(
		locked.contains("source = \"../units\"\n\t\thash = \"sha256:"),
		"the path of a path dependency, from the lockfile's folder: {locked}"
	);
	if has_tool("bun") {
		let (stdout, stderr, code) = scratch.fa(&["run", "."], &package);
		assert_eq!((stdout.as_str(), code), ("Hello, Ada!\n", 0), "{stderr}");
	}
	// the folder changes: locked like any other source
	fs::write(
		shared.join("library/greet.fa"),
		"export function = (name: String): String => \"Hi, {name}\"\n",
	)
	.unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 1);
	assert!(
		stderr.contains(
			"error[DEP001]: the content of `shared` changed since it was locked\n  source: ../shared\n  help: run `fa update shared`"
		),
		"{stderr}"
	);
	let (_, stderr, code) = scratch.fa(&["update", "shared"], &package);
	assert_eq!(code, 0, "{stderr}");
	assert_ne!(lockfile(&package), locked);
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	// a folder that holds no package
	fs::write(
		package.join("dependencies.fa"),
		"export = {\n\tshared = \"../shared\"\n\tnothing = \"../nothing\"\n}\n",
	)
	.unwrap();
	fs::create_dir_all(scratch.directory.join("nothing")).unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 1);
	assert!(
		stderr.contains(
			"error[DEP008]: `nothing` = \"../nothing\" is not a folder holding a package"
		),
		"{stderr}"
	);
}

#[test]
fn remote_files_are_locked_by_their_content() {
	let scratch = Scratch::new("remote");
	let served = scratch.write(
		"served",
		&[
			(
				"v1/types.fa",
				"export namespace = {\n\tfunction celsius = (): Integer => 21\n}\n",
			),
			(
				"ui@1.0.0/manifest.fa",
				"export = [\"button.fa\", \"layout/grid.fa\"]\n",
			),
			(
				"ui@1.0.0/button.fa",
				"export function = (): String => \"[ok]\"\n",
			),
			(
				"ui@1.0.0/layout/grid.fa",
				"export function = (): Integer => 12\n",
			),
		],
	);
	let types = format!("file://{}/v1/types.fa", served.display());
	let ui = format!("file://{}/ui@1.0.0/", served.display());
	let package = scratch.write(
		"app",
		&[
			("package.fa", "export = { name = \"app\" }\n"),
			(
				"dependencies.fa",
				&format!("export = {{\n\tweather = \"{types}\"\n\tui = \"{ui}\"\n}}\n"),
			),
			(
				"application/main.fa",
				"export function = () => {\n\tconsole.log(weather.celsius(), ui.button(), ui.layout.grid())\n}\n",
			),
		],
	);
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	let locked = lockfile(&package);
	assert!(
		locked.contains(&format!("source = \"{types}\"\n\t\thash = \"sha256:")),
		"{locked}"
	);
	assert!(locked.contains(&format!("source = \"{ui}\"")), "{locked}");
	if has_tool("bun") {
		let (stdout, stderr, code) = scratch.fa(&["run", "."], &package);
		assert_eq!((stdout.as_str(), code), ("21 [ok] 12\n", 0), "{stderr}");
	}
	// the remote file changes: the cache still has the locked content
	fs::write(
		served.join("v1/types.fa"),
		"export namespace = {\n\tfunction celsius = (): Integer => 22\n}\n",
	)
	.unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 0, "{stderr}");
	// without the cache, the change is found
	fs::remove_dir_all(scratch.directory.join("cache")).unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 1);
	assert!(
		stderr.contains(
			"error[DEP001]: the content of `weather` changed since it was locked\n  source: file://"
		),
		"{stderr}"
	);
	let (_, stderr, code) = scratch.fa(&["update", "weather"], &package);
	assert_eq!(code, 0, "{stderr}");
	assert_ne!(lockfile(&package), locked);
	// a folder without a manifest
	fs::remove_file(served.join("ui@1.0.0/manifest.fa")).unwrap();
	fs::remove_dir_all(scratch.directory.join("cache")).unwrap();
	let (_, stderr, code) = scratch.fa(&["check"], &package);
	assert_eq!(code, 1);
	assert!(stderr.contains("error[DEP007]"), "{stderr}");
}

/// Reads the output of a child until a text appears, or the time runs out.
fn wait_for(output: &std::sync::Arc<std::sync::Mutex<String>>, text: &str, seconds: u64) -> bool {
	let start = Instant::now();
	while start.elapsed() < Duration::from_secs(seconds) {
		if output.lock().unwrap().contains(text) {
			return true;
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	false
}

/// Starts `fa` with its output collected, ending after `runs` runs.
fn spawn(
	scratch: &Scratch,
	arguments: &[&str],
	directory: &Path,
	runs: usize,
) -> (
	std::process::Child,
	std::sync::Arc<std::sync::Mutex<String>>,
) {
	let mut child = scratch
		.command(arguments, directory)
		.env("FA_DEVELOP_RUNS", runs.to_string())
		.env("FA_WATCH_INTERVAL_MS", "50")
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("`fa` starts");
	let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
	for mut stream in [
		Box::new(child.stdout.take().unwrap()) as Box<dyn std::io::Read + Send>,
		Box::new(child.stderr.take().unwrap()),
	] {
		let output = std::sync::Arc::clone(&output);
		std::thread::spawn(move || {
			let mut buffer = [0; 4096];
			while let Ok(count) = stream.read(&mut buffer) {
				if count == 0 {
					break;
				}
				output
					.lock()
					.unwrap()
					.push_str(&String::from_utf8_lossy(&buffer[..count]));
			}
		});
	}
	(child, output)
}

/// Waits for a child, killing it after a while.
fn finish(mut child: std::process::Child, seconds: u64) -> Option<i32> {
	let start = Instant::now();
	while start.elapsed() < Duration::from_secs(seconds) {
		if let Ok(Some(status)) = child.try_wait() {
			return status.code();
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	let _ = child.kill();
	None
}

#[test]
fn develop_runs_the_program_again_after_a_change() {
	if !has_tool("bun") {
		return;
	}
	let scratch = Scratch::new("develop");
	let root = demo(&scratch);
	let shared = root.join("packages/shared/library/greet.fa");
	let (child, output) = spawn(&scratch, &["develop", "api"], &root, 3);
	assert!(
		wait_for(&output, "Hello, Bo! (3 cows)", 30),
		"{}",
		output.lock().unwrap()
	);
	assert!(wait_for(&output, "watching for changes", 10));
	// an error keeps the program until it is fixed
	fs::write(
		&shared,
		"export function = (user: models): String => \"Hi, {user.nam}!\"\n",
	)
	.unwrap();
	assert!(
		wait_for(&output, "fix the errors to run the program again", 20),
		"{}",
		output.lock().unwrap()
	);
	// a change in the sibling package it depends on
	fs::write(
		&shared,
		"export function = (user: models): String => \"Hi, {user.name}!\"\n",
	)
	.unwrap();
	assert!(
		wait_for(&output, "Hi, Bo!", 20),
		"{}",
		output.lock().unwrap()
	);
	// a change of its own file
	let main = root.join("packages/applications/api/application/main.fa");
	fs::write(
		&main,
		"export function = () => {\n\tconsole.log(\"edited\")\n}\n",
	)
	.unwrap();
	let code = finish(child, 30);
	let output = output.lock().unwrap().clone();
	assert_eq!(code, Some(0), "{output}");
	assert!(output.contains("edited\n"), "{output}");
	// Its last stdout line can arrive before develop observes the process
	// exit. The next edit may therefore restart it or start it again.
	assert!(
		output.contains("fa develop: `application/main.fa` changed: running again")
			|| output.contains("fa develop: `application/main.fa` changed: restarting"),
		"{output}"
	);
	assert!(output.contains("library/greet.fa` changed"), "{output}");
}

#[test]
fn test_watch_runs_the_tests_again_after_a_change() {
	if !has_tool("bun") {
		return;
	}
	let scratch = Scratch::new("test-watch");
	let root = demo(&scratch);
	let (child, output) = spawn(&scratch, &["test", "shared", "--watch"], &root, 2);
	assert!(
		wait_for(&output, "watching for changes", 30),
		"{}",
		output.lock().unwrap()
	);
	fs::write(
		root.join("packages/shared/library/greet.fa"),
		"export function = (user: models): String => \"Hi\"\n",
	)
	.unwrap();
	let code = finish(child, 30);
	let output = output.lock().unwrap().clone();
	assert_eq!(code, Some(0), "{output}");
	assert!(output.contains("1 passed, 0 failed"), "{output}");
	assert!(
		output.contains("fa test: `library/greet.fa` changed: running the tests again"),
		"{output}"
	);
	assert!(output.contains("0 passed, 1 failed"), "{output}");
}

#[test]
fn new_workspace_scaffolds_a_workspace() {
	let scratch = Scratch::new("new-workspace");
	let (_, stderr, code) = scratch.fa(&["new", "workspace", "farmland"], &scratch.directory);
	assert_eq!(code, 0, "{stderr}");
	let root = scratch.directory.join("farmland");
	assert_eq!(
		fs::read_to_string(root.join("workspace.fa")).unwrap(),
		"export = {\n\tname = \"farmland\"\n}\n"
	);
	assert_eq!(
		fs::read_to_string(root.join("dependencies.fa")).unwrap(),
		"export = {}\n"
	);
	assert!(root.join("packages").is_dir());
	let (_, stderr, code) = scratch.fa(&["new", "workspace", "farmland"], &scratch.directory);
	assert_eq!(code, 1);
	assert!(stderr.contains("CLI007"), "{stderr}");
	// in the current folder
	let here = scratch.directory.join("here");
	fs::create_dir_all(&here).unwrap();
	let (_, stderr, code) = scratch.fa(&["new", "workspace"], &here);
	assert_eq!(code, 0, "{stderr}");
	assert!(
		fs::read_to_string(here.join("workspace.fa"))
			.unwrap()
			.contains("name = \"here\"")
	);
	let (_, stderr, code) = scratch.fa(&["new", "workspace"], &here);
	assert_eq!(code, 1);
	assert!(stderr.contains("CLI007"), "{stderr}");
	// a package in it, checked from the root
	let (_, stderr, code) = scratch.fa(&["new", "package", "tool"], &here.join("packages"));
	assert_eq!(code, 0, "{stderr}");
	let (_, stderr, code) = scratch.fa(&["check"], &here);
	assert_eq!(code, 0, "{stderr}");
	// two packages with the same name
	copy(&here.join("packages/tool"), &here.join("packages/copy"));
	let (_, stderr, code) = scratch.fa(&["check"], &here);
	assert_eq!(code, 1);
	assert!(
		stderr.contains("error[CFG004]: two packages of the workspace are named `tool`"),
		"{stderr}"
	);
}

#[test]
fn help_describes_the_commands() {
	let scratch = Scratch::new("help");
	let (stdout, _, code) = scratch.fa(&["help"], &scratch.directory);
	assert_eq!(code, 0);
	for command in [
		"run", "check", "build", "test", "develop", "preview", "format", "update", "new",
	] {
		assert!(
			stdout.contains(&format!("\n  {command} ")),
			"{command}: {stdout}"
		);
	}
	let (stdout, _, code) = scratch.fa(&["help", "develop"], &scratch.directory);
	assert_eq!(code, 0);
	assert!(
		stdout.starts_with("Usage: fa develop [<package>]"),
		"{stdout}"
	);
	let (_, stderr, code) = scratch.fa(&["help", "deploy"], &scratch.directory);
	assert_eq!(code, 1, "a usage error exits with 1");
	assert!(stderr.contains("unknown command `deploy`"), "{stderr}");
}
