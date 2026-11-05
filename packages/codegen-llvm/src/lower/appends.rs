//! A counted loop that appends one character per round to a string
//! (`for 0..length { text += letters[…] }`): room for one byte per round is
//! reserved before the loop, in a heap block of the size needed that the
//! string owns (even when the result would fit inline: the appends then
//! need no test of the representation), and each
//! append of a one-byte simple character is a store and a new length, with
//! no capacity check. A longer append takes the general path, then reserves
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
			let rounds = self
				.builder
				.build_int_truncate_or_bit_cast(rounds, self.usize, "")?;
			let extra =
				self.builder
					.build_int_nuw_mul(rounds, self.usize.const_int(least, false), "")?;
			self.call_runtime("fa_string_reserve", &[slot.pointer.into(), extra.into()])?;
		}
		Ok(())
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
		let left = self
			.builder
			.build_int_truncate_or_bit_cast(left, self.usize, "")?;
		self.call_runtime("fa_string_reserve_owned", &[target.into(), left.into()])?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
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
