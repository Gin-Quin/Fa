//! `Fields` values (types brief §3.9, `types/fields.md`): an unsigned
//! integer of the smallest width holding one bit per flag (8, 16, 32, 64 or
//! 128 bits: a `Fields` type has at most 128 flags), `Repr::Int`. The
//! analysis gives the bits of every literal as a compile-time value; `+` is
//! `or`, `-` is `and not`, `x is in s` is `(s & x) == x`, `==` and the
//! conversions are the integer ones. `toStringArray` and printing are
//! unrolled over the flags, which the type knows.

use ethnum::I256;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{ArithOp, NodeList, View};
use fa_analyzer::types::TypeKind;
use fa_analyzer::types::store::fields_mask;
use inkwell::IntPredicate;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

impl<'a, 'ctx> Lowering<'a, 'ctx> {
	/// Whether a type is a `Fields` type.
	pub(crate) fn is_fields(&self, ty: TypeId) -> bool {
		matches!(self.types().kind(ty), TypeKind::Fields(_))
	}

	/// Whether a node's value is a set of flags, which prints its flags,
	/// not a number.
	pub(crate) fn prints_flags(&self, node: NodeId) -> bool {
		self.is_fields(self.concrete_type(self.type_of(node)))
	}

	/// The `Fields` type of a printed node: its type, or the type of its
	/// value when it is optional.
	pub(crate) fn printed_fields_type(&self, node: NodeId) -> Option<TypeId> {
		if node.index() == 0 {
			return None;
		}
		let ty = self.concrete_type(self.type_of(node));
		let types = self.types();
		types
			.union_members(&ty)
			.iter()
			.copied()
			.find(|&member| self.is_fields(member))
			.or_else(|| self.is_fields(ty).then_some(ty))
	}

	/// Appends a value of type `ty` nested in a composite: a set of flags
	/// prints its flags.
	pub(crate) fn append_nested_of(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		if self.is_fields(ty) && self.fields_repr(ty) == value.repr {
			self.append_fields(value, ty, node)
		} else {
			self.append_nested(value, node)
		}
	}

	/// The representation of a `Fields` type.
	pub(crate) fn fields_repr(&self, ty: TypeId) -> Repr {
		Repr::Int(IntRepr {
			bits: self.types().fields_storage_bits(ty),
			signed: false,
		})
	}

	/// A constant of the width of a `Fields` value.
	fn fields_constant(&self, value: IntValue<'ctx>, bits: u128) -> IntValue<'ctx> {
		self.const_int(value.get_type().get_bit_width(), I256::from(bits))
	}

	/// `a + b + …` (union) and `a - b - …` (difference) of field sets.
	pub(crate) fn fields_arithmetic(
		&mut self,
		node: NodeId,
		op: ArithOp,
		operands: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.fields_repr(self.concrete_type(self.type_of(node)));
		let mut result: Option<Val<'ctx>> = None;
		for operand in operands.iter() {
			let value = self.expr(operand)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			result = Some(match result {
				None => value,
				Some(acc) => self.fields_step(op, acc, value)?,
			});
		}
		let Some(result) = result else {
			return Err(self.unsupported(node.index(), "this operation"));
		};
		Ok(Val::new(repr, result.int()))
	}

	/// One step of `+` (`or`) or `-` (`and not`) on two field sets.
	pub(crate) fn fields_step(
		&mut self,
		op: ArithOp,
		left: Val<'ctx>,
		right: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let (raw_left, raw_right) = (left.int(), right.int());
		let result = if op == ArithOp::Add {
			self.builder.build_or(raw_left, raw_right, "")?
		} else {
			let inverse = self.builder.build_not(raw_right, "")?;
			self.builder.build_and(raw_left, inverse, "")?
		};
		Ok(Val::new(left.repr, result))
	}

	/// `value is in set`: every flag of `value` is in `set`.
	pub(crate) fn fields_is_in(
		&mut self,
		value: Val<'ctx>,
		set: Val<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let (value, set) = (value.int(), set.int());
		let common = self.builder.build_and(set, value, "")?;
		Ok(self
			.builder
			.build_int_compare(IntPredicate::EQ, common, value, "")?)
	}

	/// The `Fields` type of the receiver of a method call (`set.toStringArray()`).
	fn fields_receiver_type(&self, call: NodeId) -> Option<TypeId> {
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let receiver = match members.len() {
			0 => return None,
			1 => target,
			count => members.get(count - 2)?,
		};
		let ty = self.concrete_type(self.type_of(receiver));
		self.is_fields(ty).then_some(ty)
	}

	/// `set.toStringArray()`: the dotted paths of its flags, in bit order.
	pub(crate) fn fields_to_string_array(&mut self, call: NodeId) -> Result<Val<'ctx>, Error> {
		let Some(ty) = self.fields_receiver_type(call) else {
			return Err(self.unsupported(call.index(), "`toStringArray`"));
		};
		let value = self.receiver(call)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let bits = value.int();
		let i64 = self.context.i64_type();
		let count = self.count_ones(bits)?;
		let length = self
			.builder
			.build_int_cast_sign_flag(count, i64, false, "")?;
		let array = self.new_array(Repr::Str, length, length)?;
		let (items, _) = self.array_parts(array)?;
		for (index, path) in self.types().fields_leaves(ty).iter().enumerate() {
			let bit = self.fields_constant(bits, 1u128 << index);
			let present = self.flag_set(bits, bit)?;
			let then = self.append_block("fields.flag");
			let next = self.append_block("fields.next");
			self.builder.build_conditional_branch(present, then, next)?;
			self.builder.position_at_end(then);
			// its position: the flags before it
			let below = self.fields_constant(bits, fields_mask(index as u32));
			let before = self.builder.build_and(bits, below, "")?;
			let position = self.count_ones(before)?;
			let position = self
				.builder
				.build_int_cast_sign_flag(position, i64, false, "")?;
			let slot = unsafe {
				self.builder
					.build_in_bounds_gep(self.string_type, items, &[position], "")?
			};
			let name = self.string_constant(path);
			self.builder
				.build_store(slot, name.value.expect("a string"))?;
			self.builder.build_unconditional_branch(next)?;
			self.builder.position_at_end(next);
		}
		Ok(self.fresh(Val::new(Repr::Array(TypeId::STRING), array)))
	}

	/// The number of bits set.
	fn count_ones(&self, bits: IntValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let ctpop = self.intrinsic("llvm.ctpop", &[bits.get_type().into()])?;
		Ok(self
			.builder
			.build_call(ctpop, &[bits.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a count")
			.into_int_value())
	}

	/// Whether `bits` has one of the bits of `mask`.
	fn flag_set(
		&self,
		bits: IntValue<'ctx>,
		mask: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let common = self.builder.build_and(bits, mask, "")?;
		Ok(self.builder.build_int_compare(
			IntPredicate::NE,
			common,
			mask.get_type().const_zero(),
			"",
		)?)
	}

	/// Appends a set as a key set: `{ read, nested { a } }`, `{}` when empty.
	pub(crate) fn append_fields(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		let bits = value.int();
		let empty = self.builder.build_int_compare(
			IntPredicate::EQ,
			bits,
			bits.get_type().const_zero(),
			"",
		)?;
		let none = self.append_block("fields.empty");
		let some = self.append_block("fields.some");
		let end = self.append_block("fields.end");
		self.builder.build_conditional_branch(empty, none, some)?;
		self.builder.position_at_end(none);
		let text = self.string_constant("{}");
		self.append_value(text, node)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(some);
		self.append_fields_level(bits, ty, 0, node, 0)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		Ok(())
	}

	/// `{ a, g { b } }`: the flags of one level, whose first bit is `offset`
	/// (at least one is set). A separator precedes a flag when a flag before
	/// it on its level is set.
	fn append_fields_level(
		&mut self,
		bits: IntValue<'ctx>,
		level: TypeId,
		offset: u32,
		node: NodeId,
		depth: u32,
	) -> Result<(), Error> {
		if depth > 32 {
			return Err(self.unsupported(node.index(), "printing this value"));
		}
		let open = self.string_constant("{ ");
		self.append_value(open, node)?;
		let mut before: u128 = 0;
		let mut position = offset;
		for (name, children) in self.types().fields_keys(level) {
			let width = if children.is_unset() {
				1
			} else {
				self.types().fields_width(children)
			};
			let own = fields_mask(width).checked_shl(position).unwrap_or(0);
			let mask = self.fields_constant(bits, own);
			let present = self.flag_set(bits, mask)?;
			let then = self.append_block("fields.flag");
			let next = self.append_block("fields.next");
			self.builder.build_conditional_branch(present, then, next)?;
			self.builder.position_at_end(then);
			if before != 0 {
				let earlier = self.fields_constant(bits, before);
				let separated = self.flag_set(bits, earlier)?;
				let comma = self.append_block("fields.comma");
				let name_block = self.append_block("fields.name");
				self.builder
					.build_conditional_branch(separated, comma, name_block)?;
				self.builder.position_at_end(comma);
				let separator = self.string_constant(", ");
				self.append_value(separator, node)?;
				self.builder.build_unconditional_branch(name_block)?;
				self.builder.position_at_end(name_block);
			}
			let text = if children.is_unset() {
				name
			} else {
				format!("{name} ")
			};
			let text = self.string_constant(&text);
			self.append_value(text, node)?;
			if !children.is_unset() {
				self.append_fields_level(bits, children, position, node, depth + 1)?;
			}
			self.builder.build_unconditional_branch(next)?;
			self.builder.position_at_end(next);
			before |= own;
			position += width;
		}
		let close = self.string_constant(" }");
		self.append_value(close, node)
	}

	/// The `Fields` type and the bits of a key set bound to a key-set
	/// parameter (`Tuple(Fields type, bits)`).
	pub(crate) fn key_set_bits(&self, literal: TypeId) -> Option<(TypeId, I256)> {
		let types = self.types();
		let TypeKind::Tuple(list) = types.kind(literal) else {
			return None;
		};
		let [fields, bits] = types.list_items(list) else {
			return None;
		};
		if !self.is_fields(*fields) {
			return None;
		}
		let bits = types.as_int(*bits).and_then(|int| int.literal())?;
		Some((*fields, bits))
	}

	/// The keys of an `@for` over a key set and the analyses of its body
	/// for each: of the `static` key set, or of the application of the
	/// key-set parameter the instance being lowered stands for.
	pub(crate) fn key_loop_analyses(
		&self,
		node: NodeId,
		iterable: NodeId,
	) -> Option<(
		Vec<String>,
		&'a [std::sync::Arc<fa_analyzer::analysis::Analysis>],
	)> {
		let main: &'a fa_analyzer::analysis::Analysis = self.analysis;
		let instances = main.key_loop_bodies.get(&node)?;
		let instance = match instances.as_slice() {
			[instance] if instance.param.is_none() => instance,
			_ => {
				let Resolution::Local(symbol) = main.resolution(iterable) else {
					return None;
				};
				let literal = self.value_bindings.borrow().get(&symbol).copied()?;
				instances.iter().find(|instance| {
					instance.key_set == literal
						&& instance.param.is_none_or(|param| {
							let repr = self.bindings.borrow().get(&param).copied();
							repr.and_then(|repr| {
								self.instance_types.borrow().get(&(param, repr)).copied()
							})
							.is_none_or(|subject| subject == instance.subject)
						})
				})?
			}
		};
		Some((instance.keys.clone(), instance.analyses.as_slice()))
	}

	/// `value[key]`, `key` the variable of an `@for` over a key set: the
	/// field of `value` at the key's path, a view of it. The read's type
	/// parameter stands for the field's type while the key is lowered.
	pub(crate) fn key_read(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if self.is_place(target) {
			let slot = self.key_place(node, target, index)?;
			return self.load(slot);
		}
		let key = self.current_key(node, index)?;
		let mut value = self.expr(target)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		for name in key.split('.') {
			if let Repr::Rec(rec) = value.repr {
				value = self.unfold_rec(value, rec, node.index())?;
			}
			let Repr::Object(object) = value.repr else {
				return Err(self.unsupported(node.index(), "this key of a key set"));
			};
			let types = self.types();
			let position = types
				.fields_of(object)
				.iter()
				.position(|field| types.syms.resolve(field.name) == name);
			let Some(position) = position else {
				return Err(self.unsupported(node.index(), "this key of a key set"));
			};
			let repr = self.field_repr(object, position, node.index())?;
			let structure = value.value.expect("an object").into_struct_value();
			let raw = self
				.builder
				.build_extract_value(structure, position as u32, "")?;
			value = Val::new(repr, raw);
		}
		Ok(value)
	}

	/// Borrowing a keyed field uses the same fact as an ordinary read.
	pub(crate) fn key_place(
		&mut self,
		node: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Result<super::Slot<'ctx>, Error> {
		let key = self.current_key(node, index)?;
		let mut slot = self.place(target)?;
		for name in key.split('.') {
			if let Repr::Rec(target) = slot.repr {
				slot = super::Slot {
					pointer: self.load_edge(slot)?,
					repr: self.repr_of(target, node.index())?,
				};
			}
			let Repr::Object(object) = slot.repr else {
				return Err(self.unsupported(node.index(), "this key of a key set"));
			};
			let position = self
				.types()
				.fields_of(object)
				.iter()
				.position(|field| self.types().syms.resolve(field.name) == name)
				.ok_or_else(|| self.unsupported(node.index(), "this key of a key set"))?;
			let structure = self.object_type(object);
			slot = super::Slot {
				pointer: self.builder.build_struct_gep(
					structure,
					slot.pointer,
					position as u32,
					"",
				)?,
				repr: self.field_repr(object, position, node.index())?,
			};
		}
		Ok(slot)
	}

	fn current_key(&self, node: NodeId, index: NodeId) -> Result<String, Error> {
		match self.analysis.resolution(index) {
			Resolution::Local(symbol) => self.current_keys.get(&symbol).cloned(),
			_ => None,
		}
		.ok_or_else(|| self.unsupported(node.index(), "this key of a key set"))
	}

	/// Forgets the slots of the locals declared in the body of a key loop:
	/// each key's analysis gives them their own types.
	pub(crate) fn forget_body_slots(&mut self, body: NodeId) {
		let span = self.tree.spans[body.index()];
		let inside: Vec<fa_analyzer::ids::SymbolId> = self
			.analysis
			.symbols
			.symbols()
			.iter()
			.enumerate()
			.filter(|(_, symbol)| {
				let declared = self.tree.spans[symbol.decl_node.index()];
				span.start <= declared.start && declared.end <= span.end
			})
			.map(|(index, _)| fa_analyzer::ids::SymbolId::new(index))
			.collect();
		for symbol in inside {
			self.slots.remove(&symbol);
		}
	}
}
