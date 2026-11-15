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

use fa_analyzer::analysis::GenericMethod;
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

type GenericInstance = (Vec<Repr>, Vec<TypeId>, Vec<GenericMethod>);
type ProducerKey = (
	FileId,
	NodeId,
	Vec<(Repr, TypeId)>,
	Vec<(ParamId, GenericMethod)>,
	ProducerKind,
);

/// The generic function types equal up to the names of their parameters.
pub(crate) struct GenericClass {
	/// The first type of the class met (global ids).
	representative: TypeId,
	/// Its own parameters (global ids).
	generics: Vec<ParamId>,
	/// The instances, in table order: the type arguments' representations
	/// and types.
	instances: Vec<GenericInstance>,
}

/// What makes values of a class.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ProducerKind {
	/// A generic lambda, under the bindings of the instance around it.
	Lambda,
	/// A generic function declaration used as a value.
	Declaration,
	/// A `with` closure whose remaining parameters are generic.
	Partial,
	DefaultLambda,
	DefaultDeclaration,
	DefaultPartial,
	/// A signature conversion forwards each class request to its source.
	Adapter {
		from: TypeId,
		to: TypeId,
	},
}

impl ProducerKind {
	fn base(self) -> Self {
		match self {
			Self::DefaultLambda => Self::Lambda,
			Self::DefaultDeclaration => Self::Declaration,
			Self::DefaultPartial => Self::Partial,
			other => other,
		}
	}
	pub(super) fn with_defaults(self) -> Self {
		match self.base() {
			Self::Lambda => Self::DefaultLambda,
			Self::Declaration => Self::DefaultDeclaration,
			Self::Partial => Self::DefaultPartial,
			_ => unreachable!(),
		}
	}
	fn captures_outer(self) -> bool {
		matches!(self.base(), Self::Lambda | Self::Partial)
	}

	fn defaults(self) -> bool {
		matches!(
			self,
			Self::DefaultLambda | Self::DefaultDeclaration | Self::DefaultPartial
		)
	}
}

enum InstanceIndex<'ctx> {
	Constant(inkwell::values::IntValue<'ctx>),
	Global(GlobalValue<'ctx>),
}

/// A maker of values of a class, and its table.
pub(crate) struct Producer<'ctx> {
	class: usize,
	file: FileId,
	node: NodeId,
	kind: ProducerKind,
	bindings: FxHashMap<ParamId, Repr>,
	types: FxHashMap<ParamId, TypeId>,
	methods: Vec<(ParamId, GenericMethod)>,
	values: FxHashMap<fa_analyzer::ids::SymbolId, TypeId>,
	/// Stands for the table until it is defined (`finish_generic_tables`).
	placeholder: GlobalValue<'ctx>,
	entries: Vec<PointerValue<'ctx>>,
}

/// The classes and the producers of the program.
#[derive(Default)]
pub(crate) struct GenericValues<'ctx> {
	classes: Vec<GenericClass>,
	producers: Vec<Producer<'ctx>>,
	keys: FxHashMap<ProducerKey, usize>,
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
	pub(super) fn instantiate_generic(&self, generic: TypeId, arguments: &[TypeId]) -> TypeId {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		let global = merger.ty(generic);
		let arguments: Vec<TypeId> = arguments.iter().map(|&ty| merger.ty(ty)).collect();
		let module = merger.module(module);
		drop(merger);
		let mut scratch = TypeStore::view(self.types().global());
		let own = scratch_generics(&scratch, global);
		let bindings = Bindings {
			params: own,
			types: arguments,
			..Default::default()
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
		let module = merger.module(module);
		drop(merger);
		let mut scratch = TypeStore::view(self.types().global());
		let instance = scratch.substitute(
			global,
			&Bindings {
				params,
				types: arguments,
				..Default::default()
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
					..Default::default()
				},
			);
			let opened = scratch.substitute(
				class.representative,
				&Bindings {
					params: class.generics.clone(),
					types: theirs,
					..Default::default()
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
		methods: Vec<GenericMethod>,
		node: NodeId,
	) -> Result<usize, Error> {
		let site = self.location(node.index());
		let data = &self.generic_values.classes[class];
		if let Some(index) = data.instances.iter().position(|(known, exact, selected)| {
			*known == reprs && *exact == types && *selected == methods
		}) {
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
				.map(|(_, arguments, _)| {
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
		data.instances.push((reprs, types, methods));
		Ok(data.instances.len() - 1)
	}

	/// The producer of a class for a node of the file being lowered, under
	/// the current bindings (a lambda) or none (a declaration).
	fn producer(&mut self, class: usize, node: NodeId, kind: ProducerKind) -> usize {
		let bindings = if !kind.captures_outer() {
			FxHashMap::default()
		} else {
			self.bindings.borrow().clone()
		};
		let mut entries: Vec<(ParamId, Repr)> = bindings
			.iter()
			.map(|(&param, &repr)| (param, repr))
			.collect();
		entries.sort_by_key(|(param, _)| *param);
		let mut methods = if kind.captures_outer() {
			self.method_bindings.borrow().clone()
		} else {
			Vec::new()
		};
		methods.sort_by_key(|(param, method)| (*param, method.name));
		let types = if kind.captures_outer() {
			self.type_bindings.borrow().clone()
		} else {
			FxHashMap::default()
		};
		let key = (
			self.file,
			node,
			entries
				.into_iter()
				.map(|(param, repr)| (repr, types.get(&param).copied().unwrap_or(TypeId::UNKNOWN)))
				.collect(),
			methods.clone(),
			kind,
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
			types,
			methods,
			values: if !kind.captures_outer() {
				FxHashMap::default()
			} else {
				self.value_bindings.borrow().clone()
			},
			placeholder,
			entries: Vec::new(),
		});
		self.generic_values.keys.insert(key, index);
		index
	}

	/// A type of this unit's view in the global store (shared by the units of
	/// a build).
	pub(super) fn global_type(&self, ty: TypeId) -> TypeId {
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		merger.ty(ty)
	}

	/// The key of the class of a (concrete) generic function type, the same
	/// in every unit of a build: the stable hash of the type with its own
	/// parameters replaced by markers, in order (their names and
	/// constraints left out, as `generic_class` compares them).
	pub(super) fn class_key(&self, ty: TypeId) -> String {
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
				..Default::default()
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

	fn global_methods(&self, methods: &[GenericMethod]) -> Vec<GenericMethod> {
		let mut merger = self.types().merger(Some(self.session.module_id(self.file)));
		methods
			.iter()
			.map(|method| method.merged(&mut merger))
			.collect()
	}

	/// The table of the values a node makes: a placeholder this module
	/// defines at the end, or, compiled per unit, the table the entry
	/// object defines once every instance of the class is known (the
	/// producer reported to the driver).
	pub(super) fn generic_table_of(
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
		self.split_generic_table(ty, node, kind, self.session.module(self.file).to_string())
	}

	fn split_generic_table(
		&mut self,
		ty: TypeId,
		node: NodeId,
		kind: ProducerKind,
		module: String,
	) -> Result<PointerValue<'ctx>, Error> {
		// the bindings of the instance around a lambda, by the position of
		// each parameter in its owner's list (what another session can
		// find again)
		let mut bindings = Vec::new();
		let mut methods = Vec::new();
		if kind.captures_outer() {
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
				let bound = self
					.type_bindings
					.borrow()
					.get(&param)
					.copied()
					.or_else(|| self.instance_types.borrow().get(&(param, repr)).copied());
				let (Some((owner, index)), Some(bound)) = (owner, bound) else {
					return Err(self.unsupported(
						node.index(),
						"a generic lambda in this instance (split build)",
					));
				};
				bindings.push((owner, index, self.global_type(bound)));
			}
			bindings.sort_by_key(|&(owner, index, _)| (owner, index));
			let captured = self.method_bindings.borrow().clone();
			for (param, method) in captured {
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
				if let Some((owner, index)) = owner {
					methods.push((owner, index, self.global_methods(&[method])[0].clone()));
				}
			}
			methods.sort_by_key(|(owner, index, method)| (*owner, *index, method.name));
		}
		let mut table = format!("fa.gv.table.{module}.{}", node.index());
		if kind.defaults() {
			table.push_str(".defaults");
		}
		if let ProducerKind::Adapter { from, to } = kind {
			table.push_str(&format!(
				".adapter.{:x}.{:x}",
				self.types().stable_hash(from),
				self.types().stable_hash(to)
			));
		}
		if !bindings.is_empty() {
			let types: Vec<TypeId> = bindings.iter().map(|&(_, _, ty)| ty).collect();
			table.push('.');
			table.push_str(&self.instance_key(&types));
		}
		let selected: Vec<_> = methods
			.iter()
			.map(|(_, _, method)| method.clone())
			.collect();
		table.push_str(&super::units::method_instance_suffix(
			self.types(),
			&selected,
		));
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
			lambda: !matches!(kind.base(), ProducerKind::Declaration),
			defaults: kind.defaults(),
			adapter: match kind {
				ProducerKind::Adapter { from, to } => Some((from, to)),
				_ => None,
			},
			asynchronous: self.tasks.asynchronous
				&& matches!(self.types().kind(ty), TypeKind::Function(sig) if self.types().sig(sig).is_async),
			bindings,
			methods,
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
	pub(super) fn instance_index(
		&mut self,
		generic: TypeId,
		reprs: Vec<Repr>,
		arguments: &[TypeId],
		methods: &[GenericMethod],
		node: NodeId,
	) -> Result<inkwell::values::IntValue<'ctx>, Error> {
		match self.instance_index_source(generic, reprs, arguments, methods, node)? {
			InstanceIndex::Constant(index) => Ok(index),
			InstanceIndex::Global(global) => Ok(self
				.builder
				.build_load(self.usize, global.as_pointer_value(), "")?
				.into_int_value()),
		}
	}

	pub(super) fn instance_index_for_builder(
		&mut self,
		generic: TypeId,
		reprs: Vec<Repr>,
		arguments: &[TypeId],
		methods: &[GenericMethod],
		node: NodeId,
		builder: &inkwell::builder::Builder<'ctx>,
	) -> Result<inkwell::values::IntValue<'ctx>, Error> {
		match self.instance_index_source(generic, reprs, arguments, methods, node)? {
			InstanceIndex::Constant(index) => Ok(index),
			InstanceIndex::Global(global) => Ok(builder
				.build_load(self.usize, global.as_pointer_value(), "")?
				.into_int_value()),
		}
	}

	fn instance_index_source(
		&mut self,
		generic: TypeId,
		reprs: Vec<Repr>,
		arguments: &[TypeId],
		methods: &[GenericMethod],
		node: NodeId,
	) -> Result<InstanceIndex<'ctx>, Error> {
		if !self.is_split() {
			let class = self.generic_class(generic);
			let index =
				self.class_instance(class, reprs, arguments.to_vec(), methods.to_vec(), node)?;
			return Ok(InstanceIndex::Constant(
				self.usize.const_int(index as u64, false),
			));
		}
		let arguments: Vec<TypeId> = arguments
			.iter()
			.map(|&argument| self.global_type(argument))
			.collect();
		let instance = crate::split::GenericInstance {
			class: self.class_key(generic),
			key: format!(
				"{}{}",
				self.instance_key(&arguments),
				super::units::method_instance_suffix(self.types(), methods)
			),
			arguments,
			methods: self.global_methods(methods),
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
		Ok(InstanceIndex::Global(global))
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
		let mut types = FxHashMap::default();
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
			types.insert(param, bound);
		}
		let mut reprs = Vec::new();
		for &argument in &request.instance.arguments {
			reprs.push(self.repr_of(argument, node.index())?);
		}
		let kind = if let Some((from, to)) = producer.adapter {
			ProducerKind::Adapter { from, to }
		} else if self.analysis.partial_applications.contains_key(&node) {
			ProducerKind::Partial
		} else if producer.lambda {
			ProducerKind::Lambda
		} else {
			ProducerKind::Declaration
		};
		let kind = if producer.defaults {
			kind.with_defaults()
		} else {
			kind
		};
		let saved = self.bindings.replace(bindings);
		let parameter_values: Vec<_> = types.iter().map(|(&param, &ty)| (param, ty)).collect();
		let saved_types = self.type_bindings.replace(types);
		let saved_values = self
			.value_bindings
			.replace(self.value_bindings_for(&parameter_values));
		let mut captured_methods = Vec::new();
		for &(owner, index, ref method) in &producer.methods {
			let param = self
				.generic_functions
				.get(&NodeId(owner))
				.and_then(|info| info.generics.get(index as usize))
				.copied()
				.ok_or_else(|| self.unsupported(node.index(), "this captured generic method"))?;
			captured_methods.push((param, method.clone()));
		}
		let saved_methods = self.method_bindings.replace(captured_methods);
		let entry = self.generic_entry_in_file(
			node,
			kind,
			&reprs,
			&request.instance.arguments,
			&request.instance.methods,
		);
		self.bindings.replace(saved);
		self.type_bindings.replace(saved_types);
		self.method_bindings.replace(saved_methods);
		self.value_bindings.replace(saved_values);
		let entry = entry?;
		let name = producer.entry_symbol(&request.instance);
		if producer.asynchronous {
			self.export_async_entries(entry, &name);
		} else {
			self.export_entry(entry, &name)?;
		}
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
		let closure = if by_reference {
			closure
		} else {
			self.fresh(closure)
		};
		self.generic_function_defaults(closure, ty, node, ProducerKind::Lambda)
	}

	/// A generic function declaration used as a generic function value: its
	/// table and the shared environment of every instance when it captures locals.
	pub(crate) fn generic_declaration_value(
		&mut self,
		declaration: NodeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let ty = self.concrete_type(self.type_of(node));
		if !self.is_generic_function_type(ty) {
			return Err(self.unsupported(node.index(), "this generic function value"));
		}
		let captures = self.find_captures(declaration);
		if captures.is_empty() {
			// Enclosing value/type binders can affect defaults and methods
			// even when this function captures no runtime data.
			let kind = if self.type_bindings.borrow().is_empty()
				&& self.method_bindings.borrow().is_empty()
				&& self.value_bindings.borrow().is_empty()
			{
				ProducerKind::Declaration
			} else {
				ProducerKind::Lambda
			};
			let table = self.generic_table_of(ty, declaration, kind)?;
			let closure =
				self.closure_pair(table, self.pointer_type().const_null(), self.type_of(node))?;
			return self.generic_function_defaults(closure, ty, declaration, kind);
		}
		let table = self.generic_table_of(ty, declaration, ProducerKind::Lambda)?;
		let lambda = self
			.analysis
			.function(declaration)
			.expect("a generic declaration")
			.lambda;
		let by_reference = !self.analysis.escapes.contains(&lambda);
		let environment = self.build_environment(&captures, by_reference, node)?;
		let closure = self.closure_pair(table, environment, self.type_of(node))?;
		let closure = if by_reference {
			closure
		} else {
			self.fresh(closure)
		};
		self.generic_function_defaults(closure, ty, declaration, ProducerKind::Lambda)
	}

	/// Module-level foreign functions have no runtime captures. Keep the
	/// producer in its defining module, with the caller's closed forall type.
	pub(super) fn foreign_generic_declaration_value(
		&mut self,
		ty: TypeId,
		declaration: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let table = self.generic_table_of(ty, declaration, ProducerKind::Declaration)?;
		let closure = self.closure_pair(table, self.pointer_type().const_null(), ty)?;
		self.generic_function_defaults(closure, ty, declaration, ProducerKind::Declaration)
	}

	pub(super) fn remote_generic_declaration_value(
		&mut self,
		key: fa_analyzer::ids::DeclKey,
		ty: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let module = self.types().decl_key_data(key).module;
		let Some(interface) = self.session.interface(module) else {
			return Err(self.unsupported(node.index(), "this generic function of another module"));
		};
		let Some(declaration) = interface.node_of(key) else {
			return Err(self.unsupported(node.index(), "this generic function of another module"));
		};
		let defaults = interface
			.parameters
			.get(&declaration)
			.is_some_and(|params| params.iter().any(|param| param.has_default));
		let path = self.types().module_path(module).to_string();
		let table =
			self.split_generic_table(ty, declaration, ProducerKind::Declaration, path.clone())?;
		let closure = self.closure_pair(table, self.pointer_type().const_null(), ty)?;
		if !defaults {
			return Ok(closure);
		}
		let table =
			self.split_generic_table(ty, declaration, ProducerKind::DefaultDeclaration, path)?;
		self.with_default_entry(closure, table)
	}

	fn generic_function_defaults(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
		kind: ProducerKind,
	) -> Result<Val<'ctx>, Error> {
		let defaults = self
			.analysis
			.function(node)
			.is_some_and(|info| info.params.iter().any(|param| param.default.is_some()));
		if !defaults {
			return Ok(value);
		}
		let entry = self.generic_table_of(ty, node, kind.with_defaults())?;
		self.with_default_entry(value, entry)
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
		let given = self.types().list_items(list);
		let mut methods = self.call_methods(node, given);
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
		let offset = given.len() - count;
		methods.retain_mut(|method| {
			if (method.parameter as usize) < offset {
				return false;
			}
			method.parameter -= offset as u16;
			true
		});
		let mut reprs = Vec::with_capacity(count);
		for &argument in &arguments {
			reprs.push(self.repr_of(argument, node.index())?);
		}
		let index = self.instance_index(generic, reprs, &arguments, &methods, node)?;
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
		if self.analysis.applied_defaults.contains(&node) {
			let defaults = self.default_entry(&self.builder, environment)?;
			let slot = unsafe {
				self.builder
					.build_in_bounds_gep(self.pointer_type(), defaults, &[index], "")?
			};
			let defaults = self
				.builder
				.build_load(self.pointer_type(), slot, "")?
				.into_pointer_value();
			// The specialized entry still unwraps one defaults adapter. Give
			// it an adapter owning the original payload, with a specialized
			// default entry instead of the polymorphic table.
			let original = self.default_environment(&self.builder, environment)?;
			let value = self.closure_pair(entry, original, instance)?;
			return self.with_default_entry(value, defaults);
		}
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
				let Some((reprs, types, methods)) = self.generic_values.classes[class]
					.instances
					.get(done)
					.cloned()
				else {
					break;
				};
				let entry = self.generic_entry(producer, &reprs, &types, &methods)?;
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
		methods: &[GenericMethod],
	) -> Result<PointerValue<'ctx>, Error> {
		let (file, node, kind, bindings, types_bindings, captured_methods, values) = {
			let data = &self.generic_values.producers[producer];
			(
				data.file,
				data.node,
				data.kind,
				data.bindings.clone(),
				data.types.clone(),
				data.methods.clone(),
				data.values.clone(),
			)
		};
		let current = self.file;
		self.switch_file(file);
		let saved = self.bindings.replace(bindings);
		let saved_types = self.type_bindings.replace(types_bindings);
		let saved_methods = self.method_bindings.replace(captured_methods);
		let saved_values = self.value_bindings.replace(values);
		let entry = self.generic_entry_in_file(node, kind, reprs, types, methods);
		self.value_bindings.replace(saved_values);
		self.bindings.replace(saved);
		self.type_bindings.replace(saved_types);
		self.method_bindings.replace(saved_methods);
		self.switch_file(current);
		entry
	}

	fn generic_entry_in_file(
		&mut self,
		node: NodeId,
		kind: ProducerKind,
		reprs: &[Repr],
		types: &[TypeId],
		methods: &[GenericMethod],
	) -> Result<PointerValue<'ctx>, Error> {
		let defaults = kind.defaults();
		let kind = kind.base();
		if let ProducerKind::Adapter { from, to } = kind {
			if !self.instance_allowed(&self.own_generics(to), types) {
				return Ok(self.never_called_entry());
			}
			return self.generic_adapter_entry(node, from, to, types, methods);
		}
		// A non-generic compile-time closure from another unit still needs
		// its own wave-local definition. Reusing the main object's internal
		// template would export only a declaration from an instance wave.
		if matches!(kind, ProducerKind::Lambda)
			&& !self.generic_functions.contains_key(&node)
			&& let Some(info) = self.analysis.function(node).cloned()
		{
			self.generic_functions.insert(node, info);
		}
		let generics = if matches!(kind, ProducerKind::Partial) {
			self.own_generics(self.type_of(node))
		} else {
			match self.generic_functions.get(&node) {
				Some(info) => info.generics.clone(),
				None => self
					.analysis
					.function(node)
					.map(|info| info.generics.clone())
					.ok_or_else(|| self.unsupported(node.index(), "this generic function value"))?,
			}
		};
		if !self.instance_allowed(&generics, types) {
			return Ok(self.never_called_entry());
		}
		self.note_instance_types(&generics, types, reprs);
		if matches!(kind, ProducerKind::Partial) {
			let mut bindings = self.bindings.borrow().clone();
			bindings.extend(generics.iter().copied().zip(reprs.iter().copied()));
			let mut type_bindings = self.type_bindings.borrow().clone();
			type_bindings.extend(generics.iter().copied().zip(types.iter().copied()));
			let mut selected = self.method_bindings.borrow().clone();
			selected.extend(methods.iter().filter_map(|method| {
				generics
					.get(method.parameter as usize)
					.map(|&param| (param, method.clone()))
			}));
			let saved = self.bindings.replace(bindings);
			let saved_types = self.type_bindings.replace(type_bindings);
			let saved_methods = self.method_bindings.replace(selected);
			let entry = self.partial_generic_entry(node, defaults);
			self.bindings.replace(saved);
			self.type_bindings.replace(saved_types);
			self.method_bindings.replace(saved_methods);
			return entry;
		}
		let key = match kind {
			ProducerKind::Lambda => {
				let mut type_bindings = self.type_bindings.borrow().clone();
				type_bindings.extend(generics.iter().copied().zip(types.iter().copied()));
				let saved_types = self.type_bindings.replace(type_bindings);
				let mut selected = self.method_bindings.borrow().clone();
				selected.extend(methods.iter().filter_map(|method| {
					generics
						.get(method.parameter as usize)
						.map(|&param| (param, method.clone()))
				}));
				let saved = self.method_bindings.replace(selected);
				let parameter_values: Vec<_> = generics
					.iter()
					.copied()
					.zip(types.iter().copied())
					.collect();
				let saved_values = self
					.value_bindings
					.replace(self.value_bindings_for(&parameter_values));
				let result = if self.generic_functions.contains_key(&node) {
					self.lambda_instance(node, reprs)
				} else {
					Ok(node)
				};
				self.method_bindings.replace(saved);
				self.type_bindings.replace(saved_types);
				self.value_bindings.replace(saved_values);
				result?
			}
			ProducerKind::Declaration => {
				let values = generics
					.iter()
					.zip(types)
					.filter_map(|(&param, &ty)| {
						self.types().param(param).value.then_some((param, ty))
					})
					.collect();
				self.instance_here(
					node,
					&generics,
					types,
					reprs.to_vec(),
					values,
					methods.to_vec(),
				)?
			}
			_ => unreachable!("handled above"),
		};
		let function = self.functions[&key].clone();
		if defaults {
			return self.function_default_entry(&function);
		}
		let thunk = self.needs_value_thunk(&function);
		let entry = if thunk {
			self.thunk_of(&function)?
				.as_global_value()
				.as_pointer_value()
		} else {
			function.value.as_global_value().as_pointer_value()
		};
		self.async_value_entry(&function, entry, thunk)
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
