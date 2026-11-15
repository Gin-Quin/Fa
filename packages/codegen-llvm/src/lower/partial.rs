//! A partial function owns its target and bound values in one counted
//! environment. Calls lend its immutable data to the original function.

use fa_analyzer::analysis::PartialApplication;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::TypeKind;
use inkwell::builder::Builder;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, PointerValue};
use inkwell::{AtomicOrdering, AtomicRMWBinOp, IntPredicate};

use crate::error::Error;
use crate::lower::generic_values::ProducerKind;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};
use crate::target::Target;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn partial_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let partial = self.analysis.partial_applications[&node].clone();
		let ty = self.concrete_type(self.type_of(node));
		let original = self.partial_original_type(&partial);
		let TypeKind::Function(signature) = self.types().kind(original) else {
			return Err(self.unsupported(node.index(), "this partial function"));
		};
		let params = self
			.types()
			.params(self.types().sig(signature).params)
			.to_vec();
		let saved = self.clear_partial_generics(&partial);
		let function = self.expr(partial.function);
		self.bindings.replace(saved.0);
		self.type_bindings.replace(saved.1);
		let function = function?;
		if function.repr == Repr::Never {
			return Ok(function);
		}
		let function = self.own(function, Some(partial.function))?;
		let mut values = vec![function];
		for &(index, value) in &partial.bound {
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let expected = self.repr_of(params[index as usize].ty, value.index())?;
			let lowered = self.convert(lowered, expected, value.index())?;
			values.push(self.own(lowered, Some(value))?);
		}
		let reprs: Vec<_> = values.iter().map(|value| value.repr).collect();
		let layout = self.partial_layout(&reprs);
		let entry = if self.is_generic_function_type(ty) {
			self.generic_table_of(ty, node, ProducerKind::Partial)?
		} else {
			self.partial_entry(node, &partial, layout, &reprs)?
		};
		let key = self.partial_key(node);
		let (drop, copy) = self.partial_environment_functions(&key, layout, &reprs)?;
		let environment = self.allocate(layout.size_of().expect("a sized environment"))?;
		for (index, function) in [drop, copy].into_iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(layout, environment, index as u32, "")?;
			self.builder
				.build_store(field, function.as_global_value().as_pointer_value())?;
		}
		let count = self.builder.build_struct_gep(layout, environment, 2, "")?;
		self.builder
			.build_store(count, self.context.i64_type().const_int(1, false))?;
		for (index, value) in values.into_iter().enumerate() {
			self.unregister(value);
			let field = self
				.builder
				.build_struct_gep(layout, environment, index as u32 + 3, "")?;
			self.builder
				.build_store(field, value.value.expect("a captured value"))?;
		}
		let closure = self.closure_type();
		let pair = self
			.builder
			.build_insert_value(closure.get_undef(), entry, 0, "")?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		let value = self.fresh(Val::new(Repr::Function(ty), pair.into_struct_value()));
		if !self.partial_has_defaults(&partial) {
			return Ok(value);
		}
		let entry = if self.is_generic_function_type(ty) {
			self.generic_table_of(ty, node, ProducerKind::DefaultPartial)?
		} else {
			self.partial_entry_with_defaults(node, &partial, layout, &reprs, true)?
		};
		self.with_default_entry(value, entry)
	}

	fn partial_original_type(&self, partial: &PartialApplication) -> TypeId {
		let arguments: Vec<_> = self
			.types()
			.list_items(partial.generic_arguments)
			.iter()
			.map(|&ty| self.concrete_type(ty))
			.collect();
		self.substitute_params(
			partial.original_type,
			&partial.generic_parameters,
			&arguments,
		)
	}

	fn clear_partial_generics(
		&self,
		partial: &PartialApplication,
	) -> (
		rustc_hash::FxHashMap<fa_analyzer::ids::ParamId, Repr>,
		rustc_hash::FxHashMap<fa_analyzer::ids::ParamId, TypeId>,
	) {
		let mut bindings = self.bindings.borrow().clone();
		let mut types = self.type_bindings.borrow().clone();
		for param in &partial.generic_parameters {
			bindings.remove(param);
			types.remove(param);
		}
		(
			self.bindings.replace(bindings),
			self.type_bindings.replace(types),
		)
	}

	pub(super) fn partial_generic_entry(
		&mut self,
		node: NodeId,
		defaults: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let partial = self.analysis.partial_applications[&node].clone();
		let original = self.partial_original_type(&partial);
		let TypeKind::Function(signature) = self.types().kind(original) else {
			return Err(self.unsupported(node.index(), "this partial function"));
		};
		let params = self
			.types()
			.params(self.types().sig(signature).params)
			.to_vec();
		let mut reprs = vec![Repr::Function(partial.original_type)];
		for &(index, value) in &partial.bound {
			reprs.push(self.repr_of(params[index as usize].ty, value.index())?);
		}
		self.partial_entry_with_defaults(
			node,
			&partial,
			self.partial_layout(&reprs),
			&reprs,
			defaults,
		)
	}

	fn partial_has_defaults(&self, partial: &PartialApplication) -> bool {
		partial
			.remaining
			.iter()
			.any(|&index| partial.defaults[index as usize])
	}

	fn partial_default_flags(
		&self,
		partial: &PartialApplication,
		function: FunctionValue<'ctx>,
		defaults: bool,
	) -> Vec<inkwell::values::IntValue<'ctx>> {
		if !defaults {
			return Vec::new();
		}
		let mut at = partial.remaining.len() as u32 + 1;
		let remaining: Vec<_> = partial
			.remaining
			.iter()
			.map(|&index| {
				if partial.defaults[index as usize] {
					let value = function
						.get_nth_param(at)
						.expect("a default presence flag")
						.into_int_value();
					at += 1;
					Some(value)
				} else {
					None
				}
			})
			.collect();
		partial
			.defaults
			.iter()
			.enumerate()
			.filter(|(_, default)| **default)
			.map(|(index, _)| {
				partial
					.remaining
					.iter()
					.position(|&param| param as usize == index)
					.and_then(|position| remaining[position])
					.unwrap_or_else(|| self.context.bool_type().const_int(1, false))
			})
			.collect()
	}

	pub(super) fn partial_layout(&self, reprs: &[Repr]) -> StructType<'ctx> {
		let pointer = self.pointer_type();
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![
			pointer.into(),
			pointer.into(),
			self.context.i64_type().into(),
		];
		fields.extend(reprs.iter().map(|&repr| self.llvm_type(repr)));
		self.context.struct_type(&fields, false)
	}

	fn partial_key(&self, node: NodeId) -> String {
		let mut bindings: Vec<_> = self
			.type_bindings
			.borrow()
			.iter()
			.map(|(&param, &ty)| {
				let info = self.types().param(param);
				(
					self.types().decl_hash(info.owner),
					info.index,
					self.types().stable_hash(ty),
				)
			})
			.collect();
		bindings.sort_unstable();
		let mut methods: Vec<_> = self
			.method_bindings
			.borrow()
			.iter()
			.map(|(param, method)| {
				let info = self.types().param(*param);
				(
					self.types().decl_hash(info.owner),
					info.index,
					method.stable_hash(self.types()),
				)
			})
			.collect();
		methods.sort_unstable();
		format!(
			"{}.{}.{bindings:x?}.{methods:x?}",
			node.index(),
			self.shape_key(Repr::Function(self.type_of(node)))
		)
	}

	fn partial_function(
		&mut self,
		node: NodeId,
		partial: &PartialApplication,
		layout: StructType<'ctx>,
		reprs: &[Repr],
		defaults: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!(
			"fa.partial.{}.{}",
			self.partial_key(node),
			if defaults { "defaults" } else { "call" }
		);
		if let Some(function) = self.module.get_function(&name) {
			return Ok(function);
		}
		let ty = self.concrete_type(self.type_of(node));
		let (params, ret, throws) = self.value_signature(ty, node)?;
		let requirements = self.value_requirement_keys(ty).len();
		let original = self.partial_original_type(partial);
		let (original_params, original_ret, original_throws) =
			self.value_signature(original, node)?;
		if ret != original_ret || throws != original_throws {
			return Err(self.unsupported(node.index(), "this partial function result conversion"));
		}
		let count = if defaults {
			partial
				.remaining
				.iter()
				.filter(|&&index| partial.defaults[index as usize])
				.count()
		} else {
			0
		};
		let (function, builder) = self.helper_function(
			&name,
			self.indirect_type_with_defaults(&params, ret, throws, count, requirements),
		);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		let environment = function
			.get_first_param()
			.expect("an environment")
			.into_pointer_value();
		let environment = if self.partial_has_defaults(partial) {
			self.default_environment(&builder, environment)?
		} else {
			environment
		};
		let (mut entry, target_environment) =
			self.partial_target(node, partial, layout, environment, &builder, defaults)?;
		if matches!(self.types().kind(original), TypeKind::Function(sig) if self.types().sig(sig).is_async)
		{
			entry = self.synchronous_entry(function, &builder, entry)?;
		}
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = vec![target_environment.into()];
		for (index, &repr) in original_params.iter().enumerate() {
			if let Some(position) = partial
				.bound
				.iter()
				.position(|(bound, _)| *bound as usize == index)
			{
				let field =
					builder.build_struct_gep(layout, environment, position as u32 + 4, "")?;
				if self.value_borrows_parameter(repr) {
					arguments.push(field.into());
				} else {
					let value =
						builder.build_load(self.llvm_type(reprs[position + 1]), field, "")?;
					arguments.push(value.into());
				}
			} else {
				let position = partial
					.remaining
					.iter()
					.position(|remaining| *remaining as usize == index)
					.expect("a residual parameter");
				arguments.push(
					function
						.get_nth_param(position as u32 + 1)
						.expect("a residual argument")
						.into(),
				);
			}
		}
		arguments.extend(
			self.partial_default_flags(partial, function, defaults)
				.into_iter()
				.map(BasicMetadataValueEnum::from),
		);
		arguments.extend(
			function
				.get_param_iter()
				.skip(params.len() + 1 + count)
				.map(BasicMetadataValueEnum::from),
		);
		let call = builder.build_indirect_call(
			self.indirect_type_with_defaults(
				&original_params,
				ret,
				throws,
				if defaults {
					partial.defaults.iter().filter(|&&default| default).count()
				} else {
					0
				},
				requirements,
			),
			entry,
			&arguments,
			"",
		)?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		call.set_tail_call(true);
		match call.try_as_basic_value().basic() {
			Some(value) => builder.build_return(Some(&value))?,
			None => builder.build_return(None)?,
		};
		Ok(function)
	}

	pub(super) fn partial_entry(
		&mut self,
		node: NodeId,
		partial: &PartialApplication,
		layout: StructType<'ctx>,
		reprs: &[Repr],
	) -> Result<PointerValue<'ctx>, Error> {
		self.partial_entry_with_defaults(node, partial, layout, reprs, false)
	}

	pub(super) fn partial_entry_with_defaults(
		&mut self,
		node: NodeId,
		partial: &PartialApplication,
		layout: StructType<'ctx>,
		reprs: &[Repr],
		defaults: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let synchronous = self.partial_function(node, partial, layout, reprs, defaults)?;
		let synchronous = synchronous.as_global_value().as_pointer_value();
		if !self.tasks.asynchronous
			|| !matches!(self.types().kind(self.type_of(node)), TypeKind::Function(sig) if self.types().sig(sig).is_async)
		{
			return Ok(synchronous);
		}
		let asynchronous = self.partial_async_function(node, partial, layout, defaults)?;
		Ok(self.async_entries(
			synchronous,
			asynchronous.as_global_value().as_pointer_value(),
		))
	}

	/// Reads the original function value, selecting the full instance from
	/// its table before either the synchronous or asynchronous call.
	fn partial_target(
		&mut self,
		node: NodeId,
		partial: &PartialApplication,
		layout: StructType<'ctx>,
		environment: PointerValue<'ctx>,
		builder: &Builder<'ctx>,
		defaults: bool,
	) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>), Error> {
		let field = builder.build_struct_gep(layout, environment, 3, "")?;
		let target = builder
			.build_load(self.closure_type(), field, "")?
			.into_struct_value();
		let mut entry = builder
			.build_extract_value(target, 0, "")?
			.into_pointer_value();
		let target_environment = builder
			.build_extract_value(target, 1, "")?
			.into_pointer_value();
		if defaults {
			entry = self.default_entry(builder, target_environment)?;
		}
		if self.is_generic_function_type(partial.original_type) {
			let saved = self.clear_partial_generics(partial);
			let generic = self.concrete_type(partial.original_type);
			self.bindings.replace(saved.0);
			self.type_bindings.replace(saved.1);
			let arguments: Vec<_> = self
				.types()
				.list_items(partial.generic_arguments)
				.iter()
				.map(|&ty| self.concrete_type(ty))
				.collect();
			let argument_reprs = arguments
				.iter()
				.map(|&ty| self.repr_of(ty, node.index()))
				.collect::<Result<Vec<_>, _>>()?;
			let given = self.types().list_items(partial.generic_arguments);
			let methods = self.call_methods(node, given);
			let index = self.instance_index_for_builder(
				generic,
				argument_reprs,
				&arguments,
				&methods,
				node,
				builder,
			)?;
			// SAFETY: the generic value table includes every requested instance.
			let slot =
				unsafe { builder.build_in_bounds_gep(self.pointer_type(), entry, &[index], "")? };
			entry = builder
				.build_load(self.pointer_type(), slot, "")?
				.into_pointer_value();
		}
		Ok((entry, target_environment))
	}

	fn partial_async_function(
		&mut self,
		node: NodeId,
		partial: &PartialApplication,
		layout: StructType<'ctx>,
		defaults: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!(
			"fa.partial.{}.async.{}",
			self.partial_key(node),
			if defaults { "defaults" } else { "call" }
		);
		if let Some(function) = self.module.get_function(&name) {
			return Ok(function);
		}
		let ty = self.concrete_type(self.type_of(node));
		let (params, ret, throws) = self.value_signature(ty, node)?;
		let requirements = self.value_requirement_keys(ty).len();
		let original = self.partial_original_type(partial);
		let (original_params, _, _) = self.value_signature(original, node)?;
		let count = if defaults {
			partial
				.remaining
				.iter()
				.filter(|&&index| partial.defaults[index as usize])
				.count()
		} else {
			0
		};
		let mut parameters = self
			.indirect_type_with_defaults(&params, ret, throws, count, requirements)
			.get_param_types();
		parameters.push(self.context.i32_type().into());
		let function = self.module.add_function(
			&name,
			self.pointer_type().fn_type(&parameters, false),
			Some(inkwell::module::Linkage::Internal),
		);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(function, &["nounwind"]);
		self.nested_function(function, ret, |this| {
			this.frame_mut().throws = throws;
			let environment = function
				.get_first_param()
				.expect("an environment")
				.into_pointer_value();
			let owner_environment = environment;
			let environment = if this.partial_has_defaults(partial) {
				this.default_environment(&this.builder, environment)?
			} else {
				environment
			};
			let mode = function.get_last_param().expect("a mode").into_int_value();
			this.begin_coroutine(ret, throws, mode, true, false, "a partial function".into())?;
			// Starting the call must retain its environment before the first
			// suspension; the source function value may then leave its scope.
			let closure =
				this.make_closure_of(this.pointer_type().const_null(), owner_environment, ty)?;
			let owned = this.emit_copy(closure)?;
			let slot = this.alloca(owned.repr, "partial_environment")?;
			this.builder
				.build_store(slot.pointer, owned.value.expect("a function"))?;
			this.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Local(slot));
			let TypeKind::Function(signature) = this.types().kind(ty) else {
				unreachable!()
			};
			let parameter_types = this
				.types()
				.params(this.types().sig(signature).params)
				.to_vec();
			let mut values = Vec::new();
			let mut places = Vec::new();
			for (index, &repr) in params.iter().enumerate() {
				let argument = function
					.get_nth_param(index as u32 + 1)
					.expect("an argument");
				let place = this
					.value_borrows_parameter(repr)
					.then(|| argument.into_pointer_value());
				let value = match place {
					Some(pointer) => this.builder.build_load(this.llvm_type(repr), pointer, "")?,
					None => argument,
				};
				values.push((
					Val::new(repr, value),
					this.types()
						.takes_task_parameter(this.concrete_type(parameter_types[index].ty)),
				));
				places.push(place);
			}
			let borrowed: Vec<_> = values
				.iter()
				.copied()
				.filter(|(_, moved)| !*moved)
				.collect();
			this.prepare_value_parameters(&borrowed)?;
			let builder = this.context.create_builder();
			builder.position_at_end(this.builder.get_insert_block().expect("a block"));
			let (entry, target_environment) =
				this.partial_target(node, partial, layout, environment, &builder, defaults)?;
			let target = this.make_closure_of(entry, target_environment, original)?;
			let mut arguments = Vec::new();
			let mut argument_places = Vec::new();
			for (index, &repr) in original_params.iter().enumerate() {
				let value = if let Some(position) = partial
					.bound
					.iter()
					.position(|(bound, _)| *bound as usize == index)
				{
					let field = this.builder.build_struct_gep(
						layout,
						environment,
						position as u32 + 4,
						"",
					)?;
					argument_places.push(this.value_borrows_parameter(repr).then_some(field));
					Val::new(
						repr,
						this.builder.build_load(this.llvm_type(repr), field, "")?,
					)
				} else {
					let position = partial
						.remaining
						.iter()
						.position(|remaining| *remaining as usize == index)
						.expect("a residual parameter");
					argument_places.push(places[position]);
					values[position].0
				};
				arguments.push(value);
			}
			let requirements: Vec<_> = function
				.get_param_iter()
				.skip(params.len() + 1 + count)
				.take(requirements)
				.map(|value| value.into_pointer_value())
				.collect();
			let flags = this.partial_default_flags(partial, function, defaults);
			let child = this.create_wrapped_frame(
				target,
				original,
				(&arguments, &argument_places),
				&flags,
				&requirements,
				node,
			)?;
			let child_owner =
				this.alloca(Repr::Task(this.wrapper_task_type(ty)), "partial_child")?;
			this.builder.build_store(child_owner.pointer, child)?;
			let active = this.alloca(Repr::Bool, "partial_child_owned")?;
			this.builder
				.build_store(active.pointer, this.context.bool_type().const_int(1, false))?;
			this.own_flagged(child_owner, active.pointer);
			this.suspend_value_body()?;
			// Await owns cancellation from this point. Before the first turn,
			// the flagged local above cancels the child before its arguments.
			this.builder
				.build_store(active.pointer, this.context.bool_type().const_zero())?;
			let result = this.await_wrapped_frame(child, ret, throws, node)?;
			this.build_return(result, node)?;
			this.cleanups.borrow_mut().clear();
			this.end_coroutine()
		})?;
		Ok(function)
	}

	pub(super) fn partial_environment_functions(
		&self,
		key: &str,
		layout: StructType<'ctx>,
		reprs: &[Repr],
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let drop_name = format!("fa.partial.drop.{key}");
		let copy_name = format!("fa.partial.copy.{key}");
		if let (Some(drop), Some(copy)) = (
			self.module.get_function(&drop_name),
			self.module.get_function(&copy_name),
		) {
			return Ok((drop, copy));
		}
		let pointer = self.pointer_type();
		let (copy, builder) =
			self.helper_function(&copy_name, pointer.fn_type(&[pointer.into()], false));
		let environment = copy
			.get_first_param()
			.expect("an environment")
			.into_pointer_value();
		self.partial_count(&builder, layout, environment, false)?;
		builder.build_return(Some(&environment))?;
		let (drop, builder) = self.helper_function(
			&drop_name,
			self.context.void_type().fn_type(&[pointer.into()], false),
		);
		let environment = drop
			.get_first_param()
			.expect("an environment")
			.into_pointer_value();
		let old = self.partial_count(&builder, layout, environment, true)?;
		let last = builder.build_int_compare(
			IntPredicate::EQ,
			old,
			self.context.i64_type().const_int(1, false),
			"",
		)?;
		let destroy = self.context.append_basic_block(drop, "destroy");
		let done = self.context.append_basic_block(drop, "done");
		builder.build_conditional_branch(last, destroy, done)?;
		builder.position_at_end(destroy);
		for (index, &repr) in reprs.iter().enumerate().rev() {
			let field = builder.build_struct_gep(layout, environment, index as u32 + 3, "")?;
			let value = builder.build_load(self.llvm_type(repr), field, "")?;
			builder.build_call(self.drop_function(repr)?, &[value.into()], "")?;
		}
		let size = builder.build_int_cast(
			layout.size_of().expect("a sized environment"),
			self.usize,
			"",
		)?;
		builder.build_call(
			self.runtime.free()?,
			&[
				environment.into(),
				size.into(),
				self.usize.const_int(16, false).into(),
			],
			"",
		)?;
		builder.build_unconditional_branch(done)?;
		builder.position_at_end(done);
		builder.build_return(None)?;
		Ok((drop, copy))
	}

	fn partial_count(
		&self,
		builder: &Builder<'ctx>,
		layout: StructType<'ctx>,
		environment: PointerValue<'ctx>,
		release: bool,
	) -> Result<inkwell::values::IntValue<'ctx>, Error> {
		let count = builder.build_struct_gep(layout, environment, 2, "")?;
		let i64_type = self.context.i64_type();
		let amount = if release {
			i64_type.const_all_ones()
		} else {
			i64_type.const_int(1, false)
		};
		if self.target == Target::Wasm {
			let old = builder.build_load(i64_type, count, "")?.into_int_value();
			let next = builder.build_int_add(old, amount, "")?;
			builder.build_store(count, next)?;
			Ok(old)
		} else {
			Ok(builder.build_atomicrmw(
				AtomicRMWBinOp::Add,
				count,
				amount,
				if release {
					AtomicOrdering::AcquireRelease
				} else {
					AtomicOrdering::Monotonic
				},
			)?)
		}
	}
}
