//! Function values: lambdas, and functions used as values.
//!
//! A function value is a pair `{ ptr function, ptr environment }`. A lambda
//! is an `internal fastcc` function whose first parameter is its
//! environment: a block holding a copy of the locals of the enclosing
//! functions it reads, made when the lambda is evaluated, after the common
//! lifetime header. Owning handles share this immutable environment. A function
//! declaration used as a value goes through a thunk that ignores the
//! environment. Calling a function value is an indirect call, with the
//! signature of the value's own type.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::types::TypeKind;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, PointerValue};

use crate::error::Error;
use crate::lower::environments::ENVIRONMENT_FIELDS;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// `{ ptr function, ptr environment }`.
	pub(crate) fn closure_type(&self) -> StructType<'ctx> {
		let pointer = self.pointer_type();
		self.context
			.struct_type(&[pointer.into(), pointer.into()], false)
	}

	/// The locals of enclosing functions that a lambda reads.
	pub(crate) fn find_captures(&mut self, lambda: NodeId) -> Vec<SymbolId> {
		// Use checked reads, not every resolved name in the syntax tree:
		// named-argument labels resolve to the callee's parameters for IDE
		// navigation, but do not capture or read those parameters.
		let mut captures: Vec<_> = self
			.analysis
			.function(lambda)
			.into_iter()
			.flat_map(|info| &info.captures)
			.copied()
			.filter(|&symbol| {
				let data = self.analysis.symbols.symbol(symbol);
				!matches!(
					data.kind,
					SymbolKind::Function | SymbolKind::Type | SymbolKind::Namespace
				) && self.owner_of_scope(data.scope).is_some()
			})
			.collect();
		captures.sort();
		captures
	}

	/// The common lifetime header, then captured values (or pointers to them).
	fn environment_type(
		&self,
		captures: &[SymbolId],
		by_reference: bool,
	) -> Result<StructType<'ctx>, Error> {
		let mut fields: Vec<BasicTypeEnum<'ctx>> = Vec::new();
		if by_reference {
			fields.extend(
				captures
					.iter()
					.map(|_| BasicTypeEnum::from(self.pointer_type())),
			);
			return Ok(self.environment_layout(&fields));
		}
		for &symbol in captures {
			let data = self.analysis.symbols.symbol(symbol);
			let repr = self.repr_of(data.ty, data.decl_node.index())?;
			fields.push(self.llvm_type(repr));
		}
		Ok(self.environment_layout(&fields))
	}

	/// Loads the captured values of a lambda from its environment into slots
	/// of its own (returns the slots they replace, to restore after).
	pub(crate) fn bind_captures(
		&mut self,
		environment: PointerValue<'ctx>,
		captures: &[SymbolId],
		by_reference: bool,
	) -> Result<Vec<(SymbolId, Option<Slot<'ctx>>)>, Error> {
		let structure = self.environment_type(captures, by_reference)?;
		let mut saved = Vec::new();
		for (index, &symbol) in captures.iter().enumerate() {
			let data = self.analysis.symbols.symbol(symbol);
			let repr = self.repr_of(data.ty, data.decl_node.index())?;
			let pointer = self.builder.build_struct_gep(
				structure,
				environment,
				index as u32 + ENVIRONMENT_FIELDS,
				"",
			)?;
			if by_reference {
				// the variable itself
				let pointer = self
					.builder
					.build_load(self.pointer_type(), pointer, "")?
					.into_pointer_value();
				saved.push((symbol, self.slots.insert(symbol, Slot { pointer, repr })));
				continue;
			}
			let value = self.builder.build_load(self.llvm_type(repr), pointer, "")?;
			let slot = self.alloca(repr, "captured")?;
			self.builder.build_store(slot.pointer, value)?;
			saved.push((symbol, self.slots.insert(symbol, slot)));
		}
		Ok(saved)
	}

	/// The instance of a lambda of a generic function (or of a generic
	/// lambda, whose own parameters are `own`): declared for the bindings of
	/// the instance being lowered, lowered at the end. Its key in
	/// `instances` is the lambda and the representations of every bound
	/// parameter, in the order of their ids.
	pub(crate) fn lambda_instance(&mut self, node: NodeId, own: &[Repr]) -> Result<NodeId, Error> {
		let Some(info) = self.generic_functions.get(&node).cloned() else {
			return Err(self.unsupported(node.index(), "this lambda"));
		};
		let mut bindings = self.bindings.borrow().clone();
		for (&param, &repr) in info.generics.iter().zip(own) {
			bindings.insert(param, repr);
		}
		let mut entries: Vec<(fa_analyzer::ids::ParamId, Repr)> = bindings
			.iter()
			.map(|(&param, &repr)| (param, repr))
			.collect();
		entries.sort_by_key(|(param, _)| *param);
		let reprs: Vec<Repr> = entries.iter().map(|(_, repr)| *repr).collect();
		let mut values: Vec<_> = entries
			.iter()
			.map(|(param, _)| {
				self.type_bindings
					.borrow()
					.get(param)
					.copied()
					.unwrap_or(TypeId::UNKNOWN)
			})
			.collect();
		values.extend(self.sorted_value_bindings());
		let mut methods: Vec<_> = self
			.method_bindings
			.borrow()
			.iter()
			.filter_map(|(param, method)| {
				entries
					.iter()
					.position(|(bound, _)| bound == param)
					.map(|index| fa_analyzer::analysis::GenericMethod {
						parameter: index as u16,
						..method.clone()
					})
			})
			.collect();
		methods.sort_by_key(|method| (method.parameter, method.name));
		if let Some(key) =
			self.instances
				.get(&(node, reprs.clone(), values.clone(), methods.clone()))
		{
			return Ok(*key);
		}
		let saved = self.bindings.replace(bindings);
		let suffix = format!(".{}", self.instances.len());
		let function = self.declare_function(node, info, &suffix);
		self.bindings.replace(saved);
		let function = function?;
		// a key that is no node of the tree
		let key = NodeId::new(self.tree.nodes.len() + self.instances.len() + 1);
		self.functions.insert(key, function);
		self.instances.insert((node, reprs, values, methods), key);
		self.pending_instances.push((self.file, key));
		Ok(key)
	}

	/// A lambda evaluated: its function and a new environment.
	pub(crate) fn lambda_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// a generic lambda (`generic_values.rs`)
		if self
			.generic_functions
			.get(&node)
			.is_some_and(|info| !info.generics.is_empty() && info.declaration.is_none())
		{
			return self.generic_lambda_value(node);
		}
		let key = if self.functions.contains_key(&node) {
			node
		} else {
			self.lambda_instance(node, &[])?
		};
		let Some(function) = self.functions.get(&key).cloned() else {
			return Err(self.unsupported(node.index(), "this lambda"));
		};
		let (value, captures) = (function.value, function.captures.clone());
		let by_reference = function.by_reference;
		let environment = self.build_environment(&captures, by_reference, node)?;
		// an async lambda: its async instance too (tasks.rs)
		let entry =
			self.async_value_entry(&function, value.as_global_value().as_pointer_value(), false)?;
		let closure = self.make_closure(entry, environment, node)?;
		// a closure on the heap owns its environment
		Ok(if by_reference {
			closure
		} else {
			self.fresh(closure)
		})
	}

	/// The environment of a closure: null without captures, pointers to the
	/// captured variables in the frame when it doesn't escape, copies of
	/// them in a heap block otherwise.
	pub(crate) fn build_environment(
		&mut self,
		captures: &[SymbolId],
		by_reference: bool,
		node: NodeId,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(if captures.is_empty() {
			self.pointer_type().const_null()
		} else if by_reference {
			let structure = self.environment_type(captures, true)?;
			let pointer = self.alloca_type(structure.into(), "environment")?;
			let (drop, copy) = self.shared_environment_functions();
			self.initialize_environment(pointer, drop, copy, false)?;
			for (index, &symbol) in captures.iter().enumerate() {
				let slot = self.slot_of(symbol, node)?;
				let field = self.builder.build_struct_gep(
					structure,
					pointer,
					index as u32 + ENVIRONMENT_FIELDS,
					"",
				)?;
				self.builder.build_store(field, slot.pointer)?;
			}
			pointer
		} else {
			let structure = self.environment_type(captures, false)?;
			let pointer = self.allocate(structure.size_of().expect("a sized environment"))?;
			let (drop, copy) = self.environment_functions(structure, captures)?;
			self.initialize_environment(pointer, drop, copy, true)?;
			for (index, &symbol) in captures.iter().enumerate() {
				let slot = self.slot_of(symbol, node)?;
				let loaded = self.load(slot)?;
				// the environment holds copies
				let loaded = self.emit_copy(loaded)?;
				let field = self.builder.build_struct_gep(
					structure,
					pointer,
					index as u32 + ENVIRONMENT_FIELDS,
					"",
				)?;
				self.builder
					.build_store(field, loaded.value.expect("a value"))?;
			}
			pointer
		})
	}

	/// The drop and the copy of an environment in a frame: nothing to free,
	/// and a copy shares it (the closure doesn't outlive the frame).
	fn shared_environment_functions(&self) -> (FunctionValue<'ctx>, FunctionValue<'ctx>) {
		let pointer = self.pointer_type();
		let drop = self
			.module
			.get_function("fa.keep_environment")
			.unwrap_or_else(|| {
				let function = self.module.add_function(
					"fa.keep_environment",
					self.context.void_type().fn_type(&[pointer.into()], false),
					Some(Linkage::Internal),
				);
				let builder = self.context.create_builder();
				builder.position_at_end(self.context.append_basic_block(function, "entry"));
				let _ = builder.build_return(None);
				function
			});
		let copy = self
			.module
			.get_function("fa.share_environment")
			.unwrap_or_else(|| {
				let function = self.module.add_function(
					"fa.share_environment",
					pointer.fn_type(&[pointer.into()], false),
					Some(Linkage::Internal),
				);
				let builder = self.context.create_builder();
				builder.position_at_end(self.context.append_basic_block(function, "entry"));
				let environment = function.get_nth_param(0).expect("an environment");
				let _ = builder.build_return(Some(&environment));
				function
			});
		(drop, copy)
	}

	/// The final release drops the captured values; handle copies share them.
	fn environment_functions(
		&self,
		structure: StructType<'ctx>,
		captures: &[SymbolId],
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let mut reprs = Vec::new();
		for &symbol in captures {
			let data = self.analysis.symbols.symbol(symbol);
			reprs.push(self.repr_of(data.ty, data.decl_node.index())?);
		}
		self.owned_environment_functions("fa.environment.drop", structure, &reprs)
	}

	fn make_closure(
		&self,
		function: PointerValue<'ctx>,
		environment: PointerValue<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		self.make_closure_of(function, environment, self.type_of(node))
	}

	/// A closure of a function of type `ty`: the type its entry is called
	/// with (a value given where more errors are allowed is converted:
	/// `adapt_function_errors`).
	pub(super) fn make_closure_of(
		&self,
		function: PointerValue<'ctx>,
		environment: PointerValue<'ctx>,
		ty: TypeId,
	) -> Result<Val<'ctx>, Error> {
		let closure = self.closure_type();
		let pair = self
			.builder
			.build_insert_value(closure.get_undef(), function, 0, "")?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		Ok(Val::new(Repr::Function(ty), pair.into_struct_value()))
	}

	/// The type of the entry of a function value: the function's own type
	/// (its errors, not those of the type it is given to).
	pub(super) fn own_function_type(
		&self,
		function: &super::Function<'ctx>,
		node: NodeId,
	) -> TypeId {
		let own = function.info.ty;
		if own.is_unset() || matches!(self.types().kind(own), TypeKind::Unknown) {
			self.type_of(node)
		} else {
			own
		}
	}

	/// A function declaration used as a value: a thunk with an environment.
	pub(crate) fn function_value(
		&mut self,
		declaration: NodeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// `function f = g then h` (compose.rs)
		if let fa_analyzer::syntax::View::Function { value, .. } = self.view(declaration)
			&& self.analysis.compositions.contains_key(&value)
		{
			return self.composition_value(value);
		}
		let Some(function) = self.functions.get(&declaration).cloned() else {
			// a generic function used as a generic function value (`generic_values.rs`)
			if self.generic_functions.contains_key(&declaration) {
				// given where an instance is expected (`names.map(identity)`)
				if self.analysis.instantiation(node).is_some()
					&& !self.is_generic_function_type(self.type_of(node))
				{
					let key = self.instance(declaration, node)?;
					let function = self.functions[&key].clone();
					let thunk = self.thunk_of(&function)?;
					// the type of the instance
					let list = self.analysis.instantiation(node).expect("an instantiation");
					let arguments: Vec<TypeId> = self
						.types()
						.list_items(list)
						.iter()
						.map(|&argument| self.concrete_type(argument))
						.collect();
					let generics = self.generic_functions[&declaration].generics.clone();
					let ty = self.substitute_params(self.type_of(node), &generics, &arguments);
					return self.make_closure_of(
						thunk.as_global_value().as_pointer_value(),
						self.pointer_type().const_null(),
						ty,
					);
				}
				return self.generic_declaration_value(declaration, node);
			}
			return Err(self.unsupported(node.index(), "this function value"));
		};
		if function.lambda {
			// a local function using locals around it: its own environment
			let (target, captures, by_reference) = (
				function.value,
				function.captures.clone(),
				function.by_reference,
			);
			let environment = self.build_environment(&captures, by_reference, node)?;
			let entry = self.async_value_entry(
				&function,
				target.as_global_value().as_pointer_value(),
				false,
			)?;
			let ty = self.own_function_type(&function, node);
			let closure = self.make_closure_of(entry, environment, ty)?;
			let closure = if by_reference {
				closure
			} else {
				self.fresh(closure)
			};
			// given where more errors are allowed: adapted (`adapt_function_errors`)
			return self.convert(closure, Repr::Function(self.type_of(node)), node.index());
		}
		let thunk = self.thunk_of(&function)?;
		// an async function: its async instance too (tasks.rs)
		let entry =
			self.async_value_entry(&function, thunk.as_global_value().as_pointer_value(), true)?;
		let ty = self.own_function_type(&function, node);
		let closure = self.make_closure_of(entry, self.pointer_type().const_null(), ty)?;
		// given where more errors are allowed: adapted (`adapt_function_errors`)
		self.convert(closure, Repr::Function(self.type_of(node)), node.index())
	}

	/// An explicitly applied function from another unit, with a concrete
	/// function type at this node and no captured environment.
	pub(crate) fn applied_external_function_value(
		&mut self,
		function: &super::Function<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let thunk = self.thunk_of(function)?;
		let entry =
			self.async_value_entry(function, thunk.as_global_value().as_pointer_value(), true)?;
		self.make_closure_of(entry, self.pointer_type().const_null(), self.type_of(node))
	}

	/// The thunk of a function declaration used as a value: the indirect
	/// call convention, whose environment it ignores.
	pub(crate) fn thunk_of(
		&mut self,
		function: &super::Function<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		let target = function.value;
		let name = format!("{}.thunk", target.get_name().to_string_lossy());
		if let Some(thunk) = self.module.get_function(&name) {
			return Ok(thunk);
		}
		let target_type = target.get_type();
		let target_parameters = target_type.get_param_types();
		let params = function.params.clone();
		let by_pointer = function.by_pointer.clone();
		// the caller's storage for the result comes first, as in the target
		// (`return_destinations.rs`, `collection_storage.rs`)
		let return_parameters =
			usize::from(function.return_slot) + 2 * usize::from(function.return_collection);
		// the indirect ABI (`indirect_type`): the result's storage, every
		// argument, the aggregates borrowed (`borrows_parameter`), no default
		// flags; then the target's other parameters (requirements)
		let mut parameters: Vec<BasicMetadataTypeEnum<'ctx>> = vec![self.pointer_type().into()];
		parameters.extend(target_parameters.iter().take(return_parameters).copied());
		let mut at = return_parameters;
		for (repr, has_default) in &params {
			parameters.push(if self.borrows_parameter(*repr) {
				self.pointer_type().into()
			} else {
				self.llvm_type(*repr).into()
			});
			at += 1 + usize::from(*has_default);
		}
		parameters.extend(target_parameters.iter().skip(at).copied());
		let thunk_type = match target_type.get_return_type() {
			Some(ret) => ret.fn_type(&parameters, false),
			None => self.context.void_type().fn_type(&parameters, false),
		};
		let thunk = self
			.module
			.add_function(&name, thunk_type, Some(Linkage::Internal));
		thunk.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(thunk, &["nounwind"]);
		if function.return_slot {
			self.add_return_slot_attribute(thunk, function.ret, function.throws, 1);
		}
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(thunk, "entry"));
		let mut given = thunk.get_param_iter().skip(1);
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = given
			.by_ref()
			.take(return_parameters)
			.map(BasicMetadataValueEnum::from)
			.collect();
		for (index, (repr, has_default)) in params.iter().enumerate() {
			let argument = given.next().expect("an argument");
			if by_pointer[index] && !self.borrows_parameter(*repr) {
				// a parameter the target changes: a copy of the value
				let slot = builder.build_alloca(self.llvm_type(*repr), "argument")?;
				builder.build_store(slot, argument)?;
				arguments.push(slot.into());
			} else {
				arguments.push(argument.into());
			}
			if *has_default {
				arguments.push(self.context.bool_type().const_int(1, false).into());
			}
		}
		arguments.extend(given.map(BasicMetadataValueEnum::from));
		let call = builder.build_call(target, &arguments, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		match call.try_as_basic_value().basic() {
			Some(result) => builder.build_return(Some(&result))?,
			None => builder.build_return(None)?,
		};
		Ok(thunk)
	}

	/// A function value given where its type throws errors and its own
	/// doesn't (`(x) => T` stored as a `(x) => T throws E`): the call
	/// conventions differ (a value, or a value and an error), so the value
	/// becomes a closure of an adapter, whose environment holds a copy of it,
	/// and which returns its result with no error. A value whose type throws
	/// too keeps its convention: the same pair.
	pub(crate) fn adapt_function_errors(
		&self,
		value: Val<'ctx>,
		from: TypeId,
		to: TypeId,
		node: usize,
	) -> Result<Val<'ctx>, Error> {
		let at = NodeId::new(node);
		let retagged = Val {
			destination: None,
			repr: Repr::Function(to),
			value: value.value,
		};
		let (Ok((from_params, from_ret, from_throws)), Ok((to_params, to_ret, to_throws))) =
			(self.value_signature(from, at), self.value_signature(to, at))
		else {
			return Ok(retagged);
		};
		if from_throws || !to_throws || from_params != to_params || from_ret != to_ret {
			return Ok(retagged);
		}
		let Some(raw) = value.value else {
			return Ok(retagged);
		};
		let closure_type = self.closure_type();
		let environment_type = self.environment_layout(&[closure_type.into()]);
		let size = environment_type.size_of().expect("a sized environment");
		let key = format!("{from:?}.{to:?}");
		// only the value of an `async` function type may be a tagged table
		let is_async = matches!(
			self.types().kind(from),
			TypeKind::Function(sig) if self.types().sig(sig).is_async
		);
		let adapter =
			self.function_errors_adapter(&key, &from_params, from_ret, is_async, environment_type)?;
		let (drop, copy) = self.adapter_environment_functions(&key, from, environment_type)?;
		// the environment: its drop, its copy, a copy of the value
		let environment = self.allocate(size)?;
		self.initialize_environment(environment, drop, copy, true)?;
		let copied = self.emit_copy(Val::new(Repr::Function(from), raw))?;
		let field =
			self.builder
				.build_struct_gep(environment_type, environment, ENVIRONMENT_FIELDS, "")?;
		self.builder
			.build_store(field, copied.value.expect("a closure"))?;
		let pair = self.builder.build_insert_value(
			closure_type.get_undef(),
			adapter.as_global_value().as_pointer_value(),
			0,
			"",
		)?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		Ok(self.fresh(Val::new(Repr::Function(to), pair.into_struct_value())))
	}

	/// The adapter of [`Lowering::adapt_function_errors`]: calls the closure
	/// its environment holds (its synchronous entry), returns `{ value, null }`.
	fn function_errors_adapter(
		&self,
		key: &str,
		params: &[Repr],
		ret: Repr,
		is_async: bool,
		environment_type: StructType<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!("fa.adapt_errors.{key}");
		if let Some(function) = self.module.get_function(&name) {
			return Ok(function);
		}
		let pointer = self.pointer_type();
		let inner_type = self.indirect_type(params, ret, false);
		let function = self.module.add_function(
			&name,
			self.indirect_type(params, ret, true),
			Some(Linkage::Internal),
		);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		let entry_block = self.context.append_basic_block(function, "entry");
		let tagged_block = self.context.append_basic_block(function, "tagged");
		let call_block = self.context.append_basic_block(function, "call");
		builder.position_at_end(entry_block);
		let environment = function
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let field =
			builder.build_struct_gep(environment_type, environment, ENVIRONMENT_FIELDS, "")?;
		let closure = builder
			.build_load(self.closure_type(), field, "")?
			.into_struct_value();
		let entry = builder
			.build_extract_value(closure, 0, "")?
			.into_pointer_value();
		let inner_environment = builder.build_extract_value(closure, 1, "")?;
		// an async function's value: its synchronous entry is in its table
		let address = builder.build_ptr_to_int(entry, self.usize, "")?;
		let tag = builder.build_and(address, self.usize.const_int(1, false), "")?;
		let tagged = if is_async {
			builder.build_int_compare(
				inkwell::IntPredicate::NE,
				tag,
				self.usize.const_zero(),
				"",
			)?
		} else {
			self.context.bool_type().const_zero()
		};
		builder.build_conditional_branch(tagged, tagged_block, call_block)?;
		builder.position_at_end(tagged_block);
		let table = unsafe {
			builder.build_gep(
				self.context.i8_type(),
				entry,
				&[self.context.i64_type().const_all_ones()],
				"",
			)?
		};
		let sync_entry = builder.build_load(pointer, table, "")?.into_pointer_value();
		builder.build_unconditional_branch(call_block)?;
		builder.position_at_end(call_block);
		let target = builder.build_phi(pointer, "entry")?;
		target.add_incoming(&[(&entry, entry_block), (&sync_entry, tagged_block)]);
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = vec![inner_environment.into()];
		for index in 1..function.count_params() {
			arguments.push(function.get_nth_param(index).expect("a parameter").into());
		}
		let call = builder.build_indirect_call(
			inner_type,
			target.as_basic_value().into_pointer_value(),
			&arguments,
			"",
		)?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		// a result built in the caller's slot: the same slot for both, and
		// only the error is returned (`return_destinations.rs`)
		let return_slot = self.uses_return_slot(ret);
		if return_slot {
			self.add_return_slot_attribute(function, ret, true, 1);
			self.add_return_slot_call_attribute(call, ret, false, 1);
		}
		let null = pointer.const_null();
		if ret.has_value() && !return_slot {
			let pair_type = self
				.context
				.struct_type(&[self.llvm_type(ret), pointer.into()], false);
			let value = call
				.try_as_basic_value()
				.basic()
				.unwrap_or_else(|| super::zero_of(self.llvm_type(ret)));
			let pair = builder.build_insert_value(pair_type.get_undef(), value, 0, "")?;
			let pair = builder.build_insert_value(pair, null, 1, "")?;
			builder.build_return(Some(&inkwell::values::BasicValue::as_basic_value_enum(
				&pair,
			)))?;
		} else {
			builder.build_return(Some(&null))?;
		}
		Ok(function)
	}

	/// The adapter owns one handle; copying the adapter retains its environment.
	fn adapter_environment_functions(
		&self,
		key: &str,
		from: TypeId,
		environment_type: StructType<'ctx>,
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		self.owned_environment_functions(
			&format!("fa.adapt_errors.drop.{key}"),
			environment_type,
			&[Repr::Function(from)],
		)
	}

	/// The IR signature of a call through a function value of type `ty`
	/// (without the environment): the parameter and result representations.
	pub(crate) fn value_signature(
		&self,
		ty: TypeId,
		node: NodeId,
	) -> Result<(Vec<Repr>, Repr, bool), Error> {
		let TypeKind::Function(sig) = self.types().kind(ty) else {
			return Err(self.unsupported(node.index(), "calling this value"));
		};
		let sig = self.types().sig(sig);
		let mut params = Vec::new();
		for param in self.types().params(sig.params) {
			params.push(self.repr_of(param.ty, node.index())?);
		}
		let ret = self.repr_of(sig.ret, node.index())?;
		Ok((params, ret, sig.throws != TypeId::NEVER))
	}

	pub(crate) fn indirect_type(
		&self,
		params: &[Repr],
		ret: Repr,
		throws: bool,
	) -> FunctionType<'ctx> {
		let mut types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![self.pointer_type().into()];
		types.extend(params.iter().map(|repr| {
			if self.borrows_parameter(*repr) {
				self.pointer_type().into()
			} else {
				BasicMetadataTypeEnum::from(self.llvm_type(*repr))
			}
		}));
		self.function_type_of(types, true, ret, throws).0
	}

	/// Collection callbacks borrow their original element slots. A recursive
	/// parameter can use that address directly instead of allocating a copy.
	pub(super) fn call_collection_callback(
		&mut self,
		callback: Val<'ctx>,
		node: NodeId,
		before: &[Val<'ctx>],
		items: &[Slot<'ctx>],
		after: &[Val<'ctx>],
	) -> Result<Val<'ctx>, Error> {
		self.call_collection_callback_into(callback, node, before, items, after, None)
	}

	/// `call_collection_callback` whose result is built in `destination`
	/// when the callback returns it in a caller's slot.
	pub(super) fn call_collection_callback_into(
		&mut self,
		callback: Val<'ctx>,
		node: NodeId,
		before: &[Val<'ctx>],
		items: &[Slot<'ctx>],
		after: &[Val<'ctx>],
		destination: Option<Slot<'ctx>>,
	) -> Result<Val<'ctx>, Error> {
		// the type of the value given (an instance of a generic lambda, `generic_values.rs`)
		let ty = match callback.repr {
			Repr::Function(ty)
				if self.is_generic_function_type(self.type_of(node))
					|| self.value_signature(self.type_of(node), node).is_err() =>
			{
				ty
			}
			_ => self.type_of(node),
		};
		let (params, _, _) = self.value_signature(ty, node)?;
		let mark = self.mark();
		let mut arguments = before.to_vec();
		let mut places = vec![None; before.len()];
		let mut write_backs = Vec::new();
		for &slot in items {
			let expected = params.get(arguments.len()).copied().unwrap_or(slot.repr);
			let argument = self.collection_item_view(slot, expected, node.index())?;
			if matches!(slot.repr, Repr::Row(_)) && self.borrows_parameter(expected) {
				// Gathered column fields are a borrowed view of the row. The
				// callback's writes must be scattered even when it throws.
				let view = self.alloca(expected, "callback_row")?;
				self.builder
					.build_store(view.pointer, argument.value.expect("a row"))?;
				places.push(Some(view.pointer));
				write_backs.push(crate::lower::references::WriteBack::Fields { place: slot, view });
				arguments.push(argument);
				continue;
			}
			// an item of the parameter's representation: lent in place
			places.push(
				(slot.repr == expected && self.borrows_parameter(expected)).then_some(slot.pointer),
			);
			arguments.push(argument);
		}
		arguments.extend_from_slice(after);
		let call = |this: &mut Self| {
			this.with_destination(node, destination, |this| {
				this.call_closure_lending(callback, ty, &arguments, &places, node)
			})
		};
		let result = if write_backs.is_empty() {
			call(self)?
		} else {
			self.callback_with_fixup(node, call, |this| {
				for &write_back in &write_backs {
					this.write_back(write_back)?;
				}
				Ok(())
			})?
		};
		for write_back in write_backs {
			self.write_back(write_back)?;
		}
		self.end_scope(mark, Some(result))?;
		Ok(result)
	}

	/// Whether a callback given to a built-in may throw (its type's errors):
	/// the built-in then throws them (`design/decisions.md`).
	pub(crate) fn callback_may_throw(&self, node: NodeId) -> bool {
		self.value_signature(self.type_of(node), node)
			.is_ok_and(|(_, _, throws)| throws)
	}

	/// A callback call of a built-in's loop whose error must restore what
	/// the loop changed first (`fixup`: drops what was built, closes a gap),
	/// then leaves like any error (`raise`). Without errors, just `call`.
	pub(crate) fn callback_with_fixup(
		&mut self,
		callback_node: NodeId,
		call: impl FnOnce(&mut Self) -> Result<Val<'ctx>, Error>,
		fixup: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<Val<'ctx>, Error> {
		if !self.callback_may_throw(callback_node) {
			return call(self);
		}
		let handler = self.append_block("callback_failed");
		let error_slot = self.alloca(Repr::Union, "callback_error")?;
		let mark = self.mark();
		self.frame_mut()
			.catches
			.push((handler, error_slot.pointer, mark));
		let result = call(self);
		self.frame_mut().catches.pop();
		let result = result?;
		let resume = self.builder.get_insert_block();
		self.builder.position_at_end(handler);
		fixup(self)?;
		let error = self
			.builder
			.build_load(self.pointer_type(), error_slot.pointer, "")?
			.into_pointer_value();
		self.raise(error)?;
		if let Some(resume) = resume {
			self.builder.position_at_end(resume);
		}
		Ok(result)
	}

	/// Calls a function value of type `ty` with lowered arguments (as many as
	/// its type declares: extra ones, like the `index` of a callback, are left out).
	pub(crate) fn call_closure(
		&mut self,
		closure: Val<'ctx>,
		ty: TypeId,
		arguments: &[Val<'ctx>],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		self.call_closure_lending(closure, ty, arguments, &[], node)
	}

	/// `call_closure` where some arguments are places (`places`, by
	/// position) that a borrowed parameter may see directly.
	pub(crate) fn call_closure_lending(
		&mut self,
		closure: Val<'ctx>,
		ty: TypeId,
		arguments: &[Val<'ctx>],
		places: &[Option<PointerValue<'ctx>>],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// a value of an `async` function type (tasks.rs)
		if let Some(value) = self.async_value_call(closure, ty, arguments, node)? {
			return Ok(value);
		}
		// a value of a generic function type: its instance for the call (`generic_values.rs`)
		let (closure, ty) = if self.is_generic_function_type(ty) {
			let instance = self.generic_instance_value(closure, node)?;
			let Repr::Function(instance_type) = instance.repr else {
				return Err(self.unsupported(node.index(), "calling this generic function value"));
			};
			(instance, instance_type)
		} else {
			(closure, ty)
		};
		let (mut params, ret, throws) = self.value_signature(ty, node)?;
		// a lambda or a function may declare fewer parameters than its type
		// (the `index` of a callback): it gets the ones it declares
		if let Some(arity) = self.declared_arity(node) {
			params.truncate(arity);
		}
		let pair = closure.value.expect("a function value").into_struct_value();
		let function = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let environment = self.builder.build_extract_value(pair, 1, "")?;
		let mut lowered: Vec<BasicMetadataValueEnum<'ctx>> = vec![environment.into()];
		// Argument conversions can allocate (an inline collection item passed
		// as a recursive edge). Their temporaries belong to this call, which
		// may itself be inside a conditional callback branch.
		let mark = self.mark();
		for (position, (repr, argument)) in params.iter().zip(arguments).enumerate() {
			if self.borrows_parameter(*repr) {
				// a borrowed parameter: the place itself, or the value in a
				// slot of the caller for the call
				let pointer = match places.get(position).copied().flatten() {
					Some(pointer) => pointer,
					None => {
						let argument = self.convert(*argument, *repr, node.index())?;
						let slot = self.alloca(*repr, "argument")?;
						self.builder
							.build_store(slot.pointer, argument.value.expect("an argument"))?;
						slot.pointer
					}
				};
				lowered.push(pointer.into());
				continue;
			}
			let argument = self.convert(*argument, *repr, node.index())?;
			lowered.push(argument.value.expect("an argument").into());
		}
		let return_slot = if self.uses_return_slot(ret) {
			let slot = self.call_return_slot(node, ret)?;
			lowered.insert(1, slot.pointer.into());
			Some(slot)
		} else {
			None
		};
		if self.uses_collection_return(ret) {
			let offer = self.collection_call_buffer(node)?;
			let (pointer, bytes) = offer
				.map(|o| (o.pointer, o.bytes))
				.unwrap_or((self.pointer_type().const_null(), self.usize.const_zero()));
			lowered.insert(1, pointer.into());
			lowered.insert(2, bytes.into());
		}
		let fn_type = self.indirect_type(&params, ret, throws);
		let call = self
			.builder
			.build_indirect_call(fn_type, function, &lowered, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		if return_slot.is_some() {
			self.add_return_slot_call_attribute(call, ret, throws, 1);
		}
		let result =
			self.slot_call_result(call.try_as_basic_value().basic(), ret, throws, return_slot)?;
		self.end_scope(mark, Some(result))?;
		Ok(result)
	}

	/// The value of a call: raises its error, if it throws.
	pub(crate) fn call_result(
		&mut self,
		result: Option<inkwell::values::BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
	) -> Result<Val<'ctx>, Error> {
		if throws {
			let result = result.expect("a result");
			let (value, error) = if ret.has_value() {
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
			return Ok(match value {
				Some(value) if ret != Repr::Never => self.fresh(Val::new(ret, value)),
				_ => Val::void(),
			});
		}
		if ret == Repr::Never {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		Ok(match result {
			Some(result) => self.fresh(Val::new(ret, result)),
			None => Val::void(),
		})
	}

	/// The number of parameters a lambda or a named function declares.
	pub(crate) fn declared_arity(&self, node: NodeId) -> Option<usize> {
		let mut node = node;
		while let fa_analyzer::syntax::View::Group(inner) = self.view(node) {
			node = inner;
		}
		if let Some(function) = self.functions.get(&node) {
			return Some(function.info.params.len());
		}
		if let Resolution::Local(symbol) = self.analysis.resolution(node) {
			let declaration = self.analysis.symbols.symbol(symbol).decl_node;
			return self
				.functions
				.get(&declaration)
				.map(|function| function.info.params.len());
		}
		None
	}

	/// The function value of a name: a lambda's, or a declaration's thunk.
	pub(crate) fn function_of_symbol(
		&mut self,
		symbol: SymbolId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let declaration = self.analysis.symbols.symbol(symbol).decl_node;
		self.function_value(declaration, node)
	}
}
