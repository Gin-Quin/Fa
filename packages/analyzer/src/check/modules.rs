//! The module system in the walk (advanced brief §15): the names of the
//! other files, exports, namespaces, `use` aliases of module paths (D14).
//!
//! - **Imports.** Before the walk, every name written in the file that the
//!   module system resolves (a sibling, a root entry, `library`) gets a
//!   *proxy* symbol in the file's imports scope, the parent of its module
//!   scope: local declarations shadow them, and they shadow the prelude,
//!   which is the lookup order of §15.3. A proxy has the kind of what it
//!   stands for (an `export type` is a `Type`, a folder a `Namespace`…), so
//!   that every lookup of the walk treats it like a local declaration; its
//!   type is read from the other file's [`Interface`] at the first use
//!   (analyzing that file on demand). At the end of the walk, the
//!   resolutions to proxies become [`Resolution::External`] (or `Folder`).
//! - **Exports.** The export of a file is a symbol of its module scope named
//!   after the file ("the file's own name designates its export"), whose
//!   declaration node is the `export` statement: the handlers of `let`,
//!   `function`, `type` and `namespace` analyze it.
//! - **Namespaces.** A namespace's members are declared in the scope of its
//!   block, hoisted like the module-level declarations of a package file.
//!   `a.b.c` follows namespaces (local ones, other files' exports, folders)
//!   member by member; `#private` members are only visible inside
//!   (`MOD009`); a `mutable` member is written only from its file (`EFF013`).
//! - **Cycles between files.** A file whose analysis is in progress is read
//!   through the interface it published so far (or in the previous round of
//!   [`crate::Session::analyze_program`]); a declaration not published yet is
//!   `UNKNOWN`, silently, and the file is marked *deferred*: the program
//!   analysis walks it again once the other file is done.

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::check::{PlaceInfo, Walker};
use crate::diagnostics::SourceSpan;
use crate::diagnostics::codes::advanced::{EFF013, MOD005, MOD006, MOD008, MOD009, MOD014};
use crate::diagnostics::codes::types::TY008;
use crate::diagnostics::codes::values::E_DECL_1;
use crate::ids::{DeclKey, ModuleId, NodeId, ScopeId, Sym, SymbolId, TypeId};
use crate::modules::{Exported, FileRole, ModuleEntry};
use crate::symbols::{DeclState, Resolution, ScopeKind, Symbol, SymbolKind};
use crate::syntax::{self, Name, NodeList, View};

/// What a proxy symbol stands for.
#[derive(Clone, Debug)]
pub(crate) enum Proxy {
	/// The export of another file.
	Export { module: ModuleId },
	/// A member of a namespace of another file.
	Member {
		module: ModuleId,
		owner: NodeId,
		name: Sym,
	},
	/// A folder namespace.
	Folder { path: String },
	/// Another declaration of another file (a function of its type, a
	/// parameter of its function): its node there, its key.
	Decl {
		module: ModuleId,
		decl: NodeId,
		key: DeclKey,
	},
}

/// The module state of a walk.
#[derive(Default, Debug)]
pub(crate) struct ModuleState {
	/// The scope of the proxies, parent of the module scope.
	pub imports: Option<ScopeId>,
	/// The proxies.
	pub proxies: FxHashMap<SymbolId, Proxy>,
	/// The member scope of each namespace symbol (local, proxy or alias).
	pub member_scopes: FxHashMap<SymbolId, ScopeId>,
	/// The export statement and its symbol.
	pub export: Option<(NodeId, SymbolId)>,
	/// The module-level values declared `Pending` (package files, namespaces).
	pub hoisted_values: FxHashSet<NodeId>,
	/// A read of a declaration that is not available yet.
	pub deferred: bool,
	/// The `use` aliases of module paths: alias → the symbol it names.
	pub aliases: FxHashMap<SymbolId, SymbolId>,
	/// The modules looked up, found or not.
	pub reads: Vec<String>,
	/// The functions of other files' types met as members: whether each
	/// takes `self` (a method).
	pub methods: FxHashMap<SymbolId, bool>,
	/// The scope of their proxies.
	pub method_scope: Option<ScopeId>,
	/// The proxy of each function of another file's type, by declaration.
	pub decl_proxies: FxHashMap<DeclKey, SymbolId>,
	/// The modules the file may read, directly or not (computed once).
	pub closure: Option<FxHashSet<ModuleId>>,
	/// The block of each namespace declaration of the file → the declaration.
	pub namespace_blocks: FxHashMap<NodeId, NodeId>,
	/// The symbol of each declaration key given (`Walker::symbol_key`).
	pub keyed: FxHashMap<DeclKey, SymbolId>,
	/// The next owner node of a proxy member scope (counting down from the top).
	next_owner: u32,
}

/// The owner node of the imports scope (no real node has it).
const IMPORTS_OWNER: u32 = u32::MAX;

/// The export statement of a tree and the kind of what it exports.
pub(crate) fn export_of(tree: &fa_parser::TypedSyntaxTree) -> Option<(NodeId, SymbolKind)> {
	let root = NodeId::new(tree.root);
	let View::Module { statements } = syntax::view(tree, root) else {
		return None;
	};
	statements
		.iter()
		.find_map(|statement| export_kind(tree, statement).map(|kind| (statement, kind)))
}

/// The kind of what an export statement exports.
fn export_kind(tree: &fa_parser::TypedSyntaxTree, statement: NodeId) -> Option<SymbolKind> {
	Some(match syntax::view(tree, statement) {
		View::ExportValue { mutable: true, .. } => SymbolKind::Mutable,
		View::ExportValue { .. } => SymbolKind::Let,
		View::ExportFunction { .. } => SymbolKind::Function,
		View::ExportType { .. } => SymbolKind::Type,
		View::ExportNamespace(_) => SymbolKind::Namespace,
		_ => return None,
	})
}

/// The block holding the members of a namespace declaration.
pub(crate) fn namespace_block(tree: &fa_parser::TypedSyntaxTree, decl: NodeId) -> Option<NodeId> {
	match syntax::view(tree, decl) {
		View::Namespace { value, .. } | View::ExportNamespace(value) => matches!(
			syntax::view(tree, value),
			View::Block { .. } | View::Object { .. }
		)
		.then_some(value),
		_ => None,
	}
}

impl Walker<'_> {
	// ── the start and the end of a file ─────────────────────────────────

	/// Before the walk: the imports scope and its proxies, the export symbol,
	/// and the rules of what a file of its role may contain.
	pub(super) fn start_modules(&mut self, root: NodeId) {
		let module_scope = self.a.symbols.scope_for(root, None, ScopeKind::Module);
		let imports = self
			.a
			.symbols
			.scope_for(NodeId(IMPORTS_OWNER), None, ScopeKind::Imports);
		self.a.symbols.set_parent(module_scope, Some(imports));
		self.modules.imports = Some(imports);
		self.modules.next_owner = IMPORTS_OWNER - 1;
		// the namespace blocks were found when the file was parsed (scan.rs)
		self.modules.namespace_blocks = self.s.source_file(self.file).scan.namespace_blocks.clone();
		self.declare_export(root, module_scope);
		self.import_names(imports);
		self.check_file_contents(root);
	}

	/// After the walk: the resolutions to proxies become external, the
	/// interface is published, the session learns what the file read.
	pub(super) fn finish_modules(&mut self, root: NodeId) {
		// Not done when a name resolves: a member proxy is made, and an
		// alias declared, after names resolved to them (hoisting, lazy
		// proxies). One loop over the dense resolution table instead, which
		// only looks further at the symbols that are proxies or aliases.
		if !self.modules.proxies.is_empty() || !self.modules.aliases.is_empty() {
			let mut external = vec![false; self.a.symbols.symbol_count()];
			for &symbol in self
				.modules
				.proxies
				.keys()
				.chain(self.modules.aliases.keys())
			{
				if let Some(slot) = external.get_mut(symbol.index()) {
					*slot = true;
				}
			}
			for index in 0..self.a.resolutions.len() {
				let Resolution::Local(symbol) = self.a.resolutions[index] else {
					continue;
				};
				if external.get(symbol.index()).copied().unwrap_or(true)
					&& let Some(resolution) = self.external_resolution(symbol)
				{
					self.a.resolutions[index] = resolution;
				}
			}
		}
		let scopes: Vec<(SymbolId, ScopeId)> = self
			.modules
			.member_scopes
			.iter()
			.map(|(&symbol, &scope)| (symbol, scope))
			.collect();
		for (namespace, scope) in scopes {
			if self.modules.proxies.contains_key(&namespace)
				|| self.modules.aliases.contains_key(&namespace)
			{
				continue;
			}
			let members = self.a.symbols.scope(scope).symbols.clone();
			for member in members {
				self.publish(member);
			}
		}
		if let Some((_, export)) = self.modules.export {
			self.publish(export);
		}
		// Retention is settled before its portable parameter summaries.
		self.finish_parameter_retention();
		// the rest of what the program and the backends read of the file
		self.publish_interface(root);
		let deferred = self.modules.deferred;
		let reads = std::mem::take(&mut self.modules.reads);
		let source = self.s.source_file_mut(self.file);
		source.deferred = deferred;
		source.reads = reads;
	}

	/// The resolution a symbol stands for, when it is a proxy or an alias.
	pub(super) fn external_resolution(&mut self, symbol: SymbolId) -> Option<Resolution> {
		let mut symbol = symbol;
		let mut seen = 0;
		while let Some(&target) = self.modules.aliases.get(&symbol) {
			symbol = target;
			seen += 1;
			if seen > 32 {
				return None;
			}
		}
		let Some(proxy) = self.modules.proxies.get(&symbol).cloned() else {
			return (seen > 0).then_some(Resolution::Local(symbol));
		};
		match proxy {
			Proxy::Folder { path } => Some(Resolution::Folder(self.s.types.module(&path))),
			Proxy::Export { module } => {
				let export = self.s.interface(module)?.export?;
				Some(Resolution::External(export.key))
			}
			Proxy::Member {
				module,
				owner,
				name,
			} => {
				let member = *self.s.interface(module)?.members.get(&(owner, name))?;
				Some(Resolution::External(member.key))
			}
			Proxy::Decl { key, .. } => Some(Resolution::External(key)),
		}
	}

	/// The function `name` of a type declared in another file: a proxy of
	/// it (its methods take `self`). Every file that declared a type of
	/// this structure is looked at, in the order of their paths (two files
	/// may declare the same object type, each with its functions).
	/// The type named `name` declared inside a type of another file
	/// (`Database.OpenError`), from that file's interface.
	pub(super) fn external_nested_type(&mut self, ty: TypeId, name: Sym) -> Option<TypeId> {
		let own = self.s.module_id(self.file);
		if self.modules.closure.is_none() {
			self.modules.closure = Some(self.s.dependency_closure(self.file));
		}
		let closure = self.modules.closure.as_ref()?;
		let origins = self
			.s
			.type_origins(ty, |module| module != own && closure.contains(&module));
		for origin in origins {
			let module = self.s.types.decl_key_data(origin).module;
			let Some(interface) = self.s.interface(module) else {
				continue;
			};
			let nested = interface
				.node_of(origin)
				.and_then(|decl| interface.nested_types.get(&decl))
				.and_then(|nested| nested.iter().find(|nested| nested.name == name))
				.map(|nested| nested.ty);
			match nested {
				Some(nested) => return Some(nested),
				// a file whose walk is in progress (a cycle)
				None if !interface.complete => self.modules.deferred = true,
				None => {}
			}
		}
		None
	}

	pub(super) fn external_type_function(&mut self, ty: TypeId, name: &str) -> Option<SymbolId> {
		let ty = match self.s.types.kind(ty) {
			crate::types::TypeKind::Member(nominal, _)
			| crate::types::TypeKind::EnumMember(nominal, _) => self.s.types.nominal_type(nominal),
			_ => ty,
		};
		let own = self.s.module_id(self.file);
		let sym = self.s.types.syms.get(name)?;
		let mut found = None;
		// only the files this one may read (directly or not) gave it values
		if self.modules.closure.is_none() {
			self.modules.closure = Some(self.s.dependency_closure(self.file));
		}
		let closure = self.modules.closure.as_ref()?;
		let origins = self
			.s
			.type_origins(ty, |module| module != own && closure.contains(&module));
		for origin in origins {
			let module = self.s.types.decl_key_data(origin).module;
			let Some(interface) = self.s.interface(module) else {
				continue;
			};
			let function = interface
				.node_of(origin)
				.and_then(|decl| interface.type_functions.get(&decl))
				.and_then(|functions| functions.iter().find(|function| function.name == sym))
				.copied();
			match function {
				Some(function) => {
					found = Some((module, function));
					break;
				}
				// a file whose walk is in progress (a cycle): the program
				// analysis comes back once it is done
				None if !interface.complete => self.modules.deferred = true,
				None => {}
			}
		}
		let (module, function) = found?;
		self.type_function_proxy(module, function, name)
	}

	/// A method of one precise written declaration, without searching other
	/// declarations that happen to have the same structural data type.
	pub(super) fn external_type_function_at(
		&mut self,
		origin: DeclKey,
		name: &str,
	) -> Option<SymbolId> {
		let module = self.s.types.decl_key_data(origin).module;
		self.modules
			.reads
			.push(self.s.types.module_path(module).to_string());
		let sym = self.s.types.syms.get(name)?;
		let interface = self.s.interface(module)?;
		let function = interface
			.node_of(origin)
			.and_then(|decl| interface.type_functions.get(&decl))
			.and_then(|functions| functions.iter().find(|function| function.name == sym))
			.copied();
		if function.is_none() && !interface.complete {
			self.modules.deferred = true;
		}
		self.type_function_proxy(module, function?, name)
	}

	fn type_function_proxy(
		&mut self,
		module: ModuleId,
		function: crate::modules::TypeFunction,
		name: &str,
	) -> Option<SymbolId> {
		if let Some(&existing) = self.modules.decl_proxies.get(&function.key) {
			return Some(existing);
		}
		// a scope of their own, which no lookup goes through
		let scope = match self.modules.method_scope {
			Some(scope) => scope,
			None => {
				let owner = NodeId(self.modules.next_owner);
				self.modules.next_owner -= 1;
				let scope = self.a.symbols.scope_for(owner, None, ScopeKind::Namespace);
				self.modules.method_scope = Some(scope);
				scope
			}
		};
		let proxy = Proxy::Decl {
			module,
			decl: function.decl,
			key: function.key,
		};
		let symbol = self.declare_proxy(scope, name, SymbolKind::Function, proxy, function.ty);
		self.modules.decl_proxies.insert(function.key, symbol);
		self.modules.methods.insert(symbol, function.method);
		Some(symbol)
	}

	/// The default values of the fields of an object type declared in
	/// another file (`Interface::field_defaults`): those of the first file,
	/// in the order of their paths, that declared this structure.
	pub(super) fn external_field_defaults(
		&mut self,
		ty: TypeId,
	) -> Option<Vec<(crate::ids::Sym, Option<fa_interpreter::Value>)>> {
		let own = self.s.module_id(self.file);
		if self.modules.closure.is_none() {
			self.modules.closure = Some(self.s.dependency_closure(self.file));
		}
		let closure = self.modules.closure.as_ref()?;
		let origins = self
			.s
			.type_origins(ty, |module| module != own && closure.contains(&module));
		for origin in origins {
			let module = self.s.types.decl_key_data(origin).module;
			let Some(interface) = self.s.interface(module) else {
				continue;
			};
			if !interface.complete {
				// a cycle: the program analysis comes back once it is done
				self.modules.deferred = true;
				continue;
			}
			if let Some(defaults) = interface
				.node_of(origin)
				.and_then(|decl| interface.field_defaults.get(&decl))
			{
				return Some(defaults.clone());
			}
		}
		None
	}

	/// The declaration of another file a proxy stands for, once that file
	/// is analyzed.
	pub(super) fn proxy_decl(&self, symbol: SymbolId) -> Option<(ModuleId, NodeId)> {
		let mut symbol = symbol;
		while let Some(&target) = self.modules.aliases.get(&symbol) {
			symbol = target;
		}
		let (module, decl) = match self.modules.proxies.get(&symbol)? {
			Proxy::Export { module } => (*module, self.s.interface(*module)?.export?.decl),
			Proxy::Member {
				module,
				owner,
				name,
			} => (
				*module,
				self.s
					.interface(*module)?
					.members
					.get(&(*owner, *name))?
					.decl,
			),
			Proxy::Decl { module, decl, .. } => (*module, *decl),
			Proxy::Folder { .. } => return None,
		};
		self.s.interface(module)?.complete.then_some((module, decl))
	}

	/// A proxy of a module-level value of another file this file doesn't
	/// name (the effect of a call on it, `mutates people` of a function it
	/// calls): the place of that effect here.
	pub(crate) fn module_value_proxy(&mut self, key: DeclKey) -> Option<SymbolId> {
		let module = self.s.types.decl_key_data(key).module;
		if module == self.s.module_id(self.file) {
			return None;
		}
		if let Some(&existing) = self.modules.decl_proxies.get(&key) {
			return Some(existing);
		}
		// a name of this file standing for it (`shared`, `library.shared`):
		// the effect is on the place the file reads
		let mut named: Vec<SymbolId> = self
			.modules
			.proxies
			.iter()
			.filter(|(_, proxy)| match proxy {
				Proxy::Export { module: other } | Proxy::Member { module: other, .. } => {
					*other == module
				}
				_ => false,
			})
			.map(|(&symbol, _)| symbol)
			.collect();
		named.sort();
		for symbol in named {
			if self.external_value_key(symbol) == Some(key) {
				self.module_values.insert(key, symbol);
				return Some(symbol);
			}
		}
		let interface = self.s.interface(module)?;
		let exported = interface
			.export
			.iter()
			.chain(interface.members.values())
			.find(|exported| exported.key == key)
			.copied()?;
		if !matches!(
			exported.kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
		) {
			return None;
		}
		let names = self.s.types.decl_key_data(key).names;
		let name = *self.s.types.sym_list(names).last()?;
		let text = self.s.types.syms.resolve(name).to_string();
		let scope = match self.modules.method_scope {
			Some(scope) => scope,
			None => {
				let owner = NodeId(self.modules.next_owner);
				self.modules.next_owner -= 1;
				let scope = self.a.symbols.scope_for(owner, None, ScopeKind::Namespace);
				self.modules.method_scope = Some(scope);
				scope
			}
		};
		let proxy = Proxy::Decl {
			module,
			decl: exported.decl,
			key,
		};
		let symbol = self.declare_proxy(scope, &text, exported.kind, proxy, exported.ty);
		self.modules.decl_proxies.insert(key, symbol);
		self.module_values.insert(key, symbol);
		Some(symbol)
	}

	/// The key of the declaration of another file a proxy (or an alias of
	/// one) stands for. The export of a file is keyed by its module and the
	/// file's name, without analyzing it: a relation names a collection of
	/// another file by key, and that file reads this one for the type of its
	/// elements (relations.rs).
	pub(super) fn external_value_key(&mut self, symbol: SymbolId) -> Option<DeclKey> {
		let mut symbol = symbol;
		let mut seen = 0;
		while let Some(&target) = self.modules.aliases.get(&symbol) {
			symbol = target;
			seen += 1;
			if seen > 32 {
				return None;
			}
		}
		if let Some(Proxy::Export { module }) = self.modules.proxies.get(&symbol).cloned() {
			if let Some(export) = self
				.s
				.interface(module)
				.and_then(|interface| interface.export)
			{
				return Some(export.key);
			}
			let file = self.s.file_of(module)?;
			let name = self.s.own_name(file).unwrap_or_else(|| {
				let path = self.s.module(file);
				path.rsplit('/').next().unwrap_or(path).to_string()
			});
			let name = self.s.types.syms.intern(&name);
			let names = self.s.types.intern_sym_list(&[name]);
			return Some(self.s.types.decl_key_of(module, names, 0));
		}
		match self.external_resolution(symbol)? {
			Resolution::External(key) => Some(key),
			_ => None,
		}
	}

	/// The parameters of a function of another file called directly: per
	/// parameter, its name (a proxy of the parameter) and whether it has a
	/// default, so that named arguments and defaults work across files.
	#[allow(clippy::type_complexity)]
	pub(super) fn external_parameters(
		&mut self,
		symbol: SymbolId,
		count: usize,
		method: bool,
	) -> Option<Vec<(Option<(Sym, SymbolId)>, bool, bool)>> {
		let (module, decl) = self.proxy_decl(symbol)?;
		let params = self.s.interface(module)?.parameters.get(&decl)?.clone();
		// a method called on a value: the receiver fills `self`
		let skip = usize::from(method);
		if params.len() != count + skip {
			return None;
		}
		let scope = self.proxy_scope(symbol);
		let mut parameters = Vec::with_capacity(count);
		for param in params.iter().skip(skip) {
			let proxy = match self.a.symbols.lookup_in(scope, param.name) {
				Some(proxy) => proxy,
				None => {
					let text = self.s.types.syms.resolve(param.name).to_string();
					let proxy = Proxy::Decl {
						module,
						decl: param.decl,
						key: param.key,
					};
					self.declare_proxy(scope, &text, param.kind, proxy, param.ty)
				}
			};
			parameters.push((
				Some((param.name, proxy)),
				param.has_default,
				param.only_called,
			));
		}
		Some(parameters)
	}

	/// The declaration key of a symbol of this file (decision A10): its
	/// module, the names of the declarations enclosing it (namespaces,
	/// types, functions) then its own name, and a disambiguator telling
	/// apart two declarations with the same names (counted in the order
	/// the keys are asked, which the walk fixes). Other files and the
	/// program name a declaration by its key, which a new version of the
	/// file keeps as long as the declaration keeps its names.
	pub(crate) fn symbol_key(&mut self, symbol: SymbolId) -> DeclKey {
		if let Some(key) = self.a.symbols.symbol(symbol).key {
			return key;
		}
		let data = self.a.symbols.symbol(symbol);
		let mut names: smallvec::SmallVec<[Sym; 4]> = smallvec::smallvec![data.name];
		let mut scope = Some(data.scope);
		let mut depth = 0;
		while let Some(current) = scope {
			let info = self.a.symbols.scope(current);
			if matches!(info.kind, ScopeKind::Module | ScopeKind::Imports) || depth > 64 {
				break;
			}
			if let Some(name) = self
				.owner_name(info.owner)
				.or_else(|| self.type_body_name(info.owner, info.parent))
			{
				names.push(name);
			}
			scope = info.parent;
			depth += 1;
		}
		names.reverse();
		let module = self.s.module_id(self.file);
		let names = self.s.types.intern_sym_list(&names);
		let mut disambiguator: u16 = 0;
		let key = loop {
			let key = self.s.types.decl_key_of(module, names, disambiguator);
			match self.modules.keyed.get(&key) {
				Some(&other) if other != symbol => disambiguator = disambiguator.saturating_add(1),
				_ => break key,
			}
			if disambiguator == u16::MAX {
				break key;
			}
		};
		self.modules.keyed.insert(key, symbol);
		self.a.symbols.symbol_mut(symbol).key = Some(key);
		key
	}

	/// The key of the declaration of a node of this file, if it declares a
	/// symbol.
	pub(crate) fn node_key(&mut self, node: NodeId) -> Option<DeclKey> {
		let symbol = self.a.symbols.symbol_of_decl(node)?;
		Some(self.symbol_key(symbol))
	}

	/// The name of the type whose body is `owner`, declared in `parent`: the
	/// declarations inside a type are keyed under its name (`Farm.Error`,
	/// which a C header names `farm_FarmError`).
	pub(super) fn type_body_name(&self, owner: NodeId, parent: Option<ScopeId>) -> Option<Sym> {
		let parent = parent?;
		self.a
			.symbols
			.scope(parent)
			.symbols
			.iter()
			.map(|&symbol| self.a.symbols.symbol(symbol))
			.filter(|symbol| symbol.kind == SymbolKind::Type)
			.find(|symbol| match syntax::view(&self.tree, symbol.decl_node) {
				View::TypeDecl { value, .. } | View::ExportType { value, .. } => value == owner,
				_ => false,
			})
			.map(|symbol| symbol.name)
	}

	/// The name of the declaration owning a scope (a namespace's block, a
	/// type, a function), if it has one.
	fn owner_name(&self, owner: NodeId) -> Option<Sym> {
		let decl = match self.modules.namespace_blocks.get(&owner) {
			Some(&decl) => decl,
			None => match self
				.a
				.functions
				.get(&owner)
				.and_then(|info| info.declaration)
			{
				Some(decl) => decl,
				None => owner,
			},
		};
		let symbol = self.a.symbols.symbol_of_decl(decl)?;
		Some(self.a.symbols.symbol(symbol).name)
	}

	/// Publishes a symbol other files may read: the export, or a member of
	/// a namespace of this file.
	pub(super) fn publish(&mut self, symbol: SymbolId) {
		let key = self.symbol_key(symbol);
		let generics = self.declared_generics(symbol);
		let generics: Vec<TypeId> = generics
			.into_iter()
			.map(|param| self.s.types.intern(crate::types::TypeKind::Param(param)))
			.collect();
		let generics = self.s.types.list(&generics);
		let data = self.a.symbols.symbol(symbol);
		let members = namespace_block(&self.tree, data.decl_node);
		let exported = Exported {
			decl: data.decl_node,
			key,
			kind: data.kind,
			ty: data.ty,
			members,
			generics,
		};
		if self
			.modules
			.export
			.is_some_and(|(_, export)| export == symbol)
		{
			self.s.source_file_mut(self.file).interface.export = Some(exported);
			return;
		}
		let scope = self.a.symbols.scope(data.scope);
		if scope.kind != ScopeKind::Namespace
			|| !self.modules.namespace_blocks.contains_key(&scope.owner)
		{
			return;
		}
		let key = (scope.owner, data.name);
		self.s
			.source_file_mut(self.file)
			.interface
			.members
			.insert(key, exported);
	}

	/// Declares the export of the file: a symbol named after the file, whose
	/// declaration is the first `export` statement (`Pending`, analyzed like
	/// the declaration it exports).
	fn declare_export(&mut self, root: NodeId, module_scope: ScopeId) {
		let Some((node, kind)) = export_of(&self.tree) else {
			return;
		};
		let name = self.s.own_name(self.file).unwrap_or_else(|| {
			let module = self.s.module(self.file);
			module.rsplit('/').next().unwrap_or(module).to_string()
		});
		if self.s.role(self.file) == FileRole::Test {
			let diagnostic = self.diagnostic(
				&MOD005,
				node,
				"a test file exports nothing: move the export to a file of its own",
			);
			self.report(diagnostic);
		}
		let sym = self.s.types.syms.intern(&name);
		let span = self.span(node);
		let name_span = SourceSpan::new(span.start, span.start.saturating_add(6).min(span.end));
		let mut symbol = Symbol::new(sym, kind, node, name_span);
		symbol.state = DeclState::Pending;
		let (symbol, _) = self.a.symbols.declare(module_scope, symbol);
		self.modules.export = Some((node, symbol));
		let _ = root;
	}

	/// The name of the export of this file, when `n` is its export statement.
	pub(super) fn export_name(&self, n: NodeId) -> Option<String> {
		let (node, symbol) = self.modules.export?;
		(node == n).then(|| {
			let name = self.a.symbols.symbol(symbol).name;
			self.s.types.syms.resolve(name).to_string()
		})
	}

	/// The text of a declaration's name: the file's name for its export.
	pub(super) fn decl_text(&self, n: NodeId, name: Name) -> String {
		if name.is_empty()
			&& let Some(text) = self.export_name(n)
		{
			return text;
		}
		self.tree.name(name).to_string()
	}

	/// Whether `n` is a second `export` of the file (`MOD005`, reported once).
	pub(super) fn is_extra_export(&mut self, n: NodeId) -> bool {
		if self.modules.export.is_none_or(|(node, _)| node == n) {
			return false;
		}
		self.memo(n, |w| {
			let diagnostic = w.diagnostic(
				&MOD005,
				n,
				"each file exports one entity: export a namespace to export several values",
			);
			w.report(diagnostic);
			TypeId::VOID
		});
		self.walk_children(n);
		true
	}

	/// Creates the proxies of the names written in the file that the module
	/// system resolves.
	fn import_names(&mut self, imports: ScopeId) {
		let Some(resolver) = self.s.resolver() else {
			return;
		};
		// the names were resolved when the file was parsed (scan.rs)
		let scan = &self.s.source_file(self.file).scan;
		let found: Vec<(Box<str>, ModuleEntry)> = if scan.resolved {
			scan.imports.clone()
		} else {
			let module = self.s.module(self.file).to_string();
			crate::scan::scan(&self.tree, &module, Some(resolver)).imports
		};
		let own = self.s.own_name(self.file);
		let program = self.s.role(self.file) == FileRole::Program;
		for (text, entry) in found {
			// a program's implicit `arguments` is its own (D4)
			let implicit = &*text == "arguments" && program;
			if implicit || own.as_deref() == Some(&*text) {
				continue;
			}
			self.modules.reads.push(text.to_string());
			self.make_proxy(imports, &text, entry);
		}
	}

	/// Declares a proxy for a module entry in a scope.
	fn make_proxy(&mut self, scope: ScopeId, text: &str, entry: ModuleEntry) -> Option<SymbolId> {
		let (kind, proxy) = match entry {
			ModuleEntry::Folder(path) => (SymbolKind::Namespace, Proxy::Folder { path }),
			ModuleEntry::File(module) => {
				// a module of another unit of a parallel run: its scan (not loaded)
				let scanned = self.s.scan_of(&module).map(|scan| scan.export);
				let export = match scanned {
					Some(export) => export,
					None => {
						let file = self.s.file_of_module(&module)?;
						self.s.source_file(file).scan.export
					}
				};
				let kind = export.map_or(SymbolKind::Let, |(_, kind)| kind);
				let module_id = self.s.types.module(&module);
				self.modules.reads.push(module);
				(kind, Proxy::Export { module: module_id })
			}
		};
		Some(self.declare_proxy(scope, text, kind, proxy, TypeId::UNSET))
	}

	/// Declares a proxy symbol.
	fn declare_proxy(
		&mut self,
		scope: ScopeId,
		text: &str,
		kind: SymbolKind,
		proxy: Proxy,
		ty: TypeId,
	) -> SymbolId {
		let sym = self.s.types.syms.intern(text);
		let owner = self.a.symbols.scope(scope).owner;
		let mut symbol = Symbol::new(sym, kind, owner, SourceSpan::default());
		symbol.state = DeclState::Done;
		symbol.ty = if kind == SymbolKind::Namespace {
			TypeId::UNKNOWN
		} else {
			ty
		};
		let (symbol, _) = self.a.symbols.declare(scope, symbol);
		self.modules.proxies.insert(symbol, proxy);
		symbol
	}

	/// The rules of what a file contains, by role: only declarations in a
	/// module (`MOD006`) or a test file (`MOD014`), exactly one export in a
	/// module (`MOD005`).
	fn check_file_contents(&mut self, root: NodeId) {
		let role = self.s.role(self.file);
		if !role.is_module() {
			return;
		}
		let tree = Arc::clone(&self.tree);
		let View::Module { statements } = syntax::view(&tree, root) else {
			return;
		};
		for statement in statements.iter() {
			if self.is_declaration_statement(statement, role == FileRole::Test) {
				continue;
			}
			let diagnostic = if role == FileRole::Test {
				self.diagnostic(
					&MOD014,
					statement,
					"a test file holds declarations and tests: move this statement into a test",
				)
			} else {
				self.diagnostic(
					&MOD006,
					statement,
					"a statement at the top of a package file: move it into a function; top-level statements are only allowed in standalone files and scripts",
				)
			};
			self.report(diagnostic);
		}
		if role == FileRole::Module && self.modules.export.is_none() && self.s.resolver().is_some()
		{
			let name = self.s.own_name(self.file).unwrap_or_default();
			let diagnostic = self.diagnostic_at(
				&MOD005,
				SourceSpan::new(0, 0),
				format!(
					"each file exports one entity: add `export` to the main declaration of `{name}`"
				),
			);
			self.report(diagnostic);
		}
		self.check_initializers(statements);
	}

	/// Whether a top-level statement is a declaration (what a module may hold).
	fn is_declaration_statement(&self, statement: NodeId, test_file: bool) -> bool {
		match self.view(statement) {
			View::Declaration { .. }
			| View::Use { .. }
			| View::Destructuring { .. }
			| View::TypeDecl { .. }
			| View::Namespace { .. }
			| View::Function { .. }
			| View::ExportValue { .. }
			| View::ExportFunction { .. }
			| View::ExportType { .. }
			| View::ExportNamespace(_)
			| View::Error => true,
			View::If {
				compile_time: true, ..
			}
			| View::For {
				compile_time: true, ..
			} => true,
			View::Test { .. } => test_file,
			// `TEST001` says more about a top-level `assert` of a test file
			View::Assert(_) => test_file,
			View::Unsupported(_) => true,
			_ => false,
		}
	}

	/// `MOD008`: a module-level initializer can't propagate an error with
	/// `try`, there is no function to propagate it to.
	fn check_initializers(&mut self, statements: NodeList<'_>) {
		let tree = Arc::clone(&self.tree);
		for statement in statements.iter() {
			let value = match syntax::view(&tree, statement) {
				View::Declaration {
					value: Some(value), ..
				}
				| View::ExportValue { value, .. } => value,
				_ => continue,
			};
			let mut stack = vec![value];
			while let Some(node) = stack.pop() {
				match syntax::view(&tree, node) {
					View::Lambda { .. } => continue,
					View::Try(_) => {
						let diagnostic = self.diagnostic(
							&MOD008,
							node,
							"an initializer can't propagate an error: handle it with `catch`",
						);
						self.report(diagnostic);
					}
					_ => {}
				}
				stack.extend(syntax::children(&tree, node));
			}
		}
	}

	/// Whether the module-level values of a block are hoisted: the top of a
	/// package file or a test file, and every namespace.
	pub(super) fn hoists_values(&self, block: NodeId) -> bool {
		if block.index() == self.tree.root {
			return self.s.role(self.file).is_module();
		}
		self.modules.namespace_blocks.contains_key(&block)
	}

	/// For a module-level value declared `Pending`: analyze it now through
	/// [`Walker::hoisted`] (`Some(f(self))`), or skip it when it was
	/// analyzed on demand already (`Some(VOID)`); `None` for any other
	/// declaration, or one being analyzed.
	pub(super) fn hoisted_value(
		&mut self,
		n: NodeId,
		f: impl FnOnce(&mut Self) -> TypeId,
	) -> Option<TypeId> {
		if !self.modules.hoisted_values.contains(&n) {
			return None;
		}
		let symbol = self.a.symbols.symbol_of_decl(n)?;
		match self.a.symbols.symbol(symbol).state {
			DeclState::Pending => Some(self.hoisted(n, f)),
			DeclState::Done => Some(TypeId::VOID),
			_ => None,
		}
	}

	// ── proxies and namespaces ──────────────────────────────────────────

	/// The declaration of another file a proxy symbol stands for, once
	/// published (`None` for any other symbol).
	pub(super) fn proxy_exported(&mut self, symbol: SymbolId) -> Option<Exported> {
		let symbol = match self.modules.aliases.get(&symbol) {
			Some(&target) => target,
			None => symbol,
		};
		match self.modules.proxies.get(&symbol).cloned()? {
			Proxy::Export { module } => {
				self.ensure_analyzed(module);
				self.s
					.interface(module)
					.and_then(|interface| interface.export)
			}
			Proxy::Member {
				module,
				owner,
				name,
			} => {
				self.ensure_analyzed(module);
				self.s
					.interface(module)
					.and_then(|interface| interface.members.get(&(owner, name)).copied())
			}
			Proxy::Folder { .. } | Proxy::Decl { .. } => None,
		}
	}

	/// The type of a proxy or alias symbol at a use; `None` for any other.
	pub(super) fn proxy_type(&mut self, symbol: SymbolId, use_node: NodeId) -> Option<TypeId> {
		if let Some(&target) = self.modules.aliases.get(&symbol) {
			let ty = self.symbol_type(target, use_node);
			return Some(ty);
		}
		let proxy = self.modules.proxies.get(&symbol).cloned()?;
		let module = match &proxy {
			Proxy::Export { module }
			| Proxy::Member { module, .. }
			| Proxy::Decl { module, .. } => Some(*module),
			Proxy::Folder { .. } => None,
		};
		if let Some(module) = module {
			self.ensure_analyzed(module);
			if let Some(interface) = self.s.interface(module) {
				for (&param, writes) in &interface.implicit_writes {
					self.implicit_writes
						.entry(param)
						.or_insert_with(|| writes.clone());
				}
				for (&param, &info) in &interface.key_set_params {
					self.key_sets.metadata.insert(param, info);
					self.a.key_set_params.insert(param, info.subject);
				}
				for (&param, &info) in &interface.derived_params {
					self.a.derived_params.insert(
						param,
						crate::analysis::DerivedParam {
							node: use_node,
							..info
						},
					);
				}
				if !interface.complete {
					self.modules.deferred = true;
				}
			}
		}
		let current = self.a.symbols.symbol(symbol).ty;
		if !current.is_unset() {
			return Some(current);
		}
		let exported = match proxy {
			Proxy::Folder { .. } | Proxy::Decl { .. } => return Some(TypeId::UNKNOWN),
			Proxy::Export { module } => {
				self.ensure_analyzed(module);
				let export = self
					.s
					.interface(module)
					.and_then(|interface| interface.export);
				if export.is_none() && self.module_complete(module) {
					let path = self.s.display_path_of(module);
					let name = self
						.s
						.types
						.syms
						.resolve(self.a.symbols.symbol(symbol).name);
					let message = format!(
						"`{path}` exports nothing: add `export` to its main declaration to use `{name}`"
					);
					let diagnostic = self.diagnostic(&E_DECL_1, use_node, message);
					self.report(diagnostic);
					self.a.symbols.symbol_mut(symbol).ty = TypeId::UNKNOWN;
					return Some(TypeId::UNKNOWN);
				}
				export
			}
			Proxy::Member {
				module,
				owner,
				name,
			} => {
				self.ensure_analyzed(module);
				self.s
					.interface(module)
					.and_then(|interface| interface.members.get(&(owner, name)).copied())
			}
		};
		match exported {
			Some(exported) if !exported.ty.is_unset() => {
				self.a.symbols.symbol_mut(symbol).ty = exported.ty;
				Some(exported.ty)
			}
			// a file in progress that hasn't got there yet (a cycle between files)
			_ => {
				self.modules.deferred = true;
				Some(TypeId::UNKNOWN)
			}
		}
	}

	/// The value of a `static` of another file a proxy (or an alias of one)
	/// stands for: its reads are folded like the reads of a local `static`.
	pub(super) fn external_static(&mut self, symbol: SymbolId) -> Option<fa_interpreter::Value> {
		if self.a.symbols.symbol(symbol).kind != SymbolKind::Static {
			return None;
		}
		let resolution = self.external_resolution(symbol)?;
		let (module, decl) = self.s.external_node(resolution)?;
		self.s.interface(module)?.statics.get(&decl).cloned()
	}

	/// Whether the walk of a module finished (its interface is complete).
	fn module_complete(&self, module: ModuleId) -> bool {
		self.s
			.interface(module)
			.is_some_and(|interface| interface.complete)
	}

	/// Analyzes another file, unless its analysis ran or is running.
	fn ensure_analyzed(&mut self, module: ModuleId) {
		let Some(file) = self.s.file_of(module) else {
			return;
		};
		if file != self.file && !self.s.source_file(file).analyzed {
			self.s.analyze_file(file);
		}
	}

	/// Whether a symbol is a namespace (a local one, another file's, a
	/// folder, or an alias of one).
	pub(super) fn is_namespace(&self, symbol: SymbolId) -> bool {
		self.a.symbols.symbol(symbol).kind == SymbolKind::Namespace
	}

	/// The symbol a name used as the target of `a.b` designates when it is a
	/// namespace.
	pub(super) fn namespace_target(&mut self, target: NodeId) -> Option<SymbolId> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, target) else {
			return None;
		};
		// `@tables.squares(5)`: a compile-time call through a namespace
		let text = tree.name(name);
		let sym = self
			.s
			.types
			.syms
			.intern(text.strip_prefix('@').unwrap_or(text));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		self.is_namespace(symbol).then_some(symbol)
	}

	/// The member `text` of a namespace symbol: `MOD009` for a private
	/// member seen from outside; `None` when there is no such member
	/// (reported by the caller).
	fn namespace_member(
		&mut self,
		namespace: SymbolId,
		text: &str,
		at: NodeId,
	) -> Option<SymbolId> {
		let mut namespace = namespace;
		while let Some(&target) = self.modules.aliases.get(&namespace) {
			namespace = target;
		}
		if text.starts_with('#') && !self.is_inside(namespace) {
			let diagnostic = self.diagnostic(
				&MOD009,
				at,
				format!("`{text}` is private: it is only visible inside its namespace"),
			);
			self.report(diagnostic);
			return None;
		}
		let sym = self.s.types.syms.intern(text);
		match self.modules.proxies.get(&namespace).cloned() {
			Some(Proxy::Folder { path }) => {
				let scope = self.proxy_scope(namespace);
				if let Some(found) = self.a.symbols.lookup_in(scope, sym) {
					return Some(found);
				}
				let entry = self.s.resolver()?.member(&path, text);
				self.modules.reads.push(format!("{path}/{text}"));
				self.make_proxy(scope, text, entry?)
			}
			Some(Proxy::Export { module }) => {
				self.ensure_analyzed(module);
				let export = self
					.s
					.interface(module)
					.and_then(|interface| interface.export);
				let Some(export) = export else {
					// a file in progress that hasn't published its export yet
					// (a cycle between files): the program analysis comes back
					if !self.module_complete(module) {
						self.modules.deferred = true;
					}
					return None;
				};
				self.member_proxy(namespace, module, export.members, sym, text)
			}
			Some(Proxy::Decl { .. }) => None,
			Some(Proxy::Member {
				module,
				owner,
				name,
			}) => {
				let members = self.s.interface(module).and_then(|interface| {
					interface
						.members
						.get(&(owner, name))
						.and_then(|member| member.members)
				});
				self.member_proxy(namespace, module, members, sym, text)
			}
			None => {
				// a namespace of this file: analyzed first, its members are in its scope
				if !self.modules.member_scopes.contains_key(&namespace) {
					self.symbol_type(namespace, at);
				}
				let scope = *self.modules.member_scopes.get(&namespace)?;
				self.a.symbols.lookup_in(scope, sym)
			}
		}
	}

	/// The proxy of a member of another file's namespace (its block `owner`).
	fn member_proxy(
		&mut self,
		namespace: SymbolId,
		module: ModuleId,
		owner: Option<NodeId>,
		sym: Sym,
		text: &str,
	) -> Option<SymbolId> {
		let scope = self.proxy_scope(namespace);
		if let Some(found) = self.a.symbols.lookup_in(scope, sym) {
			return Some(found);
		}
		let Some(owner) = owner else {
			self.modules.deferred |= !self.module_complete(module);
			return None;
		};
		let member = self
			.s
			.interface(module)
			.and_then(|interface| interface.members.get(&(owner, sym)).copied());
		let Some(member) = member else {
			if !self.module_complete(module) {
				// not published yet: the program analysis comes back
				self.modules.deferred = true;
			}
			return None;
		};
		let proxy = Proxy::Member {
			module,
			owner,
			name: sym,
		};
		Some(self.declare_proxy(scope, text, member.kind, proxy, member.ty))
	}

	/// The scope holding the member proxies of a proxy namespace.
	fn proxy_scope(&mut self, namespace: SymbolId) -> ScopeId {
		if let Some(&scope) = self.modules.member_scopes.get(&namespace) {
			return scope;
		}
		let owner = NodeId(self.modules.next_owner);
		self.modules.next_owner -= 1;
		let scope = self.a.symbols.scope_for(owner, None, ScopeKind::Namespace);
		self.modules.member_scopes.insert(namespace, scope);
		scope
	}

	/// Whether the current scope is inside a namespace of this file.
	fn is_inside(&self, namespace: SymbolId) -> bool {
		let Some(&members) = self.modules.member_scopes.get(&namespace) else {
			return false;
		};
		if self.modules.proxies.contains_key(&namespace) {
			return false;
		}
		let mut current = Some(self.body.scope);
		while let Some(scope) = current {
			if scope == members {
				return true;
			}
			current = self.a.symbols.scope(scope).parent;
		}
		false
	}

	/// Resolves `namespace.member.member…` as long as the members are
	/// namespaces: the symbol reached and the number of members used. Sets
	/// the resolutions of the names. Type part.
	fn resolve_namespace_chain(
		&mut self,
		target: NodeId,
		namespace: SymbolId,
		members: NodeList<'_>,
	) -> Option<(SymbolId, usize)> {
		self.set_resolution(target, Resolution::Local(namespace));
		self.set_type(target, TypeId::UNKNOWN);
		let tree = Arc::clone(&self.tree);
		let mut current = namespace;
		let mut used = 0;
		for member in members.iter() {
			if !self.is_namespace(current) {
				break;
			}
			let View::Name(name) = syntax::view(&tree, member) else {
				return None;
			};
			let text = tree.name(name);
			let Some(found) = self.namespace_member(current, text, member) else {
				if !self.modules.deferred && !text.starts_with('#') {
					let owner = tree.text(target.index());
					let message = format!("`{owner}` has no member `{text}`");
					let diagnostic = self.diagnostic(&E_DECL_1, member, message);
					self.report(diagnostic);
				}
				return None;
			};
			self.set_resolution(member, Resolution::Local(found));
			if self.is_namespace(found) {
				self.set_type(member, TypeId::UNKNOWN);
			}
			current = found;
			used += 1;
		}
		Some((current, used))
	}

	/// `a.b.c` whose target is a namespace (a folder, a file exporting a
	/// namespace, a namespace of this file): the member's value, then the
	/// fields of that value. `None` when the target is not a namespace.
	pub(super) fn check_module_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Option<TypeId> {
		let namespace = self.namespace_target(target)?;
		Some(self.memo(n, |w| {
			let Some((symbol, used)) = w.resolve_namespace_chain(target, namespace, members) else {
				return TypeId::UNKNOWN;
			};
			let rest = members.skip(used);
			let last = if used == 0 {
				target
			} else {
				members.get(used - 1).unwrap_or(target)
			};
			match w.a.symbols.symbol(symbol).kind {
				SymbolKind::Namespace => {
					let text = w.tree.text(n.index()).to_string();
					let diagnostic = w.diagnostic(
						&TY008,
						n,
						format!("`{text}` is a namespace, not a value: use one of its members"),
					);
					w.report(diagnostic);
					TypeId::UNKNOWN
				}
				SymbolKind::Type => {
					let ty = w.symbol_type(symbol, last);
					if rest.is_empty() {
						let text = w.tree.text(n.index()).to_string();
						let diagnostic =
							w.diagnostic(&TY008, n, format!("`{text}` is a type, not a value"));
						w.report(diagnostic);
						return TypeId::UNKNOWN;
					}
					let origin = w.written_origin(last);
					let saved = std::mem::replace(&mut w.receiver_origin, origin);
					let result = w.module_type_members(ty, rest, true);
					w.receiver_origin = saved;
					result
				}
				_ => {
					let mut current = w.symbol_type(symbol, last);
					w.set_type(last, current);
					let mut origin = w.written_origin(last);
					// `library.settings.port`: a `static` of another file is folded
					if rest.is_empty()
						&& let Some(value) = w.external_static(symbol)
					{
						w.a.constants.insert(n, value);
					}
					for member in rest.iter() {
						let owner = origin.clone();
						let saved = std::mem::replace(&mut w.receiver_origin, origin.take());
						let found = w.member_of(member, current);
						let selected_owner = w.receiver_origin.clone();
						w.receiver_origin = saved;
						match found {
							Some((ty, super::objects::MemberKind::Field(_))) => {
								if let Some(owner) = owner
									&& let View::Name(name) = w.view(member)
								{
									let name = w.s.types.syms.intern(w.tree.name(name));
									origin = w.written_field_origin(owner, name);
								}
								current = ty;
							}
							Some((ty, _)) => {
								if let Some(owner) = selected_owner {
									w.a.method_instantiations.insert(member, owner);
								}
								current = ty;
							}
							None => return TypeId::UNKNOWN,
						}
						if origin.is_none() {
							origin = w.written_origin(member);
						}
						if let Some(origin) = &origin {
							w.a.written_types.insert(member, origin.clone());
						}
					}
					if let Some(origin) = origin {
						w.a.written_types.insert(n, origin);
					}
					current
				}
			}
		}))
	}

	/// The members of a named union or enumeration reached through a module
	/// path (`library.Shape.Circle`).
	fn module_type_members(&mut self, ty: TypeId, rest: NodeList<'_>, value: bool) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let mut current = ty;
		// after a type declared inside a type, the members are resolved (the
		// backends build `library.Farm.Error.Empty` from its last member)
		let mut nested = false;
		for member in rest.iter() {
			if self.s.types.is_recovering(current) {
				return TypeId::UNKNOWN;
			}
			let View::Name(name) = syntax::view(&tree, member) else {
				return TypeId::UNKNOWN;
			};
			let text = tree.name(name);
			// `library.Box.make(4)`: a function of the type, as a value
			if value
				&& self.nominal_member(current, text).is_none()
				&& let Some(function) = self.type_function(current, text)
			{
				if let Some(owner) = self.receiver_origin.clone() {
					self.a.method_instantiations.insert(member, owner);
				}
				self.set_resolution(member, Resolution::Local(function));
				let ty = self.symbol_type(function, member);
				self.set_type(member, ty);
				return ty;
			}
			// `library.Database.OpenError`: a type declared inside the type
			if self.nominal_member(current, text).is_none()
				&& let Some(nested_type) = self.nested_type(current, text, member)
			{
				self.set_type(member, nested_type);
				self.set_flag(member, crate::analysis::NodeFlags::TYPE_EXPR);
				current = nested_type;
				nested = true;
				continue;
			}
			let Some(found) = self.nominal_member(current, text) else {
				let message = format!(
					"`{}` has no member `{}`",
					self.display(current),
					tree.name(name)
				);
				let diagnostic = self.diagnostic(&TY008, member, message);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			self.set_type(member, found);
			if nested {
				self.set_member_resolution(member, found);
			}
			current = found;
		}
		current
	}

	/// `library.Person` in a type: the type a module path names. `None` when
	/// the target is not a namespace.
	pub(super) fn module_type_path(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Option<TypeId> {
		let namespace = self.namespace_target(target)?;
		Some(self.memo(n, |w| {
			let Some((symbol, used)) = w.resolve_namespace_chain(target, namespace, members) else {
				return TypeId::UNKNOWN;
			};
			let last = members.get(used.saturating_sub(1)).unwrap_or(target);
			if w.a.symbols.symbol(symbol).kind != SymbolKind::Type {
				let text = w.tree.text(n.index()).to_string();
				let diagnostic = w.diagnostic(&TY008, n, format!("`{text}` is not a type"));
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let ty = w.symbol_type(symbol, last);
			w.set_type(last, ty);
			w.set_flag(last, crate::analysis::NodeFlags::TYPE_EXPR);
			w.module_type_members(ty, members.skip(used), false)
		}))
	}

	/// The type a namespace path names (`library.Pair`), its nodes resolved:
	/// `None` when the path names no type of a namespace.
	pub(super) fn module_type_symbol(
		&mut self,
		target: NodeId,
		namespace: SymbolId,
		members: NodeList<'_>,
	) -> Option<SymbolId> {
		let (symbol, used) = self.resolve_namespace_chain(target, namespace, members)?;
		if used != members.len() || self.a.symbols.symbol(symbol).kind != SymbolKind::Type {
			return None;
		}
		let last = members.last()?;
		let ty = self.symbol_type(symbol, last);
		self.set_type(last, ty);
		self.set_flag(last, crate::analysis::NodeFlags::TYPE_EXPR);
		Some(symbol)
	}

	/// `library.counter.count = 10`: an assignment target reached through a
	/// namespace. A `mutable` member of a namespace is written only from its
	/// own file (`EFF013`); an `export mutable` value from any file. `None`
	/// when the target is not a namespace.
	pub(super) fn module_place(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Option<PlaceInfo> {
		let namespace = self.namespace_target(target)?;
		let ty = self.check_module_access(n, target, members)?;
		let last = members.last()?;
		let symbol = match self.a.resolution(last) {
			Resolution::Local(symbol) => Some(symbol),
			_ => None,
		};
		let resolved = members
			.iter()
			.take_while(|&member| matches!(self.a.resolution(member), Resolution::Local(_)))
			.count();
		let below = resolved < members.len();
		let root = members.get(resolved.saturating_sub(1)).and_then(|member| {
			match self.a.resolution(member) {
				Resolution::Local(symbol) => Some(symbol),
				_ => None,
			}
		});
		let _ = namespace;
		let Some(root) = root else {
			return Some(PlaceInfo::unknown());
		};
		let kind = self.a.symbols.symbol(root).kind;
		let foreign_member = matches!(self.modules.proxies.get(&root), Some(Proxy::Member { .. }));
		let mut assignable = kind.is_assignable();
		if assignable && foreign_member {
			if self.reported_once.insert((n, "EFF013")) {
				let text = self.tree.text(n.index()).to_string();
				let path = match self.modules.proxies.get(&root) {
					Some(Proxy::Member { module, .. }) => self.s.display_path_of(*module),
					_ => String::new(),
				};
				let diagnostic = self.diagnostic(
					&EFF013,
					n,
					format!(
						"`{text}` can only be written from `{path}`: call a function of its namespace that changes it"
					),
				);
				self.report(diagnostic);
			}
			assignable = true;
		}
		Some(PlaceInfo {
			symbol: if below { None } else { symbol },
			place: None,
			ty,
			assignable,
			below,
		})
	}

	/// A name designating a namespace used as a value (`let x = utils`).
	pub(super) fn namespace_as_value(&mut self, n: NodeId, text: &str) -> TypeId {
		let diagnostic = self.diagnostic(
			&TY008,
			n,
			format!("`{text}` is a namespace, not a value: use one of its members, ex: `{text}.…`"),
		);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	// ── namespaces of this file ─────────────────────────────────────────

	/// `namespace name = { … }` and `export namespace = { … }`: the members
	/// are declared in the scope of the block, hoisted, then walked in order.
	pub(super) fn check_namespace_members(&mut self, n: NodeId, value: NodeId) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let statements = match syntax::view(&tree, value) {
			View::Block { statements } | View::Object { items: statements } => statements,
			_ => return self.unsupported(n, "a namespace that is not a block"),
		};
		let Some(symbol) = self.a.symbols.symbol_of_decl(n) else {
			return TypeId::VOID;
		};
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(value, Some(parent), ScopeKind::Namespace);
		self.modules.member_scopes.insert(symbol, scope);
		self.body.scope = scope;
		self.declare_hoisted(value);
		let test_file = self.s.role(self.file) == FileRole::Test;
		for statement in statements.iter() {
			if !self.is_declaration_statement(statement, test_file) {
				let diagnostic = self.diagnostic(
					&MOD006,
					statement,
					"a namespace holds declarations: move this statement into a function",
				);
				self.report(diagnostic);
			}
		}
		self.walk_declarations(statements);
		self.body.scope = parent;
		let symbol = self.a.symbols.symbol_mut(symbol);
		if symbol.ty.is_unset() {
			symbol.ty = TypeId::UNKNOWN;
		}
		TypeId::VOID
	}

	/// Walks the statements of a module or a namespace: the ones not
	/// hoisted in order, then the hoisted ones not analyzed on demand yet.
	pub(super) fn walk_declarations(&mut self, statements: NodeList<'_>) {
		// a `derived` value is hoisted only to be read before its declaration
		// (a cycle, reactivity.rs): it is analyzed in order
		let is_hoisted = |w: &Self, statement: NodeId| {
			w.a.symbols.symbol_of_decl(statement).is_some_and(|symbol| {
				let symbol = w.a.symbols.symbol(symbol);
				symbol.state == DeclState::Pending && symbol.kind != SymbolKind::Derived
			})
		};
		for statement in statements.iter() {
			if !is_hoisted(self, statement) {
				self.check_stmt(statement);
			}
		}
		for statement in statements.iter() {
			if is_hoisted(self, statement) {
				self.check_stmt(statement);
			}
		}
	}

	// ── `use` aliases of module paths (D14) ──────────────────────────────

	/// The symbol a module path designates (`utils`, `utils.text`,
	/// `react.Component`), when it starts with a namespace.
	fn module_path_symbol(&mut self, value: NodeId) -> Option<SymbolId> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, value) {
			View::Call { .. } => {
				let symbol = self.import_proxy(value)?;
				self.set_resolution(value, Resolution::Local(symbol));
				self.set_type(value, TypeId::UNKNOWN);
				self.is_namespace(symbol).then_some(symbol)
			}
			View::Name(_) => {
				let symbol = self.namespace_target(value)?;
				self.set_resolution(value, Resolution::Local(symbol));
				self.set_type(value, TypeId::UNKNOWN);
				Some(symbol)
			}
			View::Access { target, members } => {
				let namespace = self.namespace_target(target)?;
				let (symbol, used) = self.resolve_namespace_chain(target, namespace, members)?;
				(used == members.len()).then_some(symbol)
			}
			View::Group(inner) => self.module_path_symbol(inner),
			_ => None,
		}
	}

	/// Whether a `use` value is a module path (a name alias, D14) rather
	/// than a place (a tracked reference).
	pub(super) fn is_module_path(&mut self, value: NodeId) -> bool {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, value) {
			View::Name(_) => {
				self.namespace_target(value).is_some() || {
					let View::Name(name) = syntax::view(&tree, value) else {
						return false;
					};
					let sym = self.s.types.syms.intern(tree.name(name));
					self.a
						.symbols
						.lookup(self.body.scope, sym)
						.is_some_and(|symbol| self.modules.proxies.contains_key(&symbol))
				}
			}
			View::Access { target, .. } => self.namespace_target(target).is_some(),
			View::Group(inner) => self.is_module_path(inner),
			View::Call { .. } => self.import_proxy(value).is_some(),
			_ => false,
		}
	}

	/// The proxy of the module an `@import("source")` designates: `fa`
	/// resolved its source (scan.rs, `import_name`).
	pub(super) fn import_proxy(&mut self, value: NodeId) -> Option<SymbolId> {
		let tree = Arc::clone(&self.tree);
		let source = crate::scan::import_source(&tree, value)?;
		let sym = self.s.types.syms.intern(&crate::scan::import_name(source));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		self.modules.proxies.contains_key(&symbol).then_some(symbol)
	}

	/// `use Name = path`: a name alias of a module path.
	pub(super) fn check_use_alias(&mut self, n: NodeId, name: Name, value: NodeId) -> TypeId {
		self.memo(n, |w| {
			let tree = Arc::clone(&w.tree);
			let target = match syntax::view(&tree, value) {
				View::Name(target_name) if w.namespace_target(value).is_none() => {
					// a file's export: the proxy itself
					let sym = w.s.types.syms.intern(tree.name(target_name));
					let symbol = w.a.symbols.lookup(w.body.scope, sym);
					if let Some(symbol) = symbol {
						w.set_resolution(value, Resolution::Local(symbol));
					}
					symbol
				}
				// `use weather = @import("…")`: the proxy of its source
				View::Call { .. } => {
					let symbol = w.import_proxy(value);
					if let Some(symbol) = symbol {
						w.set_resolution(value, Resolution::Local(symbol));
					}
					symbol
				}
				_ => w.module_path_symbol(value),
			};
			if let Some(target) = target {
				w.declare_alias(n, name, target, value);
			}
			TypeId::VOID
		})
	}

	/// `use { a, b as c, folder { d } } = path`: aliases of members.
	pub(super) fn check_use_aliases(
		&mut self,
		n: NodeId,
		pattern: NodeId,
		value: NodeId,
	) -> TypeId {
		self.memo(n, |w| {
			if let Some(namespace) = w.module_path_symbol(value) {
				w.alias_members(n, pattern, namespace);
			}
			TypeId::VOID
		})
	}

	/// Declares the aliases of an object pattern's fields, members of `namespace`.
	fn alias_members(&mut self, n: NodeId, pattern: NodeId, namespace: SymbolId) {
		let tree = Arc::clone(&self.tree);
		let View::ObjectPattern { fields, .. } = syntax::view(&tree, pattern) else {
			let diagnostic = self.diagnostic(
				&TY008,
				pattern,
				"a namespace is destructured with braces, ex: `use { format } = utils`",
			);
			self.report(diagnostic);
			return;
		};
		for field in fields.iter() {
			let text = tree.name(field.name);
			let Some(member) = self.namespace_member(namespace, text, pattern) else {
				if !self.modules.deferred && !text.starts_with('#') {
					let shown = tree.text(pattern.index());
					let message = format!("`{text}` is not a member of this namespace ({shown})");
					let diagnostic =
						self.diagnostic_at(&E_DECL_1, syntax::name_span(field.name), message);
					self.report(diagnostic);
				}
				continue;
			};
			match field.nested {
				Some(nested) => self.alias_members(n, nested, member),
				None => {
					let name = field.alias.unwrap_or(field.name);
					self.declare_alias(n, name, member, pattern);
				}
			}
		}
	}

	/// Declares `name` as an alias of `target`.
	fn declare_alias(&mut self, n: NodeId, name: Name, target: SymbolId, at: NodeId) {
		if name.is_empty() {
			return;
		}
		let kind = self.a.symbols.symbol(target).kind;
		let ty = if kind == SymbolKind::Namespace {
			TypeId::UNKNOWN
		} else {
			self.symbol_type(target, at)
		};
		let symbol = self.declare(n, name, kind);
		let data = self.a.symbols.symbol_mut(symbol);
		data.ty = ty;
		data.state = DeclState::Done;
		self.modules.aliases.insert(symbol, target);
	}
}
