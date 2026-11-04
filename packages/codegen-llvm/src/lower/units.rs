//! One unit of a program compiled per unit (design/parallel-pipeline.md
//! §3.3, step 6): the files of one strongly connected component of the
//! module graph, lowered into a module of their own. The other modules are
//! read through their published interfaces only.
//!
//! - **Symbols**: a declaration other modules may name (it has a
//!   [`DeclKey`]) gets the symbol `fa.<module path>.<names>[#<n>]`, hidden;
//!   everything else is internal. A function or a module value of another
//!   unit is declared with its symbol, its signature built from its type in
//!   the interface.
//! - **Module values**: `fa.init.<module>.<node>` initializes the values of
//!   one declaration, and `fa.drop.<module>.<node>` drops them; the entry
//!   object calls them in the program's initialization order, and the drops
//!   at a normal exit in the reverse order.
//! - **The entry file**: its top level and the call of its entry function
//!   are `fa.main.<module>`, which returns the exit code.
//! - **Unions**: the ids of the members are stable hashes (the same in
//!   every unit); a unit's `fa.drop_box`, `fa.copy_box` and
//!   `fa_format_union` switches are renamed after the unit and called by
//!   the entry object's switches, for the ids the unit registered.
//! - **The runtime** is declared only: it is an object of its own.

use fa_analyzer::ids::{DeclKey, FileId, ModuleId, NodeId, ScopeId, SymbolId, TypeId};
use fa_analyzer::symbols::SymbolKind;
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;
use inkwell::GlobalVisibility;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::FunctionValue;
use rustc_hash::{FxHashMap, FxHashSet};

use super::ownership::Cleanup;
use super::values::Repr;
use super::{FAST_CALLING_CONVENTION, Function, Lowering, Slot};
use crate::error::Error;
use crate::split::UnitSymbols;

/// What a declaration of another unit is, from its module's interface.
#[derive(Copy, Clone)]
struct RemoteDecl {
	kind: SymbolKind,
	ty: TypeId,
	decl: NodeId,
	module: ModuleId,
}

/// The state of the lowering of one unit.
pub(crate) struct Split<'ctx> {
	/// The unit's name: its first module path.
	name: String,
	/// The unit's files.
	local: FxHashSet<FileId>,
	/// The entry file of the program, when it is in this unit.
	entry: Option<FileId>,
	/// The keys of the unit's declarations other modules may name.
	keys: FxHashMap<(FileId, NodeId), DeclKey>,
	/// The functions and values of other units declared so far.
	remote_functions: FxHashMap<String, Function<'ctx>>,
	remote_globals: FxHashMap<DeclKey, Slot<'ctx>>,
	/// The declarations of other modules, by module (from their interfaces).
	remote_index:
		std::cell::RefCell<FxHashMap<ModuleId, std::rc::Rc<FxHashMap<DeclKey, RemoteDecl>>>>,
	/// The symbols the unit defines for the other objects.
	exports: FxHashSet<String>,
	/// What the entry object needs to know of the unit.
	symbols: UnitSymbols,
	/// The instances of generic functions of other units its code calls:
	/// their keys and type arguments (types of this unit's view).
	requests: Vec<Request>,
	/// Lowering instances requested by other units (an instance wave): the
	/// unit's own functions and values are defined by its main object.
	wave: bool,
	/// A module value an instance of a wave reads that other objects can't
	/// name (no key).
	wave_error: Option<String>,
	/// The functions of a wave that the unit's main object defines.
	elsewhere: FxHashSet<String>,
}

/// An instance of another unit's generic function: its key and its type
/// arguments.
pub(crate) type Request = (DeclKey, Vec<TypeId>);

/// The symbol of an instance of a generic function: its declaration's,
/// and the stable hashes of its type arguments.
pub(crate) fn instance_symbol(
	types: &fa_analyzer::types::TypeStore,
	key: DeclKey,
	arguments: &[TypeId],
) -> String {
	let hashes: Vec<String> = arguments
		.iter()
		.map(|&argument| format!("{:x}", types.stable_hash(argument)))
		.collect();
	format!("{}<{}>", mangle(types, key), hashes.join(","))
}

/// The symbol of a declaration other modules may name.
pub(crate) fn mangle(types: &fa_analyzer::types::TypeStore, key: DeclKey) -> String {
	let data = types.decl_key_data(key);
	let mut name = format!("fa.{}", types.module_path(data.module));
	for &sym in types.sym_list(data.names) {
		name.push('.');
		name.push_str(types.syms.resolve(sym));
	}
	if data.disambiguator != 0 {
		name.push_str(&format!("#{}", data.disambiguator));
	}
	name
}

/// A node standing for no node of the tree (the parts of a function of
/// another unit that only its own unit reads).
const NO_NODE: NodeId = NodeId(u32::MAX);

impl<'ctx> Split<'ctx> {
	/// The state of a unit of these files.
	pub(crate) fn new(
		session: &fa_analyzer::Session,
		name: String,
		files: &[FileId],
		entry: Option<FileId>,
	) -> Self {
		let mut keys = FxHashMap::default();
		for &file in files {
			for (&key, keyed) in &session.source_file(file).interface.keys {
				keys.insert((file, keyed.node), key);
			}
		}
		Split {
			name,
			local: files.iter().copied().collect(),
			entry,
			keys,
			remote_functions: FxHashMap::default(),
			remote_globals: FxHashMap::default(),
			remote_index: Default::default(),
			exports: FxHashSet::default(),
			symbols: UnitSymbols::default(),
			requests: Vec::new(),
			wave: false,
			wave_error: None,
			elsewhere: FxHashSet::default(),
		}
	}
}

impl<'a, 'ctx> Lowering<'a, 'ctx> {
	/// Makes this lowering the lowering of one unit (`files`: the unit's
	/// files, `entry`: the program's entry file when it is one of them).
	pub(crate) fn into_unit(
		mut self,
		name: String,
		files: &[FileId],
		entry: Option<FileId>,
	) -> Self {
		self.split = Some(Box::new(Split::new(self.session, name, files, entry)));
		self.runtime = self.runtime.declaring_only();
		self
	}

	/// Whether `fa_format_union` registers every built-in error: the whole
	/// program's, or the unit holding the program's entry.
	pub(crate) fn registers_builtin_errors(&self) -> bool {
		self.split
			.as_ref()
			.is_none_or(|split| split.entry.is_some())
	}

	/// Whether the program is compiled per unit.
	pub(crate) fn is_split(&self) -> bool {
		self.split.is_some()
	}

	fn split_mut(&mut self) -> &mut Split<'ctx> {
		self.split.as_mut().expect("a unit")
	}

	/// The key of a declaration of the unit other modules may name.
	fn local_key(&self, node: NodeId) -> Option<DeclKey> {
		let split = self.split.as_ref()?;
		split.keys.get(&(self.file, node)).copied()
	}

	/// Exports a function of the unit that other modules call (it has a
	/// key): its symbol, hidden.
	pub(crate) fn export_function(&mut self, node: NodeId, value: FunctionValue<'ctx>) {
		let Some(key) = self.local_key(node) else {
			return;
		};
		let name = mangle(self.types(), key);
		value.as_global_value().set_name(&name);
		let split = self.split_mut();
		if split.wave {
			// defined by the unit's main object
			split.elsewhere.insert(name);
		} else {
			split.exports.insert(name);
		}
	}

	/// Exports the global of a module value of the unit that other modules
	/// read (it has a key).
	pub(crate) fn export_global(
		&mut self,
		symbol: SymbolId,
		global: inkwell::values::GlobalValue<'ctx>,
	) {
		if self.split.is_none() {
			return;
		}
		let node = self.analysis.symbols.symbol(symbol).decl_node;
		let wave = self.split.as_ref().is_some_and(|split| split.wave);
		let Some(key) = self.local_key(node) else {
			if wave && self.module_file {
				// a module value of the unit: defined by its main object
				let location = self.location(node.index());
				self.split_mut().wave_error.get_or_insert(location);
			}
			return;
		};
		let name = mangle(self.types(), key);
		global.set_name(&name);
		if wave {
			// defined by the unit's main object
			// SAFETY: a null initializer makes the global a declaration
			unsafe {
				llvm_sys::core::LLVMSetInitializer(
					inkwell::values::AsValueRef::as_value_ref(&global),
					std::ptr::null_mut(),
				);
			}
			global.set_constant(false);
			global.set_linkage(Linkage::External);
			global.set_visibility(GlobalVisibility::Hidden);
			return;
		}
		self.split_mut().exports.insert(name);
	}

	/// Whether a key names a declaration of one of the unit's modules.
	pub(crate) fn is_local_key(&self, key: DeclKey) -> bool {
		let Some(split) = self.split.as_ref() else {
			return true;
		};
		let module = self.types().decl_key_data(key).module;
		self.session
			.file_of(module)
			.is_some_and(|file| split.local.contains(&file))
	}

	/// A declaration of another unit, and whether it is a function
	/// (`None`: a type, a namespace).
	pub(crate) fn remote(&self, key: DeclKey) -> Option<(DeclKey, bool)> {
		match self.remote_decl(key)?.kind {
			SymbolKind::Function => Some((key, true)),
			SymbolKind::Let | SymbolKind::Mutable => Some((key, false)),
			_ => None,
		}
	}

	/// What the interface of its module says of a declaration of another unit.
	fn remote_decl(&self, key: DeclKey) -> Option<RemoteDecl> {
		let split = self.split.as_ref()?;
		let module = self.types().decl_key_data(key).module;
		let index = split.remote_index.borrow().get(&module).cloned();
		let index = match index {
			Some(index) => index,
			None => {
				let mut index = FxHashMap::default();
				if let Some(interface) = self.session.interface(module) {
					for exported in interface.members.values().chain(&interface.export) {
						index.insert(
							exported.key,
							RemoteDecl {
								kind: exported.kind,
								ty: exported.ty,
								decl: exported.decl,
								module,
							},
						);
					}
					for functions in interface.type_functions.values() {
						for function in functions {
							index.insert(
								function.key,
								RemoteDecl {
									kind: SymbolKind::Function,
									ty: function.ty,
									decl: function.decl,
									module,
								},
							);
						}
					}
				}
				let index = std::rc::Rc::new(index);
				split
					.remote_index
					.borrow_mut()
					.insert(module, std::rc::Rc::clone(&index));
				index
			}
		};
		index.get(&key).copied()
	}

	/// The id of a member of a named union, the same in every unit: a hash
	/// of the union's declaration and type arguments (never a built-in
	/// error's id, below 16).
	pub(crate) fn stable_member_id(&self, nominal: fa_analyzer::ids::NominalId, index: u16) -> u64 {
		let types = self.types();
		let info = types.nominal(nominal);
		let mut hash = types.decl_hash(info.key);
		for &argument in types.list_items(info.args) {
			hash = hash.rotate_left(7) ^ types.stable_hash(argument);
		}
		let hash =
			(hash ^ (hash >> 64)) as u64 ^ u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15);
		(hash & !(1 << 63)) | (1 << 62)
	}

	/// The id of a member type of an anonymous union, the same in every unit.
	pub(crate) fn stable_type_id(&self, ty: TypeId) -> u64 {
		let hash = self.types().stable_hash(ty);
		(hash ^ (hash >> 64)) as u64 | (1 << 63)
	}

	/// The global of a module value of another unit, declared.
	pub(crate) fn remote_global(
		&mut self,
		key: DeclKey,
		node: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		if let Some(slot) = self
			.split
			.as_ref()
			.and_then(|split| split.remote_globals.get(&key))
		{
			return Ok(*slot);
		}
		let Some(declared) = self.remote_decl(key) else {
			return Err(self.unsupported(node.index(), "this value of another module"));
		};
		let repr = self.repr_of(declared.ty, node.index())?;
		if !repr.has_value() {
			return Err(self.unsupported(node.index(), "this value of another module"));
		}
		let name = mangle(self.types(), key);
		let global = match self.module.get_global(&name) {
			Some(global) => global,
			None => {
				let global = self.module.add_global(self.llvm_type(repr), None, &name);
				global.set_linkage(Linkage::External);
				global.set_visibility(GlobalVisibility::Hidden);
				global
			}
		};
		let slot = Slot {
			pointer: global.as_pointer_value(),
			repr,
		};
		self.split_mut().remote_globals.insert(key, slot);
		Ok(slot)
	}

	/// A function of another unit, declared: its signature built from its
	/// type, as [`Lowering::declare_function`] builds it from the function.
	pub(crate) fn remote_function(
		&mut self,
		key: DeclKey,
		node: NodeId,
	) -> Result<Function<'ctx>, Error> {
		let Some(declared) = self.remote_decl(key) else {
			return Err(self.unsupported(node.index(), "calling this function of another module"));
		};
		let TypeKind::Function(sig_id) = self.types().kind(declared.ty) else {
			return Err(self.unsupported(node.index(), "calling this function of another module"));
		};
		let sig = self.types().sig(sig_id);
		if sig.is_async {
			return Err(self.unsupported(
				node.index(),
				"an async function of another module (split build)",
			));
		}
		let params: Vec<fa_analyzer::types::ParamSig> = self.types().params(sig.params).to_vec();
		// a generic function: the instance for the call's type arguments,
		// which its own unit makes in an instance wave
		let mut generics = Vec::new();
		for ty in params
			.iter()
			.map(|param| param.ty)
			.chain([sig.ret, sig.throws])
		{
			self.collect_params(ty, &mut generics);
		}
		let (name, bindings, arguments) = if generics.is_empty() {
			(mangle(self.types(), key), FxHashMap::default(), Vec::new())
		} else {
			let Some(list) = self.analysis.instantiation(node) else {
				return Err(self.unsupported(node.index(), "this generic call"));
			};
			let arguments: Vec<TypeId> = self.types().list_items(list).to_vec();
			if arguments
				.iter()
				.any(|&argument| self.mentions_param(argument))
			{
				return Err(self.unsupported(
					node.index(),
					"a generic function of another module called from a generic function (split build)",
				));
			}
			let mut bindings = FxHashMap::default();
			for &param in &generics {
				let index = self.types().param(param).index as usize;
				let Some(&argument) = arguments.get(index) else {
					return Err(self.unsupported(node.index(), "this generic call"));
				};
				bindings.insert(param, self.repr_of(argument, node.index())?);
			}
			(
				instance_symbol(self.types(), key, &arguments),
				bindings,
				arguments,
			)
		};
		if let Some(function) = self
			.split
			.as_ref()
			.and_then(|split| split.remote_functions.get(&name))
		{
			return Ok(function.clone());
		}
		if !arguments.is_empty() {
			self.split_mut().requests.push((key, arguments));
		}
		let mut merged = self.bindings.borrow().clone();
		merged.extend(bindings.iter().map(|(param, repr)| (*param, *repr)));
		let saved = self.bindings.replace(merged);
		let function = self.remote_signature(key, node, name, declared, sig_id, params, bindings);
		self.bindings.replace(saved);
		let function = function?;
		self.split_mut().remote_functions.insert(
			function.value.get_name().to_string_lossy().into_owned(),
			function.clone(),
		);
		Ok(function)
	}

	/// The declaration of a function of another unit, under the bindings of
	/// its type parameters (an instance).
	#[allow(clippy::too_many_arguments)]
	fn remote_signature(
		&mut self,
		key: DeclKey,
		node: NodeId,
		name: String,
		declared: RemoteDecl,
		sig_id: fa_analyzer::ids::SigId,
		params: Vec<fa_analyzer::types::ParamSig>,
		bindings: FxHashMap<fa_analyzer::ids::ParamId, Repr>,
	) -> Result<Function<'ctx>, Error> {
		let _ = key;
		let sig = self.types().sig(sig_id);
		let defaults: Vec<bool> = self
			.session
			.interface(declared.module)
			.and_then(|interface| interface.parameters.get(&declared.decl))
			.map(|parameters| {
				parameters
					.iter()
					.map(|parameter| parameter.has_default)
					.collect()
			})
			.unwrap_or_default();
		let ret = self.repr_of(sig.ret, node.index())?;
		let mut by_pointer = vec![false; params.len()];
		let mut borrowed = vec![false; params.len()];
		let (mutates, invalidates) = self.types().effects(sig.effects);
		for path in mutates.iter().chain(invalidates) {
			let (data, _) = self.types().path_data(*path);
			let index = match data.root {
				fa_analyzer::types::PathRoot::Param(index) => index as usize,
				fa_analyzer::types::PathRoot::SelfParam => 0,
				_ => continue,
			};
			if let Some(flag) = by_pointer.get_mut(index) {
				*flag = true;
			}
		}
		let mut types: Vec<BasicMetadataTypeEnum> = Vec::new();
		let mut lowered_params = Vec::new();
		let mut infos = Vec::new();
		for (position, param) in params.iter().enumerate() {
			let repr = self.repr_of(param.ty, node.index())?;
			if !repr.has_value() {
				return Err(self.unsupported(node.index(), "this parameter type"));
			}
			if !by_pointer[position] && self.borrows_parameter(repr) {
				by_pointer[position] = true;
				borrowed[position] = true;
			}
			let has_default = defaults.get(position).copied().unwrap_or(false);
			if by_pointer[position] {
				if has_default && !borrowed[position] {
					return Err(
						self.unsupported(node.index(), "a changed parameter with a default value")
					);
				}
				types.push(self.pointer_type().into());
			} else {
				types.push(self.llvm_type(repr).into());
			}
			if has_default {
				types.push(self.context.bool_type().into());
			}
			lowered_params.push((repr, has_default));
			infos.push(fa_analyzer::analysis::FnParam {
				node: NO_NODE,
				symbol: SymbolId(u32::MAX),
				default: has_default.then_some(NO_NODE),
				variadic: param.variadic,
			});
		}
		for _ in self.types().requirements(sig.requires) {
			types.push(self.pointer_type().into());
		}
		let throws = sig.throws != TypeId::NEVER;
		let fn_type = match (ret.has_value(), throws) {
			(true, false) => self.llvm_type(ret).fn_type(&types, false),
			(false, false) => self.context.void_type().fn_type(&types, false),
			(true, true) => self
				.context
				.struct_type(&[self.llvm_type(ret), self.pointer_type().into()], false)
				.fn_type(&types, false),
			(false, true) => self.pointer_type().fn_type(&types, false),
		};
		let value = match self.module.get_function(&name) {
			Some(value) => value,
			None => {
				let value = self
					.module
					.add_function(&name, fn_type, Some(Linkage::External));
				value
					.as_global_value()
					.set_visibility(GlobalVisibility::Hidden);
				value.set_call_conventions(FAST_CALLING_CONVENTION);
				self.runtime.add_attributes(value, &["nounwind"]);
				if ret == Repr::Never && !throws {
					self.runtime.add_attributes(value, &["noreturn"]);
				}
				value
			}
		};
		let info = fa_analyzer::analysis::FnInfo {
			ty: declared.ty,
			sig: Some(sig_id),
			symbol: None,
			declaration: Some(NO_NODE),
			lambda: NO_NODE,
			params: infos,
			body: NO_NODE,
			block_body: false,
			declared_return: true,
			scope: ScopeId(u32::MAX),
			throws: sig.throws,
			captures: Vec::new(),
			generics: Vec::new(),
		};
		let function = Function {
			value,
			arena_body: None,
			info,
			params: lowered_params,
			by_pointer,
			borrowed,
			ret,
			throws,
			lambda: false,
			by_reference: false,
			captures: Vec::new(),
			declaration: NO_NODE,
			bindings,
		};
		Ok(function)
	}

	/// The type parameters a type mentions, in order of appearance.
	fn collect_params(&self, ty: TypeId, found: &mut Vec<fa_analyzer::ids::ParamId>) {
		let mut stack = vec![ty];
		let mut seen = FxHashSet::default();
		while let Some(ty) = stack.pop() {
			if !seen.insert(ty) || seen.len() > 4096 {
				continue;
			}
			match self.types().kind(ty) {
				TypeKind::Param(param) => {
					if !found.contains(&param) {
						found.push(param);
					}
				}
				TypeKind::Collection(_, item, key) => stack.extend([item, key]),
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					stack.extend(self.types().list_items(list).iter().copied());
				}
				TypeKind::Pointer(item) | TypeKind::Ref(item, _) | TypeKind::Iterator(item, _) => {
					stack.push(item);
				}
				TypeKind::Task(value, error) => stack.extend([value, error]),
				TypeKind::Sync(_, value) => stack.push(value),
				TypeKind::Function(sig) => {
					let sig = self.types().sig(sig);
					stack.extend(self.types().params(sig.params).iter().map(|param| param.ty));
					stack.push(sig.ret);
				}
				TypeKind::Object(_) => {
					for field in self.types().fields_of(ty) {
						stack.push(field.ty);
					}
				}
				_ => {}
			}
		}
	}

	/// Whether a type mentions a generic parameter.
	fn mentions_param(&self, ty: TypeId) -> bool {
		let mut found = Vec::new();
		self.collect_params(ty, &mut found);
		!found.is_empty()
	}

	/// Lowers the unit: its functions, the initialization and the drops of
	/// its module values, the top level of the entry file if it is one of
	/// its files. Gives what the entry object needs to know.
	pub(crate) fn lower_unit(mut self) -> Result<(UnitSymbols, Vec<Request>), Error> {
		// the flag of the drops at exit: defined by the entry object
		let flag = self
			.module
			.add_global(self.context.bool_type(), None, "fa.drops_at_exit");
		flag.set_linkage(Linkage::External);
		flag.set_constant(true);
		flag.set_visibility(GlobalVisibility::Hidden);
		let mut files: Vec<FileId> = self.other_files.keys().copied().collect();
		files.push(self.file);
		files.sort();
		for &file in &files {
			self.switch_file(file);
			self.index_functions()?;
			self.find_shared_locals();
			let lambdas: Vec<NodeId> = self
				.functions
				.iter()
				.filter(|(_, function)| function.lambda)
				.map(|(node, _)| *node)
				.collect();
			for lambda in lambdas {
				let captures = self.find_captures(lambda);
				if let Some(function) = self.functions.get_mut(&lambda) {
					function.captures = captures;
				}
			}
			self.copy_cleanup_captures();
		}
		for &file in &files {
			self.switch_file(file);
			let mut declarations: Vec<NodeId> = self.functions.keys().copied().collect();
			declarations.sort();
			for declaration in declarations {
				self.lower_function(declaration)?;
			}
		}
		for &file in &files {
			self.switch_file(file);
			if self.module_file {
				self.lower_module_values()?;
			}
		}
		if let Some(entry) = self.split.as_ref().and_then(|split| split.entry) {
			self.switch_file(entry);
			self.lower_entry_body()?;
		}
		while let Some((file, instance)) = self.pending_instances.pop() {
			self.switch_file(file);
			self.lower_function(instance)?;
		}
		if self.tasks.asynchronous {
			return Err(Error::Unsupported {
				message: "an asynchronous program compiled per unit".to_string(),
				location: String::new(),
			});
		}
		self.define_format_union()?;
		self.define_boxes()?;
		self.export_box_functions();
		let mut symbols = std::mem::take(&mut self.split_mut().symbols);
		symbols.effectful_drops = self.effectful_drops.get();
		symbols.uses_arguments = self.uses_arguments();
		let mut boxes: Vec<u64> = self.union_ids.borrow().values().copied().collect();
		boxes.extend(self.type_ids.borrow().values().copied());
		boxes.sort_unstable();
		boxes.dedup();
		symbols.boxes = boxes;
		self.finish_linkage()?;
		let requests = std::mem::take(&mut self.split_mut().requests);
		Ok((symbols, requests))
	}

	/// Lowers the instances of the unit's generic functions other units
	/// requested (an instance wave): the instances, and the private
	/// functions they call (internal copies); the rest is the unit's main
	/// object's.
	pub(crate) fn lower_instances(
		mut self,
		requests: &[Request],
	) -> Result<(UnitSymbols, Vec<Request>), Error> {
		self.split_mut().wave = true;
		let mut files: Vec<FileId> = self.other_files.keys().copied().collect();
		files.push(self.file);
		files.sort();
		for &file in &files {
			self.switch_file(file);
			self.index_functions()?;
			self.find_shared_locals();
			let lambdas: Vec<NodeId> = self
				.functions
				.iter()
				.filter(|(_, function)| function.lambda)
				.map(|(node, _)| *node)
				.collect();
			for lambda in lambdas {
				let captures = self.find_captures(lambda);
				if let Some(function) = self.functions.get_mut(&lambda) {
					function.captures = captures;
				}
			}
			self.copy_cleanup_captures();
		}
		let no_location = || Error::Unsupported {
			message: "an instance of a generic function (split build)".to_string(),
			location: String::new(),
		};
		for (key, arguments) in requests {
			let (module, keyed) = self.session.keyed(*key).ok_or_else(no_location)?;
			let file = self.session.file_of(module).ok_or_else(no_location)?;
			self.switch_file(file);
			if !self.generic_functions.contains_key(&keyed.node) {
				return Err(
					self.unsupported(keyed.node.index(), "this generic function (split build)")
				);
			}
			let mut reprs = Vec::new();
			for &argument in arguments {
				reprs.push(self.repr_of(argument, keyed.node.index())?);
			}
			let instance = self.instance_of(keyed.node, reprs)?;
			let name = instance_symbol(self.types(), *key, arguments);
			let value = self.functions[&instance].value;
			if self.module.get_function(&name).is_some() {
				// the same representations as another request: one function
				// can't have both names
				return Err(self.unsupported(
					keyed.node.index(),
					"two instances of one representation (split build)",
				));
			}
			value.as_global_value().set_name(&name);
			self.split_mut().exports.insert(name);
		}
		// the instances, then the private functions they call
		loop {
			while let Some((file, instance)) = self.pending_instances.pop() {
				self.switch_file(file);
				self.lower_function(instance)?;
			}
			let mut lowered = false;
			for &file in &files {
				self.switch_file(file);
				let mut needed: Vec<NodeId> = self
					.functions
					.iter()
					.filter(|(_, function)| {
						function.value.count_basic_blocks() == 0
							&& function.value.get_linkage() == Linkage::Internal
							&& !self.split.as_ref().is_some_and(|split| {
								split
									.elsewhere
									.contains(function.value.get_name().to_string_lossy().as_ref())
							}) && inkwell::values::BasicValue::get_first_use(
							&function.value.as_global_value().as_pointer_value(),
						)
						.is_some()
					})
					.map(|(node, _)| *node)
					.collect();
				needed.sort();
				for node in needed {
					self.lower_function(node)?;
					lowered = true;
				}
			}
			if !lowered && self.pending_instances.is_empty() {
				break;
			}
		}
		if let Some(location) = self
			.split
			.as_ref()
			.and_then(|split| split.wave_error.clone())
		{
			return Err(Error::Unsupported {
				message: "a module value without a key read by a generic instance (split build)"
					.to_string(),
				location,
			});
		}
		self.define_format_union()?;
		self.define_boxes()?;
		self.export_box_functions();
		let mut symbols = std::mem::take(&mut self.split_mut().symbols);
		symbols.effectful_drops = self.effectful_drops.get();
		let mut boxes: Vec<u64> = self.union_ids.borrow().values().copied().collect();
		boxes.extend(self.type_ids.borrow().values().copied());
		boxes.sort_unstable();
		boxes.dedup();
		symbols.boxes = boxes;
		self.finish_linkage()?;
		let requests = std::mem::take(&mut self.split_mut().requests);
		Ok((symbols, requests))
	}

	/// The initialization of the module values of the file (a module) other
	/// modules or the program may read: a function per declaration, and
	/// the function dropping what it declared.
	fn lower_module_values(&mut self) -> Result<(), Error> {
		let module = self.session.module(self.file).to_string();
		let interface = &self.session.source_file(self.file).interface;
		let mut nodes: Vec<NodeId> = interface
			.keys
			.values()
			.map(|keyed| keyed.node)
			.filter(|node| {
				interface.declared.get(node).is_some_and(|declared| {
					matches!(declared.kind, SymbolKind::Let | SymbolKind::Mutable)
				})
			})
			.collect();
		nodes.sort();
		nodes.dedup();
		let void = self.context.void_type().fn_type(&[], false);
		for node in nodes {
			if !matches!(self.view(node), View::Declaration { .. }) {
				return Err(self.unsupported(node.index(), "this module value (split build)"));
			}
			let name = format!("fa.init.{module}.{}", node.0);
			let function = self
				.module
				.add_function(&name, void, Some(Linkage::External));
			self.runtime.add_attributes(function, &["nounwind"]);
			self.begin_function(function, Repr::Void, None);
			let mark = self.mark();
			self.statement(node)?;
			self.end_statement(mark, None)?;
			let kept = self.cleanups.take();
			if !self.terminated() {
				self.builder.build_return(None)?;
			}
			self.end_function()?;
			self.split_mut().exports.insert(name);
			self.split_mut()
				.symbols
				.inits
				.push((module.clone(), node.0));
			// its values, dropped at a normal exit
			let mut slots = Vec::new();
			for entry in kept {
				match entry {
					Cleanup::Local(slot) if slot.pointer.is_const() => slots.push(slot),
					_ => {
						return Err(
							self.unsupported(node.index(), "this module value (split build)")
						);
					}
				}
			}
			if slots.is_empty() {
				continue;
			}
			let name = format!("fa.drop.{module}.{}", node.0);
			let function = self
				.module
				.add_function(&name, void, Some(Linkage::External));
			self.runtime.add_attributes(function, &["nounwind"]);
			self.begin_function(function, Repr::Void, None);
			self.cleanups
				.borrow_mut()
				.extend(slots.into_iter().map(Cleanup::Local));
			self.drop_to(0)?;
			self.cleanups.borrow_mut().clear();
			if !self.terminated() {
				self.builder.build_return(None)?;
			}
			self.end_function()?;
			self.split_mut().exports.insert(name);
			self.split_mut()
				.symbols
				.drops
				.push((module.clone(), node.0));
		}
		Ok(())
	}

	/// `fa.main.<module>`: the top level of the entry file and the call of
	/// its entry function; it drops what the top level owns and returns
	/// the exit code (0 without an `Integer` entry function).
	fn lower_entry_body(&mut self) -> Result<(), Error> {
		let module = self.session.module(self.file).to_string();
		let name = format!("fa.main.{module}");
		let i32 = self.context.i32_type();
		let function =
			self.module
				.add_function(&name, i32.fn_type(&[], false), Some(Linkage::External));
		self.runtime.add_attributes(function, &["nounwind"]);
		self.begin_function(function, Repr::Void, None);
		let root = NodeId::new(self.tree.root);
		let View::Module { statements } = self.view(root) else {
			return Err(self.unsupported(self.tree.root, "this file"));
		};
		let statements: Vec<NodeId> = statements
			.iter()
			.filter(|&statement| {
				// the module-level values are initialized by `fa.init.*`
				!(self.module_file
					&& matches!(
						self.view(statement),
						View::Declaration { .. } | View::ExportValue { .. }
					))
			})
			.collect();
		self.top_level_statements(&statements)?;
		let mark = self.mark();
		self.call_entry_function()?;
		self.end_statement(mark, None)?;
		self.release_reactivity()?;
		if !self.terminated() {
			if self.leak_check {
				self.drop_to(0)?;
			} else {
				self.drop_at_exit()?;
			}
		}
		if !self.terminated() {
			let code = match self.exit_code {
				Some(slot) => self.builder.build_load(i32, slot, "")?.into_int_value(),
				None => i32.const_zero(),
			};
			self.builder.build_return(Some(&code))?;
		}
		self.end_function()?;
		self.split_mut().exports.insert(name);
		self.split_mut().symbols.entry = Some(module);
		Ok(())
	}

	/// Renames the unit's switches over the ids of union members after the
	/// unit (the entry object's switches call them), and makes the unit's
	/// own uses call the entry object's.
	fn export_box_functions(&mut self) {
		let unit = self.split.as_ref().expect("a unit").name.clone();
		for name in ["fa.drop_box", "fa.copy_box", "fa_format_union"] {
			let Some(function) = self.module.get_function(name) else {
				continue;
			};
			if function.count_basic_blocks() == 0 {
				continue;
			}
			let renamed = format!("{name}.{unit}");
			function.as_global_value().set_name(&renamed);
			let shared =
				self.module
					.add_function(name, function.get_type(), Some(Linkage::External));
			shared
				.as_global_value()
				.set_visibility(GlobalVisibility::Hidden);
			self.runtime.add_attributes(shared, &["nounwind"]);
			function.replace_all_uses_with(shared);
			self.split_mut().exports.insert(renamed);
		}
	}

	/// What other objects use is hidden and external, the rest internal.
	fn finish_linkage(&mut self) -> Result<(), Error> {
		let exports = std::mem::take(&mut self.split_mut().exports);
		let functions: Vec<FunctionValue<'ctx>> = self.module.get_functions().collect();
		for function in functions {
			if function.count_basic_blocks() == 0 {
				if function.get_linkage() != Linkage::Internal {
					continue;
				}
				// a function of the unit declared but not lowered here (a wave):
				// the one its main object defines, or nothing
				let used = inkwell::values::BasicValue::get_first_use(
					&function.as_global_value().as_pointer_value(),
				)
				.is_some();
				let name = function.get_name().to_string_lossy().into_owned();
				if !used {
					// SAFETY: nothing uses it
					unsafe { function.delete() };
				} else if name.starts_with("fa.")
					&& self.split.as_ref().is_some_and(|split| split.wave)
				{
					function.set_linkage(Linkage::External);
					function
						.as_global_value()
						.set_visibility(GlobalVisibility::Hidden);
				} else {
					return Err(Error::Llvm(format!("`{name}` is declared but not defined")));
				}
				continue;
			}
			let name = function.get_name().to_string_lossy();
			if exports.contains(name.as_ref()) {
				function.set_linkage(Linkage::External);
				function
					.as_global_value()
					.set_visibility(GlobalVisibility::Hidden);
			} else {
				function.set_linkage(Linkage::Internal);
			}
		}
		for global in self.module.get_globals() {
			if global.is_declaration() {
				continue;
			}
			let name = global.get_name().to_string_lossy();
			if exports.contains(name.as_ref()) {
				global.set_linkage(Linkage::External);
				global.set_visibility(GlobalVisibility::Hidden);
			} else if global.get_linkage() == Linkage::External {
				global.set_linkage(Linkage::Internal);
			}
		}
		Ok(())
	}
}
