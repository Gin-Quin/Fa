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

use rustc_hash::{FxHashMap, FxHashSet};

use fa_codebase_parser::{Codebase, Configuration, Entry, FileKind, Layout};

use crate::ids::{DeclKey, FileId, ListId, ModuleId, NodeId, ParamId, Sym, TypeId};
use crate::symbols::SymbolKind;
use crate::types::{FieldFlags, TypeKind, TypeStore};

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
pub trait ModuleResolver: Send + Sync {
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
	/// The file of a module on the disk, if it has one (go-to-definition).
	fn file(&self, _module: &str) -> Option<PathBuf> {
		None
	}
	/// The modules the program is made of: every file of the roots, the
	/// configuration files excepted.
	fn modules(&self) -> Vec<String>;
	/// The modules below a folder (`library/math` → `library/math/add`…),
	/// a folder of a dependency included (`$dependencies/farm@4.2.1/library`).
	fn modules_in(&self, folder: &str) -> Vec<String> {
		let prefix = format!("{folder}/");
		self.modules()
			.into_iter()
			.filter(|module| module.starts_with(&prefix))
			.collect()
	}
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

	fn file(&self, module: &str) -> Option<PathBuf> {
		let &file = self.by_module.get(module)?;
		Some(self.base.as_ref()?.join(&self.codebase.files[file].path))
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

/// A module system whose texts are replaced by the ones given (the unsaved
/// documents of an editor): the language server shares the texts with it
/// and changes them as documents are edited.
pub struct OverlayResolver {
	/// The module system read from the disk.
	pub inner: Box<dyn ModuleResolver>,
	/// The texts that replace the disk's, by module path.
	pub texts: Arc<std::sync::RwLock<FxHashMap<String, Arc<str>>>>,
}

impl ModuleResolver for OverlayResolver {
	fn role(&self, module: &str) -> FileRole {
		self.inner.role(module)
	}

	fn name(&self, module: &str) -> Option<String> {
		self.inner.name(module)
	}

	fn lookup(&self, from: &str, name: &str) -> Option<ModuleEntry> {
		self.inner.lookup(from, name)
	}

	fn member(&self, folder: &str, name: &str) -> Option<ModuleEntry> {
		self.inner.member(folder, name)
	}

	fn source(&self, module: &str) -> Option<Arc<str>> {
		let texts = self
			.texts
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		match texts.get(module) {
			Some(text) => Some(Arc::clone(text)),
			None => self.inner.source(module),
		}
	}

	fn path(&self, module: &str) -> String {
		self.inner.path(module)
	}

	fn file(&self, module: &str) -> Option<PathBuf> {
		self.inner.file(module)
	}

	fn modules(&self) -> Vec<String> {
		self.inner.modules()
	}

	fn modules_in(&self, folder: &str) -> Vec<String> {
		self.inner.modules_in(folder)
	}

	fn configurations(&self) -> Vec<String> {
		self.inner.configurations()
	}

	fn scan_diagnostics(&self) -> Vec<(String, &'static str, String)> {
		self.inner.scan_diagnostics()
	}

	fn is_test_name(&self, from: &str, name: &str) -> bool {
		self.inner.is_test_name(from, name)
	}

	fn visible_names(&self, from: &str) -> Vec<String> {
		self.inner.visible_names(from)
	}

	fn read_input(&self, path: &str) -> Result<String, crate::comptime::inputs::InputError> {
		self.inner.read_input(path)
	}

	fn list_input(
		&self,
		path: &str,
	) -> Result<Vec<crate::comptime::inputs::DirectoryEntry>, crate::comptime::inputs::InputError>
	{
		self.inner.list_input(path)
	}
}

/// A declaration other files can use: the export of a file, or a member of
/// one of its namespaces.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Exported {
	/// Its declaration node.
	pub decl: NodeId,
	/// Its declaration key (what the files using it name it by).
	pub key: DeclKey,
	/// What it declares.
	pub kind: SymbolKind,
	/// Its type (`UNSET` while its analysis is pending).
	pub ty: TypeId,
	/// For a namespace: the node owning its member scope (its block).
	pub members: Option<NodeId>,
	/// The generic parameters of a generic type or function, in order (as
	/// `TypeKind::Param` types): what other files apply explicitly
	/// (`library.Pair<Integer, String>`, `library.firstOf<Integer>`).
	pub generics: ListId,
}

/// A function declared in a type (a method or an associated function).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeFunction {
	/// Its name.
	pub name: Sym,
	/// Its declaration node.
	pub decl: NodeId,
	/// Its declaration key.
	pub key: DeclKey,
	/// Its type.
	pub ty: TypeId,
	/// Whether it takes `self` (a method).
	pub method: bool,
}

/// A parameter of a function other files call (for named arguments and
/// defaults across files).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
	/// Its name.
	pub name: Sym,
	/// Its declaration node.
	pub decl: NodeId,
	/// Its declaration key.
	pub key: DeclKey,
	/// What it is (a parameter, `self`).
	pub kind: SymbolKind,
	/// Its type.
	pub ty: TypeId,
	/// Whether it has a default value.
	pub has_default: bool,
	/// A function-typed parameter the function only calls: its argument
	/// may throw (`check/callbacks.rs`).
	pub only_called: bool,
}

/// A declaration of a file as the rest of the program sees it: the
/// module-level declarations (namespace members included) and the
/// functions, with the module-level declarations each one uses (the
/// initialization order of the program, advanced brief §15.7).
#[derive(Clone, Debug, PartialEq)]
pub struct Declared {
	/// What it declares.
	pub kind: SymbolKind,
	/// Its name.
	pub name: Sym,
	/// The span of its declaration.
	pub span: crate::diagnostics::SourceSpan,
	/// For a function: the node of its function (compile-time lowering).
	pub function: Option<NodeId>,
	/// The module-level declarations (and functions called directly) its
	/// declaration uses, by key, sorted.
	pub uses: Vec<DeclKey>,
	/// For a value: whether its type is thread-safe (a `Mutex`, an atomic,
	/// a `Channel`), which worker code may use (`CONC008`).
	pub thread_safe: bool,
}

/// Where a declaration other files may name is, in the current version of
/// its file: what [`Resolution::External`](crate::symbols::Resolution)
/// keys lead to (go-to-definition into a file whose tree is dropped).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeyedDecl {
	/// Its declaration node.
	pub node: NodeId,
	/// The span of its declaration.
	pub span: crate::diagnostics::SourceSpan,
	/// The span of its name.
	pub name_span: crate::diagnostics::SourceSpan,
}

/// What an erased generic function needs from an application at runtime.
/// These are semantic capabilities; backends choose their representations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericRequirement {
	Value(ParamId),
	Method(ParamId, Sym),
}

/// The complete generic parameter order and runtime needs of a function.
#[derive(Clone, Debug)]
pub struct GenericFunction {
	pub generics: ListId,
	pub requirements: Vec<GenericRequirement>,
}

/// A compile-time key-set parameter, with the type parameter it selects.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenericKeySet {
	pub subject: ParamId,
	pub deep: bool,
}

/// An application whose declaring file must check its `@for` bodies.
/// Locations name modules, since a unit's file ids die with its session.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeySetApplication {
	pub param: ParamId,
	pub subject: TypeId,
	pub key_set: TypeId,
	pub caller: ModuleId,
	pub span: crate::diagnostics::SourceSpan,
}

/// The signature-derived capabilities shared by local and imported calls.
pub fn generic_requirements(types: &TypeStore, generics: &[ParamId]) -> Vec<GenericRequirement> {
	let mut requirements = Vec::new();
	for &param in generics {
		let info = types.param(param);
		if info.value {
			requirements.push(GenericRequirement::Value(param));
			continue;
		}
		for field in types.fields_of(info.constraint) {
			if field.flags.contains(FieldFlags::METHOD) {
				requirements.push(GenericRequirement::Method(param, field.name));
			}
		}
	}
	requirements
}

/// What other files see of a file: everything another file, the program
/// analysis and the backends read of it, so that its tree and its analysis
/// can be dropped once it is analyzed and generated
/// (design/parallel-pipeline.md §2.2).
#[derive(Clone, Debug, Default)]
pub struct Interface {
	/// Its export.
	pub export: Option<Exported>,
	/// The members of its namespaces, by (namespace block, name).
	pub members: FxHashMap<(NodeId, Sym), Exported>,
	/// The functions of its types, by type declaration.
	pub type_functions: FxHashMap<NodeId, Vec<TypeFunction>>,
	/// The complete generic parameter lists of named types, including phantom parameters.
	pub type_parameters: FxHashMap<NodeId, ListId>,
	/// Written types of exported values and aliases (methods are not structural).
	pub written_types: FxHashMap<NodeId, crate::analysis::WrittenType>,
	/// The written return types of functions, including generic projections.
	pub written_returns: FxHashMap<NodeId, crate::analysis::WrittenReturn>,
	/// Written field types, or projections of an owner's generic argument.
	pub written_fields: FxHashMap<(NodeId, Sym), crate::analysis::WrittenReturn>,
	/// Types owning method binders; the method's own generic list stays separate.
	pub function_owners: FxHashMap<NodeId, DeclKey>,
	/// The types declared inside its types (`Database.OpenError`), by type
	/// declaration (`method` is always false).
	pub nested_types: FxHashMap<NodeId, Vec<TypeFunction>>,
	/// The parameters of its functions other files may call, by declaration.
	pub parameters: FxHashMap<NodeId, Vec<Parameter>>,
	/// The fields written into copies of an implicit `Type<parameter>`,
	/// keyed by that hidden generic parameter. Callers must reject narrower
	/// field types even when the callee's tree has been dropped (`EF004`).
	pub implicit_writes: FxHashMap<ParamId, Vec<(Vec<Sym>, TypeId)>>,
	/// Generic parameter order and capabilities of externally callable functions.
	pub generic_functions: FxHashMap<NodeId, GenericFunction>,
	/// Key sets and derived types reached by the exported signatures.
	pub key_set_params: FxHashMap<ParamId, GenericKeySet>,
	pub derived_params: FxHashMap<ParamId, crate::analysis::DerivedParam>,
	/// Foreign key-set applications, and local parameters iterated by `@for`.
	pub key_set_applications: Vec<KeySetApplication>,
	pub key_loop_params: Vec<ParamId>,
	/// The C symbols of its `@extern` functions, by declaration.
	pub externs: FxHashMap<NodeId, String>,
	/// The values of its module-level `static`s, by declaration.
	pub statics: FxHashMap<NodeId, fa_interpreter::Value>,
	/// The default values of the fields of its types, by type declaration:
	/// each field with a default, and its value when the default is a
	/// constant expression (the typed literals of other files fill them in).
	pub field_defaults: FxHashMap<NodeId, Vec<(Sym, Option<fa_interpreter::Value>)>>,
	/// The names its declarations are exported as to the other modules of a
	/// program: the file's name, then the namespace path, joined with `$`
	/// (`counter$count`); `Type$name` for the functions of its types.
	pub export_names: FxHashMap<NodeId, String>,
	/// Its module-level declarations and functions, by declaration.
	pub declared: FxHashMap<NodeId, Declared>,
	/// The module-level declarations the whole file uses, by key, sorted.
	pub uses: Vec<DeclKey>,
	/// Every declaration other files or the program may name, by key.
	pub keys: FxHashMap<DeclKey, KeyedDecl>,
	/// Whether the analysis of the file read `target` or `configuration`.
	pub reads_target: bool,
	/// What the work given to `WorkerPool.submit` uses: the module-level
	/// declarations and functions it names or calls, with the span of the
	/// name or call (the program's worker rule, `CONC008`).
	pub worker_work: Vec<(DeclKey, crate::diagnostics::SourceSpan)>,
	/// Whether it is complete (the file's walk finished).
	pub complete: bool,
	/// The async summaries of its functions, by declaration node
	/// (`design/async.md` §2.1, `check/tasks.rs`).
	pub async_summaries: FxHashMap<NodeId, crate::analysis::AsyncSummary>,
	/// The relations declared by the file (relations across files): their
	/// collections are named by key or by owner type
	/// ([`CollectionRef::External`](crate::analysis::CollectionRef),
	/// `ExternalField`), so that every file whose values may be of their
	/// types writes and deletes them with both sides.
	pub relations: Vec<crate::analysis::Relation>,
	/// Aliases whose denoted type contains a reference before runtime erasure.
	pub reference_aliases: FxHashSet<NodeId>,
	/// Reference-containing generic defaults, by declaration and position.
	pub reference_defaults: FxHashMap<NodeId, Vec<bool>>,
	pub reference_layouts: FxHashMap<NodeId, crate::analysis::ReferenceLayout>,
	pub reference_origins: FxHashMap<NodeId, Vec<crate::analysis::ReferenceOrigin>>,
	pub reference_sensitive: FxHashMap<NodeId, Vec<ParamId>>,
}

/// The generic parameters reached by exported types, including binders
/// inside returned function values. Interface-only summaries keep these
/// parameters' metadata and discard the metadata of private local types.
pub(crate) fn reachable_params(
	types: &TypeStore,
	roots: impl IntoIterator<Item = TypeId>,
) -> FxHashSet<ParamId> {
	let mut seen = FxHashSet::default();
	let mut params = FxHashSet::default();
	let mut pending: Vec<TypeId> = roots.into_iter().collect();
	while let Some(ty) = pending.pop() {
		if !seen.insert(ty) {
			continue;
		}
		match types.kind(ty) {
			TypeKind::Param(param) => {
				params.insert(param);
				let info = types.param(param);
				pending.extend([info.constraint, info.default]);
			}
			TypeKind::Rec(_) => pending.extend(types.rec_target(ty)),
			TypeKind::Object(shape) => {
				pending.extend(types.shape_fields(shape).iter().map(|field| field.ty))
			}
			TypeKind::Tuple(list) | TypeKind::Union(list) | TypeKind::Fields(list) => {
				pending.extend(types.list_items(list))
			}
			TypeKind::Function(sig) => {
				let sig = types.sig(sig);
				pending.extend(types.params(sig.params).iter().map(|param| param.ty));
				pending.extend([sig.ret, sig.throws]);
				pending.extend(types.list_items(sig.generics));
				pending.extend(
					types
						.requirements(sig.requires)
						.iter()
						.map(|requirement| requirement.ty),
				);
			}
			TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
				pending.extend([item, key])
			}
			TypeKind::Pointer(item)
			| TypeKind::CArray(item, _)
			| TypeKind::Sync(_, item)
			| TypeKind::Iterator(item, _)
			| TypeKind::Ref(item, _) => pending.push(item),
			TypeKind::NamedUnion(nominal, _)
			| TypeKind::Member(nominal, _)
			| TypeKind::CStruct(nominal) => {
				let info = types.nominal(nominal);
				pending.extend(types.list_items(info.args));
				for index in 0..types.sym_list(info.members).len() {
					pending.push(types.payload(nominal, index as u16));
				}
			}
			_ => {}
		}
	}
	params
}

impl Interface {
	/// A method of this file's type. Anonymous exported namespaces may
	/// qualify the declaration key after the nominal type was created, so
	/// use its exact self type when the nominal key has no direct entry.
	pub fn type_method(
		&self,
		types: &crate::types::TypeStore,
		ty: TypeId,
		name: &str,
	) -> Option<&TypeFunction> {
		let crate::types::TypeKind::Object(shape) = types.kind(ty) else {
			return None;
		};
		let key = types.nominal(types.shape(shape).nominal?).key;
		self.keys
			.get(&key)
			.and_then(|declaration| self.type_functions.get(&declaration.node))
			.and_then(|methods| {
				methods
					.iter()
					.find(|method| method.method && types.syms.resolve(method.name) == name)
			})
			.or_else(|| {
				self.type_functions.values().flatten().find(|method| {
					if !method.method || types.syms.resolve(method.name) != name {
						return false;
					}
					let crate::types::TypeKind::Function(sig) = types.kind(method.ty) else {
						return false;
					};
					types
						.params(types.sig(sig).params)
						.first()
						.is_some_and(|parameter| parameter.ty == ty)
				})
			})
	}

	/// A hash of what other files read of the interface, for the
	/// incremental analysis: it changes when a declaration other files can
	/// see changes its kind or its type, or the value of a `static`. Built
	/// from the stable hashes of the types and of the declaration keys (no
	/// type is printed, no id or node is read): a declaration that only
	/// moves keeps its key, which the files using it name it by.
	pub fn hash(&self, types: &crate::types::TypeStore) -> u128 {
		use std::hash::BuildHasher;
		let kind_hash = |kind: SymbolKind| rustc_hash::FxBuildHasher.hash_one(kind) as u128;
		// (owner, key, kind, type): order-independent once sorted
		let mut parts: Vec<(u128, u128, u128, u128)> = Vec::with_capacity(
			self.members.len() + self.type_functions.len() + self.statics.len() + 1,
		);
		let by_node: FxHashMap<NodeId, DeclKey> = self
			.keys
			.iter()
			.map(|(&key, keyed)| (keyed.node, key))
			.collect();
		let key_hash = |node: NodeId| by_node.get(&node).map_or(0, |&key| types.decl_hash(key));
		for (&decl, &parameters) in &self.type_parameters {
			let hash = types
				.list_items(parameters)
				.iter()
				.fold(0u128, |hash, &ty| {
					let mut hash = hash.rotate_left(7) ^ types.stable_hash(ty);
					if let crate::types::TypeKind::Param(param) = types.kind(ty) {
						let info = types.param(param);
						hash ^= types.stable_hash(info.constraint).rotate_left(11)
							^ types.stable_hash(info.default).rotate_left(13)
							^ u128::from(info.value);
					}
					hash
				});
			parts.push((100, key_hash(decl), 0, hash));
		}
		for (&decl, written) in &self.written_types {
			parts.push((101, key_hash(decl), 0, written.stable_hash(types)));
		}
		for (&decl, written) in &self.written_returns {
			match written {
				crate::analysis::WrittenReturn::Named(written) => {
					parts.push((102, key_hash(decl), 0, written.stable_hash(types)))
				}
				crate::analysis::WrittenReturn::Generic(parameter) => {
					parts.push((103, key_hash(decl), u128::from(*parameter), 0))
				}
			}
		}
		for (&(decl, name), written) in &self.written_fields {
			let name = rustc_hash::FxBuildHasher.hash_one(types.syms.resolve(name)) as u128;
			let origin = match written {
				crate::analysis::WrittenReturn::Named(written) => written.stable_hash(types),
				crate::analysis::WrittenReturn::Generic(index) => u128::from(*index),
			};
			parts.push((104, key_hash(decl), name, origin));
		}
		for (&function, &owner) in &self.function_owners {
			parts.push((105, key_hash(function), types.decl_hash(owner), 0));
		}
		for exported in self.members.values().chain(&self.export) {
			parts.push((
				1,
				types.decl_hash(exported.key),
				kind_hash(exported.kind),
				types.stable_hash(exported.ty)
					^ types
						.list_items(exported.generics)
						.iter()
						.fold(0, |hash: u128, &param| {
							hash.wrapping_mul(31).wrapping_add(types.stable_hash(param))
						}),
			));
		}
		for (&decl, functions) in &self.type_functions {
			let owner = key_hash(decl);
			for function in functions {
				parts.push((
					owner,
					types.decl_hash(function.key),
					u128::from(function.method),
					types.stable_hash(function.ty),
				));
			}
		}
		for (&decl, nested) in &self.nested_types {
			let owner = key_hash(decl);
			for nested in nested {
				parts.push((
					owner,
					types.decl_hash(nested.key),
					6,
					types.stable_hash(nested.ty),
				));
			}
		}
		for params in self.parameters.values() {
			for param in params {
				parts.push((
					2,
					types.decl_hash(param.key),
					u128::from(param.has_default) | u128::from(param.only_called) << 1,
					types.stable_hash(param.ty),
				));
			}
		}
		for (&param, writes) in &self.implicit_writes {
			let info = types.param(param);
			let owner = types.decl_hash(info.owner) ^ u128::from(info.index);
			for (path, written) in writes {
				let names: Vec<&str> = path.iter().map(|&name| types.syms.resolve(name)).collect();
				parts.push((
					7,
					owner,
					rustc_hash::FxBuildHasher.hash_one(names) as u128,
					types.stable_hash(*written),
				));
			}
		}
		let param_hash = |param| {
			let info = types.param(param);
			types.decl_hash(info.owner).rotate_left(16) ^ u128::from(info.index)
		};
		for (&decl, function) in &self.generic_functions {
			let generics = types
				.list_items(function.generics)
				.iter()
				.fold(0u128, |hash, &ty| {
					hash.wrapping_mul(31).wrapping_add(types.stable_hash(ty))
				});
			parts.push((8, key_hash(decl), 0, generics));
			for (index, requirement) in function.requirements.iter().enumerate() {
				let (kind, value) = match *requirement {
					GenericRequirement::Value(param) => (1, param_hash(param)),
					GenericRequirement::Method(param, name) => (
						2,
						param_hash(param)
							^ rustc_hash::FxBuildHasher.hash_one(types.syms.resolve(name)) as u128,
					),
				};
				parts.push((8, key_hash(decl), ((index as u128) << 8) | kind, value));
			}
		}
		for (&param, info) in &self.key_set_params {
			parts.push((
				9,
				param_hash(param),
				u128::from(info.deep),
				param_hash(info.subject),
			));
		}
		for (&param, info) in &self.derived_params {
			parts.push((
				10,
				param_hash(param),
				u128::from(info.pick),
				param_hash(info.subject) ^ param_hash(info.keys).rotate_left(7),
			));
		}
		for application in &self.key_set_applications {
			parts.push((
				11,
				param_hash(application.param),
				types.stable_hash(application.subject),
				types.stable_hash(application.key_set),
			));
		}
		for &param in &self.key_loop_params {
			parts.push((12, param_hash(param), 0, 0));
		}
		// A Param type's stable hash deliberately excludes its late-bound
		// constraint and default. They are nevertheless facts every caller
		// reads, including binders of a returned generic function value.
		let roots = self
			.members
			.values()
			.chain(&self.export)
			.flat_map(|exported| {
				std::iter::once(exported.ty)
					.chain(types.list_items(exported.generics).iter().copied())
			})
			.chain(
				self.type_functions
					.values()
					.flatten()
					.map(|function| function.ty),
			)
			.chain(self.nested_types.values().flatten().map(|nested| nested.ty))
			.chain(
				self.generic_functions
					.values()
					.flat_map(|function| types.list_items(function.generics).iter().copied()),
			)
			.filter(|&ty| types.flags(ty).contains(crate::types::TypeFlags::HAS_PARAM));
		for param in reachable_params(types, roots) {
			let info = types.param(param);
			parts.push((
				13,
				param_hash(param),
				types.stable_hash(info.constraint),
				types.stable_hash(info.default).rotate_left(1) ^ u128::from(info.value),
			));
		}
		for (&decl, value) in &self.statics {
			let text = format!("{value:?}");
			parts.push((
				3,
				key_hash(decl),
				rustc_hash::FxBuildHasher.hash_one(text) as u128,
				0,
			));
		}
		for (&decl, defaults) in &self.field_defaults {
			for (name, value) in defaults {
				let text = format!("{}={value:?}", types.syms.resolve(*name));
				parts.push((
					5,
					key_hash(decl),
					rustc_hash::FxBuildHasher.hash_one(text) as u128,
					0,
				));
			}
		}
		for &decl in &self.reference_aliases {
			parts.push((14, key_hash(decl), 1, 0));
		}
		for (&decl, defaults) in &self.reference_defaults {
			for (index, &reference) in defaults.iter().enumerate() {
				parts.push((15, key_hash(decl), index as u128, u128::from(reference)));
			}
		}
		for (&decl, &layout) in &self.reference_layouts {
			let value = match layout {
				crate::analysis::ReferenceLayout::Owner => 1,
				crate::analysis::ReferenceLayout::General => 2,
				crate::analysis::ReferenceLayout::Field(name) => {
					(rustc_hash::FxBuildHasher.hash_one(types.syms.resolve(name)) as u128) << 2 | 3
				}
			};
			parts.push((16, key_hash(decl), 0, value));
		}
		for (&decl, params) in &self.reference_sensitive {
			for &param in params {
				parts.push((17, key_hash(decl), 0, param_hash(param)));
			}
		}
		for (&decl, origins) in &self.reference_origins {
			for origin in origins {
				parts.push((
					18,
					key_hash(decl),
					types.path_hash(origin.path),
					u128::from(origin.descendant) | u128::from(origin.readonly) << 1,
				));
			}
		}
		parts.push((4, u128::from(self.reads_target), 0, 0));
		// the relations: their kind, types, sides and collections
		for relation in &self.relations {
			use crate::analysis::CollectionRef;
			let name = |sym: crate::ids::Sym| {
				rustc_hash::FxBuildHasher.hash_one(types.syms.resolve(sym)) as u128
			};
			let collection = |reference: CollectionRef| match reference {
				CollectionRef::External { key } => types.decl_hash(key),
				CollectionRef::ExternalField { owner, name: field } => {
					types.stable_hash(owner) ^ name(field)
				}
				CollectionRef::Field { .. } | CollectionRef::Module { .. } => 0,
			};
			parts.push((
				6,
				types.stable_hash(relation.holder_type) ^ name(relation.direct),
				kind_hash(SymbolKind::Let) ^ relation.kind as u128,
				types.stable_hash(relation.target_type)
					^ relation.reverse.map_or(0, name)
					^ collection(relation.holder).rotate_left(7)
					^ collection(relation.target).rotate_left(13),
			));
		}
		parts.sort_unstable();
		let mut hasher = crate::types::store::StableHasher::new();
		for (owner, key, kind, ty) in parts {
			hasher.hash(owner).hash(key).hash(kind).hash(ty);
		}
		// the async summaries: whether a function suspends, starts tasks,
		// and where it can be called blocking (design/async.md §2.1)
		let mut summaries: Vec<(u128, u8)> = self
			.async_summaries
			.iter()
			.map(|(&decl, summary)| {
				let bits = u8::from(summary.suspends) | (u8::from(summary.starts) << 1);
				(key_hash(decl), bits | (summary.blocking.bits() << 2))
			})
			.collect();
		summaries.sort_unstable();
		for (decl, bits) in summaries {
			hasher.hash(decl).hash(u128::from(bits));
		}
		hasher.finish()
	}

	/// The declaration node of a key in this version of the file.
	pub fn node_of(&self, key: DeclKey) -> Option<NodeId> {
		self.keys.get(&key).map(|keyed| keyed.node)
	}

	/// The kind and name of a declaration of the file, from its interface.
	pub fn declared(&self, decl: NodeId) -> Option<&Declared> {
		self.declared.get(&decl)
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
	/// Whether the program runs in asynchronous mode (`design/async.md`
	/// A5–A7): `build.asynchronous` when configured, otherwise detected from
	/// the entry's transitive summary (A6). Off, the whole program is one
	/// blocking call of `main`: only synchronous instances exist.
	pub asynchronous: bool,
	/// Why [`Program::asynchronous`] is what it is (`fa build -v`): "on:
	/// main awaits Mutex.lock", "off: nothing starts a task or waits".
	pub asynchronous_reason: String,
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
