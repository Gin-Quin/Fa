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
//! A program compiled per unit compiles a program with generic function
//! values as one module (its tables are a property of the whole program).

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
const MOST_INSTANCES: usize = 64;

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

	/// A program compiled per unit compiles generic function values as one
	/// module.
	fn generic_values_need_one_module(&self, node: NodeId) -> Result<(), Error> {
		if self.is_split() {
			return Err(self.unsupported(
				node.index(),
				"a generic function value in a program compiled per unit (compiled as one module)",
			));
		}
		Ok(())
	}

	/// A generic lambda evaluated, `<T>(value: T) => value`: its table and
	/// a new environment.
	pub(crate) fn generic_lambda_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		self.generic_values_need_one_module(node)?;
		let ty = self.concrete_type(self.type_of(node));
		let class = self.generic_class(ty);
		let producer = self.producer(class, node, ProducerKind::Lambda);
		let table = self.generic_values.producers[producer]
			.placeholder
			.as_pointer_value();
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
		self.generic_values_need_one_module(node)?;
		let ty = self.concrete_type(self.type_of(node));
		if !self.is_generic_function_type(ty) {
			return Err(self.unsupported(node.index(), "this generic function value"));
		}
		let class = self.generic_class(ty);
		let producer = self.producer(class, declaration, ProducerKind::Declaration);
		let table = self.generic_values.producers[producer]
			.placeholder
			.as_pointer_value();
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
		self.generic_values_need_one_module(node)?;
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
		let class = self.generic_class(generic);
		let index = self.class_instance(class, reprs, arguments.clone(), node)?;
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
			self.builder.build_in_bounds_gep(
				self.pointer_type(),
				table,
				&[self.usize.const_int(index as u64, false)],
				"",
			)?
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
