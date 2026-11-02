//! Optionals and patterns: `a ?? b`, `when` (through the analyzer's decision
//! list, `fa_analyzer::lower::when`), the narrowing of an optional local,
//! and `for` over the characters of a string.

use fa_analyzer::ids::NodeId;
use fa_analyzer::lower::when::{Literal, Test, decision_list};
use fa_analyzer::syntax::{NodeList, View};
use inkwell::IntPredicate;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
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

	/// A read where the analysis narrowed an optional to its value (`if x
	/// != none { x + 1 }`, a `when` arm), or a union to one of its member
	/// types: the value.
	pub(crate) fn narrow_to(
		&self,
		value: Val<'ctx>,
		narrowed: Repr,
		_node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if value.repr == Repr::Union && narrowed.has_value() && narrowed != Repr::Union {
			return self.unbox(value, narrowed);
		}
		let Repr::Optional(scalar) = value.repr else {
			return Ok(value);
		};
		if scalar == Scalar::Union && narrowed.has_value() && narrowed != Repr::Union {
			let (_, inner) = self.optional_split(value)?;
			return self.unbox(inner.expect("a box"), narrowed);
		}
		if narrowed == scalar.repr() {
			let (_, inner) = self.optional_split(value)?;
			return Ok(inner.expect("a value"));
		}
		if narrowed == Repr::None {
			return Ok(Val {
				repr: Repr::None,
				value: None,
			});
		}
		Ok(value)
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
			if let Some(binding) = decision.binding {
				self.bind_arm(subject, binding, &decision.tests)?;
			}
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

	/// `pattern as name`: the subject, its data, or its value (an optional).
	fn bind_arm(
		&mut self,
		subject: Val<'ctx>,
		binding: NodeId,
		tests: &[Test<'_>],
	) -> Result<(), Error> {
		if let View::ObjectPattern { fields, rest: None } = self.view(binding) {
			// `as { a, b as c }`: the fields of the data
			let data = match (subject.repr, tests) {
				(Repr::Union, [Test::IsType(ty)]) => self.box_data(subject, *ty, binding)?,
				_ => subject,
			};
			let Repr::Object(object) = data.repr else {
				return Err(self.unsupported(binding.index(), "this `as` pattern"));
			};
			let structure = data.value.expect("an object").into_struct_value();
			for field in fields.iter() {
				let name = self.tree.name(field.name).to_string();
				let bound = self
					.tree
					.name(field.alias.unwrap_or(field.name))
					.to_string();
				let types = self.types();
				let Some(index) = types
					.fields_of(object)
					.iter()
					.position(|entry| types.syms.resolve(entry.name) == name)
				else {
					return Err(self.unsupported(binding.index(), "this `as` pattern"));
				};
				let field_type = types.fields_of(object)[index].ty;
				let symbol = self
					.analysis
					.symbols
					.symbols()
					.iter()
					.position(|symbol| {
						symbol.decl_node == binding && types.syms.resolve(symbol.name) == bound
					})
					.map(fa_analyzer::ids::SymbolId::new);
				let Some(symbol) = symbol else {
					continue;
				};
				let repr = self.repr_of(field_type, binding.index())?;
				let raw = self
					.builder
					.build_extract_value(structure, index as u32, "")?;
				let slot = self.slot_of(symbol, binding)?;
				self.store(slot, Val::new(repr, raw), binding)?;
			}
			return Ok(());
		}
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(binding) else {
			return Err(self.unsupported(binding.index(), "this `as` pattern"));
		};
		let slot = self.slot_of(symbol, binding)?;
		let value = match (subject.repr, slot.repr, tests) {
			(Repr::Union, Repr::Union, _) => subject,
			(Repr::Union, _, [Test::IsType(ty)]) => self.box_data(subject, *ty, binding)?,
			(Repr::Optional(_), repr, _) => self.narrow_to(subject, repr, binding)?,
			_ => subject,
		};
		self.store(slot, value, binding)
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
			} => self.range_test(subject, *start, *end, *inclusive, node),
			Test::IsType(ty) => self.type_test(subject, *ty, node),
		}
	}

	/// Whether a value is in `start..end` (`..=` when `inclusive`).
	pub(crate) fn range_test(
		&mut self,
		subject: Val<'ctx>,
		start: Option<NodeId>,
		end: Option<NodeId>,
		inclusive: bool,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let (start, end, inclusive) = (&start, &end, &inclusive);
		{
			{
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
		}
	}

	/// Whether a value is of a type: a member of its union, `none`…
	pub(crate) fn type_test(
		&mut self,
		subject: Val<'ctx>,
		ty: fa_analyzer::ids::TypeId,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let ty = &ty;
		{
			match subject.repr {
				Repr::Union => self.box_is(subject, *ty),
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
					let (present, inner) = self.optional_split(subject)?;
					if *ty == fa_analyzer::ids::TypeId::NONE {
						Ok(self.builder.build_not(present, "")?)
					} else if self.repr_of(*ty, node.index())? == scalar.repr() {
						Ok(present)
					} else if scalar == Scalar::Union {
						// a member of the union inside the optional
						let inner = inner.expect("a box");
						let pointer = inner.value.expect("a box").into_pointer_value();
						let null = self.pointer_type().const_null();
						let safe = self
							.builder
							.build_select(present, pointer, null, "")?
							.into_pointer_value();
						let is_box = self.builder.build_is_not_null(safe, "")?;
						let check = self.append_block("is_member");
						let join = self.append_block("is_done");
						let result = self.alloca(Repr::Bool, "is")?;
						self.builder.build_store(result.pointer, is_box)?;
						self.builder.build_conditional_branch(is_box, check, join)?;
						self.builder.position_at_end(check);
						let member = self.box_is(Val::new(Repr::Union, safe), *ty)?;
						self.builder.build_store(result.pointer, member)?;
						self.builder.build_unconditional_branch(join)?;
						self.builder.position_at_end(join);
						Ok(self.load(result)?.int())
					} else {
						Err(self.unsupported(node.index(), "this type pattern"))
					}
				}
				Repr::None => Ok(self
					.context
					.bool_type()
					.const_int((*ty == fa_analyzer::ids::TypeId::NONE) as u64, false)),
				_ => Ok(self.context.bool_type().const_int(1, false)),
			}
		}
	}

	/// `for character in text`: the grapheme clusters of a string.
	pub(crate) fn for_characters(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		text: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let (bytes, length) = self.string_parts(text)?;
		let index_slot = match has_index
			.then(|| self.analysis.symbols.symbol_of_decl(node))
			.flatten()
		{
			Some(symbol) => Some(self.slot_of(symbol, node)?),
			None => None,
		};
		let position = self.alloca(Repr::Int(IntRepr::I64), "character_index")?;
		self.builder
			.build_store(position.pointer, self.context.i64_type().const_zero())?;
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
		let current = self.load(position)?.int();
		if let Some(index_slot) = index_slot {
			self.store(index_slot, Val::new(Repr::Int(IntRepr::I64), current), node)?;
		}
		let following = self.builder.build_int_nsw_add(
			current,
			self.context.i64_type().const_int(1, false),
			"",
		)?;
		self.builder.build_store(position.pointer, following)?;
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
