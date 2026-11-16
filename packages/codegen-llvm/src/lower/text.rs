//! Strings and printing: literals, interpolation, concatenation, `length`,
//! `toString` through the runtime's text builder (values brief §2.8, §7).

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{TemplatePart, TemplateParts, View};
use inkwell::IntPredicate;
use inkwell::values::{BasicValue, InstructionOpcode, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// Decimal text borrows a private scratch buffer until the statement
	/// ends. `take` copies the view when a string must escape into an owner;
	/// immediate consumers (padding, parsing, interpolation) need no copy.
	/// In a coroutine, the rest of the statement may suspend (`await`): the
	/// view would then point into a stack that the resumption doesn't keep,
	/// so the text is an owned temporary instead.
	pub(crate) fn integer_string(&mut self, value: Val<'ctx>) -> Result<Option<Val<'ctx>>, Error> {
		let Repr::Int(int) = value.repr else {
			return Ok(None);
		};
		if int.bits > 64 {
			return Ok(None);
		}
		let raw = if int.signed {
			self.builder
				.build_int_s_extend_or_bit_cast(value.int(), self.context.i64_type(), "")?
		} else {
			self.builder
				.build_int_z_extend_or_bit_cast(value.int(), self.context.i64_type(), "")?
		};
		if self.in_coroutine() {
			let name = if int.signed {
				"fa_string_i64"
			} else {
				"fa_string_u64"
			};
			let text = self.call_with_pair(name, &[raw.into()], Repr::Str)?;
			return Ok(Some(self.fresh(text)));
		}
		let scratch = self.alloca_type(self.context.i8_type().array_type(24).into(), "digits")?;
		let name = if int.signed {
			"fa_decimal_i64"
		} else {
			"fa_decimal_u64"
		};
		let function = self.runtime.function(name)?;
		let offset = self
			.call_value(function, &[raw.into(), scratch.into()])?
			.into_int_value();
		let bytes = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), scratch, &[offset], "")?
		};
		let length = self
			.builder
			.build_int_sub(self.usize.const_int(24, false), offset, "")?;
		let pair = self
			.builder
			.build_insert_value(self.string_type.get_undef(), bytes, 0, "")?;
		let pair = self.builder.build_insert_value(pair, length, 1, "")?;
		let capacity = self
			.builder
			.build_or(self.simple_flag(), self.string_kind_mask(), "")?;
		let pair = self.builder.build_insert_value(pair, capacity, 2, "")?;
		// Not a fresh owning temporary: an escaping value must be copied.
		Ok(Some(Val::new(Repr::Str, pair.into_struct_value())))
	}

	/// The only part of a template, an integer of at most 64 bits.
	pub(crate) fn integer_part(&self, parts: TemplateParts<'_>) -> Option<NodeId> {
		let mut meaningful = parts
			.iter()
			.filter(|p| !matches!(p, TemplatePart::Text(text) if text.is_empty()));
		let Some(TemplatePart::Expression(node)) = meaningful.next() else {
			return None;
		};
		if meaningful.next().is_some() {
			return None;
		}
		match self.node_repr(node) {
			Ok(Repr::Int(int)) if int.bits <= 64 && !self.prints_flags(node) => Some(node),
			_ => None,
		}
	}

	fn single_integer_template(
		&mut self,
		parts: TemplateParts<'_>,
	) -> Result<Option<Val<'ctx>>, Error> {
		let mut meaningful = parts
			.iter()
			.filter(|p| !matches!(p, TemplatePart::Text(text) if text.is_empty()));
		if let Some(TemplatePart::Expression(node)) = meaningful.next()
			&& meaningful.next().is_none()
			&& matches!(self.node_repr(node)?, Repr::Int(int) if int.bits <= 64)
			&& !self.prints_flags(node)
		{
			let value = self.expr(node)?;
			if value.repr == Repr::Never {
				return Ok(Some(value));
			}
			return self.integer_string(value);
		}
		Ok(None)
	}

	/// Whether evaluating an expression runs no code that could change a
	/// place: no call, no assignment, only reads and operators.
	fn is_inert(&self, node: NodeId) -> bool {
		matches!(
			self.view(node),
			View::Integer(_)
				| View::Float(_)
				| View::String(_)
				| View::Boolean(_)
				| View::None | View::Name(_)
				| View::Group(_)
				| View::Access { .. }
				| View::Index { .. }
				| View::Arithmetic { .. }
				| View::Negate(_)
				| View::Equality { .. }
				| View::Comparison { .. }
				| View::Logic { .. }
				| View::Not(_)
		) && fa_analyzer::syntax::children(self.tree, node).all(|child| self.is_inert(child))
	}

	/// Common textual/scalar templates need no shared builder. Evaluate all
	/// expressions first, then borrow their values for one exact-sized join.
	pub(super) fn string_template(
		&mut self,
		parts: TemplateParts<'_>,
		destination: Option<Slot<'ctx>>,
		local: Option<super::frame_strings::FrameString<'ctx>>,
	) -> Result<Option<Val<'ctx>>, Error> {
		for part in parts.iter() {
			if let TemplatePart::Expression(node) = part {
				match self.node_repr(node)? {
					Repr::Str => {
						if let Some(destination) = destination
							&& local.is_none()
						{
							// Only independent owning locals can be read while the
							// destination's buffer is reused. References and calls
							// keep the ordinary allocating path.
							let Resolution::Local(symbol) = self.analysis.resolution(node) else {
								return Ok(None);
							};
							if !matches!(
								self.analysis.symbols.symbol(symbol).kind,
								fa_analyzer::symbols::SymbolKind::Let
									| fa_analyzer::symbols::SymbolKind::Mutable
							) || self.slot_of(symbol, node)?.pointer == destination.pointer
							{
								return Ok(None);
							}
						}
					}
					Repr::Int(int) if int.bits <= 64 && !self.prints_flags(node) => {}
					_ => return Ok(None),
				}
			}
		}
		// A borrowed string must outlive the evaluation of the later parts:
		// copy it when one of them may run code that changes its place.
		let last_effect = parts
			.iter()
			.enumerate()
			.filter(
				|(_, part)| matches!(part, TemplatePart::Expression(node) if !self.is_inert(*node)),
			)
			.map(|(index, _)| index)
			.last();
		let mut values = Vec::new();
		let mut simple = self.simple_flag();
		for (index, part) in parts.iter().enumerate() {
			let value = match part {
				TemplatePart::Text("") => continue,
				TemplatePart::Text(text) => self.string_constant(text),
				TemplatePart::Expression(node) => self.expr(node)?,
			};
			if value.repr == Repr::Never {
				return Ok(Some(value));
			}
			let value = if value.repr != Repr::Str {
				self.integer_string(value)?.expect("a checked integer")
			} else if last_effect.is_some_and(|last| index < last)
				&& matches!(part, TemplatePart::Expression(_))
				&& !self.is_temporary(value)
			{
				self.own(value, None)?
			} else {
				value
			};
			let (bytes, length) = self.string_parts(value)?;
			let empty = self.builder.build_int_compare(
				IntPredicate::EQ,
				length,
				self.usize.const_zero(),
				"",
			)?;
			let flag = self
				.builder
				.build_select(empty, self.simple_flag(), self.string_capacity(value)?, "")?
				.into_int_value();
			simple = self.builder.build_and(simple, flag, "")?;
			values.push((bytes, length));
		}
		let mut total = self.usize.const_zero();
		for &(_, length) in &values {
			total = self
				.call_value(
					self.runtime.function("fa_string_size")?,
					&[total.into(), length.into()],
				)?
				.into_int_value();
		}
		// Keep each byte range in SSA rather than in an addressable array.
		// Writes to the result cannot obscure literal lengths or flags.
		let result = match destination {
			Some(slot) => slot,
			None => self.alloca(Repr::Str, "template")?,
		};
		let bytes = if let Some(local) = local {
			self.call_value(
				self.runtime.function("fa_string_allocate_local")?,
				&[
					total.into(),
					simple.into(),
					local.buffer.into(),
					result.pointer.into(),
					self.context
						.i32_type()
						.const_int(local.mutable as u64, false)
						.into(),
				],
			)?
		} else {
			let allocate = if destination.is_some() {
				"fa_string_replace_buffer"
			} else {
				"fa_string_allocate"
			};
			self.call_value(
				self.runtime.function(allocate)?,
				&[total.into(), simple.into(), result.pointer.into()],
			)?
		}
		.into_pointer_value();
		let mut offset = self.usize.const_zero();
		for (part, length) in values {
			let target = unsafe {
				self.builder
					.build_in_bounds_gep(self.context.i8_type(), bytes, &[offset], "")?
			};
			self.call_runtime(
				"fa_copy_bytes",
				&[target.into(), part.into(), length.into()],
			)?;
			offset = self.builder.build_int_add(offset, length, "")?;
		}
		self.call_runtime(
			"fa_string_finish_known",
			&[result.pointer.into(), simple.into()],
		)?;
		if destination.is_some() {
			return Ok(Some(Val::void()));
		}
		let result = self.load(result)?;
		Ok(Some(self.fresh(result)))
	}

	pub(crate) fn string_unit_node(&self, mut node: NodeId) -> Option<NodeId> {
		while let View::Group(inner) | View::Try(inner) = self.view(node) {
			node = inner;
		}
		(self
			.flags(node)
			.contains(fa_analyzer::analysis::NodeFlags::STRING_CODE_UNIT)
			&& matches!(self.view(node), View::Index { .. }))
		.then_some(node)
	}

	pub(crate) fn string_unit_index(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let View::Index { target, index } = self.view(node) else {
			return Err(self.unsupported(node.index(), "this code unit index"));
		};
		let text = self.expr(target)?;
		let position = self.expr(index)?;
		if text.repr == Repr::Never || position.repr == Repr::Never {
			return Ok(Val::never());
		}
		self.string_unit_at(node, text, position)
	}

	pub(crate) fn string_unit_at(
		&mut self,
		node: NodeId,
		text: Val<'ctx>,
		position: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let position = self
			.convert(position, Repr::Int(IntRepr::I64), node.index())?
			.int();
		let (bytes, length) = self.string_parts(text)?;
		let capacity = self.string_capacity(text)?;
		let unit = self
			.call_value(
				self.runtime.function("fa_string_code_unit_at")?,
				&[
					bytes.into(),
					length.into(),
					capacity.into(),
					position.into(),
				],
			)?
			.into_int_value();
		if self
			.flags(node)
			.contains(fa_analyzer::analysis::NodeFlags::INDEX_CHECK)
		{
			let missing = self.builder.build_int_compare(
				IntPredicate::SLT,
				unit,
				unit.get_type().const_zero(),
				"",
			)?;
			self.guard(missing, |this| {
				let count = this
					.call_value(
						this.runtime.function("fa_string_length")?,
						&[bytes.into(), length.into(), capacity.into()],
					)?
					.into_int_value();
				let count = this.builder.build_int_z_extend_or_bit_cast(
					count,
					this.context.i64_type(),
					"",
				)?;
				this.raise_runtime("fa_error_index", &[position.into(), count.into()])
			})?;
		}
		let unit = self
			.builder
			.build_int_truncate(unit, self.context.i16_type(), "")?;
		Ok(Val::new(
			Repr::Int(IntRepr {
				bits: 16,
				signed: false,
			}),
			unit,
		))
	}

	/// Inline strings have zero padding after their bytes. Two such values
	/// compare in three words; the optional SIMPLE optimization flag is ignored.
	pub(crate) fn strings_equal(
		&mut self,
		left: Val<'ctx>,
		right: Val<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let a = left.value.expect("a string").into_struct_value();
		let b = right.value.expect("a string").into_struct_value();
		let ac = self.string_capacity(left)?;
		let bc = self.string_capacity(right)?;
		let both = self.builder.build_and(ac, bc, "")?;
		let flag = self
			.builder
			.build_and(both, self.string_inline_flag(), "")?;
		let inline =
			self.builder
				.build_int_compare(IntPredicate::NE, flag, self.usize.const_zero(), "")?;
		let small = self.append_block("inline_equal");
		let external = self.append_block("external_equal");
		let end = self.append_block("string_equal");
		self.builder
			.build_conditional_branch(inline, small, external)?;
		self.builder.position_at_end(small);
		let mut same = self.context.bool_type().const_int(1, false);
		for index in 0..3 {
			let av = self.builder.build_extract_value(a, index, "")?;
			let bv = self.builder.build_extract_value(b, index, "")?;
			let (av, bv) = if index == 0 {
				(
					self.builder
						.build_ptr_to_int(av.into_pointer_value(), self.usize, "")?,
					self.builder
						.build_ptr_to_int(bv.into_pointer_value(), self.usize, "")?,
				)
			} else if index == 2 {
				let mask = self.simple_flag().const_not();
				(
					self.builder.build_and(av.into_int_value(), mask, "")?,
					self.builder.build_and(bv.into_int_value(), mask, "")?,
				)
			} else {
				(av.into_int_value(), bv.into_int_value())
			};
			let equal = self
				.builder
				.build_int_compare(IntPredicate::EQ, av, bv, "")?;
			same = self.builder.build_and(same, equal, "")?;
		}
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(external);
		let (ap, al) = self.string_parts(left)?;
		let (bp, bl) = self.string_parts(right)?;
		let equal = self
			.call_value(
				self.runtime.function("fa_string_equal")?,
				&[ap.into(), al.into(), bp.into(), bl.into()],
			)?
			.into_int_value();
		let equal = self.builder.build_int_compare(
			IntPredicate::NE,
			equal,
			self.context.i32_type().const_zero(),
			"",
		)?;
		let external_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let result = self.builder.build_phi(self.context.bool_type(), "equal")?;
		result.add_incoming(&[(&same, small), (&equal, external_end)]);
		Ok(result.as_basic_value().into_int_value())
	}

	/// Append without exposing the destination header to the slow runtime
	/// call. The ordinary path updates SSA values; only growth spills a
	/// snapshot, allowing LLVM to promote a local header across the loop.
	pub(crate) fn append_string(
		&mut self,
		target: PointerValue<'ctx>,
		suffix: Val<'ctx>,
	) -> Result<(), Error> {
		let slot = Slot {
			pointer: target,
			repr: Repr::Str,
		};
		let current = self.load(slot)?;
		// The heap path can use the raw fields. Decoding the inline length
		// here would add shifts/selects to every byte of a large append loop.
		let pair = current.value.expect("a string").into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(pair, 1, "")?
			.into_int_value();
		let capacity = self.string_capacity(current)?;
		let (suffix_bytes, suffix_length) = self.string_parts(suffix)?;
		let suffix_capacity = self.string_capacity(suffix)?;
		let needed = self.builder.build_int_add(length, suffix_length, "")?;
		let simple = self.builder.build_int_compare(
			IntPredicate::SLT,
			suffix_capacity,
			self.usize.const_zero(),
			"",
		)?;
		let room = self.owned_room(capacity, needed)?;
		let fast = self.builder.build_and(simple, room, "")?;
		let append = self.append_block("append");
		let grow = self.append_block("append_slow");
		let join = self.append_block("appended");
		let branch = self.builder.build_conditional_branch(fast, append, grow)?;
		self.likely(branch)?;
		self.builder.position_at_end(append);
		let destination = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), bytes, &[length], "")?
		};
		if suffix_length.get_zero_extended_constant().is_some() {
			self.builder
				.build_memcpy(destination, 1, suffix_bytes, 1, suffix_length)?;
		} else {
			self.call_runtime(
				"fa_copy_bytes",
				&[
					destination.into(),
					suffix_bytes.into(),
					suffix_length.into(),
				],
			)?;
		}
		// only the length changes: the bytes and the capacity stay
		let length_pointer = self
			.builder
			.build_struct_gep(self.string_type, target, 1, "")?;
		self.builder.build_store(length_pointer, needed)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(grow);
		// a string in memory anyway (a field, an item: not a local the
		// optimizer keeps in registers): the runtime appends to it in place,
		// one call, so that the caller keeps few values alive around it
		if target.as_instruction().is_none_or(|instruction| {
			instruction.get_opcode() != inkwell::values::InstructionOpcode::Alloca
		}) {
			self.call_runtime(
				"fa_string_append_cold",
				&[
					target.into(),
					suffix_bytes.into(),
					suffix_length.into(),
					suffix_capacity.into(),
				],
			)?;
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(join);
			return Ok(());
		}
		// one simple byte into an inline string with room (a character
		// appended to a short string): the byte is ORed into its word of the
		// value (the unused bytes are zero), the length in the tag byte grows,
		// all in registers
		let word_bits = self.usize.get_bit_width() as u64;
		let word_bytes = word_bits / 8;
		let length_shift = self.usize.const_int(word_bits - 8, false);
		let inline = self
			.builder
			.build_and(capacity, self.string_inline_flag(), "")?;
		let inline = self.builder.build_int_compare(
			IntPredicate::NE,
			inline,
			self.usize.const_zero(),
			"",
		)?;
		let tag_length = self
			.builder
			.build_right_shift(capacity, length_shift, false, "")?;
		let tag_length = self
			.builder
			.build_and(tag_length, self.usize.const_int(31, false), "")?;
		let room = self.builder.build_int_compare(
			IntPredicate::ULT,
			tag_length,
			self.usize.const_int(3 * word_bytes - 1, false),
			"",
		)?;
		let one_byte = self.builder.build_int_compare(
			IntPredicate::EQ,
			suffix_length,
			self.usize.const_int(1, false),
			"",
		)?;
		let small = self.builder.build_and(inline, room, "")?;
		let small = self.builder.build_and(small, one_byte, "")?;
		let small = self.builder.build_and(small, simple, "")?;
		let inline_append = self.append_block("append_inline");
		let snapshot_append = self.append_block("append_snapshot");
		self.builder
			.build_conditional_branch(small, inline_append, snapshot_append)?;
		self.builder.position_at_end(inline_append);
		let byte = self
			.builder
			.build_load(self.context.i8_type(), suffix_bytes, "")?
			.into_int_value();
		let byte = self.builder.build_int_z_extend(byte, self.usize, "")?;
		let word_index = self.builder.build_right_shift(
			tag_length,
			self.usize
				.const_int(word_bytes.trailing_zeros() as u64, false),
			false,
			"",
		)?;
		let offset =
			self.builder
				.build_and(tag_length, self.usize.const_int(word_bytes - 1, false), "")?;
		let offset = self
			.builder
			.build_left_shift(offset, self.usize.const_int(3, false), "")?;
		let piece = self.builder.build_left_shift(byte, offset, "")?;
		let first = self.builder.build_ptr_to_int(bytes, self.usize, "")?;
		let mut words = [first, length, capacity];
		for (index, word) in words.iter_mut().enumerate() {
			let here = self.builder.build_int_compare(
				IntPredicate::EQ,
				word_index,
				self.usize.const_int(index as u64, false),
				"",
			)?;
			let part = self
				.builder
				.build_select(here, piece, self.usize.const_zero(), "")?
				.into_int_value();
			*word = self.builder.build_or(*word, part, "")?;
		}
		let longer = self.builder.build_int_add(
			words[2],
			self.usize.const_int(1 << (word_bits - 8), false),
			"",
		)?;
		let first = self
			.builder
			.build_int_to_ptr(words[0], self.pointer_type(), "")?;
		let updated = self.string_type.get_undef();
		let updated = self.builder.build_insert_value(updated, first, 0, "")?;
		let updated = self.builder.build_insert_value(updated, words[1], 1, "")?;
		let updated = self.builder.build_insert_value(updated, longer, 2, "")?;
		self.builder.build_store(target, updated)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(snapshot_append);
		// one slot per function: each use is a store, a call and a load
		let function = self.frame().function;
		let existing = self.append_headers.borrow().get(&function).copied();
		let snapshot = match existing {
			Some(pointer) => Slot {
				pointer,
				repr: Repr::Str,
			},
			None => {
				let slot = self.alloca(Repr::Str, "append_header")?;
				self.append_headers
					.borrow_mut()
					.insert(function, slot.pointer);
				slot
			}
		};
		self.builder
			.build_store(snapshot.pointer, current.value.expect("a string"))?;
		self.call_runtime(
			"fa_string_append",
			&[
				snapshot.pointer.into(),
				suffix_bytes.into(),
				suffix_length.into(),
				suffix_capacity.into(),
			],
		)?;
		let updated = self.load(snapshot)?;
		self.builder
			.build_store(target, updated.value.expect("a string"))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// The length and `SIMPLE` flag shared by every string a node may
	/// evaluate to, when each is a literal: a literal, or an `if` or a `when`
	/// whose every branch is one (`when op is { 0 => " + ", … }`). Literals
	/// are static strings (`string_constant`), passed unchanged through the
	/// branches, so the appended value's length and capacity are constants.
	pub(crate) fn literal_shape(&self, node: NodeId) -> Option<(u64, bool)> {
		use fa_analyzer::syntax::View;
		let shape = |text: &str| (text.len() as u64, text.is_ascii());
		match self.view(node) {
			View::String(text) => Some(shape(text)),
			View::Group(inner) => self.literal_shape(inner),
			View::Block { statements } if statements.len() == 1 => {
				self.literal_shape(statements.first().expect("one statement"))
			}
			View::If {
				then_body,
				else_body: Some(else_body),
				compile_time: false,
				..
			} => {
				let then_shape = self.literal_shape(then_body)?;
				(self.literal_shape(else_body)? == then_shape).then_some(then_shape)
			}
			View::When { arms, .. } => {
				let mut common = None;
				for arm in arms.iter() {
					let arm = self.literal_shape(arm.value)?;
					if common.is_some_and(|common| common != arm) {
						return None;
					}
					common = Some(arm);
				}
				common
			}
			_ => None,
		}
	}

	/// A string value whose length and `SIMPLE` flag are known statically
	/// ([`Lowering::literal_shape`]): its bytes pointer with the constants.
	pub(crate) fn with_literal_shape(
		&self,
		value: Val<'ctx>,
		(length, simple): (u64, bool),
	) -> Result<Val<'ctx>, Error> {
		let pair = value.value.expect("a string").into_struct_value();
		let bytes = self.builder.build_extract_value(pair, 0, "")?;
		let capacity = if simple {
			self.simple_flag()
		} else {
			self.string_capacity(value)?
		};
		let updated =
			self.builder
				.build_insert_value(self.string_type.get_undef(), bytes, 0, "")?;
		let updated =
			self.builder
				.build_insert_value(updated, self.usize.const_int(length, false), 1, "")?;
		let updated = self.builder.build_insert_value(updated, capacity, 2, "")?;
		Ok(Val {
			value: Some(updated.into_struct_value().into()),
			..value
		})
	}

	/// Whether a string has writable room for `needed` bytes. Removing its
	/// SIMPLE and frame bits leaves a heap capacity or the frame's 64 bytes;
	/// inline values retain their INLINE bit, and views normalize to the
	/// excluded limit. Only the length changes on the fast append path;
	/// the original capacity word retains the frame tag.
	pub(crate) fn owned_room(
		&self,
		capacity: IntValue<'ctx>,
		needed: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let kind = self.builder.build_and(
			capacity,
			self.string_flags()
				.const_add(self.usize.const_int(1, false))
				.const_not(),
			"",
		)?;
		let room = self
			.builder
			.build_int_compare(IntPredicate::ULE, needed, kind, "")?;
		let owned = self.builder.build_int_compare(
			IntPredicate::ULT,
			kind,
			self.string_kind_mask()
				.const_sub(self.usize.const_int(1, false)),
			"",
		)?;
		Ok(self.builder.build_and(room, owned, "")?)
	}

	/// The most digits of an integer that [`Lowering::append_integer`]
	/// writes in place.
	const IN_PLACE_DIGITS: u32 = 5;

	/// `text += "{value}"` for an integer of at most 64 bits: a value below
	/// 10^5 is written in place, one case per number of digits, after one
	/// room check, without a scratch buffer, a copy or a call (a value whose
	/// range LLVM knows keeps only its cases); a larger or negative value,
	/// or a string without room, takes the decimal text and `append_string`.
	pub(crate) fn append_integer(
		&mut self,
		target: PointerValue<'ctx>,
		value: Val<'ctx>,
	) -> Result<(), Error> {
		let Repr::Int(int) = value.repr else {
			return Err(Error::Llvm("an integer is appended".into()));
		};
		let usize = self.usize;
		let current = self.load(Slot {
			pointer: target,
			repr: Repr::Str,
		})?;
		let pair = current.value.expect("a string").into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(pair, 1, "")?
			.into_int_value();
		let capacity = self.string_capacity(current)?;
		let digits = Self::IN_PLACE_DIGITS;
		let needed =
			self.builder
				.build_int_add(length, usize.const_int(digits as u64, false), "")?;
		let room = self.owned_room(capacity, needed)?;
		// a negative value is a large unsigned one
		let wide = if int.signed {
			self.builder
				.build_int_s_extend_or_bit_cast(value.int(), self.context.i64_type(), "")?
		} else {
			self.builder
				.build_int_z_extend_or_bit_cast(value.int(), self.context.i64_type(), "")?
		};
		let i64 = self.context.i64_type();
		let small = self.builder.build_int_compare(
			IntPredicate::ULT,
			wide,
			i64.const_int(10u64.pow(digits), false),
			"",
		)?;
		let fast = self.builder.build_and(room, small, "")?;
		let write = self.append_block("digits_in_place");
		let slow = self.append_block("digits_slow");
		let join = self.append_block("digits_appended");
		let branch = self.builder.build_conditional_branch(fast, write, slow)?;
		self.likely(branch)?;
		self.builder.position_at_end(write);
		let destination = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), bytes, &[length], "")?
		};
		let length_pointer = self
			.builder
			.build_struct_gep(self.string_type, target, 1, "")?;
		for count in 1..=digits {
			let next = if count < digits {
				let case = self.append_block("digits_case");
				let next = self.append_block("digits_more");
				let fits = self.builder.build_int_compare(
					IntPredicate::ULT,
					wide,
					i64.const_int(10u64.pow(count), false),
					"",
				)?;
				self.builder.build_conditional_branch(fits, case, next)?;
				self.builder.position_at_end(case);
				Some(next)
			} else {
				None
			};
			self.write_small_decimal(destination, wide, count)?;
			let longer =
				self.builder
					.build_int_nuw_add(length, usize.const_int(count as u64, false), "")?;
			self.builder.build_store(length_pointer, longer)?;
			self.builder.build_unconditional_branch(join)?;
			if let Some(next) = next {
				self.builder.position_at_end(next);
			}
		}
		self.builder.position_at_end(slow);
		// one digit: a static one-byte string, which `append_string` adds to
		// an inline string in registers
		let digit = self.append_block("digit_alone");
		let decimal = self.append_block("digits_decimal");
		let alone = self.builder.build_int_compare(
			IntPredicate::ULT,
			wide,
			i64.const_int(10, false),
			"",
		)?;
		self.builder
			.build_conditional_branch(alone, digit, decimal)?;
		self.builder.position_at_end(digit);
		let digits = self.constant_bytes(b"0123456789");
		let offset = self
			.builder
			.build_int_truncate_or_bit_cast(wide, usize, "")?;
		let pointer = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), digits, &[offset], "")?
		};
		let one = self.string_type.const_named_struct(&[
			self.pointer_type().const_null().into(),
			usize.const_int(1, false).into(),
			self.simple_flag().into(),
		]);
		let one = self.builder.build_insert_value(one, pointer, 0, "")?;
		self.append_string(target, Val::new(Repr::Str, one.into_struct_value()))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(decimal);
		// A coroutine decimal owns a temporary that exists only in this
		// branch. Drop it here, before the other append paths join.
		let mark = self.mark();
		let text = self
			.integer_string(value)?
			.expect("an integer of at most 64 bits");
		self.append_string(target, text)?;
		self.end_statement(mark, None)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// The bytes and the length of a string value.
	pub(crate) fn string_parts(
		&self,
		value: Val<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		self.string_parts_with(&self.builder, value)
	}

	/// Decode a String in either representation. An immutable parameter may
	/// borrow its original place; other inline bytes use an SSA snapshot so an
	/// intervening write cannot change them. Entry allocas bound loop storage.
	pub(crate) fn string_parts_with(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		value: Val<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		// a field read once at the entry of the method (`frozen.rs`)
		if let Some(parts) = self.frozen_parts(value) {
			return Ok(parts);
		}
		let pair = value.value.expect("a string").into_struct_value();
		let bytes = builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let length = builder.build_extract_value(pair, 1, "")?.into_int_value();
		let capacity = builder.build_extract_value(pair, 2, "")?.into_int_value();
		let flag = builder.build_and(capacity, self.string_inline_flag(), "")?;
		let inline =
			builder.build_int_compare(IntPredicate::NE, flag, self.usize.const_zero(), "")?;
		let function = builder
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		// A readonly parameter is an immutable reference for the entire call.
		// Its loaded value can borrow inline bytes from that original place;
		// mutable places and computed values still require an SSA snapshot.
		let borrowed = pair.as_instruction_value().and_then(|instruction| {
			if instruction.get_opcode() != InstructionOpcode::Load {
				return None;
			}
			let pointer = instruction.get_operand(0)?.value()?.into_pointer_value();
			self.frame
				.as_ref()
				.is_some_and(|frame| {
					frame.function == function && frame.readonly_strings.contains(&pointer)
				})
				.then_some(pointer)
		});
		let bytes = match borrowed {
			Some(pointer) => builder
				.build_select(inline, pointer, bytes, "")?
				.into_pointer_value(),
			None => {
				let entry = function.get_first_basic_block().expect("an entry");
				let allocator = self.context.create_builder();
				if let Some(first) = entry.get_first_instruction() {
					allocator.position_before(&first);
				} else {
					allocator.position_at_end(entry);
				}
				let snapshot = allocator.build_alloca(self.string_type, "inline_string")?;
				// only an inline string is stored (a view or a heap string, like the
				// parts of a split, needs no snapshot)
				let before = builder.get_insert_block().expect("a block");
				let store = self.context.append_basic_block(function, "inline_snapshot");
				let join = self.context.append_basic_block(function, "string_bytes");
				builder.build_conditional_branch(inline, store, join)?;
				builder.position_at_end(store);
				builder.build_store(snapshot, pair)?;
				builder.build_unconditional_branch(join)?;
				builder.position_at_end(join);
				let phi = builder.build_phi(self.pointer_type(), "bytes")?;
				phi.add_incoming(&[(&snapshot, store), (&bytes, before)]);
				phi.as_basic_value().into_pointer_value()
			}
		};
		let shifted = builder.build_right_shift(
			capacity,
			self.usize
				.const_int((self.usize.get_bit_width() - 8) as u64, false),
			false,
			"",
		)?;
		let inline_length = builder.build_and(shifted, self.usize.const_int(31, false), "")?;
		let length = builder
			.build_select(inline, inline_length, length, "")?
			.into_int_value();
		// Only the decoded length is nonnegative: an inline raw word holds
		// UTF-8 bytes and may have its sign bit set.
		if length.get_zero_extended_constant().is_none() {
			let positive = builder.build_int_compare(
				IntPredicate::SGE,
				length,
				self.usize.const_zero(),
				"",
			)?;
			let assume = inkwell::intrinsics::Intrinsic::find("llvm.assume")
				.and_then(|intrinsic| intrinsic.get_declaration(self.module, &[]))
				.expect("llvm.assume");
			builder.build_call(assume, &[positive.into()], "")?;
		}
		Ok((bytes, length))
	}

	/// The capacity word of a string value (its top bit: `SIMPLE`, one byte
	/// per character; see `runtime/fa_runtime.c`).
	pub(crate) fn string_capacity(&self, value: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let pair = value.value.expect("a string").into_struct_value();
		Ok(self
			.builder
			.build_extract_value(pair, 2, "")?
			.into_int_value())
	}

	/// Calls a runtime function that writes a `{ ptr, usize }` to its last
	/// argument, and loads it.
	pub(crate) fn call_with_pair(
		&mut self,
		name: &str,
		arguments: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
		repr: Repr,
	) -> Result<Val<'ctx>, Error> {
		let out = self.alloca(repr, "result")?;
		let mut all = arguments.to_vec();
		all.push(out.pointer.into());
		self.call_runtime(name, &all)?;
		self.load(out)
	}

	/// `a + b` on strings.
	pub(crate) fn concat(&mut self, left: Val<'ctx>, right: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let (a, a_length) = self.string_parts(left)?;
		let (b, b_length) = self.string_parts(right)?;
		let result = self.call_with_pair(
			"fa_concat",
			&[
				a.into(),
				a_length.into(),
				self.string_capacity(left)?.into(),
				b.into(),
				b_length.into(),
				self.string_capacity(right)?.into(),
			],
			Repr::Str,
		)?;
		Ok(self.fresh(result))
	}

	/// `left + right` where the caller owns `left`: `right` appended to it in
	/// place when its block has room (`fa_string_plus`), the new value.
	pub(crate) fn append_in_place(
		&mut self,
		left: Val<'ctx>,
		right: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let slot = self.alloca(Repr::Str, "concatenation")?;
		self.builder
			.build_store(slot.pointer, left.value.expect("a string"))?;
		let (bytes, length) = self.string_parts(right)?;
		let capacity = self.string_capacity(right)?;
		self.call_runtime(
			"fa_string_plus",
			&[
				slot.pointer.into(),
				bytes.into(),
				length.into(),
				capacity.into(),
			],
		)?;
		self.load(slot)
	}

	/// Appends the `toString` of a value to the runtime's text builder.
	pub(crate) fn append_value(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		match value.repr {
			Repr::Str => {
				let (bytes, length) = self.string_parts(value)?;
				// its `SIMPLE` flag spares the builder a scan of its bytes
				let capacity = self.string_capacity(value)?;
				self.call_runtime(
					"fa_b_string",
					&[bytes.into(), length.into(), capacity.into()],
				)
			}
			// a set of flags: `{ read, write }` (fields.rs)
			Repr::Int(_)
				if let Some(ty) = self.printed_fields_type(node)
					&& self.fields_repr(ty) == value.repr =>
			{
				self.append_fields(value, ty, node)
			}
			Repr::Int(int) => {
				let raw = value.int();
				if int.bits <= 64 {
					let i64 = self.context.i64_type();
					let (name, wide) = match int.signed {
						true if int.bits < 64 => {
							("fa_b_i64", self.builder.build_int_s_extend(raw, i64, "")?)
						}
						true => ("fa_b_i64", raw),
						false if int.bits < 64 => {
							("fa_b_u64", self.builder.build_int_z_extend(raw, i64, "")?)
						}
						false => ("fa_b_u64", raw),
					};
					self.call_runtime(name, &[wide.into()])
				} else {
					let i64 = self.context.i64_type();
					let low = self.builder.build_int_truncate(raw, i64, "")?;
					let shifted = self.builder.build_right_shift(
						raw,
						raw.get_type().const_int(64, false),
						false,
						"",
					)?;
					let high = self.builder.build_int_truncate(shifted, i64, "")?;
					let name = if int.signed { "fa_b_i128" } else { "fa_b_u128" };
					self.call_runtime(name, &[low.into(), high.into()])
				}
			}
			Repr::Bool => {
				let wide =
					self.builder
						.build_int_z_extend(value.int(), self.context.i32_type(), "")?;
				self.call_runtime("fa_b_bool", &[wide.into()])
			}
			Repr::BigInt => self.call_runtime("fa_b_bigint", &[value.int().into()]),
			Repr::F64 => self.call_runtime("fa_b_f64", &[value.value.expect("a float").into()]),
			Repr::F32 => self.call_runtime("fa_b_f32", &[value.value.expect("a float").into()]),
			Repr::Percentage => {
				self.call_runtime("fa_b_percentage", &[value.value.expect("a float").into()])
			}
			Repr::Enum(nominal) => {
				let name = self.enum_name(value, nominal)?;
				self.append_value(name, node)
			}
			Repr::Union | Repr::InlineUnion(_) => {
				let value = self.union_view(value)?;
				let nested = self.context.i32_type().const_zero();
				self.call_runtime(
					"fa_format_union",
					&[value.value.expect("a box").into(), nested.into()],
				)
			}
			Repr::Object(ty) => self.append_object(value, ty, node),
			Repr::Array(_) | Repr::Map(_) | Repr::Set(_) | Repr::Tuple(_) => {
				self.append_aggregate(value, node)
			}
			Repr::Growing(_) => {
				let mark = self.mark();
				let view = self.growing_view(value)?;
				self.append_aggregate(view, node)?;
				self.end_scope(mark, None)
			}
			Repr::Pool(_) | Repr::Columnar(_) => {
				let mark = self.mark();
				let view = self.pool_view(value, node)?;
				self.append_aggregate(view, node)?;
				self.end_scope(mark, None)
			}
			Repr::Row(_) | Repr::Watch | Repr::Task(_) => {
				Err(self.unsupported(node.index(), "printing this value"))
			}
			Repr::Link(_) | Repr::Links(_) => {
				Err(self.unsupported(node.index(), "printing a relation side"))
			}
			Repr::None => {
				let none = self.string_constant("none");
				self.append_value(none, node)
			}
			Repr::Optional(_) => self.append_optional(value, node, false),
			Repr::Function(_) => Err(self.unsupported(node.index(), "printing a function")),
			Repr::Pointer => Err(self.unsupported(node.index(), "printing a pointer")),
			Repr::Sync(_) => Err(self.unsupported(node.index(), "printing a thread-safe value")),
			Repr::Range => self.append_range(value, node),
			Repr::Rec(target) => {
				let unfolded = self.unfold_rec(value, target, node.index())?;
				self.append_value(unfolded, node)
			}
			Repr::Void | Repr::Never => {
				Err(self.unsupported(node.index(), "printing a value of type `Void`"))
			}
		}
	}

	/// Declare a composite formatter before lowering its fields/items, so
	/// recursive values produce recursive calls instead of infinite IR
	/// expansion. The borrowed value owns no additional storage here.
	pub(super) fn append_aggregate(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		let key = format!("format {:?} {}", self.file, self.shape_key(value.repr));
		let cached = self.drop_functions.borrow().get(&key).copied();
		let function = if let Some(function) = cached {
			function
		} else {
			let signature = self
				.context
				.void_type()
				.fn_type(&[self.llvm_type(value.repr).into()], false);
			let function = self.module.add_function(
				"fa.format.value",
				signature,
				Some(inkwell::module::Linkage::Internal),
			);
			self.runtime.add_attributes(function, &["nounwind"]);
			self.drop_functions.borrow_mut().insert(key, function);
			let saved_frame = self.frame.take();
			let saved_block = self.builder.get_insert_block();
			self.begin_function(function, Repr::Void, None);
			let argument = Val::new(value.repr, function.get_nth_param(0).expect("a value"));
			match value.repr {
				Repr::Object(ty) => self.append_object_fields(argument, ty, node)?,
				Repr::Tuple(ty) => self.append_tuple(argument, ty, node)?,
				Repr::Array(item) => self.append_array(argument, item, node)?,
				Repr::Map(_) | Repr::Set(_) => self.append_table(argument, node)?,
				_ => unreachable!("a composite formatter"),
			}
			self.end_scope(0, None)?;
			self.builder.build_return(None)?;
			self.end_function()?;
			self.frame = saved_frame;
			if let Some(block) = saved_block {
				self.builder.position_at_end(block);
			}
			function
		};
		self.builder
			.build_call(function, &[value.value.expect("a value").into()], "")?;
		Ok(())
	}

	/// The name of a member of an enumeration (`red`).
	pub(crate) fn enum_name(
		&mut self,
		value: Val<'ctx>,
		nominal: fa_analyzer::ids::NominalId,
	) -> Result<Val<'ctx>, Error> {
		// `red`: a table of the names of the members, by index
		let types = self.types();
		let info = types.nominal(nominal);
		let names: Vec<String> = types
			.sym_list(info.members)
			.iter()
			.map(|&member| types.syms.resolve(member).to_string())
			.collect();
		let entries: Vec<_> = names
			.iter()
			.map(|name| {
				let pointer = self.constant_bytes(name.as_bytes());
				let length = self.usize.const_int(name.len() as u64, false);
				self.string_type.const_named_struct(&[
					pointer.into(),
					length.into(),
					self.static_capacity(name).into(),
				])
			})
			.collect();
		let table_type = self.string_type.array_type(entries.len() as u32);
		let table = self.module.add_global(table_type, None, "fa.enum_names");
		table.set_initializer(&self.string_type.const_array(&entries));
		table.set_constant(true);
		table.set_linkage(inkwell::module::Linkage::Private);
		let index = value.int();
		let zero = self.context.i32_type().const_zero();
		let entry = unsafe {
			self.builder.build_in_bounds_gep(
				table_type,
				table.as_pointer_value(),
				&[zero, index],
				"",
			)?
		};
		let name = self.builder.build_load(self.string_type, entry, "")?;
		Ok(Val::new(Repr::Str, name))
	}

	/// An optional: `none`, or its value (quoted when `nested`).
	pub(crate) fn append_optional(
		&mut self,
		value: Val<'ctx>,
		node: NodeId,
		nested: bool,
	) -> Result<(), Error> {
		let Repr::Optional(_) = value.repr else {
			return self.append_value(value, node);
		};
		let (present, inner) = self.split_optional(&self.builder, value)?;
		let some = self.append_block("print_some");
		let none = self.append_block("print_none");
		let join = self.append_block("printed");
		self.builder.build_conditional_branch(present, some, none)?;
		self.builder.position_at_end(some);
		if nested {
			self.append_nested(inner, node)?;
		} else {
			self.append_value(inner, node)?;
		}
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(none);
		let text = self.string_constant("none");
		self.append_value(text, node)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// `"text {value} text"`: every part is evaluated first, then the text is built.
	pub(crate) fn template(&mut self, parts: TemplateParts<'_>) -> Result<Val<'ctx>, Error> {
		if let Some(value) = self.single_integer_template(parts)? {
			return Ok(value);
		}
		if let Some(value) = self.string_template(parts, None, None)? {
			return Ok(value);
		}
		if !self.build_template(parts)? {
			return Ok(Val::never());
		}
		let result = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
		Ok(self.fresh(result))
	}

	/// `text += "… {value} …"`: the template is built in the runtime's text
	/// builder and appended from there, without a string of its own.
	pub(crate) fn append_template(
		&mut self,
		target: PointerValue<'ctx>,
		place: NodeId,
		parts: TemplateParts<'_>,
	) -> Result<Val<'ctx>, Error> {
		if let Some(node) = self.integer_part(parts) {
			let value = self.expr(node)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			self.append_integer(target, value)?;
			return Ok(Val::void());
		}
		// Strings and integers that don't read the destination are evaluated
		// first (the template's snapshot), then appended in place after one
		// reservation for them all: no global builder, rescan or copy.
		let mut independent = true;
		for part in parts.iter() {
			if let TemplatePart::Expression(node) = part {
				let textual = match self.node_repr(node)? {
					Repr::Str => true,
					Repr::Int(int) => int.bits <= 64 && !self.prints_flags(node),
					_ => false,
				};
				if !textual || self.reads_root_of(node, place) {
					independent = false;
					break;
				}
			}
		}
		// a part whose function builds its result in the destination
		// (`destination.rs`): the parts are appended one after the other
		if independent
			&& parts.iter().any(|part| {
				matches!(part, TemplatePart::Expression(node)
					if self.destination_call(node, place).is_some())
			}) {
			for part in parts.iter() {
				let appended = match part {
					TemplatePart::Text("") => continue,
					TemplatePart::Text(text) => {
						let text = self.string_constant(text);
						self.append_string(target, text)?;
						continue;
					}
					TemplatePart::Expression(node) => self.append_part(target, place, node)?,
				};
				if appended.repr == Repr::Never {
					return Ok(appended);
				}
			}
			return Ok(Val::void());
		}
		if independent {
			let mut values = Vec::new();
			for part in parts.iter() {
				let value = match part {
					TemplatePart::Text("") => continue,
					TemplatePart::Text(text) => self.string_constant(text),
					TemplatePart::Expression(node) => {
						let value = self.expr(node)?;
						if value.repr == Repr::Never {
							return Ok(value);
						}
						if value.repr == Repr::Str {
							value
						} else {
							self.integer_string(value)?.expect("a checked integer")
						}
					}
				};
				values.push(value);
			}
			let mut extra = self.usize.const_zero();
			for value in &values {
				let (_, length) = self.string_parts(*value)?;
				extra = self
					.call_value(
						self.runtime.function("fa_string_size")?,
						&[extra.into(), length.into()],
					)?
					.into_int_value();
			}
			self.call_runtime("fa_string_reserve", &[target.into(), extra.into()])?;
			for value in values {
				self.append_string(target, value)?;
			}
			return Ok(Val::void());
		}
		// A template of disjoint string locals and literals can be appended
		// directly. No global builder, rescan for ASCII, or intermediate copy.
		// Restrict this to owning bindings: `use` aliases and a read of the
		// destination itself must retain the snapshot semantics of templates.
		let mut direct = true;
		for part in parts.iter() {
			if let TemplatePart::Expression(node) = part {
				let Resolution::Local(symbol) = self.analysis.resolution(node) else {
					direct = false;
					break;
				};
				let kind = self.analysis.symbols.symbol(symbol).kind;
				if !matches!(
					kind,
					fa_analyzer::symbols::SymbolKind::Let
						| fa_analyzer::symbols::SymbolKind::Mutable
				) || self.node_repr(node)? != Repr::Str
					|| self.slot_of(symbol, node)?.pointer == target
				{
					direct = false;
					break;
				}
			}
		}
		if direct {
			for part in parts.iter() {
				let value = match part {
					TemplatePart::Text(text) => self.string_constant(text),
					TemplatePart::Expression(node) => self.expr(node)?,
				};
				self.append_string(target, value)?;
			}
			return Ok(Val::void());
		}
		if !self.build_template(parts)? {
			return Ok(Val::never());
		}
		self.call_runtime("fa_b_append_into", &[target.into()])?;
		Ok(Val::void())
	}

	/// Whether an expression reads the local a place is rooted at, or a
	/// reference (which may designate it): a template reading its
	/// destination is built aside.
	pub(crate) fn reads_root_of(&self, node: NodeId, place: NodeId) -> bool {
		use fa_analyzer::symbols::SymbolKind;
		use fa_analyzer::syntax::View;
		let mut root = place;
		loop {
			match self.view(root) {
				View::Group(inner) => root = inner,
				View::Access { target, .. } | View::Index { target, .. } => root = target,
				_ => break,
			}
		}
		let Resolution::Local(root) = self.analysis.resolution(root) else {
			return true;
		};
		let mut stack = vec![node.index()];
		while let Some(index) = stack.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(index))
				&& (symbol == root
					|| matches!(
						self.analysis.symbols.symbol(symbol).kind,
						SymbolKind::Use | SymbolKind::MutableUse
					)) {
				return true;
			}
			self.tree.nodes[index].for_each_child(|child| stack.push(child));
		}
		false
	}

	/// Builds a template in the text builder; `false` when a part never ends.
	fn build_template(&mut self, parts: TemplateParts<'_>) -> Result<bool, Error> {
		let mut values = Vec::new();
		for part in parts.iter() {
			match part {
				TemplatePart::Text(text) => values.push((None, Some(text.to_string()))),
				TemplatePart::Expression(node) => {
					let value = self.expr(node)?;
					if value.repr == Repr::Never {
						return Ok(false);
					}
					values.push((Some((value, node)), None));
				}
			}
		}
		self.call_runtime("fa_b_reset", &[])?;
		for (value, text) in values {
			match (value, text) {
				(Some((value, node)), _) => self.append_value(value, node)?,
				(None, Some(text)) => {
					let text = self.string_constant(&text);
					self.append_value(text, NodeId::new(0))?;
				}
				(None, None) => {}
			}
		}
		Ok(true)
	}

	/// `console.log(values…)` (fd 1) and `console.error(values…)` (fd 2):
	/// the values separated by a space, then a new line.
	pub(crate) fn print(&mut self, fd: u64, arguments: &[NodeId]) -> Result<Val<'ctx>, Error> {
		let fd_value = self.context.i32_type().const_int(fd, false);
		// only literals: one write of a constant line
		let literals: Option<Vec<&str>> = arguments
			.iter()
			.map(|&argument| match self.view(argument) {
				fa_analyzer::syntax::View::String(text) => Some(text),
				_ => None,
			})
			.collect();
		if let Some(literals) = literals {
			let line = format!("{}\n", literals.join(" "));
			let bytes = self.constant_bytes(line.as_bytes());
			let length = self.usize.const_int(line.len() as u64, false);
			self.call_runtime("fa_out", &[fd_value.into(), bytes.into(), length.into()])?;
			return Ok(Val::void());
		}
		let mut values = Vec::new();
		for &argument in arguments {
			let (node, spread) = match self.view(argument) {
				View::Spread(inner) => (inner, true),
				_ => (argument, false),
			};
			let value = self.expr(node)?;
			if value.repr == Repr::Never {
				return Ok(Val::never());
			}
			values.push((value, node, spread));
		}
		let separated = self.alloca(Repr::Bool, "print_separator")?;
		self.builder
			.build_store(separated.pointer, self.context.bool_type().const_zero())?;
		self.call_runtime("fa_b_reset", &[])?;
		for (value, node, spread) in values {
			if spread {
				let Repr::Array(item_type) = value.repr else {
					return Err(self.unsupported(node.index(), "spreading this collection"));
				};
				let array = value.value.expect("an array").into_pointer_value();
				let item = self.item_repr(item_type, node)?;
				let (_, length) = self.array_parts(array)?;
				self.for_each_item_value(array, item, length, |this, value| {
					this.print_item(value, node, separated)
				})?;
			} else {
				self.print_item(value, node, separated)?;
			}
		}
		self.call_runtime("fa_b_line", &[fd_value.into()])?;
		Ok(Val::void())
	}

	/// Appends a console item, with a separator only after an earlier item.
	fn print_item(
		&mut self,
		value: Val<'ctx>,
		node: NodeId,
		separated: Slot<'ctx>,
	) -> Result<(), Error> {
		let space = self.append_block("print_space");
		let item = self.append_block("print_item");
		let previous = self.load(separated)?.int();
		self.builder
			.build_conditional_branch(previous, space, item)?;
		self.builder.position_at_end(space);
		let separator = self.string_constant(" ");
		self.append_value(separator, node)?;
		self.branch_to(item)?;
		self.builder.position_at_end(item);
		self.append_value(value, node)?;
		self.builder.build_store(
			separated.pointer,
			self.context.bool_type().const_int(1, false),
		)?;
		Ok(())
	}

	/// `value.property` of a built-in type, on the evaluated value.
	pub(crate) fn builtin_property(
		&mut self,
		node: NodeId,
		value: Val<'ctx>,
		member: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Resolution::Builtin(id) = self.analysis.resolution(member) else {
			return Err(self.unsupported(node.index(), "this member access"));
		};
		match fa_analyzer::builtins::get(id).path {
			"Pointer#value" => {
				let target = self.type_of(member);
				self.pointer_value(value, target, member)
			}
			"CArray#length" => {
				let count = match value.repr {
					Repr::Tuple(ty) => self.c_array_count(ty).unwrap_or(0),
					_ => 0,
				};
				Ok(Val::new(
					Repr::Int(IntRepr::I64),
					self.context.i64_type().const_int(u64::from(count), false),
				))
			}
			"String#length" => {
				let (bytes, length) = self.string_parts(value)?;
				let capacity = self.string_capacity(value)?;
				let graphemes = self.runtime.function("fa_string_length")?;
				let count = self
					.call_value(graphemes, &[bytes.into(), length.into(), capacity.into()])?
					.into_int_value();
				let i64 = self.context.i64_type();
				let count = self
					.builder
					.build_int_z_extend_or_bit_cast(count, i64, "")?;
				Ok(Val::new(
					Repr::Int(crate::lower::values::IntRepr::I64),
					count,
				))
			}
			"Array#length" if matches!(value.repr, Repr::Growing(_)) => {
				let array = value.value.expect("an array").into_pointer_value();
				Ok(Val::new(
					Repr::Int(crate::lower::values::IntRepr::I64),
					self.growing_length(&self.builder, array)?,
				))
			}
			"Array#length" => {
				if let fa_analyzer::syntax::View::Access { target, members } = self.view(node)
					&& members.len() == 1
					&& members.first() == Some(member)
					&& let Some(length) = self.constant_array_length(target)
				{
					return Ok(Val::new(
						Repr::Int(IntRepr::I64),
						self.context.i64_type().const_int(length as u64, false),
					));
				}
				self.array_length(value)
			}

			"Set#size" | "Map#size" => self.table_size(value),
			path if path.ends_with("#capacity") => self.capacity_property(node, value),
			"Pool#size" | "Columnar#size" => self.pool_size(value),
			"RelationPool#size" => self.links_size_value(value),
			"OrderedColumnar#length" => self.array_length(value),
			"Range#start" => self.range_property(member, value, "start"),
			"Range#end" => self.range_property(member, value, "end"),
			"Range#includesEnd" => self.range_property(member, value, "includesEnd"),
			"Guard#value" => self.guard_value(value, member),
			"TcpListener#port" => self.listener_port(value),
			path => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}
}
