//! Arrays: a pointer to a header `{ ptr items, i64 length, i64 capacity }`
//! whose items are stored inline in a block of the allocator.
//!
//! Arrays are values (types brief §5): binding or storing an array read
//! from a place copies it ([`Lowering::copy_if_place`]); a parameter shares
//! the caller's array (it is read-only, or `mutates` it).

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{NodeList, View};
use inkwell::IntPredicate;
use inkwell::types::StructType;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
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
	fn item_size(&self, item: Repr) -> Result<IntValue<'ctx>, Error> {
		let size = inkwell::types::BasicType::size_of(&self.llvm_type(item)).expect("a sized item");
		Ok(self
			.builder
			.build_int_cast(size, self.context.i64_type(), "")?)
	}

	pub(crate) fn item_repr(&self, item: TypeId, node: NodeId) -> Result<Repr, Error> {
		// the items of an empty literal (`[]`, an `Array(Never)`) never exist
		match self.repr_of(item, node.index())? {
			Repr::Never | Repr::Void => Ok(Repr::Bool),
			// A collection provides this value's storage even when some of
			// its own fields still require boxes to break direct cycles.
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

	/// The items pointer and the length of an array.
	pub(crate) fn array_parts(
		&self,
		array: PointerValue<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		let header = self.header_type();
		let items_pointer = self.builder.build_struct_gep(header, array, 0, "")?;
		let items = self
			.builder
			.build_load(self.pointer_type(), items_pointer, "items")?;
		let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
		let length = self
			.builder
			.build_load(self.context.i64_type(), length_pointer, "length")?;
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
		let (Repr::Array(item_type)
		| Repr::Growing(item_type)
		| Repr::Pool(item_type)
		| Repr::Columnar(item_type)) = repr
		else {
			return Err(self.unsupported(node.index(), "this array"));
		};
		let item = self.item_repr(item_type, node)?;
		let mut values = Vec::new();
		for value in items.iter() {
			if matches!(self.view(value), View::Spread(_)) {
				return Err(self.unsupported(value.index(), "spreading into an array"));
			}
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			values.push((self.owned_as(lowered, item, Some(value))?, value));
		}
		if let Repr::Pool(_) | Repr::Columnar(_) = repr {
			return self.pool_literal(values, repr, node);
		}
		let count = self
			.context
			.i64_type()
			.const_int(values.len() as u64, false);
		let array = self.new_array(item, count, count)?;
		let (pointer, _) = self.array_parts(array)?;
		for (index, (value, _)) in values.into_iter().enumerate() {
			let index = self.context.i64_type().const_int(index as u64, false);
			let slot = self.item_pointer(pointer, item, index)?;
			self.builder
				.build_store(slot, value.value.expect("an item"))?;
		}
		if let Repr::Growing(_) = repr {
			let growing = self.growing_from_array(array, item)?;
			return Ok(self.fresh(Val::new(repr, growing)));
		}
		Ok(self.fresh(Val::new(Repr::Array(item_type), array)))
	}

	/// `Array.from(source, capacity = n)`: a copy of the source (already
	/// copied), with room for `capacity` items.
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
	fn checked_item(
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

	/// Whether an integer node may hold a negative value (its facts, else its type).
	pub(crate) fn may_be_negative(&self, node: NodeId) -> bool {
		if let Some(interval) = self.analysis.interval(node) {
			return interval.lo < ethnum::I256::ZERO;
		}
		match self.types().as_int(self.type_of(node)) {
			Some(range) => range.min < ethnum::I256::ZERO,
			None => true,
		}
	}

	/// The place `array[index]`.
	pub(crate) fn index_place(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let array = self.expr(target)?;
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
		let item = self.item_repr(item_type, node)?;
		let value = self.owned_as(value, item, source)?;
		let array = array.value.expect("an array").into_pointer_value();
		if self.reserved_pushes.contains_key(&node) {
			// room was reserved before the loop: no capacity check
			let (items, length) = self.array_parts(array)?;
			let slot = self.item_pointer(items, item, length)?;
			let store = self
				.builder
				.build_store(slot, value.value.expect("an item"))?;
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
		self.call_runtime("fa_array_grow", &[array.into(), size.into()])?;
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
		let stored = self
			.builder
			.build_store(slot, value.value.expect("an item"))?;
		self.tag(stored, false)?;
		let next = self
			.builder
			.build_int_nsw_add(length, i64.const_int(1, false), "")?;
		let stored = self.builder.build_store(length_pointer, next)?;
		self.tag(stored, true)?;
		Ok(())
	}

	/// `array.get(index)`: the item, or `none`.
	fn array_get(
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
	pub(crate) fn array_method(
		&mut self,
		node: NodeId,
		path: &str,
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
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

	fn array_item_slot(
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
	fn array_of(
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
			View::Index { target, .. } => target,
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
				if fa_analyzer::builtins::get(id).path == "String#codePoints" =>
			{
				Some(inner)
			}
			_ => None,
		}
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
	fn code_point_get(
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
		let value = self
			.builder
			.build_int_truncate(code, self.context.i32_type(), "")?;
		let value = self.convert(
			Val::new(
				Repr::Int(IntRepr {
					bits: 32,
					signed: false,
				}),
				value,
			),
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
			let (bytes, length) = this.string_parts(text)?;
			let capacity = this.string_capacity(text)?;
			let count = this.runtime.function("fa_string_code_point_count")?;
			let count = this
				.call_value(count, &[bytes.into(), length.into(), capacity.into()])?
				.into_int_value();
			this.raise_runtime("fa_error_index", &[position.into(), count.into()])
		})?;
		let value = self
			.builder
			.build_int_truncate(code, self.context.i32_type(), "")?;
		Ok(Val::new(
			Repr::Int(IntRepr {
				bits: 32,
				signed: false,
			}),
			value,
		))
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
	fn empty_array(
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
				let result = self.new_array(result_item, length, length)?;
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let mapped = this.call_collection_callback(
						*callback,
						callback_node,
						&[],
						&[slot],
						&[index_value],
					)?;
					let mapped = this.owned_as(mapped, result_item, None)?;
					let (items, _) = this.array_parts(result)?;
					let pointer = this.item_pointer(items, result_item, index)?;
					if let Some(raw) = mapped.value {
						this.builder.build_store(pointer, raw)?;
					}
					Ok(())
				})?;
				Ok(self.fresh(Val::new(Repr::Array(result_type), result)))
			}
			("Array#filter", [callback]) => {
				let (_, length) = self.array_parts(array)?;
				let result = self.empty_array(item, length)?;
				let callback_node = arguments[0];
				let result_value = Val::new(array_value.repr, result);
				// a temporary array (`text.split(" ").filter(…)`) gives its
				// items up: the kept ones move, the others are dropped, and
				// its block is freed without its items
				let moving = matches!(array_value.repr, Repr::Array(_))
					&& self.needs_drop(item)
					&& self.is_temporary(array_value)
					&& self.cannot_leave(callback_node);
				if moving {
					self.take(array_value, None)?;
				}
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
					let other = if moving {
						this.append_block("filter_drop")
					} else {
						next
					};
					this.builder
						.build_conditional_branch(keep.int(), push, other)?;
					this.builder.position_at_end(push);
					if moving {
						let size = this.item_size(item)?;
						let reserve = this.runtime.function("fa_array_push_slot")?;
						let slot = this
							.call_value(reserve, &[result.into(), size.into()])?
							.into_pointer_value();
						this.builder
							.build_store(slot, value.value.expect("an item"))?;
					} else {
						this.array_push(result_value, value, node, None)?;
					}
					this.builder.build_unconditional_branch(next)?;
					if moving {
						this.builder.position_at_end(other);
						this.emit_drop(value)?;
						this.builder.build_unconditional_branch(next)?;
					}
					this.builder.position_at_end(next);
					Ok(())
				})?;
				if moving {
					let size = self.item_size(item)?;
					self.call_runtime("fa_array_free", &[array.into(), size.into()])?;
				}
				Ok(self.fresh(result_value))
			}
			("Array#deleteWhere", [callback]) => {
				let (_, length) = self.array_parts(array)?;
				let kept = self.alloca(Repr::Int(IntRepr::I64), "kept")?;
				self.builder.build_store(kept.pointer, i64.const_zero())?;
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let value = this.load(slot)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let delete = this.call_collection_callback(
						*callback,
						callback_node,
						&[],
						&[slot],
						&[index_value],
					)?;
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
					this.builder
						.build_store(pointer, value.value.expect("an item"))?;
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
			("Array#find" | "Array#some" | "Array#every", [callback]) => {
				let repr = self.collection_optional_repr(self.node_repr(node)?, item);
				let result = self.alloca(repr, "found")?;
				let initial = match path {
					"Array#every" => {
						Val::new(Repr::Bool, self.context.bool_type().const_int(1, false))
					}
					"Array#some" => Val::new(Repr::Bool, self.context.bool_type().const_zero()),
					_ => Val {
						repr: Repr::None,
						value: None,
					},
				};
				let initial = self.convert(initial, repr, node.index())?;
				self.builder
					.build_store(result.pointer, initial.value.expect("a value"))?;
				let (_, length) = self.array_parts(array)?;
				let end = self.append_block("end_search");
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let slot = this.array_item_slot(array, item, index)?;
					let value = this.load(slot)?;
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
					let answer = match path {
						"Array#every" => {
							Val::new(Repr::Bool, this.context.bool_type().const_zero())
						}
						"Array#some" => {
							Val::new(Repr::Bool, this.context.bool_type().const_int(1, false))
						}
						_ => this.convert(value, repr, node.index())?,
					};
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
					let next = this.call_collection_callback(
						*callback,
						callback_node,
						&[current],
						&[slot],
						&[],
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
				let none = self.convert(
					Val {
						repr: Repr::None,
						value: None,
					},
					repr,
					node.index(),
				)?;
				self.builder
					.build_store(result.pointer, none.value.expect("an optional"))?;
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
				self.builder
					.build_store(result.pointer, value.value.expect("an optional"))?;
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
				self.builder
					.build_store(pointer, value.value.expect("an item"))?;
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
			("Array#sort" | "Array#sortInPlace", _) => {
				let target = if path == "Array#sort" {
					let copy = self.emit_copy(array_value)?;
					copy.value.expect("an array").into_pointer_value()
				} else {
					array
				};
				let comparator = self.comparator(
					item,
					values.first().copied(),
					arguments.first().copied(),
					node,
				)?;
				let size = self.item_size(item)?;
				let context = match values.first() {
					Some(callback) => {
						let slot = self.alloca(callback.repr, "comparator")?;
						self.builder
							.build_store(slot.pointer, callback.value.expect("a function"))?;
						slot.pointer
					}
					None => self.pointer_type().const_null(),
				};
				self.call_runtime(
					"fa_array_sort",
					&[
						target.into(),
						size.into(),
						comparator.as_global_value().as_pointer_value().into(),
						context.into(),
					],
				)?;
				Ok(if path == "Array#sort" {
					self.fresh(Val::new(array_value.repr, target))
				} else {
					Val::void()
				})
			}
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

	/// A C-convention comparator for `fa_array_sort`: `(context, a, b) -> i32`
	/// (negative, zero or positive), calling the Fa comparator in the context
	/// or comparing in the natural order.
	fn comparator(
		&mut self,
		item: Repr,
		callback: Option<Val<'ctx>>,
		callback_node: Option<NodeId>,
		node: NodeId,
	) -> Result<inkwell::values::FunctionValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let i32 = self.context.i32_type();
		let fn_type = i32.fn_type(&[pointer.into(), pointer.into(), pointer.into()], false);
		let function = self.module.add_function(
			"fa.comparator",
			fn_type,
			Some(inkwell::module::Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		// lowered as a function of its own
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, Repr::Void, None);
		let a_pointer = function.get_nth_param(1).expect("a").into_pointer_value();
		let b_pointer = function.get_nth_param(2).expect("b").into_pointer_value();
		let a = self.load(Slot {
			pointer: a_pointer,
			repr: item,
		})?;
		let b = self.load(Slot {
			pointer: b_pointer,
			repr: item,
		})?;
		let result = match (callback, callback_node) {
			(Some(callback), Some(callback_node)) => {
				let context = function
					.get_nth_param(0)
					.expect("a context")
					.into_pointer_value();
				let closure = self.load(Slot {
					pointer: context,
					repr: callback.repr,
				})?;
				let order = self.call_collection_callback(
					closure,
					callback_node,
					&[],
					&[
						Slot {
							pointer: a_pointer,
							repr: item,
						},
						Slot {
							pointer: b_pointer,
							repr: item,
						},
					],
					&[],
				)?;
				match order.repr {
					Repr::Int(int) => {
						let zero = self.int_type(int.bits).const_zero();
						let negative = self.builder.build_int_compare(
							IntPredicate::SLT,
							order.int(),
							zero,
							"",
						)?;
						let positive = self.builder.build_int_compare(
							IntPredicate::SGT,
							order.int(),
							zero,
							"",
						)?;
						self.sign(negative, positive)?
					}
					_ => {
						let order = order.value.expect("a float").into_float_value();
						let zero = order.get_type().const_zero();
						let negative = self.builder.build_float_compare(
							inkwell::FloatPredicate::OLT,
							order,
							zero,
							"",
						)?;
						let positive = self.builder.build_float_compare(
							inkwell::FloatPredicate::OGT,
							order,
							zero,
							"",
						)?;
						self.sign(negative, positive)?
					}
				}
			}
			_ => {
				let less = self.values_order(a, b, IntPredicate::SLT, node)?;
				let greater = self.values_order(b, a, IntPredicate::SLT, node)?;
				self.sign(less, greater)?
			}
		};
		self.builder.build_return(Some(&result))?;
		let frame = self.frame.take().expect("the comparator");
		self.frame = Some(frame);
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		Ok(function)
	}

	/// -1, 0 or 1 from two conditions.
	fn sign(
		&self,
		negative: IntValue<'ctx>,
		positive: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i32 = self.context.i32_type();
		let positive = self
			.builder
			.build_select(positive, i32.const_int(1, false), i32.const_zero(), "")?
			.into_int_value();
		Ok(self
			.builder
			.build_select(negative, i32.const_all_ones(), positive, "")?
			.into_int_value())
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
		let value = self.load(Slot {
			pointer,
			repr: item,
		})?;
		self.append_nested(value, node)?;
		let next = self
			.builder
			.build_int_nsw_add(index, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(end);
		let close = self.string_constant("]");
		self.append_value(close, node)
	}

	/// Whether a node reads a place (a local, a field, an item).
	pub(crate) fn is_place_read(&self, node: NodeId) -> bool {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		matches!(
			self.view(inner),
			View::Name(_) | View::Access { .. } | View::Index { .. }
		)
	}

	/// Whether the items of an array copy bytewise (they own no memory).
	pub(crate) fn copies_bytewise(&self, item_type: TypeId, node: NodeId) -> Result<bool, Error> {
		Ok(!self.needs_drop(self.item_repr(item_type, node)?))
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
	fn for_each_index(
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
		let (growing_length, run) = if growing {
			let cursor = self.pointer_alloca("segment_cursor")?;
			let end = self.pointer_alloca("segment_end")?;
			let null = self.pointer_type().const_null();
			self.builder.build_store(cursor, null)?;
			self.builder.build_store(end, null)?;
			(
				Some(self.growing_length(&self.builder, array)?),
				Some((cursor, end)),
			)
		} else {
			(None, None)
		};
		// a body acting only on the items equal to a constant byte: the loop
		// goes from one such item to the next (`copies.rs`)
		let matched = self.matched_byte(pattern, item, body)?;
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
			let next = self.runtime.function("fa_segmented_next_byte")?;
			position = self
				.call_value(
					next,
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
			// the segment cursor now designates the item found
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
		let pointer = match (parts, run) {
			(Some((items, _)), _) => self.item_pointer(items, item, position)?,
			(None, Some(run)) => self.growing_next_item(array, item, position, run)?,
			(None, None) => self.growing_at(&self.builder, array, item, position)?,
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
		self.builder.build_unconditional_branch(test)?;
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
				let (Repr::Object(object), Resolution::Field(_, index)) =
					(slot.repr, self.analysis.resolution(member))
				else {
					continue 'push;
				};
				slot = Slot {
					pointer: self.builder.build_struct_gep(
						self.object_type(object),
						slot.pointer,
						index as u32,
						"",
					)?,
					repr: self.field_repr(object, index as usize, member.index())?,
				};
			}
			let Repr::Array(item_type) = slot.repr else {
				continue;
			};
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
			// room grown geometrically, as the pushes would: a loop nested in
			// another one doesn't reallocate at every outer round
			self.call_runtime("fa_array_room", &[array.into(), rounds.into(), size.into()])?;
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
