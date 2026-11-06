//! A counted loop that appends one character per round to a string
//! (`for 0..length { text += letters[…] }`): room for one byte per round is
//! reserved before the loop: inline when it fits, otherwise in an owned
//! heap block. Every append of a one-byte simple character stores a byte
//! and updates the length, with no capacity check. A longer append takes the general path, then reserves
//! again for the rounds left, so that the room always covers them. An
//! integer appended alone (`text += "{digit}"`) is one stored byte when it
//! is below 10, and goes the general way otherwise.
//!
//! A loop over the items of an array reserves, in each string local its
//! block appends to unconditionally, the least those appends add per item
//! (the literal parts of the templates) times the number of items
//! (`reserve_for_item_appends`): never more than the loop appends.
//!
//! Conditions (syntactic, like the reserved pushes of `arrays.rs`): the
//! append is a statement of the loop's block, of a `let`/`mutable` string
//! local of the file mentioned nowhere else in the body; the body has no
//! early exit (`break`, `return`, `throw`, `try`), no lambda, and calls
//! only built-ins and functions without effects, so nothing else writes the
//! string during the loop.

use fa_analyzer::analysis::Callee;
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{ArithOp, View};
use inkwell::IntPredicate;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// Reserves the room of the appends of a counted loop (see the module).
	pub(crate) fn reserve_for_appends(
		&mut self,
		body: NodeId,
		start: IntValue<'ctx>,
		end: IntValue<'ctx>,
		inclusive: bool,
		counter: PointerValue<'ctx>,
	) -> Result<Vec<NodeId>, Error> {
		let mut reserved = Vec::new();
		if self.tasks.asynchronous {
			return Ok(reserved);
		}
		let View::Block { statements } = self.view(body) else {
			return Ok(reserved);
		};
		let statements: Vec<NodeId> = statements.iter().collect();
		let mut stack = vec![body.index()];
		while let Some(node) = stack.pop() {
			let at = NodeId::new(node);
			match self.view(at) {
				View::Break(_)
				| View::Return(_)
				| View::Throw(_)
				| View::Try(_)
				| View::Lambda { .. }
				| View::Function { .. }
				| View::Start { .. }
				| View::Use { .. }
				| View::Reuse { .. } => return Ok(reserved),
				View::Call { .. } => {
					let pure = match self.analysis.call(at).map(|info| info.callee) {
						Some(Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_)) => {
							true
						}
						Some(Callee::Function(function)) => {
							self.analysis.function(function).is_some_and(|info| {
								info.sig.is_some_and(|sig| {
									let sig = self.types().sig(sig);
									let (mutates, invalidates) = self.types().effects(sig.effects);
									mutates.is_empty() && invalidates.is_empty() && !sig.is_async
								})
							})
						}
						_ => false,
					};
					if !pure {
						return Ok(reserved);
					}
				}
				_ => {}
			}
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		for statement in statements {
			let View::CompoundAssignment {
				op: ArithOp::Add,
				target,
				value,
			} = self.view(statement)
			else {
				continue;
			};
			// a template is reserved for when it is one integer (one digit per
			// round, `text += "{digit}"`)
			let template = match self.view(value) {
				View::Template { parts } => Some(self.integer_part(parts).is_some()),
				_ => None,
			};
			if template == Some(false) || !matches!(self.view(target), View::Name(_)) {
				continue;
			}
			let Resolution::Local(symbol) = self.analysis.resolution(target) else {
				continue;
			};
			if !matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable
			) || self.references.contains_key(&symbol)
				|| self.shared.contains_key(&symbol)
				|| self.reactive_sources.contains_key(&symbol)
				|| self.node_repr(target)? != Repr::Str
			{
				continue;
			}
			// the string is mentioned once in the body: by this append
			let mut uses = 0;
			let mut stack = vec![body.index()];
			while let Some(node) = stack.pop() {
				if self.analysis.resolution(NodeId::new(node)) == Resolution::Local(symbol) {
					uses += 1;
				}
				self.tree.nodes[node].for_each_child(|child| stack.push(child));
			}
			if uses != 1 {
				continue;
			}
			let slot = self.slot_of(symbol, target)?;
			if slot.repr != Repr::Str {
				continue;
			}
			let rounds = self.rounds_left(start, end, inclusive, false)?;
			let rounds = self.string_byte_count(rounds)?;
			// A dynamic short loop can stay inline too. The helper converts
			// a static prefix (including the empty literal) into an inline
			// value when it fits; otherwise it reserves the owned block.
			self.call_runtime(
				"fa_string_reserve_inline",
				&[slot.pointer.into(), rounds.into()],
			)?;
			let mode = self.alloca(Repr::Bool, "append_inline")?.pointer;
			self.set_append_mode(slot.pointer, mode)?;
			self.inline_reserved.insert((self.file, statement), mode);
			self.reserved_appends
				.insert((self.file, statement), (counter, end, inclusive));
			reserved.push(statement);
		}
		Ok(reserved)
	}

	/// Before a loop over the items of an array: room in each string local
	/// that the body appends to unconditionally (`out += "{{\"sku\":…}}"`,
	/// a statement of the loop's block) for the bytes those appends add at
	/// least, times the number of items: the literal parts of a template or
	/// a literal, one byte per integer part, nothing for a string part. A
	/// lower bound, so the reservation never exceeds what the loop appends;
	/// the string's geometric growth covers the rest. The body has no early
	/// exit and the local is declared outside of it.
	pub(crate) fn reserve_for_item_appends(
		&mut self,
		body: NodeId,
		array: Val<'ctx>,
	) -> Result<(), Error> {
		if self.tasks.asynchronous {
			return Ok(());
		}
		let View::Block { statements } = self.view(body) else {
			return Ok(());
		};
		let mut inside = rustc_hash::FxHashSet::default();
		let mut stack = vec![body.index()];
		while let Some(node) = stack.pop() {
			if matches!(
				self.view(NodeId::new(node)),
				View::Break(_) | View::Return(_) | View::Throw(_) | View::Try(_)
			) {
				return Ok(());
			}
			inside.insert(node);
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		let mut bytes: Vec<(SymbolId, NodeId, u64)> = Vec::new();
		for statement in statements.iter() {
			let View::CompoundAssignment {
				op: ArithOp::Add,
				target,
				value,
			} = self.view(statement)
			else {
				continue;
			};
			let least = match self.view(value) {
				View::String(text) => text.len() as u64,
				View::Template { parts } => {
					let mut least = 0;
					for part in parts.iter() {
						least += match part {
							fa_analyzer::syntax::TemplatePart::Text(text) => text.len() as u64,
							fa_analyzer::syntax::TemplatePart::Expression(node) => {
								u64::from(matches!(self.node_repr(node)?, Repr::Int(_)))
							}
						};
					}
					least
				}
				_ => 0,
			};
			if least == 0 || !matches!(self.view(target), View::Name(_)) {
				continue;
			}
			let Resolution::Local(symbol) = self.analysis.resolution(target) else {
				continue;
			};
			let declaration = self.analysis.symbols.symbol(symbol).decl_node;
			if !matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable
			) || inside.contains(&declaration.index())
				|| self.references.contains_key(&symbol)
				|| self.shared.contains_key(&symbol)
				|| self.reactive_sources.contains_key(&symbol)
				|| self.node_repr(target)? != Repr::Str
			{
				continue;
			}
			match bytes.iter_mut().find(|(known, _, _)| *known == symbol) {
				Some((_, _, total)) => *total += least,
				None => bytes.push((symbol, target, least)),
			}
		}
		if bytes.is_empty() {
			return Ok(());
		}
		let rounds = self.array_length(array)?.int();
		for (symbol, target, least) in bytes {
			let slot = self.slot_of(symbol, target)?;
			let multiply =
				self.intrinsic("llvm.umul.with.overflow", &[rounds.get_type().into()])?;
			let product = self
				.call_value(
					multiply,
					&[
						rounds.into(),
						rounds.get_type().const_int(least, false).into(),
					],
				)?
				.into_struct_value();
			let amount = self
				.builder
				.build_extract_value(product, 0, "")?
				.into_int_value();
			let overflow = self
				.builder
				.build_extract_value(product, 1, "")?
				.into_int_value();
			let limit = self.builder.build_int_z_extend_or_bit_cast(
				self.string_kind_mask(),
				rounds.get_type(),
				"",
			)?;
			let amount = self
				.builder
				.build_select(overflow, limit, amount, "")?
				.into_int_value();
			let extra = self.string_byte_count(amount)?;
			self.call_runtime("fa_string_reserve", &[slot.pointer.into(), extra.into()])?;
		}
		Ok(())
	}

	/// An unrepresentable reservation must stop before the unchecked stores
	/// of its loop. In wasm, truncating 2^32 bytes to zero would reserve
	/// nothing. Saturate at the string-size limit, which the runtime rejects.
	pub(super) fn string_byte_count(&self, count: IntValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let count = if self.usize.get_bit_width() < count.get_type().get_bit_width() {
			let limit =
				self.builder
					.build_int_z_extend(self.string_kind_mask(), count.get_type(), "")?;
			let minimum = self.intrinsic("llvm.umin", &[count.get_type().into()])?;
			self.call_value(minimum, &[count.into(), limit.into()])?
				.into_int_value()
		} else {
			count
		};
		Ok(self
			.builder
			.build_int_truncate_or_bit_cast(count, self.usize, "")?)
	}

	/// `end - from` (plus one when inclusive, minus one after the current
	/// round), saturated, and at least 0.
	fn rounds_left(
		&self,
		from: IntValue<'ctx>,
		end: IntValue<'ctx>,
		inclusive: bool,
		after_current: bool,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let saturating = |name: &str, a: IntValue<'ctx>, b: IntValue<'ctx>| {
			let function = self.intrinsic(name, &[i64.into()])?;
			Ok::<_, Error>(
				self.call_value(function, &[a.into(), b.into()])?
					.into_int_value(),
			)
		};
		let mut rounds = saturating("llvm.ssub.sat", end, from)?;
		if inclusive && !after_current {
			rounds = saturating("llvm.sadd.sat", rounds, i64.const_int(1, false))?;
		}
		if !inclusive && after_current {
			rounds = saturating("llvm.ssub.sat", rounds, i64.const_int(1, false))?;
		}
		let negative =
			self.builder
				.build_int_compare(IntPredicate::SLT, rounds, i64.const_zero(), "")?;
		Ok(self
			.builder
			.build_select(negative, i64.const_zero(), rounds, "")?
			.into_int_value())
	}

	/// A reserved append of an integer (`text += "{digit}"`): a value below
	/// 10 is one byte stored at the end of the heap block; another is
	/// appended as usual, then the room of the rounds left is reserved again.
	pub(crate) fn reserved_integer_append(
		&mut self,
		target: PointerValue<'ctx>,
		value: Val<'ctx>,
		counter: PointerValue<'ctx>,
		end: IntValue<'ctx>,
		inclusive: bool,
		inline_mode: Option<PointerValue<'ctx>>,
	) -> Result<(), Error> {
		let Repr::Int(int) = value.repr else {
			return Err(Error::Llvm("an integer is appended".into()));
		};
		let i64 = self.context.i64_type();
		let wide = if int.signed {
			self.builder
				.build_int_s_extend_or_bit_cast(value.int(), i64, "")?
		} else {
			self.builder
				.build_int_z_extend_or_bit_cast(value.int(), i64, "")?
		};
		let digit = self.builder.build_int_compare(
			IntPredicate::ULT,
			wide,
			i64.const_int(10, false),
			"",
		)?;
		let store = self.append_block("reserved_digit");
		let other = self.append_block("reserved_digits_other");
		let join = self.append_block("reserved_digits_appended");
		let branch = self.builder.build_conditional_branch(digit, store, other)?;
		self.likely(branch)?;
		self.builder.position_at_end(store);
		let i8 = self.context.i8_type();
		let header = self
			.load(Slot {
				pointer: target,
				repr: Repr::Str,
			})?
			.value
			.expect("a string")
			.into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(header, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(header, 1, "")?
			.into_int_value();
		let byte = self.builder.build_int_truncate(wide, i8, "")?;
		let byte = self
			.builder
			.build_or(byte, i8.const_int(b'0' as u64, false), "")?;
		if let Some(mode) = inline_mode {
			let inline = self
				.builder
				.build_load(self.context.bool_type(), mode, "")?
				.into_int_value();
			self.reserved_inline_byte(target, header, byte, join, inline)?;
		}
		let destination = unsafe { self.builder.build_in_bounds_gep(i8, bytes, &[length], "")? };
		self.builder.build_store(destination, byte)?;
		let longer = self
			.builder
			.build_int_nuw_add(length, self.usize.const_int(1, false), "")?;
		let length_pointer = self
			.builder
			.build_struct_gep(self.string_type, target, 1, "")?;
		self.builder.build_store(length_pointer, longer)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(other);
		self.append_integer(target, value)?;
		let current = self.builder.build_load(i64, counter, "")?.into_int_value();
		let left = self.rounds_left(current, end, inclusive, true)?;
		let left = self.string_byte_count(left)?;
		self.call_runtime(
			if inline_mode.is_some() {
				"fa_string_reserve_inline"
			} else {
				"fa_string_reserve_owned"
			},
			&[target.into(), left.into()],
		)?;
		if let Some(mode) = inline_mode {
			self.set_append_mode(target, mode)?;
		}
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// Whether a capacity word is an inline string's, and its length.
	fn inline_tag(
		&self,
		capacity: IntValue<'ctx>,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let flag = self
			.builder
			.build_and(capacity, self.string_inline_flag(), "")?;
		let inline =
			self.builder
				.build_int_compare(IntPredicate::NE, flag, self.usize.const_zero(), "")?;
		let shift = self
			.usize
			.const_int(u64::from(self.usize.get_bit_width()) - 8, false);
		let length = self.builder.build_right_shift(capacity, shift, false, "")?;
		let length = self
			.builder
			.build_and(length, self.usize.const_int(31, false), "")?;
		Ok((inline, length))
	}

	/// In a reserved append of a loop whose string may stay inline: when it
	/// is inline, the byte is stored in the string's own bytes and its tag
	/// counts one more (its room was checked before the loop), then `join`;
	/// otherwise the builder continues with the heap block's store.
	fn reserved_inline_byte(
		&mut self,
		target: PointerValue<'ctx>,
		header: inkwell::values::StructValue<'ctx>,
		byte: IntValue<'ctx>,
		join: inkwell::basic_block::BasicBlock<'ctx>,
		inline: IntValue<'ctx>,
	) -> Result<(), Error> {
		let capacity = self
			.builder
			.build_extract_value(header, 2, "")?
			.into_int_value();
		let in_place = self.append_block("reserved_inline");
		let heap = self.append_block("reserved_heap");
		self.builder
			.build_conditional_branch(inline, in_place, heap)?;
		self.builder.position_at_end(in_place);
		let (_, length) = self.inline_tag(capacity)?;
		let i8 = self.context.i8_type();
		let destination = unsafe {
			self.builder
				.build_in_bounds_gep(i8, target, &[length], "")?
		};
		self.builder.build_store(destination, byte)?;
		// The last word also contains inline payload bytes. Replacing that
		// word from the old header would overwrite a byte just appended at
		// offset16 (offset8 on wasm). Only the final tag byte changes.
		let shift = self
			.usize
			.const_int(u64::from(self.usize.get_bit_width()) - 8, false);
		let tag = self.builder.build_right_shift(capacity, shift, false, "")?;
		let tag = self.builder.build_int_truncate(tag, i8, "")?;
		let longer = self
			.builder
			.build_int_nuw_add(tag, i8.const_int(1, false), "")?;
		let last = u64::from(self.usize.get_bit_width() / 8) * 3 - 1;
		let tag_pointer = unsafe {
			self.builder.build_in_bounds_gep(
				i8,
				target,
				&[self.usize.const_int(last, false)],
				"",
			)?
		};
		self.builder.build_store(tag_pointer, longer)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(heap);
		Ok(())
	}

	/// Keep the mode separately from the header's changing inline length.
	/// A one-byte loop never changes this bit, so it can be unswitched;
	/// the slow path refreshes it after a longer suffix may grow the string.
	fn set_append_mode(
		&self,
		target: PointerValue<'ctx>,
		mode: PointerValue<'ctx>,
	) -> Result<(), Error> {
		let header = self
			.load(Slot {
				pointer: target,
				repr: Repr::Str,
			})?
			.value
			.expect("a string")
			.into_struct_value();
		let capacity = self
			.builder
			.build_extract_value(header, 2, "")?
			.into_int_value();
		let (inline, _) = self.inline_tag(capacity)?;
		self.builder.build_store(mode, inline)?;
		Ok(())
	}

	/// A reserved append (see the module): a one-byte simple character is
	/// stored at the end of the heap block; anything else is appended as
	/// usual, then the room of the rounds left is reserved again.
	pub(crate) fn reserved_append(
		&mut self,
		target: PointerValue<'ctx>,
		suffix: Val<'ctx>,
		counter: PointerValue<'ctx>,
		end: IntValue<'ctx>,
		inclusive: bool,
		inline_mode: Option<PointerValue<'ctx>>,
	) -> Result<(), Error> {
		let (suffix_bytes, suffix_length) = self.string_parts(suffix)?;
		let suffix_capacity = self.string_capacity(suffix)?;
		let simple = self.builder.build_int_compare(
			IntPredicate::SLT,
			suffix_capacity,
			self.usize.const_zero(),
			"",
		)?;
		let one = self.builder.build_int_compare(
			IntPredicate::EQ,
			suffix_length,
			self.usize.const_int(1, false),
			"",
		)?;
		let fast = self.builder.build_and(simple, one, "")?;
		let store = self.append_block("reserved_append");
		let other = self.append_block("reserved_append_other");
		let join = self.append_block("reserved_appended");
		let branch = self.builder.build_conditional_branch(fast, store, other)?;
		self.likely(branch)?;
		self.builder.position_at_end(store);
		let i8 = self.context.i8_type();
		let slot = Slot {
			pointer: target,
			repr: Repr::Str,
		};
		let header = self
			.load(slot)?
			.value
			.expect("a string")
			.into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(header, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(header, 1, "")?
			.into_int_value();
		let byte = self
			.builder
			.build_load(i8, suffix_bytes, "")?
			.into_int_value();
		if let Some(mode) = inline_mode {
			let inline = self
				.builder
				.build_load(self.context.bool_type(), mode, "")?
				.into_int_value();
			self.reserved_inline_byte(target, header, byte, join, inline)?;
		}
		let destination = unsafe { self.builder.build_in_bounds_gep(i8, bytes, &[length], "")? };
		self.builder.build_store(destination, byte)?;
		let longer = self
			.builder
			.build_int_nuw_add(length, self.usize.const_int(1, false), "")?;
		let updated = self.builder.build_insert_value(header, longer, 1, "")?;
		self.builder.build_store(target, updated)?;
		self.builder.build_unconditional_branch(join)?;
		// a longer character: appended as usual, then room again for the
		// rounds left
		self.builder.position_at_end(other);
		self.append_string(target, suffix)?;
		let current = self
			.builder
			.build_load(self.context.i64_type(), counter, "")?
			.into_int_value();
		let left = self.rounds_left(current, end, inclusive, true)?;
		let left = self.string_byte_count(left)?;
		self.call_runtime(
			if inline_mode.is_some() {
				"fa_string_reserve_inline"
			} else {
				"fa_string_reserve_owned"
			},
			&[target.into(), left.into()],
		)?;
		if let Some(mode) = inline_mode {
			self.set_append_mode(target, mode)?;
		}
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}
}
