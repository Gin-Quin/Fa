//! The column layout of `Columnar` and `GrowingColumnar`
//! (collections/columnars.md, implementation notes).
//!
//! A columnar is a pool of the runtime (`pools.rs`) whose slots are 4 bytes
//! wide (they hold the free list of the chunk while dead); after them, each
//! chunk holds one run per field of the element type, every run aligned on
//! 64 bytes. All the runs of a chunk have the capacity of the chunk, a
//! power of two chosen so that the widest field fills about 64 KiB, and the
//! chunks are aligned on their size like a pool's.
//!
//! A reference to an element is the address of its slot ([`Repr::Row`] in
//! a [`Slot`]): stable, unique, and enough to find every field (the chunk is
//! the address with its low bits cleared, the slot index its offset in the
//! slots divided by 4). Reading an element whole gathers its fields from
//! their runs, writing it scatters them; a field is one address
//! computation away (`john.age` reads the `age` run only).
//!
//! The elements of a type with relation sides stay in rows (a pool): the
//! runtime updates sides at byte offsets in the element.

use fa_analyzer::analysis::{PatternBinding, PatternInfo};
use fa_analyzer::ids::{NodeId, TypeId};
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue, StructValue};

use crate::error::Error;
use crate::lower::values::Repr;
use crate::lower::{Lowering, Slot};

/// The page size of the runtime's allocator.
const PAGE_SIZE: u64 = 65536;
/// The size of a chunk header (`PoolChunk` before its occupancy words).
const CHUNK_HEADER: u64 = 24;

/// Where the runs of a columnar of one element type are in its chunks.
#[derive(Clone, Debug)]
pub(crate) struct ColumnLayout {
	/// The slots of a chunk (a power of two).
	pub capacity: u64,
	/// The pages of a chunk (a power of two): chunks are aligned on
	/// `pages * PAGE_SIZE`.
	pub pages: u64,
	/// The offset of the slots in a chunk.
	pub slots_offset: u64,
	/// Per field (shape order): the offset of its run, and its item size.
	pub columns: Vec<(u64, u64)>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether a columnar of elements of this item type is stored by
	/// columns: objects without relation sides.
	pub(crate) fn columnar_element(&self, item_repr: Repr) -> Option<TypeId> {
		match item_repr {
			Repr::Object(element) if !self.has_sides(element) => Some(element),
			_ => None,
		}
	}

	/// The layout of the chunks of a columnar of this element type.
	pub(crate) fn column_layout(&self, element: TypeId) -> ColumnLayout {
		let data = self.target_data();
		let sizes: Vec<u64> = self
			.object_fields(element)
			.into_iter()
			.map(|repr| data.get_abi_size(&self.llvm_type(repr)).max(1))
			.collect();
		let widest = sizes.iter().copied().max().unwrap_or(1).max(4);
		let mut capacity = 64;
		while capacity * 2 * widest <= 65536 {
			capacity *= 2;
		}
		let words = capacity.div_ceil(64);
		let slots_offset = (CHUNK_HEADER + words * 8 + 15) & !15;
		let mut end = slots_offset + capacity * 4;
		let mut columns = Vec::new();
		for size in sizes {
			let start = (end + 63) & !63;
			columns.push((start, size));
			end = start + capacity * size;
		}
		let mut pages = 1;
		while pages * PAGE_SIZE < end {
			pages *= 2;
		}
		ColumnLayout {
			capacity,
			pages,
			slots_offset,
			columns,
		}
	}

	/// The address of the field at `index` of the element whose slot is at
	/// `slot`: `slot + (run + index * size - offset of the slot)`.
	pub(crate) fn row_field(
		&self,
		builder: &Builder<'ctx>,
		slot: PointerValue<'ctx>,
		element: TypeId,
		index: usize,
	) -> Result<PointerValue<'ctx>, Error> {
		let layout = self.column_layout(element);
		let (run, size) = layout.columns[index];
		let usize = self.usize;
		let mask = usize.const_int(layout.pages * PAGE_SIZE - 1, false);
		let address = builder.build_ptr_to_int(slot, usize, "")?;
		// the offset of the slot in its chunk, and the slot's index
		let offset = builder.build_and(address, mask, "")?;
		let position =
			builder.build_int_sub(offset, usize.const_int(layout.slots_offset, false), "")?;
		let position = builder.build_right_shift(position, usize.const_int(2, false), false, "")?;
		let within = builder.build_int_nuw_mul(position, usize.const_int(size, false), "")?;
		let target = builder.build_int_add(within, usize.const_int(run, false), "")?;
		let delta: IntValue<'ctx> = builder.build_int_sub(target, offset, "")?;
		Ok(unsafe { builder.build_gep(self.context.i8_type(), slot, &[delta], "field")? })
	}

	/// The element whose slot is at `slot`, gathered from its runs (a view).
	pub(crate) fn gather(
		&self,
		builder: &Builder<'ctx>,
		slot: PointerValue<'ctx>,
		element: TypeId,
	) -> Result<StructValue<'ctx>, Error> {
		let fields = self.object_fields(element);
		let mut value = self.object_type(element).get_undef();
		for (index, repr) in fields.into_iter().enumerate() {
			let pointer = self.row_field(builder, slot, element, index)?;
			let loaded = builder.build_load(self.llvm_type(repr), pointer, "")?;
			value = builder
				.build_insert_value(value, loaded, index as u32, "")?
				.into_struct_value();
		}
		Ok(value)
	}

	/// Stores an element into its runs.
	pub(crate) fn scatter(
		&self,
		builder: &Builder<'ctx>,
		slot: PointerValue<'ctx>,
		element: TypeId,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		let structure = value.into_struct_value();
		let count = self.object_fields(element).len();
		for index in 0..count {
			let pointer = self.row_field(builder, slot, element, index)?;
			let field = builder.build_extract_value(structure, index as u32, "")?;
			builder.build_store(pointer, field)?;
		}
		Ok(())
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// `FaPool` of the runtime: `{ size, chunks, available, spare, count,
	/// table_capacity, slot_size, capacity, pages, slots_offset }`.
	pub(crate) fn pool_header(&self) -> inkwell::types::StructType<'ctx> {
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let pointer = self.pointer_type();
		self.context.struct_type(
			&[
				i64.into(),
				pointer.into(),
				pointer.into(),
				pointer.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				i64.into(),
			],
			false,
		)
	}

	/// `PoolChunk` of the runtime: `{ slots, index, live, bumped, free_head,
	/// occupancy[] }`.
	fn chunk_header(&self) -> inkwell::types::StructType<'ctx> {
		let i32 = self.context.i32_type();
		self.context.struct_type(
			&[
				self.pointer_type().into(),
				i32.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				self.context.i64_type().array_type(0).into(),
			],
			false,
		)
	}

	/// The loop of a `for` over a pool or a columnar, in slot order: over
	/// its chunks, their occupancy words, and the bits of each word. The
	/// builder is left in the body; returns the block that moves to the
	/// next element, the address of the element, its chunk and its slot
	/// index. A word is read once (the body can't remove elements: the
	/// analyzer forbids it).
	#[allow(clippy::type_complexity)]
	pub(crate) fn chunk_loop(
		&mut self,
		pool: PointerValue<'ctx>,
		exit: BasicBlock<'ctx>,
	) -> Result<
		(
			BasicBlock<'ctx>,
			PointerValue<'ctx>,
			PointerValue<'ctx>,
			IntValue<'ctx>,
		),
		Error,
	> {
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let pointer_type = self.pointer_type();
		let header = self.pool_header();
		let chunk_type = self.chunk_header();
		let chunk_index = self.alloca_type(i64.into(), "chunk_index")?;
		let word_index = self.alloca_type(i64.into(), "word_index")?;
		let bit_index = self.alloca_type(i64.into(), "bit_index")?;
		let word_slot = self.alloca_type(i64.into(), "word")?;
		let chunk_slot = self.alloca_type(pointer_type.into(), "chunk")?;
		let end_slot = self.alloca_type(i64.into(), "bumped")?;
		self.builder.build_store(chunk_index, i64.const_zero())?;
		let chunk_test = self.append_block("chunk_test");
		let chunk_load = self.append_block("chunk_load");
		let chunk_start = self.append_block("chunk_start");
		let word_test = self.append_block("word_test");
		let word_load = self.append_block("word_load");
		let bit_test = self.append_block("bit_test");
		let bit_check = self.append_block("bit_check");
		let body = self.append_block("for_element_body");
		self.builder.build_unconditional_branch(chunk_test)?;
		// the next chunk
		self.builder.position_at_end(chunk_test);
		let index = self
			.builder
			.build_load(i64, chunk_index, "")?
			.into_int_value();
		let count_field = self.builder.build_struct_gep(header, pool, 4, "")?;
		let count = self
			.builder
			.build_load(i32, count_field, "")?
			.into_int_value();
		let count = self.builder.build_int_z_extend(count, i64, "")?;
		let more = self
			.builder
			.build_int_compare(IntPredicate::ULT, index, count, "")?;
		self.builder
			.build_conditional_branch(more, chunk_load, exit)?;
		self.builder.position_at_end(chunk_load);
		let chunks_field = self.builder.build_struct_gep(header, pool, 1, "")?;
		let chunks = self
			.builder
			.build_load(pointer_type, chunks_field, "")?
			.into_pointer_value();
		let entry = unsafe {
			self.builder
				.build_in_bounds_gep(pointer_type, chunks, &[index], "")?
		};
		let chunk = self
			.builder
			.build_load(pointer_type, entry, "")?
			.into_pointer_value();
		let next_chunk = self
			.builder
			.build_int_add(index, i64.const_int(1, false), "")?;
		self.builder.build_store(chunk_index, next_chunk)?;
		self.builder.build_store(chunk_slot, chunk)?;
		let absent = self.builder.build_is_null(chunk, "")?;
		self.builder
			.build_conditional_branch(absent, chunk_test, chunk_start)?;
		self.builder.position_at_end(chunk_start);
		let bumped_field = self.builder.build_struct_gep(chunk_type, chunk, 3, "")?;
		let bumped = self
			.builder
			.build_load(i32, bumped_field, "")?
			.into_int_value();
		let bumped = self.builder.build_int_z_extend(bumped, i64, "")?;
		self.builder.build_store(end_slot, bumped)?;
		self.builder.build_store(word_index, i64.const_zero())?;
		self.builder.build_unconditional_branch(word_test)?;
		// the next occupancy word of the chunk
		self.builder.position_at_end(word_test);
		let word_at = self
			.builder
			.build_load(i64, word_index, "")?
			.into_int_value();
		let first_slot = self
			.builder
			.build_int_mul(word_at, i64.const_int(64, false), "")?;
		let end = self.builder.build_load(i64, end_slot, "")?.into_int_value();
		let more = self
			.builder
			.build_int_compare(IntPredicate::ULT, first_slot, end, "")?;
		self.builder
			.build_conditional_branch(more, word_load, chunk_test)?;
		self.builder.position_at_end(word_load);
		let chunk = self
			.builder
			.build_load(pointer_type, chunk_slot, "")?
			.into_pointer_value();
		let words = self.builder.build_struct_gep(chunk_type, chunk, 5, "")?;
		let word_pointer = unsafe {
			self.builder
				.build_in_bounds_gep(i64, words, &[word_at], "")?
		};
		let word = self
			.builder
			.build_load(i64, word_pointer, "")?
			.into_int_value();
		self.builder.build_store(word_slot, word)?;
		let next_word = self
			.builder
			.build_int_add(word_at, i64.const_int(1, false), "")?;
		self.builder.build_store(word_index, next_word)?;
		self.builder.build_store(bit_index, i64.const_zero())?;
		let empty = self
			.builder
			.build_int_compare(IntPredicate::EQ, word, i64.const_zero(), "")?;
		self.builder
			.build_conditional_branch(empty, word_test, bit_test)?;
		// the next bit of the word
		self.builder.position_at_end(bit_test);
		let bit = self
			.builder
			.build_load(i64, bit_index, "")?
			.into_int_value();
		let more =
			self.builder
				.build_int_compare(IntPredicate::ULT, bit, i64.const_int(64, false), "")?;
		self.builder
			.build_conditional_branch(more, bit_check, word_test)?;
		self.builder.position_at_end(bit_check);
		let word = self
			.builder
			.build_load(i64, word_slot, "")?
			.into_int_value();
		let shifted = self.builder.build_right_shift(word, bit, false, "")?;
		let live = self
			.builder
			.build_and(shifted, i64.const_int(1, false), "")?;
		let next_bit = self
			.builder
			.build_int_add(bit, i64.const_int(1, false), "")?;
		self.builder.build_store(bit_index, next_bit)?;
		let word_at = self
			.builder
			.build_load(i64, word_index, "")?
			.into_int_value();
		let word_at = self
			.builder
			.build_int_sub(word_at, i64.const_int(1, false), "")?;
		let slot = self
			.builder
			.build_int_mul(word_at, i64.const_int(64, false), "")?;
		let slot = self.builder.build_int_add(slot, bit, "slot")?;
		let live = self
			.builder
			.build_int_compare(IntPredicate::NE, live, i64.const_zero(), "")?;
		self.builder
			.build_conditional_branch(live, body, bit_test)?;
		// the element
		self.builder.position_at_end(body);
		let chunk = self
			.builder
			.build_load(pointer_type, chunk_slot, "")?
			.into_pointer_value();
		let slots_field = self.builder.build_struct_gep(chunk_type, chunk, 0, "")?;
		let slots = self
			.builder
			.build_load(pointer_type, slots_field, "")?
			.into_pointer_value();
		let size_field = self.builder.build_struct_gep(header, pool, 6, "")?;
		let slot_size = self
			.builder
			.build_load(i32, size_field, "")?
			.into_int_value();
		let slot_size = self.builder.build_int_z_extend(slot_size, i64, "")?;
		let offset = self.builder.build_int_mul(slot, slot_size, "")?;
		let address = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), slots, &[offset], "element")?
		};
		Ok((bit_test, address, chunk, slot))
	}

	/// The fields a pattern takes from an element of a columnar, bound to
	/// their places in the runs of its chunk (`for { age } in columnar`).
	pub(crate) fn bind_columns(
		&mut self,
		info: &PatternInfo,
		object: TypeId,
		chunk: PointerValue<'ctx>,
		slot: IntValue<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		let layout = self.column_layout(object);
		let i64 = self.context.i64_type();
		for element in &info.elements {
			let (PatternBinding::Symbol(symbol), Some(field)) = (element.binding, element.field)
			else {
				return Err(self.unsupported(node.index(), "this pattern in a `for`"));
			};
			let Some(index) = self
				.types()
				.fields_of(object)
				.iter()
				.position(|entry| entry.name == field)
			else {
				return Err(self.unsupported(node.index(), "this pattern in a `for`"));
			};
			let (run, size) = layout.columns[index];
			let within = self
				.builder
				.build_int_nuw_mul(slot, i64.const_int(size, false), "")?;
			let offset = self
				.builder
				.build_int_add(within, i64.const_int(run, false), "")?;
			let pointer = unsafe {
				self.builder
					.build_in_bounds_gep(self.context.i8_type(), chunk, &[offset], "")?
			};
			let repr = self.field_repr(object, index, node.index())?;
			self.slots.insert(symbol, Slot { pointer, repr });
		}
		Ok(())
	}
}
