//! `GrowingArray`: a segmented list whose items never move
//! (growing-collections.md, implementation notes), so that a reference into
//! it stays valid while it grows (`use first = growing[0]` then `push`).
//!
//! A `GrowingArray` is a pointer to a `FaSegmented` of the runtime
//! (`fa_runtime.c`, "segmented storage"): a 64-byte header with a length,
//! segment count, per-array `k`, cursor, end, and a directory (three pointers
//! inline, allocated after that). The first segment holds `2^k` items
//! (about 64 bytes by default, [`Lowering::segment_bits`]), each next one doubles
//! the capacity; item `i` is found with one `clz` ([`Lowering::growing_at`]).
//!
//! - `push` writes at the cursor until it reaches the end of its segment:
//!   one comparison, one store and one increment, the next segment found
//!   (or allocated) out of line by `fa_segmented_next`.
//! - `for` loops walk each segment with a pointer (`fa_segmented_run` once
//!   per segment); the loops of `copies.rs` copy, fill and search segment
//!   by segment.
//! - `growing = []` drops the items and keeps the segments (`fa_segmented_reset`).
//! - Indexing, `length`, `capacity`, `reserve`, `isEmpty`, `get`, drops and copies work on the
//!   segments. The other (read-only) methods run on a *view*: the items
//!   copied bytewise into a temporary array that doesn't own them, freed at
//!   the end of the statement (`Cleanup::Shallow`).

use fa_analyzer::analysis::{Callee, NodeFlags};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::StructType;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

/// The inline segment pointers of a `FaSegmented` (`FA_INLINE_SEGMENTS`).
const SEGMENTS: u32 = 3;

impl<'ctx> Lowering<'_, 'ctx> {
	/// `{ i64 length, i32 segments, i32 k, ptr cursor, ptr end, ptr directory, [3 x ptr] }`.
	pub(crate) fn segmented_type(&self) -> StructType<'ctx> {
		let i64 = self.context.i64_type();
		self.context.struct_type(
			&[
				i64.into(),
				self.segment_metadata_type().into(),
				self.context.i32_type().into(),
				self.pointer_type().into(),
				self.pointer_type().into(),
				self.pointer_type().into(),
				self.pointer_type().array_type(SEGMENTS).into(),
			],
			false,
		)
	}

	pub(crate) fn segment_metadata_type(&self) -> StructType<'ctx> {
		let i16 = self.context.i16_type();
		self.context.struct_type(&[i16.into(), i16.into()], false)
	}

	/// The `k` of an item, as the runtime takes it.
	pub(crate) fn segment_k(&self, item: Repr) -> IntValue<'ctx> {
		self.segment_arguments(item).1
	}

	/// A header field of a `GrowingArray` (0 length, 3 cursor, 4 end), loaded
	/// with the header tag.
	fn growing_field(
		&self,
		array: PointerValue<'ctx>,
		field: u32,
		name: &str,
	) -> Result<(PointerValue<'ctx>, inkwell::values::BasicValueEnum<'ctx>), Error> {
		let pointer = self
			.builder
			.build_struct_gep(self.segmented_type(), array, field, "")?;
		let ty: inkwell::types::BasicTypeEnum = if field == 0 {
			self.context.i64_type().into()
		} else {
			self.pointer_type().into()
		};
		let value = self.builder.build_load(ty, pointer, name)?;
		if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&value) {
			self.tag(instruction, true)?;
		}
		Ok((pointer, value))
	}

	/// `k`: the first segment holds `2^k` items, about 64 bytes.
	pub(crate) fn segment_bits(&self, item: Repr) -> u32 {
		let data = inkwell::targets::TargetData::create(
			self.module
				.get_data_layout()
				.as_str()
				.to_str()
				.unwrap_or(""),
		);
		let size = data.get_abi_size(&self.llvm_type(item)).max(1);
		let mut bits = 0;
		while (size << (bits + 1)) <= 64 {
			bits += 1;
		}
		bits
	}

	/// The size of an item and `k`, as the runtime takes them.
	fn segment_arguments(&self, item: Repr) -> (IntValue<'ctx>, IntValue<'ctx>) {
		(
			self.item_size_const(item),
			self.context
				.i32_type()
				.const_int(u64::from(self.segment_bits(item)), false),
		)
	}

	/// A sufficient contiguity proof: a live item outside the first block
	/// requires a second allocated segment. Zero segments means zero items.
	pub(super) fn growing_single_segment(
		&self,
		array: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let metadata = self
			.builder
			.build_struct_gep(self.segmented_type(), array, 1, "")?;
		let field = self
			.builder
			.build_struct_gep(self.segment_metadata_type(), metadata, 0, "")?;
		let count = self
			.builder
			.build_load(self.context.i16_type(), field, "segment_count")?;
		self.tag(
			inkwell::values::BasicValue::as_instruction_value(&count).expect("a load"),
			true,
		)?;
		Ok(self.builder.build_int_compare(
			IntPredicate::ULE,
			count.into_int_value(),
			self.context.i16_type().const_int(1, false),
			"contiguous",
		)?)
	}

	/// The length of a `GrowingArray`.
	pub(crate) fn growing_length(
		&self,
		builder: &Builder<'ctx>,
		array: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let field = builder.build_struct_gep(self.segmented_type(), array, 0, "")?;
		let length = builder.build_load(self.context.i64_type(), field, "length")?;
		if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&length) {
			self.tag(instruction, true)?;
		}
		Ok(length.into_int_value())
	}

	/// The address of item `index` (in bounds).
	pub(crate) fn growing_at(
		&self,
		builder: &Builder<'ctx>,
		array: PointerValue<'ctx>,
		item: Repr,
		index: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		// `fa_segmented_at`, written here with the header tag: a store into an
		// item doesn't make LLVM read the segment pointers again
		let k = self.growing_k(builder, array)?;
		self.growing_at_with_k(builder, array, item, index, k)
	}

	fn growing_k(
		&self,
		builder: &Builder<'ctx>,
		array: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let k_pointer = builder.build_struct_gep(self.segmented_type(), array, 2, "")?;
		let k = builder
			.build_load(self.context.i32_type(), k_pointer, "segment_k")?
			.into_int_value();
		if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&k) {
			self.tag(instruction, true)?;
		}
		Ok(builder.build_int_z_extend(k, self.context.i64_type(), "")?)
	}

	fn growing_at_with_k(
		&self,
		builder: &Builder<'ctx>,
		array: PointerValue<'ctx>,
		item: Repr,
		index: IntValue<'ctx>,
		k: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let function = builder
			.get_insert_block()
			.and_then(|block| block.get_parent())
			.expect("a function");
		let segment_items = |builder: &Builder<'ctx>, segment: IntValue<'ctx>| {
			let field = builder.build_struct_gep(self.segmented_type(), array, 5, "")?;
			let directory = builder.build_load(self.pointer_type(), field, "segments")?;
			self.tag(
				inkwell::values::BasicValue::as_instruction_value(&directory).expect("a load"),
				true,
			)?;
			let inline = builder.build_struct_gep(self.segmented_type(), array, 6, "")?;
			// One past the inline array is a valid base for negative offsets
			// back into it. The overflow directory uses nonnegative offsets.
			// Both choices therefore share segment - SEGMENTS, without forming
			// a pointer before either allocation.
			let inline_end = unsafe {
				builder.build_in_bounds_gep(
					self.pointer_type(),
					inline,
					&[i64.const_int(u64::from(SEGMENTS), false)],
					"",
				)?
			};
			let in_header = builder.build_int_compare(
				IntPredicate::ULT,
				segment,
				i64.const_int(u64::from(SEGMENTS), false),
				"inline_segment",
			)?;
			let base = builder
				.build_select(
					in_header,
					inline_end,
					directory.into_pointer_value(),
					"segment_base",
				)?
				.into_pointer_value();
			let offset =
				builder.build_int_sub(segment, i64.const_int(u64::from(SEGMENTS), false), "")?;
			let slot =
				unsafe { builder.build_in_bounds_gep(self.pointer_type(), base, &[offset], "")? };
			let items = builder.build_load(self.pointer_type(), slot, "segment_items")?;
			self.tag(
				inkwell::values::BasicValue::as_instruction_value(&items).expect("a load"),
				true,
			)?;
			Ok::<_, Error>(items.into_pointer_value())
		};
		let first_capacity =
			builder.build_left_shift(i64.const_int(1, false), k, "first_capacity")?;
		let in_first =
			builder.build_int_compare(IntPredicate::ULT, index, first_capacity, "first_segment")?;
		let first_block = self.context.append_basic_block(function, "first_segment");
		let later_block = self.context.append_basic_block(function, "later_segment");
		let join = self.context.append_basic_block(function, "segment_item");
		builder.build_conditional_branch(in_first, first_block, later_block)?;
		builder.position_at_end(first_block);
		let first_slot = builder.build_struct_gep(self.segmented_type(), array, 6, "")?;
		let items = builder.build_load(self.pointer_type(), first_slot, "first_items")?;
		if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&items) {
			self.tag(instruction, true)?;
		}
		let items = items.into_pointer_value();
		let in_first_segment =
			unsafe { builder.build_in_bounds_gep(self.llvm_type(item), items, &[index], "")? };
		builder.build_unconditional_branch(join)?;
		builder.position_at_end(later_block);
		// Here index >= 2^k, hence it is nonzero. Its highest set bit
		// gives both the segment and its start without shifting the index
		// before ctlz: floor(log2(index)) - k + 1.
		let ctlz = self.intrinsic("llvm.ctlz", &[i64.into()])?;
		let zeros = builder
			.build_call(
				ctlz,
				&[
					index.into(),
					self.context.bool_type().const_all_ones().into(),
				],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a count")
			.into_int_value();
		let bit = builder.build_int_sub(i64.const_int(63, false), zeros, "index_bit")?;
		let segment = builder.build_int_sub(bit, k, "")?;
		let segment = builder.build_int_add(segment, i64.const_int(1, false), "segment")?;
		let first = builder.build_left_shift(i64.const_int(1, false), bit, "")?;
		let items = segment_items(builder, segment)?;
		let later_end = builder.get_insert_block().expect("a block");
		let offset = builder.build_int_sub(index, first, "")?;
		let in_later_segment =
			unsafe { builder.build_in_bounds_gep(self.llvm_type(item), items, &[offset], "")? };
		builder.build_unconditional_branch(join)?;
		builder.position_at_end(join);
		let pointer = builder.build_phi(self.pointer_type(), "item")?;
		pointer.add_incoming(&[
			(&in_first_segment, first_block),
			(&in_later_segment, later_end),
		]);
		Ok(pointer.as_basic_value().into_pointer_value())
	}

	/// A `GrowingArray` taking the items of an owned array (not registered).
	pub(crate) fn growing_from_array(
		&self,
		array: PointerValue<'ctx>,
		item: Repr,
	) -> Result<PointerValue<'ctx>, Error> {
		let (size, bits) = self.segment_arguments(item);
		Ok(self
			.call_value(
				self.runtime.function("fa_segmented_from_array")?,
				&[array.into(), size.into(), bits.into()],
			)?
			.into_pointer_value())
	}

	/// An array holding the items of an owned `GrowingArray` (moved; not
	/// registered).
	fn array_from_growing(
		&self,
		array: PointerValue<'ctx>,
		item: Repr,
	) -> Result<PointerValue<'ctx>, Error> {
		let (size, bits) = self.segment_arguments(item);
		let flat = self
			.call_value(
				self.runtime.function("fa_segmented_flatten")?,
				&[array.into(), size.into(), bits.into()],
			)?
			.into_pointer_value();
		self.builder.build_call(
			self.runtime.function("fa_segmented_free")?,
			&[array.into(), size.into(), bits.into()],
			"",
		)?;
		Ok(flat)
	}

	/// A view of a `GrowingArray` as an array: its items copied bytewise into
	/// a block that doesn't own them, freed at the end of the statement. The
	/// view is a borrowed value (like a place read): taking it copies it.
	pub(crate) fn growing_view(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let Repr::Growing(item_type) = value.repr else {
			return Ok(value);
		};
		let item = self.item_repr(item_type, NodeId::new(0))?;
		let (size, bits) = self.segment_arguments(item);
		let flat = self
			.call_value(
				self.runtime.function("fa_segmented_flatten")?,
				&[
					value.value.expect("a growing array").into(),
					size.into(),
					bits.into(),
				],
			)?
			.into_pointer_value();
		self.cleanups
			.borrow_mut()
			.push(super::ownership::Cleanup::Shallow(flat, size));
		Ok(Val::new(Repr::Array(item_type), flat))
	}

	/// Converts between `Array(T)` and `GrowingArray(T)`: moves a temporary,
	/// copies anything else first.
	pub(crate) fn convert_growing(&self, value: Val<'ctx>, to: Repr) -> Result<Val<'ctx>, Error> {
		let (Repr::Array(item_type) | Repr::Growing(item_type)) = value.repr else {
			return Ok(value);
		};
		let item = self.item_repr(item_type, NodeId::new(0))?;
		let owned = if self.is_temporary(value) {
			self.take(value, None)?
		} else {
			self.emit_copy(value)?
		};
		let pointer = owned.value.expect("an array").into_pointer_value();
		let converted = match to {
			Repr::Growing(_) => self.growing_from_array(pointer, item)?,
			_ => self.array_from_growing(pointer, item)?,
		};
		Ok(self.fresh(Val::new(to, converted)))
	}

	/// An empty owner whose complete use set only indexes, iterates and
	/// pushes cannot change its default k. Unknown calls, aliases, reserve,
	/// replacement and exported/multifile uses conservatively reject it.
	fn growing_default_origin(&self, node: NodeId, item: Repr) -> bool {
		if !self.other_files.is_empty()
			|| !self.analysis.foreign_args.is_empty()
			|| self.collection_data_size(self.llvm_type(item)) == 0
		{
			return false;
		}
		let View::Index { target, .. } = self.view(node) else {
			return false;
		};
		let Resolution::Local(symbol) = self.analysis.resolution(self.ungrouped(target)) else {
			return false;
		};
		if let Some(&known) = self
			.default_growing_origins
			.borrow()
			.get(&(self.file, symbol))
		{
			return known;
		}
		let binding = self.analysis.symbols.symbol(symbol);
		let empty = matches!(binding.kind, SymbolKind::Let | SymbolKind::Mutable)
			&& matches!(self.view(binding.decl_node), View::Declaration { value: Some(value), .. }
				if matches!(self.view(self.ungrouped(value)), View::Array { items } if items.iter().next().is_none()));
		let parents = self.parents();
		let known = empty && self.analysis.resolutions.iter().enumerate().all(|(index, resolution)| {
			if *resolution != Resolution::Local(symbol) { return true; }
			let mut at = NodeId::new(index);
			loop {
				let Some(&parent) = parents.get(at.index()) else { return false; };
				if parent == usize::MAX { return false; }
				let parent = NodeId::new(parent);
				match self.view(parent) {
					View::Group(_) => at = parent,
					View::Index { target, .. } => return target == at,
					View::For { iterable, .. } => return iterable == at,
					View::Access { target, .. } if target == at => {
						let Some(&call) = parents.get(parent.index()) else { return false; };
						if call == usize::MAX { return false; }
						return self.analysis.call(NodeId::new(call)).is_some_and(|call| {
							matches!(call.callee, Callee::Builtin(id) if fa_analyzer::builtins::get(id).path == "Array#push")
						});
					}
					_ => return false,
				}
			}
		});
		self.default_growing_origins
			.borrow_mut()
			.insert((self.file, symbol), known);
		known
	}

	/// The place `growing[index]`, checked when the node has `INDEX_CHECK`.
	pub(crate) fn growing_item(
		&mut self,
		node: NodeId,
		array: Val<'ctx>,
		index: Val<'ctx>,
		index_node: Option<NodeId>,
	) -> Result<Slot<'ctx>, Error> {
		let Repr::Growing(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "indexing this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let array = array.value.expect("a growing array").into_pointer_value();
		let index = self
			.convert(index, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let length = self.growing_length(&self.builder, array)?;
		let may_be_negative = index_node.is_none_or(|index_node| self.may_be_negative(index_node));
		let checked = self.flags(node).contains(NodeFlags::INDEX_CHECK);
		let default_k = self.growing_default_origin(node, item).then(|| {
			self.context
				.i64_type()
				.const_int(u64::from(self.segment_bits(item)), false)
		});
		let contiguous = if default_k.is_some() {
			self.context.bool_type().const_zero()
		} else {
			self.growing_single_segment(array)?
		};
		let first_slot = self
			.builder
			.build_struct_gep(self.segmented_type(), array, 6, "")?;
		let first_items =
			self.builder
				.build_load(self.pointer_type(), first_slot, "first_items")?;
		self.tag(
			inkwell::values::BasicValue::as_instruction_value(&first_items).expect("a load"),
			true,
		)?;
		let position = if checked && may_be_negative {
			// `0 <= index < length` on the common path, as for arrays
			// (`checked_item`): a negative index is counted from the end in
			// a cold block
			let inside = self
				.builder
				.build_int_compare(IntPredicate::ULT, index, length, "")?;
			let before = self.builder.get_insert_block().expect("a block");
			let found = self.append_block("index_found");
			let other = self.append_block("index_from_end");
			let branch = self
				.builder
				.build_conditional_branch(inside, found, other)?;
			self.likely(branch)?;
			self.builder.position_at_end(other);
			let from_end = self
				.call_value(
					self.runtime.function("fa_index_from_end")?,
					&[index.into(), length.into()],
				)?
				.into_int_value();
			let outside = self.builder.build_int_compare(
				IntPredicate::SLT,
				from_end,
				index.get_type().const_zero(),
				"",
			)?;
			self.guard(outside, |this| {
				this.raise_runtime("fa_error_index", &[index.into(), length.into()])
			})?;
			let other_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(found)?;
			self.builder.position_at_end(found);
			let position = self.builder.build_phi(index.get_type(), "position")?;
			position.add_incoming(&[(&index, before), (&from_end, other_end)]);
			position.as_basic_value().into_int_value()
		} else {
			let position = if may_be_negative {
				self.counted_from_end(index, length)?
			} else {
				index
			};
			if checked {
				let outside =
					self.builder
						.build_int_compare(IntPredicate::UGE, position, length, "")?;
				self.guard(outside, |this| {
					this.raise_runtime("fa_error_index", &[index.into(), length.into()])
				})?;
			}
			position
		};
		let first = self.append_block("index_contiguous");
		let segmented = self.append_block("index_segmented");
		let join = self.append_block("index_item");
		self.builder
			.build_conditional_branch(contiguous, first, segmented)?;
		self.builder.position_at_end(first);
		let direct = unsafe {
			self.builder.build_in_bounds_gep(
				self.llvm_type(item),
				first_items.into_pointer_value(),
				&[position],
				"",
			)?
		};
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(segmented);
		let pointer = if let Some(k) = default_k {
			self.growing_at_with_k(&self.builder, array, item, position, k)?
		} else {
			self.growing_at(&self.builder, array, item, position)?
		};
		let fallback = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		let merged = self
			.builder
			.build_phi(self.pointer_type(), "indexed_item")?;
		merged.add_incoming(&[(&direct, first), (&pointer, fallback)]);
		let pointer = merged.as_basic_value().into_pointer_value();

		Ok(Slot {
			pointer,
			repr: item,
		})
	}

	/// `growing.push(value)`: the new item goes after the last one, in a
	/// new segment when the last is full; no item moves.
	pub(crate) fn growing_push(
		&mut self,
		array: Val<'ctx>,
		value: Val<'ctx>,
		node: NodeId,
		source: Option<NodeId>,
		receiver: Option<PointerValue<'ctx>>,
	) -> Result<(), Error> {
		let Repr::Growing(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "`push` on this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let value = self.owned_as(value, item, source)?;
		let (size, bits) = self.segment_arguments(item);
		let array = array.value.expect("a growing array").into_pointer_value();
		// `fa_segmented_push_slot` with the header tags: until a segment is
		// full, a push is one comparison, one store and one increment, and
		// consecutive pushes keep the cursor in a register (see `push_onto`)
		let (_, cursor) = self.growing_field(array, 3, "cursor")?;
		let (_, end) = self.growing_field(array, 4, "end")?;
		let room = self.builder.build_int_compare(
			IntPredicate::NE,
			cursor.into_pointer_value(),
			end.into_pointer_value(),
			"",
		)?;
		let room_block = self.builder.get_insert_block().expect("a block");
		let next = self.append_block("push_next_segment");
		let store = self.append_block("push_store");
		let branch = self.builder.build_conditional_branch(room, store, next)?;
		self.likely(branch)?;
		self.builder.position_at_end(next);
		self.call_runtime(
			"fa_segmented_next",
			&[array.into(), size.into(), bits.into()],
		)?;
		let header = match receiver {
			Some(slot) => {
				let reloaded = self
					.builder
					.build_load(self.pointer_type(), slot, "header")?;
				if let Some(instruction) =
					inkwell::values::BasicValue::as_instruction_value(&reloaded)
				{
					self.tag(instruction, false)?;
				}
				reloaded.into_pointer_value()
			}
			None => array,
		};
		let (_, next_cursor) = self.growing_field(header, 3, "cursor")?;
		let next_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(store)?;
		self.builder.position_at_end(store);
		let slot = self.builder.build_phi(self.pointer_type(), "slot")?;
		slot.add_incoming(&[(&cursor, room_block), (&next_cursor, next_end)]);
		let slot = slot.as_basic_value().into_pointer_value();
		let array = if receiver.is_some() {
			let phi = self.builder.build_phi(self.pointer_type(), "header")?;
			phi.add_incoming(&[(&array, room_block), (&header, next_end)]);
			phi.as_basic_value().into_pointer_value()
		} else {
			array
		};
		let stored = self
			.builder
			.build_store(slot, value.value.expect("an item"))?;
		self.tag(stored, false)?;
		let advanced = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), slot, &[size], "")?
		};
		let cursor_pointer = self
			.builder
			.build_struct_gep(self.segmented_type(), array, 3, "")?;
		let stored = self.builder.build_store(cursor_pointer, advanced)?;
		self.tag(stored, true)?;
		let (length_pointer, length) = self.growing_field(array, 0, "length")?;
		let length = self.builder.build_int_nsw_add(
			length.into_int_value(),
			self.context.i64_type().const_int(1, false),
			"",
		)?;
		let stored = self.builder.build_store(length_pointer, length)?;
		self.tag(stored, true)?;
		Ok(())
	}

	/// The first item of the segment holding item `index`, storing the end
	/// of that segment in `end` (an `alloca` of a pointer).
	pub(crate) fn growing_run(
		&self,
		array: PointerValue<'ctx>,
		item: Repr,
		index: IntValue<'ctx>,
		end: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let (size, bits) = self.segment_arguments(item);
		Ok(self
			.call_value(
				self.runtime.function("fa_segmented_run")?,
				&[
					array.into(),
					index.into(),
					size.into(),
					bits.into(),
					end.into(),
				],
			)?
			.into_pointer_value())
	}

	/// An `alloca` holding a pointer, in the entry block.
	pub(crate) fn pointer_alloca(&self, name: &str) -> Result<PointerValue<'ctx>, Error> {
		let builder = self.context.create_builder();
		let allocas = self.frame().allocas;
		match allocas.get_first_instruction() {
			Some(first) => builder.position_before(&first),
			None => builder.position_at_end(allocas),
		}
		Ok(builder.build_alloca(self.pointer_type(), name)?)
	}

	/// The item at `position` of a loop over a `GrowingArray`, walking each
	/// segment with a pointer (`cursor`, `end`: `alloca`s, equal when the
	/// segment of `position` must be found): one comparison per item, the
	/// segment found once per segment.
	pub(crate) fn growing_next_item(
		&mut self,
		array: PointerValue<'ctx>,
		item: Repr,
		position: IntValue<'ctx>,
		(cursor, end): (PointerValue<'ctx>, PointerValue<'ctx>),
	) -> Result<PointerValue<'ctx>, Error> {
		let current = self
			.builder
			.build_load(self.pointer_type(), cursor, "cursor")?
			.into_pointer_value();
		let stop = self
			.builder
			.build_load(self.pointer_type(), end, "end")?
			.into_pointer_value();
		let inside = self
			.builder
			.build_int_compare(IntPredicate::NE, current, stop, "")?;
		let before = self.builder.get_insert_block().expect("a block");
		let find = self.append_block("next_segment");
		let found = self.append_block("segment_item");
		let branch = self.builder.build_conditional_branch(inside, found, find)?;
		self.likely(branch)?;
		self.builder.position_at_end(find);
		let first = self.growing_run(array, item, position, end)?;
		let find_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(found)?;
		self.builder.position_at_end(found);
		let pointer = self.builder.build_phi(self.pointer_type(), "item")?;
		pointer.add_incoming(&[(&current, before), (&first, find_end)]);
		let pointer = pointer.as_basic_value().into_pointer_value();
		let next = unsafe {
			self.builder.build_in_bounds_gep(
				self.context.i8_type(),
				pointer,
				&[self.item_size_const(item)],
				"",
			)?
		};
		self.builder.build_store(cursor, next)?;
		Ok(pointer)
	}

	/// `growing = []` on a place holding a `GrowingArray`: its items dropped,
	/// its segments kept for the next pushes (its references are invalid
	/// anyway).
	pub(crate) fn growing_reset(&self, array: PointerValue<'ctx>, item: Repr) -> Result<(), Error> {
		if self.needs_drop(item) {
			let length = self.growing_length(&self.builder, array)?;
			let function = self.frame().function;
			let item_llvm = self.llvm_type(item);
			self.helper_loop(&self.builder, function, length, |builder, index| {
				let pointer = self.growing_at(builder, array, item, index)?;
				let loaded = builder.build_load(item_llvm, pointer, "")?;
				self.call_drop(builder, item, loaded)
			})?;
		}
		let (size, bits) = self.segment_arguments(item);
		self.call_runtime(
			"fa_segmented_reset",
			&[array.into(), size.into(), bits.into()],
		)
	}

	/// The drop of a `GrowingArray`: its items, its segments, its header.
	pub(crate) fn build_growing_drop(
		&self,
		builder: &Builder<'ctx>,
		function: inkwell::values::FunctionValue<'ctx>,
		item_type: TypeId,
		array: PointerValue<'ctx>,
	) -> Result<(), Error> {
		let item = self.item_repr(item_type, NodeId::new(0))?;
		let metadata = builder.build_struct_gep(self.segmented_type(), array, 1, "")?;
		let field = builder.build_struct_gep(self.segment_metadata_type(), metadata, 1, "")?;
		let storage = builder
			.build_load(self.context.i16_type(), field, "")?
			.into_int_value();
		let owned = builder.build_int_compare(
			IntPredicate::NE,
			storage,
			storage.get_type().const_int(2, false),
			"",
		)?;
		self.helper_when(builder, function, owned, |builder| {
			builder.build_return(None)?;
			Ok(())
		})?;
		if self.needs_drop(item) {
			let length = self.growing_length(builder, array)?;
			let item_llvm = self.llvm_type(item);
			self.helper_loop(builder, function, length, |builder, index| {
				let pointer = self.growing_at(builder, array, item, index)?;
				let loaded = builder.build_load(item_llvm, pointer, "")?;
				self.call_drop(builder, item, loaded)
			})?;
		}
		let (size, bits) = self.segment_arguments(item);
		builder.build_call(
			self.runtime.function("fa_segmented_free")?,
			&[array.into(), size.into(), bits.into()],
			"",
		)?;
		Ok(())
	}

	/// The copy of a `GrowingArray`: a new one with copies of the items.
	pub(crate) fn build_growing_copy(
		&self,
		builder: &Builder<'ctx>,
		function: inkwell::values::FunctionValue<'ctx>,
		item_type: TypeId,
		array: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let item = self.item_repr(item_type, NodeId::new(0))?;
		let (size, bits) = self.segment_arguments(item);
		let copy = builder
			.build_call(self.runtime.function("fa_segmented_new")?, &[], "")?
			.try_as_basic_value()
			.basic()
			.expect("a growing array")
			.into_pointer_value();
		let length = self.growing_length(builder, array)?;
		builder.build_call(
			self.runtime.function("fa_segmented_reserve")?,
			&[copy.into(), length.into(), size.into(), bits.into()],
			"",
		)?;
		let item_llvm = self.llvm_type(item);
		self.helper_loop(builder, function, length, |builder, index| {
			let pointer = self.growing_at(builder, array, item, index)?;
			let loaded = builder.build_load(item_llvm, pointer, "")?;
			let copied = self.call_copy(builder, item, loaded)?;
			let slot = builder
				.build_call(
					self.runtime.function("fa_segmented_push_slot")?,
					&[copy.into(), size.into(), bits.into()],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a slot")
				.into_pointer_value();
			builder.build_store(slot, copied)?;
			Ok(())
		})?;
		Ok(copy)
	}
}
