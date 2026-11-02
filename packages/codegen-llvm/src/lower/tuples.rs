//! Tuples, ranges as values, slices, `is in`, string indexing and
//! destructuring.
//!
//! - A tuple is a struct of its items, by value.
//! - A range is `{ i64 start, i64 end, i8 flags }`: bit 0 a start, bit 1 an
//!   end, bit 2 an inclusive end (`..=`).
//! - Destructuring follows the analyzer's [`PatternInfo`]: the value is
//!   evaluated once, then each element is read and bound in order.

use fa_analyzer::analysis::{NodeFlags, PatternBinding};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::TypeKind;
use inkwell::IntPredicate;
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
		let mut value = structure.get_undef();
		for (index, (item, item_type)) in items.iter().zip(item_types).enumerate() {
			let lowered = self.expr(item)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let item_repr = self.repr_of(item_type, item.index())?;
			let lowered = self.owned_as(lowered, item_repr, Some(item))?;
			if let Some(raw) = lowered.value {
				value = self
					.builder
					.build_insert_value(value, raw, index as u32, "")?
					.into_struct_value();
			}
		}
		Ok(self.fresh(Val::new(repr, value)))
	}

	/// Appends `(1, "one")`.
	pub(crate) fn append_tuple(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		let structure = value.value.expect("a tuple").into_struct_value();
		for (index, item_type) in self.tuple_items(ty).into_iter().enumerate() {
			let prefix = if index == 0 { "(" } else { ", " };
			let prefix = self.string_constant(prefix);
			self.append_value(prefix, node)?;
			let repr = self.repr_of(item_type, node.index())?;
			let raw = self
				.builder
				.build_extract_value(structure, index as u32, "")?;
			self.append_nested(Val::new(repr, raw), node)?;
		}
		let close = self.string_constant(")");
		self.append_value(close, node)
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
				let slice = self.runtime.function("fa_string_slice")?;
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
			Repr::Array(_)
				if self.node_repr(index)? != Repr::Range
					&& let Some(code_points) = self.code_points_of(node) =>
			{
				Ok(Some(self.code_point_index(node, code_points, index)?))
			}
			Repr::Array(item_type) if self.node_repr(index)? == Repr::Range => {
				let array = self.expr(target)?;
				let range = self.expr(index)?;
				let item = self.item_repr(item_type, node)?;
				let size = inkwell::types::BasicType::size_of(&self.llvm_type(item))
					.expect("a sized item");
				let size = self
					.builder
					.build_int_cast(size, self.context.i64_type(), "")?;
				let (start, end, flags) = self.range_parts(range)?;
				let out = self.alloca(Repr::Array(item_type), "slice")?;
				let slice = self.runtime.function("fa_array_slice")?;
				let error = self
					.call_value(
						slice,
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
				// the items are copied bytewise: an item that owns memory needs its own
				let sliced = self.load(out)?;
				self.copy_items(sliced.value.expect("an array").into_pointer_value(), item)?;
				Ok(Some(self.fresh(sliced)))
			}
			_ => Ok(None),
		}
	}

	/// The place `tuple[0]`, or `None` for another target.
	pub(crate) fn tuple_place(
		&mut self,
		target: NodeId,
		index: NodeId,
	) -> Result<Option<Slot<'ctx>>, Error> {
		let Repr::Tuple(ty) = self.node_repr(target)? else {
			return Ok(None);
		};
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
					let target = self.repr_of(element.ty, pattern.index())?;
					let result = self.alloca(target, "with_default")?;
					let (present, inner) = {
						let pair = read.value.expect("an optional").into_struct_value();
						let present = self
							.builder
							.build_extract_value(pair, 0, "")?
							.into_int_value();
						let Repr::Optional(scalar) = read.repr else {
							unreachable!("an optional")
						};
						let inner = self.builder.build_extract_value(pair, 1, "")?;
						(present, Val::new(scalar.repr(), inner))
					};
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
		if info.rest.is_some() {
			return Err(self.unsupported(pattern.index(), "a rest pattern"));
		}
		Ok(())
	}
}
