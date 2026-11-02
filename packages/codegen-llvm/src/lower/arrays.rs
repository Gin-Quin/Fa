//! Arrays: a pointer to a header `{ ptr items, i64 length, i64 capacity }`
//! whose items are stored inline in a block of the allocator.
//!
//! Arrays are values (types brief §5): binding or storing an array read
//! from a place copies it ([`Lowering::copy_if_place`]); a parameter shares
//! the caller's array (it is read-only, or `mutates` it).

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{NodeList, View};
use inkwell::IntPredicate;
use inkwell::types::StructType;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
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

	fn item_repr(&self, item: TypeId, node: NodeId) -> Result<Repr, Error> {
		self.repr_of(item, node.index())
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
			.build_load(self.pointer_type(), items_pointer, "items")?
			.into_pointer_value();
		let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
		let length = self
			.builder
			.build_load(self.context.i64_type(), length_pointer, "length")?
			.into_int_value();
		Ok((items, length))
	}

	/// A pointer to item `index` (not checked).
	fn item_pointer(
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
		let Repr::Array(item_type) = self.node_repr(node)? else {
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
			let lowered = self.copy_if_place(lowered, value)?;
			values.push((self.convert(lowered, item, value.index())?, value));
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
		Ok(Val::new(Repr::Array(item_type), array))
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
	) -> Result<PointerValue<'ctx>, Error> {
		let index = self
			.convert(index, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let (items, length) = self.array_parts(array)?;
		if self.flags(node).contains(NodeFlags::INDEX_CHECK) {
			// unsigned: a negative index is beyond the length
			let outside = self
				.builder
				.build_int_compare(IntPredicate::UGE, index, length, "")?;
			self.guard(outside, |this| {
				this.raise_runtime("fa_error_index", &[index.into(), length.into()])
			})?;
		}
		self.item_pointer(items, item, index)
	}

	/// The place `array[index]`.
	pub(crate) fn index_place(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let array = self.expr(target)?;
		let Repr::Array(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "indexing this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let index = self.expr(index)?;
		let array = array.value.expect("an array").into_pointer_value();
		let pointer = self.checked_item(node, array, item, index)?;
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
	) -> Result<(), Error> {
		let Repr::Array(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "`push` on this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let value = self.convert(value, item, node.index())?;
		let array = array.value.expect("an array").into_pointer_value();
		let size = self.item_size(item)?;
		let reserve = self.runtime.function("fa_array_push_slot")?;
		let slot = self
			.call_value(reserve, &[array.into(), size.into()])?
			.into_pointer_value();
		self.builder
			.build_store(slot, value.value.expect("an item"))?;
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
		let (items, length) =
			self.array_parts(array.value.expect("an array").into_pointer_value())?;
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
		receiver: NodeId,
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
		let array = self.expr(receiver)?;
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
		match (path, values.as_slice()) {
			("Array#push", [value]) => {
				let value = self.copy_if_place(*value, arguments[0])?;
				self.array_push(array, value, node)?;
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

	/// A copy of a value read from a place (a local, a field, an item):
	/// arrays are values, so a binding or a store gets its own.
	pub(crate) fn copy_if_place(
		&mut self,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let is_place = matches!(
			self.view(inner),
			View::Name(_) | View::Access { .. } | View::Index { .. }
		);
		if !is_place || !self.needs_copy(value.repr) {
			return Ok(value);
		}
		self.copy_value(value, node)
	}

	/// Whether a value of this representation owns an array.
	fn needs_copy(&self, repr: Repr) -> bool {
		match repr {
			Repr::Array(_) => true,
			Repr::Object(ty) => self.types().fields_of(ty).iter().any(|field| {
				self.repr_of(field.ty, 0)
					.is_ok_and(|repr| self.needs_copy(repr))
			}),
			Repr::Optional(scalar) => self.needs_copy(scalar.repr()),
			_ => false,
		}
	}

	/// A deep copy of the arrays of a value.
	pub(crate) fn copy_value(
		&mut self,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		match value.repr {
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let size = self.item_size(item)?;
				let copy = self.runtime.function("fa_array_copy")?;
				let array = value.value.expect("an array").into_pointer_value();
				let copied = self
					.call_value(copy, &[array.into(), size.into()])?
					.into_pointer_value();
				if self.needs_copy(item) {
					// the items own arrays too
					let (items, length) = self.array_parts(copied)?;
					self.for_each_index(length, |this, index| {
						let pointer = this.item_pointer(items, item, index)?;
						let slot = Slot {
							pointer,
							repr: item,
						};
						let loaded = this.load(slot)?;
						let deep = this.copy_value(loaded, node)?;
						this.builder
							.build_store(pointer, deep.value.expect("an item"))?;
						Ok(())
					})?;
				}
				Ok(Val::new(value.repr, copied))
			}
			Repr::Object(ty) => {
				let mut structure = value.value.expect("an object").into_struct_value();
				let fields: Vec<TypeId> = self
					.types()
					.fields_of(ty)
					.iter()
					.map(|field| field.ty)
					.collect();
				for (index, field_type) in fields.into_iter().enumerate() {
					let repr = self.repr_of(field_type, node.index())?;
					if !self.needs_copy(repr) {
						continue;
					}
					let raw = self
						.builder
						.build_extract_value(structure, index as u32, "")?;
					let copied = self.copy_value(Val::new(repr, raw), node)?;
					structure = self
						.builder
						.build_insert_value(
							structure,
							copied.value.expect("a field"),
							index as u32,
							"",
						)?
						.into_struct_value();
				}
				Ok(Val::new(value.repr, structure))
			}
			_ => Ok(value),
		}
	}

	/// Runs `body` for every index in `0..length`.
	fn for_each_index(
		&mut self,
		length: IntValue<'ctx>,
		body: impl Fn(&mut Self, IntValue<'ctx>) -> Result<(), Error>,
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
		body(self, index)?;
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
		array: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Array(item_type) = array.repr else {
			return Err(self.unsupported(node.index(), "`for` over this value"));
		};
		let item = self.item_repr(item_type, node)?;
		let array = array.value.expect("an array").into_pointer_value();
		let variable =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => Some(self.slot_of(symbol, node)?),
				None => None,
			};
		let index_slot = match has_index
			.then(|| self.analysis.symbols.symbol_of_decl(node))
			.flatten()
		{
			Some(symbol) => Some(self.slot_of(symbol, node)?),
			None => None,
		};
		let counter = self.alloca(Repr::Int(IntRepr::I64), "position")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("for_item");
		let body_block = self.append_block("for_item_body");
		let step = self.append_block("for_item_step");
		let exit = self.append_block("end_for_item");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let (_, length) = self.array_parts(array)?;
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, length, "")?;
		self.builder
			.build_conditional_branch(more, body_block, exit)?;
		self.builder.position_at_end(body_block);
		let (items, _) = self.array_parts(array)?;
		let pointer = self.item_pointer(items, item, position)?;
		if let Some(variable) = variable {
			let value = self.load(Slot {
				pointer,
				repr: item,
			})?;
			self.store(variable, value, node)?;
		}
		if let Some(index_slot) = index_slot {
			self.store(
				index_slot,
				Val::new(Repr::Int(IntRepr::I64), position),
				node,
			)?;
		}
		self.frame_mut().loops.push(LoopFrame {
			node,
			exit,
			next: step,
			value: None,
		});
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let position = self.load(counter)?.int();
		if let Some(variable) = variable {
			// the item may have been assigned through the loop variable
			let (items, length) = self.array_parts(array)?;
			let inside = self
				.builder
				.build_int_compare(IntPredicate::SLT, position, length, "")?;
			let write = self.append_block("write_item");
			let next = self.append_block("next_item");
			self.builder.build_conditional_branch(inside, write, next)?;
			self.builder.position_at_end(write);
			let pointer = self.item_pointer(items, item, position)?;
			let value = self.load(variable)?;
			self.store(
				Slot {
					pointer,
					repr: item,
				},
				value,
				node,
			)?;
			self.builder.build_unconditional_branch(next)?;
			self.builder.position_at_end(next);
		}
		let next = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}
}
