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

	pub(crate) fn item_repr(&self, item: TypeId, node: NodeId) -> Result<Repr, Error> {
		// the items of an empty literal (`[]`, an `Array(Never)`) never exist
		match self.repr_of(item, node.index())? {
			Repr::Never | Repr::Void => Ok(Repr::Bool),
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
		Ok(Val::new(repr, array))
	}

	/// `array.length`.
	pub(crate) fn array_length(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let array = value.value.expect("an array").into_pointer_value();
		let (_, length) = self.array_parts(array)?;
		Ok(Val::new(Repr::Int(IntRepr::I64), length))
	}

	/// The pointer to an item, after the bounds check when the node has
	/// `INDEX_CHECK` (an `IndexError`). An index that may be negative counts
	/// from the end (`numbers[-1]`).
	fn checked_item(
		&mut self,
		node: NodeId,
		array: PointerValue<'ctx>,
		item: Repr,
		(index, may_be_negative): (Val<'ctx>, bool),
	) -> Result<PointerValue<'ctx>, Error> {
		let index = self
			.convert(index, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let (items, length) = self.array_parts(array)?;
		let index = if may_be_negative {
			let zero = index.get_type().const_zero();
			let negative = self
				.builder
				.build_int_compare(IntPredicate::SLT, index, zero, "")?;
			let from_end = self.builder.build_int_add(index, length, "")?;
			self.builder
				.build_select(negative, from_end, index, "")?
				.into_int_value()
		} else {
			index
		};
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
		let may_be_negative = self
			.analysis
			.interval(index)
			.or_else(|| self.types().interval_of(self.type_of(index)))
			.is_none_or(|interval| interval.lo < ethnum::I256::ZERO);
		let index = self.expr(index)?;
		let array = array.value.expect("an array").into_pointer_value();
		let pointer = self.checked_item(node, array, item, (index, may_be_negative))?;
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
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
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
		let (items, _) = self.array_parts(array)?;
		let pointer = self.item_pointer(items, item, index)?;
		self.load(Slot {
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

	/// Calls a callback with an item and its index.
	fn callback(
		&mut self,
		callback: Val<'ctx>,
		callback_node: NodeId,
		arguments: &[Val<'ctx>],
	) -> Result<Val<'ctx>, Error> {
		let ty = self.type_of(callback_node);
		self.call_closure(callback, ty, arguments, callback_node)
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
					let value = this.load_item(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let mapped = this.callback(*callback, callback_node, &[value, index_value])?;
					let mapped = this.convert(mapped, result_item, node.index())?;
					let (items, _) = this.array_parts(result)?;
					let pointer = this.item_pointer(items, result_item, index)?;
					if let Some(raw) = mapped.value {
						this.builder.build_store(pointer, raw)?;
					}
					Ok(())
				})?;
				Ok(Val::new(Repr::Array(result_type), result))
			}
			("Array#filter", [callback]) => {
				let (_, length) = self.array_parts(array)?;
				let result = self.empty_array(item, length)?;
				let callback_node = arguments[0];
				let result_value = Val::new(array_value.repr, result);
				self.for_each_index(length, |this, index| {
					let value = this.load_item(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let keep = this.callback(*callback, callback_node, &[value, index_value])?;
					let push = this.append_block("filter_keep");
					let next = this.append_block("filter_next");
					this.builder
						.build_conditional_branch(keep.int(), push, next)?;
					this.builder.position_at_end(push);
					this.array_push(result_value, value, node)?;
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
					let value = this.load_item(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let delete = this.callback(*callback, callback_node, &[value, index_value])?;
					let keep = this.append_block("delete_keep");
					let next = this.append_block("delete_next");
					this.builder
						.build_conditional_branch(delete.int(), next, keep)?;
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
				let repr = self.node_repr(node)?;
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
					let value = this.load_item(array, item, index)?;
					let index_value = Val::new(Repr::Int(IntRepr::I64), index);
					let holds = this.callback(*callback, callback_node, &[value, index_value])?;
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
				self.store(accumulator, *initial, arguments[1])?;
				let (_, length) = self.array_parts(array)?;
				let callback_node = arguments[0];
				self.for_each_index(length, |this, index| {
					let value = this.load_item(array, item, index)?;
					let current = this.load(accumulator)?;
					let next = this.callback(*callback, callback_node, &[current, value])?;
					this.store(accumulator, next, node)?;
					Ok(())
				})?;
				self.load(accumulator)
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
				let repr = self.node_repr(node)?;
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
				self.load(result)
			}
			("Array#clear", []) => {
				let header = self.header_type();
				let length_pointer = self.builder.build_struct_gep(header, array, 1, "")?;
				self.builder.build_store(length_pointer, i64.const_zero())?;
				Ok(Val::void())
			}
			("Array#reverse" | "Array#reverseInPlace", []) => {
				let target = if path == "Array#reverse" {
					let copy = self.copy_value(array_value, node)?;
					copy.value.expect("an array").into_pointer_value()
				} else {
					array
				};
				let size = self.item_size(item)?;
				self.call_runtime("fa_array_reverse", &[target.into(), size.into()])?;
				Ok(if path == "Array#reverse" {
					Val::new(array_value.repr, target)
				} else {
					Val::void()
				})
			}
			("Array#insert", [index, value]) => {
				let position = self.normalized_index(array, *index, node, true)?;
				let value = self.convert(*value, item, node.index())?;
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
					value
				} else {
					Val::void()
				})
			}
			("Array#sort" | "Array#sortInPlace", _) => {
				let target = if path == "Array#sort" {
					let copy = self.copy_value(array_value, node)?;
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
					Val::new(array_value.repr, target)
				} else {
					Val::void()
				})
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
				let order = self.callback(closure, callback_node, &[a, b])?;
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

	/// Whether the items of an array copy bytewise (they own no array).
	pub(crate) fn copies_bytewise(&self, item_type: TypeId, node: NodeId) -> Result<bool, Error> {
		Ok(!self.needs_copy(self.item_repr(item_type, node)?))
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
