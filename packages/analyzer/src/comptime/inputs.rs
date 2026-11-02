//! The files compile-time code reads (advanced brief §12.5): `@readFile`
//! and `@readDirectory`.
//!
//! A path is relative to the folder of the file that reads it, and must stay
//! inside the root (the package folder, or the standalone file's folder):
//! `META008` otherwise; a missing file is `META009`. The files and folders
//! read are recorded as build inputs of the reading file
//! (`Analysis::build_inputs`), as paths relative to the root.

/// Why a path can't be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
	/// The path leaves the root (`META008`).
	Outside,
	/// No such file or folder (`META009`).
	NotFound,
	/// Not valid UTF-8 text.
	NotText,
}

/// An entry of a folder: its name and whether it is a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
	/// The name of the entry.
	pub name: String,
	/// Whether it is a folder.
	pub is_directory: bool,
}

/// The root-relative path of `relative`, read from the file at the
/// root-relative path `from` (`library/math.fa`): `..` and `.` resolved,
/// `Outside` when it leaves the root (or is absolute).
pub fn resolve(from: &str, relative: &str) -> Result<String, InputError> {
	if relative.starts_with('/') || relative.contains('\\') || relative.contains(':') {
		return Err(InputError::Outside);
	}
	let mut parts: Vec<&str> = from.split('/').collect();
	// the folder of the reading file
	parts.pop();
	for part in relative.split('/') {
		match part {
			"" | "." => {}
			".." => {
				if parts.pop().is_none() {
					return Err(InputError::Outside);
				}
			}
			part => parts.push(part),
		}
	}
	parts.retain(|part| !part.is_empty());
	Ok(parts.join("/"))
}

/// The path of a folder entry, relative to the folder of the reading file:
/// the path given to `@readDirectory` joined with the entry's name (D28).
pub fn entry_path(directory: &str, name: &str) -> String {
	let directory = directory.trim_end_matches('/');
	if directory.is_empty() || directory == "." {
		name.to_string()
	} else {
		format!("{directory}/{name}")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn paths_stay_inside_the_root() {
		assert_eq!(resolve("main.fa", "version.txt"), Ok("version.txt".into()));
		assert_eq!(
			resolve("library/math.fa", "../data/x.json"),
			Ok("data/x.json".into())
		);
		assert_eq!(
			resolve("library/math.fa", "./tables/a.txt"),
			Ok("library/tables/a.txt".into())
		);
		assert_eq!(
			resolve("main.fa", "../../etc/passwd"),
			Err(InputError::Outside)
		);
		assert_eq!(resolve("main.fa", "/etc/passwd"), Err(InputError::Outside));
		assert_eq!(resolve("main.fa", "."), Ok(String::new()));
		assert_eq!(entry_path("../routes", "a.fa"), "../routes/a.fa");
		assert_eq!(entry_path(".", "a.fa"), "a.fa");
	}
}
