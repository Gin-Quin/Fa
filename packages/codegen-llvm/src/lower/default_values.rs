//! Declaration defaults stay outside structural function types. Values
//! carrying them own a small adapter whose ordinary entry still uses the
//! callback ABI; its other entry accepts presence flags for named calls.

use fa_analyzer::analysis::Argument;
use fa_analyzer::ids::{NodeId, TypeId};
use inkwell::builder::Builder;
use inkwell::types::{BasicType, FunctionType, StructType};
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{Repr, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	fn default_environment_type(&self) -> StructType<'ctx> {
		self.partial_layout(&[Repr::Pointer, Repr::Function(TypeId::UNKNOWN)])
	}

	pub(super) fn default_environment(
		&self,
		builder: &Builder<'ctx>,
		environment: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let field =
			builder.build_struct_gep(self.default_environment_type(), environment, 4, "")?;
		let original = builder
			.build_load(self.closure_type(), field, "")?
			.into_struct_value();
		Ok(builder
			.build_extract_value(original, 1, "")?
			.into_pointer_value())
	}

	pub(super) fn default_entry(
		&self,
		builder: &Builder<'ctx>,
		environment: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let field =
			builder.build_struct_gep(self.default_environment_type(), environment, 3, "")?;
		Ok(builder
			.build_load(self.pointer_type(), field, "")?
			.into_pointer_value())
	}

	pub(super) fn with_default_entry(
		&mut self,
		value: Val<'ctx>,
		entry: PointerValue<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Function(ty) = value.repr else {
			unreachable!("a function carrying declaration defaults")
		};
		let value = self.own(value, None)?;
		let layout = self.default_environment_type();
		let key = format!("defaults.{}", self.shape_key(value.repr));
		let (drop, copy) =
			self.partial_environment_functions(&key, layout, &[Repr::Pointer, value.repr])?;
		let environment = self.allocate(layout.size_of().expect("a sized adapter"))?;
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
		let field = self.builder.build_struct_gep(layout, environment, 3, "")?;
		self.builder.build_store(field, entry)?;
		let field = self.builder.build_struct_gep(layout, environment, 4, "")?;
		self.unregister(value);
		self.builder
			.build_store(field, value.value.expect("the original function"))?;
		let original = value.value.expect("a function").into_struct_value();
		let entry = self
			.builder
			.build_extract_value(original, 0, "")?
			.into_pointer_value();
		let value = self.make_closure_of(entry, environment, ty)?;
		Ok(self.fresh(value))
	}

	pub(super) fn function_default_entry(
		&mut self,
		function: &super::Function<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let sync = self.default_forwarding_thunk(function, function.value)?;
		let sync = sync.as_global_value().as_pointer_value();
		if !self.tasks.asynchronous || !self.is_async_function(function) {
			return Ok(sync);
		}
		let asynchronous = self.async_instance(function)?;
		let asynchronous = self.default_forwarding_thunk(function, asynchronous)?;
		Ok(self.async_entries(sync, asynchronous.as_global_value().as_pointer_value()))
	}

	pub(super) fn function_with_defaults(
		&mut self,
		value: Val<'ctx>,
		function: &super::Function<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		if !function.params.iter().any(|(_, default)| *default) {
			return Ok(value);
		}
		let entry = self.function_default_entry(function)?;
		self.with_default_entry(value, entry)
	}

	pub(super) fn indirect_type_with_defaults(
		&self,
		params: &[Repr],
		ret: Repr,
		throws: bool,
		defaults: usize,
		requirements: usize,
	) -> FunctionType<'ctx> {
		let original = self.indirect_type_with_requirements(params, ret, throws, requirements);
		if defaults == 0 {
			return original;
		}
		let mut parameters = original.get_param_types();
		parameters.splice(
			params.len() + 1..params.len() + 1,
			std::iter::repeat_n(self.context.bool_type().into(), defaults),
		);
		match original.get_return_type() {
			Some(ret) => ret.fn_type(&parameters, false),
			None => self.context.void_type().fn_type(&parameters, false),
		}
	}

	pub(super) fn call_default_flags(&self, node: NodeId) -> Vec<IntValue<'ctx>> {
		let Some(defaults) = self.analysis.function_defaults.get(&node) else {
			return Vec::new();
		};
		let Some(call) = self.analysis.call(node) else {
			return Vec::new();
		};
		defaults
			.iter()
			.zip(&call.arguments)
			.filter(|(default, _)| **default)
			.map(|(_, argument)| {
				self.context
					.bool_type()
					.const_int(u64::from(!matches!(argument, Argument::Default)), false)
			})
			.collect()
	}
}
