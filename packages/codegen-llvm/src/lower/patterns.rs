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
	pub(crate) fn optional_split(
		&self,
		value: Val<'ctx>,
	) -> Result<(IntValue<'ctx>, Option<Val<'ctx>>), Error> {
		match value.repr {
			Repr::None => Ok((self.context.bool_type().const_zero(), None)),
			Repr::Optional(_) => {
				let (present, inner) = self.split_optional(&self.builder, value)?;
				Ok((present, Some(inner)))
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
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if value.repr == narrowed {
			return Ok(value);
		}
		if let Repr::Rec(target) = value.repr
			&& self.compressed_union_link(value.repr).is_some()
		{
			let decoded = self.unfold_rec(value, target, node.index())?;
			return self.narrow_to(decoded, narrowed, node);
		}
		if matches!(value.repr, Repr::Union | Repr::InlineUnion(_))
			&& narrowed.has_value()
			&& narrowed != Repr::Union
		{
			return self.unbox(value, narrowed);
		}
		let Repr::Optional(scalar) = value.repr else {
			return Ok(value);
		};
		if matches!(scalar, Scalar::Union | Scalar::InlineUnion(_))
			&& narrowed.has_value()
			&& narrowed != Repr::Union
			&& narrowed != scalar.repr()
		{
			let (_, inner) = self.optional_split(value)?;
			return self.unbox(inner.expect("a box"), narrowed);
		}
		if narrowed == scalar.repr() {
			let (_, inner) = self.optional_split(value)?;
			return Ok(inner.expect("a value"));
		}
		if let Scalar::Rec(target) = scalar
			&& self.repr_of(target, node.index())? == narrowed
		{
			let (_, inner) = self.optional_split(value)?;
			return self.convert(inner.expect("a recursive value"), narrowed, node.index());
		}
		if narrowed == Repr::None {
			return Ok(Val {
				repr: Repr::None,
				value: None,
			});
		}
		Ok(value)
	}

	/// The place of the payload of an optional or a union narrowed to
	/// `narrowed` (the analysis' `NARROWED` contract): a write through it
	/// changes the payload the place holds, never a copy. The optional's own
	/// slot when nullable (a pointer either way), its field 1 otherwise; the
	/// data of a union's box.
	pub(crate) fn payload_place(
		&mut self,
		slot: Slot<'ctx>,
		narrowed: Repr,
		node: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		if slot.repr == narrowed || !narrowed.has_value() {
			return Ok(slot);
		}
		match slot.repr {
			Repr::Optional(scalar) => {
				let inner = if scalar.nullable() {
					Slot {
						pointer: slot.pointer,
						repr: scalar.repr(),
					}
				} else {
					let ty = self.llvm_type(slot.repr).into_struct_type();
					Slot {
						pointer: self.builder.build_struct_gep(ty, slot.pointer, 1, "")?,
						repr: scalar.repr(),
					}
				};
				if let Repr::Rec(target) = inner.repr
					&& self.repr_of(target, node.index())? == narrowed
				{
					let pointer = self.load_edge(inner)?;
					return Ok(Slot {
						pointer,
						repr: narrowed,
					});
				}
				self.payload_place(inner, narrowed, node)
			}
			Repr::Union if narrowed != Repr::Union => {
				let pointer = self
					.builder
					.build_load(self.pointer_type(), slot.pointer, "")?
					.into_pointer_value();
				if let Repr::Object(data) = narrowed
					&& (!self.compact_array_data(data).is_empty()
						|| !self.compact_string_data(data).is_empty()
						|| !self.compact_scalar_data(data).is_empty())
				{
					let value = self.unbox(Val::new(Repr::Union, pointer), narrowed)?;
					let out = self.alloca(narrowed, "member_view")?;
					self.builder
						.build_store(out.pointer, value.value.expect("a payload"))?;
					return Ok(out);
				}
				let ty = self.box_type(Some(narrowed));
				Ok(Slot {
					pointer: self.builder.build_struct_gep(ty, pointer, 1, "")?,
					repr: narrowed,
				})
			}
			_ => Ok(slot),
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
			// each operand is a region: what it registers is dropped on both ways
			let mark = self.mark();
			let value = self.expr(operand)?;
			if value.repr == Repr::Never {
				break;
			}
			let last = index + 1 == count;
			match value.repr {
				Repr::Optional(_) | Repr::None if !last || !matches!(repr, Repr::Optional(_)) => {
					// a temporary optional gives its value up (absent, it owns nothing)
					let moved = self.is_temporary(value);
					if moved {
						self.take(value, None)?;
					}
					let (present, inner) = self.optional_split(value)?;
					let take = self.append_block("fallback_present");
					let next = self.append_block("fallback_next");
					self.builder.build_conditional_branch(present, take, next)?;
					self.builder.position_at_end(take);
					if let Some(inner) = inner {
						let inner = if moved { self.fresh(inner) } else { inner };
						let inner = self.owned_as(inner, repr, Some(operand))?;
						self.store(slot, inner, operand)?;
					}
					self.drop_to(mark)?;
					self.builder.build_unconditional_branch(end)?;
					self.builder.position_at_end(next);
					self.drop_to(mark)?;
					self.cleanups.borrow_mut().truncate(mark);
					if last {
						self.builder.build_unreachable()?;
					}
				}
				_ => {
					let value = self.owned_as(value, repr, Some(operand))?;
					self.store(slot, value, operand)?;
					self.drop_to(mark)?;
					self.cleanups.borrow_mut().truncate(mark);
					self.builder.build_unconditional_branch(end)?;
					self.continue_unreachable();
					break;
				}
			}
		}
		self.branch_to(end)?;
		self.builder.position_at_end(end);
		Ok(self.fresh(self.load(slot)?))
	}

	/// `when subject is { … }`: the subject once, then the tests of each arm
	/// in order; the last arm runs by default when there is no `else` (every
	/// `when` of a checked program is exhaustive).
	/// A `when` choosing among string literals of one length by an integer
	/// (`0 => " + "`, `1 => " - "`, …, `else => " / "`): the literals packed
	/// into one constant in the order of the integers, `else` last, and the
	/// chosen one at `base + length × index` (the index clamped to the
	/// `else` one), with no branch nor table load. The arms are the integers
	/// `0` to `count - 1`, one per arm, without `as` nor escapes.
	fn packed_literal_choice(
		&mut self,
		list: &fa_analyzer::lower::when::DecisionList<'_>,
		subject: Val<'ctx>,
	) -> Result<Option<Val<'ctx>>, Error> {
		let Repr::Int(int) = subject.repr else {
			return Ok(None);
		};
		if int.bits > 64 || !list.has_else() || list.decisions.len() < 3 {
			return Ok(None);
		}
		let count = list.decisions.len() - 1;
		let mut texts: Vec<Option<&str>> = vec![None; count + 1];
		for (position, decision) in list.decisions.iter().enumerate() {
			if decision.binding.is_some() {
				return Ok(None);
			}
			let mut value = decision.value;
			loop {
				match self.view(value) {
					View::Group(inner) => value = inner,
					View::Block { statements } if statements.len() == 1 => {
						value = statements.first().expect("one statement")
					}
					_ => break,
				}
			}
			let View::String(text) = self.view(value) else {
				return Ok(None);
			};
			if text.is_empty() || text.contains(['\\', '{', '}']) {
				return Ok(None);
			}
			let index = if decision.is_else() {
				count
			} else {
				let [Test::Equals(Literal::Integer(value))] = decision.tests.as_slice() else {
					return Ok(None);
				};
				match usize::try_from(*value) {
					Ok(index) if index < count => index,
					_ => return Ok(None),
				}
			};
			if position == count && index != count || texts[index].is_some() {
				return Ok(None);
			}
			texts[index] = Some(text);
		}
		let Some(texts) = texts.into_iter().collect::<Option<Vec<&str>>>() else {
			return Ok(None);
		};
		let length = texts[0].len();
		let simple = |text: &str| text.bytes().all(|byte| byte < 0x80 && byte != b'\r');
		if texts.len() != count + 1
			|| texts
				.iter()
				.any(|text| text.len() != length || simple(text) != simple(texts[0]))
		{
			return Ok(None);
		}
		let packed: String = texts.concat();
		let base = self.constant_bytes(packed.as_bytes());
		// the subject as an unsigned index: a negative one is above `count`
		let i64 = self.context.i64_type();
		let raw = subject.value.expect("an integer").into_int_value();
		let index = if int.bits < 64 {
			self.builder
				.build_int_cast_sign_flag(raw, i64, int.signed, "")?
		} else {
			self.builder.build_int_truncate_or_bit_cast(raw, i64, "")?
		};
		let last = i64.const_int(count as u64, false);
		let inside = self
			.builder
			.build_int_compare(IntPredicate::ULT, index, last, "")?;
		let index = self
			.builder
			.build_select(inside, index, last, "")?
			.into_int_value();
		let offset =
			self.builder
				.build_int_nuw_mul(index, i64.const_int(length as u64, false), "")?;
		let bytes = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), base, &[offset], "literal")?
		};
		let value = self
			.builder
			.build_insert_value(self.string_type.get_undef(), bytes, 0, "")?;
		let value = self.builder.build_insert_value(
			value,
			self.usize.const_int(length as u64, false),
			1,
			"",
		)?;
		let value = self
			.builder
			.build_insert_value(value, self.static_capacity(&packed), 2, "")?;
		Ok(Some(Val::new(Repr::Str, value.into_struct_value())))
	}

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
		if repr == Repr::Str
			&& let Some(value) = self.packed_literal_choice(&list, subject)?
		{
			return Ok(value);
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
			let owned = slot.is_some();
			let arm = decision.value;
			let value = self.region(slot.map(|slot| slot.repr), arm, |this| {
				match this.view(arm) {
					View::Block { .. } => this.block(arm, owned),
					_ => this.expr(arm),
				}
			})?;
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
			Some(slot) => Ok(self.fresh(self.load(slot)?)),
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
				(Repr::Union | Repr::InlineUnion(_), [Test::IsType(ty)]) => {
					self.box_data(subject, *ty, binding)?
				}
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
			(Repr::Union, Repr::InlineUnion(nominal), _) => {
				self.boxed_to_inline(subject, nominal)?
			}
			(Repr::InlineUnion(a), Repr::InlineUnion(b), _) if a == b => subject,
			(Repr::InlineUnion(_), _, [Test::IsType(ty)]) => {
				self.box_data(subject, *ty, binding)?
			}
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
		let subject_node = match self.view(node) {
			View::When { subject, .. } => subject,
			View::Is { value, .. } => value,
			_ => node,
		};
		let subject_type = Some(self.type_of(subject_node));
		let subject = if self.compressed_union_link(subject.repr).is_some()
			&& ty != fa_analyzer::ids::TypeId::NONE
		{
			let repr = if matches!(subject.repr, Repr::Optional(_)) {
				Repr::Optional(Scalar::Union)
			} else {
				Repr::Union
			};
			self.convert(subject, repr, node.index())?
		} else {
			subject
		};
		let ty = &ty;
		{
			match subject.repr {
				Repr::Union | Repr::InlineUnion(_) => self.box_is_typed(subject, *ty, subject_type),
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
					} else if matches!(scalar, Scalar::InlineUnion(_)) {
						let member =
							self.box_is_typed(inner.expect("a union"), *ty, subject_type)?;
						Ok(self.builder.build_and(present, member, "")?)
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
						let member =
							self.box_is_typed(Val::new(Repr::Union, safe), *ty, subject_type)?;
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
				// a value that can't be `none` (an optional narrowed already)
				_ if *ty == fa_analyzer::ids::TypeId::NONE => {
					Ok(self.context.bool_type().const_zero())
				}
				_ => self.value_in_type(subject, *ty, node),
			}
		}
	}

	/// Whether a value that isn't boxed belongs to a type of its own kind:
	/// the alternatives of `1 | 2 | 10..`, a literal, a range of integers.
	fn value_in_type(
		&mut self,
		subject: Val<'ctx>,
		ty: fa_analyzer::ids::TypeId,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		use fa_analyzer::types::TypeKind;
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Union(list) => {
				let items: Vec<_> = types.list_items(list).to_vec();
				let mut result = self.context.bool_type().const_zero();
				for item in items {
					let holds = self.value_in_type(subject, item, node)?;
					result = self.builder.build_or(result, holds, "")?;
				}
				Ok(result)
			}
			TypeKind::StringLit(_) | TypeKind::BoolLit(_) | TypeKind::FloatLit(..)
				if subject.repr != Repr::Union =>
			{
				let text = self.session.display_type(ty);
				let constant = match types.kind(ty) {
					TypeKind::BoolLit(value) => Val::new(
						Repr::Bool,
						self.context.bool_type().const_int(value as u64, false),
					),
					TypeKind::StringLit(sym) => {
						let text = types.syms.resolve(sym).to_string();
						self.string_constant(&text)
					}
					_ => {
						let value: f64 = text.parse().unwrap_or(f64::NAN);
						Val::new(Repr::F64, self.context.f64_type().const_float(value))
					}
				};
				let subject = if matches!(subject.repr, Repr::F32) {
					self.convert(subject, Repr::F64, node.index())?
				} else {
					subject
				};
				self.values_equal(subject, constant, node)
			}
			TypeKind::Int(_) => {
				let (Some(range), Repr::Int(int)) = (types.as_int(ty).copied(), subject.repr)
				else {
					return Ok(self.context.bool_type().const_int(1, false));
				};
				let bits = int.bits;
				let (low, high) = if int.signed {
					(
						-(ethnum::I256::ONE << (bits - 1)),
						(ethnum::I256::ONE << (bits - 1)) - 1,
					)
				} else {
					(ethnum::I256::ZERO, (ethnum::I256::ONE << bits) - 1)
				};
				let value = subject.int();
				let mut result = self.context.bool_type().const_int(1, false);
				let (above, below) = if int.signed {
					(IntPredicate::SGE, IntPredicate::SLE)
				} else {
					(IntPredicate::UGE, IntPredicate::ULE)
				};
				if range.min > low {
					let bound = self.const_int(bits, range.min.min(high));
					let holds = self.builder.build_int_compare(above, value, bound, "")?;
					result = self.builder.build_and(result, holds, "")?;
				}
				if range.max < high {
					let bound = self.const_int(bits, range.max.max(low));
					let holds = self.builder.build_int_compare(below, value, bound, "")?;
					result = self.builder.build_and(result, holds, "")?;
				}
				Ok(result)
			}
			_ => Ok(self.context.bool_type().const_int(1, false)),
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
		let span = self.tree.spans[body.index()];
		let small_body = self
			.tree
			.spans
			.iter()
			.filter(|s| span.start <= s.start && s.end <= span.end)
			.take(201)
			.count() <= 200;
		// one version inside another would double the code again
		if self.tasks.asynchronous || self.versioned_characters || !small_body {
			return self.for_character_path(node, pattern, has_index, text, body, None);
		}
		self.versioned_characters = true;
		let lowered = self.for_character_versions(node, pattern, has_index, text, body);
		self.versioned_characters = false;
		lowered
	}

	/// The loop of `for_characters` twice: for a simple text, then another.
	fn for_character_versions(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		text: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// A character's byte length and SIMPLE flag must stay known throughout
		// the user body. LLVM's size-limited unswitching otherwise often keeps
		// the Unicode branch and the string-view tests in every iteration.
		let capacity = self.string_capacity(text)?;
		let simple = self.builder.build_int_compare(
			IntPredicate::SLT,
			capacity,
			self.usize.const_zero(),
			"",
		)?;
		let ascii = self.append_block("ascii_characters");
		let unicode = self.append_block("unicode_characters");
		let end = self.append_block("characters_finished");
		self.builder
			.build_conditional_branch(simple, ascii, unicode)?;
		self.builder.position_at_end(ascii);
		self.for_character_path(node, pattern, has_index, text, body, Some(true))?;
		self.branch_to(end)?;
		self.builder.position_at_end(unicode);
		self.for_character_path(node, pattern, has_index, text, body, Some(false))?;
		self.branch_to(end)?;
		self.builder.position_at_end(end);
		Ok(Val::void())
	}

	fn for_character_path(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		text: Val<'ctx>,
		body: NodeId,
		simple_path: Option<bool>,
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
		// one byte per character in a simple text, else the next boundary
		let capacity = self.string_capacity(text)?;
		let simple = match simple_path {
			Some(simple) => self.context.bool_type().const_int(simple as u64, false),
			None => self.builder.build_int_compare(
				IntPredicate::SLT,
				capacity,
				self.usize.const_zero(),
				"",
			)?,
		};
		let one_byte = self.append_block("simple_character");
		let cluster = self.append_block("cluster");
		let found = self.append_block("character_found");
		self.builder
			.build_conditional_branch(simple, one_byte, cluster)?;
		self.builder.position_at_end(one_byte);
		let after_byte =
			self.builder
				.build_int_nuw_add(start, self.usize.const_int(1, false), "")?;
		self.builder.build_unconditional_branch(found)?;
		self.builder.position_at_end(cluster);
		let next_boundary = self.runtime.function("fa_next_grapheme")?;
		let after_cluster = self
			.call_value(next_boundary, &[bytes.into(), length.into(), start.into()])?
			.into_int_value();
		let cluster_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(found)?;
		self.builder.position_at_end(found);
		let stop = self.builder.build_phi(self.usize, "stop")?;
		stop.add_incoming(&[(&after_byte, one_byte), (&after_cluster, cluster_end)]);
		let stop = stop.as_basic_value().into_int_value();
		let character_bytes = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), bytes, &[start], "")?
		};
		let character_length = self.builder.build_int_sub(stop, start, "")?;
		let pair = self.builder.build_insert_value(
			self.string_type.get_undef(),
			character_bytes,
			0,
			"",
		)?;
		let pair = self
			.builder
			.build_insert_value(pair, character_length, 1, "")?;
		// a view of the text's bytes: never freed, copied when stored;
		// simple when the text is
		let flag = match simple_path {
			Some(true) => self.simple_flag(),
			Some(false) => self.usize.const_zero(),
			None => self.builder.build_and(capacity, self.simple_flag(), "")?,
		};
		let view = self.builder.build_or(flag, self.string_kind_mask(), "")?;
		let pair = self.builder.build_insert_value(pair, view, 2, "")?;
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
		let mark = self.mark();
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: test,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// `for code in text.codePoints()`: the code points decoded in place,
	/// without the array (`text` the receiver of `codePoints`); with
	/// `bytes_only`, `for byte in text.bytes()`.
	pub(crate) fn for_code_points(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		text: Val<'ctx>,
		body: NodeId,
		bytes_only: bool,
	) -> Result<Val<'ctx>, Error> {
		let (bytes, length) = self.string_parts(text)?;
		let capacity = self.string_capacity(text)?;
		let simple = self.builder.build_int_compare(
			IntPredicate::SLT,
			capacity,
			self.usize.const_zero(),
			"",
		)?;
		let i64 = self.context.i64_type();
		let index_slot = match has_index
			.then(|| self.analysis.symbols.symbol_of_decl(node))
			.flatten()
		{
			Some(symbol) => Some(self.slot_of(symbol, node)?),
			None => None,
		};
		let position = self.alloca(Repr::Int(IntRepr::I64), "code_point_index")?;
		self.builder
			.build_store(position.pointer, i64.const_zero())?;
		let offset = self.alloca(Repr::Int(IntRepr::I64), "offset")?;
		self.builder.build_store(offset.pointer, i64.const_zero())?;
		let usize_repr = IntRepr {
			bits: self.usize.get_bit_width(),
			signed: false,
		};
		let variable: Option<Slot<'ctx>> =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => Some(self.slot_of(symbol, node)?),
				None => None,
			};
		let test = self.append_block("for_code_point");
		let body_block = self.append_block("for_code_point_body");
		let exit = self.append_block("end_for_code_point");
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
		let (code, code_repr) = if bytes_only {
			// the byte at the offset
			let address = unsafe {
				self.builder
					.build_in_bounds_gep(self.context.i8_type(), bytes, &[start], "")?
			};
			let byte = self
				.builder
				.build_load(self.context.i8_type(), address, "byte")?
				.into_int_value();
			let wide = self.int_to(start, usize_repr, IntRepr::I64)?;
			let after = self
				.builder
				.build_int_nuw_add(wide, i64.const_int(1, false), "")?;
			self.builder.build_store(offset.pointer, after)?;
			(
				byte,
				IntRepr {
					bits: 8,
					signed: false,
				},
			)
		} else {
			// The flag is invariant for the whole traversal: LLVM can unswitch
			// this into a byte loop and a decoding loop. The simple path needs
			// neither a UTF-8 branch per byte nor a second bounds check.
			let ascii = self.append_block("code_point_ascii");
			let decode = self.append_block("code_point_decode");
			let decoded = self.append_block("code_point_read");
			self.builder
				.build_conditional_branch(simple, ascii, decode)?;
			self.builder.position_at_end(ascii);
			let at = unsafe {
				self.builder
					.build_in_bounds_gep(self.context.i8_type(), bytes, &[start], "")?
			};
			let byte = self
				.builder
				.build_load(self.context.i8_type(), at, "")?
				.into_int_value();
			let byte = self
				.builder
				.build_int_z_extend(byte, self.context.i32_type(), "")?;
			let after = self
				.builder
				.build_int_add(start, self.usize.const_int(1, false), "")?;
			let after = self.int_to(after, usize_repr, IntRepr::I64)?;
			self.builder.build_store(offset.pointer, after)?;
			self.builder.build_unconditional_branch(decoded)?;
			self.builder.position_at_end(decode);
			let next = self.runtime.function("fa_next_code_point")?;
			let code = self
				.call_value(next, &[bytes.into(), length.into(), offset.pointer.into()])?
				.into_int_value();
			let decode_end = self.builder.get_insert_block().expect("the decoder");
			self.builder.build_unconditional_branch(decoded)?;
			self.builder.position_at_end(decoded);
			let joined = self
				.builder
				.build_phi(self.context.i32_type(), "code_point")?;
			joined.add_incoming(&[(&byte, ascii), (&code, decode_end)]);
			let code = joined.as_basic_value().into_int_value();
			(
				code,
				IntRepr {
					bits: 32,
					signed: false,
				},
			)
		};
		if let Some(variable) = variable {
			self.store(variable, Val::new(Repr::Int(code_repr), code), node)?;
		}
		let current = self.load(position)?.int();
		if let Some(index_slot) = index_slot {
			self.store(index_slot, Val::new(Repr::Int(IntRepr::I64), current), node)?;
		}
		let following = self
			.builder
			.build_int_nsw_add(current, i64.const_int(1, false), "")?;
		self.builder.build_store(position.pointer, following)?;
		let mark = self.mark();
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: test,
			value: None,
			mark,
		});
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}
}
