//! `fa format [<file or folder>...] [--check]` (architecture/overview.md): formats the source
//! files with `fa-formatter`. A folder stands for every `.fa` file inside it (without the
//! hidden folders, `build`, `node_modules` and `target`); without a path, the closest package
//! of the current folder is formatted (else its workspace). `-` formats the standard input to the
//! standard output.
//!
//! With `--check`, nothing is written: the files that are not formatted are listed, and the
//! exit code is 1 if there is one. A file with a syntax error is never changed: its errors are
//! printed, and the exit code is 1.

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use fa_codebase_parser::SKIPPED_FOLDERS;

use crate::USAGE;

pub(crate) fn format(arguments: &[OsString]) -> i32 {
	let mut check = false;
	let mut paths = Vec::new();
	for argument in arguments {
		match argument.to_str() {
			Some("--check") => check = true,
			Some(option) if option.starts_with("--") => {
				eprintln!("fa format: unknown option `{option}`");
				return USAGE;
			}
			_ => paths.push(PathBuf::from(argument)),
		}
	}
	if paths.iter().any(|path| path.as_os_str() == "-") {
		if paths.len() > 1 {
			eprintln!("fa format: `-` (the standard input) can't be combined with files");
			return USAGE;
		}
		return format_standard_input(check);
	}
	if paths.is_empty() {
		let current = PathBuf::from(".");
		match find_root(&current) {
			Some(root) => paths.push(root),
			None => {
				eprintln!(
					"fa format: no `package.fa` or `workspace.fa` found in this folder or its parents: give the files or folders to format"
				);
				return 1;
			}
		}
	}

	let mut files = Vec::new();
	for path in &paths {
		if path.is_dir() {
			collect_files(path, &mut files);
		} else if path.is_file() {
			files.push(path.clone());
		} else {
			eprintln!(
				"fa format: `{}` is neither a file nor a folder",
				path.display()
			);
			return 1;
		}
	}
	files.sort();
	files.dedup();

	let mut failed = false;
	let mut unformatted = 0;
	for file in &files {
		let source = match fs::read_to_string(file) {
			Ok(source) => source,
			Err(error) => {
				eprintln!("{}: {error}", file.display());
				failed = true;
				continue;
			}
		};
		let formatted = match fa_formatter::format(&source) {
			Ok(formatted) => formatted,
			Err(diagnostics) => {
				print_errors(&file.display().to_string(), &source, &diagnostics);
				failed = true;
				continue;
			}
		};
		if formatted == source {
			continue;
		}
		unformatted += 1;
		if check {
			println!("{}", file.display());
		} else if let Err(error) = fs::write(file, formatted) {
			eprintln!("{}: {error}", file.display());
			failed = true;
		} else {
			println!("formatted {}", file.display());
		}
	}
	if check && unformatted > 0 {
		eprintln!(
			"fa format: {unformatted} of {} files are not formatted: run `fa format`",
			files.len()
		);
	}
	i32::from(failed || (check && unformatted > 0))
}

fn format_standard_input(check: bool) -> i32 {
	let mut source = String::new();
	if let Err(error) = std::io::stdin().read_to_string(&mut source) {
		eprintln!("fa format: can't read the standard input: {error}");
		return 1;
	}
	match fa_formatter::format(&source) {
		Ok(formatted) if check => i32::from(formatted != source),
		Ok(formatted) => {
			let _ = std::io::stdout().write_all(formatted.as_bytes());
			0
		}
		Err(diagnostics) => {
			print_errors("<stdin>", &source, &diagnostics);
			1
		}
	}
}

fn print_errors(shown: &str, source: &str, diagnostics: &[fa_parser::Diagnostic]) {
	let source_map = fa_parser::SourceMap::new(source);
	for diagnostic in diagnostics {
		eprintln!("{shown}:{}", diagnostic.display(&source_map));
	}
	eprintln!("{shown}: not formatted: fix the syntax errors first");
}

/// What `fa format` formats without a path: the closest package containing `start`, else its
/// workspace (like every other command).
fn find_root(start: &Path) -> Option<PathBuf> {
	let start = fs::canonicalize(start).ok()?;
	fa_codebase_parser::discover(&start)
		.package_or_workspace()
		.map(Path::to_path_buf)
}

/// The `.fa` files inside `directory`.
fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let name = entry.file_name();
		let name = name.to_string_lossy();
		if path.is_dir() {
			if !name.starts_with('.') && !SKIPPED_FOLDERS.contains(&name.as_ref()) {
				collect_files(&path, files);
			}
		} else if path.extension().is_some_and(|extension| extension == "fa") {
			files.push(path);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A new empty folder in the temporary folder.
	fn setup_test_directory(name: &str) -> PathBuf {
		let directory =
			std::env::temp_dir().join(format!("fa-format-{name}-{}", std::process::id()));
		let _ = fs::remove_dir_all(&directory);
		fs::create_dir_all(&directory).expect("the temporary folder should be writable");
		directory
	}

	#[test]
	fn formats_the_files_of_a_folder() {
		let directory = setup_test_directory("folder");
		fs::create_dir_all(directory.join("library")).unwrap();
		fs::create_dir_all(directory.join("build")).unwrap();
		fs::write(directory.join("package.fa"), "export = { name = \"x\" }\n").unwrap();
		fs::write(directory.join("library/a.fa"), "let a=1\n").unwrap();
		fs::write(directory.join("build/b.fa"), "let b=1\n").unwrap();

		let arguments = vec![
			OsString::from("--check"),
			directory.clone().into_os_string(),
		];
		assert_eq!(format(&arguments), 1);
		assert_eq!(
			fs::read_to_string(directory.join("library/a.fa")).unwrap(),
			"let a=1\n"
		);

		assert_eq!(format(&[directory.clone().into_os_string()]), 0);
		assert_eq!(
			fs::read_to_string(directory.join("library/a.fa")).unwrap(),
			"let a = 1\n"
		);
		assert_eq!(
			fs::read_to_string(directory.join("build/b.fa")).unwrap(),
			"let b=1\n"
		);

		assert_eq!(format(&arguments), 0);
		let _ = fs::remove_dir_all(&directory);
	}

	#[test]
	fn never_changes_a_file_with_a_syntax_error() {
		let directory = setup_test_directory("errors");
		let file = directory.join("broken.fa");
		fs::write(&file, "let a = (1 +\n").unwrap();
		assert_eq!(format(&[file.clone().into_os_string()]), 1);
		assert_eq!(fs::read_to_string(&file).unwrap(), "let a = (1 +\n");
		let _ = fs::remove_dir_all(&directory);
	}

	#[test]
	fn rejects_an_unknown_option() {
		assert_eq!(format(&[OsString::from("--write")]), USAGE);
	}
}
