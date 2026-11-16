//! Immutable owned callable environments share their payload. Borrowed
//! frame environments retain their separate no-op lifetime operations.

use inkwell::builder::Builder;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::{AtomicOrdering, AtomicRMWBinOp, IntPredicate};

use crate::error::Error;
use crate::target::Target;

use super::Lowering;
use super::values::Repr;

pub(super) const ENVIRONMENT_FIELDS: u32 = 3;

impl<'ctx> Lowering<'_, 'ctx> {
	/// The common prefix is `{ ptr drop, ptr retain, usize references }`.
	pub(crate) fn environment_header(&self) -> StructType<'ctx> {
		self.environment_layout(&[])
	}

	pub(super) fn environment_layout(&self, payload: &[BasicTypeEnum<'ctx>]) -> StructType<'ctx> {
		let pointer = self.pointer_type();
		let mut fields = vec![pointer.into(), pointer.into(), self.usize.into()];
		fields.extend_from_slice(payload);
		self.context.struct_type(&fields, false)
	}

	pub(super) fn initialize_environment(
		&self,
		environment: PointerValue<'ctx>,
		drop: FunctionValue<'ctx>,
		retain: FunctionValue<'ctx>,
		owned: bool,
	) -> Result<(), Error> {
		let header = self.environment_header();
		for (index, function) in [drop, retain].into_iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(header, environment, index as u32, "")?;
			self.builder
				.build_store(field, function.as_global_value().as_pointer_value())?;
		}
		let count = self.builder.build_struct_gep(header, environment, 2, "")?;
		self.builder
			.build_store(count, self.usize.const_int(u64::from(owned), false))?;
		Ok(())
	}

	/// Threaded native owners use atomic reference counts. Current wasm
	/// programs are single-threaded and need no atomic target feature.
	fn change_environment_count(
		&self,
		builder: &Builder<'ctx>,
		environment: PointerValue<'ctx>,
		retain: bool,
	) -> Result<IntValue<'ctx>, Error> {
		let count = builder.build_struct_gep(self.environment_header(), environment, 2, "")?;
		let one = self.usize.const_int(1, false);
		if self.target != Target::Wasm {
			return Ok(builder.build_atomicrmw(
				if retain {
					AtomicRMWBinOp::Add
				} else {
					AtomicRMWBinOp::Sub
				},
				count,
				one,
				if retain {
					AtomicOrdering::Monotonic
				} else {
					AtomicOrdering::AcquireRelease
				},
			)?);
		}
		let previous = builder.build_load(self.usize, count, "")?.into_int_value();
		let next = if retain {
			builder.build_int_add(previous, one, "")?
		} else {
			builder.build_int_sub(previous, one, "")?
		};
		builder.build_store(count, next)?;
		Ok(previous)
	}

	fn retain_environment_function(&self) -> Result<FunctionValue<'ctx>, Error> {
		if let Some(function) = self.module.get_function("fa.retain_environment") {
			return Ok(function);
		}
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			"fa.retain_environment",
			pointer.fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let environment = function
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		self.change_environment_count(&builder, environment, true)?;
		builder.build_return(Some(&environment))?;
		Ok(function)
	}

	/// One reference initially owns the payload; only the last release
	/// destroys its fields. Copying any callable kind uses the same retain.
	pub(super) fn owned_environment_functions(
		&self,
		name: &str,
		layout: StructType<'ctx>,
		reprs: &[Repr],
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let retain = self.retain_environment_function()?;
		let pointer = self.pointer_type();
		let drop = self.module.add_function(
			name,
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(drop, &["nounwind"]);
		let builder = self.context.create_builder();
		let entry = self.context.append_basic_block(drop, "entry");
		let last = self.context.append_basic_block(drop, "last_owner");
		let done = self.context.append_basic_block(drop, "done");
		builder.position_at_end(entry);
		let environment = drop
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let previous = self.change_environment_count(&builder, environment, false)?;
		let is_last = builder.build_int_compare(
			IntPredicate::EQ,
			previous,
			self.usize.const_int(1, false),
			"",
		)?;
		builder.build_conditional_branch(is_last, last, done)?;
		builder.position_at_end(last);
		for (index, &repr) in reprs.iter().enumerate().rev() {
			if self.needs_drop(repr) {
				let field = builder.build_struct_gep(
					layout,
					environment,
					index as u32 + ENVIRONMENT_FIELDS,
					"",
				)?;
				let value = builder.build_load(self.llvm_type(repr), field, "")?;
				builder.build_call(self.drop_function(repr)?, &[value.into()], "")?;
			}
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
		Ok((drop, retain))
	}
}
