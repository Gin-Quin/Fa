use crate::file_system_modules::*;
use std::fs::{ self, File };
use std::path::Path;
use tempfile::TempDir;

fn create_test_file(base_path: &Path, relative_path: &str) -> std::io::Result<()> {
	let file_path = base_path.join(relative_path);

	// Create parent directories if they don't exist
	if let Some(parent) = file_path.parent() {
		fs::create_dir_all(parent)?;
	}

	// Create the file
	File::create(file_path)?;
	Ok(())
}

fn setup_test_directory() -> TempDir {
	let temp_dir = TempDir::new().expect("Failed to create temp directory");
	let base_path = temp_dir.path();

	// Create test files with different extensions
	create_test_file(base_path, "test1.fa").unwrap();
	create_test_file(base_path, "test2.fab").unwrap();
	create_test_file(base_path, "test3.json").unwrap();
	create_test_file(base_path, "test4.txt").unwrap(); // Should be ignored
	create_test_file(base_path, "test5.rs").unwrap(); // Should be ignored

	// Create nested directories with files
	create_test_file(base_path, "dir1/nested1.fa").unwrap();
	create_test_file(base_path, "dir1/nested2.txt").unwrap(); // Should be ignored

	// Create empty directory
	fs::create_dir(base_path.join("empty_dir")).unwrap();

	// Create directory with hidden files (should be ignored)
	create_test_file(base_path, "dir2/.hidden.fa").unwrap();

	// Create hidden directory (should be ignored)
	fs::create_dir(base_path.join(".hidden_dir")).unwrap();
	create_test_file(base_path, ".hidden_dir/file.fa").unwrap();

	temp_dir
}

fn count_files(module: &FileSystemModule) -> usize {
	match module {
		FileSystemModule::File { .. } => 1,
		FileSystemModule::Directory { modules, .. } => {
			modules.iter().map(count_files).sum()
		}
	}
}

fn count_directories(module: &FileSystemModule) -> usize {
	match module {
		FileSystemModule::File { .. } => 0,
		FileSystemModule::Directory { modules, .. } => {
			1 + modules.iter().map(count_directories).sum::<usize>()
		}
	}
}

fn contains_file_with_path(module: &FileSystemModule, path_suffix: &str) -> bool {
	match module {
		FileSystemModule::File { path } => path.ends_with(path_suffix),
		FileSystemModule::Directory { modules, .. } => {
			modules.iter().any(|m| contains_file_with_path(m, path_suffix))
		}
	}
}

#[test]
fn test_read_file_system_modules_basic() {
	let temp_dir = setup_test_directory();
	let root_path = temp_dir.path().to_str().unwrap();

	let result = FileSystemModule::read_directory(root_path).unwrap();

	// Check that we have the correct structure
	match &result {
		FileSystemModule::Directory { path, modules } => {
			assert_eq!(path, root_path);

			// We should have 3 supported files in the root + 2 directories (dir1 and empty_dir)
			// dir2 and .hidden_dir should be filtered out or empty
			assert!(modules.len() >= 5);
		}
		_ => panic!("Root should be a directory"),
	}

	// We should have 4 files total (3 in root, 1 in dir1)
	assert_eq!(count_files(&result), 4);

	// We should have at least 2 directories (root + dir1 + empty_dir)
	assert!(count_directories(&result) >= 3);
}

#[test]
fn test_supported_file_extensions() {
	let temp_dir = setup_test_directory();
	let root_path = temp_dir.path().to_str().unwrap();

	let result = FileSystemModule::read_directory(root_path).unwrap();

	// Check that supported files are included
	assert!(contains_file_with_path(&result, "test1.fa"));
	assert!(contains_file_with_path(&result, "test2.fab"));
	assert!(contains_file_with_path(&result, "test3.json"));
	assert!(contains_file_with_path(&result, "dir1/nested1.fa"));

	// Check that unsupported files are excluded
	assert!(!contains_file_with_path(&result, "test4.txt"));
	assert!(!contains_file_with_path(&result, "test5.rs"));
	assert!(!contains_file_with_path(&result, "dir1/nested2.txt"));
}

#[test]
fn test_hidden_files_and_directories() {
	let temp_dir = setup_test_directory();
	let root_path = temp_dir.path().to_str().unwrap();

	let result = FileSystemModule::read_directory(root_path).unwrap();

	// Hidden files and directories should be excluded
	assert!(!contains_file_with_path(&result, ".hidden.fa"));
	assert!(!contains_file_with_path(&result, ".hidden_dir/file.fa"));
}

#[test]
fn test_empty_directory() {
	let temp_dir = setup_test_directory();
	let root_path = temp_dir.path().to_str().unwrap();

	let result = FileSystemModule::read_directory(root_path).unwrap();

	// Check that empty directories are included
	let empty_dir_found = match &result {
		FileSystemModule::Directory { modules, .. } => {
			modules.iter().any(|m| {
				if let FileSystemModule::Directory { path, modules } = m {
					path.ends_with("empty_dir") && modules.is_empty()
				} else {
					false
				}
			})
		}
		_ => false,
	};

	assert!(empty_dir_found);
}
