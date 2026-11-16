//! Explicit copies borrow their input and build a wholly owned result.
//! Source/target helpers are declared before their bodies, so recursive
//! conversions emit runtime calls rather than recurse during code generation.
//! These builders never consult the caller's frame or temporary registry.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::{CollKind, TypeKind};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};

use super::Lowering;
use super::values::{Repr, Val};
use crate::error::Error;

#[derive(Clone, Copy)]
struct CopyShape {
	ty: TypeId,
	repr: Repr,
}

#[derive(Clone, Copy)]
pub(super) struct CopyPair {
	from: CopyShape,
	to: CopyShape,
}

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn explicit_copy(
		&mut self,
		value: Val<'ctx>,
		source: TypeId,
		target: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let source = self.concrete_type(source);
		let target = self.concrete_type(target);
		let to = self.repr_of(target, node.index())?;
		let pair = CopyPair {
			from: CopyShape {
				ty: source,
				repr: value.repr,
			},
			to: CopyShape {
				ty: target,
				repr: to,
			},
		};
		let Some(raw) = value.value else {
			return Ok(Val::new(to, super::zero_of(self.llvm_type(to))));
		};
		let copied = self.call_converting_copy(&self.builder, pair, raw)?;
		self.finish_converting_copies()?;
		Ok(self.fresh(Val::new(to, copied)))
	}

	fn call_converting_copy(
		&self,
		builder: &Builder<'ctx>,
		pair: CopyPair,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if matches!(pair.from.repr, Repr::Never | Repr::Void) {
			return Ok(super::zero_of(self.llvm_type(pair.to.repr)));
		}
		if pair.from.ty == pair.to.ty && pair.from.repr == pair.to.repr {
			return self.call_copy(builder, pair.from.repr, value);
		}
		let function = self.converting_copy_function(pair)?;
		Ok(builder
			.build_call(function, &[value.into()], "copied")?
			.try_as_basic_value()
			.basic()
			.expect("a copied value"))
	}

	fn converting_copy_function(&self, pair: CopyPair) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!(
			"convert:{:?}:{}->{:?}:{}",
			pair.from.ty,
			self.shape_key(pair.from.repr),
			pair.to.ty,
			self.shape_key(pair.to.repr)
		);
		if let Some(function) = self.copy_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let from = self.llvm_type(pair.from.repr);
		let to = self.llvm_type(pair.to.repr);
		let (function, _builder) =
			self.helper_function("fa.copy.convert", to.fn_type(&[from.into()], false));
		self.copy_functions.borrow_mut().insert(key, function);
		self.pending_converting_copies
			.borrow_mut()
			.push((pair, function));
		Ok(function)
	}

	fn finish_converting_copies(&mut self) -> Result<(), Error> {
		loop {
			let pending = self.pending_converting_copies.borrow_mut().pop();
			let Some((pair, function)) = pending else {
				return Ok(());
			};
			let builder = self.context.create_builder();
			builder.position_at_end(function.get_first_basic_block().expect("an entry"));
			let value = function.get_first_param().expect("the borrowed source");
			let arena = self.arena_repr(pair.to.repr);
			if arena {
				builder.build_call(self.runtime.function("fa_arena_enter")?, &[], "")?;
			}
			let copied = self.build_converting_copy(&builder, function, pair, value)?;
			if arena {
				let root = self.tree_root_pointer(&builder, pair.to.repr, copied)?;
				builder.build_call(self.runtime.function("fa_arena_leave")?, &[root.into()], "")?;
			}
			builder.build_return(Some(&copied))?;
		}
	}

	fn build_converting_copy(
		&mut self,
		b: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pair: CopyPair,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let CopyPair { from, to } = pair;
		if from.repr == to.repr && !matches!(to.repr, Repr::Union | Repr::InlineUnion(_)) {
			return self.call_copy(b, from.repr, value);
		}
		if let Repr::Optional(target) = to.repr {
			if from.repr == Repr::None {
				return Ok(super::zero_of(self.llvm_type(to.repr)));
			}
			let (present, input) = match from.repr {
				Repr::Optional(_) => self.split_optional(b, Val::new(from.repr, value))?,
				_ => (
					self.context.bool_type().const_int(1, false),
					Val::new(from.repr, value),
				),
			};
			let start = b.get_insert_block().expect("a block");
			let some = self.context.append_basic_block(function, "some");
			let join = self.context.append_basic_block(function, "optional_copy");
			b.build_conditional_branch(present, some, join)?;
			b.position_at_end(some);
			let inner = CopyPair {
				from: CopyShape {
					ty: self.without_none(from.ty),
					repr: input.repr,
				},
				to: CopyShape {
					ty: self.without_none(to.ty),
					repr: target.repr(),
				},
			};
			let copied =
				self.call_converting_copy(b, inner, input.value.expect("a present value"))?;
			let wrapped = self.pack_optional(b, target, present, copied)?;
			let end = b.get_insert_block().expect("a block");
			b.build_unconditional_branch(join)?;
			b.position_at_end(join);
			let phi = b.build_phi(self.llvm_type(to.repr), "")?;
			phi.add_incoming(&[
				(&super::zero_of(self.llvm_type(to.repr)), start),
				(&wrapped, end),
			]);
			return Ok(phi.as_basic_value());
		}
		if let Repr::Rec(source) = from.repr {
			let address = self.edge_address(b, from.repr, value)?;
			let repr = self.repr_of(source, 0)?;
			let raw = b.build_load(self.llvm_type(repr), address, "source_node")?;
			return self.call_converting_copy(
				b,
				CopyPair {
					from: CopyShape { ty: source, repr },
					to,
				},
				raw,
			);
		}
		if let Repr::Rec(target) = to.repr {
			let repr = self.repr_of(target, 0)?;
			let copied = self.call_converting_copy(
				b,
				CopyPair {
					from,
					to: CopyShape { ty: target, repr },
				},
				value,
			)?;
			let block = if self.arena_object(target) {
				self.allocate_tree_node(b, repr)?
			} else {
				self.allocate_for_with(b, self.llvm_type(repr))?
			};
			b.build_store(block, copied)?;
			return self.address_edge(b, to.repr, block);
		}
		if let Some(converted) = self.scalar_conversion(b, Val::new(from.repr, value), to.repr)? {
			return Ok(converted.value.expect("a scalar"));
		}
		if matches!(from.repr, Repr::Union | Repr::InlineUnion(_)) {
			if self.preserves_box_alternative(from.ty, to.ty) {
				return self.copy_union_representation(b, from.repr, to.repr, value);
			}
			if let Some((nominal, index)) = self.member_of_type(from.ty)
				&& let Some(payload) = self.member_data(nominal, index)
			{
				let pointer = self.copy_union_pointer(b, from.repr, value)?;
				let repr = self.repr_of(payload, 0)?;
				let raw = self.member_payload_value(b, pointer, nominal, index, repr)?;
				return self.call_converting_copy(
					b,
					CopyPair {
						from: CopyShape { ty: payload, repr },
						to,
					},
					raw,
				);
			}
			return self.build_union_converting_copy(b, function, pair, value);
		}
		if matches!(to.repr, Repr::Union | Repr::InlineUnion(_)) {
			if let Some(member) = self.copy_literal_member(b, from.ty, to)? {
				return Ok(member);
			}
			let chosen = self.copy_alternative(from.ty, to.ty)?;
			let repr = self.repr_of(chosen, 0)?;
			let copied = self.call_converting_copy(
				b,
				CopyPair {
					from,
					to: CopyShape { ty: chosen, repr },
				},
				value,
			)?;
			let id = self
				.context
				.i64_type()
				.const_int(self.type_id(chosen), false);
			let layout = self.box_type(Some(repr));
			let pointer = self.allocate_for_with(b, layout.into())?;
			let tag = b.build_struct_gep(layout, pointer, 0, "")?;
			b.build_store(tag, id)?;
			let payload = b.build_struct_gep(layout, pointer, 1, "")?;
			b.build_store(payload, copied)?;
			return Ok(pointer.into());
		}

		match (from.repr, to.repr) {
			(Repr::Object(source), Repr::Object(target)) => {
				let source_fields = self.types().fields_of(source).to_vec();
				let target_fields = self.types().fields_of(target).to_vec();
				let mut result = self.object_type(target).get_undef();
				for (index, field) in target_fields.iter().enumerate() {
					let position = source_fields
						.iter()
						.position(|input| {
							self.types().syms.resolve(input.name)
								== self.types().syms.resolve(field.name)
						})
						.ok_or_else(|| {
							Error::Llvm("a converting copy lost a retained field".into())
						})?;
					let from = CopyShape {
						ty: source_fields[position].ty,
						repr: self.field_repr(source, position, 0)?,
					};
					let to = CopyShape {
						ty: field.ty,
						repr: self.field_repr(target, index, 0)?,
					};
					let raw =
						b.build_extract_value(value.into_struct_value(), position as u32, "")?;
					let copied = self.call_converting_copy(b, CopyPair { from, to }, raw)?;
					result = b
						.build_insert_value(result, copied, index as u32, "")?
						.into_struct_value();
				}
				Ok(result.into())
			}
			(Repr::Tuple(source), Repr::Tuple(target)) => {
				let source_items = self.tuple_items(source);
				let target_items = self.tuple_items(target);
				let mut result = self.tuple_type(target).get_undef();
				for (index, (&source, &target)) in
					source_items.iter().zip(&target_items).enumerate()
				{
					let from = CopyShape {
						ty: source,
						repr: self.repr_of(source, 0)?,
					};
					let to = CopyShape {
						ty: target,
						repr: self.repr_of(target, 0)?,
					};
					let raw = b.build_extract_value(value.into_struct_value(), index as u32, "")?;
					let copied = self.call_converting_copy(b, CopyPair { from, to }, raw)?;
					result = b
						.build_insert_value(result, copied, index as u32, "")?
						.into_struct_value();
				}
				Ok(result.into())
			}
			(Repr::Array(_) | Repr::Growing(_), Repr::Array(_) | Repr::Growing(_)) => {
				self.build_sequence_copy(b, function, pair, value.into_pointer_value())
			}
			(
				Repr::Pool(source) | Repr::Columnar(source),
				Repr::Pool(target) | Repr::Columnar(target),
			) => {
				let from_item = self.item_repr(source, NodeId::new(0))?;
				let to_item = self.item_repr(target, NodeId::new(0))?;
				let from_place = self.pool_place(from.repr, NodeId::new(0))?;
				let to_place = self.pool_place(to.repr, NodeId::new(0))?;
				let result = self.collection_new(b, to.repr, to_item)?;
				self.helper_pool_each(b, function, value.into_pointer_value(), |b, address| {
					let raw = self.element_read(b, from_place, address)?;
					let copied = self.call_converting_copy(
						b,
						CopyPair {
							from: CopyShape {
								ty: source,
								repr: from_item,
							},
							to: CopyShape {
								ty: target,
								repr: to_item,
							},
						},
						raw,
					)?;
					let slot = b
						.build_call(
							self.runtime.function("fa_pool_add_slot")?,
							&[result.into()],
							"",
						)?
						.try_as_basic_value()
						.basic()
						.expect("a slot")
						.into_pointer_value();
					self.element_write(b, to_place, slot, copied)
				})?;
				Ok(result.into())
			}
			(Repr::Map(_) | Repr::Set(_), Repr::Map(_) | Repr::Set(_)) => {
				self.build_converting_table(b, function, pair, value.into_pointer_value())
			}
			_ => Err(Error::Llvm(format!(
				"converting copy from {:?} to {:?}",
				from.repr, to.repr
			))),
		}
	}

	fn build_sequence_copy(
		&self,
		b: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pair: CopyPair,
		input: PointerValue<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let (Repr::Array(source) | Repr::Growing(source)) = pair.from.repr else {
			unreachable!()
		};
		let (Repr::Array(target) | Repr::Growing(target)) = pair.to.repr else {
			unreachable!()
		};
		let from = CopyShape {
			ty: source,
			repr: self.item_repr(source, NodeId::new(0))?,
		};
		let to = CopyShape {
			ty: target,
			repr: self.item_repr(target, NodeId::new(0))?,
		};
		let source_grows = matches!(pair.from.repr, Repr::Growing(_));
		let target_grows = matches!(pair.to.repr, Repr::Growing(_));
		let (items, length) = if source_grows {
			(
				self.pointer_type().const_null(),
				self.growing_length(b, input)?,
			)
		} else {
			self.helper_array_parts(b, input)?
		};
		let (size, bits) = self.segment_arguments(to.repr);
		let result = if target_grows {
			let result = b
				.build_call(self.runtime.function("fa_segmented_new")?, &[], "")?
				.try_as_basic_value()
				.basic()
				.expect("a growing array")
				.into_pointer_value();
			b.build_call(
				self.runtime.function("fa_segmented_reserve")?,
				&[result.into(), length.into(), size.into(), bits.into()],
				"",
			)?;
			result
		} else {
			b.build_call(
				self.runtime.function("fa_array_new")?,
				&[
					length.into(),
					length.into(),
					self.item_size_const(to.repr).into(),
				],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an array")
			.into_pointer_value()
		};
		let output = if target_grows {
			self.pointer_type().const_null()
		} else {
			self.helper_array_parts(b, result)?.0
		};
		self.helper_loop(b, function, length, |b, index| {
			let address = if source_grows {
				self.growing_at(b, input, from.repr, index)?
			} else {
				unsafe { b.build_in_bounds_gep(self.llvm_type(from.repr), items, &[index], "")? }
			};
			let raw = b.build_load(self.llvm_type(from.repr), address, "")?;
			let copied = self.call_converting_copy(b, CopyPair { from, to }, raw)?;
			let slot = if target_grows {
				b.build_call(
					self.runtime.function("fa_segmented_push_slot")?,
					&[result.into(), size.into(), bits.into()],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a slot")
				.into_pointer_value()
			} else {
				unsafe { b.build_in_bounds_gep(self.llvm_type(to.repr), output, &[index], "")? }
			};
			b.build_store(slot, copied)?;
			Ok(())
		})?;
		Ok(result.into())
	}

	fn build_converting_table(
		&mut self,
		b: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pair: CopyPair,
		input: PointerValue<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let from = self.table_of(pair.from.repr)?;
		let to = self.table_of(pair.to.repr)?;
		let parts = |repr| -> Result<_, Error> {
			let (Repr::Map(ty) | Repr::Set(ty)) = repr else {
				unreachable!()
			};
			let TypeKind::Collection(kind, item, key) = self.types().kind(ty) else {
				return Err(Error::Llvm("a table without a semantic type".into()));
			};
			Ok(if matches!(kind, CollKind::Map | CollKind::GrowingMap) {
				(key, Some(item))
			} else {
				(item, None)
			})
		};
		let (from_key, from_value) = parts(pair.from.repr)?;
		let (to_key, to_value) = parts(pair.to.repr)?;
		let key_pair = CopyPair {
			from: CopyShape {
				ty: from_key,
				repr: from.key,
			},
			to: CopyShape {
				ty: to_key,
				repr: to.key,
			},
		};
		let bitwise = if to.key == Repr::Str {
			u64::MAX
		} else {
			Self::bitwise_key_size(to)
		};
		let equals = if bitwise > 0 {
			self.pointer_type().const_null()
		} else {
			self.equals_function(to.key)?
				.as_global_value()
				.as_pointer_value()
		};
		let hash = self.hash_function(to.key)?;
		let hash_at = self
			.hash_at_function(to.key)?
			.as_global_value()
			.as_pointer_value();
		let i64 = self.context.i64_type();
		let result = b
			.build_call(
				self.runtime.function("fa_table_new")?,
				&[i64.const_zero().into(), self.entry_size(to).into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a table")
			.into_pointer_value();
		let key_slot = b.build_alloca(self.llvm_type(to.key), "converted_key")?;
		self.helper_entries(b, function, input, from, |b, entry| {
			let field = b.build_struct_gep(from.entry, entry, 0, "")?;
			let raw = b.build_load(self.llvm_type(from.key), field, "")?;
			let key = self.call_converting_copy(b, key_pair, raw)?;
			let copied_value = match (from_value, to_value, from.value, to.value) {
				(Some(source), Some(target), Some(from_repr), Some(to_repr)) => {
					let field = b.build_struct_gep(from.entry, entry, 1, "")?;
					let raw = b.build_load(self.llvm_type(from_repr), field, "")?;
					Some(self.call_converting_copy(
						b,
						CopyPair {
							from: CopyShape {
								ty: source,
								repr: from_repr,
							},
							to: CopyShape {
								ty: target,
								repr: to_repr,
							},
						},
						raw,
					)?)
				}
				_ => None,
			};
			b.build_store(key_slot, key)?;
			let key_hash = b
				.build_call(hash, &[key.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("a hash");
			let probe = b
				.build_call(
					self.runtime.function("fa_table_find")?,
					&[
						result.into(),
						key_hash.into(),
						key_slot.into(),
						equals.into(),
						self.entry_size(to).into(),
						i64.const_int(bitwise, false).into(),
					],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a probe")
				.into_int_value();
			let found = b.build_int_compare(IntPredicate::SGE, probe, i64.const_zero(), "")?;
			let present = self.context.append_basic_block(function, "existing_key");
			let absent = self.context.append_basic_block(function, "new_key");
			let end = self.context.append_basic_block(function, "inserted_key");
			b.build_conditional_branch(found, present, absent)?;
			b.position_at_end(present);
			if let (Some(repr), Some(value)) = (to.value, copied_value) {
				let old = b
					.build_call(
						self.runtime.function("fa_table_entry")?,
						&[result.into(), probe.into(), self.entry_size(to).into()],
						"",
					)?
					.try_as_basic_value()
					.basic()
					.expect("an entry")
					.into_pointer_value();
				let field = b.build_struct_gep(to.entry, old, 1, "")?;
				let previous = b.build_load(self.llvm_type(repr), field, "")?;
				self.call_drop(b, repr, previous)?;
				b.build_store(field, value)?;
			}
			self.call_drop(b, to.key, key)?;
			b.build_unconditional_branch(end)?;
			b.position_at_end(absent);
			let key_bits = self.table_key_bits(b, to, Val::new(to.key, key))?;
			let entry = b
				.build_call(
					self.runtime.function("fa_table_append")?,
					&[
						result.into(),
						key_hash.into(),
						self.entry_size(to).into(),
						probe.into(),
						key_bits.into(),
						hash_at.into(),
					],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a new entry")
				.into_pointer_value();
			let field = b.build_struct_gep(to.entry, entry, 0, "")?;
			b.build_store(field, key)?;
			if let Some(value) = copied_value {
				let field = b.build_struct_gep(to.entry, entry, 1, "")?;
				b.build_store(field, value)?;
			}
			b.build_unconditional_branch(end)?;
			b.position_at_end(end);
			Ok(())
		})?;
		Ok(result.into())
	}

	/// Select the same compatible target alternative as the semantic Copy
	/// relation, preserving a written member when it is already present.
	fn copy_alternative(&self, source: TypeId, target: TypeId) -> Result<TypeId, Error> {
		fa_analyzer::lower::copies::copy_destination(
			self.types(),
			self.session.module_id(self.file),
			source,
			target,
		)
		.ok_or_else(|| {
			Error::Llvm(format!(
				"a converting copy from {} to {} has no compatible union alternative",
				self.types().display(source),
				self.types().display(target)
			))
		})
	}

	fn preserves_box_alternative(&self, source: TypeId, target: TypeId) -> bool {
		fa_analyzer::lower::copies::preserves_copy_alternative(
			self.types(),
			self.session.module_id(self.file),
			source,
			target,
		)
	}

	fn copy_union_pointer(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		if matches!(repr, Repr::InlineUnion(_)) {
			self.inline_union_slot(b, self.freeze_union_padding(b, repr, value))
		} else {
			Ok(value.into_pointer_value())
		}
	}

	fn copy_union_representation(
		&self,
		b: &Builder<'ctx>,
		from: Repr,
		to: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if from == to {
			return self.call_copy(b, from, value);
		}
		let pointer = self.copy_union_pointer(b, from, value)?;
		if let Repr::InlineUnion(nominal) = from {
			return self.copy_inline_box(b, nominal, value);
		}
		if let Repr::InlineUnion(nominal) = to {
			return self.copy_box_to_inline(b, nominal, pointer);
		}
		self.call_copy(b, Repr::Union, pointer.into())
	}

	fn build_union_converting_copy(
		&self,
		b: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pair: CopyPair,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let source = self
			.types()
			.rec_target(pair.from.ty)
			.unwrap_or(pair.from.ty);
		let pointer = self.copy_union_pointer(b, pair.from.repr, value)?;
		let id = self.union_id(b, pointer)?;
		let otherwise = self
			.context
			.append_basic_block(function, "invalid_alternative");
		let join = self
			.context
			.append_basic_block(function, "copied_alternative");
		let start = b.get_insert_block().expect("a block");
		b.position_at_end(join);
		let result = b.build_phi(self.llvm_type(pair.to.repr), "")?;
		let mut cases = Vec::new();
		for alternative in fa_analyzer::lower::copies::copy_alternatives(
			self.types(),
			self.session.module_id(self.file),
			source,
		) {
			let tag = match alternative.member {
				Some((nominal, index)) => self.member_id(nominal, index),
				None => self.type_id(alternative.ty),
			};
			let block = self
				.context
				.append_basic_block(function, "copy_alternative");
			b.position_at_end(block);
			let copied = if let Some((nominal, index)) = alternative.member {
				if self.preserves_box_alternative(alternative.ty, pair.to.ty) {
					self.copy_union_representation(b, pair.from.repr, pair.to.repr, value)?
				} else {
					let payload = self.member_data(nominal, index).ok_or_else(|| {
						Error::Llvm("projecting data from a bare union member".into())
					})?;
					let repr = self.repr_of(payload, 0)?;
					let raw = self.member_payload_value(b, pointer, nominal, index, repr)?;
					self.call_converting_copy(
						b,
						CopyPair {
							from: CopyShape { ty: payload, repr },
							to: pair.to,
						},
						raw,
					)?
				}
			} else {
				let repr = self.repr_of(alternative.ty, 0)?;
				let layout = self.box_type(Some(repr));
				let address = b.build_struct_gep(layout, pointer, 1, "")?;
				let raw = b.build_load(self.llvm_type(repr), address, "")?;
				self.call_converting_copy(
					b,
					CopyPair {
						from: CopyShape {
							ty: alternative.ty,
							repr,
						},
						to: pair.to,
					},
					raw,
				)?
			};
			let end = b.get_insert_block().expect("a block");
			b.build_unconditional_branch(join)?;
			result.add_incoming(&[(&copied, end)]);
			cases.push((self.context.i64_type().const_int(tag, false), block));
		}
		b.position_at_end(otherwise);
		b.build_unreachable()?;
		b.position_at_end(start);
		b.build_switch(id, otherwise, &cases)?;
		b.position_at_end(join);
		Ok(result.as_basic_value())
	}

	fn copy_box_to_inline(
		&self,
		b: &Builder<'ctx>,
		nominal: fa_analyzer::ids::NominalId,
		source: PointerValue<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let layout = self.inline_union_type(nominal);
		let out = self.inline_union_slot(b, layout.const_zero().into())?;
		let tag = self.union_id(b, source)?;
		let tag_slot = b.build_struct_gep(layout, out, 0, "")?;
		b.build_store(tag_slot, tag)?;
		let start = b.get_insert_block().expect("a block");
		let function = start.get_parent().expect("a function");
		let end = self.context.append_basic_block(function, "inline_copy");
		let mut cases = Vec::new();
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len() as u16;
		for index in 0..count {
			let Some(ty) = self.member_data(nominal, index) else {
				continue;
			};
			let repr = self.repr_of(ty, 0)?;
			let block = self
				.context
				.append_basic_block(function, "copy_inline_payload");
			b.position_at_end(block);
			let raw = self.member_payload_value(b, source, nominal, index, repr)?;
			let copied = self.call_copy(b, repr, raw)?;
			let payload = b.build_struct_gep(self.box_type(Some(repr)), out, 1, "")?;
			self.store_inline_payload(b, payload, repr, copied)?;
			b.build_unconditional_branch(end)?;
			cases.push((
				self.context
					.i64_type()
					.const_int(self.member_id(nominal, index), false),
				block,
			));
		}
		b.position_at_end(start);
		b.build_switch(tag, end, &cases)?;
		b.position_at_end(end);
		Ok(b.build_load(layout, out, "")?)
	}

	fn copy_literal_member(
		&self,
		b: &Builder<'ctx>,
		source: TypeId,
		target: CopyShape,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let candidate = match self.types().kind(target.ty) {
			TypeKind::Member(nominal, index) if self.member_data(nominal, index).is_none() => {
				Some((nominal, index))
			}
			TypeKind::NamedUnion(nominal, mask) => {
				let count = self
					.types()
					.sym_list(self.types().nominal(nominal).members)
					.len() as u16;
				(0..count)
					.find(|&index| {
						self.types().mask_has(mask, index)
							&& self.types().payload(nominal, index) == source
					})
					.map(|index| (nominal, index))
			}
			_ => None,
		};
		let Some((nominal, index)) = candidate else {
			return Ok(None);
		};
		let id = self
			.context
			.i64_type()
			.const_int(self.member_id(nominal, index), false);
		if let Repr::InlineUnion(nominal) = target.repr {
			return Ok(Some(
				b.build_insert_value(self.inline_union_type(nominal).const_zero(), id, 0, "")?
					.into_struct_value()
					.into(),
			));
		}
		let ty = self.box_type(None);
		let global = self.module.add_global(ty, None, "fa.member");
		global.set_initializer(&ty.const_named_struct(&[id.into()]));
		global.set_constant(true);
		global.set_linkage(inkwell::module::Linkage::Private);
		Ok(Some(global.as_pointer_value().into()))
	}
}
