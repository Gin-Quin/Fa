//! `GrowingArray`: a segmented list whose items never move
//! (growing-collections.md, implementation notes), so that a reference into
//! it stays valid while it grows (`use first = growing[0]` then `push`).
//!
//! A `GrowingArray` is a pointer to a `FaSegmented` of the runtime
//! (`fa_runtime.c`, "segmented storage"): `{ i64 length, i64 segments,
//! ptr segment[48] }`. The first segment holds `2^k` items (about 64 bytes,
//! [`Lowering::segment_bits`]), each next one doubles the capacity; item
//! `i` is found with one `clz` (`fa_segmented_at`).
//!
//! Indexing, `push`, `length`, `isEmpty`, `get`, `for` loops, drops and
//! copies work on the segments. The other (read-only) methods run on a
//! *view*: the items copied bytewise into a temporary array that doesn't
//! own them, freed at the end of the statement (`Cleanup::Shallow`).

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, TypeId};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::StructType;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

/// The number of segment pointers of a `FaSegmented` (`FA_SEGMENTS`).
const SEGMENTS: u32 = 48;

impl<'ctx> Lowering<'_, 'ctx> {
	/// `{ i64 length, i64 segments, [48 x ptr] }`.
	pub(crate) fn segmented_type(&self) -> StructType<'ctx> {
		let i64 = self.context.i64_type();
		self.context.struct_type(
			&[
				i64.into(),
				i64.into(),
				self.pointer_type().array_type(SEGMENTS).into(),
			],
			false,
		)
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
		let (size, bits) = self.segment_arguments(item);
		Ok(builder
			.build_call(
				self.runtime.function("fa_segmented_at")?,
				&[array.into(), index.into(), size.into(), bits.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an item")
			.into_pointer_value())
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
		let position = match index_node {
			Some(index_node) if !self.may_be_negative(index_node) => index,
			_ => self.counted_from_end(index, length)?,
		};
		if self.flags(node).contains(NodeFlags::INDEX_CHECK) {
			let outside =
				self.builder
					.build_int_compare(IntPredicate::UGE, position, length, "")?;
			self.guard(outside, |this| {
				this.raise_runtime("fa_error_index", &[index.into(), length.into()])
			})?;
		}
		let pointer = self.growing_at(&self.builder, array, item, position)?;
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
	) -> Result<(), Error> {
		let Repr::Growing(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "`push` on this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let value = self.owned_as(value, item, source)?;
		let (size, bits) = self.segment_arguments(item);
		let slot = self
			.call_value(
				self.runtime.function("fa_segmented_push_slot")?,
				&[
					array.value.expect("a growing array").into(),
					size.into(),
					bits.into(),
				],
			)?
			.into_pointer_value();
		self.builder
			.build_store(slot, value.value.expect("an item"))?;
		Ok(())
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
