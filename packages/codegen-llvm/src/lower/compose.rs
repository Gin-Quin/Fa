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
//! environment's drop and copy drop and copy the closures.

use inkwell::builder::Builder;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};

/// The parameters, the result and whether it throws, of a step.
type Signature = (Vec<Repr>, Repr, bool);

/// How a composition's function reaches a step.
#[derive(Copy, Clone)]
enum Callee<'ctx> {
	/// A known function's thunk, called without environment.
	Direct(FunctionValue<'ctx>),
	/// The closure at this position of the composition's environment.
	Field(u32),
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
		let Some(function) = self.functions.get(&data.decl_node).cloned() else {
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
			if self.is_generic_function_type(step_type) || self.is_async_function_type(step_type) {
				return Err(self.unsupported(step.index(), "this step of a composition"));
			}
			let signature = self.value_signature(step_type, step)?;
			parts.push((Callee::Field(position as u32), signature));
			values.push(value);
		}
		self.check_step_conventions(node, &params, &parts)?;
		let pointer = self.pointer_type();
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![pointer.into(), pointer.into()];
		fields.extend(
			values
				.iter()
				.map(|_| BasicTypeEnum::from(self.closure_type())),
		);
		let layout = self.context.struct_type(&fields, false);
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
		for (index, function) in [drop, copy].into_iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(layout, environment, index as u32, "")?;
			self.builder
				.build_store(field, function.as_global_value().as_pointer_value())?;
		}
		for (index, value) in values.into_iter().enumerate() {
			// a temporary moves into the environment, another value is copied
			let owned = if self.unregister(value) {
				value
			} else {
				self.emit_copy(value)?
			};
			let field = self
				.builder
				.build_struct_gep(layout, environment, index as u32 + 2, "")?;
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

	/// Whether a function type is `async`.
	fn is_async_function_type(&self, ty: TypeId) -> bool {
		matches!(
			self.types().kind(ty),
			TypeKind::Function(sig) if self.types().sig(sig).is_async
		)
	}

	/// The drop and the copy of the environment of a runtime composition:
	/// the closures it holds, dropped or copied.
	fn composition_environment_functions(
		&self,
		node: NodeId,
		layout: StructType<'ctx>,
		reprs: &[Repr],
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let drop_name = format!("fa.compose.drop.{}", node.index());
		let copy_name = format!("fa.compose.copy.{}", node.index());
		if let (Some(drop), Some(copy)) = (
			self.module.get_function(&drop_name),
			self.module.get_function(&copy_name),
		) {
			return Ok((drop, copy));
		}
		let pointer = self.pointer_type();
		let size = layout.size_of().expect("a sized environment");
		let align = self.usize.const_int(16, false);
		let drop = self.module.add_function(
			&drop_name,
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(drop, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(drop, "entry"));
		let environment = drop
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		for (index, repr) in reprs.iter().enumerate().rev() {
			let field = builder.build_struct_gep(layout, environment, index as u32 + 2, "")?;
			let closure = builder.build_load(self.closure_type(), field, "")?;
			builder.build_call(self.drop_function(*repr)?, &[closure.into()], "")?;
		}
		let size_value = builder.build_int_cast(size, self.usize, "")?;
		builder.build_call(
			self.runtime.free()?,
			&[environment.into(), size_value.into(), align.into()],
			"",
		)?;
		builder.build_return(None)?;

		let copy = self.module.add_function(
			&copy_name,
			pointer.fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(copy, &["nounwind"]);
		builder.position_at_end(self.context.append_basic_block(copy, "entry"));
		let environment = copy
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let size_value = builder.build_int_cast(size, self.usize, "")?;
		let copied = builder
			.build_call(
				self.runtime.alloc()?,
				&[size_value.into(), align.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an environment")
			.into_pointer_value();
		for index in 0..2 {
			let from = builder.build_struct_gep(layout, environment, index, "")?;
			let to = builder.build_struct_gep(layout, copied, index, "")?;
			let loaded = builder.build_load(pointer, from, "")?;
			builder.build_store(to, loaded)?;
		}
		for (index, repr) in reprs.iter().enumerate() {
			let from = builder.build_struct_gep(layout, environment, index as u32 + 2, "")?;
			let to = builder.build_struct_gep(layout, copied, index as u32 + 2, "")?;
			let closure = builder.build_load(self.closure_type(), from, "")?;
			let closure = builder
				.build_call(self.copy_function(*repr)?, &[closure.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("a copy");
			builder.build_store(to, closure)?;
		}
		builder.build_return(Some(&copied))?;
		Ok((drop, copy))
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
		// the composition's own result shape
		let finish = |this: &Self,
		              value: Option<BasicValueEnum<'ctx>>,
		              error: Option<PointerValue<'ctx>>|
		 -> Result<(), Error> {
			let error = error.unwrap_or_else(|| error_type.const_null());
			match (ret.has_value(), throws) {
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
			.skip(1)
			.map(BasicMetadataValueEnum::from)
			.collect();
		let mut previous: Option<(Repr, BasicValueEnum<'ctx>)> = None;
		for (index, (callee, (step_params, step_ret, step_throws))) in parts.iter().enumerate() {
			// the step's entry and environment
			let (entry, step_environment) = match *callee {
				Callee::Direct(thunk) => (
					thunk.as_global_value().as_pointer_value(),
					self.pointer_type().const_null(),
				),
				Callee::Field(position) => {
					let layout = layout.expect("the layout of the environment");
					let field = builder.build_struct_gep(layout, environment, position + 2, "")?;
					let closure = builder
						.build_load(self.closure_type(), field, "")?
						.into_struct_value();
					(
						builder
							.build_extract_value(closure, 0, "")?
							.into_pointer_value(),
						builder
							.build_extract_value(closure, 1, "")?
							.into_pointer_value(),
					)
				}
			};
			let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = vec![step_environment.into()];
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
			let result = call.try_as_basic_value().basic();
			// the step's result and error
			let (value, error) = match (step_ret.has_value(), *step_throws) {
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
			if index + 1 == parts.len() {
				finish(self, value, None)?;
			} else if let Some(value) = value {
				previous = Some((*step_ret, value));
			}
		}
		Ok(())
	}
}
