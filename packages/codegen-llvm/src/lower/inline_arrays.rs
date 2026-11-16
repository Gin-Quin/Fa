//! Inline lists use the aggregate ABI (`Repr::Tuple`) with an array of
//! slots and a narrow initialized-prefix length. Temporary Array views are
//! stack headers only: they borrow the slots and never own or grow them.

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{NodeList, View};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::{BasicType, IntType, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};

use super::values::{IntRepr, Repr, Scalar, Val};
use super::{Lowering, Slot};
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn inline_info(&self, ty: TypeId) -> Option<(TypeId, u32)> {
		let (item, mut capacity) = self.types().inline_array_types(ty)?;
		if let fa_analyzer::types::TypeKind::Param(param) = self.types().kind(capacity) {
			capacity = *self.type_bindings.borrow().get(&param)?;
		}
		let int = self.types().as_int(capacity)?;
		Some((item, u32::try_from(int.min).ok()?))
	}

	fn inline_length_type(&self, capacity: u32) -> IntType<'ctx> {
		if capacity <= u8::MAX.into() {
			self.context.i8_type()
		} else if capacity <= u16::MAX.into() {
			self.context.i16_type()
		} else {
			self.context.i32_type()
		}
	}

	pub(super) fn inline_type(&self, ty: TypeId) -> StructType<'ctx> {
		let (item, capacity) = self.inline_info(ty).expect("an inline array");
		if capacity == 0 {
			return self.context.struct_type(
				&[
					self.context.i8_type().array_type(0).into(),
					self.context.i8_type().into(),
				],
				false,
			);
		}
		let item = self
			.item_repr(item, NodeId::new(0))
			.expect("an inline item layout");
		self.context.struct_type(
			&[
				self.llvm_type(item).array_type(capacity).into(),
				self.inline_length_type(capacity).into(),
			],
			false,
		)
	}

	fn inline_length_pointer(
		&self,
		builder: &Builder<'ctx>,
		slot: Slot<'ctx>,
		ty: TypeId,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(builder.build_struct_gep(self.inline_type(ty), slot.pointer, 1, "inline.length")?)
	}

	pub(super) fn inline_length(&self, value: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let length = self
			.builder
			.build_extract_value(
				value.value.expect("an inline array").into_struct_value(),
				1,
				"inline.length",
			)?
			.into_int_value();
		Ok(self
			.builder
			.build_int_z_extend(length, self.context.i64_type(), "")?)
	}

	pub(super) fn set_inline_length(
		&self,
		slot: Slot<'ctx>,
		ty: TypeId,
		length: IntValue<'ctx>,
	) -> Result<(), Error> {
		let (_, capacity) = self.inline_info(ty).expect("an inline array");
		let pointer = self.inline_length_pointer(&self.builder, slot, ty)?;
		let length = self.builder.build_int_truncate_or_bit_cast(
			length,
			self.inline_length_type(capacity),
			"",
		)?;
		self.builder.build_store(pointer, length)?;
		Ok(())
	}

	pub(super) fn inline_item(
		&self,
		builder: &Builder<'ctx>,
		slot: Slot<'ctx>,
		ty: TypeId,
		index: IntValue<'ctx>,
	) -> Result<Slot<'ctx>, Error> {
		let (item, capacity) = self.inline_info(ty).expect("an inline array");
		let repr = self.item_repr(item, NodeId::new(0))?;
		let slots =
			builder.build_struct_gep(self.inline_type(ty), slot.pointer, 0, "inline.slots")?;
		let pointer = unsafe {
			builder.build_in_bounds_gep(
				self.llvm_type(repr).array_type(capacity),
				slots,
				&[self.context.i64_type().const_zero(), index],
				"inline.item",
			)?
		};
		Ok(Slot { pointer, repr })
	}

	pub(super) fn inline_literal(
		&mut self,
		node: NodeId,
		ty: TypeId,
		items: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let slot = match self.destination_for(node, Repr::Tuple(ty)) {
			Some(slot) => slot,
			None => self.alloca(Repr::Tuple(ty), "inline.array")?,
		};
		self.set_inline_length(slot, ty, self.context.i64_type().const_zero())?;
		// Completed items remain cleanup owners until the complete list is
		// published, including when a later item's constructor throws.
		let mut completed = Vec::new();
		for (index, node) in items.iter().enumerate() {
			let position = self.context.i64_type().const_int(index as u64, false);
			let item = self.inline_item(&self.builder, slot, ty, position)?;
			let value = self.expr_with_destination(node, Some(item))?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			let value = self.owned_as(value, item.repr, Some(node))?;
			completed.push(self.fresh(value));
			self.store(item, value, node)?;
			self.set_inline_length(
				slot,
				ty,
				self.context.i64_type().const_int(index as u64 + 1, false),
			)?;
		}
		for value in completed {
			self.take(value, None)?;
		}
		Ok(self.fresh(self.load(slot)?))
	}

	/// A non-owning stack header for existing read-only array algorithms.
	pub(super) fn inline_view_at(&self, slot: Slot<'ctx>, ty: TypeId) -> Result<Val<'ctx>, Error> {
		let (item, capacity) = self.inline_info(ty).expect("an inline array");
		let length_pointer = self.inline_length_pointer(&self.builder, slot, ty)?;
		let length = self
			.builder
			.build_load(self.inline_length_type(capacity), length_pointer, "")?
			.into_int_value();
		let length = self
			.builder
			.build_int_z_extend(length, self.context.i64_type(), "")?;
		let slots =
			self.builder
				.build_struct_gep(self.inline_type(ty), slot.pointer, 0, "inline.slots")?;
		let header = self.alloca_type(self.header_type().into(), "inline.view")?;
		for (index, value) in [
			BasicValueEnum::from(slots),
			length.into(),
			self.context
				.i64_type()
				.const_int(capacity as u64, false)
				.into(),
		]
		.into_iter()
		.enumerate()
		{
			let pointer =
				self.builder
					.build_struct_gep(self.header_type(), header, index as u32, "")?;
			self.builder.build_store(pointer, value)?;
		}
		Ok(Val::new(Repr::Array(item), header))
	}

	pub(super) fn inline_view(&self, value: Val<'ctx>, ty: TypeId) -> Result<Val<'ctx>, Error> {
		let slot = self.alloca(value.repr, "inline.value")?;
		self.builder
			.build_store(slot.pointer, value.value.expect("an inline array"))?;
		self.inline_view_at(slot, ty)
	}

	pub(super) fn inline_element(
		&mut self,
		slot: Slot<'ctx>,
		ty: TypeId,
		index: NodeId,
		node: NodeId,
	) -> Result<Option<Slot<'ctx>>, Error> {
		let value = self.expr(index)?;
		if value.repr == Repr::Never {
			return Ok(None);
		}
		let index = self
			.convert(value, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let length = self.inline_length(self.load(slot)?)?;
		let position = self.counted_from_end(index, length)?;
		if self.flags(node).contains(NodeFlags::INDEX_CHECK) {
			let outside =
				self.builder
					.build_int_compare(IntPredicate::UGE, position, length, "")?;
			self.guard(outside, |this| {
				this.raise_runtime("fa_error_index", &[index.into(), length.into()])
			})?;
		}
		Ok(Some(self.inline_item(&self.builder, slot, ty, position)?))
	}

	/// Returns a type without evaluating its receiver. The place is evaluated
	/// once by inline_method, including indexed field receivers.
	pub(super) fn inline_receiver_type(&self, call: NodeId) -> Option<TypeId> {
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let ty = if members.len() == 1 {
			self.type_of(target)
		} else {
			self.type_of(members.get(members.len() - 2)?)
		};
		self.inline_info(ty).map(|_| ty)
	}

	pub(super) fn inline_method(
		&mut self,
		node: NodeId,
		ty: TypeId,
		path: &str,
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
		let slot = match self.receiver_place(node)? {
			Some(slot) => slot,
			None => {
				let value = self.receiver(node)?;
				let slot = self.alloca(value.repr, "inline.receiver")?;
				self.builder
					.build_store(slot.pointer, value.value.expect("an inline array"))?;
				slot
			}
		};
		let mut values = Vec::new();
		for &argument in arguments {
			let value = self.expr(argument)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			values.push(value);
		}
		let (item_type, capacity) = self.inline_info(ty).expect("an inline array");
		let item = self.item_repr(item_type, node)?;
		let length = self.inline_length(self.load(slot)?)?;
		let i64 = self.context.i64_type();
		match path {
			"Array#push" => {
				if self.flags(node).contains(NodeFlags::CAPACITY_CHECK) {
					let full = self.builder.build_int_compare(
						IntPredicate::UGE,
						length,
						i64.const_int(capacity as u64, false),
						"inline.full",
					)?;
					self.guard(full, |this| this.raise(this.builtin_error("CapacityError")))?;
				}
				let value = self.owned_as(values[0], item, Some(arguments[0]))?;
				let destination = self.inline_item(&self.builder, slot, ty, length)?;
				self.store(destination, value, node)?;
				let length = self
					.builder
					.build_int_add(length, i64.const_int(1, false), "")?;
				self.set_inline_length(slot, ty, length)?;
				Ok(Val::void())
			}
			"Array#get" => {
				let view = self.inline_view_at(slot, ty)?;
				self.array_get(view, values[0], node)
			}
			"Array#clear" => {
				self.for_each_index(length, |this, index| {
					let item_slot = this.inline_item(&this.builder, slot, ty, index)?;
					let value = this.load(item_slot)?;
					this.call_drop(&this.builder, item, value.value.expect("an item"))
				})?;
				self.set_inline_length(slot, ty, i64.const_zero())?;
				Ok(Val::void())
			}
			"Array#pop" => {
				let scalar = Scalar::of(item)
					.ok_or_else(|| self.unsupported(node.index(), "this inline item optional"))?;
				let result = self.alloca(Repr::Optional(scalar), "inline.popped")?;
				self.builder
					.build_store(result.pointer, super::zero_of(self.llvm_type(result.repr)))?;
				let present = self.builder.build_int_compare(
					IntPredicate::NE,
					length,
					i64.const_zero(),
					"",
				)?;
				let body = self.append_block("inline.pop");
				let end = self.append_block("inline.pop.end");
				self.builder.build_conditional_branch(present, body, end)?;
				self.builder.position_at_end(body);
				let last = self
					.builder
					.build_int_sub(length, i64.const_int(1, false), "")?;
				let item_slot = self.inline_item(&self.builder, slot, ty, last)?;
				let value = self.load(item_slot)?;
				let value = self.convert(value, result.repr, node.index())?;
				self.builder
					.build_store(result.pointer, value.value.expect("a popped item"))?;
				self.set_inline_length(slot, ty, last)?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				Ok(self.fresh(self.load(result)?))
			}
			"Array#delete" | "Array#take" => {
				let index = self
					.convert(values[0], Repr::Int(IntRepr::I64), node.index())?
					.int();
				let position = self.counted_from_end(index, length)?;
				if self.flags(node).contains(NodeFlags::INDEX_CHECK) {
					let outside =
						self.builder
							.build_int_compare(IntPredicate::UGE, position, length, "")?;
					self.guard(outside, |this| {
						this.raise_runtime("fa_error_index", &[index.into(), length.into()])
					})?;
				}
				let destination = self.inline_item(&self.builder, slot, ty, position)?;
				let removed = self.load(destination)?;
				if path == "Array#delete" {
					self.call_drop(&self.builder, item, removed.value.expect("a removed item"))?;
				}
				let after = self
					.builder
					.build_int_add(position, i64.const_int(1, false), "")?;
				let count = self.builder.build_int_sub(length, after, "")?;
				let source = self.inline_item(&self.builder, slot, ty, after)?;
				let bytes = self
					.builder
					.build_int_mul(count, self.item_size(item)?, "")?;
				self.builder
					.build_memmove(destination.pointer, 1, source.pointer, 1, bytes)?;
				let length = self
					.builder
					.build_int_sub(length, i64.const_int(1, false), "")?;
				self.set_inline_length(slot, ty, length)?;
				Ok(if path == "Array#take" {
					self.fresh(removed)
				} else {
					Val::void()
				})
			}
			_ => Err(self.unsupported(node.index(), "this inline array method")),
		}
	}

	/// Copy and destruction visit exactly the initialized prefix. The same
	/// helper works inside fields, parameters, results and collection slots.
	pub(super) fn inline_ownership(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		ty: TypeId,
		value: BasicValueEnum<'ctx>,
		copy: bool,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let structure = self.inline_type(ty);
		let slot = Slot {
			pointer: builder.build_alloca(structure, "inline.source")?,
			repr: Repr::Tuple(ty),
		};
		builder.build_store(slot.pointer, value)?;
		let length = builder
			.build_extract_value(value.into_struct_value(), 1, "")?
			.into_int_value();
		let count = builder.build_int_z_extend(length, self.context.i64_type(), "")?;
		let destination = if copy {
			Slot {
				pointer: builder.build_alloca(structure, "inline.copy")?,
				..slot
			}
		} else {
			slot
		};
		self.helper_loop(builder, function, count, |builder, index| {
			let source = self.inline_item(builder, slot, ty, index)?;
			let raw = builder.build_load(self.llvm_type(source.repr), source.pointer, "")?;
			if copy {
				let raw = self.call_copy(builder, source.repr, raw)?;
				let target = self.inline_item(builder, destination, ty, index)?;
				builder.build_store(target.pointer, raw)?;
			} else {
				self.call_drop(builder, source.repr, raw)?;
			}
			Ok(())
		})?;
		if copy {
			let pointer = self.inline_length_pointer(builder, destination, ty)?;
			builder.build_store(pointer, length)?;
			Ok(builder.build_load(structure, destination.pointer, "")?)
		} else {
			Ok(value)
		}
	}
}
