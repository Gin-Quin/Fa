//! Generic function values (phase 2 of "Generic functions carry their list
//! on the function value", `design/decisions.md`; the design is in
//! `design/llvm-backend-plan.md`, "Generic function values").
//!
//! A value of a generic function type, `<T>(value: T) => T`, is a closure
//! pair like any function value, `{ ptr table, ptr environment }`, with no
//! box: `table` points to a constant array holding one entry per instance
//! of its type that the program uses, in an order fixed for the type, and
//! the environment is the one every instance of the lambda reads. An entry
//! has the indirect call convention of its instance (`indirect_type`).
//!
//! The instances a type needs are known from the whole program: every call
//! of a value of the type (`transform(1)`, `transform<String>("a")`), and
//! every conversion to a function type that isn't generic, adds its type
//! arguments to the type's *class* (the generic function types equal once
//! their parameters are renamed and their constraints erased: a value of a
//! narrower type is accepted by a wider place). Every *producer* of a value
//! of the class (a generic lambda, under the bindings of the instance around
//! it, or a generic function declaration used as a value) gets a table with
//! an entry for each of them; an instance its constraints don't allow is a
//! trap, never called (no value of that producer reaches such a call). The
//! tables are filled at the end of the lowering, until no entry asks for
//! another instance; a class that keeps growing (a polymorphic recursion,
//! `f` called at `Array<T>` from its instance at `T`) is an error naming it.
//!
//! A program compiled per unit (design/decisions.md, "Generic function
//! tables per unit") reports its producers and the instances it calls to
//! the driver: the tables (`fa.gv.table.<module>.<node>[.<bindings>]`) and
//! the indices (`fa.gv.index.<class>.<instance>`) are external hidden
//! symbols; once the instance waves are over, the waves of the producers'
//! modules define their entries (`fa.gv.entry.…`), and the entry object
//! defines the tables and the indices (`units.rs`).

use fa_analyzer::ids::{FileId, NodeId, ParamId, TypeId};
use fa_analyzer::types::{Bindings, TypeKind, TypeStore};
use inkwell::module::Linkage;
use inkwell::values::{GlobalValue, PointerValue};
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};

/// The most instances a generic function type may have: more is a
/// polymorphic recursion (its instances keep growing).
pub(crate) const MOST_INSTANCES: usize = 64;

/// The generic function types equal up to the names of their parameters.
pub(crate) struct GenericClass {
	/// The first type of the class met (global ids).
	representative: TypeId,
	/// Its own parameters (global ids).
	generics: Vec<ParamId>,
	/// The instances, in table order: the type arguments' representations
	/// and types.
	instances: Vec<(Vec<Repr>, Vec<TypeId>)>,
}

/// What makes values of a class.
#[derive(Clone)]
enum ProducerKind {
	/// A generic lambda, under the bindings of the instance around it.
	Lambda,
	/// A generic function declaration used as a value.
	Declaration,
}

/// A maker of values of a class, and its table.
pub(crate) struct Producer<'ctx> {
	class: usize,
	file: FileId,
	node: NodeId,
	kind: ProducerKind,
	bindings: FxHashMap<ParamId, Repr>,
	/// Stands for the table until it is defined (`finish_generic_tables`).
	placeholder: GlobalValue<'ctx>,
	entries: Vec<PointerValue<'ctx>>,
}

/// The classes and the producers of the program.
#[derive(Default)]
pub(crate) struct GenericValues<'ctx> {
	classes: Vec<GenericClass>,
	producers: Vec<Producer<'ctx>>,
	keys: FxHashMap<(FileId, NodeId, Vec<Repr>), usize>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The parameters of a generic function type (its own), empty for any
	/// other type.
	pub(crate) fn own_generics(&self, ty: TypeId) -> Vec<ParamId> {
		let types = self.types();
		let TypeKind::Function(sig) = types.kind(ty) else {
			return Vec::new();
		};
		types
			.list_items(types.sig(sig).generics)
			.iter()
			.filter_map(|&generic| match types.kind(generic) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect()
	}

	/// Whether a type is a generic function type.
	pub(crate) fn is_generic_function_type(&self, ty: TypeId) -> bool {
		!self.own_generics(ty).is_empty()
	}

	/// A generic function type with its own parameters bound to `arguments`
	/// (both concrete): the type of an instance.
	fn instantiate_generic(&self, generic: TypeId, arguments: &[TypeId]) -> TypeId {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		let global = merger.ty(generic);
		let arguments: Vec<TypeId> = arguments.iter().map(|&ty| merger.ty(ty)).collect();
		drop(merger);
		let mut scratch = TypeStore::view(self.types().global());
		let own = scratch_generics(&scratch, global);
		let bindings = Bindings {
			params: own,
			types: arguments,
		};
		let instance = scratch.substitute(global, &bindings);
		let mut merger = scratch.merger(Some(module));
		merger.ty(instance)
	}

	/// A type with parameters bound to `arguments` (concrete): the type of an
	/// instance of a generic function declaration (`identity` given to `map`).
	pub(crate) fn substitute_params(
		&self,
		ty: TypeId,
		params: &[ParamId],
		arguments: &[TypeId],
	) -> TypeId {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		let global = merger.ty(ty);
		let params: Vec<ParamId> = params.iter().map(|&param| merger.param(param)).collect();
		let arguments: Vec<TypeId> = arguments.iter().map(|&ty| merger.ty(ty)).collect();
		drop(merger);
		let mut scratch = TypeStore::view(self.types().global());
		let instance = scratch.substitute(
			global,
			&Bindings {
				params,
				types: arguments,
			},
		);
		let mut merger = scratch.merger(Some(module));
		merger.ty(instance)
	}

	/// The class of a (concrete) generic function type.
	fn generic_class(&mut self, ty: TypeId) -> usize {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		let global = merger.ty(ty);
		drop(merger);
		let mut scratch = TypeStore::view(self.types().global());
		let own = scratch_generics(&scratch, global);
		for (index, class) in self.generic_values.classes.iter().enumerate() {
			if class.generics.len() != own.len() {
				continue;
			}
			if class.representative == global {
				return index;
			}
			// renamed to the representative's parameters, both opened
			let theirs: Vec<TypeId> = class
				.generics
				.iter()
				.map(|&param| scratch.intern(TypeKind::Param(param)))
				.collect();
			let renamed = scratch.substitute(
				global,
				&Bindings {
					params: own.clone(),
					types: theirs.clone(),
				},
			);
			let opened = scratch.substitute(
				class.representative,
				&Bindings {
					params: class.generics.clone(),
					types: theirs,
				},
			);
			if renamed == opened {
				return index;
			}
		}
		self.generic_values.classes.push(GenericClass {
			representative: global,
			generics: own,
			instances: Vec::new(),
		});
		self.generic_values.classes.len() - 1
	}

	/// The index of an instance of a class (added when new).
	fn class_instance(
		&mut self,
		class: usize,
		reprs: Vec<Repr>,
		types: Vec<TypeId>,
		node: NodeId,
	) -> Result<usize, Error> {
		let site = self.location(node.index());
		let data = &self.generic_values.classes[class];
		if let Some(index) = data.instances.iter().position(|(known, _)| *known == reprs) {
			return Ok(index);
		}
		if data.instances.len() >= MOST_INSTANCES {
			// the cycle: the call that keeps asking for a new instance, and
			// the first types it asked for
			let representative = data.representative;
			let types = self.types();
			let growing: Vec<String> = data
				.instances
				.iter()
				.take(3)
				.map(|(_, arguments)| {
					arguments
						.iter()
						.map(|&ty| types.display(ty))
						.collect::<Vec<_>>()
						.join(", ")
				})
				.collect();
			return Err(Error::Diagnostics(format!(
				"{site}: error: the generic function type `{}` is called here at a new type in each of its instances (`{}`, …): a polymorphic recursion has no end of instances to compile; call it at types that don't grow",
				self.types().display(representative),
				growing.join("`, `")
			)));
		}
		let data = &mut self.generic_values.classes[class];
		data.instances.push((reprs, types));
		Ok(data.instances.len() - 1)
	}

	/// The producer of a class for a node of the file being lowered, under
	/// the current bindings (a lambda) or none (a declaration).
	fn producer(&mut self, class: usize, node: NodeId, kind: ProducerKind) -> usize {
		let bindings = match kind {
			ProducerKind::Lambda => self.bindings.borrow().clone(),
			ProducerKind::Declaration => FxHashMap::default(),
		};
		let mut entries: Vec<(ParamId, Repr)> = bindings
			.iter()
			.map(|(&param, &repr)| (param, repr))
			.collect();
		entries.sort_by_key(|(param, _)| *param);
		let key = (
			self.file,
			node,
			entries.into_iter().map(|(_, repr)| repr).collect(),
		);
		if let Some(&producer) = self.generic_values.keys.get(&key) {
			return producer;
		}
		let index = self.generic_values.producers.len();
		let placeholder = self.module.add_global(
			self.context.i8_type(),
			None,
			&format!("fa.generic_table.{index}"),
		);
		self.generic_values.producers.push(Producer {
			class,
			file: self.file,
			node,
			kind,
			bindings,
			placeholder,
			entries: Vec::new(),
		});
		self.generic_values.keys.insert(key, index);
		index
	}

	/// A type of this unit's view in the global store (shared by the units of
	/// a build).
	fn global_type(&self, ty: TypeId) -> TypeId {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		merger.ty(ty)
	}

	/// The key of the class of a (concrete) generic function type, the same
	/// in every unit of a build: the stable hash of the type with its own
	/// parameters replaced by markers, in order (their names and
	/// constraints left out, as `generic_class` compares them).
	fn class_key(&self, ty: TypeId) -> String {
		let global = self.global_type(ty);
		let mut scratch = TypeStore::view(self.types().global());
		let own = scratch_generics(&scratch, global);
		let markers: Vec<TypeId> = (0..own.len())
			.map(|index| scratch.string_literal(&format!("\u{1}fa.generic.{index}")))
			.collect();
		let marked = scratch.substitute(
			global,
			&Bindings {
				params: own,
				types: markers,
			},
		);
		format!("{:x}", scratch.stable_hash(marked))
	}

	/// The key of an instance: the stable hashes of its type arguments.
	fn instance_key(&self, arguments: &[TypeId]) -> String {
		let hashes: Vec<String> = arguments
			.iter()
			.map(|&argument| format!("{:x}", self.types().stable_hash(argument)))
			.collect();
		hashes.join("_")
	}

	/// The table of the values a node makes: a placeholder this module
	/// defines at the end, or, compiled per unit, the table the entry
	/// object defines once every instance of the class is known (the
	/// producer reported to the driver).
	fn generic_table_of(
		&mut self,
		ty: TypeId,
		node: NodeId,
		kind: ProducerKind,
	) -> Result<PointerValue<'ctx>, Error> {
		if !self.is_split() {
			let class = self.generic_class(ty);
			let producer = self.producer(class, node, kind);
			return Ok(self.generic_values.producers[producer]
				.placeholder
				.as_pointer_value());
		}
		// the bindings of the instance around a lambda, by the position of
		// each parameter in its owner's list (what another session can
		// find again)
		let mut bindings = Vec::new();
		if matches!(kind, ProducerKind::Lambda) {
			let current: Vec<(ParamId, Repr)> = self
				.bindings
				.borrow()
				.iter()
				.map(|(&param, &repr)| (param, repr))
				.collect();
			for (param, repr) in current {
				let owner = self
					.generic_functions
					.iter()
					.filter_map(|(owner, info)| {
						info.generics
							.iter()
							.position(|&generic| generic == param)
							.map(|index| (owner.index() as u32, index as u32))
					})
					.min();
				let bound = self.instance_types.borrow().get(&(param, repr)).copied();
				let (Some((owner, index)), Some(bound)) = (owner, bound) else {
					return Err(self.unsupported(
						node.index(),
						"a generic lambda in this instance (split build)",
					));
				};
				bindings.push((owner, index, self.global_type(bound)));
			}
			bindings.sort_by_key(|&(owner, index, _)| (owner, index));
		}
		let module = self.session.module(self.file).to_string();
		let mut table = format!("fa.gv.table.{module}.{}", node.index());
		if !bindings.is_empty() {
			let types: Vec<TypeId> = bindings.iter().map(|&(_, _, ty)| ty).collect();
			table.push('.');
			table.push_str(&self.instance_key(&types));
		}
		let global = match self.module.get_global(&table) {
			Some(global) => global,
			None => {
				let global = self.module.add_global(self.context.i8_type(), None, &table);
				global.set_linkage(Linkage::External);
				global.set_constant(true);
				global.set_visibility(inkwell::GlobalVisibility::Hidden);
				global
			}
		};
		let producer = crate::split::GenericProducer {
			table,
			class: self.class_key(ty),
			module,
			node: node.index() as u32,
			lambda: matches!(kind, ProducerKind::Lambda),
			bindings,
		};
		let needs = self.generic_needs_mut();
		if !needs.producers.contains(&producer) {
			needs.producers.push(producer);
		}
		Ok(global.as_pointer_value())
	}

	/// The index of an instance in the tables of its class: a constant, or,
	/// compiled per unit, the constant the entry object defines once every
	/// instance of the class is known (`fa.gv.index.<class>.<instance>`,
	/// folded by the link's optimization).
	fn instance_index(
		&mut self,
		generic: TypeId,
		reprs: Vec<Repr>,
		arguments: &[TypeId],
		node: NodeId,
	) -> Result<inkwell::values::IntValue<'ctx>, Error> {
		if !self.is_split() {
			let class = self.generic_class(generic);
			let index = self.class_instance(class, reprs, arguments.to_vec(), node)?;
			return Ok(self.usize.const_int(index as u64, false));
		}
		let arguments: Vec<TypeId> = arguments
			.iter()
			.map(|&argument| self.global_type(argument))
			.collect();
		let instance = crate::split::GenericInstance {
			class: self.class_key(generic),
			key: self.instance_key(&arguments),
			arguments,
		};
		let symbol = instance.index_symbol();
		let global = match self.module.get_global(&symbol) {
			Some(global) => global,
			None => {
				let global = self.module.add_global(self.usize, None, &symbol);
				global.set_linkage(Linkage::External);
				global.set_constant(true);
				global.set_visibility(inkwell::GlobalVisibility::Hidden);
				global
			}
		};
		let needs = self.generic_needs_mut();
		if !needs.instances.contains(&instance) {
			needs.instances.push(instance);
		}
		Ok(self
			.builder
			.build_load(self.usize, global.as_pointer_value(), "")?
			.into_int_value())
	}

	/// Defines the entry of a producer's table for an instance of its class
	/// (an instance wave of the producer's module), exported for the table
	/// the entry object defines.
	pub(crate) fn define_generic_entry(
		&mut self,
		request: &crate::split::GenericEntryRequest,
		files: &[FileId],
	) -> Result<(), Error> {
		let producer = &request.producer;
		let unsupported = |message: &str| Error::Unsupported {
			message: message.to_string(),
			location: String::new(),
		};
		let Some(&file) = files
			.iter()
			.find(|&&file| self.session.module(file) == producer.module)
		else {
			return Err(unsupported(
				"a generic function value of another unit (split build)",
			));
		};
		let current = self.file;
		self.switch_file(file);
		let result = self.define_generic_entry_in_file(request);
		self.switch_file(current);
		result
	}

	fn define_generic_entry_in_file(
		&mut self,
		request: &crate::split::GenericEntryRequest,
	) -> Result<(), Error> {
		let producer = &request.producer;
		let node = NodeId::new(producer.node as usize);
		// the bindings of the instance around a lambda
		let mut bindings = FxHashMap::default();
		for &(owner, index, bound) in &producer.bindings {
			let Some(param) = self
				.generic_functions
				.get(&NodeId::new(owner as usize))
				.and_then(|info| info.generics.get(index as usize).copied())
			else {
				return Err(self.unsupported(node.index(), "this generic lambda (split build)"));
			};
			let repr = self.repr_of(bound, node.index())?;
			self.note_instance_types(&[param], &[bound], &[repr]);
			bindings.insert(param, repr);
		}
		let mut reprs = Vec::new();
		for &argument in &request.instance.arguments {
			reprs.push(self.repr_of(argument, node.index())?);
		}
		let kind = if producer.lambda {
			ProducerKind::Lambda
		} else {
			ProducerKind::Declaration
		};
		let saved = self.bindings.replace(bindings);
		let entry = self.generic_entry_in_file(node, kind, &reprs, &request.instance.arguments);
		self.bindings.replace(saved);
		let entry = entry?;
		let name = producer.entry_symbol(&request.instance);
		self.export_entry(entry, &name)?;
		self.generic_needs_mut().entries.push(name);
		Ok(())
	}

	/// Exports the function at `entry` as `name`: renamed when no other name
	/// is needed, else through a function of its own that calls it.
	fn export_entry(&mut self, entry: PointerValue<'ctx>, name: &str) -> Result<(), Error> {
		let target_name = entry.get_name().to_string_lossy().into_owned();
		let Some(target) = self.module.get_function(&target_name) else {
			return Err(Error::Llvm(format!(
				"the entry `{target_name}` is no function"
			)));
		};
		let shared = target_name.starts_with("fa.gv.entry.")
			|| target_name == "fa.generic_instance_never_called"
			|| self.is_exported(&target_name);
		if !shared {
			target.as_global_value().set_name(name);
			self.export_symbol(name.to_string());
			return Ok(());
		}
		let ty = target.get_type();
		let function = self.module.add_function(name, ty, None);
		function.set_call_conventions(target.get_call_conventions());
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let arguments: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = function
			.get_param_iter()
			.map(|param| param.into())
			.collect();
		let call = builder.build_call(target, &arguments, "")?;
		call.set_call_convention(target.get_call_conventions());
		call.set_tail_call(true);
		match call.try_as_basic_value().basic() {
			Some(value) => builder.build_return(Some(&value))?,
			None => builder.build_return(None)?,
		};
		self.export_symbol(name.to_string());
		Ok(())
	}

	/// A generic lambda evaluated, `<T>(value: T) => value`: its table and
	/// a new environment.
	pub(crate) fn generic_lambda_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let ty = self.concrete_type(self.type_of(node));
		let table = self.generic_table_of(ty, node, ProducerKind::Lambda)?;
		let captures = self.find_captures(node);
		// the environment of every instance (`declare_function` decides the same)
		let given_to_call = {
			let parent = self.parents().get(node.index()).copied();
			parent.is_some_and(|parent| {
				parent != usize::MAX
					&& matches!(
						self.view(NodeId::new(parent)),
						fa_analyzer::syntax::View::Call { .. }
					)
			})
		};
		let by_reference = !self.analysis.escapes.contains(&node) && given_to_call;
		let environment = self.build_environment(&captures, by_reference, node)?;
		let closure = self.closure_pair(table, environment, self.type_of(node))?;
		Ok(if by_reference {
			closure
		} else {
			self.fresh(closure)
		})
	}

	/// A generic function declaration used as a generic function value: its
	/// table, with no environment.
	pub(crate) fn generic_declaration_value(
		&mut self,
		declaration: NodeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let ty = self.concrete_type(self.type_of(node));
		if !self.is_generic_function_type(ty) {
			return Err(self.unsupported(node.index(), "this generic function value"));
		}
		let table = self.generic_table_of(ty, declaration, ProducerKind::Declaration)?;
		self.closure_pair(table, self.pointer_type().const_null(), self.type_of(node))
	}

	/// `{ table, environment }` as a value of type `ty`.
	fn closure_pair(
		&self,
		table: PointerValue<'ctx>,
		environment: PointerValue<'ctx>,
		ty: TypeId,
	) -> Result<Val<'ctx>, Error> {
		let closure = self.closure_type();
		let pair = self
			.builder
			.build_insert_value(closure.get_undef(), table, 0, "")?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		Ok(Val::new(Repr::Function(ty), pair.into_struct_value()))
	}

	/// The instance of a generic function value for the type arguments
	/// recorded at `node` (a call, a `f<Integer>`, or a value given where a
	/// function type that isn't generic is expected): a function value of
	/// the instance's type sharing the generic value's environment (a view
	/// of it, never registered as a temporary: storing it copies it).
	pub(crate) fn generic_instance_value(
		&mut self,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Function(ty) = value.repr else {
			return Ok(value);
		};
		let generic = self.concrete_type(ty);
		if !self.is_generic_function_type(generic) {
			return Ok(value);
		}
		let Some(list) = self.analysis.instantiation(node) else {
			return Err(self.unsupported(node.index(), "this instance of a generic function"));
		};
		let arguments: Vec<TypeId> = self
			.types()
			.list_items(list)
			.iter()
			.map(|&argument| self.concrete_type(argument))
			.collect();
		// the arguments of the value's own parameters: the last ones (a
		// callee's free parameters come first, `check/calls.rs`)
		let count = self.own_generics(generic).len();
		if arguments.len() < count {
			return Err(self.unsupported(node.index(), "this instance of a generic function"));
		}
		let arguments = arguments[arguments.len() - count..].to_vec();
		let mut reprs = Vec::with_capacity(count);
		for &argument in &arguments {
			reprs.push(self.repr_of(argument, node.index())?);
		}
		let index = self.instance_index(generic, reprs, &arguments, node)?;
		let instance = self.instantiate_generic(generic, &arguments);
		let Some(raw) = value.value else {
			return Ok(value);
		};
		let pair = raw.into_struct_value();
		let table = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let environment = self
			.builder
			.build_extract_value(pair, 1, "")?
			.into_pointer_value();
		let slot = unsafe {
			self.builder
				.build_in_bounds_gep(self.pointer_type(), table, &[index], "")?
		};
		let entry = self
			.builder
			.build_load(self.pointer_type(), slot, "")?
			.into_pointer_value();
		self.closure_pair(entry, environment, instance)
	}

	/// Makes the entries of the tables the classes need so far: whether one
	/// was made (it may need instances to lower, and other entries).
	pub(crate) fn fill_generic_tables(&mut self) -> Result<bool, Error> {
		let mut made = false;
		for producer in 0..self.generic_values.producers.len() {
			loop {
				let (class, done) = {
					let data = &self.generic_values.producers[producer];
					(data.class, data.entries.len())
				};
				let Some((reprs, types)) = self.generic_values.classes[class]
					.instances
					.get(done)
					.cloned()
				else {
					break;
				};
				let entry = self.generic_entry(producer, &reprs, &types)?;
				self.generic_values.producers[producer].entries.push(entry);
				made = true;
			}
		}
		Ok(made)
	}

	/// The entry of a producer's table for an instance.
	fn generic_entry(
		&mut self,
		producer: usize,
		reprs: &[Repr],
		types: &[TypeId],
	) -> Result<PointerValue<'ctx>, Error> {
		let (file, node, kind, bindings) = {
			let data = &self.generic_values.producers[producer];
			(
				data.file,
				data.node,
				data.kind.clone(),
				data.bindings.clone(),
			)
		};
		let current = self.file;
		self.switch_file(file);
		let saved = self.bindings.replace(bindings);
		let entry = self.generic_entry_in_file(node, kind, reprs, types);
		self.bindings.replace(saved);
		self.switch_file(current);
		entry
	}

	fn generic_entry_in_file(
		&mut self,
		node: NodeId,
		kind: ProducerKind,
		reprs: &[Repr],
		types: &[TypeId],
	) -> Result<PointerValue<'ctx>, Error> {
		let generics = match self.generic_functions.get(&node) {
			Some(info) => info.generics.clone(),
			None => return Err(self.unsupported(node.index(), "this generic function value")),
		};
		if !self.instance_allowed(&generics, types) {
			return Ok(self.never_called_entry());
		}
		self.note_instance_types(&generics, types, reprs);
		let key = match kind {
			ProducerKind::Lambda => self.lambda_instance(node, reprs)?,
			ProducerKind::Declaration => self.instance_of(node, reprs.to_vec())?,
		};
		let function = self.functions[&key].clone();
		Ok(match kind {
			ProducerKind::Lambda => function.value.as_global_value().as_pointer_value(),
			ProducerKind::Declaration => self
				.thunk_of(&function)?
				.as_global_value()
				.as_pointer_value(),
		})
	}

	/// Whether type arguments satisfy the constraints of parameters.
	fn instance_allowed(&self, generics: &[ParamId], types: &[TypeId]) -> bool {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		let mut pairs = Vec::new();
		for (&param, &ty) in generics.iter().zip(types) {
			let constraint = self.types().param(param).constraint;
			if constraint.is_unset() || constraint == TypeId::UNKNOWN {
				continue;
			}
			pairs.push((merger.ty(ty), merger.ty(constraint)));
		}
		drop(merger);
		if pairs.is_empty() {
			return true;
		}
		let mut scratch = TypeStore::view(self.types().global());
		pairs
			.into_iter()
			.all(|(ty, constraint)| scratch.satisfies(ty, constraint))
	}

	/// The entry of an instance a producer never gets called at.
	fn never_called_entry(&self) -> PointerValue<'ctx> {
		let name = "fa.generic_instance_never_called";
		let function = self.module.get_function(name).unwrap_or_else(|| {
			let function = self.module.add_function(
				name,
				self.context.void_type().fn_type(&[], false),
				Some(Linkage::Internal),
			);
			function.set_call_conventions(FAST_CALLING_CONVENTION);
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(function, "entry"));
			let _ = builder.build_unreachable();
			function
		});
		function.as_global_value().as_pointer_value()
	}

	/// Defines the tables, once every entry is made.
	pub(crate) fn finish_generic_tables(&mut self) -> Result<(), Error> {
		let pointer = self.pointer_type();
		for producer in std::mem::take(&mut self.generic_values.producers) {
			let array = pointer.const_array(&producer.entries);
			let name = producer
				.placeholder
				.get_name()
				.to_string_lossy()
				.to_string();
			let table = self.module.add_global(
				pointer.array_type(producer.entries.len() as u32),
				None,
				&format!("{name}.entries"),
			);
			table.set_initializer(&array);
			table.set_constant(true);
			table.set_linkage(Linkage::Internal);
			producer
				.placeholder
				.as_pointer_value()
				.replace_all_uses_with(table.as_pointer_value());
			unsafe {
				producer.placeholder.delete();
			}
		}
		Ok(())
	}
}

/// The own parameters of a generic function type of a store.
fn scratch_generics(store: &TypeStore, ty: TypeId) -> Vec<ParamId> {
	let TypeKind::Function(sig) = store.kind(ty) else {
		return Vec::new();
	};
	store
		.list_items(store.sig(sig).generics)
		.iter()
		.filter_map(|&generic| match store.kind(generic) {
			TypeKind::Param(param) => Some(param),
			_ => None,
		})
		.collect()
}
