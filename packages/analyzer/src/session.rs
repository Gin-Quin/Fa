//! The analysis session: files, their trees and analyses, the type store.
//!
//! A file is parsed and its parse diagnostics are converted at the boundary;
//! [`Session::analyze_file`] runs the walk (`check/`), which fills the rest of
//! its [`Analysis`].

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rustc_hash::{FxHashMap, FxHashSet};

use fa_parser::TypedSyntaxTree;

use crate::analysis::Analysis;
use crate::diagnostics::{Code, Diagnostic, LineIndex, Location, SourceSpan, codes};
use crate::ids::{DeclKey, FileId, ModuleId, NodeId, SymbolId, TypeId};
use crate::modules::{FileRole, Interface, ModuleResolver};
use crate::scan::FileScan;
use crate::symbols::Resolution;
use crate::types::TypeStore;

/// One parse of one file and its analysis.
pub struct SourceFile {
	/// The module path (`main` for a standalone file).
	pub module: String,
	/// The module path, interned.
	pub module_id: ModuleId,
	/// The parse tree, shared with the walk.
	pub tree: Arc<TypedSyntaxTree>,
	/// The side tables of the analysis.
	pub analysis: Analysis,
	/// Whether the walk ran (or is running).
	pub analyzed: bool,
	/// Whether the walk finished.
	pub done: bool,
	/// What the file is (a program, a module, a test file…).
	pub role: FileRole,
	/// What other files see of it (`modules.rs`).
	pub interface: Interface,
	/// Whether its walk read a declaration of a file whose analysis was in
	/// progress and not available yet: the program analysis walks it again.
	pub deferred: bool,
	/// The modules its lookups touched, found or not (incremental analysis).
	pub reads: Vec<String>,
	/// For a configuration file: its value, when it is a valid literal (D37).
	pub configuration: Option<crate::configuration::ConfigValue>,
	/// What the module system needs of the file before its walk (`scan.rs`).
	pub scan: FileScan,
}

/// Everything the analysis of a program shares: files and types.
pub struct Session {
	/// The types of every file.
	pub types: TypeStore,
	files: Vec<SourceFile>,
	/// The type of the built-ins the backends read without interning (see
	/// `with_types`), or of every built-in in a parallel run.
	prelude: Vec<Option<TypeId>>,
	/// Whether `prelude` holds the value types of the built-ins too.
	prelude_values: bool,
	/// Set to stop a running analysis (the language server sets it on an edit).
	cancel: Arc<AtomicBool>,
	/// The module system of a multi-file program (`None`: a lone file).
	resolver: Option<Arc<dyn ModuleResolver>>,
	/// The current file of each module path.
	modules: FxHashMap<String, FileId>,
	/// The file and declaration of each declared type (methods of another
	/// file's types, `check/modules.rs`).
	type_origins: FxHashMap<TypeId, Vec<DeclKey>>,
	/// Compile-time evaluation: the target, lowered functions, globals.
	pub(crate) comptime: crate::comptime::State,
	/// In a parallel run: what the units published (`pipeline.rs`).
	shared: Option<Arc<crate::pipeline::Shared>>,
	/// The reverse edges of the module graph: the loaded modules whose scan
	/// reads each module (§4 #10), kept up to date as versions are added.
	readers: FxHashMap<String, Vec<String>>,
	/// The module of the package's entry (`application/main`): its exported
	/// function infers its effects, like a program's (D51).
	entry: Option<String>,
}

impl Default for Session {
	fn default() -> Self {
		Self::new()
	}
}

impl Session {
	/// An empty session.
	pub fn new() -> Self {
		Self::with_types(TypeStore::new())
	}

	/// An empty session over the global type store of another one (an
	/// analysis for another target, reusing the other one's interfaces,
	/// `pipeline.rs`).
	pub fn sharing_types_of(other: &Session) -> Self {
		Self::with_types(TypeStore::view(other.types.global()))
	}

	/// An empty session over a view of the type store.
	///
	/// Only the built-in errors are built here (the backends read them
	/// whether the program names them or not: the runtime raises them).
	/// The other built-ins are built by the walk when it reads them
	/// (hash-consed), so that a session costs little to create (a lone file,
	/// a test, a language-server document). A parallel run builds them all
	/// once, in the global store ([`Session::global_prelude`]).
	fn with_types(types: TypeStore) -> Self {
		let mut types = types;
		let mut prelude = vec![None; crate::builtins::all().len()];
		for error in crate::builtins::errors::ERRORS {
			if let Some(id) = crate::builtins::by_path(error.name) {
				prelude[id.index()] = crate::builtins::denoted_type(&mut types, id);
			}
		}
		Self {
			types,
			files: Vec::new(),
			prelude,
			prelude_values: false,
			cancel: Arc::new(AtomicBool::new(false)),
			resolver: None,
			modules: FxHashMap::default(),
			type_origins: FxHashMap::default(),
			comptime: crate::comptime::State::default(),
			shared: None,
			readers: FxHashMap::default(),
			entry: None,
		}
	}

	/// A session for one unit of a parallel run (`pipeline.rs`): a new view
	/// of this session's global store, its resolver, target, configuration
	/// and cancel flag, the prelude of the global store, what the units
	/// published.
	pub(crate) fn unit_session(
		&self,
		shared: Arc<crate::pipeline::Shared>,
		prelude: Vec<Option<TypeId>>,
	) -> Session {
		let mut comptime = crate::comptime::State::default();
		comptime.target = self.comptime.target.clone();
		comptime.freestanding = self.comptime.freestanding;
		comptime.configuration = self.comptime.configuration.clone();
		Session {
			types: TypeStore::view(self.types.global()),
			files: Vec::new(),
			prelude,
			prelude_values: true,
			cancel: Arc::clone(&self.cancel),
			resolver: self.resolver.clone(),
			modules: FxHashMap::default(),
			type_origins: FxHashMap::default(),
			comptime,
			shared: Some(shared),
			readers: FxHashMap::default(),
			entry: self.entry.clone(),
		}
	}

	/// Reads what the units of a parallel run published (`None`: stop).
	/// The language server points the kept sessions of its open documents
	/// to the newest run's interfaces.
	pub fn set_shared(&mut self, shared: Option<Arc<crate::pipeline::Shared>>) {
		self.shared = shared;
	}

	/// The resolver, shared.
	pub(crate) fn resolver_arc(&self) -> Option<Arc<dyn ModuleResolver>> {
		self.resolver.clone()
	}

	/// The prelude made global (merged into the global store): the units of
	/// a parallel run share it.
	pub(crate) fn global_prelude(&mut self) -> Vec<Option<TypeId>> {
		if !self.prelude_values {
			let types = &mut self.types;
			self.prelude = (0..crate::builtins::all().len())
				.map(|index| {
					let id = crate::ids::BuiltinId::new(index);
					crate::builtins::value_type(types, id)
						.or_else(|| crate::builtins::denoted_type(types, id))
				})
				.collect();
			self.prelude_values = true;
		}
		let prelude = self.prelude.clone();
		let mut merger = self.types.merger(None);
		let merged = prelude
			.into_iter()
			.map(|ty| ty.map(|ty| merger.ty(ty)))
			.collect();
		merger.generic_declarations();
		merged
	}

	/// Adds a parsed module (its parse diagnostics on this file).
	pub(crate) fn add_module(&mut self, module: &str, parsed: Parsed) -> FileId {
		self.add_parsed(module, parsed)
	}

	/// Analyzes the files of a unit: again, reading what the previous round
	/// published, as long as one of them read a declaration of another one
	/// not available yet (a cycle).
	pub(crate) fn settle_unit(&mut self, files: &[FileId]) {
		for &file in files {
			self.analyze_file(file);
		}
		for _ in 1..4 {
			if !files.iter().any(|&file| self.files[file.index()].deferred) {
				break;
			}
			for &file in files {
				self.reset_analysis(file);
			}
			for &file in files {
				self.analyze_file(file);
			}
		}
	}

	/// Publishes the interfaces of a unit's files: what they reach is
	/// merged into the global store, their ids made global (P1, §1.3).
	pub(crate) fn publish_unit(&mut self, files: &[FileId]) {
		let Some(shared) = self.shared.clone() else {
			return;
		};
		let first = files.first().map(|&file| self.module_id(file));
		let mut merger = self.types.merger(first);
		let mut published = Vec::with_capacity(files.len());
		for &file in files {
			let module = merger.module(self.files[file.index()].module_id);
			let interface = merge_interface(&mut merger, &self.files[file.index()].interface);
			published.push((module, interface));
		}
		let own: FxHashSet<ModuleId> = files.iter().map(|&file| self.module_id(file)).collect();
		let mut origins: Vec<(TypeId, DeclKey)> = Vec::new();
		for (&ty, found) in &self.type_origins {
			for &key in found {
				if own.contains(&self.types.decl_key_data(key).module) {
					origins.push((merger.ty(ty), merger.decl_key(key)));
				}
			}
		}
		merger.generic_declarations();
		drop(merger);
		origins.sort_unstable_by_key(|&(ty, _)| ty.0);
		for group in origins.chunk_by(|a, b| a.0 == b.0) {
			let keys: Vec<DeclKey> = group.iter().map(|&(_, key)| key).collect();
			shared
				.origins
				.add(group[0].0, &keys, |a, b| self.key_cmp(a, b));
		}
		for (module, interface) in published {
			if let Some(slot) = shared.interfaces.get(&module) {
				let _ = slot.set(Arc::new(interface));
			}
		}
	}

	/// The target the analysis is for (decision D22): the value of the
	/// static `target`, and the arithmetic of compile-time code (D49).
	/// `bun` by default. Set it before analyzing.
	pub fn set_target(&mut self, target: &str) {
		// `native-freestanding`: the language's `target` is `native`, and
		// what needs a C library doesn't exist (`AN004`)
		self.comptime.freestanding = target == "native-freestanding";
		self.comptime.target = if self.comptime.freestanding {
			"native".to_string()
		} else {
			target.to_string()
		};
	}

	/// The module of the package's entry (`application/main`): its exported
	/// function infers its effects (D51). Set it before analyzing.
	pub fn set_entry(&mut self, entry: Option<&str>) {
		self.entry = entry.map(str::to_string);
	}

	/// Whether the program declares reactivity anywhere (a `reactive` value
	/// or field, a `derived` value, a `watch` block), from the scans of its
	/// files: a whole-program fact. Without it, no write anywhere notifies
	/// observers (check/reactivity.rs).
	pub(crate) fn program_has_reactivity(&self) -> bool {
		self.files.iter().any(|file| file.scan.has_reactivity)
			|| self
				.shared
				.as_ref()
				.is_some_and(|shared| shared.scans.values().any(|scan| scan.has_reactivity))
	}

	/// Whether a file is the package's entry module.
	pub(crate) fn is_entry(&self, file: FileId) -> bool {
		self.entry.as_deref() == Some(self.module(file))
	}

	/// Whether the program is built without a C library
	/// (`native-freestanding`).
	pub fn freestanding(&self) -> bool {
		self.comptime.freestanding
	}

	/// The target the analysis is for.
	pub fn target(&self) -> &str {
		&self.comptime.target
	}

	/// The configuration merged for the target: the value of the static
	/// `configuration` (advanced brief §12.2). Set it before analyzing.
	pub fn set_configuration(&mut self, configuration: fa_interpreter::Value) {
		self.comptime.configuration = Some(configuration);
	}

	/// Whether the analysis of a file read `target` or `configuration`: it
	/// depends on the target, and `fa check` analyzes it for each target.
	pub fn depends_on_target(&self, file: FileId) -> bool {
		self.analysis(file).reads_target
	}

	/// The text of a build input, by its path relative to the root.
	pub(crate) fn read_input(
		&self,
		path: &str,
	) -> Result<String, crate::comptime::inputs::InputError> {
		match &self.resolver {
			Some(resolver) => resolver.read_input(path),
			None => Err(crate::comptime::inputs::InputError::NotFound),
		}
	}

	/// The entries of a folder of the root, sorted by name.
	pub(crate) fn list_input(
		&self,
		path: &str,
	) -> Result<Vec<crate::comptime::inputs::DirectoryEntry>, crate::comptime::inputs::InputError>
	{
		match &self.resolver {
			Some(resolver) => resolver.list_input(path),
			None => Err(crate::comptime::inputs::InputError::NotFound),
		}
	}

	/// Sets the module system: the other files of the program are found and
	/// read through it (design §1).
	pub fn set_resolver(&mut self, resolver: Box<dyn ModuleResolver>) {
		self.resolver = Some(Arc::from(resolver));
	}

	/// The module system, if any.
	pub fn resolver(&self) -> Option<&dyn ModuleResolver> {
		self.resolver.as_deref()
	}

	/// The current file of a module, parsed from the resolver's text on first
	/// use; `None` when the resolver has no such module.
	pub fn file_of_module(&mut self, module: &str) -> Option<FileId> {
		if let Some(&file) = self.modules.get(module) {
			return Some(file);
		}
		let text = self.resolver.as_ref()?.source(module)?;
		Some(self.set_source(module, text))
	}

	/// Records the declaration of a type (its key).
	pub(crate) fn record_type_origin(&mut self, ty: TypeId, key: DeclKey) {
		let origins = self.type_origins.entry(ty).or_default();
		if !origins.contains(&key) {
			origins.push(key);
		}
	}

	/// The declarations of a type, in every module that declared one of
	/// its structure (two files may declare the same object type), in the
	/// order of their module paths. Only the modules `keep` accepts are
	/// listed (a type of the structure may be declared by every file of a
	/// package: a file looks at the ones it may read). Lazy: the published
	/// declarations are sorted already, a caller that stops at the first
	/// that fits reads no further.
	pub(crate) fn type_origins<'s>(
		&'s self,
		ty: TypeId,
		keep: impl Fn(ModuleId) -> bool + 's,
	) -> impl Iterator<Item = DeclKey> + 's {
		let kept = move |key: DeclKey| keep(self.types.decl_key_data(key).module);
		let mut own: Vec<DeclKey> = self
			.type_origins
			.get(&ty)
			.map_or(&[][..], Vec::as_slice)
			.iter()
			.copied()
			.filter(|&key| kept(key))
			.collect();
		own.sort_by(|&a, &b| self.key_cmp(a, b));
		let published = self
			.shared
			.as_ref()
			.and_then(|shared| shared.origins.get(ty))
			.unwrap_or_else(|| Arc::from([]));
		let mut own = own.into_iter().peekable();
		let mut next = 0;
		let mut last: Option<DeclKey> = None;
		std::iter::from_fn(move || {
			loop {
				while next < published.len() && !kept(published[next]) {
					next += 1;
				}
				let found = match (own.peek().copied(), published.get(next).copied()) {
					(None, None) => return None,
					(Some(local), Some(other))
						if self.key_cmp(other, local) == std::cmp::Ordering::Less =>
					{
						next += 1;
						other
					}
					(Some(_), _) => own.next()?,
					(None, Some(other)) => {
						next += 1;
						other
					}
				};
				if last != Some(found) {
					last = Some(found);
					return Some(found);
				}
			}
		})
	}

	/// The current file of a module id, if it is loaded.
	pub fn file_of(&self, module: ModuleId) -> Option<FileId> {
		self.loaded_module(self.types.module_path(module))
	}

	/// The interface of a module: what its walk published so far when its
	/// file is in this session, else what its unit published (a parallel
	/// run, `pipeline.rs`).
	pub fn interface(&self, module: ModuleId) -> Option<&Interface> {
		// published: the global version (a module loaded again keeps it)
		if let Some(shared) = &self.shared
			&& let Some(interface) = shared.interface(module)
		{
			return Some(interface);
		}
		self.file_of(module)
			.map(|file| &self.files[file.index()].interface)
	}

	/// Whether a module of another unit of a parallel run can be loaded
	/// into this unit's session (its compile-time code reads it).
	pub(crate) fn can_rehydrate(&self, module: ModuleId) -> bool {
		self.file_of(module).is_none()
			&& self
				.shared
				.as_ref()
				.is_some_and(|shared| shared.interface(module).is_some())
	}

	/// Loads a published module of another unit again into this unit's
	/// session, and analyzes it against the published interfaces
	/// (deterministic): compile-time code lowers its functions
	/// (design/parallel-pipeline.md §3.1, "rehydrate"). It is neither
	/// reported nor published again.
	pub(crate) fn rehydrate(&mut self, module: ModuleId) -> bool {
		if !self.can_rehydrate(module) {
			return false;
		}
		let path = self.types.module_path(module).to_string();
		let Some(resolver) = self.resolver.clone() else {
			return false;
		};
		let Some(text) = resolver.source(&path) else {
			return false;
		};
		let file = FileId::new(self.files.len());
		let parsed = parse_module(file, &path, text, Some(&*resolver));
		let file = self.add_parsed(&path, parsed);
		self.analyze_file(file);
		true
	}

	/// The scan of a module: its file's, or the run's (`pipeline.rs`).
	pub fn scan_of(&self, module: &str) -> Option<&FileScan> {
		match self.loaded_module(module) {
			Some(file) => Some(&self.files[file.index()].scan),
			None => self.shared.as_ref()?.scans.get(module).map(|scan| &**scan),
		}
	}

	/// The modules a file may read, directly or through the modules it
	/// reads (the edges of the scans, `scan.rs`).
	pub fn dependency_closure(&self, file: FileId) -> rustc_hash::FxHashSet<ModuleId> {
		self.modules_read(&[file])
	}

	/// The modules files may read, directly or not.
	fn modules_read(&self, files: &[FileId]) -> rustc_hash::FxHashSet<ModuleId> {
		let mut found = rustc_hash::FxHashSet::default();
		let mut seen: rustc_hash::FxHashSet<&str> = rustc_hash::FxHashSet::default();
		let mut stack: Vec<&str> = files
			.iter()
			.map(|&file| self.files[file.index()].module.as_str())
			.collect();
		while let Some(current) = stack.pop() {
			let Some(scan) = self.scan_of(current) else {
				continue;
			};
			for dependency in &scan.dependencies {
				if seen.insert(dependency) {
					if let Some(module) = self.types.existing_module(dependency) {
						found.insert(module);
					}
					stack.push(dependency);
				}
			}
		}
		found
	}

	/// The modules the names of global types are shown from in a unit: the
	/// modules its files may read, and theirs.
	pub(crate) fn visible_modules(&self, files: &[FileId]) -> Arc<rustc_hash::FxHashSet<ModuleId>> {
		let mut modules = self.modules_read(files);
		modules.extend(files.iter().map(|&file| self.module_id(file)));
		Arc::new(modules)
	}

	/// The current file of the module an [`Resolution::External`]
	/// designates, and what its interface says of the declaration.
	pub fn external_declared(
		&self,
		resolution: Resolution,
	) -> Option<(FileId, &crate::modules::Declared)> {
		let (module, decl) = self.external_node(resolution)?;
		let file = self.file_of(module)?;
		Some((
			file,
			self.files[file.index()].interface.declared.get(&decl)?,
		))
	}

	/// The module and the declaration node, in the current version of its
	/// file, of what an [`Resolution::External`] designates.
	pub fn external_node(&self, resolution: Resolution) -> Option<(ModuleId, NodeId)> {
		let Resolution::External(key) = resolution else {
			return None;
		};
		let (module, keyed) = self.keyed(key)?;
		Some((module, keyed.node))
	}

	/// Where a declaration key is in the current version of its module (its
	/// file's interface: the one of this session, else the published one).
	pub fn keyed(&self, key: DeclKey) -> Option<(ModuleId, crate::modules::KeyedDecl)> {
		let module = self.types.decl_key_data(key).module;
		if let Some(file) = self.file_of(module)
			&& let Some(&keyed) = self.files[file.index()].interface.keys.get(&key)
		{
			return Some((module, keyed));
		}
		let shared = self.shared.as_ref()?.interface(module)?;
		Some((module, *shared.keys.get(&key)?))
	}

	/// The order of declaration keys in messages and outputs: module path,
	/// names, disambiguator (never their ids, P3). Compares the texts in
	/// place: it runs for every method looked up in another file.
	pub(crate) fn key_cmp(&self, a: DeclKey, b: DeclKey) -> std::cmp::Ordering {
		if a == b {
			return std::cmp::Ordering::Equal;
		}
		let (a, b) = (self.types.decl_key_data(a), self.types.decl_key_data(b));
		let names = |data: &crate::types::DeclKeyData| {
			self.types
				.sym_list(data.names)
				.iter()
				.map(|&name| self.types.syms.resolve(name))
		};
		self.types
			.module_path(a.module)
			.cmp(self.types.module_path(b.module))
			.then_with(|| names(&a).cmp(names(&b)))
			.then(a.disambiguator.cmp(&b.disambiguator))
	}

	/// The module id of a file.
	pub fn module_id(&self, file: FileId) -> ModuleId {
		self.files[file.index()].module_id
	}

	/// Drops the tree and the analysis of a file, keeping its interface:
	/// what the other files, the program and the backends of the other
	/// files read of it (design/parallel-pipeline.md §2.2).
	pub fn forget_tree(&mut self, file: FileId) {
		if let Some(source) = self.files.get_mut(file.index()) {
			source.tree = Arc::new(fa_parser::parse(""));
			source.analysis = Analysis::new(file, source.tree.nodes.len());
		}
	}

	/// The loaded modules whose scans read a module (the reverse edges of
	/// the module graph), in path order.
	pub fn readers(&self, module: &str) -> Vec<&str> {
		let mut readers: Vec<&str> = self
			.readers
			.get(module)
			.map(|readers| readers.iter().map(String::as_str).collect())
			.unwrap_or_default();
		readers.sort_unstable();
		readers
	}

	/// The current file of a module, if it is loaded.
	pub fn loaded_module(&self, module: &str) -> Option<FileId> {
		self.modules.get(module).copied()
	}

	/// The role of a file.
	pub fn role(&self, file: FileId) -> FileRole {
		self.files[file.index()].role
	}

	/// The name a file is known by, which designates its export inside it
	/// (`math` for `library/math.fa`); `None` for a lone file, a test file or
	/// a configuration file.
	pub fn own_name(&self, file: FileId) -> Option<String> {
		let module = &self.files[file.index()].module;
		self.resolver.as_ref()?.name(module)
	}

	/// The path of a file as shown in messages (`library/math.fa`).
	pub fn display_path(&self, file: FileId) -> String {
		let module = &self.files[file.index()].module;
		match &self.resolver {
			Some(resolver) => resolver.path(module),
			None => format!("{module}.fa"),
		}
	}

	/// The path of a module as shown in messages (`library/math.fa`).
	pub fn display_path_of(&self, module: ModuleId) -> String {
		let module = self.types.module_path(module);
		match &self.resolver {
			Some(resolver) => resolver.path(module),
			None => format!("{module}.fa"),
		}
	}

	/// The source file record of a file (its interface, its state).
	pub fn source_file(&self, file: FileId) -> &SourceFile {
		&self.files[file.index()]
	}

	/// The source file record of a file, to update it.
	pub(crate) fn source_file_mut(&mut self, file: FileId) -> &mut SourceFile {
		&mut self.files[file.index()]
	}

	/// The file and the symbol an [`Resolution::External`] designates: the
	/// current file of the module, and the symbol of the declaration there.
	pub fn external(&self, resolution: Resolution) -> Option<(FileId, SymbolId)> {
		let (module, decl) = self.external_node(resolution)?;
		self.external_decl(module, decl).and_then(|(file, decl)| {
			Some((file, self.analysis(file).symbols.symbol_of_decl(decl)?))
		})
	}

	/// The current file of a module id, with a declaration node of it.
	pub fn external_decl(&self, module: ModuleId, decl: NodeId) -> Option<(FileId, NodeId)> {
		let path = self.types.module_path(module);
		let file = self.loaded_module(path)?;
		Some((file, decl))
	}

	/// The type a built-in error denotes (`OverflowError`); after
	/// [`Session::global_prelude`], the type of every built-in: its value
	/// type (`console.log`) or the type it denotes (`Integer`), see
	/// `with_types`. `None` for a namespace.
	pub fn builtin_type(&self, id: crate::ids::BuiltinId) -> Option<TypeId> {
		self.prelude.get(id.index()).copied().flatten()
	}

	/// The type of a built-in, displayed: its value type (`console.log`,
	/// `arguments`), or the type it denotes (`Integer`); `None` for a
	/// namespace. A type not built yet is built in a scratch view of the
	/// store (`with_types`).
	pub fn builtin_type_text(&self, id: crate::ids::BuiltinId) -> Option<String> {
		if let Some(ty) = self.builtin_type(id) {
			return Some(self.display_type(ty));
		}
		let mut scratch = TypeStore::view(self.types.global());
		let ty = crate::builtins::value_type(&mut scratch, id)
			.or_else(|| crate::builtins::denoted_type(&mut scratch, id))?;
		Some(scratch.display(ty))
	}

	/// The flag that stops a running analysis when set (checked between
	/// declarations). Clear it before analyzing again.
	pub fn cancel_flag(&self) -> Arc<AtomicBool> {
		Arc::clone(&self.cancel)
	}

	/// Shares a flag owned by the caller (the language server keeps one flag
	/// across sessions rebuilt from the open files).
	pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
		self.cancel = flag;
	}

	/// Drops the tree and the analysis of an old version of a file (its
	/// [`FileId`] stays valid, with an empty text).
	pub fn forget(&mut self, file: FileId) {
		if let Some(source) = self.files.get_mut(file.index()) {
			source.tree = Arc::new(fa_parser::parse(""));
			source.analysis = Analysis::new(file, source.tree.nodes.len());
			source.analyzed = true;
			source.reads = Vec::new();
		}
		self.forget_comptime(file);
	}

	/// Drops the lowered functions and the computed values of a file (a
	/// language server session edits a file thousands of times).
	fn forget_comptime(&mut self, file: FileId) {
		let comptime = &mut self.comptime;
		comptime.functions.retain(|key, _| key.0 != file.0);
		comptime.globals.retain(|key, _| key.0 != file.0);
		comptime.purity.retain(|key, _| key.0 != file.0);
	}

	/// The number of lowered functions and computed values kept for
	/// compile-time evaluation (the language server checks it stays bounded).
	pub fn comptime_entries(&self) -> usize {
		self.comptime.functions.len() + self.comptime.globals.len() + self.comptime.purity.len()
	}

	/// The number of files set, versions included.
	pub fn file_count(&self) -> usize {
		self.files.len()
	}

	/// Parses a version of a module's text: a new [`FileId`] per version.
	pub fn set_source(&mut self, module: &str, text: Arc<str>) -> FileId {
		let file = FileId::new(self.files.len());
		let parsed = parse_module(file, module, text, self.resolver.as_deref());
		self.add_parsed(module, parsed)
	}

	/// Reads and parses the modules not loaded yet, in parallel (the
	/// resolver gives their texts), then adds them in the order given, so
	/// that their [`FileId`]s don't depend on the threads. A module the
	/// resolver can't read is skipped.
	pub fn load_modules(&mut self, modules: &[String]) {
		use rayon::prelude::*;
		let Some(resolver) = self.resolver.as_deref() else {
			return;
		};
		let mut seen = rustc_hash::FxHashSet::default();
		let wanted: Vec<&String> = modules
			.iter()
			.filter(|module| !self.modules.contains_key(*module) && seen.insert(*module))
			.collect();
		let base = self.files.len();
		let parsed: Vec<(&String, Parsed)> = crate::parallel::install(|| {
			let texts: Vec<(&String, Arc<str>)> = wanted
				.par_iter()
				.filter_map(|&module| Some((module, resolver.source(module)?)))
				.collect();
			texts
				.into_par_iter()
				.enumerate()
				.map(|(index, (module, text))| {
					let file = FileId::new(base + index);
					(module, parse_module(file, module, text, Some(resolver)))
				})
				.collect()
		});
		let parsed: Vec<(String, Parsed)> = parsed
			.into_iter()
			.map(|(module, parsed)| (module.clone(), parsed))
			.collect();
		for (module, parsed) in parsed {
			self.add_parsed(&module, parsed);
		}
	}

	/// Adds a parsed version of a module.
	fn add_parsed(&mut self, module: &str, parsed: Parsed) -> FileId {
		let file = FileId::new(self.files.len());
		let Parsed {
			tree,
			diagnostics,
			scan,
		} = parsed;
		let mut analysis = Analysis::new(file, tree.nodes.len());
		analysis.diagnostics = diagnostics;
		let role = match &self.resolver {
			Some(resolver) => resolver.role(module),
			None if crate::check::is_test_module(module) => FileRole::Test,
			None => FileRole::Program,
		};
		let module_id = self.types.module(module);
		// the reverse edges: the previous version's reads are replaced
		if let Some(&previous) = self.modules.get(module) {
			for dependency in &self.files[previous.index()].scan.dependencies {
				if let Some(readers) = self.readers.get_mut(dependency) {
					readers.retain(|reader| reader != module);
				}
			}
		}
		for dependency in &scan.dependencies {
			let readers = self.readers.entry(dependency.clone()).or_default();
			if !readers.iter().any(|reader| reader == module) {
				readers.push(module.to_string());
			}
		}
		self.files.push(SourceFile {
			module: module.to_string(),
			module_id,
			tree: Arc::new(tree),
			analysis,
			analyzed: false,
			done: false,
			role,
			interface: Interface::default(),
			deferred: false,
			reads: Vec::new(),
			configuration: None,
			scan,
		});
		self.modules.insert(module.to_string(), file);
		file
	}

	/// Analyzes a file (once: the result is memoized).
	pub fn analyze_file(&mut self, file: FileId) -> &Analysis {
		if !self.files[file.index()].analyzed {
			self.files[file.index()].analyzed = true;
			match self.files[file.index()].role {
				// a configuration file is evaluated, not walked (D37)
				FileRole::Configuration(kind) => {
					let kind = match kind {
						fa_codebase_parser::Configuration::Package => {
							crate::configuration::ConfigurationKind::Package
						}
						fa_codebase_parser::Configuration::Workspace => {
							crate::configuration::ConfigurationKind::Workspace
						}
						fa_codebase_parser::Configuration::Dependencies => {
							crate::configuration::ConfigurationKind::Dependencies
						}
					};
					let source = &mut self.files[file.index()];
					let (value, diagnostics) =
						crate::configuration::evaluate(file, &source.tree, kind);
					if crate::configuration::is_computed(&diagnostics) {
						// computed: checked, then run on the interpreter
						self.evaluate_computed_configuration(file, kind);
					} else {
						source.configuration = value;
						source.analysis.diagnostics.extend(diagnostics);
					}
				}
				// on a worker thread: the walk recurses deeply, and its depth
				// must not depend on the caller's stack (`parallel.rs`)
				_ if rayon::current_thread_index().is_none() => {
					crate::parallel::install(|| crate::check::analyze(self, file));
				}
				_ => crate::check::analyze(self, file),
			}
			self.files[file.index()].done = true;
		}
		&self.files[file.index()].analysis
	}

	/// A configuration file whose export is computed (functions, names of
	/// other files, interpolations…): walked like a program, then its export
	/// evaluated on the compile-time interpreter and checked against its
	/// configuration type (advanced brief §16.3, the end of `CFG007`).
	fn evaluate_computed_configuration(
		&mut self,
		file: FileId,
		kind: crate::configuration::ConfigurationKind,
	) {
		crate::check::analyze(self, file);
		self.files[file.index()].done = true;
		let failed = self.files[file.index()]
			.analysis
			.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.severity == crate::Severity::Error);
		if failed {
			return;
		}
		let location = |session: &Self, node: NodeId| Location {
			file,
			span: session.span(file, node),
		};
		match crate::comptime::host::evaluate_export(self, file) {
			// `dependencies.fa`: the `generate` functions of its sources are
			// run by `fa` when it resolves them (not configuration values)
			Ok(value) => match if kind == crate::configuration::ConfigurationKind::Dependencies {
				crate::configuration::from_value_without_functions(&value)
			} else {
				crate::configuration::from_value(&value)
			} {
				Some(value) => {
					let tree = Arc::clone(&self.files[file.index()].tree);
					let (value, diagnostics) =
						crate::configuration::check_computed(file, &tree, kind, value);
					let source = &mut self.files[file.index()];
					source.configuration = value;
					source.analysis.diagnostics.extend(diagnostics);
				}
				None => {
					let tree = Arc::clone(&self.files[file.index()].tree);
					let node =
						crate::configuration::export_node(&tree).unwrap_or(NodeId::new(tree.root));
					let diagnostic = Diagnostic::new(
						&codes::values::E_TYPE_1,
						location(self, node),
						"a configuration holds strings, numbers, booleans, arrays and objects"
							.to_string(),
					);
					self.files[file.index()]
						.analysis
						.diagnostics
						.push(diagnostic);
				}
			},
			Err(failure) => {
				let (node, error) = *failure;
				let code = match &error {
					fa_interpreter::Error::Impure(_) => &codes::advanced::META002,
					fa_interpreter::Error::Steps(_)
					| fa_interpreter::Error::Depth(_)
					| fa_interpreter::Error::Memory(_) => &codes::advanced::META003,
					fa_interpreter::Error::Overflow(_) => &codes::advanced::META013,
					fa_interpreter::Error::Unsupported(_) => &codes::analyzer::AN001,
					_ => &codes::advanced::META005,
				};
				let diagnostic = Diagnostic::new(
					code,
					location(self, node),
					format!("the configuration can't be computed: {}", error.message()),
				);
				self.files[file.index()]
					.analysis
					.diagnostics
					.push(diagnostic);
			}
		}
	}

	/// Forgets the analysis of a file, so that the next
	/// [`Session::analyze_file`] walks it again (its interface is kept: the
	/// files analyzed meanwhile read it).
	pub fn reset_analysis(&mut self, file: FileId) {
		let source = &mut self.files[file.index()];
		let mut analysis = Analysis::new(file, source.tree.nodes.len());
		analysis.diagnostics = convert_all(file, &source.tree);
		source.analysis = analysis;
		source.analyzed = false;
		source.done = false;
		// what the previous walk published stays readable, as incomplete
		source.interface.complete = false;
		source.deferred = false;
		source.reads.clear();
		self.forget_comptime(file);
	}

	/// Adds a diagnostic to a file's analysis (program-level checks).
	pub(crate) fn push_diagnostic(&mut self, file: FileId, diagnostic: Diagnostic) {
		let diagnostics = &mut self.files[file.index()].analysis.diagnostics;
		let duplicate = diagnostics.iter().any(|other| {
			std::ptr::eq(other.code, diagnostic.code)
				&& other.location == diagnostic.location
				&& other.message == diagnostic.message
		});
		if !duplicate {
			diagnostics.push(diagnostic);
		}
	}

	/// The tree of a file and its analysis, taken out for the walk
	/// (put back with [`Session::put_analysis`]).
	pub(crate) fn take_analysis(&mut self, file: FileId) -> (Arc<TypedSyntaxTree>, Analysis) {
		let source = &mut self.files[file.index()];
		let empty = Analysis::new(file, 0);
		(
			Arc::clone(&source.tree),
			std::mem::replace(&mut source.analysis, empty),
		)
	}

	/// Puts back the analysis taken by [`Session::take_analysis`].
	pub(crate) fn put_analysis(&mut self, file: FileId, analysis: Analysis) {
		self.files[file.index()].analysis = analysis;
	}

	/// The module path of a file.
	pub fn module(&self, file: FileId) -> &str {
		&self.files[file.index()].module
	}

	/// The analysis of a file.
	pub fn analysis(&self, file: FileId) -> &Analysis {
		&self.files[file.index()].analysis
	}

	/// The diagnostics of a file.
	pub fn diagnostics(&self, file: FileId) -> &[Diagnostic] {
		&self.analysis(file).diagnostics
	}

	/// The diagnostics of a file rendered for the command line, `path` naming
	/// the file (see [`crate::diagnostics::render`]); empty when there are none.
	pub fn render_diagnostics(&self, file: FileId, path: &str) -> String {
		let source = crate::diagnostics::SourceView::new(path, self.text(file));
		crate::diagnostics::render_all(self.diagnostics(file), &source)
	}

	/// The parse tree of a file.
	pub fn tree(&self, file: FileId) -> &TypedSyntaxTree {
		&self.files[file.index()].tree
	}

	/// The text of a file.
	pub fn text(&self, file: FileId) -> &str {
		&self.tree(file).source
	}

	/// The span of a node.
	pub fn span(&self, file: FileId, node: NodeId) -> SourceSpan {
		// the synthetic owners of the imports and proxy scopes have no span
		self.tree(file)
			.spans
			.get(node.index())
			.map_or(SourceSpan::default(), |&span| span_of(span))
	}

	/// The type store.
	pub fn types(&self) -> &TypeStore {
		&self.types
	}

	/// Prints a type.
	pub fn display_type(&self, ty: crate::ids::TypeId) -> String {
		self.types.display(ty)
	}
}

/// Analyzes a standalone file: tests, `fa run x.fa`.
pub fn analyze_source(text: &str) -> (Session, FileId) {
	let mut session = Session::new();
	let file = session.set_source("main", Arc::from(text));
	session.analyze_file(file);
	(session, file)
}

/// The interface of a file with global ids (what a merge copies).
fn merge_interface(
	merger: &mut crate::types::merge::Merger<'_>,
	interface: &Interface,
) -> Interface {
	use crate::modules::{Declared, Exported, Parameter, TypeFunction};
	let exported = |merger: &mut crate::types::merge::Merger<'_>, exported: &Exported| Exported {
		ty: merger.ty(exported.ty),
		key: merger.decl_key(exported.key),
		generics: merger.list(exported.generics),
		..*exported
	};
	Interface {
		export: interface
			.export
			.as_ref()
			.map(|export| exported(merger, export)),
		members: interface
			.members
			.iter()
			.map(|(&(owner, name), member)| ((owner, merger.sym(name)), exported(merger, member)))
			.collect(),
		type_functions: interface
			.type_functions
			.iter()
			.map(|(&decl, functions)| {
				let functions = functions
					.iter()
					.map(|function| TypeFunction {
						name: merger.sym(function.name),
						ty: merger.ty(function.ty),
						key: merger.decl_key(function.key),
						..*function
					})
					.collect();
				(decl, functions)
			})
			.collect(),
		nested_types: interface
			.nested_types
			.iter()
			.map(|(&decl, nested)| {
				let nested = nested
					.iter()
					.map(|nested| TypeFunction {
						name: merger.sym(nested.name),
						ty: merger.ty(nested.ty),
						key: merger.decl_key(nested.key),
						..*nested
					})
					.collect();
				(decl, nested)
			})
			.collect(),
		parameters: interface
			.parameters
			.iter()
			.map(|(&decl, params)| {
				let params = params
					.iter()
					.map(|param| Parameter {
						name: merger.sym(param.name),
						ty: merger.ty(param.ty),
						key: merger.decl_key(param.key),
						..*param
					})
					.collect();
				(decl, params)
			})
			.collect(),
		externs: interface.externs.clone(),
		statics: interface.statics.clone(),
		field_defaults: interface
			.field_defaults
			.iter()
			.map(|(&decl, defaults)| {
				let defaults = defaults
					.iter()
					.map(|(name, value)| (merger.sym(*name), value.clone()))
					.collect();
				(decl, defaults)
			})
			.collect(),
		export_names: interface.export_names.clone(),
		declared: interface
			.declared
			.iter()
			.map(|(&decl, declared)| {
				let merged = Declared {
					kind: declared.kind,
					name: merger.sym(declared.name),
					span: declared.span,
					function: declared.function,
					uses: declared
						.uses
						.iter()
						.map(|&used| merger.decl_key(used))
						.collect(),
					thread_safe: declared.thread_safe,
				};
				(decl, merged)
			})
			.collect(),
		uses: interface
			.uses
			.iter()
			.map(|&used| merger.decl_key(used))
			.collect(),
		keys: interface
			.keys
			.iter()
			.map(|(&key, &keyed)| (merger.decl_key(key), keyed))
			.collect(),
		reads_target: interface.reads_target,
		worker_work: interface
			.worker_work
			.iter()
			.map(|&(key, span)| (merger.decl_key(key), span))
			.collect(),
		complete: interface.complete,
		// node keys and plain data: nothing to translate (check/tasks.rs)
		async_summaries: interface.async_summaries.clone(),
		relations: interface
			.relations
			.iter()
			.map(|relation| {
				use crate::analysis::{CollectionRef, Relation};
				let mut collection = |reference: CollectionRef| match reference {
					CollectionRef::External { key } => CollectionRef::External {
						key: merger.decl_key(key),
					},
					CollectionRef::ExternalField { owner, name } => CollectionRef::ExternalField {
						owner: merger.ty(owner),
						name: merger.sym(name),
					},
					local => local,
				};
				let holder = collection(relation.holder);
				let target = collection(relation.target);
				Relation {
					kind: relation.kind,
					holder_type: merger.ty(relation.holder_type),
					direct: merger.sym(relation.direct),
					target_type: merger.ty(relation.target_type),
					reverse: relation.reverse.map(|reverse| merger.sym(reverse)),
					holder,
					target,
				}
			})
			.collect(),
	}
}

/// A module parsed, on any thread: its tree, its parse diagnostics, its scan.
pub(crate) struct Parsed {
	pub(crate) tree: TypedSyntaxTree,
	pub(crate) diagnostics: Vec<Diagnostic>,
	pub(crate) scan: FileScan,
}

impl Parsed {
	/// The bytes its tree holds (nodes, spans, source).
	pub(crate) fn tree_bytes(&self) -> usize {
		self.tree.nodes.capacity() * size_of::<fa_parser::Node>()
			+ self.tree.spans.capacity() * size_of::<fa_parser::SourceSpan>()
			+ self.tree.source.len()
	}
}

/// Parses a module's text as file `file`, and scans it.
pub(crate) fn parse_module(
	file: FileId,
	module: &str,
	text: Arc<str>,
	resolver: Option<&dyn ModuleResolver>,
) -> Parsed {
	let tree = fa_parser::parse_shared(text);
	let diagnostics = convert_all(file, &tree);
	let scan = crate::scan::scan(&tree, module, resolver);
	Parsed {
		tree,
		diagnostics,
		scan,
	}
}

fn span_of(span: fa_parser::SourceSpan) -> SourceSpan {
	SourceSpan::new(span.start as u32, span.end as u32)
}

/// The code of a parse diagnostic: the code the parser gives it (a rule of
/// the specification it breaks, `E-STR-1`, `E-DST-9`), or `AN002` / `AN003`
/// for a plain syntax error. A code the registry doesn't know is a bug of
/// the parser: it falls back to `AN002` (a unit test checks every code the
/// parser uses).
fn parse_code(diagnostic: &fa_parser::Diagnostic) -> Code {
	if let Some(code) = diagnostic.code.and_then(codes::lookup) {
		return code;
	}
	match diagnostic.severity {
		fa_parser::Severity::Error => &codes::analyzer::AN002,
		fa_parser::Severity::Warning => &codes::analyzer::AN003,
	}
}

/// The parse diagnostics of a tree as analyzer diagnostics. After a lexical
/// error, the syntax errors of the same line are its consequences: they are
/// dropped, like a second lexical error with the same code on that line.
fn convert_all(file: FileId, tree: &TypedSyntaxTree) -> Vec<Diagnostic> {
	if tree.diagnostics.is_empty() {
		return Vec::new();
	}
	let lines = LineIndex::new(&tree.source);
	let mut lexical_lines: Vec<(u32, Code)> = Vec::new();
	let mut converted = Vec::with_capacity(tree.diagnostics.len());
	for diagnostic in &tree.diagnostics {
		let diagnostic = convert(file, diagnostic);
		let line = lines.line(diagnostic.location.span.start);
		let generic = diagnostic.code.id.starts_with("AN");
		let cascade = lexical_lines
			.iter()
			.any(|&(other, code)| other == line && (generic || code == diagnostic.code));
		if cascade {
			continue;
		}
		if !generic {
			lexical_lines.push((line, diagnostic.code));
		}
		converted.push(diagnostic);
	}
	converted
}

/// A parse diagnostic as an analyzer diagnostic (see [`parse_code`]).
fn convert(file: FileId, diagnostic: &fa_parser::Diagnostic) -> Diagnostic {
	let code = parse_code(diagnostic);
	let location = |span| Location {
		file,
		span: span_of(span),
	};
	let mut converted =
		Diagnostic::new(code, location(diagnostic.span), diagnostic.message.clone());
	for (span, note) in &diagnostic.related {
		converted = converted.with_related(location(*span), note.clone());
	}
	converted
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Severity;

	#[test]
	fn a_valid_file_has_no_diagnostic() {
		let (session, file) = analyze_source("let answer = 42\nconsole.log(answer)\n");
		assert!(session.diagnostics(file).is_empty());
		assert!(session.tree(file).nodes.len() > 3);
		assert_eq!(
			session.analysis(file).types.len(),
			session.tree(file).nodes.len()
		);
	}

	#[test]
	fn parse_errors_become_an002() {
		let (session, file) = analyze_source("let = = (\n");
		let diagnostics = session.diagnostics(file);
		assert!(!diagnostics.is_empty());
		assert!(diagnostics.iter().all(|d| d.code.id.starts_with("AN00")));
		assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
	}

	/// The codes of a source's diagnostics.
	fn codes_of(source: &str) -> Vec<&'static str> {
		let (session, file) = analyze_source(source);
		session
			.diagnostics(file)
			.iter()
			.map(|d| d.code.id)
			.collect()
	}

	#[test]
	fn lexical_errors_get_their_codes() {
		for (source, code) in [
			("let s = \"\\q\"\n", "E-STR-1"),
			("let s = \"\\x80\"\n", "E-STR-2"),
			("let s = \"\\x4\"\n", "E-STR-2"),
			("let s = \"\\u{D800}\"\n", "E-STR-3"),
			("let s = \"\\u{110000}\"\n", "E-STR-3"),
			("let s = \"\\u0041\"\n", "E-STR-3"),
			("let s = \"a } b\"\n", "E-STR-4"),
			("let s = \"a {} b\"\n", "E-STR-5"),
			("let s = 'a'\n", "E-STR-13"),
			("let a = 1; let b = 2\n", "E-LEX-1"),
			("let été = 1\n", "E-LEX-2"),
			("if !true { }\n", "E-LEX-4"),
			("if true {\n}\nelse {\n}\n", "E-LEX-5"),
			("let s = \"abc\n", "E-LEX-6"),
			("--- never closed\n", "E-LEX-6"),
			("let n = 1__0\n", "E-NUM-2"),
			("let loop = 1\n", "E-LEX-3"),
		] {
			let found = codes_of(source);
			assert!(
				found.contains(&code),
				"{source:?}: expected {code}, found {found:?}"
			);
		}
		assert!(codes_of("let = 1\n").contains(&"AN002"));
		for (source, code) in [
			("let x = .5\n", "E-NUM-1"),
			("let x = 0XFF\n", "E-NUM-3"),
			("let x = 12.5n\n", "E-NUM-4"),
			("let pair = (1, 2)\nlet x = pair.0\n", "E-TUP-4"),
			("let [a = 0] = [1]\n", "E-DST-6"),
			("let (a, ...r) = (1, 2)\n", "E-DST-6"),
			("let { a: first } = { a = 1 }\n", "E-DST-8"),
			("let [...r, last] = [1, 2]\n", "E-DST-9"),
			("mutable a = 1\nmutable b = 2\na = b = 3\n", "E-DECL-8"),
		] {
			let found = codes_of(source);
			assert!(
				found.contains(&code),
				"{source:?}: expected {code}, found {found:?}"
			);
		}
		// the consequences of a lexical error on its line are dropped
		assert_eq!(codes_of("let été = 1\n"), ["E-LEX-2"]);
		assert_eq!(codes_of("let s = 'a'\n"), ["E-STR-13"]);
		assert_eq!(codes_of("if !true { }\n"), ["E-LEX-4"]);
	}

	/// Every code written in the parser's sources (`"E-STR-1"`) is in the registry.
	#[test]
	fn every_code_of_the_parser_is_registered() {
		fn visit(dir: &std::path::Path, found: &mut Vec<String>) {
			for entry in std::fs::read_dir(dir).expect("the parser's sources") {
				let path = entry.expect("an entry").path();
				if path.is_dir() {
					if path.file_name().is_some_and(|name| name != "tests") {
						visit(&path, found);
					}
					continue;
				}
				let text = std::fs::read_to_string(&path).expect("a source file");
				for part in text.split('"').skip(1).step_by(2) {
					let is_code = (part.starts_with("E-") || part.starts_with("W-"))
						&& part.len() <= 12
						&& part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
						&& part.ends_with(|c: char| c.is_ascii_digit());
					if is_code {
						found.push(part.to_string());
					}
				}
			}
		}
		let mut found = Vec::new();
		let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../parser/src");
		visit(&dir, &mut found);
		assert!(
			found.len() > 20,
			"the parser's codes are not found: {found:?}"
		);
		for code in found {
			assert!(codes::lookup(&code).is_some(), "{code} is not registered");
		}
	}

	#[test]
	fn diagnostics_render_for_the_command_line() {
		let (session, file) = analyze_source("let ok = 1\nlet s = \"\\q\"\n");
		assert_eq!(
			session.render_diagnostics(file, "bad.fa"),
			concat!(
				"bad.fa:2:10: error[E-STR-1]: Unknown escape sequence `\\q`: the escapes are ",
				"`\\\\`, `\\\"`, `\\n`, `\\r`, `\\t`, `\\0`, `\\xNN` and `\\u{...}`\n",
				"  2 | let s = \"\\q\"\n",
				"    |          ^^\n",
				"\n",
				"1 error\n"
			)
		);
		let (session, file) = analyze_source("let ok = 1\n");
		assert_eq!(session.render_diagnostics(file, "ok.fa"), "");
	}

	#[test]
	fn crlf_line_breaks_in_strings_are_newlines() {
		let (session, file) = analyze_source("let n = \"First line\r\nSecond line\".length\r\n");
		assert!(session.diagnostics(file).is_empty());
		let tree = session.tree(file);
		let length = (0..tree.nodes.len())
			.find(|&node| tree.text(node).ends_with(".length"))
			.unwrap();
		let ty = session.analysis(file).type_of(NodeId::new(length));
		assert_eq!(session.display_type(ty), "22");
	}

	#[test]
	fn every_version_gets_a_new_file() {
		let mut session = Session::new();
		let a = session.set_source("main", Arc::from("let a = 1"));
		let b = session.set_source("main", Arc::from("let a = 2"));
		assert_ne!(a, b);
		assert_eq!(session.text(b), "let a = 2");
	}
}
