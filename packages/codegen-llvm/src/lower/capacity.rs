//! Storage observations and hints; reservations never alter the contents.

use fa_analyzer::analysis::{Argument, CallInfo};
use fa_analyzer::ids::NodeId;
use inkwell::IntPredicate;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn storage_capacity(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let (function, pointer) = match value.repr {
			Repr::Str => {
				let slot = self.alloca(Repr::Str, "text_capacity")?;
				self.builder
					.build_store(slot.pointer, value.value.expect("a string"))?;
				("fa_string_storage_capacity", slot.pointer)
			}
			Repr::Array(_) => (
				"fa_array_capacity",
				value.value.expect("an array").into_pointer_value(),
			),
			Repr::Growing(_) => (
				"fa_segmented_capacity",
				value.value.expect("an array").into_pointer_value(),
			),
			Repr::Map(_) | Repr::Set(_) => (
				"fa_table_capacity",
				value.value.expect("a table").into_pointer_value(),
			),
			Repr::Pool(_) | Repr::Columnar(_) => (
				"fa_pool_capacity",
				value.value.expect("a pool").into_pointer_value(),
			),
			_ => return Err(Error::Llvm("capacity of a non-collection".into())),
		};
		let capacity = self
			.call_value(self.runtime.function(function)?, &[pointer.into()])?
			.into_int_value();
		let capacity = self.builder.build_int_z_extend_or_bit_cast(
			capacity,
			self.context.i64_type(),
			"capacity",
		)?;
		Ok(Val::new(Repr::Int(IntRepr::I64), capacity))
	}

	pub(crate) fn reserve_call(
		&mut self,
		node: NodeId,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let slot = self.receiver_place(node)?;
		let receiver = match slot {
			Some(slot) => self.load(slot)?,
			None => self.receiver(node)?,
		};
		if receiver.repr == Repr::Never {
			return Ok(receiver);
		}
		let Some(Argument::Given(argument)) = info.arguments.first() else {
			return Err(self.unsupported(node.index(), "reserve without an argument"));
		};
		let additional = self.expr(*argument)?;
		if additional.repr == Repr::Never {
			return Ok(additional);
		}
		let additional = self
			.convert(additional, Repr::Int(IntRepr::I64), node.index())?
			.int();
		if receiver.repr == Repr::Str {
			let temporary = slot.is_none();
			let target = match slot {
				Some(slot) => slot,
				None => {
					let owned = self.take(receiver, None)?;
					let target = self.alloca(Repr::Str, "reserved_string")?;
					self.builder
						.build_store(target.pointer, owned.value.expect("a string"))?;
					target
				}
			};
			self.call_runtime(
				"fa_string_reserve",
				&[target.pointer.into(), additional.into()],
			)?;
			if temporary {
				self.fresh(self.load(target)?);
			}
		} else {
			let receiver = match slot {
				Some(slot) => self.load(slot)?,
				None => receiver,
			};
			self.reserve_storage(receiver, additional, node)?;
		}
		Ok(Val::void())
	}

	pub(crate) fn reserve_storage(
		&self,
		receiver: Val<'ctx>,
		additional: IntValue<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		let pointer = receiver.value.expect("a collection").into_pointer_value();
		match receiver.repr {
			Repr::Array(item) => self.call_runtime(
				"fa_array_reserve_additional",
				&[
					pointer.into(),
					additional.into(),
					self.item_size_const(self.item_repr(item, node)?).into(),
				],
			),
			Repr::Growing(item) => {
				let item = self.item_repr(item, node)?;
				self.call_runtime(
					"fa_segmented_reserve",
					&[
						pointer.into(),
						additional.into(),
						self.item_size_const(item).into(),
						self.segment_k(item).into(),
					],
				)
			}
			Repr::Map(_) | Repr::Set(_) => {
				let entry = self
					.table_of(receiver.repr)?
					.entry
					.size_of()
					.expect("an entry");
				self.call_runtime(
					"fa_table_reserve_additional",
					&[pointer.into(), additional.into(), entry.into()],
				)
			}
			Repr::Pool(_) | Repr::Columnar(_) => {
				self.call_runtime("fa_pool_reserve", &[pointer.into(), additional.into()])
			}
			_ => Err(self.unsupported(node.index(), "reserve on this value")),
		}
	}

	pub(crate) fn string_from(
		&mut self,
		node: NodeId,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some((source, capacity)) = self.array_from_arguments(info)? else {
			return Ok(Val::never());
		};
		let slot = self.alloca(Repr::Str, "string_from")?;
		self.builder
			.build_store(slot.pointer, source.value.expect("a string"))?;
		if let Some(capacity) = capacity {
			let capacity = self
				.convert(capacity, Repr::Int(IntRepr::I64), node.index())?
				.int();
			let (_, length) = self.string_parts(source)?;
			let length =
				self.builder
					.build_int_z_extend_or_bit_cast(length, self.context.i64_type(), "")?;
			let grows = self
				.builder
				.build_int_compare(IntPredicate::SGT, capacity, length, "")?;
			let extra = self.builder.build_int_sub(capacity, length, "")?;
			let extra = self
				.builder
				.build_select(grows, extra, self.context.i64_type().const_zero(), "")?
				.into_int_value();
			self.call_runtime("fa_string_reserve", &[slot.pointer.into(), extra.into()])?;
		}
		Ok(self.fresh(self.load(slot)?))
	}
}
