//! Compact immutable member payloads selected by the closed region proof.
//! A member with one Array<Root> field keeps its array in the box itself:
//! id, length, capacity, then the child pointers. Large buffers keep an
//! external region-owned Array header, tagged by the high length bit.
//! Readers reconstruct a borrowed normal Array header in their own frame.
//! ROOT_REGIONS rejects moved/temporary projections and escaping views.

use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use super::values::{Repr, Val};
use super::{Lowering, Slot};
use crate::error::Error;

#[derive(Clone, Copy)]
pub(super) struct ArrayPayload {
	pub(super) data: TypeId,
	pub(super) item: TypeId,
}

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn compact_array_payload(
		&self,
		nominal: NominalId,
		index: u16,
	) -> Option<ArrayPayload> {
		if !self.arena_member(nominal) {
			return None;
		}
		let data = self.member_data(nominal, index)?;
		let fields = self.object_fields(data);
		let [Repr::Array(item)] = fields.as_slice() else {
			return None;
		};
		(self.arena_array_nominal(Repr::Array(*item)) == Some(nominal))
			.then_some(ArrayPayload { data, item: *item })
	}

	pub(super) fn compact_array_box(&self, id: u64) -> Option<ArrayPayload> {
		let member = self
			.union_ids
			.borrow()
			.iter()
			.find_map(|(&member, &candidate)| (candidate == id).then_some(member))?;
		self.compact_array_payload(member.0, member.1)
	}

	pub(super) fn compact_array_data(&self, data: TypeId) -> Vec<(u64, ArrayPayload)> {
		let mut members = Vec::new();
		for nominal in self.tree_arenas.unions.iter().copied() {
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len();
			for index in 0..count as u16 {
				if let Some(plan) = self.compact_array_payload(nominal, index)
					&& plan.data == data
				{
					members.push((self.member_id(nominal, index), plan));
				}
			}
		}
		members.sort_by_key(|(id, _)| *id);
		members
	}

	pub(super) fn payload_word(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		index: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(unsafe {
			b.build_in_bounds_gep(
				self.context.i64_type(),
				pointer,
				&[self.usize.const_int(index, false)],
				"",
			)?
		})
	}

	pub(super) fn load_payload_word(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		index: u64,
	) -> Result<IntValue<'ctx>, Error> {
		Ok(b.build_load(
			self.context.i64_type(),
			self.payload_word(b, pointer, index)?,
			"",
		)?
		.into_int_value())
	}

	fn payload_items(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		self.payload_word(b, pointer, 3)
	}

	/// Helpers may use a builder other than the active Fa function's builder.
	/// Allocate in that builder's entry, so loops do not grow their stack and
	/// no address belongs to a different LLVM function.
	pub(super) fn payload_alloca(
		&self,
		b: &Builder<'ctx>,
		ty: BasicTypeEnum<'ctx>,
		name: &str,
	) -> Result<PointerValue<'ctx>, Error> {
		let entry = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function")
			.get_first_basic_block()
			.expect("an entry");
		let allocate = self.context.create_builder();
		match entry.get_first_instruction() {
			Some(first) => allocate.position_before(&first),
			None => allocate.position_at_end(entry),
		}
		Ok(allocate.build_alloca(ty, name)?)
	}

	fn payload_array_parts(
		&self,
		b: &Builder<'ctx>,
		array: PointerValue<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>), Error> {
		let ty = self.header_type();
		let items = b
			.build_load(
				self.pointer_type(),
				b.build_struct_gep(ty, array, 0, "")?,
				"",
			)?
			.into_pointer_value();
		let length = b
			.build_load(
				self.context.i64_type(),
				b.build_struct_gep(ty, array, 1, "")?,
				"",
			)?
			.into_int_value();
		let capacity = b
			.build_load(
				self.context.i64_type(),
				b.build_struct_gep(ty, array, 2, "")?,
				"",
			)?
			.into_int_value();
		Ok((items, length, capacity))
	}

	pub(super) fn compact_payload_value(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		plan: ArrayPayload,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let inline = self.context.append_basic_block(function, "inline_children");
		let external = self
			.context
			.append_basic_block(function, "external_children");
		let end = self.context.append_basic_block(function, "children_view");
		let length = self.load_payload_word(b, pointer, 1)?;
		let outside = b.build_int_compare(
			IntPredicate::SLT,
			length,
			length.get_type().const_zero(),
			"",
		)?;
		b.build_conditional_branch(outside, external, inline)?;
		b.position_at_end(external);
		let raw = self.load_payload_word(b, pointer, 2)?;
		let array = b.build_int_to_ptr(raw, self.pointer_type(), "")?;
		b.build_unconditional_branch(end)?;
		b.position_at_end(inline);
		// The extra marker word makes the normal runtime's header reads safe.
		// It is a borrowed view, never registered as an owning local.
		let view = self.payload_alloca(
			b,
			self.context.i64_type().array_type(4).into(),
			"children_view",
		)?;
		let ty = self.header_type();
		b.build_store(
			b.build_struct_gep(ty, view, 0, "")?,
			self.payload_items(b, pointer)?,
		)?;
		b.build_store(b.build_struct_gep(ty, view, 1, "")?, length)?;
		b.build_store(
			b.build_struct_gep(ty, view, 2, "")?,
			self.load_payload_word(b, pointer, 2)?,
		)?;
		let marker = unsafe {
			b.build_in_bounds_gep(
				self.context.i8_type(),
				view,
				&[self
					.usize
					.const_int(self.target_data().get_abi_size(&ty), false)],
				"",
			)?
		};
		b.build_store(marker, self.context.i64_type().const_zero())?;
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		let array_pointer = b.build_phi(self.pointer_type(), "children")?;
		array_pointer.add_incoming(&[(&array, external), (&view, inline)]);
		Ok(b.build_insert_value(
			self.object_type(plan.data).get_undef(),
			array_pointer.as_basic_value(),
			0,
			"",
		)?
		.into_struct_value()
		.into())
	}

	pub(super) fn member_payload_value(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		nominal: NominalId,
		index: u16,
		repr: Repr,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if let Some(plan) = self.compact_array_payload(nominal, index) {
			return self.compact_payload_value(b, pointer, plan);
		}
		if let Some(data) = self.compact_string_payload(nominal, index) {
			return self.string_payload_value(b, pointer, data);
		}
		let field = b.build_struct_gep(self.box_type(Some(repr)), pointer, 1, "")?;
		Ok(b.build_load(self.llvm_type(repr), field, "")?)
	}

	pub(super) fn compact_payload_place(
		&self,
		pointer: PointerValue<'ctx>,
		plan: ArrayPayload,
	) -> Result<Slot<'ctx>, Error> {
		let value = self.compact_payload_value(&self.builder, pointer, plan)?;
		let repr = Repr::Object(plan.data);
		let slot = self.payload_alloca(&self.builder, self.llvm_type(repr), "member_view")?;
		self.builder.build_store(slot, value)?;
		Ok(Slot {
			pointer: slot,
			repr,
		})
	}

	fn inline_payload_limit(&self, item: Repr) -> u64 {
		let pointer = u64::from(self.usize.get_bit_width() / 8);
		let header = (5 * pointer).next_multiple_of(16);
		(super::trees::ARENA_CHUNK_BYTES - header - 24)
			/ self.target_data().get_abi_size(&self.llvm_type(item))
	}

	/// The completed owned temporary is frozen into the member. A copied
	/// member instead uses the ordinary Array copy capacity (its length).
	pub(super) fn freeze_array_payload(
		&self,
		b: &Builder<'ctx>,
		id: u64,
		plan: ArrayPayload,
		source: PointerValue<'ctx>,
		copy: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let (items, length, capacity) = self.payload_array_parts(b, source)?;
		let item = self.item_repr(plan.item, NodeId::new(0))?;
		let item_type = self.llvm_type(item);
		let item_bytes = self.target_data().get_abi_size(&item_type);
		let capacity = if copy { length } else { capacity };
		let inline = self.context.append_basic_block(function, "freeze_children");
		let external = self.context.append_basic_block(function, "keep_children");
		let end = self.context.append_basic_block(function, "frozen_children");
		let fits = b.build_int_compare(
			IntPredicate::ULE,
			capacity,
			self.context
				.i64_type()
				.const_int(self.inline_payload_limit(item), false),
			"",
		)?;
		b.build_conditional_branch(fits, inline, external)?;
		b.position_at_end(inline);
		let words = b.build_int_cast(capacity, self.usize, "")?;
		let bytes = b.build_int_mul(words, self.usize.const_int(item_bytes, false), "")?;
		let size = b.build_int_add(bytes, self.usize.const_int(24, false), "")?;
		let node = b
			.build_call(
				self.runtime.function("fa_arena_alloc")?,
				&[size.into(), self.usize.const_int(8, false).into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a node")
			.into_pointer_value();
		b.build_store(node, self.context.i64_type().const_int(id, false))?;
		b.build_store(self.payload_word(b, node, 1)?, length)?;
		b.build_store(self.payload_word(b, node, 2)?, capacity)?;
		let destination = self.payload_items(b, node)?;
		if copy {
			self.helper_loop(b, function, length, |b, index| {
				let from = unsafe { b.build_in_bounds_gep(item_type, items, &[index], "")? };
				let value = b.build_load(item_type, from, "")?;
				let copied = self.call_copy(b, item, value)?;
				let to = unsafe { b.build_in_bounds_gep(item_type, destination, &[index], "")? };
				b.build_store(to, copied)?;
				Ok(())
			})?;
		} else {
			let used = b.build_int_cast(length, self.usize, "")?;
			let used = b.build_int_mul(used, self.usize.const_int(item_bytes, false), "")?;
			b.build_memcpy(
				destination,
				item_bytes as u32,
				items,
				item_bytes as u32,
				used,
			)?;
			b.build_call(
				self.runtime.function("fa_array_free")?,
				&[source.into(), self.item_size_const(item).into()],
				"",
			)?;
		}
		let inline_end = b.get_insert_block().expect("a block");
		b.build_unconditional_branch(end)?;
		b.position_at_end(external);
		// Allocate the parent before copying children, so all child copies
		// join this region even when it was empty on entry.
		let prefix = self.context.i64_type().array_type(3);
		let outside = self.allocate_tree_type(b, prefix.into())?;
		let owned = if copy {
			self.call_copy(b, Repr::Array(plan.item), source.into())?
				.into_pointer_value()
		} else {
			source
		};
		let owned = b
			.build_call(
				self.runtime.function("fa_array_into_region")?,
				&[owned.into(), self.item_size_const(item).into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an owned array")
			.into_pointer_value();
		b.build_store(outside, self.context.i64_type().const_int(id, false))?;
		let tagged = b.build_or(
			length,
			self.context.i64_type().const_int(1 << 63, false),
			"",
		)?;
		b.build_store(self.payload_word(b, outside, 1)?, tagged)?;
		let address = b.build_ptr_to_int(owned, self.context.i64_type(), "")?;
		b.build_store(self.payload_word(b, outside, 2)?, address)?;
		let external_end = b.get_insert_block().expect("a block");
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		let result = b.build_phi(self.pointer_type(), "frozen_member")?;
		result.add_incoming(&[(&node, inline_end), (&outside, external_end)]);
		Ok(result.as_basic_value().into_pointer_value())
	}

	pub(super) fn unbox_compact_data(
		&self,
		value: Val<'ctx>,
		data: TypeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		let mut plans: Vec<_> = self
			.compact_array_data(data)
			.into_iter()
			.map(|(id, plan)| (id, Some(plan)))
			.collect();
		plans.extend(
			self.compact_string_data(data)
				.into_iter()
				.map(|id| (id, None)),
		);
		if plans.is_empty() {
			return Ok(None);
		}
		let b = &self.builder;
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let pointer = value.value.expect("a box").into_pointer_value();
		let id = b
			.build_load(self.context.i64_type(), pointer, "")?
			.into_int_value();
		let dispatch = b.get_insert_block().expect("a block");
		let normal = self.context.append_basic_block(function, "normal_member");
		let end = self.context.append_basic_block(function, "member_data");
		b.position_at_end(normal);
		let repr = Repr::Object(data);
		let field = b.build_struct_gep(self.box_type(Some(repr)), pointer, 1, "")?;
		let ordinary = b.build_load(self.llvm_type(repr), field, "")?;
		b.build_unconditional_branch(end)?;
		let mut incoming = vec![(ordinary, normal)];
		let mut cases = Vec::new();
		for (id, plan) in plans {
			let block = self.context.append_basic_block(function, "compact_member");
			b.position_at_end(block);
			let data = match plan {
				Some(plan) => self.compact_payload_value(b, pointer, plan)?,
				None => self.string_payload_value(b, pointer, data)?,
			};
			let before = b.get_insert_block().expect("a block");
			b.build_unconditional_branch(end)?;
			incoming.push((data, before));
			cases.push((self.context.i64_type().const_int(id, false), block));
		}
		b.position_at_end(dispatch);
		b.build_switch(id, normal, &cases)?;
		b.position_at_end(end);
		let result = b.build_phi(self.llvm_type(repr), "member")?;
		for (value, block) in &incoming {
			result.add_incoming(&[(value, *block)]);
		}
		Ok(Some(Val::new(repr, result.as_basic_value())))
	}
}
