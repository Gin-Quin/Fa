//! Workspaces (advanced brief §16, architecture/workspaces.md): a folder with
//! a `workspace.fa` and a `packages/` folder, scanned recursively for the
//! packages; their names come from their `package.fa` and are unique
//! (`CFG004`), and a `package.fa` inside a package is `CFG006`.
//!
//! [`select`] finds what a command works on: a file, a package, or the
//! packages of a workspace that implement the command (decision D44).

use std::fs;
use std::path::{Path, PathBuf};

use fa_analyzer::configuration::{self, ConfigValue, ConfigurationKind, PackageConfiguration};
use fa_analyzer::{CodebaseResolver, Session};
use fa_codebase_parser::{Closest, Codebase};

/// A package of a workspace.
#[derive(Clone, Debug)]
pub(crate) struct Member {
	/// Its name (the `name` of its `package.fa`).
	pub name: String,
	/// Its folder.
	pub directory: PathBuf,
	/// Whether it has an application (its entry file exists).
	pub has_application: bool,
	/// Whether it has test files.
	pub has_tests: bool,
}

/// A workspace and its packages.
#[derive(Clone, Debug)]
pub(crate) struct Workspace {
	/// Its folder.
	pub root: PathBuf,
	/// Its packages, sorted by folder.
	pub members: Vec<Member>,
}

/// The written configuration of a package (`package.fa`), evaluated: a
/// literal is read directly, anything else runs on the interpreter.
pub(crate) fn package_configuration(directory: &Path) -> Option<ConfigValue> {
	let text = fs::read_to_string(directory.join("package.fa")).ok()?;
	let (value, diagnostics) = configuration::evaluate_source(&text, ConfigurationKind::Package);
	if value.is_some() || !configuration::is_computed(&diagnostics) {
		return value;
	}
	let codebase = Codebase::scan_package(directory).ok()?;
	let mut session = Session::new();
	session.set_resolver(Box::new(CodebaseResolver::new(
		codebase,
		Some(directory.to_path_buf()),
	)));
	let file = session.file_of_module("package")?;
	session.analyze_file(file);
	session.source_file(file).configuration.clone()
}

/// The written configuration of a workspace (`workspace.fa`).
pub(crate) fn workspace_configuration(root: &Path) -> Option<ConfigValue> {
	let text = fs::read_to_string(root.join("workspace.fa")).ok()?;
	configuration::evaluate_source(&text, ConfigurationKind::Workspace).0
}

/// Whether a folder holds a `*.test.fa` file (outside the output folders).
fn has_test_files(directory: &Path) -> bool {
	let Ok(entries) = fs::read_dir(directory) else {
		return false;
	};
	entries.flatten().any(|entry| {
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with('.') || fa_codebase_parser::SKIPPED_FOLDERS.contains(&name.as_str()) {
			return false;
		}
		if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
			has_test_files(&entry.path())
		} else {
			name.ends_with(".test.fa")
		}
	})
}

/// Reads a workspace: its packages with their names (`CFG004` when two
/// share one, `CFG006` for a package inside another).
pub(crate) fn load(root: &Path) -> Result<Workspace, String> {
	let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
	let found = fa_codebase_parser::scan_workspace(&root)
		.map_err(|error| format!("fa: can't read `{}`: {error}", root.display()))?;
	if let Some(nested) = found.nested.first() {
		return Err(format!(
			"{}: error[CFG006]: a package can't contain another package: move this folder into `packages/`, next to the other packages",
			crate::dependencies::shown_path(nested)
		));
	}
	let workspace = workspace_configuration(&root);
	let mut members: Vec<Member> = Vec::new();
	for directory in found.packages {
		let written = package_configuration(&directory);
		let name = written
			.as_ref()
			.and_then(|value| value.field("name"))
			.and_then(ConfigValue::as_str)
			.map(str::to_string)
			.unwrap_or_else(|| {
				directory
					.file_name()
					.map(|name| name.to_string_lossy().to_string())
					.unwrap_or_default()
			});
		if let Some(other) = members.iter().find(|member| member.name == name) {
			return Err(format!(
				"error[CFG004]: two packages of the workspace are named `{name}`: `{}` and `{}`; rename one of them in its `package.fa`",
				crate::dependencies::shown_path(&other.directory),
				crate::dependencies::shown_path(&directory)
			));
		}
		let entry = written
			.as_ref()
			.map(|written| {
				let merged = configuration::merge(workspace.as_ref(), written, "bun");
				PackageConfiguration::from_value(&merged).entry
			})
			.unwrap_or_else(|| "application/main.fa".to_string());
		members.push(Member {
			name,
			has_application: directory.join(&entry).is_file(),
			has_tests: has_test_files(&directory.join("library"))
				|| has_test_files(&directory.join("application")),
			directory,
		});
	}
	Ok(Workspace { root, members })
}

/// The packages of a workspace a command applies to (decision D44).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Implements {
	/// `check`, `format`: every package.
	All,
	/// `build`, `develop`, `preview`: the packages with an application.
	Application,
	/// `test`: the packages with test files.
	Tests,
}

/// What a command works on.
#[derive(Clone, Debug)]
pub(crate) enum Selection {
	/// A file, a package folder, or the package of the current folder
	/// (`None`), for [`crate::project::locate`].
	One(Option<PathBuf>),
	/// Packages of a workspace: (name, folder as shown).
	Packages(Vec<(String, PathBuf)>),
}

/// The closest `package.fa` or `workspace.fa` at or above a folder.
fn closest(start: &Path) -> Closest {
	fa_codebase_parser::discover(start).closest()
}

/// A folder as shown from the current folder.
fn shown(directory: &Path) -> PathBuf {
	PathBuf::from(crate::dependencies::shown_path(directory))
}

/// The packages of a workspace that implement a command; a message when
/// none does.
fn implementing(workspace: &Workspace, implements: Implements) -> Selection {
	Selection::Packages(
		workspace
			.members
			.iter()
			.filter(|member| match implements {
				Implements::All => true,
				Implements::Application => member.has_application,
				Implements::Tests => member.has_tests,
			})
			.map(|member| (member.name.clone(), shown(&member.directory)))
			.collect(),
	)
}

/// What a command works on, from its argument: a `.fa` file, a folder, the
/// name of a package of the workspace (`CLI004` when unknown), or nothing
/// (the closest package, else every package of the closest workspace that
/// implements the command).
pub(crate) fn select(argument: Option<&Path>, implements: Implements) -> Result<Selection, String> {
	let current = std::env::current_dir().map_err(|error| format!("fa: {error}"))?;
	let Some(argument) = argument else {
		return match closest(&current) {
			Closest::Workspace(root) => Ok(implementing(&load(&root)?, implements)),
			Closest::Package(_) | Closest::Nothing => Ok(Selection::One(None)),
		};
	};
	if argument.is_file()
		|| argument
			.extension()
			.is_some_and(|extension| extension == "fa")
	{
		return Ok(Selection::One(Some(argument.to_path_buf())));
	}
	if argument.is_dir() {
		let absolute = fs::canonicalize(argument).unwrap_or_else(|_| argument.to_path_buf());
		return match closest(&absolute) {
			Closest::Workspace(root) => Ok(implementing(&load(&root)?, implements)),
			Closest::Package(_) | Closest::Nothing => {
				Ok(Selection::One(Some(argument.to_path_buf())))
			}
		};
	}
	// the name of a package of the workspace
	let name = argument.to_string_lossy();
	let Some(root) = fa_codebase_parser::find_workspace(&current) else {
		return Err(format!(
			"fa: `{name}` is neither a .fa file, a folder, nor a package of a workspace (no `workspace.fa` in this folder or its parents)"
		));
	};
	let workspace = load(&root)?;
	match workspace.members.iter().find(|member| member.name == name) {
		Some(member) => Ok(Selection::One(Some(shown(&member.directory)))),
		None => Err(format!(
			"error[CLI004]: the workspace has no package named `{name}`: its packages are {}",
			if workspace.members.is_empty() {
				"none yet (create one in `packages/` with `fa new package <name>`)".to_string()
			} else {
				workspace
					.members
					.iter()
					.map(|member| format!("`{}`", member.name))
					.collect::<Vec<_>>()
					.join(", ")
			}
		)),
	}
}

/// `fa new workspace [<name>]` (decision D46): `workspace.fa`,
/// `dependencies.fa` and `packages/`, in a new folder or the current one.
pub(crate) fn new_workspace(folder: &Path, name: &str) -> Result<(), String> {
	if folder.join("workspace.fa").exists() || folder.join("package.fa").exists() {
		return Err(format!(
			"error[CLI007]: `{}` is already a workspace or a package",
			folder.display()
		));
	}
	let files = [
		(
			"workspace.fa",
			format!("export = {{\n\tname = \"{name}\"\n}}\n"),
		),
		("dependencies.fa", "export = {}\n".to_string()),
	];
	fs::create_dir_all(folder.join("packages"))
		.map_err(|error| format!("fa new: can't create `{}`: {error}", folder.display()))?;
	for (path, text) in files {
		let path = folder.join(path);
		fs::write(&path, text)
			.map_err(|error| format!("fa new: can't write `{}`: {error}", path.display()))?;
	}
	Ok(())
}
