//! Programs of several files (advanced brief §15.7): every file of the
//! `Program` is lowered into the one LLVM module.
//!
//! - The lowering holds the fields of the file being lowered; the other
//!   files' are kept in a [`FileState`] each, swapped in by
//!   [`Lowering::switch_file`] (node, symbol and scope ids are per file).
//! - The module-level values of a module file and the members of namespaces
//!   are internal globals. `main` initializes the values of the modules in
//!   `Program::init_order`, runs the top level of the entry file (a
//!   program file), then calls the entry function (the export of the entry
//!   file) with the arguments; its `Integer` result is the exit code. The
//!   values are dropped at the end, in reverse order.
//! - A name resolved to another file (`Resolution::External`) reads that
//!   file's global, or calls its function directly (with its parameters,
//!   defaults and requirements); a path through folders, files and
//!   namespaces (`library.counter.count`) reaches its last module-level
//!   member, then applies the fields after it.

use fa_analyzer::analysis::{Analysis, CallInfo, FnInfo};
use fa_analyzer::ids::{DeclKey, FileId, NodeId, ScopeId, SymbolId, TypeId};
use fa_analyzer::session::Session;
use fa_analyzer::symbols::{Resolution, ScopeKind, SymbolKind};
use fa_analyzer::syntax::{BindingKind, View};
use fa_parser::TypedSyntaxTree;
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Function, Lowering, Slot};

/// The per-file fields of a [`Lowering`], for the files not being lowered.
pub(crate) struct FileState<'a, 'ctx> {
	pub(crate) tree: &'a TypedSyntaxTree,
	pub(crate) analysis: &'a Analysis,
	pub(crate) path: String,
	pub(crate) lines: Vec<usize>,
	pub(crate) module_file: bool,
	functions: FxHashMap<NodeId, Function<'ctx>>,
	pub(crate) generic_functions: FxHashMap<NodeId, FnInfo>,
	instances: FxHashMap<(NodeId, Vec<Repr>, Vec<TypeId>), NodeId>,
	scope_owners: FxHashMap<ScopeId, Option<NodeId>>,
	shared: FxHashMap<SymbolId, ()>,
	global_slots: FxHashMap<SymbolId, Slot<'ctx>>,
	slots: FxHashMap<SymbolId, Slot<'ctx>>,
	owned_locals: FxHashMap<SymbolId, Slot<'ctx>>,
	references: FxHashMap<SymbolId, (inkwell::values::PointerValue<'ctx>, Repr)>,
	provided: FxHashMap<NodeId, Slot<'ctx>>,
	symbol_functions: FxHashMap<SymbolId, NodeId>,
	local_uses: std::cell::RefCell<FxHashMap<SymbolId, Vec<NodeId>>>,
	node_parents: std::cell::RefCell<Vec<usize>>,
	referenced: std::cell::RefCell<Option<rustc_hash::FxHashSet<SymbolId>>>,
	loop_bodies: std::cell::OnceCell<Vec<fa_parser::SourceSpan>>,
	reactive_sources: FxHashMap<SymbolId, inkwell::values::PointerValue<'ctx>>,
	derived_nodes: FxHashMap<SymbolId, inkwell::values::PointerValue<'ctx>>,
}

impl<'a, 'ctx> FileState<'a, 'ctx> {
	/// The state of a file not lowered yet.
	pub(crate) fn new(session: &'a Session, file: FileId, path: &str) -> Self {
		let tree = session.tree(file);
		let lines = std::iter::once(0)
			.chain(
				tree.source
					.bytes()
					.enumerate()
					.filter(|(_, byte)| *byte == b'\n')
					.map(|(index, _)| index + 1),
			)
			.collect();
		FileState {
			tree,
			analysis: session.analysis(file),
			path: path.to_string(),
			lines,
			module_file: session.role(file).is_module(),
			functions: FxHashMap::default(),
			generic_functions: FxHashMap::default(),
			instances: FxHashMap::default(),
			scope_owners: FxHashMap::default(),
			shared: FxHashMap::default(),
			global_slots: FxHashMap::default(),
			slots: FxHashMap::default(),
			owned_locals: FxHashMap::default(),
			references: FxHashMap::default(),
			provided: FxHashMap::default(),
			symbol_functions: FxHashMap::default(),
			local_uses: std::cell::RefCell::new(FxHashMap::default()),
			node_parents: std::cell::RefCell::new(Vec::new()),
			referenced: std::cell::RefCell::new(None),
			loop_bodies: std::cell::OnceCell::new(),
			reactive_sources: FxHashMap::default(),
			derived_nodes: FxHashMap::default(),
		}
	}

	/// Whether the file declares functions.
	pub(crate) fn has_functions(&self) -> bool {
		!self.functions.is_empty() || !self.generic_functions.is_empty()
	}
}

/// What a name of another file (or a namespace path) designates.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Reached {
	/// A function: its file and its key in that file's functions.
	Function(FileId, NodeId),
	/// A module-level value (a global): its file and symbol.
	Value(FileId, SymbolId),
	/// A function of another unit of a program compiled per unit (`units.rs`).
	RemoteFunction(DeclKey),
	/// A module-level value of another unit.
	RemoteValue(DeclKey),
}

impl<'a, 'ctx> Lowering<'a, 'ctx> {
	/// The lowered `destroy` of a resource type declared in another file of
	/// the program (`foreign.rs`, `destroy_function`).
	pub(crate) fn destroy_in_other_files(
		&self,
		ty: TypeId,
	) -> Option<inkwell::values::FunctionValue<'ctx>> {
		self.other_files.values().find_map(|state| {
			let declaration = state
				.analysis
				.type_decls
				.values()
				.find(|decl| decl.ty == ty)?;
			declaration
				.functions
				.iter()
				.map(|&function| state.analysis.symbols.symbol(function))
				.find(|symbol| self.session.types().syms.resolve(symbol.name) == "destroy")
				.and_then(|symbol| state.functions.get(&symbol.decl_node))
				.map(|function| function.value)
		})
	}

	/// Makes `file` the file being lowered.
	pub(crate) fn switch_file(&mut self, file: FileId) {
		if file == self.file {
			return;
		}
		let Some(mut state) = self.other_files.remove(&file) else {
			return;
		};
		macro_rules! swap {
			($($field:ident),*) => {
				$(std::mem::swap(&mut self.$field, &mut state.$field);)*
			};
		}
		swap!(
			tree,
			analysis,
			path,
			lines,
			module_file,
			functions,
			generic_functions,
			instances,
			scope_owners,
			shared,
			global_slots,
			slots,
			owned_locals,
			references,
			provided,
			symbol_functions,
			local_uses,
			node_parents,
			referenced,
			loop_bodies,
			reactive_sources,
			derived_nodes
		);
		let previous = std::mem::replace(&mut self.file, file);
		self.other_files.insert(previous, state);
	}

	/// The values that live as long as the program: the module-level values
	/// of a module file, the members of namespaces (globals).
	pub(crate) fn mark_module_values(&mut self) {
		let symbols = &self.analysis.symbols;
		for (index, symbol) in symbols.symbols().iter().enumerate() {
			if !matches!(symbol.kind, SymbolKind::Let | SymbolKind::Mutable) {
				continue;
			}
			let kind = symbols.scope(symbol.scope).kind;
			if (kind == ScopeKind::Module && self.module_file) || kind == ScopeKind::Namespace {
				self.shared.insert(SymbolId::new(index), ());
			}
		}
	}

	/// What a resolution designates in another file: a function or a value
	/// (`None` for a namespace, a folder, a type).
	pub(crate) fn external(&self, resolution: Resolution) -> Option<Reached> {
		// another unit, read through its interface
		if self.is_split()
			&& let Resolution::External(key) = resolution
			&& !self.is_local_key(key)
		{
			return match self.remote(key)? {
				(key, true) => Some(Reached::RemoteFunction(key)),
				(key, false) => Some(Reached::RemoteValue(key)),
			};
		}
		let (file, symbol) = self.session.external(resolution)?;
		self.reached_in(file, symbol)
	}

	/// A function or a value of a file.
	fn reached_in(&self, file: FileId, symbol: SymbolId) -> Option<Reached> {
		let (analysis, functions) = if file == self.file {
			(self.analysis, &self.symbol_functions)
		} else {
			let state = self.other_files.get(&file)?;
			(state.analysis, &state.symbol_functions)
		};
		match analysis.symbols.symbol(symbol).kind {
			SymbolKind::Function => Some(Reached::Function(file, *functions.get(&symbol)?)),
			SymbolKind::Let | SymbolKind::Mutable => Some(Reached::Value(file, symbol)),
			_ => None,
		}
	}

	/// Whether a node names a namespace: a folder, a namespace of this file,
	/// another file whose export is a namespace (or a type, whose functions
	/// follow it).
	fn is_namespace_node(&self, node: NodeId) -> bool {
		match self.analysis.resolution(node) {
			Resolution::Folder(_) => true,
			Resolution::Local(symbol) => matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Namespace | SymbolKind::Type
			),
			resolution @ Resolution::External(..) => self.external(resolution).is_none(),
			_ => false,
		}
	}

	/// The module-level member a path through namespaces reaches
	/// (`library.counter.count`, `counter.increment`), and the number of
	/// members used; `None` when the target is no namespace.
	pub(crate) fn namespace_reach(
		&self,
		target: NodeId,
		members: &[NodeId],
	) -> Option<(Reached, usize)> {
		if !self.is_namespace_node(target) {
			return None;
		}
		for (index, &member) in members.iter().enumerate() {
			match self.analysis.resolution(member) {
				Resolution::Folder(_) => continue,
				resolution @ Resolution::External(..) => match self.external(resolution) {
					Some(reached) => return Some((reached, index + 1)),
					None => continue,
				},
				Resolution::Local(symbol) => match self.reached_in(self.file, symbol) {
					Some(reached) => return Some((reached, index + 1)),
					None => continue,
				},
				_ => return None,
			}
		}
		None
	}

	/// The global of a module-level value of a file.
	pub(crate) fn global_of(
		&mut self,
		file: FileId,
		symbol: SymbolId,
		node: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let current = self.file;
		if file == current {
			return self.slot_of(symbol, node);
		}
		self.switch_file(file);
		let slot = self.slot_of(symbol, node);
		self.switch_file(current);
		slot
	}

	/// The global a value reaches (`None` for a function).
	pub(crate) fn reached_slot(
		&mut self,
		reached: Reached,
		node: NodeId,
	) -> Result<Option<Slot<'ctx>>, Error> {
		match reached {
			Reached::Value(file, symbol) => self.global_of(file, symbol, node).map(Some),
			Reached::RemoteValue(key) => self.remote_global(key, node).map(Some),
			Reached::Function(..) | Reached::RemoteFunction(_) => Ok(None),
		}
	}

	/// A function of a file (an instance of a generic one for the type
	/// arguments of `call`).
	pub(crate) fn function_in(
		&mut self,
		file: FileId,
		key: NodeId,
		call: NodeId,
	) -> Result<Function<'ctx>, Error> {
		let current = self.file;
		if file == current {
			let key = match self.generic_functions.contains_key(&key) {
				true => self.instance(key, call)?,
				false => key,
			};
			return match self.functions.get(&key) {
				Some(function) => Ok(function.clone()),
				None => Err(self.unsupported(call.index(), "calling this function")),
			};
		}
		let generic = self
			.other_files
			.get(&file)
			.is_some_and(|state| state.generic_functions.contains_key(&key));
		if generic {
			let instance = self.instance_in(file, key, call)?;
			self.switch_file(file);
			let function = self.functions[&instance].clone();
			self.switch_file(current);
			return Ok(function);
		}
		match self
			.other_files
			.get(&file)
			.and_then(|state| state.functions.get(&key))
		{
			Some(function) => Ok(function.clone()),
			None => Err(self.unsupported(call.index(), "calling this function")),
		}
	}

	/// The file and the key of a lowered function (`tasks.rs`).
	pub(crate) fn function_key(
		&self,
		value: inkwell::values::FunctionValue<'ctx>,
	) -> Option<(FileId, NodeId)> {
		let find = |functions: &FxHashMap<NodeId, Function<'ctx>>| {
			functions
				.iter()
				.find(|(_, function)| function.value == value)
				.map(|(key, _)| *key)
		};
		if let Some(key) = find(&self.functions) {
			return Some((self.file, key));
		}
		self.other_files
			.iter()
			.find_map(|(file, state)| find(&state.functions).map(|key| (*file, key)))
	}

	/// A name of another file as a value: its global, or its function.
	pub(crate) fn external_name(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let reached = self.external(self.analysis.resolution(node));
		match reached.map(|reached| self.reached_slot(reached, node)) {
			Some(Ok(Some(slot))) => self.load(slot),
			Some(Err(error)) => Err(error),
			_ => Err(self.unsupported(node.index(), "this name of another file")),
		}
	}

	/// `library.counter.count`, `counter.value.field`: the value a path
	/// through namespaces reaches, then the fields after it; `None` when the
	/// target is no namespace.
	pub(crate) fn namespace_access(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: &[NodeId],
	) -> Result<Option<Val<'ctx>>, Error> {
		let Some((reached, used)) = self.namespace_reach(target, members) else {
			return Ok(None);
		};
		let Some(slot) = self.reached_slot(reached, node)? else {
			return Ok(None);
		};
		let mut value = self.load(slot)?;
		let mut current_type = self.type_of(members[used - 1]);
		for &member in &members[used..] {
			if value.repr == Repr::Never {
				return Ok(Some(value));
			}
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		Ok(Some(value))
	}

	/// The place a path through namespaces reaches, then the fields after it.
	pub(crate) fn namespace_place(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: &[NodeId],
	) -> Result<Option<Slot<'ctx>>, Error> {
		let Some((reached, used)) = self.namespace_reach(target, members) else {
			return Ok(None);
		};
		let Some(slot) = self.reached_slot(reached, node)? else {
			return Ok(None);
		};
		let current_type = self.type_of(members[used - 1]);
		let slot = self.place_members(slot, current_type, members[used..].iter().copied())?;
		Ok(Some(slot))
	}

	/// The function a callee designates in another file (or through a
	/// namespace of this one): a name, a path, or the method of a receiver
	/// whose last member resolves to another file's function.
	pub(crate) fn callee_function(&self, callee: NodeId) -> Option<Reached> {
		let reached = match self.view(callee) {
			View::Name(_) => self.external(self.analysis.resolution(callee))?,
			View::Group(inner) => return self.callee_function(inner),
			// `library.firstOf<Integer>`: the applied function (the call reads
			// the instantiation of the application)
			View::Call { callee: inner, .. }
				if self
					.analysis
					.flags_of(callee)
					.contains(fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION) =>
			{
				return self.callee_function(inner);
			}
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				match self.namespace_reach(target, &members) {
					Some((reached, used)) if used == members.len() => reached,
					Some(_) => return None,
					// `value.method()`: a function of a type of another file
					None => self.external(self.analysis.resolution(*members.last()?))?,
				}
			}
			_ => return None,
		};
		match reached {
			Reached::Function(..) | Reached::RemoteFunction(_) => Some(reached),
			Reached::Value(..) | Reached::RemoteValue(_) => None,
		}
	}

	/// Whether a statement only names a module path (`use { a } = library`,
	/// `use x = library.y`): the analyzer resolved its uses, nothing runs.
	pub(crate) fn is_name_alias(&self, node: NodeId) -> bool {
		let value = match self.view(node) {
			View::Use {
				value,
				mutable: false,
				..
			} => value,
			View::Destructuring {
				kind: BindingKind::Use,
				value,
				..
			} => value,
			_ => return false,
		};
		let mut current = value;
		loop {
			match self.view(current) {
				View::Access { target, .. } => current = target,
				View::Group(inner) => current = inner,
				View::Name(_) => break,
				// `use weather = @import("…")`: resolved by the analyzer
				View::Call { .. } => {
					return fa_analyzer::scan::import_source(self.tree, current).is_some();
				}
				_ => return false,
			}
		}
		match self.analysis.resolution(current) {
			// `use { Mutex } = std.concurrency`
			Resolution::Folder(_) | Resolution::External(..) | Resolution::Builtin(_) => true,
			Resolution::Local(symbol) => {
				self.analysis.symbols.symbol(symbol).kind == SymbolKind::Namespace
			}
			_ => false,
		}
	}

	/// The value declarations of a namespace (`{ … }`), recursively, in order.
	pub(crate) fn namespace_values(&self, value: NodeId, out: &mut Vec<NodeId>) {
		let statements: Vec<NodeId> = match self.view(value) {
			View::Block { statements } | View::Module { statements } => statements.iter().collect(),
			View::Object { items } => items.iter().collect(),
			_ => return,
		};
		for statement in statements {
			match self.view(statement) {
				View::Namespace { value, .. } | View::ExportNamespace(value) => {
					self.namespace_values(value, out);
				}
				View::Declaration { .. } => out.push(statement),
				_ => {}
			}
		}
	}

	/// The members of the namespaces of the program file being lowered,
	/// initialized where the namespace is declared.
	pub(crate) fn namespace_statement(&mut self, value: NodeId) -> Result<Val<'ctx>, Error> {
		let mut values = Vec::new();
		self.namespace_values(value, &mut values);
		for value in values {
			let mark = self.mark();
			self.statement(value)?;
			self.end_statement(mark, None)?;
		}
		Ok(Val::void())
	}

	/// Initializes the module-level values of the modules, in the program's
	/// order, from `main`.
	pub(crate) fn initialize_modules(&mut self) -> Result<(), Error> {
		let order = self.init_order.clone();
		let entry = self.file;
		// a destructuring declares several values: it runs once
		let mut done = rustc_hash::FxHashSet::default();
		for (file, decl) in order {
			self.switch_file(file);
			if self.module_file {
				let statement = self.declaring_statement(decl);
				if !done.insert((file, statement)) {
					continue;
				}
				let mark = self.mark();
				self.statement(statement)?;
				self.end_statement(mark, None)?;
			}
		}
		self.switch_file(entry);
		Ok(())
	}

	/// The statement declaring a module value (its declaration node: a
	/// name of a destructuring's pattern is the destructuring's).
	pub(crate) fn declaring_statement(&self, node: NodeId) -> NodeId {
		let mut current = node;
		loop {
			if matches!(
				self.view(current),
				View::Declaration { .. } | View::ExportValue { .. } | View::Destructuring { .. }
			) {
				return current;
			}
			let parent = self.parents().get(current.index()).copied();
			match parent {
				Some(parent) if parent != usize::MAX => current = NodeId::new(parent),
				_ => return node,
			}
		}
	}

	/// Calls the entry function (the export of the entry file) with the
	/// arguments: its `Integer` result is the exit code.
	pub(crate) fn call_entry_function(&mut self) -> Result<(), Error> {
		let Some(export) = self.session.source_file(self.file).interface.export else {
			return Ok(());
		};
		if export.kind != SymbolKind::Function {
			return Ok(());
		}
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(export.decl) else {
			return Ok(());
		};
		let Some(&key) = self.symbol_functions.get(&symbol) else {
			return Ok(());
		};
		let function = self.functions[&key].clone();
		let mut arguments = Vec::new();
		if !function.params.is_empty() {
			let array = self.runtime.function("fa_arguments")?;
			let array = self.call_value(array, &[])?;
			let array = self.fresh(Val::new(Repr::Array(TypeId::STRING), array));
			let (repr, _) = function.params[0];
			let array = self.convert(array, repr, export.decl.index())?;
			arguments.push(array.value.expect("the arguments").into());
		}
		// awaited by the top level of an asynchronous program (tasks.rs)
		let result = self.entry_call(&function, &arguments)?;
		let result = if function.throws {
			let Some(result) = result else {
				return Ok(());
			};
			let (value, error) = if function.ret.has_value() {
				let pair = result.into_struct_value();
				(
					Some(self.builder.build_extract_value(pair, 0, "")?),
					self.builder
						.build_extract_value(pair, 1, "")?
						.into_pointer_value(),
				)
			} else {
				(None, result.into_pointer_value())
			};
			self.raise_if_error(error)?;
			value
		} else {
			result
		};
		if let (Repr::Int(int), Some(result)) = (function.ret, result) {
			let code = self.int_to(
				result.into_int_value(),
				int,
				super::values::IntRepr {
					bits: 32,
					signed: true,
				},
			)?;
			let slot = self.alloca_type(self.context.i32_type().into(), "exit_code")?;
			self.builder.build_store(slot, code)?;
			self.exit_code = Some(slot);
		} else if let Some(result) = result {
			self.fresh(Val::new(function.ret, result));
		}
		Ok(())
	}

	/// A direct call of a function of another file (`Callee::Value` whose
	/// callee designates it), with the callee's parameters and defaults.
	pub(crate) fn external_call(
		&mut self,
		node: NodeId,
		reached: Reached,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let function = match reached {
			Reached::Function(file, key) => self.function_in(file, key, node)?,
			Reached::RemoteFunction(key) => self.remote_function(key, node)?,
			Reached::Value(..) | Reached::RemoteValue(_) => {
				return Err(self.unsupported(node.index(), "calling this value"));
			}
		};
		self.call_function(node, function, info)
	}
}

/// The map types whose values something in the program may keep a reference
/// into (`design/decisions.md`, "Dense maps"): the others store their entries
/// densely (`tables.rs`, `dense_map`). Syntactic and conservative, over every
/// file of the program: a map type is unsafe when a node of that type appears
/// in the value of a `use`, `mutable use`, `reuse` or `provide` (a reference),
/// or in an argument of a call of a function of the program (parameters are
/// references) other than as the whole argument (the map itself), or in a
/// lambda (a callback may run while its caller holds a value).
pub(crate) fn unsafe_maps(
	session: &Session,
	files: &[(FileId, String)],
) -> rustc_hash::FxHashSet<TypeId> {
	use fa_analyzer::analysis::{Argument, Callee};
	use fa_analyzer::syntax::view;
	use fa_analyzer::types::{CollKind, TypeKind};
	let mut unsafe_maps = rustc_hash::FxHashSet::default();
	let types = session.types();
	let is_map = |ty: TypeId| matches!(types.kind(ty), TypeKind::Collection(CollKind::Map, _, _));
	for (file, _) in files {
		let tree = session.tree(*file);
		let analysis = session.analysis(*file);
		let mark = |root: NodeId,
		            skip: Option<NodeId>,
		            unsafe_maps: &mut rustc_hash::FxHashSet<TypeId>| {
			let mut stack = vec![root.index()];
			while let Some(at) = stack.pop() {
				let node = NodeId::new(at);
				let ty = analysis.type_of(node);
				if Some(node) != skip && is_map(ty) {
					unsafe_maps.insert(ty);
				}
				tree.nodes[at].for_each_child(|child| stack.push(child));
			}
		};
		// a function's own body is no closure
		let declared = crate::facts::declaration_lambdas(tree);
		let parents = crate::facts::parents_of(tree);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			match view(tree, node) {
				// a reference that no change of a map can outlive is fine
				View::Use { .. }
					if crate::facts::short_reference(tree, analysis, types, node, &parents) => {}
				View::Use { value, .. }
				| View::Reuse { value, .. }
				| View::Provide { value, .. } => {
					mark(value, None, &mut unsafe_maps);
				}
				View::Lambda { .. } if !declared.contains(&node) => {
					mark(node, None, &mut unsafe_maps)
				}
				View::Call { .. } => {
					let Some(info) = analysis.call(node) else {
						continue;
					};
					if matches!(
						info.callee,
						Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_)
					) {
						continue;
					}
					for argument in &info.arguments {
						if let Argument::Given(value) = argument {
							mark(*value, Some(*value), &mut unsafe_maps);
						}
					}
				}
				_ => {}
			}
		}
	}
	unsafe_maps
}

/// The map types whose entries a `deleteWhere` deletes while it iterates
/// over them: their entries stay in an array (deleting from the probe array
/// shifts entries a running iteration still has to visit).
pub(crate) fn no_inline_maps(
	session: &Session,
	files: &[(FileId, String)],
) -> rustc_hash::FxHashSet<TypeId> {
	use fa_analyzer::analysis::Callee;
	use fa_analyzer::syntax::view;
	let mut maps = rustc_hash::FxHashSet::default();
	for (file, _) in files {
		let tree = session.tree(*file);
		let analysis = session.analysis(*file);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			let View::Call { callee, .. } = view(tree, node) else {
				continue;
			};
			let Some(Callee::Builtin(id)) = analysis.call(node).map(|info| info.callee) else {
				continue;
			};
			if !fa_analyzer::builtins::get(id)
				.path
				.ends_with("#deleteWhere")
			{
				continue;
			}
			if let View::Access { target, .. } = view(tree, callee) {
				maps.insert(analysis.type_of(target));
			}
		}
	}
	maps
}
