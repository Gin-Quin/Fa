//! Function composition (`f then g`, `functions/function-composition`):
//! a composition of known functions is a function of its own,
//! `fa.compose.<node>`, with the indirect convention of its type: it calls
//! each step's thunk in turn, passes each result to the next step (a
//! borrowed aggregate through a slot), drops it after, and returns the last
//! one; an error of a step is returned at once. Its value is that function
//! with no environment: no allocation.

use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue};

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};

impl<'ctx> Lowering<'_, 'ctx> {
	/// The value of a composition node: its function, without environment.
	pub(crate) fn composition_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let function = self.composition_function(node)?;
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
		let steps = self
			.analysis
			.compositions
			.get(&node)
			.cloned()
			.unwrap_or_default();
		if steps.is_empty() {
			return Err(self.unsupported(node.index(), "this composition"));
		}
		let (params, ret, throws) = self.value_signature(self.type_of(node), node)?;
		let mut parts = Vec::with_capacity(steps.len());
		for &step in &steps {
			let thunk = self.step_thunk(step)?;
			let signature = self.value_signature(self.type_of(step), step)?;
			let expected = self.indirect_type(&signature.0, signature.1, signature.2);
			if thunk.get_type() != expected {
				return Err(self.unsupported(step.index(), "this step of a composition"));
			}
			parts.push((thunk, signature));
		}
		// a step's result goes to the next one as is
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
		let function_type = self.indirect_type(&params, ret, throws);
		let (function, builder) = self.helper_function(&name, function_type);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		let error_type = self.pointer_type();
		// the composition's own result shape
		let finish = |this: &Self,
		              value: Option<BasicValueEnum<'ctx>>,
		              error: Option<inkwell::values::PointerValue<'ctx>>|
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
		let given: Vec<BasicMetadataValueEnum<'ctx>> = function
			.get_param_iter()
			.skip(1)
			.map(BasicMetadataValueEnum::from)
			.collect();
		let mut previous: Option<(Repr, BasicValueEnum<'ctx>)> = None;
		for (index, (thunk, (step_params, step_ret, step_throws))) in parts.iter().enumerate() {
			let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> =
				vec![self.pointer_type().const_null().into()];
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
			let _ = step_params;
			let call = builder.build_call(*thunk, &arguments, "")?;
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
				builder.position_at_end(failed);
				if let Some(value) = value
					&& self.needs_drop(*step_ret)
				{
					// a step that failed returns no value to drop
					let _ = value;
				}
				finish(self, None, Some(error))?;
				builder.position_at_end(next);
			}
			if index + 1 == parts.len() {
				finish(self, value, None)?;
			} else if let Some(value) = value {
				previous = Some((*step_ret, value));
			}
		}
		Ok(function)
	}
}
