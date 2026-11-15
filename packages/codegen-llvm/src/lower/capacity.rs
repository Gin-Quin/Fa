//! Capacity natively (`design/decisions.md`, "Minimal representations"):
//! the `capacity` property of strings and collections, their
//! `reserve(additional)`, and `String.from(text, capacity = n)`.
//!
//! A string's capacity is the bytes it holds without a new block (23 inline,
//! its heap block, its length when it shares static bytes); `reserve` gives an
//! owned block room for that many more bytes (`fa_string_reserve`). An array
//! reserves items, a map or a set entries (`fa_table_reserve`). A growing
//! array, a pool and a columnar never move their elements: their capacity
//! counts the segments or the chunks they hold, and `reserve` allocates the
//! ones that are missing (`fa_segmented_reserve`, `fa_pool_reserve`).

use fa_analyzer::analysis::{Argument, CallInfo};
use fa_analyzer::ids::NodeId;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	/// `value.capacity`.
	pub(crate) fn capacity_property(
		&mut self,
		node: NodeId,
		value: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let capacity = match value.repr {
			Repr::Str => {
				let (_, length) = self.string_parts(value)?;
				let capacity = self.string_capacity(value)?;
				let room = self.runtime.function("fa_string_room")?;
				let room = self
					.call_value(room, &[length.into(), capacity.into()])?
					.into_int_value();
				self.builder.build_int_z_extend_or_bit_cast(room, i64, "")?
			}
			Repr::Array(_) => {
				// a compact array of characters (a negative capacity) holds
				// exactly its length
				let array = value.value.expect("an array").into_pointer_value();
				let (_, length) = self.array_parts(array)?;
				let header = self.header_type();
				let field = self.builder.build_struct_gep(header, array, 2, "")?;
				let capacity = self
					.builder
					.build_load(i64, field, "capacity")?
					.into_int_value();
				let compact = self.builder.build_int_compare(
					inkwell::IntPredicate::SLT,
					capacity,
					i64.const_zero(),
					"",
				)?;
				self.builder
					.build_select(compact, length, capacity, "")?
					.into_int_value()
			}
			Repr::Map(_) | Repr::Set(_) => {
				let table = value.value.expect("a table").into_pointer_value();
				let capacity = self.runtime.function("fa_table_capacity")?;
				self.call_value(capacity, &[table.into()])?.into_int_value()
			}
			Repr::Growing(_) => {
				let array = value.value.expect("an array").into_pointer_value();
				let capacity = self.runtime.function("fa_segmented_capacity")?;
				self.call_value(capacity, &[array.into()])?.into_int_value()
			}
			Repr::Pool(_) | Repr::Columnar(_) => {
				let pool = value.value.expect("a pool").into_pointer_value();
				let capacity = self.runtime.function("fa_pool_capacity")?;
				self.call_value(capacity, &[pool.into()])?.into_int_value()
			}
			_ => return Err(self.unsupported(node.index(), "the capacity of this value")),
		};
		Ok(Val::new(Repr::Int(IntRepr::I64), capacity))
	}

	/// `value.reserve(additional)` (`path` ends with `#reserve`).
	pub(crate) fn reserve_call(
		&mut self,
		node: NodeId,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some(Argument::Given(additional)) = info
			.arguments
			.iter()
			.find(|argument| matches!(argument, Argument::Given(_)))
		else {
			return Err(self.unsupported(node.index(), "this `reserve`"));
		};
		// a string is changed in its place; a collection through its handle
		let place = self.receiver_place(node)?;
		let receiver = match place {
			Some(place) => self.load(place)?,
			None => self.receiver(node)?,
		};
		if receiver.repr == Repr::Never {
			return Ok(receiver);
		}
		let additional_value = self.expr(*additional)?;
		if additional_value.repr == Repr::Never {
			return Ok(additional_value);
		}
		let additional_value = self.convert(
			additional_value,
			Repr::Int(IntRepr::I64),
			additional.index(),
		)?;
		let additional_value = additional_value.int();
		match receiver.repr {
			Repr::Str => {
				let Some(place) = place else {
					// a temporary: nothing keeps the room
					return Ok(Val::void());
				};
				let extra = self.string_byte_count(additional_value)?;
				self.call_runtime("fa_string_reserve", &[place.pointer.into(), extra.into()])?;
			}
			Repr::Array(item_type) => {
				let array = receiver.value.expect("an array").into_pointer_value();
				let (_, length) = self.array_parts(array)?;
				let saturating = self.intrinsic("llvm.sadd.sat", &[length.get_type().into()])?;
				let wanted = self
					.call_value(saturating, &[length.into(), additional_value.into()])?
					.into_int_value();
				let item = self.item_repr(item_type, node)?;
				let size = self.item_size(item)?;
				self.call_runtime(
					"fa_array_reserve",
					&[array.into(), wanted.into(), size.into()],
				)?;
			}
			Repr::Map(_) | Repr::Set(_) => {
				let table = self.table_of(receiver.repr)?;
				let pointer = receiver.value.expect("a table").into_pointer_value();
				let size = self.entry_size(table);
				// the keys are hashed again only if the index becomes wide
				let hash_at = self.hash_at_function(table.key)?;
				self.call_runtime(
					"fa_table_reserve",
					&[
						pointer.into(),
						additional_value.into(),
						size.into(),
						hash_at.as_global_value().as_pointer_value().into(),
					],
				)?;
			}
			Repr::Growing(_) | Repr::Pool(_) | Repr::Columnar(_) => {
				self.reserve_stable(receiver, additional_value, node)?;
			}
			_ => return Err(self.unsupported(node.index(), "`reserve` on this value")),
		}
		Ok(Val::void())
	}

	/// Room for `additional` more elements in a collection that never moves
	/// them (a growing array, a pool, a columnar): the segments or the chunks
	/// that are missing. Also the `capacity` of their `from`.
	pub(crate) fn reserve_stable(
		&self,
		receiver: Val<'ctx>,
		additional: IntValue<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		let pointer = receiver.value.expect("a collection").into_pointer_value();
		match receiver.repr {
			Repr::Growing(item) => {
				let item = self.item_repr(item, node)?;
				self.call_runtime(
					"fa_segmented_reserve",
					&[
						pointer.into(),
						additional.into(),
						self.item_size_const(item).into(),
						self.segment_k(item).into(),
					],
				)
			}
			Repr::Pool(_) | Repr::Columnar(_) => {
				self.call_runtime("fa_pool_reserve", &[pointer.into(), additional.into()])
			}
			_ => Err(self.unsupported(node.index(), "`reserve` on this value")),
		}
	}

	/// `String.from(text, capacity = n)`: a copy of the text with room for
	/// `n` bytes.
	pub(crate) fn string_from(
		&mut self,
		node: NodeId,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some(Argument::Given(text_node)) = info.arguments.first() else {
			return Err(self.unsupported(node.index(), "this `String.from`"));
		};
		let text = self.expr(*text_node)?;
		if text.repr == Repr::Never {
			return Ok(text);
		}
		let room = match info.arguments.get(1) {
			Some(Argument::Given(capacity)) => {
				let room = self.expr(*capacity)?;
				if room.repr == Repr::Never {
					return Ok(room);
				}
				let room = self.convert(room, Repr::Int(IntRepr::I64), capacity.index())?;
				self.string_byte_count(room.int())?
			}
			_ => self.usize.const_zero(),
		};
		let (bytes, length) = self.string_parts(text)?;
		let capacity = self.string_capacity(text)?;
		let result = self.call_with_pair(
			"fa_string_from",
			&[bytes.into(), length.into(), capacity.into(), room.into()],
			Repr::Str,
		)?;
		Ok(self.fresh(result))
	}
}
