//! Tuples, ranges as values, slices, `is in`, string indexing and
//! destructuring.
//!
//! - A tuple is a struct of its items, by value.
//! - A range is `{ i64 start, i64 end, i8 flags }`: bit 0 a start, bit 1 an
//!   end, bit 2 an inclusive end (`..=`).
//! - Destructuring follows the analyzer's [`PatternInfo`]: the value is
//!   evaluated once, then each element is read and bound in order.

use fa_analyzer::analysis::{NodeFlags, PatternBinding};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::TypeKind;
use inkwell::IntPredicate;
use inkwell::types::BasicType;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{Lowering, Slot};

pub(crate) const HAS_START: u64 = 1;
pub(crate) const HAS_END: u64 = 2;
pub(crate) const INCLUSIVE: u64 = 4;

impl<'ctx> Lowering<'_, 'ctx> {
	/// The struct of a tuple type.
	pub(crate) fn tuple_type(&self, ty: TypeId) -> StructType<'ctx> {
		if self.inline_info(ty).is_some() {
			return self.inline_type(ty);
		}
		let items: Vec<BasicTypeEnum<'ctx>> = self
			.tuple_items(ty)
			.iter()
			.map(|item| {
				let repr = self.repr_of(*item, 0).unwrap_or(Repr::Bool);
				if repr.has_value() {
					self.llvm_type(repr)
				} else {
					self.context.bool_type().into()
				}
			})
			.collect();
		self.context.struct_type(&items, false)
	}

	/// The item types of a tuple type.
	pub(crate) fn tuple_items(&self, ty: TypeId) -> Vec<TypeId> {
		match self.types().kind(ty) {
			TypeKind::Tuple(list) => self.types().list_items(list).to_vec(),
			TypeKind::CArray(item, count) => vec![item; count as usize],
			_ => Vec::new(),
		}
	}

	/// `{ i64 start, i64 end, i8 flags }`.
	pub(crate) fn range_type(&self) -> StructType<'ctx> {
		let i64 = self.context.i64_type();
		self.context.struct_type(
			&[i64.into(), i64.into(), self.context.i8_type().into()],
			false,
		)
	}

	/// `(a, b, c)`.
	pub(crate) fn tuple_literal(
		&mut self,
		node: NodeId,
		items: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let Repr::Tuple(ty) = repr else {
			return Err(self.unsupported(node.index(), "this tuple"));
		};
		let item_types = self.tuple_items(ty);
		let structure = self.tuple_type(ty);
		let destination = self.destination_for(node, repr);
		let mut completed = Vec::new();
		let mut value = structure.get_undef();
		for (index, (item, item_type)) in items.iter().zip(item_types).enumerate() {
			let item_repr = self.repr_of(item_type, item.index())?;
			let field_destination = destination
				.map(|slot| {
					self.builder
						.build_struct_gep(structure, slot.pointer, index as u32, "result.item")
						.map(|pointer| super::Slot {
							pointer,
							repr: item_repr,
						})
				})
				.transpose()?;
			let lowered = self.expr_with_destination(item, field_destination)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let lowered = self.owned_as(lowered, item_repr, Some(item))?;
			completed.push(self.fresh(lowered));
			if let Some(slot) = field_destination {
				self.store(slot, lowered, item)?;
			} else if let Some(raw) = lowered.value {
				value = self
					.builder
					.build_insert_value(value, raw, index as u32, "")?
					.into_struct_value();
			}
		}
		for item in completed {
			self.take(item, None)?;
		}
		Ok(self.fresh(match destination {
			Some(slot) => self.load_destination(slot)?,
			None => Val::new(repr, value),
		}))
	}

	/// Appends `(1, "one")`.
	pub(crate) fn append_tuple(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		if let Some((item, _)) = self.inline_info(ty) {
			let view = self.inline_view(value, ty)?;
			return self.append_array(view, item, node);
		}
		let structure = value.value.expect("a tuple").into_struct_value();
		// a `CArray` prints like an array
		let c_array = matches!(self.types().kind(ty), TypeKind::CArray(..));
		let (open, close) = if c_array { ("[", "]") } else { ("(", ")") };
		for (index, item_type) in self.tuple_items(ty).into_iter().enumerate() {
			let prefix = if index == 0 { open } else { ", " };
			let prefix = self.string_constant(prefix);
			self.append_value(prefix, node)?;
			let repr = self.repr_of(item_type, node.index())?;
			let raw = self
				.builder
				.build_extract_value(structure, index as u32, "")?;
			self.append_nested_of(Val::new(repr, raw), item_type, node)?;
		}
		let close = self.string_constant(if self.tuple_items(ty).is_empty() {
			if c_array { "[]" } else { "()" }
		} else {
			close
		});
		self.append_value(close, node)
	}

	/// The `n` of a `CArray(T, n)` type.
	pub(crate) fn c_array_count(&self, ty: TypeId) -> Option<u32> {
		match self.types().kind(ty) {
			TypeKind::CArray(_, count) => Some(count),
			_ => None,
		}
	}

	/// The address of item `index` of the `CArray` at `pointer`: counted
	/// from the end when negative, `IndexError` outside (unless proven).
	pub(crate) fn c_array_element(
		&mut self,
		pointer: inkwell::values::PointerValue<'ctx>,
		ty: TypeId,
		index: NodeId,
		node: NodeId,
	) -> Result<Option<Slot<'ctx>>, Error> {
		let Some(count) = self.c_array_count(ty) else {
			return Ok(None);
		};
		let item_type = self
			.tuple_items(ty)
			.first()
			.copied()
			.unwrap_or(TypeId::INTEGER);
		let repr = self.repr_of(item_type, node.index())?;
		let i64 = self.context.i64_type();
		let index_value = self.expr(index)?;
		if index_value.repr == Repr::Never {
			return Ok(None);
		}
		let index_value = self
			.convert(
				index_value,
				Repr::Int(crate::lower::values::IntRepr::I64),
				index.index(),
			)?
			.int();
		let length = i64.const_int(u64::from(count), false);
		let negative = self.builder.build_int_compare(
			inkwell::IntPredicate::SLT,
			index_value,
			i64.const_zero(),
			"",
		)?;
		let from_end = self.builder.build_int_add(index_value, length, "")?;
		let position = self
			.builder
			.build_select(negative, from_end, index_value, "")?
			.into_int_value();
		if self
			.flags(node)
			.contains(fa_analyzer::analysis::NodeFlags::INDEX_CHECK)
		{
			let outside =
				self.builder
					.build_int_compare(inkwell::IntPredicate::UGE, position, length, "")?;
			self.guard(outside, |this| {
				this.raise_runtime("fa_error_index", &[index_value.into(), length.into()])
			})?;
		}
		let array_type = self.llvm_type(repr).array_type(count);
		let element = unsafe {
			self.builder.build_in_bounds_gep(
				array_type,
				pointer,
				&[i64.const_zero(), position],
				"",
			)?
		};
		Ok(Some(Slot {
			pointer: element,
			repr,
		}))
	}

	/// `for item, index in values` over a `CArray`: over an array of its
	/// items, written back to its place after the loop (what the loop
	/// variable wrote).
	pub(crate) fn for_c_array(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		value: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Tuple(ty) = value.repr else {
			return Err(self.unsupported(iterable.index(), "`for` over this value"));
		};
		let item_types = self.tuple_items(ty);
		let Some(&item_type) = item_types.first() else {
			return Ok(Val::void());
		};
		let item = self.repr_of(item_type, node.index())?;
		let count = self
			.context
			.i64_type()
			.const_int(item_types.len() as u64, false);
		let array = self.new_array(item, count, count)?;
		let (items, _) = self.array_parts(array)?;
		let structure = value.value.expect("a CArray").into_struct_value();
		for index in 0..item_types.len() {
			let raw = self
				.builder
				.build_extract_value(structure, index as u32, "")?;
			let position = self.context.i64_type().const_int(index as u64, false);
			let slot = self.item_pointer(items, item, position)?;
			self.builder.build_store(slot, raw)?;
		}
		let array_value = self.fresh(Val::new(Repr::Array(item_type), array));
		let result = self.for_items(node, pattern, has_index, iterable, array_value, body)?;
		// what the loop variable wrote, back into the place
		if self.is_place(iterable) {
			let place = self.place(iterable)?;
			let array_type = self.llvm_type(item).array_type(item_types.len() as u32);
			let i64 = self.context.i64_type();
			for index in 0..item_types.len() {
				let position = i64.const_int(index as u64, false);
				let source = self.item_pointer(items, item, position)?;
				let loaded = self.builder.build_load(self.llvm_type(item), source, "")?;
				let target = unsafe {
					self.builder.build_in_bounds_gep(
						array_type,
						place.pointer,
						&[i64.const_zero(), position],
						"",
					)?
				};
				self.builder.build_store(target, loaded)?;
			}
		}
		Ok(result)
	}

	/// The constant index of a tuple item (`pair[0]`).
	fn tuple_index(&self, index: NodeId) -> Option<u32> {
		match self.view(index) {
			View::Integer(value) => u32::try_from(value.as_i128()).ok(),
			View::Group(inner) => self.tuple_index(inner),
			_ => self.types().as_int(self.type_of(index)).and_then(|int| {
				int.literal()
					.and_then(|value| u32::try_from(value.as_i128()).ok())
			}),
		}
	}

	/// `target[index]` on a tuple, a string, a slice of an array; `None`
	/// for an item of an array.
	pub(crate) fn index_special(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		match self.node_repr(target)? {
			Repr::Tuple(ty) if self.inline_info(ty).is_some() => {
				let slot = if self.is_place(target) {
					self.place(target)?
				} else {
					let value = self.expr(target)?;
					if value.repr == Repr::Never {
						return Ok(Some(value));
					}
					let slot = self.alloca(value.repr, "inline.indexed")?;
					self.builder
						.build_store(slot.pointer, value.value.expect("an inline array"))?;
					slot
				};
				Ok(Some(match self.inline_element(slot, ty, index, node)? {
					Some(slot) => self.load(slot)?,
					None => Val::never(),
				}))
			}
			// `values[i]` of a `CArray`: from memory
			Repr::Tuple(ty) if self.c_array_count(ty).is_some() => {
				let value = self.expr(target)?;
				if value.repr == Repr::Never {
					return Ok(Some(value));
				}
				let memory = self.alloca(value.repr, "c_array")?;
				self.builder
					.build_store(memory.pointer, value.value.expect("a CArray"))?;
				match self.c_array_element(memory.pointer, ty, index, node)? {
					Some(slot) => Ok(Some(self.load(slot)?)),
					None => Ok(Some(Val::never())),
				}
			}
			Repr::Tuple(ty) => {
				let Some(position) = self.tuple_index(index) else {
					return Err(self.unsupported(index.index(), "this tuple index"));
				};
				let value = self.expr(target)?;
				let item_type = self.tuple_items(ty)[position as usize];
				let repr = self.repr_of(item_type, node.index())?;
				let structure = value.value.expect("a tuple").into_struct_value();
				let raw = self.builder.build_extract_value(structure, position, "")?;
				Ok(Some(Val::new(repr, raw)))
			}
			Repr::Str if self.node_repr(index)? == Repr::Range => {
				// `text[start..end]`: a copy of the characters (a view of a literal)
				let text = self.expr(target)?;
				let range = self.expr(index)?;
				if text.repr == Repr::Never || range.repr == Repr::Never {
					return Ok(Some(Val::never()));
				}
				let (bytes, length) = self.string_parts(text)?;
				let capacity = self.string_capacity(text)?;
				let (start, end, flags) = self.range_parts(range)?;
				let out = self.alloca(Repr::Str, "slice")?;
				let slice = if self.slice_into_region(node, target, index) {
					"fa_string_slice_region"
				} else {
					"fa_string_slice"
				};
				let slice = self.runtime.function(slice)?;
				let error = self
					.call_value(
						slice,
						&[
							bytes.into(),
							length.into(),
							capacity.into(),
							start.into(),
							end.into(),
							flags.into(),
							out.pointer.into(),
						],
					)?
					.into_pointer_value();
				self.raise_if_error(error)?;
				Ok(Some(self.fresh(self.load(out)?)))
			}
			Repr::Str => {
				let text = self.expr(target)?;
				let position = self.expr(index)?;
				let position = self
					.convert(position, Repr::Int(IntRepr::I64), index.index())?
					.int();
				let (bytes, length) = self.string_parts(text)?;
				let capacity = self.string_capacity(text)?;
				let out = self.alloca(Repr::Str, "character")?;
				let get = self.runtime.function("fa_string_get")?;
				let found = self
					.call_value(
						get,
						&[
							bytes.into(),
							length.into(),
							capacity.into(),
							position.into(),
							out.pointer.into(),
						],
					)?
					.into_int_value();
				let missing = self.builder.build_int_compare(
					IntPredicate::EQ,
					found,
					found.get_type().const_zero(),
					"",
				)?;
				self.guard(missing, |this| {
					// the length only on the error path
					let graphemes = this.runtime.function("fa_string_length")?;
					let count = this
						.call_value(graphemes, &[bytes.into(), length.into(), capacity.into()])?
						.into_int_value();
					let count = this.builder.build_int_z_extend_or_bit_cast(
						count,
						this.context.i64_type(),
						"",
					)?;
					this.raise_runtime("fa_error_index", &[position.into(), count.into()])
				})?;
				// the character is a new string
				Ok(Some(self.fresh(self.load(out)?)))
			}
			Repr::Map(_) => {
				let map = self.expr(target)?;
				let key = self.expr(index)?;
				if map.repr == Repr::Never || key.repr == Repr::Never {
					return Ok(Some(Val::never()));
				}
				Ok(Some(self.table_index(node, map, key)?))
			}
			Repr::Array(_) | Repr::Bytes
				if self.node_repr(index)? != Repr::Range
					&& let Some(code_points) = self.code_points_of(node) =>
			{
				Ok(Some(self.code_point_index(node, code_points, index)?))
			}
			Repr::Bytes if self.node_repr(index)? == Repr::Range => {
				Ok(Some(self.bytes_slice(node, target, index, false)?))
			}
			Repr::Array(item_type) if self.node_repr(index)? == Repr::Range => {
				let array = self.expr(target)?;
				let range = self.expr(index)?;
				let (start, end, flags) = self.range_parts(range)?;
				Ok(Some(self.array_slice_value(
					node, array, item_type, start, end, flags,
				)?))
			}
			_ => Ok(None),
		}
	}

	/// An owned slice, shared by indexing and array rest patterns.
	pub(crate) fn array_slice_value(
		&mut self,
		node: NodeId,
		array: Val<'ctx>,
		item_type: TypeId,
		start: IntValue<'ctx>,
		end: IntValue<'ctx>,
		flags: IntValue<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let item = self.item_repr(item_type, node)?;
		let size = self.item_size(item)?;
		let out = self.alloca(Repr::Array(item_type), "slice")?;
		let error = self
			.call_value(
				self.runtime.function("fa_array_slice")?,
				&[
					array.value.expect("an array").into(),
					start.into(),
					end.into(),
					flags.into(),
					size.into(),
					out.pointer.into(),
				],
			)?
			.into_pointer_value();
		self.raise_if_error(error)?;
		let sliced = self.load(out)?;
		self.copy_items(sliced.value.expect("an array").into_pointer_value(), item)?;
		Ok(self.fresh(sliced))
	}

	/// The place `tuple[0]`, or `None` for another target.
	pub(crate) fn tuple_place(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Option<Slot<'ctx>>, Error> {
		let Repr::Tuple(ty) = self.node_repr(target)? else {
			return Ok(None);
		};
		if self.inline_info(ty).is_some() {
			let slot = self.place(target)?;
			return self.inline_element(slot, ty, index, node);
		}
		// `values[i] = …` of a `CArray`: in its place
		if self.c_array_count(ty).is_some() {
			let slot = self.place(target)?;
			return self.c_array_element(slot.pointer, ty, index, node);
		}
		let Some(position) = self.tuple_index(index) else {
			return Err(self.unsupported(index.index(), "this tuple index"));
		};
		let slot = self.place(target)?;
		let item_type = self.tuple_items(ty)[position as usize];
		let pointer =
			self.builder
				.build_struct_gep(self.tuple_type(ty), slot.pointer, position, "")?;
		Ok(Some(Slot {
			pointer,
			repr: self.repr_of(item_type, index.index())?,
		}))
	}

	/// A range literal as a value (`1..3`, `..=10`, `5..`).
	pub(crate) fn range_value(
		&mut self,
		start: Option<NodeId>,
		end: Option<NodeId>,
		inclusive: bool,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let mut flags = if inclusive { INCLUSIVE } else { 0 };
		let mut bound =
			|this: &mut Self, node: Option<NodeId>, flag: u64| -> Result<IntValue<'ctx>, Error> {
				match node {
					Some(node) => {
						flags |= flag;
						let value = this.expr(node)?;
						Ok(this
							.convert(value, Repr::Int(IntRepr::I64), node.index())?
							.int())
					}
					None => Ok(i64.const_zero()),
				}
			};
		let start = bound(self, start, HAS_START)?;
		let end = bound(self, end, HAS_END)?;
		let structure = self.range_type();
		let value = self
			.builder
			.build_insert_value(structure.get_undef(), start, 0, "")?;
		let value = self.builder.build_insert_value(value, end, 1, "")?;
		let flags = self.context.i8_type().const_int(flags, false);
		let value = self.builder.build_insert_value(value, flags, 2, "")?;
		Ok(Val::new(Repr::Range, value.into_struct_value()))
	}

	/// The start, the end and the flags of a range.
	pub(crate) fn range_parts(
		&self,
		range: Val<'ctx>,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>), Error> {
		let structure = range.value.expect("a range").into_struct_value();
		let start = self
			.builder
			.build_extract_value(structure, 0, "")?
			.into_int_value();
		let end = self
			.builder
			.build_extract_value(structure, 1, "")?
			.into_int_value();
		let flags = self
			.builder
			.build_extract_value(structure, 2, "")?
			.into_int_value();
		Ok((start, end, flags))
	}

	/// Writes a logical range field into its packed bounds/flags storage.
	pub(crate) fn range_assignment(
		&mut self,
		target: NodeId,
		value: Val<'ctx>,
	) -> Result<bool, Error> {
		let View::Access {
			target: base,
			members,
		} = self.view(target)
		else {
			return Ok(false);
		};
		let Some(member) = members.iter().last() else {
			return Ok(false);
		};
		let Resolution::Builtin(builtin) = self.analysis.resolution(member) else {
			return Ok(false);
		};
		let (index, flag) = match fa_analyzer::builtins::get(builtin).path {
			"Range#start" => (0, HAS_START),
			"Range#end" => (1, HAS_END),
			"Range#includesEnd" => (2, INCLUSIVE),
			_ => return Ok(false),
		};
		let slot = self.place(base)?;
		let slot = self.place_members(
			slot,
			self.type_of(base),
			members.iter().take(members.len() - 1),
		)?;
		let range = self.load(slot)?.value.expect("a range").into_struct_value();
		let flags = self
			.builder
			.build_extract_value(range, 2, "")?
			.into_int_value();
		let (present, range) = if index == 2 {
			(
				self.convert(value, Repr::Bool, target.index())?.int(),
				range,
			)
		} else {
			let value = self.convert(
				value,
				Repr::Optional(Scalar::Int(IntRepr::I64)),
				target.index(),
			)?;
			let (present, bound) = self.split_optional(&self.builder, value)?;
			// Keep absent bounds canonical, including for range keys' hashes.
			let bound = self.builder.build_select(
				present,
				bound.int(),
				self.context.i64_type().const_zero(),
				"",
			)?;
			let range = self
				.builder
				.build_insert_value(range, bound, index, "")?
				.into_struct_value();
			(present, range)
		};
		let bit = flags.get_type().const_int(flag, false);
		let cleared = self
			.builder
			.build_and(flags, self.builder.build_not(bit, "")?, "")?;
		let set = self.builder.build_or(cleared, bit, "")?;
		let flags = self.builder.build_select(present, set, cleared, "")?;
		let range = self.builder.build_insert_value(range, flags, 2, "")?;
		self.store(
			slot,
			Val::new(Repr::Range, range.into_struct_value()),
			target,
		)?;
		Ok(true)
	}

	pub(crate) fn ranges_equal(
		&self,
		left: Val<'ctx>,
		right: Val<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let (a, b, flags) = self.range_parts(left)?;
		let (c, d, other_flags) = self.range_parts(right)?;
		let mut equal = self
			.builder
			.build_int_compare(IntPredicate::EQ, flags, other_flags, "")?;
		for (left, right, flag) in [(a, c, HAS_START), (b, d, HAS_END)] {
			let same = self
				.builder
				.build_int_compare(IntPredicate::EQ, left, right, "")?;
			let absent = self.builder.build_not(self.range_flag(flags, flag)?, "")?;
			let same = self.builder.build_or(absent, same, "")?;
			equal = self.builder.build_and(equal, same, "")?;
		}
		Ok(equal)
	}

	/// Whether a flag of a range is set.
	fn range_flag(&self, flags: IntValue<'ctx>, flag: u64) -> Result<IntValue<'ctx>, Error> {
		let i8 = self.context.i8_type();
		let bit = self
			.builder
			.build_and(flags, i8.const_int(flag, false), "")?;
		Ok(self
			.builder
			.build_int_compare(IntPredicate::NE, bit, i8.const_zero(), "")?)
	}

	/// `range.start`, `range.end` (optionals, `none` when open) and `range.includesEnd`.
	pub(crate) fn range_property(
		&mut self,
		node: NodeId,
		range: Val<'ctx>,
		property: &str,
	) -> Result<Val<'ctx>, Error> {
		let (start, end, flags) = self.range_parts(range)?;
		if property == "includesEnd" {
			return Ok(Val::new(Repr::Bool, self.range_flag(flags, INCLUSIVE)?));
		}
		let (value, flag) = if property == "start" {
			(start, HAS_START)
		} else {
			(end, HAS_END)
		};
		let present = self.range_flag(flags, flag)?;
		let repr = self.node_repr(node)?;
		match repr {
			Repr::Optional(_) => {
				let optional = Repr::Optional(Scalar::Int(IntRepr::I64));
				let ty = self.llvm_type(optional).into_struct_type();
				let pair = self
					.builder
					.build_insert_value(ty.get_undef(), present, 0, "")?;
				let pair = self.builder.build_insert_value(pair, value, 1, "")?;
				self.convert(
					Val::new(optional, pair.into_struct_value()),
					repr,
					node.index(),
				)
			}
			_ => Ok(Val::new(Repr::Int(IntRepr::I64), value)),
		}
	}

	/// Appends `1..3`, `0..=10`, `5..`, `..10`, `..`.
	pub(crate) fn append_range(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		let (start, end, flags) = self.range_parts(value)?;
		let call = |this: &mut Self, flag: u64, bound: IntValue<'ctx>| -> Result<(), Error> {
			let present = this.range_flag(flags, flag)?;
			let print = this.append_block("print_bound");
			let next = this.append_block("printed_bound");
			this.builder
				.build_conditional_branch(present, print, next)?;
			this.builder.position_at_end(print);
			this.call_runtime("fa_b_i64", &[bound.into()])?;
			this.builder.build_unconditional_branch(next)?;
			this.builder.position_at_end(next);
			Ok(())
		};
		call(self, HAS_START, start)?;
		let dots = self.string_constant("..");
		self.append_value(dots, node)?;
		let inclusive = self.range_flag(flags, INCLUSIVE)?;
		let has_end = self.range_flag(flags, HAS_END)?;
		let equals = self.builder.build_and(inclusive, has_end, "")?;
		let print = self.append_block("print_equals");
		let next = self.append_block("printed_equals");
		self.builder.build_conditional_branch(equals, print, next)?;
		self.builder.position_at_end(print);
		let sign = self.string_constant("=");
		self.append_value(sign, node)?;
		self.builder.build_unconditional_branch(next)?;
		self.builder.position_at_end(next);
		call(self, HAS_END, end)
	}

	/// `for i in range` over a range value: from its start, up to its end
	/// (or `Integer.maxValue`).
	pub(crate) fn for_range_value(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		range: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let (start, end, flags) = self.range_parts(range)?;
		let has_end = self.range_flag(flags, HAS_END)?;
		let inclusive = self.range_flag(flags, INCLUSIVE)?;
		// the last value: `end - 1`, `end` (inclusive), or `Integer.maxValue`
		let before_end = self
			.builder
			.build_int_sub(end, i64.const_int(1, false), "")?;
		let last = self
			.builder
			.build_select(inclusive, end, before_end, "")?
			.into_int_value();
		let last = self
			.builder
			.build_select(has_end, last, i64.const_int(i64::MAX as u64, true), "")?
			.into_int_value();
		let counter =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => self.slot_of(symbol, node)?,
				None => self.alloca(Repr::Int(IntRepr::I64), "counter")?,
			};
		self.builder.build_store(counter.pointer, start)?;
		let empty = self
			.builder
			.build_int_compare(IntPredicate::SGT, start, last, "")?;
		let body_block = self.append_block("range_body");
		let step = self.append_block("range_step");
		let exit = self.append_block("end_range");
		self.builder
			.build_conditional_branch(empty, exit, body_block)?;
		self.builder.position_at_end(body_block);
		let mark = self.mark();
		self.frame_mut().loops.push(super::LoopFrame {
			exit,
			next: step,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let current = self.load(counter)?.int();
		let done = self
			.builder
			.build_int_compare(IntPredicate::EQ, current, last, "")?;
		let increment = self.append_block("range_increment");
		self.builder
			.build_conditional_branch(done, exit, increment)?;
		self.builder.position_at_end(increment);
		let next = self
			.builder
			.build_int_nsw_add(current, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(body_block)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// Whether an integer is in a range value.
	pub(crate) fn in_range(
		&mut self,
		value: Val<'ctx>,
		range: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let value = self
			.convert(value, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let (start, end, flags) = self.range_parts(range)?;
		let has_start = self.range_flag(flags, HAS_START)?;
		let has_end = self.range_flag(flags, HAS_END)?;
		let inclusive = self.range_flag(flags, INCLUSIVE)?;
		let above = self
			.builder
			.build_int_compare(IntPredicate::SGE, value, start, "")?;
		let no_start = self.builder.build_not(has_start, "")?;
		let low = self.builder.build_or(no_start, above, "")?;
		let below = self
			.builder
			.build_int_compare(IntPredicate::SLT, value, end, "")?;
		let at_end = self
			.builder
			.build_int_compare(IntPredicate::EQ, value, end, "")?;
		let at_end = self.builder.build_and(at_end, inclusive, "")?;
		let below = self.builder.build_or(below, at_end, "")?;
		let no_end = self.builder.build_not(has_end, "")?;
		let high = self.builder.build_or(no_end, below, "")?;
		Ok(self.builder.build_and(low, high, "")?)
	}

	/// `value is in collection` (a range, an array).
	pub(crate) fn is_in(
		&mut self,
		node: NodeId,
		value: NodeId,
		collection: NodeId,
		negated: bool,
	) -> Result<Val<'ctx>, Error> {
		let lowered = self.expr(value)?;
		let container = self.expr(collection)?;
		if lowered.repr == Repr::Never || container.repr == Repr::Never {
			return Ok(Val::never());
		}
		let holds = match container.repr {
			Repr::Range => self.in_range(lowered, container, node)?,
			// a set of flags (fields.rs)
			Repr::Int(_) if self.is_fields(self.concrete_type(self.type_of(collection))) => {
				self.fields_is_in(lowered, container)?
			}
			Repr::Array(_) => self
				.array_method_more(node, "Array#includes", container, &[lowered], &[value])?
				.int(),
			Repr::Map(_) | Repr::Set(_) => self.table_includes(container, lowered, node)?,
			_ => return Err(self.unsupported(node.index(), "`is in` on this value")),
		};
		let holds = if negated {
			self.builder.build_not(holds, "")?
		} else {
			holds
		};
		Ok(Val::new(Repr::Bool, holds))
	}

	/// `let pattern = value`: the value once, then each element.
	pub(crate) fn destructure(
		&mut self,
		pattern: NodeId,
		value: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let lowered = self.expr(value)?;
		if lowered.repr == Repr::Never {
			return Ok(lowered);
		}
		// the bindings own copies of the elements
		self.bind_pattern(pattern, lowered)?;
		Ok(Val::void())
	}

	/// Destructures a received parameter before later defaults use its bindings.
	pub(crate) fn bind_parameter_pattern(
		&mut self,
		node: NodeId,
		symbol: SymbolId,
	) -> Result<(), Error> {
		if let View::DestructuredParameter { pattern, .. } = self.view(node) {
			let slot = self.slot_of(symbol, node)?;
			let value = self.load(slot)?;
			self.bind_pattern(pattern, value)?;
		}
		Ok(())
	}

	/// Binds the elements of a pattern ([`fa_analyzer::analysis::PatternInfo`]) to a value.
	pub(crate) fn bind_pattern(&mut self, pattern: NodeId, value: Val<'ctx>) -> Result<(), Error> {
		let Some(info) = self.analysis.pattern(pattern).cloned() else {
			return Err(self.unsupported(pattern.index(), "this pattern"));
		};
		// an array pattern whose length isn't proven: one length check
		if let Repr::Array(_) = value.repr
			&& self.flags(pattern).contains(NodeFlags::INDEX_CHECK)
		{
			let array = value.value.expect("an array").into_pointer_value();
			let (_, length) = self.array_parts(array)?;
			let count = self
				.context
				.i64_type()
				.const_int(info.elements.len() as u64, false);
			let short = self
				.builder
				.build_int_compare(IntPredicate::SLT, length, count, "")?;
			let last = self
				.context
				.i64_type()
				.const_int(info.elements.len().saturating_sub(1) as u64, false);
			self.guard(short, |this| {
				this.raise_runtime("fa_error_index", &[last.into(), length.into()])
			})?;
		}
		for (position, element) in info.elements.iter().enumerate() {
			let read = match (value.repr, element.field) {
				_ if element.absent => None,
				(Repr::Object(object), Some(field)) => {
					let types = self.types();
					let index = types
						.fields_of(object)
						.iter()
						.position(|entry| entry.name == field)
						.ok_or_else(|| self.unsupported(pattern.index(), "this field"))?;
					let field_type = types.fields_of(object)[index].ty;
					let repr = self.repr_of(field_type, pattern.index())?;
					let structure = value.value.expect("an object").into_struct_value();
					let raw = self
						.builder
						.build_extract_value(structure, index as u32, "")?;
					Some(Val::new(repr, raw))
				}
				(Repr::Tuple(ty), None) => {
					let item_type = self.tuple_items(ty)[position];
					let repr = self.repr_of(item_type, pattern.index())?;
					let structure = value.value.expect("a tuple").into_struct_value();
					let raw = self
						.builder
						.build_extract_value(structure, position as u32, "")?;
					Some(Val::new(repr, raw))
				}
				(Repr::Array(item_type), None) => {
					let item = self.repr_of(item_type, pattern.index())?;
					let array = value.value.expect("an array").into_pointer_value();
					self.materialize_array(array, item)?;
					let (items, _) = self.array_parts(array)?;
					let index = self.context.i64_type().const_int(position as u64, false);
					let pointer = unsafe {
						self.builder.build_in_bounds_gep(
							self.llvm_type(item),
							items,
							&[index],
							"",
						)?
					};
					Some(self.load(Slot {
						pointer,
						repr: item,
					})?)
				}
				_ => return Err(self.unsupported(pattern.index(), "destructuring this value")),
			};
			// the default: when the field is absent, or holds `none`
			let read = match (read, element.default) {
				(None, Some(default)) => self.expr(default)?,
				(Some(read), Some(default)) if matches!(read.repr, Repr::Optional(_)) => {
					let (present, inner) = self.split_optional(&self.builder, read)?;
					let target = match element.binding {
						PatternBinding::Symbol(symbol) => self.slot_of(symbol, pattern)?.repr,
						PatternBinding::Nested(nested) => {
							let ty = self.analysis.pattern(nested).expect("a nested pattern").ty;
							self.repr_of(ty, nested.index())?
						}
						PatternBinding::None => inner.repr,
					};
					let result = self.alloca(target, "with_default")?;
					let given = self.append_block("given_field");
					let fallback = self.append_block("default_field");
					let join = self.append_block("bound_field");
					self.builder
						.build_conditional_branch(present, given, fallback)?;
					self.builder.position_at_end(given);
					let inner = self.owned_as(inner, target, None)?;
					self.store(result, inner, pattern)?;
					self.builder.build_unconditional_branch(join)?;
					self.builder.position_at_end(fallback);
					let default_value =
						self.region(Some(target), default, |this| this.expr(default))?;
					self.store(result, default_value, default)?;
					self.branch_to(join)?;
					self.builder.position_at_end(join);
					self.fresh(self.load(result)?)
				}
				(Some(read), _) => read,
				(None, None) => return Err(self.unsupported(pattern.index(), "this pattern")),
			};
			match element.binding {
				PatternBinding::Symbol(symbol) => {
					let slot = self.slot_of(symbol, pattern)?;
					let read = self.owned_as(read, slot.repr, None)?;
					self.store(slot, read, pattern)?;
					self.own_local(symbol, slot);
				}
				PatternBinding::Nested(nested) => self.bind_pattern(nested, read)?,
				PatternBinding::None => {}
			}
		}
		self.bind_rest(&info, value, pattern)?;
		Ok(())
	}
	/// Rest bindings own a projection of the remaining fields or a slice.
	pub(crate) fn bind_rest(
		&mut self,
		info: &fa_analyzer::analysis::PatternInfo,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		let Some((symbol, _)) = info.rest else {
			return Ok(());
		};
		let slot = self.slot_of(symbol, node)?;
		let rest = match (value.repr, slot.repr) {
			(Repr::Array(item), Repr::Array(_)) => {
				let start = self
					.context
					.i64_type()
					.const_int(info.elements.len() as u64, false);
				self.array_slice_value(
					node,
					value,
					item,
					start,
					self.context.i64_type().const_zero(),
					self.context.i8_type().const_int(HAS_START, false),
				)?
			}
			(Repr::Object(source), Repr::Object(target)) => {
				let fields = self.types().fields_of(target).to_vec();
				let mut rest = self.object_type(target).get_undef();
				for (index, field) in fields.iter().enumerate() {
					let source_index = self
						.types()
						.fields_of(source)
						.iter()
						.position(|entry| entry.name == field.name)
						.expect("a rest field belongs to the source");
					let repr = self.field_repr(source, source_index, node.index())?;
					let raw = self.builder.build_extract_value(
						value.value.expect("an object").into_struct_value(),
						source_index as u32,
						"",
					)?;
					let read = Val::new(repr, raw);
					let target_repr = self.field_repr(target, index, node.index())?;
					let owned = self.owned_as(read, target_repr, None)?;
					if let Some(raw) = owned.value {
						rest = self
							.builder
							.build_insert_value(rest, raw, index as u32, "")?
							.into_struct_value();
					}
				}
				self.fresh(Val::new(slot.repr, rest))
			}
			_ => return Err(self.unsupported(node.index(), "a rest pattern of this value")),
		};
		let rest = self.owned_as(rest, slot.repr, None)?;
		self.store(slot, rest, node)?;
		self.own_local(symbol, slot);
		Ok(())
	}
}
