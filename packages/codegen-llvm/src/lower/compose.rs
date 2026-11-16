//! Function composition (`f then g`, `functions/function-composition`):
//! a composition of known functions is a function of its own,
//! `fa.compose.<node>`, with the indirect convention of its type: it calls
//! each step's thunk in turn, passes each result to the next step (a
//! borrowed aggregate through a slot), drops it after, and returns the last
//! one; an error of a step is returned at once. Its value is that function
//! with no environment: no allocation.
//!
//! A composition of values only known at run time (parameters, lambdas,
//! results of calls) is a closure of `fa.compose.runtime.<node>`, whose
//! environment holds each step's closure, owned (a copy, or the temporary
//! itself): the function calls them in turn the same way, and the
//! environment is shared by copied handles and releases its closures once.

use inkwell::builder::Builder;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;

use crate::error::Error;
use crate::lower::environments::ENVIRONMENT_FIELDS;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};

/// The parameters, the result and whether it throws, of a step.
type Signature = (Vec<Repr>, Repr, bool);

/// How a composition's function reaches a step.
#[derive(Copy, Clone)]
enum Callee<'ctx> {
	/// A known function's thunk, called without environment.
	Direct(FunctionValue<'ctx>),
	/// The closure at this position of the composition's environment; an
	/// `async` function's value may be the tagged table of its entries
	/// (`tasks.rs`, `async_value_entry`): its synchronous entry is called.
	Field(u32, bool),
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The value of a composition node: its function, without environment
	/// when every step is a known function, else a closure holding the steps.
	pub(crate) fn composition_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let function = match self.composition_function(node) {
			Ok(function) => function,
			// a step only known at run time
			Err(Error::Unsupported { .. }) => return self.runtime_composition_value(node),
			Err(error) => return Err(error),
		};
		let ty = self.type_of(node);
		let closure = self.closure_type();
		let pair = self.builder.build_insert_value(
			closure.get_undef(),
			function.as_global_value().as_pointer_value(),
			0,
			"",
		)?;
		let pair =
			self.builder
				.build_insert_value(pair, self.pointer_type().const_null(), 1, "")?;
		Ok(Val::new(Repr::Function(ty), pair.into_struct_value()))
	}

	/// The thunk of a step that is a known function of this file (no
	/// captures, not generic, no requirements).
	fn step_thunk(&mut self, step: NodeId) -> Result<FunctionValue<'ctx>, Error> {
		let unsupported =
			|this: &Self| this.unsupported(step.index(), "this step of a composition");
		let mut node = step;
		while let View::Group(inner) = self.view(node) {
			node = inner;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(node) else {
			return Err(unsupported(self));
		};
		let data = self.analysis.symbols.symbol(symbol);
		if data.kind != SymbolKind::Function {
			return Err(unsupported(self));
		}
		let declaration = data.decl_node;
		// a generic function: its instance for the arguments the analyzer
		// recorded at the step (`check/compose.rs`)
		if self.generic_functions.contains_key(&declaration) {
			if self.analysis.instantiation(node).is_none()
				&& self.analysis.instantiation(step).is_none()
			{
				return Err(unsupported(self));
			}
			let at = if self.analysis.instantiation(node).is_some() {
				node
			} else {
				step
			};
			let key = self.instance(declaration, at)?;
			let function = self.functions[&key].clone();
			return self.thunk_of(&function);
		}
		let Some(function) = self.functions.get(&declaration).cloned() else {
			return Err(unsupported(self));
		};
		if function.lambda || !function.captures.is_empty() {
			return Err(unsupported(self));
		}
		self.thunk_of(&function)
	}

	/// `fa.compose.<node>`: the steps called in turn.
	fn composition_function(&mut self, node: NodeId) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!("fa.compose.{}", node.index());
		if let Some(function) = self.module.get_function(&name) {
			return Ok(function);
		}
		let steps = self.composition_steps(node)?;
		let (params, ret, throws) = self.value_signature(self.type_of(node), node)?;
		let mut parts = Vec::with_capacity(steps.len());
		for &step in &steps {
			let thunk = self.step_thunk(step)?;
			let signature = self.value_signature(self.type_of(step), step)?;
			let expected = self.indirect_type(&signature.0, signature.1, signature.2);
			if thunk.get_type() != expected {
				return Err(self.unsupported(step.index(), "this step of a composition"));
			}
			parts.push((Callee::Direct(thunk), signature));
		}
		self.check_step_conventions(node, &params, &parts)?;
		let function_type = self.indirect_type(&params, ret, throws);
		let (function, builder) = self.helper_function(&name, function_type);
		self.build_composition_body(function, &builder, &parts, None, ret, throws)?;
		Ok(function)
	}

	/// The steps of a composition node.
	fn composition_steps(&self, node: NodeId) -> Result<Vec<NodeId>, Error> {
		let steps = self
			.analysis
			.compositions
			.get(&node)
			.cloned()
			.unwrap_or_default();
		if steps.is_empty() {
			return Err(self.unsupported(node.index(), "this composition"));
		}
		Ok(steps)
	}

	/// The first step takes the composition's arguments, and each step's
	/// result goes to the next one as is.
	fn check_step_conventions(
		&self,
		node: NodeId,
		params: &[Repr],
		parts: &[(Callee<'ctx>, Signature)],
	) -> Result<(), Error> {
		if let Some((_, (first, _, _))) = parts.first()
			&& first.as_slice() != params
		{
			return Err(self.unsupported(
				node.index(),
				"this composition (a conversion of its arguments)",
			));
		}
		for window in parts.windows(2) {
			let (_, (_, ret, _)) = &window[0];
			let (_, (next_params, _, _)) = &window[1];
			if next_params.first() != Some(ret) {
				return Err(self.unsupported(
					node.index(),
					"this composition (a conversion between steps)",
				));
			}
		}
		Ok(())
	}

	/// A composition of steps only known at run time: a closure of
	/// `fa.compose.runtime.<node>` whose environment holds the steps'
	/// closures.
	fn runtime_composition_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let steps = self.composition_steps(node)?;
		let ty = self.type_of(node);
		let (params, ret, throws) = self.value_signature(ty, node)?;
		// the steps' values, owned by the environment
		let mut values = Vec::with_capacity(steps.len());
		let mut parts = Vec::with_capacity(steps.len());
		for (position, &step) in steps.iter().enumerate() {
			let value = self.expr(step)?;
			let Repr::Function(step_type) = value.repr else {
				return Err(self.unsupported(step.index(), "this step of a composition"));
			};
			if self.is_generic_function_type(step_type) {
				return Err(self.unsupported(step.index(), "this step of a composition"));
			}
			let signature = self.value_signature(step_type, step)?;
			// only an asynchronous program tags the values of `async` functions
			let is_async = self.tasks.asynchronous && self.is_async_function_type(step_type);
			parts.push((Callee::Field(position as u32, is_async), signature));
			values.push(value);
		}
		self.check_step_conventions(node, &params, &parts)?;
		let mut fields: Vec<BasicTypeEnum<'ctx>> = Vec::new();
		fields.extend(
			values
				.iter()
				.map(|_| BasicTypeEnum::from(self.closure_type())),
		);
		let layout = self.environment_layout(&fields);
		let name = format!("fa.compose.runtime.{}", node.index());
		let function = match self.module.get_function(&name) {
			Some(function) => function,
			None => {
				let function_type = self.indirect_type(&params, ret, throws);
				let (function, builder) = self.helper_function(&name, function_type);
				self.build_composition_body(function, &builder, &parts, Some(layout), ret, throws)?;
				function
			}
		};
		let reprs: Vec<Repr> = values.iter().map(|value| value.repr).collect();
		let (drop, copy) = self.composition_environment_functions(node, layout, &reprs)?;
		let size = layout.size_of().expect("a sized environment");
		let environment = self.allocate(size)?;
		self.initialize_environment(environment, drop, copy, true)?;
		for (index, value) in values.into_iter().enumerate() {
			// a temporary moves into the environment, another value is copied
			let owned = if self.unregister(value) {
				value
			} else {
				self.emit_copy(value)?
			};
			let field = self.builder.build_struct_gep(
				layout,
				environment,
				index as u32 + ENVIRONMENT_FIELDS,
				"",
			)?;
			self.builder
				.build_store(field, owned.value.expect("a closure"))?;
		}
		let closure = self.closure_type();
		let pair = self.builder.build_insert_value(
			closure.get_undef(),
			function.as_global_value().as_pointer_value(),
			0,
			"",
		)?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		Ok(self.fresh(Val::new(Repr::Function(ty), pair.into_struct_value())))
	}

	/// The synchronous entry of the value of an `async` function: the entry
	/// itself, or the first entry of the table its tagged address points
	/// one byte into (`tasks.rs`, `async_value_entry`). A composition calls
	/// its steps' synchronous entries in turn, as JavaScript calls them: an
	/// async step runs blocking.
	fn synchronous_entry(
		&self,
		function: FunctionValue<'ctx>,
		builder: &Builder<'ctx>,
		entry: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let from = builder.get_insert_block().expect("a block");
		let tagged_block = self.context.append_basic_block(function, "tagged");
		let call_block = self.context.append_basic_block(function, "untagged");
		let address = builder.build_ptr_to_int(entry, self.usize, "")?;
		let tag = builder.build_and(address, self.usize.const_int(1, false), "")?;
		let tagged = builder.build_int_compare(
			inkwell::IntPredicate::NE,
			tag,
			self.usize.const_zero(),
			"",
		)?;
		builder.build_conditional_branch(tagged, tagged_block, call_block)?;
		builder.position_at_end(tagged_block);
		// SAFETY: a tagged address is one byte into its table
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
		target.add_incoming(&[(&entry, from), (&sync_entry, tagged_block)]);
		Ok(target.as_basic_value().into_pointer_value())
	}

	/// Whether a function type is `async`.
	fn is_async_function_type(&self, ty: TypeId) -> bool {
		matches!(
			self.types().kind(ty),
			TypeKind::Function(sig) if self.types().sig(sig).is_async
		)
	}

	/// A composition shares its immutable environment across function handles.
	fn composition_environment_functions(
		&self,
		node: NodeId,
		layout: StructType<'ctx>,
		reprs: &[Repr],
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		self.owned_environment_functions(
			&format!("fa.compose.drop.{}", node.index()),
			layout,
			reprs,
		)
	}

	/// The body of a composition's function: each step called in turn, an
	/// error returned at once, each result lent to the next step and
	/// dropped after. `layout` is the environment's, for steps it holds.
	fn build_composition_body(
		&self,
		function: FunctionValue<'ctx>,
		builder: &Builder<'ctx>,
		parts: &[(Callee<'ctx>, Signature)],
		layout: Option<StructType<'ctx>>,
		ret: Repr,
		throws: bool,
	) -> Result<(), Error> {
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		let error_type = self.pointer_type();
		// the storage the caller gives for the result (`return_destinations.rs`,
		// `collection_storage.rs`): a slot, which the last step fills, or a
		// buffer, which it may use
		let own_slot = self.uses_return_slot(ret).then(|| {
			function
				.get_nth_param(1)
				.expect("a result slot")
				.into_pointer_value()
		});
		let own_buffer = self.uses_collection_return(ret).then(|| {
			(
				function.get_nth_param(1).expect("a buffer"),
				function.get_nth_param(2).expect("the size of a buffer"),
			)
		});
		if own_slot.is_some() {
			self.add_return_slot_attribute(function, ret, throws, 1);
		}
		let result_parameters =
			usize::from(own_slot.is_some()) + 2 * usize::from(own_buffer.is_some());
		// the composition's own result shape
		let finish = |this: &Self,
		              value: Option<BasicValueEnum<'ctx>>,
		              error: Option<PointerValue<'ctx>>|
		 -> Result<(), Error> {
			let error = error.unwrap_or_else(|| error_type.const_null());
			match (ret.has_value() && own_slot.is_none(), throws) {
				(true, false) => {
					builder.build_return(Some(&value.expect("a result")))?;
				}
				(false, false) => {
					builder.build_return(None)?;
				}
				(true, true) => {
					let shape = this
						.context
						.struct_type(&[this.llvm_type(ret), error_type.into()], false);
					let value = value.unwrap_or_else(|| this.llvm_type(ret).const_zero());
					let pair = builder.build_insert_value(shape.get_undef(), value, 0, "")?;
					let pair = builder.build_insert_value(pair, error, 1, "")?;
					builder.build_return(Some(&pair))?;
				}
				(false, true) => {
					builder.build_return(Some(&error))?;
				}
			}
			Ok(())
		};
		let environment = function
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let given: Vec<BasicMetadataValueEnum<'ctx>> = function
			.get_param_iter()
			.skip(1 + result_parameters)
			.map(BasicMetadataValueEnum::from)
			.collect();
		let mut previous: Option<(Repr, BasicValueEnum<'ctx>)> = None;
		for (index, (callee, (step_params, step_ret, step_throws))) in parts.iter().enumerate() {
			let last = index + 1 == parts.len();
			// the step's entry and environment
			let (entry, step_environment) = match *callee {
				Callee::Direct(thunk) => (
					thunk.as_global_value().as_pointer_value(),
					self.pointer_type().const_null(),
				),
				Callee::Field(position, is_async) => {
					let layout = layout.expect("the layout of the environment");
					let field = builder.build_struct_gep(
						layout,
						environment,
						position + ENVIRONMENT_FIELDS,
						"",
					)?;
					let closure = builder
						.build_load(self.closure_type(), field, "")?
						.into_struct_value();
					let entry = builder
						.build_extract_value(closure, 0, "")?
						.into_pointer_value();
					let entry = if is_async {
						self.synchronous_entry(function, builder, entry)?
					} else {
						entry
					};
					(
						entry,
						builder
							.build_extract_value(closure, 1, "")?
							.into_pointer_value(),
					)
				}
			};
			let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = vec![step_environment.into()];
			// the storage of the step's result: the composition's own for the
			// last step, else a slot of this function and no buffer
			let step_slot = if self.uses_return_slot(*step_ret) {
				let slot = match own_slot {
					Some(slot) if last => slot,
					_ => builder.build_alloca(self.llvm_type(*step_ret), "step_result")?,
				};
				arguments.push(slot.into());
				Some(slot)
			} else {
				None
			};
			if self.uses_collection_return(*step_ret) {
				match own_buffer {
					Some((buffer, bytes)) if last => {
						arguments.push(buffer.into());
						arguments.push(bytes.into());
					}
					_ => {
						arguments.push(self.pointer_type().const_null().into());
						arguments.push(self.usize.const_zero().into());
					}
				}
			}
			match &previous {
				None => arguments.extend(given.iter().copied()),
				Some((repr, value)) => {
					if self.borrows_parameter(*repr) {
						let slot = builder.build_alloca(self.llvm_type(*repr), "step")?;
						builder.build_store(slot, *value)?;
						arguments.push(slot.into());
					} else {
						arguments.push((*value).into());
					}
				}
			}
			let step_type = self.indirect_type(step_params, *step_ret, *step_throws);
			let call = builder.build_indirect_call(step_type, entry, &arguments, "")?;
			call.set_call_convention(FAST_CALLING_CONVENTION);
			if step_slot.is_some() {
				self.add_return_slot_call_attribute(call, *step_ret, *step_throws, 1);
			}
			let result = call.try_as_basic_value().basic();
			// the step's result and error
			let (value, error) = match (step_ret.has_value() && step_slot.is_none(), *step_throws) {
				(true, false) => (result, None),
				(false, false) => (None, None),
				(true, true) => {
					let pair = result.expect("a result").into_struct_value();
					(
						Some(builder.build_extract_value(pair, 0, "")?),
						Some(
							builder
								.build_extract_value(pair, 1, "")?
								.into_pointer_value(),
						),
					)
				}
				(false, true) => (None, Some(result.expect("an error").into_pointer_value())),
			};
			// the previous result, lent to this step, is dropped
			if let Some((repr, value)) = previous.take()
				&& self.needs_drop(repr)
			{
				let drop = self.drop_function(repr)?;
				builder.build_call(drop, &[value.into()], "")?;
			}
			if let Some(error) = error {
				let failed = self.context.append_basic_block(function, "failed");
				let next = self.context.append_basic_block(function, "next");
				let is_error = builder.build_is_not_null(error, "")?;
				builder.build_conditional_branch(is_error, failed, next)?;
				// a step that failed returns no value to drop
				builder.position_at_end(failed);
				finish(self, None, Some(error))?;
				builder.position_at_end(next);
			}
			// a result built in a slot is read once the step succeeded (the
			// last one's is already where the caller reads it)
			let value = match step_slot {
				Some(slot) if own_slot != Some(slot) => {
					Some(builder.build_load(self.llvm_type(*step_ret), slot, "")?)
				}
				Some(_) => None,
				None => value,
			};
			if last {
				if let (Some(slot), Some(value)) = (own_slot, value) {
					builder.build_store(slot, value)?;
				}
				finish(self, value, None)?;
			} else if let Some(value) = value {
				previous = Some((*step_ret, value));
			}
		}
		Ok(())
	}
}
