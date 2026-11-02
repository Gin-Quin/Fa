//! The ```fa blocks of the documentation (`documentation/**/*.md`) and of the README, shared by
//! the tests of the parser (`tests/doc_snippets.rs`) and of the formatter, which include this file
//! with `#[path]`. The documentation folder can be changed with the `FA_DOCUMENTATION`
//! environment variable.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Snippet {
	pub file: PathBuf,
	pub line: usize,
	pub code: String,
}

impl Snippet {
	pub fn location(&self) -> String {
		format!("{}:{}", self.file.display(), self.line)
	}
}

/// The root of the repository, from the folder of a package.
pub fn repository_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_markdown_files(directory: &Path, files: &mut Vec<PathBuf>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_markdown_files(&path, files);
		} else if path.extension().is_some_and(|extension| extension == "md") {
			files.push(path);
		}
	}
}

/// Every ```fa block of the documentation and of the README.
pub fn snippets() -> Vec<Snippet> {
	let documentation = env::var("FA_DOCUMENTATION")
		.map(PathBuf::from)
		.unwrap_or_else(|_| repository_root().join("documentation"));
	let mut files = Vec::new();
	collect_markdown_files(&documentation, &mut files);
	files.push(repository_root().join("README.md"));
	files.sort();

	let mut snippets = Vec::new();
	for file in files {
		let Ok(markdown) = fs::read_to_string(&file) else {
			continue;
		};
		let mut current: Option<(usize, String, usize)> = None;
		for (index, line) in markdown.lines().enumerate() {
			let indentation = line.len() - line.trim_start().len();
			match &mut current {
				None => {
					if line.trim() == "```fa" {
						current = Some((index + 2, String::new(), indentation));
					}
				}
				Some((start, code, indentation)) => {
					if line.trim() == "```" {
						snippets.push(Snippet {
							file: file.clone(),
							line: *start,
							code: std::mem::take(code),
						});
						current = None;
					} else {
						let content = line.get(*indentation..).unwrap_or(line.trim_start());
						code.push_str(content);
						code.push('\n');
					}
				}
			}
		}
	}
	snippets
}
