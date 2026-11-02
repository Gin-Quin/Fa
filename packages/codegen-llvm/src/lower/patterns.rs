//! Optionals and patterns: `a ?? b`, `when` (through the analyzer's decision
//! list, `fa_analyzer::lower::when`), the narrowing of an optional local,
//! and `for` over the characters of a string.

use fa_analyzer::ids::NodeId;
use fa_analyzer::lower::when::{Literal, Test, decision_list};
use fa_analyzer::syntax::{NodeList, View};
use inkwell::IntPredicate;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// The presence and the value of an optional.
	fn optional_split(
		&self,
		value: Val<'ctx>,
	) -> Result<(IntValue<'ctx>, Option<Val<'ctx>>), Error> {
		match value.repr {
			Repr::None => Ok((self.context.bool_type().const_zero(), None)),
			Repr::Optional(scalar) => {
				let pair = value.value.expect("an optional").into_struct_value();
				let present = self
					.builder
					.build_extract_value(pair, 0, "")?
					.into_int_value();
				let inner = self.builder.build_extract_value(pair, 1, "")?;
				Ok((present, Some(Val::new(scalar.repr(), inner))))
			}
			_ => Ok((self.context.bool_type().const_int(1, false), Some(value))),
		}
	}

	/// A local read where the analysis narrowed an optional to its value
	/// (`if x != none { x + 1 }`, a `when` arm): the value of the optional.
	pub(crate) fn narrow(&self, value: Val<'ctx>, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Repr::Optional(scalar) = value.repr else {
			return Ok(value);
		};
		match self.node_repr(node) {
			Ok(repr) if repr == scalar.repr() => {
				let (_, inner) = self.optional_split(value)?;
				Ok(inner.expect("a value"))
			}
			Ok(Repr::None) => Ok(Val {
				repr: Repr::None,
				value: None,
			}),
			_ => Ok(value),
		}
	}

	/// `a ?? b ?? c`: the first present value, each operand evaluated only
	/// when the previous ones are `none`.
	pub(crate) fn fallback(
		&mut self,
		node: NodeId,
		operands: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let slot = self.alloca(repr, "fallback")?;
		let end = self.append_block("end_fallback");
		let count = operands.len();
		for (index, operand) in operands.iter().enumerate() {
			let value = self.expr(operand)?;
			if value.repr == Repr::Never {
				break;
			}
			let last = index + 1 == count;
			match value.repr {
				Repr::Optional(_) | Repr::None if !last || !matches!(repr, Repr::Optional(_)) => {
					let (present, inner) = self.optional_split(value)?;
					let take = self.append_block("fallback_present");
					let next = self.append_block("fallback_next");
					self.builder.build_conditional_branch(present, take, next)?;
					self.builder.position_at_end(take);
					if let Some(inner) = inner {
						self.store(slot, inner, operand)?;
					}
					self.builder.build_unconditional_branch(end)?;
					self.builder.position_at_end(next);
					if last {
						self.builder.build_unreachable()?;
					}
				}
				_ => {
					self.store(slot, value, operand)?;
					self.builder.build_unconditional_branch(end)?;
					self.continue_unreachable();
					break;
				}
			}
		}
		self.branch_to(end)?;
		self.builder.position_at_end(end);
		self.load(slot)
	}

	/// `when subject is { … }`: the subject once, then the tests of each arm
	/// in order; the last arm runs by default when there is no `else` (every
	/// `when` of a checked program is exhaustive).
	pub(crate) fn when_expression(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let list = decision_list(self.tree, self.analysis, node)
			.map_err(|_| self.unsupported(node.index(), "this `when`"))?;
		let repr = if self.type_of(node) == fa_analyzer::ids::TypeId::UNKNOWN {
			// the analysis leaves a `when` printed by `console.log` without a
			// type: the common representation of its arms
			let mut common = Repr::Never;
			for decision in &list.decisions {
				let arm = self.node_repr(decision.value)?;
				common = match (common, arm) {
					(Repr::Never, arm) | (arm, Repr::Never) => arm,
					(Repr::Int(a), Repr::Int(b)) => Repr::Int(a.common(b)),
					(a, b) if a == b => a,
					_ => return Err(self.unsupported(node.index(), "this `when`")),
				};
			}
			common
		} else {
			self.node_repr(node)?
		};
		let slot = if repr.has_value() {
			Some(self.alloca(repr, "when")?)
		} else {
			None
		};
		let subject = self.expr(list.subject)?;
		if subject.repr == Repr::Never {
			return Ok(subject);
		}
		let end = self.append_block("end_when");
		let count = list.decisions.len();
		let mut reached = false;
		for (index, decision) in list.decisions.iter().enumerate() {
			if let Some(binding) = decision.binding {
				return Err(self.unsupported(binding.index(), "`as` in a `when` arm"));
			}
			let default = decision.is_else() || index + 1 == count;
			let body = self.append_block("when_arm");
			let next = if default {
				None
			} else {
				Some(self.append_block("when_next"))
			};
			if default {
				self.builder.build_unconditional_branch(body)?;
			} else {
				let next = next.expect("a next arm");
				let tests = decision.tests.len();
				for (position, test) in decision.tests.iter().enumerate() {
					let holds = self.when_test(subject, test, node)?;
					let otherwise = if position + 1 == tests {
						next
					} else {
						self.append_block("when_test")
					};
					self.builder
						.build_conditional_branch(holds, body, otherwise)?;
					self.builder.position_at_end(otherwise);
				}
			}
			self.builder.position_at_end(body);
			let value = match self.view(decision.value) {
				View::Block { .. } => self.block(decision.value, slot.is_some())?,
				_ => self.expr(decision.value)?,
			};
			if value.repr != Repr::Never {
				if let Some(slot) = slot {
					self.store(slot, value, decision.value)?;
				}
				reached = true;
				self.branch_to(end)?;
			}
			match next {
				Some(next) => self.builder.position_at_end(next),
				None => break,
			}
		}
		self.builder.position_at_end(end);
		if !reached {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		match slot {
			Some(slot) => self.load(slot),
			None => Ok(Val::void()),
		}
	}

	/// One test of a `when` arm on the subject.
	fn when_test(
		&mut self,
		subject: Val<'ctx>,
		test: &Test<'_>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		match test {
			Test::Equals(Literal::None) => {
				let (present, _) = self.optional_split(subject)?;
				Ok(self.builder.build_not(present, "")?)
			}
			Test::Equals(literal) => {
				let (present, inner) = self.optional_split(subject)?;
				let Some(inner) = inner else {
					return Ok(self.context.bool_type().const_zero());
				};
				let constant = match *literal {
					Literal::Integer(value) => match inner.repr {
						Repr::Int(int) => Val::new(inner.repr, self.const_int(int.bits, value)),
						_ => Val::new(
							Repr::F64,
							self.context.f64_type().const_float(value.as_f64()),
						),
					},
					Literal::Float(value) => {
						Val::new(Repr::F64, self.context.f64_type().const_float(value))
					}
					Literal::String(text) => self.string_constant(text),
					Literal::Boolean(value) => Val::new(
						Repr::Bool,
						self.context.bool_type().const_int(value as u64, false),
					),
					Literal::None => unreachable!("handled above"),
				};
				let equal = self.values_equal(inner, constant, node)?;
				Ok(self.builder.build_and(present, equal, "")?)
			}
			Test::InRange {
				start,
				end,
				inclusive,
			} => {
				let mut holds = self.context.bool_type().const_int(1, false);
				if let Some(start) = start {
					let bound = self.expr(*start)?;
					let above = self.values_order(bound, subject, IntPredicate::SLE, node)?;
					holds = self.builder.build_and(holds, above, "")?;
				}
				if let Some(end) = end {
					let bound = self.expr(*end)?;
					let predicate = if *inclusive {
						IntPredicate::SLE
					} else {
						IntPredicate::SLT
					};
					let below = self.values_order(subject, bound, predicate, node)?;
					holds = self.builder.build_and(holds, below, "")?;
				}
				Ok(holds)
			}
			Test::IsType(ty) => match subject.repr {
				Repr::Enum(_) => match self.types().kind(*ty) {
					fa_analyzer::types::TypeKind::EnumMember(_, index) => {
						Ok(self.builder.build_int_compare(
							IntPredicate::EQ,
							subject.int(),
							self.context.i32_type().const_int(index as u64, false),
							"",
						)?)
					}
					_ => Err(self.unsupported(node.index(), "this type pattern")),
				},
				Repr::Optional(scalar) => {
					let (present, _) = self.optional_split(subject)?;
					if *ty == fa_analyzer::ids::TypeId::NONE {
						Ok(self.builder.build_not(present, "")?)
					} else if self.repr_of(*ty, node.index())? == scalar.repr() {
						Ok(present)
					} else {
						Err(self.unsupported(node.index(), "this type pattern"))
					}
				}
				_ => Err(self.unsupported(node.index(), "this type pattern")),
			},
		}
	}

	/// `for character in text`: the grapheme clusters of a string.
	pub(crate) fn for_characters(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		text: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let (bytes, length) = self.string_parts(text)?;
		let offset = self.alloca(Repr::Int(IntRepr::I64), "offset")?;
		let usize_repr = IntRepr {
			bits: self.usize.get_bit_width(),
			signed: false,
		};
		self.builder
			.build_store(offset.pointer, self.context.i64_type().const_zero())?;
		let variable: Option<Slot<'ctx>> =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => Some(self.slot_of(symbol, node)?),
				None => None,
			};
		let test = self.append_block("for_character");
		let body_block = self.append_block("for_character_body");
		let exit = self.append_block("end_for_character");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let start = self.load(offset)?.int();
		let start = self.int_to(start, IntRepr::I64, usize_repr)?;
		let more = self
			.builder
			.build_int_compare(IntPredicate::ULT, start, length, "")?;
		self.builder
			.build_conditional_branch(more, body_block, exit)?;
		self.builder.position_at_end(body_block);
		let next_boundary = self.runtime.function("fa_next_grapheme")?;
		let stop = self
			.call_value(next_boundary, &[bytes.into(), length.into(), start.into()])?
			.into_int_value();
		let character_bytes = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), bytes, &[start], "")?
		};
		let character_length = self.builder.build_int_sub(stop, start, "")?;
		let pair =
			self.builder
				.build_insert_value(self.pair_type.get_undef(), character_bytes, 0, "")?;
		let pair = self
			.builder
			.build_insert_value(pair, character_length, 1, "")?;
		if let Some(variable) = variable {
			self.store(
				variable,
				Val::new(Repr::Str, pair.into_struct_value()),
				node,
			)?;
		}
		let wide_stop = self.int_to(stop, usize_repr, IntRepr::I64)?;
		self.builder.build_store(offset.pointer, wide_stop)?;
		self.frame_mut().loops.push(LoopFrame {
			node,
			exit,
			next: test,
			value: None,
		});
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}
}
