//! The packages of a workspace (advanced brief §16.2): the folders of
//! `packages/` that contain a `package.fa`, found recursively. The scan
//! doesn't enter the folder of a package: a `package.fa` deeper inside one
//! is reported (`CFG006`), not taken as another package.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::codebase::SKIPPED_FOLDERS;

/// The package folders of a workspace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspacePackages {
	/// The folders of the packages, sorted.
	pub packages: Vec<PathBuf>,
	/// The `package.fa` files found inside a package's folder (`CFG006`).
	pub nested: Vec<PathBuf>,
}

/// Scans the `packages/` folder of a workspace for the folders that contain
/// a `package.fa`. A workspace without `packages/` has no package.
pub fn scan_workspace(root: &Path) -> io::Result<WorkspacePackages> {
	let mut found = WorkspacePackages::default();
	let packages = root.join("packages");
	if packages.is_dir() {
		visit(&packages, None, &mut found)?;
	}
	found.packages.sort();
	found.nested.sort();
	Ok(found)
}

/// Visits a folder: `package` is the package it is inside of, if any.
fn visit(
	directory: &Path,
	package: Option<&Path>,
	found: &mut WorkspacePackages,
) -> io::Result<()> {
	let is_package = directory.join("package.fa").is_file();
	let inside = match (package, is_package) {
		(Some(package), true) => {
			found.nested.push(directory.join("package.fa"));
			Some(package.to_path_buf())
		}
		(Some(package), false) => Some(package.to_path_buf()),
		(None, true) => {
			found.packages.push(directory.to_path_buf());
			Some(directory.to_path_buf())
		}
		(None, false) => None,
	};
	let mut entries: Vec<_> = fs::read_dir(directory)?.filter_map(Result::ok).collect();
	entries.sort_by_key(|entry| entry.file_name());
	for entry in entries {
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with('.') || SKIPPED_FOLDERS.contains(&name.as_str()) {
			continue;
		}
		if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
			// an unreadable folder is skipped
			let _ = visit(&entry.path(), inside.as_deref(), found);
		}
	}
	Ok(())
}

/// The workspace folder above a path: the closest folder at or above
/// `start` that contains a `workspace.fa`.
pub fn find_workspace(start: &Path) -> Option<PathBuf> {
	start
		.ancestors()
		.find(|directory| directory.join("workspace.fa").is_file())
		.map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn packages_are_found_recursively_but_not_inside_a_package() {
		let root = tempfile::tempdir().unwrap();
		let root = root.path();
		for folder in [
			"packages/shared",
			"packages/applications/api",
			"packages/applications/web",
			"packages/applications/web/library/inner",
			"packages/empty/nothing",
			"packages/shared/build/copy",
		] {
			fs::create_dir_all(root.join(folder)).unwrap();
		}
		fs::write(root.join("workspace.fa"), "export = {}\n").unwrap();
		for file in [
			"packages/shared/package.fa",
			"packages/applications/api/package.fa",
			"packages/applications/web/package.fa",
			"packages/applications/web/library/inner/package.fa",
			"packages/shared/build/copy/package.fa",
		] {
			fs::write(root.join(file), "export = { name = \"x\" }\n").unwrap();
		}
		let found = scan_workspace(root).unwrap();
		assert_eq!(
			found.packages,
			vec![
				root.join("packages/applications/api"),
				root.join("packages/applications/web"),
				root.join("packages/shared"),
			]
		);
		assert_eq!(
			found.nested,
			vec![root.join("packages/applications/web/library/inner/package.fa")]
		);
		assert_eq!(
			find_workspace(&root.join("packages/shared")),
			Some(root.to_path_buf())
		);
	}
}
