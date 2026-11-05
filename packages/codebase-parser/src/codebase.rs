//! The module graph of a codebase (advanced brief §15.1–§15.3): the files of
//! a standalone program or of a package, and the **name table** of each
//! folder.
//!
//! A codebase is built from the relative paths of its `.fa` files
//! ([`Codebase::from_paths`]), read from the disk by [`Codebase::scan_package`]
//! and [`Codebase::scan_standalone`], or given directly (the `-- file:`
//! sections of the analyzer's multi-file cases). Nothing is parsed here: a
//! name table only knows the names of the files and folders.
//!
//! - A `.fa` file is named by its file name without `.fa` (and without
//!   `.worker.fa` / `.foreign.fa`); a `*.test.fa` file is never a name.
//! - A folder containing `.fa` files is a namespace; a `(group)` folder is
//!   not: its entries merge into its parent's table, recursively.
//! - Invalid names (`MOD001`), names that differ only by case (`MOD002`) and
//!   two entries with the same name (`MOD003`) are [`ScanDiagnostic`]s.
//!
//! [`Codebase::lookup`] resolves a name from a file: its siblings (its
//! folder's table, groups merged), then the entries of its root, then the
//! `library` namespace from `application/` and `scripts/`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The shape of a codebase.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Layout {
	/// A standalone file and the files and folders next to it.
	Standalone,
	/// A package: `package.fa`, `library/`, `application/`, `scripts/`.
	Package,
}

/// The root a file belongs to (advanced brief §15.1).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Root {
	/// `library/`.
	Library,
	/// `application/`.
	Application,
	/// `scripts/`.
	Scripts,
	/// The folder of a package itself: its configuration files and the `.fa`
	/// files next to them.
	Package,
	/// The folder of a standalone file.
	Standalone,
}

impl Root {
	/// The folder name of a package root (`library`), if it is one.
	pub fn folder_name(self) -> Option<&'static str> {
		match self {
			Root::Library => Some("library"),
			Root::Application => Some("application"),
			Root::Scripts => Some("scripts"),
			Root::Package | Root::Standalone => None,
		}
	}
}

/// A configuration file.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Configuration {
	/// `package.fa`
	Package,
	/// `workspace.fa`
	Workspace,
	/// `dependencies.fa`
	Dependencies,
}

/// What a file is.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum FileKind {
	/// A module: declarations and one export.
	Module,
	/// A `*.test.fa` file: declarations and tests, no export, never a name.
	Test,
	/// A `*.worker.fa` file.
	Worker,
	/// A `*.foreign.fa` file.
	Foreign,
	/// A program: the standalone file, or a script of `scripts/`. Top-level
	/// statements are allowed, the export is optional.
	Program,
	/// `package.fa`, `workspace.fa`, `dependencies.fa`.
	Configuration(Configuration),
}

/// A folder namespace (an index into [`Codebase::folders`]).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub struct FolderId(pub u32);

impl FolderId {
	/// The index.
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

/// An entry of a name table.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Entry {
	/// A file (an index into [`Codebase::files`]): its export.
	File(usize),
	/// A folder: a namespace of its entries.
	Folder(FolderId),
}

/// A `.fa` file of the codebase.
#[derive(Clone, Debug)]
pub struct SourceFile {
	/// The path relative to the codebase, with `/` separators (`library/(utils)/format.fa`).
	pub path: String,
	/// The module path: the path without `.fa` (`library/(utils)/format`).
	pub module: String,
	/// The name it is known by (`format`); for a test file, its name without `.test`.
	pub name: String,
	/// What it is.
	pub kind: FileKind,
	/// The folder whose table holds its siblings (groups are transparent).
	pub folder: FolderId,
	/// Its root.
	pub root: Root,
}

/// A folder namespace and its name table.
#[derive(Clone, Debug)]
pub struct Folder {
	/// The path relative to the codebase, groups included (`library/reports`); `""` at the top.
	pub path: String,
	/// Its name (`reports`).
	pub name: String,
	/// Its entries by name, the contents of its `(group)` folders merged in.
	pub entries: BTreeMap<String, Entry>,
	/// Its test files (never names).
	pub tests: Vec<usize>,
}

/// A problem found by the scan: `MOD001`, `MOD002` or `MOD003`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanDiagnostic {
	/// The code.
	pub code: &'static str,
	/// The message.
	pub message: String,
	/// The path of the entry (a file, or a folder).
	pub path: String,
	/// The file the diagnostic is shown on: the file itself, or for a folder
	/// the first file it contains.
	pub file: usize,
}

/// The files of a codebase and the name tables of its folders.
#[derive(Clone, Debug)]
pub struct Codebase {
	/// Its shape.
	pub layout: Layout,
	/// Its `.fa` files, sorted by path.
	pub files: Vec<SourceFile>,
	/// Its folders; the first one is the top folder (`""`).
	pub folders: Vec<Folder>,
	/// The problems of its names.
	pub diagnostics: Vec<ScanDiagnostic>,
	/// The folder of each root.
	roots: Vec<(Root, FolderId)>,
}

/// Whether a name is a valid identifier: an ASCII letter or `_`, then ASCII
/// letters, digits or `_` (getting-started/syntax-basics).
pub fn is_identifier(name: &str) -> bool {
	let mut chars = name.chars();
	chars
		.next()
		.is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
		&& chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether a folder name is a group: `(name)`.
fn is_group(name: &str) -> bool {
	name.len() >= 2 && name.starts_with('(') && name.ends_with(')')
}

/// The kind and name of a file from its file name (`format.fa`,
/// `math.test.fa`, `MathWorker.worker.fa`).
fn classify(file_name: &str) -> Option<(FileKind, &str)> {
	let stem = file_name.strip_suffix(".fa")?;
	if let Some(name) = stem.strip_suffix(".test") {
		return Some((FileKind::Test, name));
	}
	if let Some(name) = stem.strip_suffix(".worker") {
		return Some((FileKind::Worker, name));
	}
	if let Some(name) = stem.strip_suffix(".foreign") {
		return Some((FileKind::Foreign, name));
	}
	Some((FileKind::Module, stem))
}

/// The configuration file a top-level file name is.
fn configuration(file_name: &str) -> Option<Configuration> {
	match file_name {
		"package.fa" => Some(Configuration::Package),
		"workspace.fa" => Some(Configuration::Workspace),
		"dependencies.fa" => Some(Configuration::Dependencies),
		_ => None,
	}
}

/// Folder names a scan never enters: outputs and tools' folders (hidden
/// folders are skipped too).
pub const SKIPPED_FOLDERS: &[&str] = &["build", "node_modules", "target"];

/// Whether a path relative to a package's folder is a file that
/// [`Codebase::scan_package`] reads: a `.fa` file at the root, or below
/// `library/`, `application/` or `scripts/` outside hidden and skipped
/// folders.
pub fn is_package_source(relative: &Path) -> bool {
	let parts: Vec<_> = relative
		.components()
		.map(|component| component.as_os_str().to_string_lossy())
		.collect();
	let Some((file, folders)) = parts.split_last() else {
		return false;
	};
	let root = folders.first().map(|first| first.as_ref());
	file.ends_with(".fa")
		&& !file.starts_with('.')
		&& matches!(root, None | Some("library" | "application" | "scripts"))
		&& folders
			.iter()
			.all(|folder| !folder.starts_with('.') && !SKIPPED_FOLDERS.contains(&folder.as_ref()))
}

/// The most `.fa` files a standalone scan collects (a file in a large folder).
const STANDALONE_LIMIT: usize = 5_000;

impl Codebase {
	/// Builds the codebase of `.fa` files given by their relative paths (`/`
	/// separators). In a package, only the configuration files and the files
	/// of `library/`, `application/` and `scripts/` count; `entry` (a
	/// standalone layout) names the program among them.
	pub fn from_paths<'p>(
		layout: Layout,
		paths: impl IntoIterator<Item = &'p str>,
		entry: Option<&str>,
	) -> Codebase {
		let mut paths: Vec<&str> = paths
			.into_iter()
			.filter(|path| path.ends_with(".fa"))
			.collect();
		paths.sort_unstable();
		paths.dedup();
		let mut codebase = Codebase {
			layout,
			files: Vec::new(),
			folders: vec![Folder {
				path: String::new(),
				name: String::new(),
				entries: BTreeMap::new(),
				tests: Vec::new(),
			}],
			diagnostics: Vec::new(),
			roots: Vec::new(),
		};
		if layout == Layout::Standalone {
			codebase.roots.push((Root::Standalone, FolderId(0)));
		}
		// folders by their path without groups
		let mut folder_ids: BTreeMap<String, FolderId> = BTreeMap::new();
		folder_ids.insert(String::new(), FolderId(0));
		// the entries added to each table, with the path that added them (collisions)
		let mut added: BTreeMap<(FolderId, String), Vec<(String, Entry)>> = BTreeMap::new();
		for path in paths {
			let parts: Vec<&str> = path.split('/').collect();
			if parts
				.iter()
				.any(|part| part.is_empty() || part.starts_with('.'))
			{
				continue;
			}
			let (folders, file_name) = parts.split_at(parts.len() - 1);
			let file_name = file_name[0];
			let root = match layout {
				Layout::Standalone => Root::Standalone,
				Layout::Package => match folders.first() {
					None => Root::Package,
					Some(&"library") => Root::Library,
					Some(&"application") => Root::Application,
					Some(&"scripts") => Root::Scripts,
					Some(_) => continue,
				},
			};
			let Some((mut kind, name)) = classify(file_name) else {
				continue;
			};
			if root == Root::Package
				&& let Some(configuration) = configuration(file_name)
			{
				kind = FileKind::Configuration(configuration);
			}
			let is_program = match layout {
				Layout::Standalone => entry == Some(path),
				Layout::Package => root == Root::Scripts,
			};
			// a standalone foreign file runs too (its module stays `name.foreign`)
			if is_program && matches!(kind, FileKind::Module | FileKind::Foreign) {
				kind = FileKind::Program;
			}
			// the folders on the way: a package root is a root, not an entry of the top folder
			let mut folder = FolderId(0);
			let mut stripped = String::new();
			let mut shown = String::new();
			for (depth, &part) in folders.iter().enumerate() {
				if !shown.is_empty() {
					shown.push('/');
				}
				shown.push_str(part);
				if is_group(part) {
					continue;
				}
				if !stripped.is_empty() {
					stripped.push('/');
				}
				stripped.push_str(part);
				let next = match folder_ids.get(&stripped) {
					Some(&next) => next,
					None => {
						let next = FolderId(codebase.folders.len() as u32);
						codebase.folders.push(Folder {
							path: shown.clone(),
							name: part.to_string(),
							entries: BTreeMap::new(),
							tests: Vec::new(),
						});
						folder_ids.insert(stripped.clone(), next);
						let is_root = layout == Layout::Package && depth == 0;
						if is_root {
							codebase.roots.push((root, next));
						} else {
							added
								.entry((folder, part.to_string()))
								.or_default()
								.push((shown.clone(), Entry::Folder(next)));
						}
						next
					}
				};
				folder = next;
			}
			let index = codebase.files.len();
			codebase.files.push(SourceFile {
				path: path.to_string(),
				module: path.strip_suffix(".fa").unwrap_or(path).to_string(),
				name: name.to_string(),
				kind,
				folder,
				root,
			});
			match kind {
				FileKind::Test => codebase.folders[folder.index()].tests.push(index),
				FileKind::Configuration(_) => {}
				_ => added
					.entry((folder, name.to_string()))
					.or_default()
					.push((path.to_string(), Entry::File(index))),
			}
		}
		codebase.roots.sort_by_key(|&(root, _)| root as u8);
		codebase.fill_tables(added);
		codebase
	}

	/// Fills the name tables, reporting invalid names and collisions.
	fn fill_tables(&mut self, added: BTreeMap<(FolderId, String), Vec<(String, Entry)>>) {
		let mut by_folder: BTreeMap<FolderId, Vec<(String, String, Entry)>> = BTreeMap::new();
		for ((folder, name), entries) in added {
			for (path, entry) in entries {
				by_folder
					.entry(folder)
					.or_default()
					.push((name.clone(), path, entry));
			}
		}
		for (folder, entries) in by_folder {
			// lowercased name → the first name with that spelling
			let mut spellings: BTreeMap<String, (String, String, Entry)> = BTreeMap::new();
			for (name, path, entry) in entries {
				let file = self.first_file(entry);
				if !is_identifier(&name) {
					let what = match entry {
						Entry::File(_) => "file",
						Entry::Folder(_) => "folder",
					};
					let shown = path.rsplit('/').next().unwrap_or(&path).to_string();
					self.diagnostics.push(ScanDiagnostic {
						code: "MOD001",
						message: format!(
							"`{shown}` is not a valid name: the name of a {what} is an identifier (letters, digits and `_`, not starting with a digit), ex: `{}`",
							suggest_identifier(&name)
						),
						path,
						file,
					});
					continue;
				}
				let table = &mut self.folders[folder.index()].entries;
				if let Some(&previous) = table.get(&name) {
					let previous_path = self.entry_path(previous);
					let previous_file = self.first_file(previous);
					for (on, other, at) in [
						(path.clone(), previous_path.clone(), file),
						(previous_path.clone(), path.clone(), previous_file),
					] {
						self.diagnostics.push(ScanDiagnostic {
							code: "MOD003",
							message: format!(
								"`{name}` names both `{on}` and `{other}`: rename one of them"
							),
							path: on,
							file: at,
						});
					}
					continue;
				}
				let lower = name.to_ascii_lowercase();
				if let Some((other, other_path, other_entry)) = spellings.get(&lower) {
					let other_file = self.first_file(*other_entry);
					for (on, at, this, that) in [
						(path.clone(), file, name.clone(), other.clone()),
						(other_path.clone(), other_file, other.clone(), name.clone()),
					] {
						self.diagnostics.push(ScanDiagnostic {
							code: "MOD002",
							message: format!(
								"`{this}` and `{that}` differ only by case: some file systems can't tell them apart, rename one of them"
							),
							path: on,
							file: at,
						});
					}
				} else {
					spellings.insert(lower, (name.clone(), path.clone(), entry));
				}
				self.folders[folder.index()].entries.insert(name, entry);
			}
		}
		self.diagnostics
			.sort_by(|a, b| (a.file, &a.path, a.code).cmp(&(b.file, &b.path, b.code)));
		self.diagnostics.dedup();
	}

	/// The path of an entry.
	fn entry_path(&self, entry: Entry) -> String {
		match entry {
			Entry::File(file) => self.files[file].path.clone(),
			Entry::Folder(folder) => self.folders[folder.index()].path.clone(),
		}
	}

	/// The file an entry's diagnostics are shown on: the file, or the first
	/// file of a folder (by path).
	fn first_file(&self, entry: Entry) -> usize {
		match entry {
			Entry::File(file) => file,
			Entry::Folder(folder) => {
				let prefix = format!("{}/", self.folders[folder.index()].path);
				self.files
					.iter()
					.position(|file| file.path.starts_with(&prefix))
					.unwrap_or(0)
			}
		}
	}

	/// Scans a package folder: its top-level `.fa` files (configuration) and
	/// the `.fa` files of `library/`, `application/` and `scripts/`.
	pub fn scan_package(directory: &Path) -> io::Result<Codebase> {
		let mut paths = Vec::new();
		for entry in fs::read_dir(directory)? {
			let entry = entry?;
			let name = entry.file_name().to_string_lossy().to_string();
			if name.starts_with('.') {
				continue;
			}
			let kind = entry.file_type()?;
			if kind.is_file() && name.ends_with(".fa") {
				paths.push(name);
			} else if kind.is_dir()
				&& matches!(name.as_str(), "library" | "application" | "scripts")
			{
				collect_fa_files(&entry.path(), &name, &mut paths, usize::MAX)?;
			}
		}
		Ok(Codebase::from_paths(
			Layout::Package,
			paths.iter().map(String::as_str),
			None,
		))
	}

	/// Scans the library of a package used as a dependency: only the `.fa`
	/// files of its `library/` (its application, scripts and configuration
	/// are not what it exports).
	pub fn scan_library(directory: &Path) -> io::Result<Codebase> {
		let mut paths = Vec::new();
		let library = directory.join("library");
		if library.is_dir() {
			collect_fa_files(&library, "library", &mut paths, usize::MAX)?;
		}
		Ok(Codebase::from_paths(
			Layout::Package,
			paths.iter().map(String::as_str),
			None,
		))
	}

	/// Scans the folder of a standalone file, which is its root; returns the
	/// codebase and the index of the file.
	pub fn scan_standalone(file: &Path) -> io::Result<(Codebase, usize)> {
		let directory = match file.parent() {
			Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
			_ => PathBuf::from("."),
		};
		let file_name = file
			.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap_or_default();
		let mut paths = Vec::new();
		collect_fa_files(&directory, "", &mut paths, STANDALONE_LIMIT)?;
		if !paths.contains(&file_name) {
			paths.push(file_name.clone());
		}
		let codebase = Codebase::from_paths(
			Layout::Standalone,
			paths.iter().map(String::as_str),
			Some(&file_name),
		);
		let index = codebase
			.file_by_path(&file_name)
			.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "the file is not a .fa file"))?;
		Ok((codebase, index))
	}

	/// Scans the part of the folder of a standalone file that the file can
	/// reach: the files and folders of the folder whose names its files
	/// mention, transitively (a folder mentioned: all its files). A large
	/// folder, or one holding other projects, is not read whole.
	pub fn scan_standalone_reached(file: &Path) -> io::Result<(Codebase, usize)> {
		let directory = match file.parent() {
			Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
			_ => PathBuf::from("."),
		};
		let file_name = file
			.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap_or_default();
		// the entries of the folder by the name a file mentions them with
		// (`api.foreign.fa`: `api`), not yet reached
		let mut entries: BTreeMap<String, Vec<(String, bool)>> = BTreeMap::new();
		for entry in fs::read_dir(&directory)?.filter_map(Result::ok) {
			let name = entry.file_name().to_string_lossy().to_string();
			let Ok(kind) = entry.file_type() else {
				continue;
			};
			if name.starts_with('.') || name == file_name {
				continue;
			}
			let folder = kind.is_dir() && !SKIPPED_FOLDERS.contains(&name.as_str());
			if folder || (kind.is_file() && name.ends_with(".fa")) {
				let key = name.split('.').next().unwrap_or_default().to_string();
				entries.entry(key).or_default().push((name, folder));
			}
		}
		let mut paths = vec![file_name.clone()];
		let mut next = 0;
		while next < paths.len() && !entries.is_empty() && paths.len() < STANDALONE_LIMIT {
			let text = fs::read_to_string(directory.join(&paths[next])).unwrap_or_default();
			next += 1;
			let words: std::collections::HashSet<&str> = text
				.split(|c: char| !(c.is_alphanumeric() || c == '_'))
				.filter(|word| !word.is_empty())
				.collect();
			let reached: Vec<String> = entries
				.keys()
				.filter(|key| words.contains(key.as_str()))
				.cloned()
				.collect();
			for key in reached {
				for (name, folder) in entries.remove(&key).unwrap_or_default() {
					if folder {
						let _ = collect_fa_files(
							&directory.join(&name),
							&name,
							&mut paths,
							STANDALONE_LIMIT,
						);
					} else {
						paths.push(name);
					}
				}
			}
		}
		let codebase = Codebase::from_paths(
			Layout::Standalone,
			paths.iter().map(String::as_str),
			Some(&file_name),
		);
		let index = codebase
			.file_by_path(&file_name)
			.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "the file is not a .fa file"))?;
		Ok((codebase, index))
	}

	/// The index of the file with this relative path.
	pub fn file_by_path(&self, path: &str) -> Option<usize> {
		self.files.iter().position(|file| file.path == path)
	}

	/// The index of the file of a module path (`library/math`).
	pub fn file_by_module(&self, module: &str) -> Option<usize> {
		self.files.iter().position(|file| file.module == module)
	}

	/// The folder of a root, if the codebase has it.
	pub fn root_folder(&self, root: Root) -> Option<FolderId> {
		self.roots
			.iter()
			.find(|&&(other, _)| other == root)
			.map(|&(_, folder)| folder)
	}

	/// The roots of the codebase.
	pub fn roots(&self) -> impl Iterator<Item = (Root, FolderId)> + '_ {
		self.roots.iter().copied()
	}

	/// Resolves a name used in a file (advanced brief §15.3, levels 2 and 3):
	/// a sibling (an entry of its folder, groups merged), then an entry of its
	/// root, then the `library` namespace from `application/` and `scripts/`.
	/// The file's own declarations and the dependencies and prelude are the
	/// analyzer's.
	pub fn lookup(&self, from: usize, name: &str) -> Option<Entry> {
		let file = self.files.get(from)?;
		if let Some(&entry) = self.folders[file.folder.index()].entries.get(name) {
			return Some(entry);
		}
		if let Some(root) = self.root_folder(file.root)
			&& let Some(&entry) = self.folders[root.index()].entries.get(name)
		{
			return Some(entry);
		}
		if name == "library" && matches!(file.root, Root::Application | Root::Scripts) {
			return self.root_folder(Root::Library).map(Entry::Folder);
		}
		None
	}

	/// The entry named `name` of a folder namespace.
	pub fn member(&self, folder: FolderId, name: &str) -> Option<Entry> {
		self.folders.get(folder.index())?.entries.get(name).copied()
	}

	/// Every name a file sees through the scan (its siblings, its root's
	/// entries, `library`), for suggestions and completions.
	pub fn visible_names(&self, from: usize) -> Vec<String> {
		let Some(file) = self.files.get(from) else {
			return Vec::new();
		};
		let mut names: Vec<String> = self.folders[file.folder.index()]
			.entries
			.keys()
			.cloned()
			.collect();
		if let Some(root) = self.root_folder(file.root) {
			names.extend(self.folders[root.index()].entries.keys().cloned());
		}
		if matches!(file.root, Root::Application | Root::Scripts)
			&& self.root_folder(Root::Library).is_some()
		{
			names.push("library".to_string());
		}
		names.sort();
		names.dedup();
		names
	}

	/// The test files of the codebase.
	pub fn test_files(&self) -> impl Iterator<Item = usize> + '_ {
		(0..self.files.len()).filter(|&file| self.files[file].kind == FileKind::Test)
	}

	/// Whether a test file named like `name` sits in the folder of a file:
	/// "test files can't be used from other files".
	pub fn is_test_name(&self, from: usize, name: &str) -> bool {
		let Some(file) = self.files.get(from) else {
			return false;
		};
		self.folders[file.folder.index()].tests.iter().any(|&test| {
			self.files[test].name == name || format!("{}.test", self.files[test].name) == name
		})
	}
}

/// An identifier close to an invalid name (`my-file` → `myFile`).
fn suggest_identifier(name: &str) -> String {
	let mut out = String::new();
	let mut upper = false;
	for c in name.chars() {
		if c.is_ascii_alphanumeric() || c == '_' {
			if upper {
				out.push(c.to_ascii_uppercase());
			} else {
				out.push(c);
			}
			upper = false;
		} else {
			upper = !out.is_empty();
		}
	}
	if out.chars().next().is_none_or(|c| c.is_ascii_digit()) {
		out.insert(0, '_');
	}
	out
}

/// Collects the relative paths of the `.fa` files under a folder (skipping
/// names that start with `.` and the output folders), at most `limit`.
fn collect_fa_files(
	directory: &Path,
	prefix: &str,
	out: &mut Vec<String>,
	limit: usize,
) -> io::Result<()> {
	let mut entries: Vec<_> = fs::read_dir(directory)?.filter_map(Result::ok).collect();
	entries.sort_by_key(|entry| entry.file_name());
	for entry in entries {
		if out.len() >= limit {
			return Ok(());
		}
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with('.') {
			continue;
		}
		let relative = if prefix.is_empty() {
			name.clone()
		} else {
			format!("{prefix}/{name}")
		};
		let Ok(kind) = entry.file_type() else {
			continue;
		};
		if kind.is_dir() {
			if !SKIPPED_FOLDERS.contains(&name.as_str()) {
				// an unreadable folder is skipped, like a folder without `.fa` files
				let _ = collect_fa_files(&entry.path(), &relative, out, limit);
			}
		} else if name.ends_with(".fa") {
			out.push(relative);
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn package(paths: &[&str]) -> Codebase {
		Codebase::from_paths(Layout::Package, paths.iter().copied(), None)
	}

	#[test]
	fn package_sources_are_the_files_a_package_scan_reads() {
		for path in [
			"package.fa",
			"helpers.fa",
			"library/math.fa",
			"application/main.fa",
			"scripts/seed/data.fa",
			"library/math.test.fa",
		] {
			assert!(is_package_source(Path::new(path)), "{path}");
		}
		for path in [
			"notes/draft.fa",
			"build/bun/main.fa",
			"library/build/x.fa",
			"library/.hidden/x.fa",
			".x.fa",
			"library/readme.md",
			"",
		] {
			assert!(!is_package_source(Path::new(path)), "{path}");
		}
	}

	fn codes(codebase: &Codebase) -> Vec<(&'static str, String)> {
		codebase
			.diagnostics
			.iter()
			.map(|d| (d.code, d.path.clone()))
			.collect()
	}

	#[test]
	fn siblings_roots_and_the_library_namespace() {
		let codebase = package(&[
			"package.fa",
			"library/myRootFile.fa",
			"library/myFolder/myFile.fa",
			"library/myFolder/myOtherFile.fa",
			"library/myFolder/mySubFolder/mySubFile.fa",
			"application/main.fa",
		]);
		assert!(
			codebase.diagnostics.is_empty(),
			"{:?}",
			codebase.diagnostics
		);
		let my_file = codebase.file_by_path("library/myFolder/myFile.fa").unwrap();
		let other = codebase
			.file_by_module("library/myFolder/myOtherFile")
			.unwrap();
		assert_eq!(
			codebase.lookup(my_file, "myOtherFile"),
			Some(Entry::File(other))
		);
		let Some(Entry::Folder(sub)) = codebase.lookup(my_file, "mySubFolder") else {
			panic!("mySubFolder is a folder");
		};
		let sub_file = codebase
			.file_by_module("library/myFolder/mySubFolder/mySubFile")
			.unwrap();
		assert_eq!(
			codebase.member(sub, "mySubFile"),
			Some(Entry::File(sub_file))
		);
		let root = codebase.file_by_module("library/myRootFile").unwrap();
		assert_eq!(
			codebase.lookup(my_file, "myRootFile"),
			Some(Entry::File(root))
		);
		assert_eq!(codebase.lookup(my_file, "main"), None);
		assert_eq!(codebase.lookup(my_file, "library"), None);
		let main = codebase.file_by_module("application/main").unwrap();
		let library = codebase.root_folder(Root::Library).unwrap();
		assert_eq!(
			codebase.lookup(main, "library"),
			Some(Entry::Folder(library))
		);
		assert_eq!(codebase.lookup(main, "myRootFile"), None);
		let configuration = codebase.file_by_path("package.fa").unwrap();
		assert_eq!(
			codebase.files[configuration].kind,
			FileKind::Configuration(Configuration::Package)
		);
		assert_eq!(codebase.lookup(configuration, "package"), None);
	}

	#[test]
	fn a_sibling_shadows_a_root_entry() {
		let codebase = package(&[
			"library/format.fa",
			"library/reports/format.fa",
			"library/reports/summary.fa",
		]);
		let summary = codebase.file_by_module("library/reports/summary").unwrap();
		let sibling = codebase.file_by_module("library/reports/format").unwrap();
		assert_eq!(
			codebase.lookup(summary, "format"),
			Some(Entry::File(sibling))
		);
	}

	#[test]
	fn groups_merge_into_their_parent() {
		let codebase = package(&[
			"library/(utils)/format.fa",
			"library/reports/(charts)/bar.fa",
			"library/reports/summary.fa",
			"application/main.fa",
		]);
		assert!(
			codebase.diagnostics.is_empty(),
			"{:?}",
			codebase.diagnostics
		);
		let library = codebase.root_folder(Root::Library).unwrap();
		let format = codebase.file_by_module("library/(utils)/format").unwrap();
		assert_eq!(
			codebase.member(library, "format"),
			Some(Entry::File(format))
		);
		let summary = codebase.file_by_module("library/reports/summary").unwrap();
		let bar = codebase
			.file_by_module("library/reports/(charts)/bar")
			.unwrap();
		assert_eq!(codebase.lookup(summary, "bar"), Some(Entry::File(bar)));
		assert_eq!(codebase.lookup(bar, "summary"), Some(Entry::File(summary)));
		assert_eq!(codebase.lookup(bar, "format"), Some(Entry::File(format)));
	}

	#[test]
	fn collisions_and_invalid_names() {
		let codebase = package(&["library/report.fa", "library/report/x.fa"]);
		let found = codes(&codebase);
		assert_eq!(found.len(), 2, "{found:?}");
		assert!(found.iter().all(|(code, _)| *code == "MOD003"));
		let x = codebase.file_by_path("library/report/x.fa").unwrap();
		assert!(codebase.diagnostics.iter().any(|d| d.file == x));

		let codebase = package(&["library/User.fa", "library/user.fa"]);
		assert_eq!(
			codes(&codebase)
				.iter()
				.map(|(code, _)| *code)
				.collect::<Vec<_>>(),
			["MOD002", "MOD002"]
		);

		let codebase = package(&["library/my-file.fa", "library/2d/x.fa"]);
		let found = codes(&codebase);
		assert_eq!(found.len(), 2, "{found:?}");
		assert!(found.iter().all(|(code, _)| *code == "MOD001"));
		assert!(
			codebase.diagnostics[0].message.contains("myFile")
				|| codebase.diagnostics[1].message.contains("myFile")
		);

		let codebase = package(&["library/(a)/format.fa", "library/format.fa"]);
		assert_eq!(codes(&codebase).len(), 2);
	}

	#[test]
	fn test_worker_and_foreign_files() {
		let codebase = package(&[
			"library/math.fa",
			"library/math.test.fa",
			"library/MathWorker.worker.fa",
			"library/Database.foreign.fa",
			"scripts/seed.fa",
		]);
		assert!(
			codebase.diagnostics.is_empty(),
			"{:?}",
			codebase.diagnostics
		);
		let math = codebase.file_by_module("library/math").unwrap();
		let test = codebase.file_by_module("library/math.test").unwrap();
		assert_eq!(codebase.files[test].kind, FileKind::Test);
		assert_eq!(codebase.lookup(test, "math"), Some(Entry::File(math)));
		assert!(codebase.is_test_name(math, "math"));
		let worker = codebase
			.file_by_module("library/MathWorker.worker")
			.unwrap();
		assert_eq!(
			codebase.lookup(math, "MathWorker"),
			Some(Entry::File(worker))
		);
		assert_eq!(codebase.files[worker].kind, FileKind::Worker);
		let foreign = codebase.file_by_module("library/Database.foreign").unwrap();
		assert_eq!(codebase.files[foreign].kind, FileKind::Foreign);
		let seed = codebase.file_by_module("scripts/seed").unwrap();
		assert_eq!(codebase.files[seed].kind, FileKind::Program);
		let library = codebase.root_folder(Root::Library).unwrap();
		assert_eq!(
			codebase.lookup(seed, "library"),
			Some(Entry::Folder(library))
		);
		assert_eq!(codebase.test_files().collect::<Vec<_>>(), [test]);
	}

	#[test]
	fn a_standalone_file_sees_its_folder() {
		let codebase = Codebase::from_paths(
			Layout::Standalone,
			[
				"hello.fa",
				"mySiblingFile.fa",
				"utils/format.fa",
				".hidden/x.fa",
			],
			Some("hello.fa"),
		);
		let hello = codebase.file_by_path("hello.fa").unwrap();
		assert_eq!(codebase.files[hello].kind, FileKind::Program);
		let sibling = codebase.file_by_path("mySiblingFile.fa").unwrap();
		assert_eq!(codebase.files[sibling].kind, FileKind::Module);
		assert_eq!(
			codebase.lookup(hello, "mySiblingFile"),
			Some(Entry::File(sibling))
		);
		let Some(Entry::Folder(utils)) = codebase.lookup(hello, "utils") else {
			panic!("utils is a folder");
		};
		let format = codebase.file_by_path("utils/format.fa").unwrap();
		assert_eq!(codebase.member(utils, "format"), Some(Entry::File(format)));
		assert_eq!(
			codebase.lookup(format, "mySiblingFile"),
			Some(Entry::File(sibling)),
			"root entries"
		);
		assert_eq!(codebase.files.len(), 3, "dot folders are skipped");
	}

	#[test]
	fn scans_the_disk() {
		let directory = tempfile::TempDir::new().unwrap();
		let root = directory.path();
		for path in [
			"package.fa",
			"library/math.fa",
			"library/math.test.fa",
			"application/main.fa",
			"build/bun/main.js",
			"notes/x.fa",
		] {
			let path = root.join(path);
			fs::create_dir_all(path.parent().unwrap()).unwrap();
			fs::write(path, "").unwrap();
		}
		let codebase = Codebase::scan_package(root).unwrap();
		let paths: Vec<&str> = codebase.files.iter().map(|f| f.path.as_str()).collect();
		assert_eq!(
			paths,
			[
				"application/main.fa",
				"library/math.fa",
				"library/math.test.fa",
				"package.fa"
			]
		);
		let (standalone, index) = Codebase::scan_standalone(&root.join("library/math.fa")).unwrap();
		assert_eq!(standalone.files[index].path, "math.fa");
		assert_eq!(standalone.files[index].kind, FileKind::Program);
	}

	#[test]
	fn a_standalone_file_reaches_only_the_names_it_mentions() {
		let root = std::env::temp_dir().join(format!("fa-codebase-reached-{}", std::process::id()));
		let _ = fs::remove_dir_all(&root);
		for (path, text) in [
			("main.fa", "use shapes\nlet x = math.add(1, 2)\n"),
			("math.fa", "let add = (a, b) => a + b -- uses helper\n"),
			("helper.fa", "let one = 1\n"),
			("unrelated.fa", "let z = 0\n"),
			("shapes/circle.fa", "let r = 1\n"),
			("project/package.fa", "export = { name = \"other\" }\n"),
			("project/library/big.fa", "let big = 1\n"),
		] {
			let path = root.join(path);
			fs::create_dir_all(path.parent().unwrap()).unwrap();
			fs::write(path, text).unwrap();
		}
		let (codebase, index) = Codebase::scan_standalone_reached(&root.join("main.fa")).unwrap();
		let mut paths: Vec<&str> = codebase.files.iter().map(|f| f.path.as_str()).collect();
		paths.sort_unstable();
		assert_eq!(
			paths,
			["helper.fa", "main.fa", "math.fa", "shapes/circle.fa"]
		);
		assert_eq!(codebase.files[index].path, "main.fa");
		let _ = fs::remove_dir_all(&root);
	}
}
