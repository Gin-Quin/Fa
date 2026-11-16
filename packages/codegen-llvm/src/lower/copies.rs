//! Loops that only copy items into an array, lowered as one block copy.
//!
//! - `for item in source { target.push(item) }`;
//! - `for index in a..b { target.push(source[index]) }`, with or without
//!   `try` (and `a..=b`).
//!
//! The pushes then make room once and copy the items with one `memcpy`
//! (`fa_array_extend`), instead of a capacity check, a store and a length
//! update per item. The conditions:
//!
//! - the body is that single `push`, so every round runs and does nothing
//!   else; its argument has no conversion at its sink (no range check, no
//!   change of representation), and the items own no memory (a bytewise copy
//!   is their copy);
//! - the source and the target are read from a local or a parameter and its
//!   fields only (no getter, no index, no call), so finding them before the
//!   loop evaluates nothing the loop would not;
//! - neither is reactive, and the function isn't a coroutine (whose loops
//!   are cooperative points of the scheduler).
//!
//! The source and the target are distinct: the analyzer refuses a push
//! whose argument is inside the target (REF013) and a change of the array a
//! loop iterates (REF012). The copy doesn't depend on it, though: room is
//! made before the source's items are read, from the block the source has
//! then, which is what the pushes would have read.
//!
//! A range reads `source[index]` for every index of `a..b`: the copy is
//! taken only when `0 <= a <= b <= source.length` (`b < source.length` for
//! `a..=b`), checked once before the loop. Otherwise the loop runs as
//! written, so a failing index raises its `IndexError` at the same round,
//! after the same pushes, and a negative index counts from the end.

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_interpreter::Value;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{Repr, Val};

/// A loop recognized by [`Lowering::copy_loop`].
#[derive(Copy, Clone)]
pub(crate) struct CopyLoop {
	/// The `target.push(…)` call.
	push: NodeId,
	/// The array read by `source[index]` (a range loop).
	source: Option<NodeId>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The `push` of a loop that only copies items (see the module), with the
	/// array its range indexes. `range`: the loop counts (`for i in a..b`),
	/// else it iterates the items of an array.
	pub(crate) fn copy_loop(
		&self,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
		range: bool,
	) -> Result<Option<CopyLoop>, Error> {
		if has_index || self.in_coroutine() {
			return Ok(None);
		}
		let Some(variable) =
			pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern))
		else {
			return Ok(None);
		};
		if !matches!(
			self.analysis.symbols.symbol(variable).kind,
			SymbolKind::LoopVariable { .. }
		) {
			return Ok(None);
		}
		let View::Block { statements } = self.view(body) else {
			return Ok(None);
		};
		let [push] = statements.iter().collect::<Vec<_>>()[..] else {
			return Ok(None);
		};
		let Some(info) = self.analysis.call(push) else {
			return Ok(None);
		};
		let fa_analyzer::analysis::Callee::Builtin(id) = info.callee else {
			return Ok(None);
		};
		if fa_analyzer::builtins::get(id).path != "Array#push" {
			return Ok(None);
		}
		let View::Call { callee, arguments } = self.view(push) else {
			return Ok(None);
		};
		let [argument] = arguments.iter().collect::<Vec<_>>()[..] else {
			return Ok(None);
		};
		let View::Access { target, members } = self.view(callee) else {
			return Ok(None);
		};
		let members: Vec<NodeId> = members.iter().collect();
		let Some(Repr::Array(item_type) | Repr::Growing(item_type)) =
			self.invariant_place(target, &members[..members.len() - 1])?
		else {
			return Ok(None);
		};
		let item = self.item_repr(item_type, push)?;
		if self.needs_drop(item) || !item.has_value() {
			return Ok(None);
		}
		// what the pushed value is: the loop variable, or `source[variable]`
		let mut value = self.ungrouped(argument);
		let mut checked = vec![argument, value];
		let source = if range {
			if let View::Try(inner) = self.view(value) {
				value = self.ungrouped(inner);
				checked.push(value);
			}
			let View::Index { target, index } = self.view(value) else {
				return Ok(None);
			};
			let index = self.ungrouped(index);
			checked.push(index);
			if self.analysis.resolution(index) != Resolution::Local(variable) {
				return Ok(None);
			}
			let source = self.ungrouped(target);
			let Some(Repr::Array(source_type) | Repr::Growing(source_type)) =
				self.invariant_place(source, &[])?
			else {
				return Ok(None);
			};
			if self.item_repr(source_type, value)? != item {
				return Ok(None);
			}
			Some(source)
		} else {
			if self.analysis.resolution(value) != Resolution::Local(variable) {
				return Ok(None);
			}
			let (Repr::Array(source_type) | Repr::Growing(source_type)) =
				self.node_repr(iterable)?
			else {
				return Ok(None);
			};
			if self.item_repr(source_type, iterable)? != item {
				return Ok(None);
			}
			None
		};
		// the value is pushed as it is read, and nothing is tracked
		if checked
			.iter()
			.any(|&node| self.analysis.coercion(node).is_some())
		{
			return Ok(None);
		}
		let mut stack = vec![iterable.index(), body.index()];
		while let Some(node) = stack.pop() {
			let id = NodeId::new(node);
			if self.analysis.reactive_reads.contains_key(&id)
				|| self.analysis.reactive_writes.contains_key(&id)
			{
				return Ok(None);
			}
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		Ok(Some(CopyLoop { push, source }))
	}

	/// The representation of `name.field.field…` when it reads a local or a
	/// parameter and plain object fields only: reading it evaluates nothing.
	pub(crate) fn invariant_place(
		&self,
		name: NodeId,
		fields: &[NodeId],
	) -> Result<Option<Repr>, Error> {
		let View::Name(_) = self.view(name) else {
			return Ok(None);
		};
		let Resolution::Local(symbol) = self.analysis.resolution(name) else {
			return Ok(None);
		};
		if !matches!(
			self.analysis.symbols.symbol(symbol).kind,
			SymbolKind::Let
				| SymbolKind::Mutable
				| SymbolKind::Parameter { .. }
				| SymbolKind::SelfParameter
		) {
			return Ok(None);
		}
		let mut repr = self.node_repr(name)?;
		for &field in fields {
			let (Repr::Object(object), Resolution::Field(_, index)) =
				(repr, self.analysis.resolution(field))
			else {
				return Ok(None);
			};
			repr = self.field_repr(object, index as usize, field.index())?;
		}
		Ok(Some(repr))
	}

	/// `for item in source { target.push(item) }`: appends every item of
	/// `source` (its header) at once.
	pub(crate) fn copy_all_items(
		&mut self,
		copy: CopyLoop,
		source: Val<'ctx>,
		length: IntValue<'ctx>,
	) -> Result<(), Error> {
		let start = self.context.i64_type().const_zero();
		self.extend(copy, source, start, length)
	}

	/// `for index in start..end { target.push(source[index]) }`: the block
	/// copy when the whole range is inside the source; else the loop is
	/// lowered at the returned block, and its end must branch to the second
	/// one.
	pub(crate) fn copy_range(
		&mut self,
		copy: CopyLoop,
		start: IntValue<'ctx>,
		end: IntValue<'ctx>,
		inclusive: bool,
	) -> Result<(BasicBlock<'ctx>, BasicBlock<'ctx>), Error> {
		let source_node = copy.source.expect("a range copy reads an array");
		let source = self.expr(source_node)?;
		let pointer = source.value.expect("an array").into_pointer_value();
		let length = match source.repr {
			Repr::Growing(_) => self.growing_length(&self.builder, pointer)?,
			_ => self.array_parts(pointer)?.1,
		};
		let i64 = self.context.i64_type();
		let zero = i64.const_zero();
		let from_start = self
			.builder
			.build_int_compare(IntPredicate::SGE, start, zero, "")?;
		let ordered = self
			.builder
			.build_int_compare(IntPredicate::SLE, start, end, "")?;
		let inside = self.builder.build_int_compare(
			if inclusive {
				IntPredicate::SLT
			} else {
				IntPredicate::SLE
			},
			end,
			length,
			"",
		)?;
		let valid = self.builder.build_and(from_start, ordered, "")?;
		let valid = self.builder.build_and(valid, inside, "inside_source")?;
		let copy_block = self.append_block("copy_range");
		let rounds = self.append_block("copy_range_rounds");
		let done = self.append_block("copied_range");
		let branch = self
			.builder
			.build_conditional_branch(valid, copy_block, rounds)?;
		self.likely(branch)?;
		self.builder.position_at_end(copy_block);
		let mut count = self.builder.build_int_sub(end, start, "")?;
		if inclusive {
			// `end < length`: no overflow
			count = self
				.builder
				.build_int_nsw_add(count, i64.const_int(1, false), "")?;
		}
		self.extend(copy, source, start, count)?;
		self.builder.build_unconditional_branch(done)?;
		self.builder.position_at_end(rounds);
		Ok((rounds, done))
	}

	/// Appends `count` items of `source` from `start` to the target of the
	/// `push`: `fa_array_extend` between arrays; one copy per segment with a
	/// `GrowingArray` (whose items never move).
	fn extend(
		&mut self,
		copy: CopyLoop,
		source: Val<'ctx>,
		start: IntValue<'ctx>,
		count: IntValue<'ctx>,
	) -> Result<(), Error> {
		let target = self.receiver(copy.push)?;
		let (Repr::Array(item_type) | Repr::Growing(item_type)) = target.repr else {
			return Err(self.unsupported(copy.push.index(), "this copy"));
		};
		let item = self.item_repr(item_type, copy.push)?;
		let size = self.item_size_const(item);
		let i32 = self.context.i32_type();
		let pointer = source.value.expect("an array").into_pointer_value();
		let target_pointer = target.value.expect("an array");
		match (target.repr, source.repr) {
			(Repr::Array(_), Repr::Array(_)) => self.call_runtime(
				"fa_array_extend",
				&[
					target_pointer.into(),
					pointer.into(),
					start.into(),
					count.into(),
					size.into(),
				],
			),
			(_, Repr::Growing(_)) => {
				let target_k = match target.repr {
					Repr::Growing(_) => self.segment_k(item),
					_ => i32.const_all_ones(),
				};
				self.call_runtime(
					"fa_segmented_extend_to",
					&[
						target_pointer.into(),
						target_k.into(),
						pointer.into(),
						start.into(),
						count.into(),
						size.into(),
						self.segment_k(item).into(),
					],
				)
			}
			_ => {
				// an array into a `GrowingArray`: distinct values
				let (items, _) = self.array_parts(pointer)?;
				let first = unsafe {
					self.builder.build_in_bounds_gep(
						self.context.i8_type(),
						items,
						&[self.builder.build_int_mul(start, size, "")?],
						"",
					)?
				};
				self.call_runtime(
					"fa_segmented_append",
					&[
						target_pointer.into(),
						first.into(),
						count.into(),
						size.into(),
						self.segment_k(item).into(),
					],
				)
			}
		}
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// `for byte in bytes { if byte == constant { … } }`, over an array of
	/// bytes: the byte the body acts on. The rounds whose item differs do
	/// nothing (the `if` is the whole body, its condition reads the item and
	/// a constant only), so the loop goes from one match to the next with a
	/// block search (`fa_bytes_next`). The search runs when the loop would
	/// read the next items, after the body of the previous match: it reads
	/// what those rounds would have read.
	pub(crate) fn matched_byte(
		&mut self,
		pattern: Option<NodeId>,
		item: Repr,
		body: NodeId,
	) -> Result<Option<u64>, Error> {
		let Repr::Int(int) = item else {
			return Ok(None);
		};
		if int.bits != 8 || self.in_coroutine() {
			return Ok(None);
		}
		let Some(variable) =
			pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern))
		else {
			return Ok(None);
		};
		let View::Block { statements } = self.view(body) else {
			return Ok(None);
		};
		let [statement] = statements.iter().collect::<Vec<_>>()[..] else {
			return Ok(None);
		};
		let View::If {
			condition,
			else_body: None,
			compile_time: false,
			..
		} = self.view(statement)
		else {
			return Ok(None);
		};
		let condition = self.ungrouped(condition);
		let View::Equality {
			left,
			right,
			negated: false,
		} = self.view(condition)
		else {
			return Ok(None);
		};
		let (left, right) = (self.ungrouped(left), self.ungrouped(right));
		let constant = if self.analysis.resolution(left) == Resolution::Local(variable) {
			right
		} else if self.analysis.resolution(right) == Resolution::Local(variable) {
			left
		} else {
			return Ok(None);
		};
		for node in [condition, left, right] {
			if self.analysis.reactive_reads.contains_key(&node) {
				return Ok(None);
			}
		}
		let value = match self.view(constant) {
			View::Integer(value) => Some(value),
			_ => match self.comptime_value(constant) {
				Some(fa_interpreter::Value::Integer(value)) => Some(value),
				_ => None,
			},
		};
		let Some(value) = value else {
			return Ok(None);
		};
		// a constant outside the item's range never matches: keep the loop
		let (low, high) = if int.signed { (-128, 127) } else { (0, 255) };
		if value < ethnum::I256::from(low) || value > ethnum::I256::from(high) {
			return Ok(None);
		}
		Ok(Some(value.as_i64() as u8 as u64))
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// `for _ in a..b { target.push(constant) }`, on an array of bytes or
	/// booleans (stored as the bytes 0/1): the `push` and the byte.
	/// Every round pushes the same byte and does nothing
	/// else, so the loop appends `b - a` copies of it at once
	/// (`fa_array_fill`). The target is found as for a copy (see the module).
	pub(crate) fn fill_loop(
		&mut self,
		iterable: NodeId,
		body: NodeId,
	) -> Result<Option<(NodeId, u64)>, Error> {
		if self.in_coroutine() {
			return Ok(None);
		}
		let View::Block { statements } = self.view(body) else {
			return Ok(None);
		};
		let [push] = statements.iter().collect::<Vec<_>>()[..] else {
			return Ok(None);
		};
		let Some(info) = self.analysis.call(push) else {
			return Ok(None);
		};
		let fa_analyzer::analysis::Callee::Builtin(id) = info.callee else {
			return Ok(None);
		};
		if fa_analyzer::builtins::get(id).path != "Array#push" {
			return Ok(None);
		}
		let View::Call { callee, arguments } = self.view(push) else {
			return Ok(None);
		};
		let [argument] = arguments.iter().collect::<Vec<_>>()[..] else {
			return Ok(None);
		};
		let View::Access { target, members } = self.view(callee) else {
			return Ok(None);
		};
		let members: Vec<NodeId> = members.iter().collect();
		let Some(Repr::Array(item_type) | Repr::Growing(item_type)) =
			self.invariant_place(target, &members[..members.len() - 1])?
		else {
			return Ok(None);
		};
		let argument = self.ungrouped(argument);
		let value = match self.view(argument) {
			View::Integer(value) => Some(Value::Integer(value)),
			View::Boolean(value) => Some(Value::Boolean(value)),
			_ => self.comptime_value(argument),
		};
		let byte = match (self.item_repr(item_type, push)?, value) {
			(Repr::Bool, Some(Value::Boolean(value))) => u64::from(value),
			(Repr::Int(int), Some(Value::Integer(value))) if int.bits == 8 => {
				// A value the item can't hold is a range check of every push: keep them.
				let (low, high) = if int.signed { (-128, 127) } else { (0, 255) };
				if value < ethnum::I256::from(low) || value > ethnum::I256::from(high) {
					return Ok(None);
				}
				value.as_i64() as u8 as u64
			}
			_ => return Ok(None),
		};
		let mut stack = vec![iterable.index(), body.index()];
		while let Some(node) = stack.pop() {
			let id = NodeId::new(node);
			if self.analysis.reactive_reads.contains_key(&id)
				|| self.analysis.reactive_writes.contains_key(&id)
			{
				return Ok(None);
			}
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		Ok(Some((push, byte)))
	}

	/// `fa_array_fill(target, rounds, byte)`, the rounds of `start..end`
	/// (`..=`), none when the range is empty.
	pub(crate) fn fill_range(
		&mut self,
		push: NodeId,
		byte: u64,
		start: IntValue<'ctx>,
		end: IntValue<'ctx>,
		inclusive: bool,
	) -> Result<(), Error> {
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
		let target = self.receiver(push)?;
		let byte = self.context.i32_type().const_int(byte, false);
		match target.repr {
			Repr::Growing(item_type) => {
				let k = self.segment_k(self.item_repr(item_type, push)?);
				self.call_runtime(
					"fa_segmented_fill",
					&[
						target.value.expect("an array").into(),
						rounds.into(),
						byte.into(),
						k.into(),
					],
				)
			}
			_ => self.call_runtime(
				"fa_array_fill",
				&[
					target.value.expect("an array").into(),
					rounds.into(),
					byte.into(),
				],
			),
		}
	}
}
