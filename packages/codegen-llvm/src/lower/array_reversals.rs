//! Version a plain two-ended swap loop with a proven in-bounds reversal.
//! Negative/out-of-range indices retain the original checked loop, including
//! its error order. Only scalar items and private mutable counters qualify:
//! counters cannot alias elements, and no copy/drop or user effect is skipped.

use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{ArithOp, BindingKind, CompareOp, View};
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::values::{IntValue, PointerValue};

use super::values::{IntRepr, Repr};
use super::{Lowering, Slot};
use crate::error::Error;

struct Reversal {
	array: NodeId,
	low: NodeId,
	high: NodeId,
	reads: [NodeId; 2],
	writes: [NodeId; 2],
}

impl<'ctx> Lowering<'_, 'ctx> {
	fn reversal_value(&self, node: NodeId) -> NodeId {
		let node = self.ungrouped(node);
		match self.view(node) {
			View::Try(inner) => self.ungrouped(inner),
			_ => node,
		}
	}

	fn reversal_symbol(&self, node: NodeId) -> Option<SymbolId> {
		if !matches!(self.view(self.ungrouped(node)), View::Name(_)) {
			return None;
		}
		match self.analysis.resolution(self.ungrouped(node)) {
			Resolution::Local(symbol) => Some(symbol),
			_ => None,
		}
	}

	fn reversal_read(&self, declaration: NodeId) -> Option<(SymbolId, NodeId, NodeId, NodeId)> {
		let View::Declaration {
			kind: BindingKind::Let,
			annotation: None,
			value: Some(value),
			..
		} = self.view(declaration)
		else {
			return None;
		};
		let indexed = self.reversal_value(value);
		let View::Index { target, index } = self.view(indexed) else {
			return None;
		};
		Some((
			self.analysis.symbols.symbol_of_decl(declaration)?,
			target,
			index,
			value,
		))
	}

	fn reversal_store(
		&self,
		node: NodeId,
		array: SymbolId,
		index: SymbolId,
		value: SymbolId,
	) -> Option<NodeId> {
		let View::Assignment {
			target,
			annotation: None,
			value: Some(read),
		} = self.view(self.reversal_value(node))
		else {
			return None;
		};
		let View::Index {
			target,
			index: position,
		} = self.view(self.reversal_value(target))
		else {
			return None;
		};
		(self.reversal_symbol(target) == Some(array)
			&& self.reversal_symbol(position) == Some(index)
			&& self.reversal_symbol(read) == Some(value))
		.then_some(read)
	}

	fn reversal_step(&self, node: NodeId, symbol: SymbolId, operation: ArithOp) -> bool {
		matches!(self.view(self.reversal_value(node)), View::CompoundAssignment { op, target, value }
			if op == operation && self.reversal_symbol(target) == Some(symbol)
				&& matches!(self.view(self.ungrouped(value)), View::Integer(value) if value == ethnum::I256::ONE))
	}

	fn reversal_pattern(&self, condition: NodeId, body: NodeId) -> Option<Reversal> {
		let View::Comparison {
			operands,
			operators,
		} = self.view(self.ungrouped(condition))
		else {
			return None;
		};
		let [low, high] = operands.iter().collect::<Vec<_>>()[..] else {
			return None;
		};
		if operators.len() != 1 || operators.get(0) != Some(CompareOp::Less) {
			return None;
		}
		let low_symbol = self.reversal_symbol(low)?;
		let high_symbol = self.reversal_symbol(high)?;
		if low_symbol == high_symbol {
			return None;
		}
		let View::Block { statements } = self.view(body) else {
			return None;
		};
		let [first, second, put_low, put_high, step_low, step_high] =
			statements.iter().collect::<Vec<_>>()[..]
		else {
			return None;
		};
		let (left_value, array, left_index, left_read) = self.reversal_read(first)?;
		let (right_value, other, right_index, right_read) = self.reversal_read(second)?;
		let array_symbol = self.reversal_symbol(array)?;
		if self.reversal_symbol(other) != Some(array_symbol)
			|| self.reversal_symbol(left_index) != Some(low_symbol)
			|| self.reversal_symbol(right_index) != Some(high_symbol)
			|| !self.reversal_step(step_low, low_symbol, ArithOp::Add)
			|| !self.reversal_step(step_high, high_symbol, ArithOp::Subtract)
		{
			return None;
		}
		let writes = [
			self.reversal_store(put_low, array_symbol, low_symbol, right_value)?,
			self.reversal_store(put_high, array_symbol, high_symbol, left_value)?,
		];
		Some(Reversal {
			array,
			low,
			high,
			reads: [left_read, right_read],
			writes,
		})
	}

	fn reversal_counter(&self, node: NodeId) -> Option<Slot<'ctx>> {
		let symbol = self.reversal_symbol(node)?;
		if self.analysis.symbols.symbol(symbol).kind != SymbolKind::Mutable
			|| self.references.contains_key(&symbol)
			|| self.shared.contains_key(&symbol)
			|| self.reactive_sources.contains_key(&symbol)
		{
			return None;
		}
		self.slots
			.get(&symbol)
			.copied()
			.filter(|slot| slot.repr == Repr::Int(IntRepr::I64))
	}

	/// Emits the fast version and leaves the builder at the original loop's
	/// fallback entry. Its exit must join the returned block.
	pub(super) fn begin_array_reversal(
		&mut self,
		condition: NodeId,
		body: NodeId,
	) -> Result<Option<BasicBlock<'ctx>>, Error> {
		if self.tasks.asynchronous {
			return Ok(None);
		}
		let Some(pattern) = self.reversal_pattern(condition, body) else {
			return Ok(None);
		};
		let Some(low_slot) = self.reversal_counter(pattern.low) else {
			return Ok(None);
		};
		let Some(high_slot) = self.reversal_counter(pattern.high) else {
			return Ok(None);
		};
		let Some(Repr::Array(item_type)) = self.invariant_place(pattern.array, &[])? else {
			return Ok(None);
		};
		let symbol = self
			.reversal_symbol(pattern.array)
			.expect("a plain array name");
		if self.references.contains_key(&symbol)
			|| self.shared.contains_key(&symbol)
			|| self.reactive_sources.contains_key(&symbol)
		{
			return Ok(None);
		}
		let item = self.item_repr(item_type, pattern.array)?;
		if !matches!(
			item,
			Repr::Bool | Repr::Int(_) | Repr::F32 | Repr::F64 | Repr::Percentage
		) {
			return Ok(None);
		}
		for node in pattern.reads.into_iter().chain(pattern.writes) {
			if self.analysis.coercion(node).is_some() || self.node_repr(node)? != item {
				return Ok(None);
			}
		}
		let mut stack = vec![condition.index(), body.index()];
		while let Some(at) = stack.pop() {
			let node = NodeId::new(at);
			if self.analysis.reactive_reads.contains_key(&node)
				|| self.analysis.reactive_writes.contains_key(&node)
			{
				return Ok(None);
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		let array = self
			.expr(pattern.array)?
			.value
			.expect("an array")
			.into_pointer_value();
		let low = self.load(low_slot)?.int();
		let high = self.load(high_slot)?.int();
		let (items, length) = self.array_parts(array)?;
		let i64 = self.context.i64_type();
		let nonnegative =
			self.builder
				.build_int_compare(IntPredicate::SGE, low, i64.const_zero(), "")?;
		let ordered = self
			.builder
			.build_int_compare(IntPredicate::SLT, low, high, "")?;
		let bounded = self
			.builder
			.build_int_compare(IntPredicate::SLT, high, length, "")?;
		let inside = self.builder.build_and(nonnegative, ordered, "")?;
		let inside = self
			.builder
			.build_and(inside, bounded, "reverse_in_bounds")?;
		let fast = self.append_block("reverse_range");
		let fallback = self.append_block("reverse_checked");
		let done = self.append_block("reverse_done");
		self.builder
			.build_conditional_branch(inside, fast, fallback)?;
		self.builder.position_at_end(fast);
		self.reverse_scalar_pairs(item, items, (low, high), (low_slot, high_slot))?;
		self.builder.build_unconditional_branch(done)?;
		self.builder.position_at_end(fallback);
		Ok(Some(done))
	}
	/// Both halves are in bounds and disjoint; these items have no copy/drop
	/// effects. Counter updates cannot overflow inside this proven range.
	fn reverse_scalar_pairs(
		&self,
		item: Repr,
		items: PointerValue<'ctx>,
		(low, high): (IntValue<'ctx>, IntValue<'ctx>),
		(low_slot, high_slot): (Slot<'ctx>, Slot<'ctx>),
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let span = self.builder.build_int_nsw_sub(high, low, "")?;
		let count = self
			.builder
			.build_int_nsw_add(span, i64.const_int(1, false), "")?;
		let half = self.builder.build_right_shift(
			count,
			i64.const_int(1, false),
			false,
			"reverse_pairs",
		)?;
		self.helper_loop(
			&self.builder,
			self.frame().function,
			half,
			|builder, offset| {
				let left = builder.build_int_nsw_add(low, offset, "")?;
				let right = builder.build_int_nsw_sub(high, offset, "")?;
				let ty = self.llvm_type(item);
				let left =
					unsafe { builder.build_in_bounds_gep(ty, items, &[left], "reverse_left")? };
				let right =
					unsafe { builder.build_in_bounds_gep(ty, items, &[right], "reverse_right")? };
				let a = builder.build_load(ty, left, "")?;
				let b = builder.build_load(ty, right, "")?;
				for value in [a, b] {
					self.tag(
						inkwell::values::BasicValue::as_instruction_value(&value).expect("a load"),
						false,
					)?;
				}
				self.tag(builder.build_store(left, b)?, false)?;
				self.tag(builder.build_store(right, a)?, false)?;
				Ok(())
			},
		)?;
		self.builder.build_store(
			low_slot.pointer,
			self.builder.build_int_nsw_add(low, half, "")?,
		)?;
		self.builder.build_store(
			high_slot.pointer,
			self.builder.build_int_nsw_sub(high, half, "")?,
		)?;
		Ok(())
	}
}
