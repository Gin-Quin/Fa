use anyhow::Result;
use rayon::iter::{ IntoParallelIterator, IntoParallelRefIterator, ParallelIterator };
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::read_dir;
use std::path::Path;

#[derive(Debug)]
pub enum FileSystemModule {
	File {
		path: String,
	},
	Directory {
		path: String,
		modules: Vec<FileSystemModule>,
	},
}

impl FileSystemModule {
	/// Recursively read all modules in a directory.
	pub fn read_directory<S: AsRef<OsStr> + ?Sized + Display + AsRef<Path>>(
		root: &S
	) -> Result<FileSystemModule> {
		let entries = read_dir(root).map_err(|e|
			anyhow::anyhow!("Failed to read directory {}: {}", root, e)
		)?;
		let entries: Vec<_> = entries.collect();

		let modules_result: Vec<FileSystemModule> = entries
			.into_par_iter()
			.filter_map(|entry_result| {
				let entry = entry_result.ok()?;
				let path = entry.path();
				let path_str = entry.path().to_str()?.to_string();
				let file_name = entry.file_name().to_str()?.to_string();

				if file_name.starts_with(".") {
					None
				} else if path.is_dir() {
					FileSystemModule::read_directory(&path_str).ok()
				} else if path.is_file() {
					if
						file_name.ends_with(".fa") ||
						file_name.ends_with(".fab") ||
						file_name.ends_with(".json")
					{
						Some(FileSystemModule::File { path: path_str })
					} else {
						None
					}
				} else if path.is_symlink() {
					println!("Symlinks are not supported yet: {}", path_str);
					None
				} else {
					println!("Unknown entry type: {}", path_str);
					None
				}
			})
			.collect();

		Ok(FileSystemModule::Directory {
			path: root.to_string(),
			modules: modules_result,
		})
	}

	/// Count recursively the number of entities in a file system module.
	pub fn get_count(self: &FileSystemModule) -> usize {
		match self {
			FileSystemModule::File { .. } => 1,
			FileSystemModule::Directory { modules, .. } => {
				modules
					.par_iter()
					.map(|m| m.get_count())
					.sum()
			}
		}
	}
}
