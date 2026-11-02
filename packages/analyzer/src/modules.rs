//! Multi-file programs (advanced brief §15): how the analyzer finds the other
//! files of a program, and what it keeps of each file for the others.
//!
//! - A [`ModuleResolver`] answers the name lookups of the module system
//!   (siblings, root entries, `library`) and gives the text of each module.
//!   [`CodebaseResolver`] implements it over the name tables of
//!   `fa-codebase-parser`.
//! - Each file gets a [`FileRole`]: a standalone program, a module of a
//!   package, a test file, a configuration file.
//! - The [`Interface`] of a file is what other files see of it: its export
//!   and the members of its namespaces, with their types. It is published at
//!   the end of the file's walk (and while it runs, for each declaration
//!   done), so that a file whose analysis is in progress can still be read by
//!   the files it uses (cycles between files, `check/modules.rs`).
//! - [`Program`] is what the backends receive from
//!   [`crate::Session::analyze_program`].

use std::path::PathBuf;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use fa_codebase_parser::{Codebase, Configuration, Entry, FileKind, Layout};

use crate::ids::{FileId, NodeId, Sym, TypeId};
use crate::symbols::SymbolKind;

/// What a file is to the analyzer.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum FileRole {
	/// A program: a standalone file, or a script of `scripts/`. Top-level
	/// statements run in order; the export is optional (an entry function).
	Program,
	/// A module of a package, or a file next to a standalone file:
	/// declarations only, exactly one export (`MOD005`, `MOD006`).
	Module,
	/// A `*.test.fa` file: declarations and tests, no export (`MOD014`).
	Test,
	/// `package.fa`, `workspace.fa` or `dependencies.fa`: literal
	/// configuration (D37, [`crate::configuration`]).
	Configuration(Configuration),
}

impl FileRole {
	/// Whether the file holds only declarations, its module-level values
	/// hoisted and initialized in dependency order (advanced brief §15.7).
	pub fn is_module(self) -> bool {
		matches!(self, FileRole::Module | FileRole::Test)
	}
}

/// What a name of the module system designates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleEntry {
	/// A file: its export (the module path, `library/math`).
	File(String),
	/// A folder namespace (its path, `library/reports`).
	Folder(String),
}

/// The module system of a program, as the analyzer sees it (design §1).
/// Modules are named by their path without `.fa` (`library/math`).
pub trait ModuleResolver: Send {
	/// The role of a module.
	fn role(&self, module: &str) -> FileRole;
	/// The name a file is known by (`math` for `library/math`), which
	/// designates its export inside the file itself.
	fn name(&self, module: &str) -> Option<String>;
	/// Resolves a name used in a module, through its siblings, its root's
	/// entries and the `library` namespace (advanced brief §15.3, levels 2–3).
	fn lookup(&self, from: &str, name: &str) -> Option<ModuleEntry>;
	/// The entry `name` of a folder namespace.
	fn member(&self, folder: &str, name: &str) -> Option<ModuleEntry>;
	/// The text of a module.
	fn source(&self, module: &str) -> Option<Arc<str>>;
	/// The path of a module as shown in messages (`library/math.fa`).
	fn path(&self, module: &str) -> String;
	/// The modules the program is made of: every file of the roots, the
	/// configuration files excepted.
	fn modules(&self) -> Vec<String>;
	/// The configuration files of the package (`package`, `dependencies`…).
	fn configurations(&self) -> Vec<String> {
		Vec::new()
	}
	/// The problems of the scan: `(module, code, message)`.
	fn scan_diagnostics(&self) -> Vec<(String, &'static str, String)> {
		Vec::new()
	}
	/// Whether `name` is the name of a test file next to `from` (a better
	/// message than "unknown name").
	fn is_test_name(&self, _from: &str, _name: &str) -> bool {
		false
	}
	/// Every name a module sees through the module system (suggestions).
	fn visible_names(&self, _from: &str) -> Vec<String> {
		Vec::new()
	}
	/// The text of a file of the root read at compile time (`@readFile`),
	/// by its root-relative path (`comptime/inputs.rs`).
	fn read_input(&self, _path: &str) -> Result<String, crate::comptime::inputs::InputError> {
		Err(crate::comptime::inputs::InputError::NotFound)
	}
	/// The entries of a folder of the root (`@readDirectory`), sorted by name.
	fn list_input(
		&self,
		_path: &str,
	) -> Result<Vec<crate::comptime::inputs::DirectoryEntry>, crate::comptime::inputs::InputError>
	{
		Err(crate::comptime::inputs::InputError::NotFound)
	}
}

/// A [`ModuleResolver`] over the name tables of a [`Codebase`], reading the
/// files from the disk (below `base`) unless their text is given.
pub struct CodebaseResolver {
	/// The scanned codebase.
	pub codebase: Codebase,
	/// The folder the relative paths of the codebase start from.
	pub base: Option<PathBuf>,
	/// Texts given instead of the disk's (tests, unsaved editor buffers).
	pub texts: FxHashMap<String, Arc<str>>,
	/// Other files given instead of the disk's (the build inputs of a test),
	/// by root-relative path.
	pub inputs: FxHashMap<String, String>,
	/// Module path → file index.
	by_module: FxHashMap<String, usize>,
	/// Folder path → folder index.
	by_folder: FxHashMap<String, fa_codebase_parser::FolderId>,
}

impl CodebaseResolver {
	/// A resolver reading the files of `codebase` from `base`.
	pub fn new(codebase: Codebase, base: Option<PathBuf>) -> Self {
		let by_module = codebase
			.files
			.iter()
			.enumerate()
			.map(|(index, file)| (file.module.clone(), index))
			.collect();
		let by_folder = codebase
			.folders
			.iter()
			.enumerate()
			.map(|(index, folder)| {
				(
					folder.path.clone(),
					fa_codebase_parser::FolderId(index as u32),
				)
			})
			.collect();
		Self {
			codebase,
			base,
			texts: FxHashMap::default(),
			inputs: FxHashMap::default(),
			by_module,
			by_folder,
		}
	}

	/// A resolver over files given as `(relative path, text)`: the
	/// `-- file:` sections of a multi-file case. A package when a path
	/// starts with `library/`, `application/` or `scripts/`, or is
	/// `package.fa`; otherwise a standalone program whose entry is `entry`.
	pub fn from_texts(files: &[(String, String)], entry: Option<&str>) -> Self {
		let is_package = files.iter().any(|(path, _)| {
			path == "package.fa"
				|| ["library/", "application/", "scripts/"]
					.iter()
					.any(|root| path.starts_with(root))
		});
		let layout = if is_package {
			Layout::Package
		} else {
			Layout::Standalone
		};
		let codebase = Codebase::from_paths(
			layout,
			files
				.iter()
				.map(|(path, _)| path.as_str())
				.filter(|path| path.ends_with(".fa")),
			entry,
		);
		let mut resolver = Self::new(codebase, None);
		for (path, text) in files {
			match path.strip_suffix(".fa") {
				Some(module) => {
					resolver
						.texts
						.insert(module.to_string(), Arc::from(text.as_str()));
				}
				// another file: a build input
				None => {
					resolver.inputs.insert(path.clone(), text.clone());
				}
			}
		}
		resolver
	}

	/// The module path of an entry.
	fn entry(&self, entry: Entry) -> ModuleEntry {
		match entry {
			Entry::File(file) => ModuleEntry::File(self.codebase.files[file].module.clone()),
			Entry::Folder(folder) => {
				ModuleEntry::Folder(self.codebase.folders[folder.index()].path.clone())
			}
		}
	}
}

impl ModuleResolver for CodebaseResolver {
	fn role(&self, module: &str) -> FileRole {
		let Some(&file) = self.by_module.get(module) else {
			return if module.ends_with(".test") {
				FileRole::Test
			} else {
				FileRole::Program
			};
		};
		match self.codebase.files[file].kind {
			FileKind::Module | FileKind::Worker | FileKind::Foreign => FileRole::Module,
			FileKind::Test => FileRole::Test,
			FileKind::Program => FileRole::Program,
			FileKind::Configuration(configuration) => FileRole::Configuration(configuration),
		}
	}

	fn name(&self, module: &str) -> Option<String> {
		let &file = self.by_module.get(module)?;
		let file = &self.codebase.files[file];
		(!matches!(file.kind, FileKind::Test | FileKind::Configuration(_)))
			.then(|| file.name.clone())
	}

	fn lookup(&self, from: &str, name: &str) -> Option<ModuleEntry> {
		let &file = self.by_module.get(from)?;
		self.codebase
			.lookup(file, name)
			.map(|entry| self.entry(entry))
	}

	fn member(&self, folder: &str, name: &str) -> Option<ModuleEntry> {
		let &folder = self.by_folder.get(folder)?;
		self.codebase
			.member(folder, name)
			.map(|entry| self.entry(entry))
	}

	fn source(&self, module: &str) -> Option<Arc<str>> {
		if let Some(text) = self.texts.get(module) {
			return Some(Arc::clone(text));
		}
		let &file = self.by_module.get(module)?;
		let base = self.base.as_ref()?;
		let text = std::fs::read_to_string(base.join(&self.codebase.files[file].path)).ok()?;
		Some(Arc::from(text))
	}

	fn path(&self, module: &str) -> String {
		match self.by_module.get(module) {
			Some(&file) => self.codebase.files[file].path.clone(),
			None => format!("{module}.fa"),
		}
	}

	fn modules(&self) -> Vec<String> {
		self.codebase
			.files
			.iter()
			.filter(|file| !matches!(file.kind, FileKind::Configuration(_)))
			.map(|file| file.module.clone())
			.collect()
	}

	fn configurations(&self) -> Vec<String> {
		self.codebase
			.files
			.iter()
			.filter(|file| matches!(file.kind, FileKind::Configuration(_)))
			.map(|file| file.module.clone())
			.collect()
	}

	fn scan_diagnostics(&self) -> Vec<(String, &'static str, String)> {
		self.codebase
			.diagnostics
			.iter()
			.map(|diagnostic| {
				(
					self.codebase.files[diagnostic.file].module.clone(),
					diagnostic.code,
					diagnostic.message.clone(),
				)
			})
			.collect()
	}

	fn is_test_name(&self, from: &str, name: &str) -> bool {
		self.by_module
			.get(from)
			.is_some_and(|&file| self.codebase.is_test_name(file, name))
	}

	fn visible_names(&self, from: &str) -> Vec<String> {
		self.by_module
			.get(from)
			.map(|&file| self.codebase.visible_names(file))
			.unwrap_or_default()
	}

	fn read_input(&self, path: &str) -> Result<String, crate::comptime::inputs::InputError> {
		use crate::comptime::inputs::InputError;
		if let Some(text) = self.inputs.get(path) {
			return Ok(text.clone());
		}
		if let Some(module) = path.strip_suffix(".fa")
			&& let Some(text) = self.texts.get(module)
		{
			return Ok(text.to_string());
		}
		let Some(base) = &self.base else {
			return Err(InputError::NotFound);
		};
		let file = base.join(path);
		if !file.is_file() {
			return Err(InputError::NotFound);
		}
		let bytes = std::fs::read(file).map_err(|_| InputError::NotFound)?;
		String::from_utf8(bytes).map_err(|_| InputError::NotText)
	}

	fn list_input(
		&self,
		path: &str,
	) -> Result<Vec<crate::comptime::inputs::DirectoryEntry>, crate::comptime::inputs::InputError>
	{
		use crate::comptime::inputs::{DirectoryEntry, InputError};
		let mut entries: Vec<DirectoryEntry> = Vec::new();
		let prefix = if path.is_empty() {
			String::new()
		} else {
			format!("{path}/")
		};
		// the files given as texts
		let given = self
			.inputs
			.keys()
			.cloned()
			.chain(self.texts.keys().map(|module| format!("{module}.fa")));
		for file in given {
			if let Some(rest) = file.strip_prefix(&prefix) {
				let (name, is_directory) = match rest.split_once('/') {
					Some((folder, _)) => (folder.to_string(), true),
					None => (rest.to_string(), false),
				};
				if !entries.iter().any(|entry| entry.name == name) {
					entries.push(DirectoryEntry { name, is_directory });
				}
			}
		}
		if let Some(base) = &self.base {
			let folder = base.join(path);
			if let Ok(read) = std::fs::read_dir(&folder) {
				for entry in read.flatten() {
					let name = entry.file_name().to_string_lossy().to_string();
					let is_directory = entry.file_type().is_ok_and(|kind| kind.is_dir());
					if !entries.iter().any(|other| other.name == name) {
						entries.push(DirectoryEntry { name, is_directory });
					}
				}
			} else if entries.is_empty() {
				return Err(InputError::NotFound);
			}
		} else if entries.is_empty() {
			return Err(InputError::NotFound);
		}
		entries.sort_by(|a, b| a.name.cmp(&b.name));
		Ok(entries)
	}
}

/// A declaration other files can use: the export of a file, or a member of
/// one of its namespaces.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Exported {
	/// Its declaration node.
	pub decl: NodeId,
	/// What it declares.
	pub kind: SymbolKind,
	/// Its type (`UNSET` while its analysis is pending).
	pub ty: TypeId,
	/// For a namespace: the node owning its member scope (its block).
	pub members: Option<NodeId>,
}

/// What other files see of a file.
#[derive(Clone, Debug, Default)]
pub struct Interface {
	/// Its export.
	pub export: Option<Exported>,
	/// The members of its namespaces, by (namespace block, name).
	pub members: FxHashMap<(NodeId, Sym), Exported>,
}

impl Interface {
	/// A hash of the interface for the incremental analysis: it changes
	/// when a declaration other files can see changes its kind or its type,
	/// or moves to another node (the files using it name it by its node).
	pub fn hash(&self, types: &crate::types::TypeStore) -> u64 {
		let mut parts: Vec<(String, String, String)> = self
			.members
			.iter()
			.map(|(&(owner, name), exported)| {
				(
					format!(
						"{}:{}@{}",
						owner.index(),
						types.syms.resolve(name),
						exported.decl.index()
					),
					format!("{:?}", exported.kind),
					types.display(exported.ty),
				)
			})
			.collect();
		if let Some(export) = &self.export {
			parts.push((
				format!("@{}", export.decl.index()),
				format!("{:?}", export.kind),
				types.display(export.ty),
			));
		}
		parts.sort();
		let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
		for (a, b, c) in &parts {
			for byte in a.bytes().chain(b.bytes()).chain(c.bytes()).chain([0]) {
				hash ^= u64::from(byte);
				hash = hash.wrapping_mul(0x0100_0000_01b3);
			}
		}
		hash
	}
}

/// An analyzed program, as the backends receive it (design §1).
#[derive(Clone, Debug, Default)]
pub struct Program {
	/// Every analyzed file of the program, the dependencies of a file before
	/// it (for a cycle between files, in the order of their paths).
	pub files: Vec<FileId>,
	/// The program's entry: the standalone file or script given, or the
	/// entry file of the application (`application/main.fa`).
	pub entry: Option<FileId>,
	/// The module-level values that need a runtime initialization, in
	/// initialization order (advanced brief §15.7): `(file, declaration)`.
	pub init_order: Vec<(FileId, NodeId)>,
	/// The test files of the program.
	pub tests: Vec<FileId>,
	/// The configuration files of the package, evaluated (D37).
	pub configurations: Vec<FileId>,
}

impl Program {
	/// Whether one of the files has an error.
	pub fn has_errors(&self, session: &crate::Session) -> bool {
		self.files
			.iter()
			.chain(&self.tests)
			.chain(&self.configurations)
			.any(|&file| {
				session
					.diagnostics(file)
					.iter()
					.any(|diagnostic| diagnostic.severity == crate::Severity::Error)
			})
	}
}
