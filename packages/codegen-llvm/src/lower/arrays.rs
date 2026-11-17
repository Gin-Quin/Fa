//! Arrays: a pointer to a header `{ ptr items, i64 length, i64 capacity }`
//! whose items are stored inline in a block of the allocator.
//!
//! Owning stores, explicit copies and moves use the ownership lowering.
//! A parameter borrows the caller's array (read-only, or `mutates` it).

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{NodeList, View};
use inkwell::IntPredicate;
use inkwell::types::StructType;
use inkwell::values::{BasicValue, InstructionOpcode, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

/// The marker of an immutable array in read-only data, after its header
/// (`ARRAY_STATIC` of the runtime): neither its items nor its blocks are freed.
pub(crate) const ARRAY_STATIC_MARKER: u64 = 32;
/// The marker of an array in a function's frame (`ARRAY_FRAME` of the
/// runtime): its items are dropped, its block is never freed.
pub(crate) const ARRAY_FRAME_MARKER: u64 = 64;

impl<'ctx> Lowering<'_, 'ctx> {
	fn array_capacities_unobserved(&self) -> bool {
		if self.c_library.is_some() {
			return false;
		}
		if let Some(facts) = &self.program_facts {
			return facts
				.consulted(crate::facts::ARRAY_CAPACITIES, self.fact_log.as_deref())
				.contains(fa_analyzer::capacities::UNOBSERVED);
		}
		*self.array_capacities_unobserved.get_or_init(|| {
			if !self.whole_program {
				return false;
			}
			let mut files: Vec<_> = self.other_files.keys().copied().collect();
			files.push(self.file);
			fa_analyzer::capacities::resolve_claims(fa_analyzer::capacities::unit_claims(
				self.session,
				&files,
			))
			.contains(fa_analyzer::capacities::UNOBSERVED)
		})
	}

	/// Compact ASCII character collections materialize only when actual
	/// string slots (mutation, references, generic iteration) are needed.
	pub(crate) fn materialize_array(
		&self,
		array: PointerValue<'ctx>,
		item: Repr,
	) -> Result<(), Error> {
		if item == Repr::Str {
			self.builder.build_call(
				self.runtime.function("fa_array_materialize")?,
				&[array.into()],
				"",
			)?;
		}
		Ok(())
	}

	/// `{ ptr items, i64 length, i64 capacity }`.
	pub(crate) fn header_type(&self) -> StructType<'ctx> {
		let i64 = self.context.i64_type();
		self.context
			.struct_type(&[self.pointer_type().into(), i64.into(), i64.into()], false)
	}

	/// The size of an item, in bytes, as an `i64`.
	pub(crate) fn item_size(&self, item: Repr) -> Result<IntValue<'ctx>, Error> {
		let size = inkwell::types::BasicType::size_of(&self.llvm_type(item)).expect("a sized item");
		Ok(self
			.builder
			.build_int_cast(size, self.context.i64_type(), "")?)
	}

	pub(crate) fn item_repr(&self, item: TypeId, node: NodeId) -> Result<Repr, Error> {
		if let Some(root) = self.compressed_union_item(item) {
			return Ok(Repr::Rec(root));
		}
		// the items of an empty literal (`[]`, an `Array(Never)`) never exist
		match self.repr_of(item, node.index())? {
			Repr::Never | Repr::Void => Ok(Repr::Bool),
			// A collection provides this value's storage even when some of
			// its own fields still require boxes to break direct cycles.
			Repr::Rec(target)
				if self.types().contains_inline_array(target)
					&& !self.recursive_layout_is_finite(target) =>
			{
				self.repr_of(item, node.index())
			}
			Repr::Rec(target) => self.repr_of(target, node.index()),
			repr => Ok(repr),
		}
	}

	/// A new array of `length` items (uninitialized) and at least that capacity.
	pub(crate) fn new_array(
		&mut self,
		item: Repr,
		length: IntValue<'ctx>,
		capacity: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let size = self.item_size(item)?;
		let function = self.runtime.function("fa_array_new")?;
		Ok(self
			.call_value(function, &[length.into(), capacity.into(), size.into()])?
			.into_pointer_value())
	}

	/// A fresh result constructed in a bounded caller destination, if offered.
	fn new_array_into(
		&mut self,
		node: NodeId,
		item: Repr,
		length: IntValue<'ctx>,
		capacity: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let Some(buffer) = self.collection_call_buffer(node)? else {
			return self.new_array(item, length, capacity);
		};
		let size = self.item_size(item)?;
		Ok(self
			.call_value(
				self.runtime.function("fa_array_new_into")?,
				&[
					length.into(),
					capacity.into(),
					size.into(),
					buffer.pointer.into(),
					buffer.bytes.into(),
				],
			)?
			.into_pointer_value())
	}

	/// Substitute only a loaded source slot with agreeing exact-length facts.
	/// Coalescing/inlining can map several symbols to the same address; every
	/// such binding must qualify. Storage itself may still move on reserve.
	fn fixed_array_extent(&self, array: PointerValue<'ctx>) -> Option<u64> {
		if self.c_library.is_some() {
			return None;
		}
		let load = array.as_instruction_value()?;
		if load.get_opcode() != InstructionOpcode::Load {
			return None;
		}
		let address = load.get_operand(0)?.value()?.into_pointer_value();
		let mut length = None;
		for (&symbol, slot) in self.slots.iter().chain(&self.global_slots) {
			if slot.pointer != address {
				continue;
			}
			if !matches!(slot.repr, Repr::Array(_)) {
				return None;
			}
			let known = self.fixed_array_length(symbol)?;
			if length.is_some_and(|prior| prior != known) {
				return None;
			}
			length = Some(known);
		}
		length
	}

	/// The items pointer and the length of an array.
	pub(crate) fn array_parts(
		&self,
		array: PointerValue<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		// read before the counted loop being lowered (`hoisting.rs`)
		if let Some(&parts) = self.hoisted_parts.get(&array) {
			return Ok(parts);
		}
		let header = self.header_type();
		let items_pointer = self.builder.build_struct_gep(header, array, 0, "")?;
		let items = self
			.builder
			.build_load(self.pointer_type(), items_pointer, "items")?;
		let length = if let Some(length) = self.fixed_array_extent(array) {
			self.context.i64_type().const_int(length, false).into()
		} else {
			let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
			self.builder
				.build_load(self.context.i64_type(), length_pointer, "length")?
		};
		for loaded in [items, length] {
			if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&loaded) {
				self.tag(instruction, true)?;
			}
		}
		Ok((items.into_pointer_value(), length.into_int_value()))
	}

	/// A pointer to item `index` (not checked).
	pub(crate) fn item_pointer(
		&self,
		items: PointerValue<'ctx>,
		item: Repr,
		index: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(unsafe {
			self.builder
				.build_in_bounds_gep(self.llvm_type(item), items, &[index], "")?
		})
	}

	/// An owned array stored into a value being built (a field of a literal,
	/// the data of a member) from a local, usually built by pushes: when
	/// small, its items move into the block of its header, of their exact
	/// size (`fa_array_compact`), so that reading it takes one block instead
	/// of two, without the room its growth left.
	pub(crate) fn compact_moved_array(
		&mut self,
		value: Val<'ctx>,
		source: Option<NodeId>,
	) -> Result<Val<'ctx>, Error> {
		if self.frame().arena && self.arena_child_array_type(value.repr) {
			let Repr::Array(item_type) = value.repr else {
				unreachable!("a child array")
			};
			let item = self.item_repr(item_type, source.unwrap_or(NodeId::new(0)))?;
			let size = self.item_size(item)?;
			let array = self.call_value(
				self.runtime.function("fa_array_into_region")?,
				&[value.value.expect("an array").into(), size.into()],
			)?;
			return Ok(Val::new(value.repr, array));
		}
		let (Repr::Array(item_type), Some(source), Some(raw)) = (value.repr, source, value.value)
		else {
			return Ok(value);
		};
		if !matches!(self.view(source), View::Name(_)) || self.packed_layout(item_type).is_some() {
			return Ok(value);
		}
		let item = self.item_repr(item_type, source)?;
		let size = self.item_size(item)?;
		let compact = self.runtime.function("fa_array_compact")?;
		let compacted = self.call_value(compact, &[raw.into(), size.into()])?;
		Ok(Val::new(value.repr, compacted))
	}

	/// `[a, b, c]`.
	pub(crate) fn array_literal(
		&mut self,
		node: NodeId,
		items: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		if let Repr::Map(_) | Repr::Set(_) = repr {
			return self.table_literal(node, repr, items);
		}
		if let Repr::Tuple(ty) = repr
			&& self.inline_info(ty).is_some()
		{
			return self.inline_literal(node, ty, items);
		}
		// `[1.0, 2.0, 3.0]` as a `CArray(Float32, 3)`: its items inline
		if let Repr::Tuple(ty) = repr
			&& self.c_array_count(ty).is_some()
		{
			let structure = self.tuple_type(ty);
			let item_types = self.tuple_items(ty);
			let mut value = structure.get_undef();
			for (index, (item_node, item_type)) in items.iter().zip(item_types).enumerate() {
				let item = self.repr_of(item_type, item_node.index())?;
				let lowered = self.expr(item_node)?;
				if lowered.repr == Repr::Never {
					return Ok(lowered);
				}
				let lowered = self.owned_as(lowered, item, Some(item_node))?;
				if let Some(raw) = lowered.value {
					value = self
						.builder
						.build_insert_value(value, raw, index as u32, "")?
						.into_struct_value();
				}
			}
			return Ok(Val::new(repr, value));
		}
		let (Repr::Array(item_type)
		| Repr::Growing(item_type)
		| Repr::Pool(item_type)
		| Repr::Columnar(item_type)) = repr
		else {
			return Err(self.unsupported(node.index(), "this array"));
		};
		if items.is_empty()
			&& matches!(repr, Repr::Array(_))
			&& self.packed_layout(item_type).is_some()
		{
			return self.packed_new(item_type);
		}
		let item = self.item_repr(item_type, node)?;
		if matches!(repr, Repr::Array(_) | Repr::Growing(_)) {
			return self.linear_literal(node, items, repr, item);
		}
		let mut values = Vec::new();
		for value in items.iter() {
			if matches!(self.view(value), View::Spread(_)) {
				return Err(self.unsupported(value.index(), "spreading into an array"));
			}
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let owned = self.owned_as(lowered, item, Some(value))?;
			values.push((self.fresh(owned), value));
		}
		for (value, _) in &values {
			self.take(*value, None)?;
		}
		self.pool_literal(values, repr, node)
	}

	fn literal_array_storage(
		&self,
		buffer: PointerValue<'ctx>,
		capacity: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let zero = self.context.i64_type().const_zero();
		let items = unsafe {
			self.builder.build_gep(
				self.context.i8_type(),
				buffer,
				&[self.context.i64_type().const_int(32, false)],
				"collection.items",
			)?
		};
		let fields: [inkwell::values::BasicValueEnum; 3] =
			[items.into(), zero.into(), capacity.into()];
		for (index, value) in fields.into_iter().enumerate() {
			let field =
				self.builder
					.build_struct_gep(self.header_type(), buffer, index as u32, "")?;
			self.builder.build_store(field, value)?;
		}
		let marker = unsafe {
			self.builder.build_gep(
				self.context.i8_type(),
				buffer,
				&[self.context.i64_type().const_int(24, false)],
				"collection.marker",
			)?
		};
		self.builder.build_store(
			marker,
			self.context.i64_type().const_int(ARRAY_FRAME_MARKER, false),
		)?;
		Ok(buffer)
	}

	/// A fresh child array of a root region (`trees.rs`): its header and its
	/// first four slots in the builder's frame, until a member takes it.
	fn region_local_array(
		&mut self,
		item: Repr,
		count: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let size = self.item_size(item)?;
		let bytes = 32 + 4 * self.target_data().get_abi_size(&self.llvm_type(item));
		let packet = self.alloca_type(
			self.context
				.i64_type()
				.array_type((bytes / 8) as u32)
				.into(),
			"child_array",
		)?;
		let zero = self.context.i64_type().const_zero();
		self.call_runtime(
			"fa_array_init_region_local",
			&[packet.into(), zero.into(), count.into(), size.into()],
		)?;
		Ok(packet)
	}

	/// Register the completed prefix before evaluating another item. A fresh
	/// aggregate-producing call can use its actual element address.
	fn linear_literal(
		&mut self,
		node: NodeId,
		items: NodeList<'_>,
		repr: Repr,
		item: Repr,
	) -> Result<Val<'ctx>, Error> {
		let count = self.context.i64_type().const_int(items.len() as u64, false);
		let zero = self.context.i64_type().const_zero();
		let growing = matches!(repr, Repr::Growing(_));
		let array = if growing {
			let size = self.collection_data_size(self.llvm_type(item));
			let capacity = (items.len() as u64)
				.next_power_of_two()
				.max(1u64 << self.segment_bits(item));
			let header = self.collection_data_size(self.segmented_type().into());
			let bytes = ((header + 15) & !15) + capacity * size;
			let buffer = if let Some(offer) = self.forwarded_collection_buffer(node) {
				Some(offer)
			} else {
				self.collection_stack(node, bytes)?.map(|pointer| {
					super::collection_storage::CollectionBuffer {
						pointer,
						bytes: self.usize.const_int(bytes, false),
					}
				})
			};
			let (pointer, available) = buffer
				.map(|o| (o.pointer, o.bytes))
				.unwrap_or((self.pointer_type().const_null(), self.usize.const_zero()));
			let array = self
				.call_value(
					self.runtime.function("fa_segmented_new_into")?,
					&[
						count.into(),
						self.item_size_const(item).into(),
						self.segment_k(item).into(),
						pointer.into(),
						available.into(),
					],
				)?
				.into_pointer_value();
			let length = self
				.builder
				.build_struct_gep(self.segmented_type(), array, 0, "")?;
			self.builder.build_store(length, zero)?;
			array
		} else if self.region_child_array(node, repr) {
			self.region_local_array(item, count)?
		} else if let Some(offer) = self.forwarded_collection_buffer(node) {
			let size = self.item_size(item)?;
			self.call_value(
				self.runtime.function("fa_array_new_into")?,
				&[
					zero.into(),
					count.into(),
					size.into(),
					offer.pointer.into(),
					offer.bytes.into(),
				],
			)?
			.into_pointer_value()
		} else if let Some(buffer) = self.collection_stack(
			node,
			32 + self.collection_data_size(self.llvm_type(item)) * items.len() as u64,
		)? {
			self.literal_array_storage(buffer, count)?
		} else {
			self.new_array(item, zero, count)?
		};
		let result = self.fresh(Val::new(repr, array));
		for (index, value) in items.iter().enumerate() {
			if matches!(self.view(value), View::Spread(_)) {
				return Err(self.unsupported(value.index(), "spreading into an array"));
			}
			let position = self.context.i64_type().const_int(index as u64, false);
			let pointer = if growing {
				self.growing_at(&self.builder, array, item, position)?
			} else {
				let (items, _) = self.array_parts(array)?;
				self.item_pointer(items, item, position)?
			};
			let slot = Slot {
				pointer,
				repr: item,
			};
			let lowered = self.expr_with_destination(value, Some(slot))?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let owned = self.owned_as(lowered, item, Some(value))?;
			self.store(slot, owned, value)?;
			let length = if growing {
				self.builder
					.build_struct_gep(self.segmented_type(), array, 0, "")?
			} else {
				self.builder
					.build_struct_gep(self.header_type(), array, 1, "")?
			};
			self.builder.build_store(
				length,
				self.context.i64_type().const_int(index as u64 + 1, false),
			)?;
		}
		Ok(result)
	}

	/// `Array.from(source, capacity = n)`: a copy of the source (already
	/// copied), with room for `capacity` items.
	pub(crate) fn array_from_copy_into(
		&mut self,
		node: NodeId,
		info: &fa_analyzer::analysis::CallInfo,
	) -> Result<Option<Val<'ctx>>, Error> {
		use fa_analyzer::analysis::Argument;
		let Some(Argument::Given(source)) = info.arguments.first() else {
			return Ok(None);
		};
		// A live borrowed source needs a value copy anyway. Construct that
		// copy directly; never relocate an already owned/movable heap result.
		// Capacity hints retain their snapshot-before-hint evaluation path.
		if info
			.arguments
			.iter()
			.skip(1)
			.any(|a| !matches!(a, Argument::Default))
			|| !self.is_place(*source)
		{
			return Ok(None);
		}
		let repr = self.node_repr(node)?;
		let Repr::Array(item_type) = repr else {
			return Ok(None);
		};
		if self.node_repr(*source)? != repr || self.movable_local(*source, repr).is_some() {
			return Ok(None);
		}
		let Some(buffer) = self.collection_call_buffer(node)? else {
			return Ok(None);
		};
		let source_value = self.expr(*source)?;
		if source_value.repr == Repr::Never {
			return Ok(Some(source_value));
		}
		let item = self.item_repr(item_type, node)?;
		let (array, _) = self.array_of(source_value, node)?;
		self.materialize_array(array, item)?;
		let (_, length) = self.array_parts(array)?;
		let size = self.item_size(item)?;
		let result = self
			.call_value(
				self.runtime.function("fa_array_new_into")?,
				&[
					self.context.i64_type().const_zero().into(),
					length.into(),
					size.into(),
					buffer.pointer.into(),
					buffer.bytes.into(),
				],
			)?
			.into_pointer_value();
		let result_value = self.fresh(Val::new(repr, result));
		self.for_each_index(length, |this, index| {
			let source = this.array_item_slot(array, item, index)?;
			let value = this.load(source)?;
			let copied = this.emit_copy(value)?;
			let (items, _) = this.array_parts(result)?;
			let target = this.item_pointer(items, item, index)?;
			this.builder
				.build_store(target, copied.value.expect("an item"))?;
			let completed = this.builder.build_int_add(
				index,
				this.context.i64_type().const_int(1, false),
				"",
			)?;
			let length_slot = this
				.builder
				.build_struct_gep(this.header_type(), result, 1, "")?;
			this.builder.build_store(length_slot, completed)?;
			Ok(())
		})?;
		Ok(Some(result_value))
	}

	pub(crate) fn array_from(
		&mut self,
		node: NodeId,
		(source, capacity): (Val<'ctx>, Option<Val<'ctx>>),
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let Repr::Array(item_type) = repr else {
			return Err(self.unsupported(node.index(), "`Array.from`"));
		};
		let item = self.item_repr(item_type, node)?;
		let source = if matches!(source.repr, Repr::Pool(_) | Repr::Columnar(_)) {
			let source = self.fresh(source);
			let view = self.pool_view(source, node)?;
			self.emit_copy(view)?
		} else {
			source
		};
		let source = if let Repr::Growing(_) = source.repr {
			// taken: its items move into an array
			let source = self.fresh(source);
			let converted = self.convert(source, repr, node.index())?;
			self.take(converted, None)?
		} else {
			source
		};
		let array = source.value.expect("an array").into_pointer_value();
		if let Some(capacity) = capacity {
			let capacity = self
				.convert(capacity, Repr::Int(IntRepr::I64), node.index())?
				.int();
			let size = self.item_size(item)?;
			self.call_runtime(
				"fa_array_reserve",
				&[array.into(), capacity.into(), size.into()],
			)?;
		}
		Ok(self.fresh(Val::new(repr, array)))
	}

	/// `array.length`.
	pub(crate) fn array_length(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let array = value.value.expect("an array").into_pointer_value();
		let (_, length) = self.array_parts(array)?;
		Ok(Val::new(Repr::Int(IntRepr::I64), length))
	}

	/// The pointer to an item, after the bounds check when the node has
	/// `INDEX_CHECK` (an `IndexError`).
	pub(super) fn checked_item(
		&mut self,
		node: NodeId,
		array: PointerValue<'ctx>,
		item: Repr,
		index: Val<'ctx>,
		index_node: Option<NodeId>,
	) -> Result<PointerValue<'ctx>, Error> {
		self.materialize_array(array, item)?;
		let index = self
			.convert(index, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let (items, length) = self.array_parts(array)?;
		// an index of the array by a whole-program proof (`index_proofs.rs`)
		if self.flags(node).contains(NodeFlags::INDEX_CHECK) && self.proven_index(node) {
			return self.item_pointer(items, item, index);
		}
		let may_be_negative = index_node.is_none_or(|index_node| self.may_be_negative(index_node));
		if self.flags(node).contains(NodeFlags::INDEX_CHECK) {
			// `0 <= index < length` first, the form that LLVM's range check
			// elimination (`irce`) takes out of loops; a negative index
			// (counted from the end) or an error in a cold block
			let inside = self
				.builder
				.build_int_compare(IntPredicate::ULT, index, length, "")?;
			if !may_be_negative {
				let outside = self.builder.build_not(inside, "")?;
				self.guard(outside, |this| {
					this.raise_runtime("fa_error_index", &[index.into(), length.into()])
				})?;
				return self.item_pointer(items, item, index);
			}
			let before = self.builder.get_insert_block().expect("a block");
			let found = self.append_block("index_found");
			let other = self.append_block("index_from_end");
			let branch = self
				.builder
				.build_conditional_branch(inside, found, other)?;
			self.likely(branch)?;
			self.builder.position_at_end(other);
			let from_end = self.runtime.function("fa_index_from_end")?;
			let from_end = self
				.call_value(from_end, &[index.into(), length.into()])?
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
			let position = position.as_basic_value().into_int_value();
			return self.item_pointer(items, item, position);
		}
		// a negative index counts from the end
		let position = if may_be_negative {
			self.counted_from_end(index, length)?
		} else {
			index
		};
		self.item_pointer(items, item, position)
	}

	/// `index < 0 ? length + index : index`.
	pub(crate) fn counted_from_end(
		&self,
		index: IntValue<'ctx>,
		length: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let negative = self.builder.build_int_compare(
			IntPredicate::SLT,
			index,
			index.get_type().const_zero(),
			"",
		)?;
		let from_end = self.builder.build_int_add(length, index, "")?;
		Ok(self
			.builder
			.build_select(negative, from_end, index, "")?
			.into_int_value())
	}

	/// Whether an integer node may hold a negative value (its facts, else its
	/// type), unless the places it reads are proven never negative
	/// (`index_proofs.rs`).
	pub(crate) fn may_be_negative(&self, node: NodeId) -> bool {
		let negative = if let Some(interval) = self.reliable_interval(node) {
			interval.lo < ethnum::I256::ZERO
		} else {
			match self.types().as_int(self.type_of(node)) {
				Some(range) => range.min < ethnum::I256::ZERO,
				None => true,
			}
		};
		negative && !self.proven_non_negative(node)
	}

	/// The place `array[index]`.
	pub(crate) fn index_place(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let array = self.expr(target)?;
		if array.repr == Repr::Bytes {
			return self.bytes_index_place(node, array, index);
		}
		if let Repr::Growing(_) = array.repr {
			let index_node = index;
			let index = self.expr(index)?;
			return self.growing_item(node, array, index, Some(index_node));
		}
		let Repr::Array(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "indexing this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let index_node = index;
		let index = self.expr(index)?;
		let array = array.value.expect("an array").into_pointer_value();
		let pointer = self.checked_item(node, array, item, index, Some(index_node))?;
		Ok(Slot {
			pointer,
			repr: item,
		})
	}

	/// `array[index]`.
	pub(crate) fn index_read(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let slot = self.index_place(node, target, index)?;
		self.load(slot)
	}

	/// `array.push(item)`.
	pub(crate) fn array_push(
		&mut self,
		array: Val<'ctx>,
		value: Val<'ctx>,
		node: NodeId,
		source: Option<NodeId>,
	) -> Result<(), Error> {
		self.push_onto(array, value, node, source, None)
	}

	/// The slot holding the array of `place.push(argument)`, when the place
	/// is a local or a parameter and its plain fields: its header was just
	/// loaded from that slot, and the argument assigns nothing.
	fn push_receiver_slot(
		&mut self,
		call: NodeId,
		argument: NodeId,
	) -> Result<Option<PointerValue<'ctx>>, Error> {
		let View::Call { callee, .. } = self.view(call) else {
			return Ok(None);
		};
		let View::Access { target, members } = self.view(callee) else {
			return Ok(None);
		};
		let members: Vec<NodeId> = members.iter().collect();
		if !matches!(
			self.invariant_place(target, &members[..members.len() - 1])?,
			Some(Repr::Array(_) | Repr::Growing(_))
		) {
			return Ok(None);
		}
		let mut stack = vec![argument.index()];
		while let Some(node) = stack.pop() {
			if matches!(
				self.view(NodeId::new(node)),
				View::Assignment { .. } | View::CompoundAssignment { .. }
			) {
				return Ok(None);
			}
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		Ok(match self.receiver_place(call)? {
			Some(slot) if matches!(slot.repr, Repr::Array(_) | Repr::Growing(_)) => {
				Some(slot.pointer)
			}
			_ => None,
		})
	}

	/// `array.push(item)`; `slot`: the slot the array was loaded from
	/// ([`Self::push_receiver_slot`]).
	fn push_onto(
		&mut self,
		array: Val<'ctx>,
		value: Val<'ctx>,
		node: NodeId,
		source: Option<NodeId>,
		receiver: Option<PointerValue<'ctx>>,
	) -> Result<(), Error> {
		let Repr::Array(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "`push` on this value"));
		};
		if let Some(layout) = self.packed_layout(item_type) {
			// the item's bytes are copied: the value stays with its owner
			let array = array.value.expect("an array").into_pointer_value();
			return self.packed_push(&layout, array, value);
		}
		let item = self.item_repr(item_type, node)?;
		let value = self.owned_as(value, item, source)?;
		let array = array.value.expect("an array").into_pointer_value();
		if self.reserved_pushes.contains_key(&node) {
			// room was reserved before the loop: no capacity check
			let (items, length) = self.array_parts(array)?;
			let slot = self.item_pointer(items, item, length)?;
			let store = self.builder.build_store(
				slot,
				value
					.value
					.unwrap_or_else(|| self.context.bool_type().const_zero().into()),
			)?;
			self.tag(store, false)?;
			let length_pointer = self
				.builder
				.build_struct_gep(self.header_type(), array, 1, "")?;
			let next = self.builder.build_int_nsw_add(
				length,
				length.get_type().const_int(1, false),
				"",
			)?;
			let store = self.builder.build_store(length_pointer, next)?;
			self.tag(store, true)?;
			return Ok(());
		}
		// `fa_array_push_slot`, written here with the type-based alias tags:
		// the header (`fa header`) never shares memory with an item, so a
		// loop of pushes keeps the header pointer, the length, the capacity
		// and the items pointer in registers, reloading them only after the
		// cold call that grows a full array (it never changes the length).
		let header = self.header_type();
		let i64 = self.context.i64_type();
		let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
		let length = self.builder.build_load(i64, length_pointer, "length")?;
		let capacity_pointer = self.builder.build_struct_gep(header, array, 2, "")?;
		let capacity = self.builder.build_load(i64, capacity_pointer, "capacity")?;
		for loaded in [length, capacity] {
			if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&loaded) {
				self.tag(instruction, true)?;
			}
		}
		let length = length.into_int_value();
		let room = self.builder.build_int_compare(
			IntPredicate::SLT,
			length,
			capacity.into_int_value(),
			"",
		)?;
		let room_block = self.builder.get_insert_block().expect("a block");
		let grow = self.append_block("push_grow");
		let store = self.append_block("push_store");
		let branch = self.builder.build_conditional_branch(room, store, grow)?;
		self.likely(branch)?;
		self.builder.position_at_end(grow);
		let size = self.item_size(item)?;
		// a fresh child array of a root region has four slots in its block
		let region = matches!(self.view(node), View::Call { callee, .. }
			if matches!(self.view(callee), View::Access { target, .. }
				if self.region_child_array(target, Repr::Array(item_type))));
		let grow = if region {
			"fa_array_region_grow"
		} else {
			"fa_array_grow"
		};
		self.call_runtime(grow, &[array.into(), size.into()])?;
		// The call doesn't change the slot, but LLVM can't know it: read it
		// again, so that the next pushes find the header (then its length,
		// its capacity, its items) in registers on both paths.
		let reloaded = match receiver {
			Some(slot) => {
				let reloaded = self
					.builder
					.build_load(self.pointer_type(), slot, "header")?;
				if let Some(instruction) =
					inkwell::values::BasicValue::as_instruction_value(&reloaded)
				{
					self.tag(instruction, false)?;
				}
				Some((reloaded, self.builder.get_insert_block().expect("a block")))
			}
			None => None,
		};
		self.builder.build_unconditional_branch(store)?;
		self.builder.position_at_end(store);
		let array = match reloaded {
			Some((reloaded, grown)) => {
				let phi = self.builder.build_phi(self.pointer_type(), "header")?;
				phi.add_incoming(&[(&array, room_block), (&reloaded, grown)]);
				phi.as_basic_value().into_pointer_value()
			}
			None => array,
		};
		let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
		let items_pointer = self.builder.build_struct_gep(header, array, 0, "")?;
		let items = self
			.builder
			.build_load(self.pointer_type(), items_pointer, "items")?;
		if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&items) {
			self.tag(instruction, true)?;
		}
		let slot = self.item_pointer(items.into_pointer_value(), item, length)?;
		let stored = self.builder.build_store(
			slot,
			value
				.value
				.unwrap_or_else(|| self.context.bool_type().const_zero().into()),
		)?;
		self.tag(stored, false)?;
		let next = self
			.builder
			.build_int_nsw_add(length, i64.const_int(1, false), "")?;
		let stored = self.builder.build_store(length_pointer, next)?;
		self.tag(stored, true)?;
		Ok(())
	}

	/// `array.get(index)`: the item, or `none`.
	pub(super) fn array_get(
		&mut self,
		array: Val<'ctx>,
		index: Val<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Array(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "`get` on this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let Some(scalar) = Scalar::of(item) else {
			return Err(self.unsupported(node.index(), "`get` on this array"));
		};
		let repr = Repr::Optional(scalar);
		let result = self.alloca(repr, "got")?;
		let ty = self.llvm_type(repr);
		self.builder
			.build_store(result.pointer, super::zero_of(ty))?;
		let index = self
			.convert(index, Repr::Int(IntRepr::I64), node.index())?
			.int();
		if item == Repr::Str {
			let out = self.alloca(Repr::Str, "character")?;
			self.builder
				.build_store(out.pointer, self.string_type.const_zero())?;
			let pointer = array.value.expect("an array").into_pointer_value();
			let found = self
				.call_value(
					self.runtime.function("fa_string_array_get")?,
					&[pointer.into(), index.into(), out.pointer.into()],
				)?
				.into_int_value();
			let present = self.builder.build_int_compare(
				IntPredicate::NE,
				found,
				found.get_type().const_zero(),
				"",
			)?;
			let value = self.load(out)?;
			let raw = self.pack_optional(
				&self.builder,
				Scalar::Str,
				present,
				value.value.expect("a string"),
			)?;
			return Ok(Val::new(repr, raw));
		}
		let (items, length) =
			self.array_parts(array.value.expect("an array").into_pointer_value())?;
		let index = self.counted_from_end(index, length)?;
		let inside = self
			.builder
			.build_int_compare(IntPredicate::ULT, index, length, "")?;
		let present = self.append_block("get_present");
		let end = self.append_block("end_get");
		self.builder
			.build_conditional_branch(inside, present, end)?;
		self.builder.position_at_end(present);
		let pointer = self.item_pointer(items, item, index)?;
		let value = self.load(Slot {
			pointer,
			repr: item,
		})?;
		let value = self.convert(value, repr, node.index())?;
		self.builder
			.build_store(result.pointer, value.value.expect("an optional"))?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		self.load(result)
	}

	/// A method of arrays: the receiver, then the arguments.
	/// Search once and retain the actual item address, for both a value
	/// result and a builtin reference binding. Growing storage stays put.
	pub(crate) fn array_find_pointer(
		&mut self,
		collection: Val<'ctx>,
		callback: Val<'ctx>,
		callback_node: NodeId,
		node: NodeId,
	) -> Result<(PointerValue<'ctx>, Repr), Error> {
		let (Repr::Array(item_type) | Repr::Growing(item_type)) = collection.repr else {
			return Err(self.unsupported(node.index(), "finding an item of this collection"));
		};
		let item = self.item_repr(item_type, node)?;
		let pointer = collection.value.expect("a collection").into_pointer_value();
		let length = if matches!(collection.repr, Repr::Growing(_)) {
			self.growing_length(&self.builder, pointer)?
		} else {
			self.array_parts(pointer)?.1
		};
		let result = self.alloca_type(self.pointer_type().into(), "found_item")?;
		self.builder
			.build_store(result, self.pointer_type().const_null())?;
		let end = self.append_block("end_find");
		self.for_each_index(length, |this, index| {
			let index_value = Val::new(Repr::Int(IntRepr::I64), index);
			let slot = if matches!(collection.repr, Repr::Growing(_)) {
				Slot {
					pointer: this.growing_at(&this.builder, pointer, item, index)?,
					repr: item,
				}
			} else {
				this.array_item_slot(pointer, item, index)?
			};
			let holds = this.call_collection_callback(
				callback,
				callback_node,
				&[],
				&[slot],
				&[index_value],
			)?;
			let found = this.append_block("find_present");
			let next = this.append_block("find_next");
			this.builder
				.build_conditional_branch(holds.int(), found, next)?;
			this.builder.position_at_end(found);
			this.builder.build_store(result, slot.pointer)?;
			this.builder.build_unconditional_branch(end)?;
			this.builder.position_at_end(next);
			Ok(())
		})?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let result = self
			.builder
			.build_load(self.pointer_type(), result, "")?
			.into_pointer_value();
		Ok((result, item))
	}

	pub(crate) fn array_method(
		&mut self,
		node: NodeId,
		path: &str,
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
		if let Some(ty) = self.inline_receiver_type(node) {
			return self.inline_method(node, ty, path, arguments);
		}
		if path == "Array#get"
			&& let [index] = arguments
			&& let Some(text) = self.code_points_of(node)
		{
			return self.code_point_get(node, text, *index);
		}
		let array = self.receiver(node)?;
		if array.repr == Repr::Never {
			return Ok(array);
		}
		let mut values = Vec::new();
		for &argument in arguments {
			let value = self.expr(argument)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			values.push(value);
		}
		if path == "Array#find"
			&& let ([callback], [callback_node]) = (values.as_slice(), arguments)
		{
			let (pointer, item) =
				self.array_find_pointer(array, *callback, *callback_node, node)?;
			return self.optional_element(pointer, item, node);
		}
		if let Repr::Growing(item_type) = array.repr {
			match (path, values.as_slice()) {
				("Array#push", [value]) => {
					let slot = self.push_receiver_slot(node, arguments[0])?;
					self.growing_push(array, *value, node, Some(arguments[0]), slot)?;
					return Ok(Val::void());
				}
				("Array#get", [index]) => {
					// in bounds: the item, else `none`
					let item = self.item_repr(item_type, node)?;
					let Some(scalar) = Scalar::of(item) else {
						return Err(self.unsupported(node.index(), "`get` on this array"));
					};
					let repr = Repr::Optional(scalar);
					let result = self.alloca(repr, "got")?;
					self.builder
						.build_store(result.pointer, super::zero_of(self.llvm_type(repr)))?;
					let index = self
						.convert(*index, Repr::Int(IntRepr::I64), node.index())?
						.int();
					let pointer = array.value.expect("an array").into_pointer_value();
					let length = self.growing_length(&self.builder, pointer)?;
					let index = self.counted_from_end(index, length)?;
					let inside =
						self.builder
							.build_int_compare(IntPredicate::ULT, index, length, "")?;
					let present = self.append_block("get_present");
					let end = self.append_block("end_get");
					self.builder
						.build_conditional_branch(inside, present, end)?;
					self.builder.position_at_end(present);
					let pointer = self.growing_at(&self.builder, pointer, item, index)?;
					let value = self.load(Slot {
						pointer,
						repr: item,
					})?;
					let value = self.convert(value, repr, node.index())?;
					self.builder
						.build_store(result.pointer, value.value.expect("an optional"))?;
					self.builder.build_unconditional_branch(end)?;
					self.builder.position_at_end(end);
					return self.load(result);
				}
				_ => {}
			}
		}
		// the other methods read a view of a `GrowingArray`
		let array = self.growing_view(array)?;
		if !matches!(path, "Array#get" | "Array#isEmpty")
			&& let Repr::Array(item_type) = array.repr
		{
			self.materialize_array(
				array.value.expect("array").into_pointer_value(),
				self.item_repr(item_type, node)?,
			)?;
		}
		match (path, values.as_slice()) {
			("Array#push", [value]) => {
				let slot = self.push_receiver_slot(node, arguments[0])?;
				self.push_onto(array, *value, node, Some(arguments[0]), slot)?;
				Ok(Val::void())
			}
			("Array#get", [index]) => self.array_get(array, *index, node),
			("Array#isEmpty", []) => {
				let length = self.array_length(array)?.int();
				let empty = self.builder.build_int_compare(
					IntPredicate::EQ,
					length,
					length.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, empty))
			}
			_ => self.array_method_more(node, path, array, &values, arguments),
		}
	}

	/// The item at `index` of an array.
	fn load_item(
		&self,
		array: PointerValue<'ctx>,
		item: Repr,
		index: IntValue<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		self.load(self.array_item_slot(array, item, index)?)
	}

	pub(crate) fn array_item_slot(
		&self,
		array: PointerValue<'ctx>,
		item: Repr,
		index: IntValue<'ctx>,
	) -> Result<Slot<'ctx>, Error> {
		let (items, _) = self.array_parts(array)?;
		let pointer = self.item_pointer(items, item, index)?;
		Ok(Slot {
			pointer,
			repr: item,
		})
	}

	/// An index counted from the end when negative, checked against the
	/// length (`IndexError`).
	fn normalized_index(
		&mut self,
		array: PointerValue<'ctx>,
		index: Val<'ctx>,
		node: NodeId,
		inclusive_end: bool,
	) -> Result<IntValue<'ctx>, Error> {
		let index = self
			.convert(index, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let (_, length) = self.array_parts(array)?;
		let zero = index.get_type().const_zero();
		let negative = self
			.builder
			.build_int_compare(IntPredicate::SLT, index, zero, "")?;
		let from_end = self.builder.build_int_add(index, length, "")?;
		let position = self
			.builder
			.build_select(negative, from_end, index, "")?
			.into_int_value();
		let predicate = if inclusive_end {
			IntPredicate::UGT
		} else {
			IntPredicate::UGE
		};
		let outside = self
			.builder
			.build_int_compare(predicate, position, length, "")?;
		self.guard(outside, |this| {
			this.raise_runtime("fa_error_index", &[index.into(), length.into()])
		})?;
		Ok(position)
	}

	/// The items and the representation of an array value.
	pub(crate) fn array_of(
		&self,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<(PointerValue<'ctx>, Repr), Error> {
		let Repr::Array(item_type) = value.repr else {
			return Err(self.unsupported(node.index(), "this array method"));
		};
		Ok((
			value.value.expect("an array").into_pointer_value(),
			self.item_repr(item_type, node)?,
		))
	}

	/// `text.codePoints().get(…)` or `text.codePoints()[…]` (`node` the call
	/// of `get`, or the index), or the same on a local bound to it
	/// ([`Lowering::virtual_code_points_of`]): the call of `codePoints`, so
	/// that the code point is read from the text without building the array.
	pub(crate) fn code_points_of(&self, node: NodeId) -> Option<NodeId> {
		let target = match self.view(node) {
			View::Index { target, index } if self.node_repr(index).ok() != Some(Repr::Range) => {
				target
			}
			View::Call { callee, .. } => match self.view(callee) {
				View::Access { target, members } if members.len() == 1 => target,
				_ => return None,
			},
			_ => return None,
		};
		if let Some(call) = self.virtual_code_points_call(target) {
			return Some(call);
		}
		self.code_points_call(target)
	}

	/// The call of `codePoints` that a local read at `node` stands for.
	pub(crate) fn virtual_code_points_call(&self, node: NodeId) -> Option<NodeId> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(inner) else {
			return None;
		};
		self.virtual_code_points.get(&symbol).copied()
	}

	/// Whether `let codes = text.codePoints()` (`declaration`, `value`) can
	/// stay virtual: `codes` is only indexed (`codes[i]`, `codes.get(i)`) and
	/// measured (`codes.length`), and `text` is a `let`, or the variable of a
	/// `for` over a collection that nothing changes in the block of `codes`
	/// (fields of them are fine: values don't change under a `let`).
	pub(crate) fn virtual_code_points_of(
		&self,
		declaration: NodeId,
		symbol: SymbolId,
		value: NodeId,
	) -> bool {
		if self.analysis.symbols.symbol(symbol).kind != SymbolKind::Let {
			return false;
		}
		let Some(call) = self.code_points_call(value) else {
			return false;
		};
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let View::Access { target, members } = self.view(callee) else {
			return false;
		};
		// `word.codePoints()`, `say.text.codePoints()`: fields only
		if members
			.iter()
			.take(members.len() - 1)
			.any(|member| !matches!(self.analysis.resolution(member), Resolution::Field(..)))
		{
			return false;
		}
		let Resolution::Local(root) = self.analysis.resolution(target) else {
			return false;
		};
		let parents = self.parents();
		// the uses of the local
		for index in 0..self.analysis.resolutions.len() {
			if self.analysis.resolutions[index] != Resolution::Local(symbol) {
				continue;
			}
			if index == declaration.index() {
				continue;
			}
			let parent = parents.get(index).copied().unwrap_or(usize::MAX);
			if parent == usize::MAX {
				return false;
			}
			let allowed = match self.view(NodeId::new(parent)) {
				View::Index {
					target,
					index: position,
				} => target.index() == index && self.node_repr(position).ok() != Some(Repr::Range),
				View::Access { target, members }
					if target.index() == index && members.len() == 1 =>
				{
					let name = self.tree.text(members.first().expect("a member").index());
					match name {
						"length" => true,
						// `codes.get(i)`: the access is the callee of a call
						"get" => parents.get(parent).is_some_and(|&call| {
							call != usize::MAX
								&& matches!(
									self.view(NodeId::new(call)),
									View::Call { callee, .. } if callee.index() == parent
								)
						}),
						_ => false,
					}
				}
				_ => false,
			};
			if !allowed {
				return false;
			}
		}
		let kind = self.analysis.symbols.symbol(root).kind;
		match kind {
			SymbolKind::Let => true,
			SymbolKind::LoopVariable { .. } => {
				// the block of `codes` changes neither the variable nor its collection
				let mut block = declaration.index();
				while let Some(&parent) = parents.get(block) {
					if parent == usize::MAX {
						return false;
					}
					block = parent;
					if matches!(self.view(NodeId::new(block)), View::Block { .. }) {
						break;
					}
				}
				let mut stack = vec![block];
				while let Some(current) = stack.pop() {
					match self.view(NodeId::new(current)) {
						View::Assignment { .. }
						| View::CompoundAssignment { .. }
						| View::Use { .. }
						| View::Reuse { .. } => {
							// a change of a local of the block (`total += …`) is fine
							let target = match self.view(NodeId::new(current)) {
								View::Assignment { target, .. }
								| View::CompoundAssignment { target, .. } => Some(target),
								_ => None,
							};
							let local = target.is_some_and(|target| {
								matches!(self.view(target), View::Name(_))
									&& self.analysis.resolution(target) != Resolution::Local(root)
							});
							if !local {
								return false;
							}
						}
						// a method call may change its receiver
						View::Call { callee, .. }
							if matches!(self.view(callee), View::Access { target, .. }
								if self.analysis.resolution(target) != Resolution::Local(symbol))
								&& !matches!(
									self.analysis.call(NodeId::new(current)).map(|info| info.callee),
									Some(fa_analyzer::analysis::Callee::Builtin(id))
										if fa_analyzer::builtins::get(id).path.starts_with("String#")
								) =>
						{
							return false;
						}
						_ => {}
					}
					self.tree.nodes[current].for_each_child(|child| stack.push(child));
				}
				true
			}
			_ => false,
		}
	}

	/// The call `text.codePoints()` (through parentheses), or `None`.
	pub(crate) fn code_points_call(&self, node: NodeId) -> Option<NodeId> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let View::Call { arguments, .. } = self.view(inner) else {
			return None;
		};
		if !arguments.is_empty() {
			return None;
		}
		let info = self.analysis.call(inner)?;
		match info.callee {
			fa_analyzer::analysis::Callee::Builtin(id)
				if matches!(
					fa_analyzer::builtins::get(id).path,
					"String#codePoints" | "String#toUtf8"
				) =>
			{
				Some(inner)
			}
			_ => None,
		}
	}

	/// Whether a call that [`Lowering::code_points_call`] found is
	/// `text.toUtf8()` rather than `text.codePoints()`.
	pub(crate) fn is_bytes_call(&self, call: NodeId) -> bool {
		matches!(
			self.analysis.call(call).map(|info| info.callee),
			Some(fa_analyzer::analysis::Callee::Builtin(id))
				if fa_analyzer::builtins::get(id).path == "String#toUtf8"
		)
	}

	/// The integer type of the items of a call of `codePoints` (`UInt32`)
	/// or of `bytes` (`UInt8`).
	pub(crate) fn text_view_item(&self, call: NodeId) -> IntRepr {
		IntRepr {
			bits: if self.is_bytes_call(call) { 8 } else { 32 },
			signed: false,
		}
	}

	/// The encoded UTF-8 byte length of a text, as an `i64`.
	pub(crate) fn byte_length(&self, text: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let length = if let Some(length) = self.frozen_byte_length(text) {
			length
		} else {
			let (bytes, physical) = self.string_parts(text)?;
			self.call_value(
				self.runtime.function("fa_string_encoded_length")?,
				&[
					bytes.into(),
					physical.into(),
					self.string_capacity(text)?.into(),
				],
			)?
			.into_int_value()
		};
		self.int_to(
			length,
			IntRepr {
				bits: self.usize.get_bit_width(),
				signed: false,
			},
			IntRepr::I64,
		)
	}

	/// The number of items of `text.codePoints()` or `text.toUtf8()` (`call`),
	/// without the array: an `i64`.
	pub(crate) fn text_view_length(
		&mut self,
		call: NodeId,
		text: Val<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		if self.is_bytes_call(call) {
			return self.byte_length(text);
		}
		let (bytes, length) = self.string_parts(text)?;
		let capacity = self.string_capacity(text)?;
		let count = self.runtime.function("fa_string_code_point_count")?;
		Ok(self
			.call_value(count, &[bytes.into(), length.into(), capacity.into()])?
			.into_int_value())
	}

	/// The byte at `position` (an `i64`, negative from the end) of a text,
	/// zero-extended to an `i64`, or -1 outside of it.
	fn byte_at(
		&mut self,
		text: Val<'ctx>,
		position: IntValue<'ctx>,
		index: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let (bytes, physical) = self.string_parts(text)?;
		let length = self.byte_length(text)?;
		let i64 = self.context.i64_type();
		let position = if self.may_be_negative(index) {
			self.counted_from_end(position, length)?
		} else {
			position
		};
		let inside = self
			.builder
			.build_int_compare(IntPredicate::ULT, position, length, "")?;
		let before = self.builder.get_insert_block().expect("a block");
		let read = self.append_block("byte_inside");
		let end = self.append_block("byte_read");
		let branch = self.builder.build_conditional_branch(inside, read, end)?;
		self.likely(branch)?;
		self.builder.position_at_end(read);
		let offset = self.builder.build_int_cast(position, self.usize, "")?;
		let byte = self
			.call_value(
				self.runtime.function("fa_string_utf8_byte")?,
				&[
					bytes.into(),
					physical.into(),
					self.string_capacity(text)?.into(),
					offset.into(),
				],
			)?
			.into_int_value();
		self.runtime_integer_range(byte, 0, 256)?;
		let byte = self.builder.build_int_z_extend_or_bit_cast(byte, i64, "")?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let code = self.builder.build_phi(i64, "byte_or_missing")?;
		code.add_incoming(&[(&byte, read), (&i64.const_all_ones(), before)]);
		Ok(code.as_basic_value().into_int_value())
	}

	/// The code point at `index` of the receiver of `code_points` (a call
	/// of `codePoints`), or -1 (an `i64`).
	fn code_point_at(
		&mut self,
		code_points: NodeId,
		index: NodeId,
		node: NodeId,
	) -> Result<Option<(IntValue<'ctx>, IntValue<'ctx>, Val<'ctx>)>, Error> {
		let text = self.receiver(code_points)?;
		if text.repr == Repr::Never {
			return Ok(None);
		}
		let position = self.expr(index)?;
		if position.repr == Repr::Never {
			return Ok(None);
		}
		let position = self
			.convert(position, Repr::Int(IntRepr::I64), node.index())?
			.int();
		if self.is_bytes_call(code_points) {
			let byte = self.byte_at(text, position, index)?;
			return Ok(Some((byte, position, text)));
		}
		let (bytes, length) = self.string_parts(text)?;
		let capacity = self.string_capacity(text)?;
		let at = self.runtime.function("fa_string_code_point_at")?;
		let code = self
			.call_value(
				at,
				&[
					bytes.into(),
					length.into(),
					capacity.into(),
					position.into(),
				],
			)?
			.into_int_value();
		Ok(Some((code, position, text)))
	}

	/// `text.codePoints().get(index)`: a `UInt32?`.
	pub(super) fn code_point_get(
		&mut self,
		node: NodeId,
		code_points: NodeId,
		index: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some((code, _, _)) = self.code_point_at(code_points, index, node)? else {
			return Ok(Val::never());
		};
		let repr = self.node_repr(node)?;
		let Repr::Optional(scalar) = repr else {
			return Err(self.unsupported(node.index(), "`get` on code points"));
		};
		let present = self.builder.build_int_compare(
			IntPredicate::SGE,
			code,
			code.get_type().const_zero(),
			"",
		)?;
		let item = self.text_view_item(code_points);
		let value = self.builder.build_int_truncate(
			code,
			self.llvm_type(Repr::Int(item)).into_int_type(),
			"",
		)?;
		let value = self.convert(
			Val::new(Repr::Int(item), value),
			scalar.repr(),
			node.index(),
		)?;
		let ty = self.llvm_type(repr).into_struct_type();
		let pair = self
			.builder
			.build_insert_value(ty.get_undef(), present, 0, "")?;
		let pair =
			self.builder
				.build_insert_value(pair, value.value.expect("a code point"), 1, "")?;
		Ok(Val::new(repr, pair.into_struct_value()))
	}

	/// `text.codePoints()[index]`: a `UInt32`, else an `IndexError`.
	pub(crate) fn code_point_index(
		&mut self,
		node: NodeId,
		code_points: NodeId,
		index: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some((code, position, text)) = self.code_point_at(code_points, index, node)? else {
			return Ok(Val::never());
		};
		let missing = self.builder.build_int_compare(
			IntPredicate::SLT,
			code,
			code.get_type().const_zero(),
			"",
		)?;
		self.guard(missing, |this| {
			let count = this.text_view_length(code_points, text)?;
			this.raise_runtime("fa_error_index", &[position.into(), count.into()])
		})?;
		let item = self.text_view_item(code_points);
		let value = self.builder.build_int_truncate(
			code,
			self.llvm_type(Repr::Int(item)).into_int_type(),
			"",
		)?;
		Ok(Val::new(Repr::Int(item), value))
	}

	/// Whether a callback is a lambda that can't raise an error (no `try`,
	/// no `throw`), so that a loop calling it runs to its end.
	fn cannot_leave(&self, callback: NodeId) -> bool {
		if !matches!(self.view(callback), View::Lambda { .. }) {
			return false;
		}
		let mut stack = vec![callback.index()];
		while let Some(current) = stack.pop() {
			if matches!(
				self.view(NodeId::new(current)),
				View::Try(_) | View::Throw(_)
			) {
				return false;
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		true
	}

	/// A new empty array of items of this representation, with room for `capacity`.
	pub(crate) fn empty_array(
		&mut self,
		item: Repr,
		capacity: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let zero = self.context.i64_type().const_zero();
		self.new_array(item, zero, capacity)
	}

	/// The methods of arrays that take a callback or compare items.
	pub(crate) fn array_method_more(
		&mut self,
		node: NodeId,
		path: &str,
		array_value: Val<'ctx>,
		values: &[Val<'ctx>],
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
		let (array, item) = self.array_of(array_value, node)?;
		let i64 = self.context.i64_type();
		match (path, values) {
			("Array#map", [callback]) => {
				let Repr::Array(result_type) = self.node_repr(node)? else {
					return Err(self.unsupported(node.index(), path));
				};
				let result_item = self.item_repr(result_type, node)?;
				let (_, length) = self.array_parts(array)?;
				let result = self.new_array_into(node, result_item, i64.const_zero(), length)?;
				// A throwing callback drops precisely the completed prefix.
				let result_value = self.fresh(Val::new(Repr::Array(result_type), result));
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let (items, _) = this.array_parts(result)?;
					let pointer = this.item_pointer(items, result_item, index)?;
					let mapped = this.call_collection_callback_into(
						*callback,
						callback_node,
						&[],
						&[slot],
						&[index_value],
						Some(Slot {
							pointer,
							repr: result_item,
						}),
					)?;
					let mapped = this.owned_as(mapped, result_item, None)?;
					this.store(
						Slot {
							pointer,
							repr: result_item,
						},
						mapped,
						node,
					)?;
					let completed =
						this.builder
							.build_int_add(index, i64.const_int(1, false), "")?;
					let length_slot =
						this.builder
							.build_struct_gep(this.header_type(), result, 1, "")?;
					this.builder.build_store(length_slot, completed)?;
					Ok(())
				})?;
				Ok(result_value)
			}
			("Array#filter", [callback]) => {
				let (_, length) = self.array_parts(array)?;
				let callback_node = arguments[0];
				// a temporary array (`text.split(" ").filter(…)`) is filtered in
				// place: the kept items move down to the next free position, the
				// others are dropped, and the array becomes the result
				let in_place = matches!(array_value.repr, Repr::Array(_))
					&& self.is_temporary(array_value)
					&& self.cannot_leave(callback_node);
				if in_place {
					self.take(array_value, None)?;
					let kept = self.alloca(Repr::Int(IntRepr::I64), "kept")?;
					self.builder.build_store(kept.pointer, i64.const_zero())?;
					self.for_each_index(length, |this, index| {
						let slot = this.array_item_slot(array, item, index)?;
						let value = this.load(slot)?;
						let index_value = Val::new(Repr::Int(IntRepr::I64), index);
						let keep = this.call_collection_callback(
							*callback,
							callback_node,
							&[],
							&[slot],
							&[index_value],
						)?;
						let push = this.append_block("filter_keep");
						let drop = this.append_block("filter_drop");
						let next = this.append_block("filter_next");
						this.builder
							.build_conditional_branch(keep.int(), push, drop)?;
						this.builder.position_at_end(push);
						let position = this
							.builder
							.build_load(i64, kept.pointer, "")?
							.into_int_value();
						if let Some(raw) = value.value {
							let target = this.array_item_slot(array, item, position)?;
							this.builder.build_store(target.pointer, raw)?;
						}
						let following =
							this.builder
								.build_int_add(position, i64.const_int(1, false), "")?;
						this.builder.build_store(kept.pointer, following)?;
						this.builder.build_unconditional_branch(next)?;
						this.builder.position_at_end(drop);
						this.emit_drop(value)?;
						this.builder.build_unconditional_branch(next)?;
						this.builder.position_at_end(next);
						Ok(())
					})?;
					let count = self.builder.build_load(i64, kept.pointer, "")?;
					let length_pointer =
						self.builder
							.build_struct_gep(self.header_type(), array, 1, "")?;
					self.builder.build_store(length_pointer, count)?;
					return Ok(self.fresh(Val::new(array_value.repr, array)));
				}
				let result = self.new_array_into(node, item, i64.const_zero(), length)?;
				// A throwing callback drops precisely the kept prefix.
				let result_value = self.fresh(Val::new(array_value.repr, result));
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let value = this.load(slot)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let keep = this.call_collection_callback(
						*callback,
						callback_node,
						&[],
						&[slot],
						&[index_value],
					)?;
					let push = this.append_block("filter_keep");
					let next = this.append_block("filter_next");
					this.builder
						.build_conditional_branch(keep.int(), push, next)?;
					this.builder.position_at_end(push);
					this.array_push(result_value, value, node, None)?;
					this.builder.build_unconditional_branch(next)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				Ok(result_value)
			}
			("Array#deleteWhere", [callback]) => {
				let (_, length) = self.array_parts(array)?;
				let kept = self.alloca(Repr::Int(IntRepr::I64), "kept")?;
				self.builder.build_store(kept.pointer, i64.const_zero())?;
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let delete = this.callback_with_fixup(
						callback_node,
						|this| {
							this.call_collection_callback(
								*callback,
								callback_node,
								&[],
								&[slot],
								&[index_value],
							)
						},
						// an error: the items not tested yet move down after the kept ones
						|this| {
							let position = this.load(kept)?.int();
							let rest = this.builder.build_int_sub(length, index, "")?;
							this.for_each_index(rest, |this, offset| {
								let from = this.builder.build_int_add(index, offset, "")?;
								let to = this.builder.build_int_add(position, offset, "")?;
								let source = this.array_item_slot(array, item, from)?;
								let moved = this.load(source)?;
								let target = this.array_item_slot(array, item, to)?;
								this.builder
									.build_store(target.pointer, moved.value.expect("an item"))?;
								Ok(())
							})?;
							let header = this.header_type();
							let length_pointer =
								this.builder.build_struct_gep(header, array, 1, "")?;
							let remaining = this.builder.build_int_add(position, rest, "")?;
							this.builder.build_store(length_pointer, remaining)?;
							Ok(())
						},
					)?;
					// The callback borrows this slot and may have changed its fields.
					let value = this.load(slot)?;
					let keep = this.append_block("delete_keep");
					let drop = this.append_block("delete_drop");
					let next = this.append_block("delete_next");
					this.builder
						.build_conditional_branch(delete.int(), drop, keep)?;
					this.builder.position_at_end(drop);
					this.emit_drop(value)?;
					this.builder.build_unconditional_branch(next)?;
					this.builder.position_at_end(keep);
					let position = this.load(kept)?.int();
					let (items, _) = this.array_parts(array)?;
					let pointer = this.item_pointer(items, item, position)?;
					this.builder.build_store(
						pointer,
						value
							.value
							.unwrap_or_else(|| self.context.bool_type().const_zero().into()),
					)?;
					let following =
						this.builder
							.build_int_nsw_add(position, i64.const_int(1, false), "")?;
					this.builder.build_store(kept.pointer, following)?;
					this.builder.build_unconditional_branch(next)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				let header = self.header_type();
				let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
				let kept = self.load(kept)?.int();
				self.builder.build_store(length_pointer, kept)?;
				Ok(Val::void())
			}
			("Array#some" | "Array#every", [callback]) => {
				let repr = self.node_repr(node)?;
				let result = self.alloca(repr, "found")?;
				let every = path == "Array#every";
				let initial = Val::new(
					Repr::Bool,
					self.context.bool_type().const_int(u64::from(every), false),
				);
				self.builder
					.build_store(result.pointer, initial.value.expect("a value"))?;
				let (_, length) = self.array_parts(array)?;
				let end = self.append_block("end_search");
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let holds = this.call_collection_callback(
						*callback,
						callback_node,
						&[],
						&[slot],
						&[index_value],
					)?;
					let found = this.append_block("search_found");
					let next = this.append_block("search_next");
					let stop = if path == "Array#every" {
						this.builder.build_not(holds.int(), "")?
					} else {
						holds.int()
					};
					this.builder.build_conditional_branch(stop, found, next)?;
					this.builder.position_at_end(found);
					let answer = Val::new(
						Repr::Bool,
						this.context.bool_type().const_int(u64::from(!every), false),
					);
					this.builder
						.build_store(result.pointer, answer.value.expect("a value"))?;
					this.builder.build_unconditional_branch(end)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				self.load(result)
			}
			("Array#reduce", [callback, initial]) => {
				let repr = self.node_repr(node)?;
				let accumulator = self.alloca(repr, "accumulator")?;
				let initial = self.owned_as(*initial, repr, Some(arguments[1]))?;
				self.store(accumulator, initial, arguments[1])?;
				let (_, length) = self.array_parts(array)?;
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let current = this.load(accumulator)?;
					// `(total, value, index) => …`: the index too
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let next = this.callback_with_fixup(
						callback_node,
						|this| {
							this.call_collection_callback(
								*callback,
								callback_node,
								&[current],
								&[slot],
								&[index_value],
							)
						},
						|this| this.emit_drop(current),
					)?;
					let next = this.owned_as(next, repr, None)?;
					this.emit_drop(current)?;
					this.store(accumulator, next, node)?;
					Ok(())
				})?;
				Ok(self.fresh(self.load(accumulator)?))
			}
			("Array#sum", []) => {
				let repr = self.node_repr(node)?;
				let zero = match repr {
					Repr::Int(int) => Val::new(repr, self.int_type(int.bits).const_zero()),
					Repr::F32 => Val::new(repr, self.context.f32_type().const_zero()),
					_ => Val::new(Repr::F64, self.context.f64_type().const_zero()),
				};
				let total = self.alloca(repr, "sum")?;
				self.store(total, zero, node)?;
				let (_, length) = self.array_parts(array)?;
				self.for_each_index(length, |this, index| {
					let value = this.load_item(array, item, index)?;
					let current = this.load(total)?;
					let sum = match repr {
						Repr::Int(_) if !this.overflow_checks() => {
							let value = this.convert(value, repr, node.index())?;
							let sum = this.builder.build_int_add(current.int(), value.int(), "")?;
							Val::new(repr, sum)
						}
						Repr::Int(int) => {
							let value = this.convert(value, repr, node.index())?;
							let add = this.intrinsic(
								"llvm.sadd.with.overflow",
								&[this.int_type(int.bits).into()],
							)?;
							let result = this
								.call_value(add, &[current.int().into(), value.int().into()])?
								.into_struct_value();
							let sum = this.builder.build_extract_value(result, 0, "")?;
							let overflowed = this
								.builder
								.build_extract_value(result, 1, "")?
								.into_int_value();
							let site = this.site("Integer", node);
							this.guard(overflowed, |this| {
								this.call_noreturn("fa_overflow_unknown", &[site.into()])
							})?;
							Val::new(repr, sum)
						}
						_ => {
							let a = current.value.expect("a float").into_float_value();
							let b = this
								.convert(value, repr, node.index())?
								.value
								.expect("a float")
								.into_float_value();
							Val::new(repr, this.builder.build_float_add(a, b, "")?)
						}
					};
					this.store(total, sum, node)?;
					Ok(())
				})?;
				self.load(total)
			}
			("Array#includes" | "Array#indexOf", [searched]) => {
				let found = self.alloca(Repr::Int(IntRepr::I64), "index")?;
				self.builder
					.build_store(found.pointer, i64.const_all_ones())?;
				let (_, length) = self.array_parts(array)?;
				let end = self.append_block("end_index_of");
				self.for_each_index(length, |this, index| {
					let value = this.load_item(array, item, index)?;
					let equal = this.values_equal_any(value, *searched, node)?;
					let hit = this.append_block("index_found");
					let next = this.append_block("index_next");
					this.builder.build_conditional_branch(equal, hit, next)?;
					this.builder.position_at_end(hit);
					this.builder.build_store(found.pointer, index)?;
					this.builder.build_unconditional_branch(end)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				let index = self.load(found)?.int();
				let present = self.builder.build_int_compare(
					IntPredicate::SGE,
					index,
					i64.const_zero(),
					"",
				)?;
				if path == "Array#includes" {
					return Ok(Val::new(Repr::Bool, present));
				}
				let repr = Repr::Optional(Scalar::Int(IntRepr::I64));
				let ty = self.llvm_type(repr).into_struct_type();
				let pair = self
					.builder
					.build_insert_value(ty.get_undef(), present, 0, "")?;
				let pair = self.builder.build_insert_value(pair, index, 1, "")?;
				Ok(Val::new(repr, pair.into_struct_value()))
			}
			("Array#pop", []) => {
				let repr = self.collection_optional_repr(self.node_repr(node)?, item);
				let (_, length) = self.array_parts(array)?;
				let result = self.alloca(repr, "popped")?;
				let none = self.convert(Val::none(), repr, node.index())?;
				self.builder.build_store(
					result.pointer,
					none.value
						.unwrap_or_else(|| self.context.bool_type().const_zero().into()),
				)?;
				let empty = self.builder.build_int_compare(
					IntPredicate::EQ,
					length,
					i64.const_zero(),
					"",
				)?;
				let take = self.append_block("pop");
				let end = self.append_block("popped");
				self.builder.build_conditional_branch(empty, end, take)?;
				self.builder.position_at_end(take);
				let last = self
					.builder
					.build_int_nsw_sub(length, i64.const_int(1, false), "")?;
				let value = self.load_item(array, item, last)?;
				let value = self.convert(value, repr, node.index())?;
				self.builder.build_store(
					result.pointer,
					value
						.value
						.unwrap_or_else(|| self.context.bool_type().const_zero().into()),
				)?;
				let header = self.header_type();
				let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
				self.builder.build_store(length_pointer, last)?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				Ok(self.fresh(self.load(result)?))
			}
			("Array#clear", []) => {
				if self.needs_drop(item) {
					let (_, length) = self.array_parts(array)?;
					self.for_each_index(length, |this, index| {
						let value = this.load_item(array, item, index)?;
						this.emit_drop(value)
					})?;
				}
				let header = self.header_type();
				let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
				self.builder.build_store(length_pointer, i64.const_zero())?;
				Ok(Val::void())
			}
			("Array#reverse" | "Array#reverseInPlace", []) => {
				let target = if path == "Array#reverse" {
					let copy = self.emit_copy(array_value)?;
					copy.value.expect("an array").into_pointer_value()
				} else {
					array
				};
				let size = self.item_size(item)?;
				self.call_runtime("fa_array_reverse", &[target.into(), size.into()])?;
				Ok(if path == "Array#reverse" {
					self.fresh(Val::new(array_value.repr, target))
				} else {
					Val::void()
				})
			}
			("Array#insert", [index, value]) => {
				let position = self.normalized_index(array, *index, node, true)?;
				let value = self.owned_as(*value, item, arguments.get(1).copied())?;
				let size = self.item_size(item)?;
				let slot = self.runtime.function("fa_array_insert_slot")?;
				let pointer = self
					.call_value(slot, &[array.into(), position.into(), size.into()])?
					.into_pointer_value();
				self.builder.build_store(
					pointer,
					value
						.value
						.unwrap_or_else(|| self.context.bool_type().const_zero().into()),
				)?;
				Ok(Val::void())
			}
			("Array#delete" | "Array#take", [index]) => {
				let position = self.normalized_index(array, *index, node, false)?;
				let value = self.load_item(array, item, position)?;
				let size = self.item_size(item)?;
				self.call_runtime(
					"fa_array_remove",
					&[array.into(), position.into(), size.into()],
				)?;
				Ok(if path == "Array#take" {
					self.fresh(value)
				} else {
					self.emit_drop(value)?;
					Val::void()
				})
			}
			("Array#sort" | "Array#sortInPlace", _) => self.sort_array(
				array_value,
				item,
				path == "Array#sort",
				values,
				arguments,
				node,
			),
			("Array#join" | "Array#joinedLength", [separator]) if item == Repr::Str => {
				let (bytes, length) = self.string_parts(*separator)?;
				if path == "Array#joinedLength" {
					let length = self
						.call_value(
							self.runtime.function("fa_array_join_length")?,
							&[array.into(), bytes.into(), length.into()],
						)?
						.into_int_value();
					let length = self
						.builder
						.build_int_z_extend_or_bit_cast(length, i64, "")?;
					return Ok(Val::new(Repr::Int(IntRepr::I64), length));
				}
				let result = self.call_with_pair(
					"fa_array_join",
					&[array.into(), bytes.into(), length.into()],
					Repr::Str,
				)?;
				Ok(self.fresh(result))
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// Appends `[1, 2, 3]`.
	pub(crate) fn append_array(
		&mut self,
		value: Val<'ctx>,
		item_type: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		let item = self.item_repr(item_type, node)?;
		let array = value.value.expect("an array").into_pointer_value();
		self.materialize_array(array, item)?;
		let open = self.string_constant("[");
		self.append_value(open, node)?;
		let (items, length) = self.array_parts(array)?;
		let counter = self.alloca(Repr::Int(IntRepr::I64), "printed")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("print_item");
		let body = self.append_block("print_item_body");
		let end = self.append_block("printed_items");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let index = self.load(counter)?.int();
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, index, length, "")?;
		self.builder.build_conditional_branch(more, body, end)?;
		self.builder.position_at_end(body);
		let first =
			self.builder
				.build_int_compare(IntPredicate::EQ, index, i64.const_zero(), "")?;
		let separator = self.append_block("print_separator");
		let item_block = self.append_block("print_item_value");
		self.builder
			.build_conditional_branch(first, item_block, separator)?;
		self.builder.position_at_end(separator);
		let comma = self.string_constant(", ");
		self.append_value(comma, node)?;
		self.builder.build_unconditional_branch(item_block)?;
		self.builder.position_at_end(item_block);
		let pointer = self.item_pointer(items, item, index)?;
		if let Repr::Tuple(ty) = item
			&& self.inline_info(ty).is_some()
		{
			let format = self.aggregate_formatter(item, node)?;
			self.builder.build_call(format, &[pointer.into()], "")?;
		} else {
			let value = self.load(Slot {
				pointer,
				repr: item,
			})?;
			self.append_nested_of(value, item_type, node)?;
		}
		let next = self
			.builder
			.build_int_nsw_add(index, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(end);
		let close = self.string_constant("]");
		self.append_value(close, node)
	}

	/// Replaces every item of an array that shares its memory with another
	/// array (a bytewise copy) by a copy of its own.
	pub(crate) fn copy_items(
		&mut self,
		array: PointerValue<'ctx>,
		item: Repr,
	) -> Result<(), Error> {
		if !self.needs_drop(item) {
			return Ok(());
		}
		self.materialize_array(array, item)?;
		let (items, length) = self.array_parts(array)?;
		self.for_each_index(length, |this, index| {
			let pointer = this.item_pointer(items, item, index)?;
			let loaded = this.load(Slot {
				pointer,
				repr: item,
			})?;
			let copied = this.emit_copy(loaded)?;
			this.builder
				.build_store(pointer, copied.value.expect("an item"))?;
			Ok(())
		})
	}

	/// `for { name, age } in people`, `for (key, value) in pairs`: each name
	/// designates its part of the item.
	pub(crate) fn bind_item_pattern(
		&mut self,
		info: &fa_analyzer::analysis::PatternInfo,
		item: Slot<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		for (position, element) in info.elements.iter().enumerate() {
			let fa_analyzer::analysis::PatternBinding::Symbol(symbol) = element.binding else {
				return Err(self.unsupported(node.index(), "this pattern in a `for`"));
			};
			let (structure, index, repr) = match (item.repr, element.field) {
				// a field of an element of a columnar: in its column
				(Repr::Row(object), Some(field)) => {
					let types = self.types();
					let Some(index) = types
						.fields_of(object)
						.iter()
						.position(|entry| entry.name == field)
					else {
						return Err(self.unsupported(node.index(), "this pattern in a `for`"));
					};
					let pointer = self.row_field(&self.builder, item.pointer, object, index)?;
					let repr = self.field_repr(object, index, node.index())?;
					self.slots.insert(symbol, Slot { pointer, repr });
					continue;
				}
				(Repr::Object(object), Some(field)) => {
					let types = self.types();
					let Some(index) = types
						.fields_of(object)
						.iter()
						.position(|entry| entry.name == field)
					else {
						return Err(self.unsupported(node.index(), "this pattern in a `for`"));
					};
					let repr = self.field_repr(object, index, node.index())?;
					(self.object_type(object), index, repr)
				}
				(Repr::Tuple(tuple), None) => {
					let ty = self.tuple_items(tuple)[position];
					let repr = self.repr_of(ty, node.index())?;
					(self.tuple_type(tuple), position, repr)
				}
				_ => return Err(self.unsupported(node.index(), "this pattern in a `for`")),
			};
			let pointer =
				self.builder
					.build_struct_gep(structure, item.pointer, index as u32, "")?;
			self.slots.insert(symbol, Slot { pointer, repr });
		}
		if info.rest.is_some() {
			return Err(self.unsupported(node.index(), "a rest pattern in a `for`"));
		}
		Ok(())
	}

	/// Runs `body` for every item of an array (the items of `0..length`).
	pub(crate) fn for_each_item_value(
		&mut self,
		array: PointerValue<'ctx>,
		item: Repr,
		length: IntValue<'ctx>,
		mut body: impl FnMut(&mut Self, Val<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		self.materialize_array(array, item)?;
		self.for_each_index(length, |this, index| {
			let value = this.load_item(array, item, index)?;
			body(this, value)
		})
	}

	/// Runs `body` for every index in `0..length`.
	pub(crate) fn for_each_index(
		&mut self,
		length: IntValue<'ctx>,
		mut body: impl FnMut(&mut Self, IntValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let counter = self.alloca(Repr::Int(IntRepr::I64), "index")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("each");
		let block = self.append_block("each_body");
		let end = self.append_block("end_each");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let index = self.load(counter)?.int();
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, index, length, "")?;
		self.builder.build_conditional_branch(more, block, end)?;
		self.builder.position_at_end(block);
		let mark = self.mark();
		body(self, index)?;
		self.end_statement(mark, None)?;
		let next = self
			.builder
			.build_int_nsw_add(index, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(end);
		Ok(())
	}

	/// `for item, index in array { … }`: the loop variable is the item, and
	/// what the body assigns to it is written back (loop references).
	pub(crate) fn for_items(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		array: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let (Repr::Array(item_type) | Repr::Growing(item_type)) = array.repr else {
			return Err(self.unsupported(node.index(), "`for` over this value"));
		};
		let growing = matches!(array.repr, Repr::Growing(_));
		let item = self.item_repr(item_type, node)?;
		if growing {
			self.iterating_views = false;
		}
		let array = array.value.expect("an array").into_pointer_value();
		if !growing {
			self.materialize_array(array, item)?;
		}
		// the variable designates the item (a `for copy` holds a copy of it)
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		let copied = variable.is_some_and(|symbol| {
			matches!(
				self.analysis.symbols.symbol(symbol).kind,
				fa_analyzer::symbols::SymbolKind::LoopVariable { copy: true }
			)
		});
		let copy_slot = match (variable, copied) {
			(Some(symbol), true) => Some(self.slot_of(symbol, node)?),
			_ => None,
		};
		let index_slot = match has_index
			.then(|| self.analysis.symbols.symbol_of_decl(node))
			.flatten()
		{
			Some(symbol) => Some(self.slot_of(symbol, node)?),
			None => None,
		};
		if let Some(copy) = self.copy_loop(pattern, has_index, iterable, body, false)? {
			// a loop only pushing the items onto another array (`copies.rs`)
			let (repr, length) = if growing {
				(
					Repr::Growing(item_type),
					self.growing_length(&self.builder, array)?,
				)
			} else {
				(Repr::Array(item_type), self.array_parts(array)?.1)
			};
			self.copy_all_items(copy, Val::new(repr, array), length)?;
			return Ok(Val::void());
		}
		let counter = self.alloca(Repr::Int(IntRepr::I64), "position")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		// The iteration rule forbids changing the array's storage or length,
		// including through aliases and calls. Item mutations are allowed,
		// so keep the items themselves live but load the header only once.
		let parts = if growing {
			None
		} else {
			Some(self.array_parts(array)?)
		};
		// a `GrowingArray` is walked segment by segment, with a pointer
		let matched = self.matched_byte(pattern, item, body)?;
		let segmented_scan =
			growing && matched.is_none() && self.collection_data_size(self.llvm_type(item)) != 0;
		let (growing_length, run) = if growing && !segmented_scan {
			let cursor = self.pointer_alloca("segment_cursor")?;
			let end = self.pointer_alloca("segment_end")?;
			let null = self.pointer_type().const_null();
			self.builder.build_store(cursor, null)?;
			self.builder.build_store(end, null)?;
			(
				Some(self.growing_length(&self.builder, array)?),
				Some((cursor, end)),
			)
		} else if growing {
			(Some(self.growing_length(&self.builder, array)?), None)
		} else {
			(None, None)
		};
		// a body acting only on the items equal to a constant byte: the loop
		// goes from one such item to the next (`copies.rs`)
		// The iteration rule keeps this header stable. A contiguous live
		// prefix can use the ordinary byte search and direct item address;
		// only the bytes remain live across executions of the loop body.
		let growing_flat = if growing && matched.is_some() {
			let contiguous = self.growing_single_segment(array)?;
			let first = self.growing_first_items(&self.builder, array)?;
			Some((first, contiguous))
		} else {
			None
		};
		let test = self.append_block("for_item");
		let body_block = self.append_block("for_item_body");
		let step = self.append_block("for_item_step");
		let exit = self.append_block("end_for_item");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let mut position = self.load(counter)?.int();
		if let (Some(byte), Some((items, length))) = (matched, parts) {
			let next = self.runtime.function("fa_bytes_next")?;
			position = self
				.call_value(
					next,
					&[
						items.into(),
						position.into(),
						length.into(),
						self.context.i32_type().const_int(byte, false).into(),
					],
				)?
				.into_int_value();
			self.builder.build_store(counter.pointer, position)?;
		}
		if let (Some(byte), Some(length), Some((cursor, end))) = (matched, growing_length, run) {
			let flat = if let Some((items, contiguous)) = growing_flat {
				let direct = self.append_block("search_contiguous");
				let segmented = self.append_block("search_segmented");
				let join = self.append_block("search_found");
				self.builder
					.build_conditional_branch(contiguous, direct, segmented)?;
				self.builder.position_at_end(direct);
				let found = self
					.call_value(
						self.runtime.function("fa_bytes_next")?,
						&[
							items.into(),
							position.into(),
							length.into(),
							self.context.i32_type().const_int(byte, false).into(),
						],
					)?
					.into_int_value();
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(segmented);
				Some((found, direct, join))
			} else {
				None
			};
			position = self
				.call_value(
					self.runtime.function("fa_segmented_next_byte")?,
					&[
						array.into(),
						position.into(),
						length.into(),
						self.context.i32_type().const_int(byte, false).into(),
						self.segment_k(item).into(),
						cursor.into(),
						end.into(),
					],
				)?
				.into_int_value();
			if let Some((found, direct, join)) = flat {
				let segmented = self.builder.get_insert_block().expect("a block");
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(join);
				let merged = self.builder.build_phi(i64, "found_position")?;
				merged.add_incoming(&[(&found, direct), (&position, segmented)]);
				position = merged.as_basic_value().into_int_value();
			}
			self.builder.build_store(counter.pointer, position)?;
		}
		let length = match (parts, growing_length) {
			(Some((_, length)), _) => length,
			(None, Some(length)) => length,
			(None, None) => self.growing_length(&self.builder, array)?,
		};
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, length, "")?;
		self.builder
			.build_conditional_branch(more, body_block, exit)?;
		self.builder.position_at_end(body_block);
		// The outer loop enters once per live segment. The inner loop has
		// one counted bound and direct addresses, just like an Array scan.
		// Iteration forbids changing this array's storage or length, even
		// through calls; elements remain live and may be mutated normally.
		let segment = if segmented_scan {
			let end = self.pointer_alloca("scan_segment_end")?;
			let first = self.growing_run(array, item, position, end)?;
			let end = self
				.builder
				.build_load(self.pointer_type(), end, "scan_end")?
				.into_pointer_value();
			let bytes = self.builder.build_int_sub(
				self.builder.build_ptr_to_int(end, self.usize, "")?,
				self.builder.build_ptr_to_int(first, self.usize, "")?,
				"",
			)?;
			let size = self
				.builder
				.build_int_cast(self.item_size_const(item), self.usize, "")?;
			let count = self
				.builder
				.build_int_unsigned_div(bytes, size, "segment_room")?;
			let count = self
				.builder
				.build_int_z_extend_or_bit_cast(count, i64, "")?;
			let remaining = self.builder.build_int_sub(length, position, "")?;
			let partial =
				self.builder
					.build_int_compare(IntPredicate::ULT, remaining, count, "")?;
			let count = self
				.builder
				.build_select(partial, remaining, count, "segment_count")?
				.into_int_value();
			let limit = self
				.builder
				.build_int_add(position, count, "segment_limit")?;
			let inner = self.append_block("for_segment_item");
			self.builder.build_unconditional_branch(inner)?;
			self.builder.position_at_end(inner);
			let current = self.load(counter)?.int();
			let offset = self
				.builder
				.build_int_sub(current, position, "segment_offset")?;
			let pointer = self.item_pointer(first, item, offset)?;
			position = current;
			Some((pointer, limit, inner))
		} else {
			None
		};
		let pointer = if let Some((pointer, _, _)) = segment {
			pointer
		} else if let Some((items, contiguous)) = growing_flat {
			let direct = self.append_block("item_contiguous");
			let segmented = self.append_block("item_segmented");
			let join = self.append_block("item_found");
			self.builder
				.build_conditional_branch(contiguous, direct, segmented)?;
			self.builder.position_at_end(direct);
			let first = self.item_pointer(items, item, position)?;
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(segmented);
			let later =
				self.growing_next_item(array, item, position, run.expect("a segmented cursor"))?;
			let segmented_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(join);
			let pointer = self.builder.build_phi(self.pointer_type(), "found_item")?;
			pointer.add_incoming(&[(&first, direct), (&later, segmented_end)]);
			pointer.as_basic_value().into_pointer_value()
		} else {
			match (parts, run) {
				(Some((items, _)), _) => self.item_pointer(items, item, position)?,
				(None, Some(run)) => self.growing_next_item(array, item, position, run)?,
				(None, None) => self.growing_at(&self.builder, array, item, position)?,
			}
		};

		// a reference to a later item in a nested loop never overlaps this one
		let disjoint = variable
			.filter(|_| copy_slot.is_none() && has_index && !growing)
			.and_then(|_| self.disjoint_reference(node, body))
			.map(|other| (other, self.disjoint_scopes()));
		if let Some((_, scopes)) = disjoint {
			self.declare_disjoint_scopes(scopes)?;
		}
		let first_body_block = self.builder.get_insert_block().expect("a block");
		let item_slot = Slot {
			pointer,
			repr: item,
		};
		if std::mem::take(&mut self.iterating_views) && item == Repr::Str {
			// a view of a split text: never inline, so that reading its bytes
			// tests nothing (`views.rs`)
			let capacity_pointer =
				self.builder
					.build_struct_gep(self.string_type, pointer, 2, "")?;
			let capacity = self
				.builder
				.build_load(self.usize, capacity_pointer, "")?
				.into_int_value();
			let flag = self
				.builder
				.build_and(capacity, self.string_inline_flag(), "")?;
			let external = self.builder.build_int_compare(
				IntPredicate::EQ,
				flag,
				self.usize.const_zero(),
				"",
			)?;
			let assume = inkwell::intrinsics::Intrinsic::find("llvm.assume")
				.and_then(|intrinsic| intrinsic.get_declaration(self.module, &[]))
				.expect("llvm.assume");
			self.builder.build_call(assume, &[external.into()], "")?;
		}
		let destructured = pattern.and_then(|pattern| self.analysis.pattern(pattern).cloned());
		match (variable, copy_slot) {
			_ if destructured.is_some() => {
				self.bind_item_pattern(destructured.as_ref().expect("a pattern"), item_slot, node)?;
			}
			(Some(symbol), None) => {
				self.slots.insert(symbol, item_slot);
			}
			(_, Some(copy)) => {
				let value = self.load(item_slot)?;
				let value = self.emit_copy(value)?;
				self.store(copy, value, node)?;
			}
			_ => {}
		}
		if let Some(index_slot) = index_slot {
			self.store(
				index_slot,
				Val::new(Repr::Int(IntRepr::I64), position),
				node,
			)?;
		}
		let mark = self.mark();
		if let Some(copy) = copy_slot {
			// dropped at the end of each round, and by `break` and `continue`
			self.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Local(copy));
		}
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: step,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.end_scope(mark, None)?;
		self.branch_to(step)?;
		if let Some((other, scopes)) = disjoint
			&& let Some(other_slot) = self.slots.get(&other).copied()
		{
			// the body's blocks: its first, and those made after `exit`
			let mut blocks = vec![first_body_block];
			let mut next = exit.get_next_basic_block();
			while let Some(block) = next {
				blocks.push(block);
				next = block.get_next_basic_block();
			}
			self.tag_disjoint(&blocks, (pointer, other_slot.pointer), scopes)?;
		}
		self.builder.position_at_end(step);
		let position = self.load(counter)?.int();
		let next = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		if let Some((_, limit, inner)) = segment {
			let more = self
				.builder
				.build_int_compare(IntPredicate::ULT, next, limit, "")?;
			self.builder.build_conditional_branch(more, inner, test)?;
		} else {
			self.builder.build_unconditional_branch(test)?;
		}
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// Before a counted loop (`for i in a..b`) whose body pushes one item
	/// onto a local array or its field at its top level, mentions its root nowhere else,
	/// and can't leave early: reserves room for every round, so that the
	/// pushes need no capacity check (and a loop filling an array becomes a
	/// `memset`).
	pub(crate) fn reserve_for_pushes(
		&mut self,
		body: NodeId,
		start: IntValue<'ctx>,
		end: Option<IntValue<'ctx>>,
		inclusive: bool,
	) -> Result<Vec<NodeId>, Error> {
		let mut reserved = Vec::new();
		let Some(end) = end else {
			return Ok(reserved);
		};
		let View::Block { statements } = self.view(body) else {
			return Ok(reserved);
		};
		let statements: Vec<NodeId> = statements.iter().collect();
		// no early exit: every round runs, and pushes
		let mut stack = vec![body.index()];
		while let Some(node) = stack.pop() {
			if matches!(
				self.view(NodeId::new(node)),
				View::Break(_) | View::Return(_) | View::Throw(_) | View::Try(_)
			) {
				return Ok(reserved);
			}
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		'push: for statement in statements {
			let Some(info) = self.analysis.call(statement) else {
				continue;
			};
			let fa_analyzer::analysis::Callee::Builtin(id) = info.callee else {
				continue;
			};
			if fa_analyzer::builtins::get(id).path != "Array#push" {
				continue;
			}
			let View::Call { callee, .. } = self.view(statement) else {
				continue;
			};
			let View::Access { target, members } = self.view(callee) else {
				continue;
			};
			let Resolution::Local(symbol) = self.analysis.resolution(target) else {
				continue;
			};
			// A parameter (a `mutates` one, to push) is a root of its own in
			// the function: no other parameter, module-level value or
			// callback argument overlaps it (reference checking).
			if !matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Parameter { .. }
			) {
				continue;
			}
			// the array is mentioned once: by this push
			let mut uses = 0;
			let mut calls = 0;
			let mut stack = vec![body.index()];
			while let Some(node) = stack.pop() {
				if self.analysis.resolution(NodeId::new(node)) == Resolution::Local(symbol) {
					uses += 1;
				}
				if matches!(self.view(NodeId::new(node)), View::Call { .. }) {
					calls += 1;
				}
				self.tree.nodes[node].for_each_child(|child| stack.push(child));
			}
			if uses != 1 {
				continue;
			}
			// A local that functions or closures reach (a module-level value,
			// a capture) may be replaced by a call in the loop, with a block
			// without the reserved room: no call but this push.
			let reached = self.shared.contains_key(&symbol)
				|| self
					.analysis
					.functions
					.values()
					.any(|function| function.captures.contains(&symbol));
			if reached && calls != 1 {
				continue;
			}
			let mut slot = self.slot_of(symbol, target)?;
			for member in members.iter().take(members.len() - 1) {
				// Only ordinary object fields: finding the array before the loop
				// must not evaluate a getter, an index or an optional access.
				let (Repr::Object(object), Resolution::Field(..)) =
					(slot.repr, self.analysis.resolution(member))
				else {
					continue 'push;
				};
				let index = self.concrete_field_index(object, member)?;
				slot = Slot {
					pointer: self.builder.build_struct_gep(
						self.object_type(object),
						slot.pointer,
						index,
						"",
					)?,
					repr: self.field_repr(object, index as usize, member.index())?,
				};
			}
			let Repr::Array(item_type) = slot.repr else {
				continue;
			};
			// a packed array grows by bytes (`packed.rs`)
			if self.packed_layout(item_type).is_some() {
				continue;
			}
			let item = self.item_repr(item_type, target)?;
			let i64 = self.context.i64_type();
			let saturating = |this: &Self, name: &str, a: IntValue<'ctx>, b: IntValue<'ctx>| {
				let function = this.intrinsic(name, &[i64.into()])?;
				Ok::<_, Error>(
					this.call_value(function, &[a.into(), b.into()])?
						.into_int_value(),
				)
			};
			let mut rounds = saturating(self, "llvm.ssub.sat", end, start)?;
			if inclusive {
				rounds = saturating(self, "llvm.sadd.sat", rounds, i64.const_int(1, false))?;
			}
			let negative =
				self.builder
					.build_int_compare(IntPredicate::SLT, rounds, i64.const_zero(), "")?;
			let rounds = self
				.builder
				.build_select(negative, i64.const_zero(), rounds, "")?
				.into_int_value();
			let array = self
				.load(slot)?
				.value
				.expect("an array")
				.into_pointer_value();
			let (_, length) = self.array_parts(array)?;
			let size = self.item_size(item)?;
			let end_reserve = if members.len() > 1 {
				// Reserving a field early must not move existing items: an alias
				// could still read them before the first push. An empty array has
				// no such references, and its header itself never moves.
				let empty = self.builder.build_int_compare(
					IntPredicate::EQ,
					length,
					i64.const_zero(),
					"",
				)?;
				let reserve = self.append_block("reserve_empty_field");
				let end = self.append_block("end_reserve_field");
				self.builder.build_conditional_branch(empty, reserve, end)?;
				self.builder.position_at_end(reserve);
				Some(end)
			} else {
				None
			};
			// An empty field has no live item references. Under the closed
			// capacity proof, avoid geometric slack; retain its header address
			// and each push's check in case a call reaches an alias. Locals
			// keep geometric growth for loops nested in repeated appends.
			let reserve = if end_reserve.is_some() && self.array_capacities_unobserved() {
				"fa_array_reserve"
			} else {
				"fa_array_room"
			};
			self.call_runtime(reserve, &[array.into(), rounds.into(), size.into()])?;
			if let Some(end) = end_reserve {
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				// Calls through aliases may change the field during the loop.
				// Keep the normal capacity check on each push.
			} else {
				self.reserved_pushes.insert(statement, ());
				reserved.push(statement);
			}
		}
		Ok(reserved)
	}
}
