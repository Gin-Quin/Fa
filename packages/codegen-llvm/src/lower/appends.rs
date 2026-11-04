//! A counted loop that appends one character per round to a string
//! (`for 0..length { text += letters[…] }`): room for one byte per round is
//! reserved before the loop, in a heap block of the size needed that the
//! string owns (even when the result would fit inline: the appends then
//! need no test of the representation), and each
//! append of a one-byte simple character is a store and a new length, with
//! no capacity check. A longer append takes the general path, then reserves
//! again for the rounds left, so that the room always covers them.
//!
//! Conditions (syntactic, like the reserved pushes of `arrays.rs`): the
//! append is a statement of the loop's block, of a `let`/`mutable` string
//! local of the file mentioned nowhere else in the body; the body has no
//! early exit (`break`, `return`, `throw`, `try`), no lambda, and calls
//! only built-ins and functions without effects, so nothing else writes the
//! string during the loop.

use fa_analyzer::analysis::Callee;
use fa_analyzer::ids::NodeId;
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
			if matches!(self.view(value), View::Template { .. })
				|| !matches!(self.view(target), View::Name(_))
			{
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
			let rounds = self
				.builder
				.build_int_truncate_or_bit_cast(rounds, self.usize, "")?;
			self.call_runtime(
				"fa_string_reserve_owned",
				&[slot.pointer.into(), rounds.into()],
			)?;
			self.reserved_appends
				.insert(statement, (counter, end, inclusive));
			reserved.push(statement);
		}
		Ok(reserved)
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
		let byte = self.builder.build_load(i8, suffix_bytes, "")?;
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
		let left = self
			.builder
			.build_int_truncate_or_bit_cast(left, self.usize, "")?;
		self.call_runtime("fa_string_reserve_owned", &[target.into(), left.into()])?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}
}
