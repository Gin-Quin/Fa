//! Sorting: direct in-place sorting, or a staged permutation when callbacks throw.

use fa_analyzer::ids::{NodeId, TypeId};
use inkwell::IntPredicate;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn sort_array(
		&mut self,
		array_value: Val<'ctx>,
		item: Repr,
		copy: bool,
		values: &[Val<'ctx>],
		arguments: &[NodeId],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let array = array_value.value.expect("an array").into_pointer_value();
		let target = if copy {
			let copy = self.emit_copy(array_value)?;
			copy.value.expect("an array").into_pointer_value()
		} else {
			array
		};
		let staged = arguments
			.first()
			.is_some_and(|&callback| self.callback_may_throw(callback));
		let sorted = if copy {
			self.fresh(Val::new(array_value.repr, target))
		} else {
			Val::void()
		};
		let (sort_target, sort_item) = if staged {
			let (_, length) = self.array_parts(target)?;
			let index_repr = Repr::Int(IntRepr::I64);
			let permutation = self.new_array(index_repr, length, length)?;
			self.fresh(Val::new(Repr::Array(TypeId::INTEGER), permutation));
			self.for_each_index(length, |this, index| {
				let slot = this.array_item_slot(permutation, index_repr, index)?;
				this.builder.build_store(slot.pointer, index)?;
				Ok(())
			})?;
			(permutation, index_repr)
		} else {
			(target, item)
		};
		let comparator = self.comparator(
			item,
			values.first().copied(),
			arguments.first().copied(),
			staged,
			node,
		)?;
		let size = self.item_size(sort_item)?;
		// the context: the comparator, and the error it threw (null)
		let (context, error) = match values.first() {
			Some(callback) => {
				let structure = self.context.struct_type(
					&[
						self.llvm_type(callback.repr),
						self.pointer_type().into(),
						self.pointer_type().into(),
					],
					false,
				);
				let slot = self.builder.build_alloca(structure, "comparator")?;
				let function = self.builder.build_struct_gep(structure, slot, 0, "")?;
				self.builder
					.build_store(function, callback.value.expect("a function"))?;
				let error = self.builder.build_struct_gep(structure, slot, 1, "")?;
				self.builder
					.build_store(error, self.pointer_type().const_null())?;
				let source = self.builder.build_struct_gep(structure, slot, 2, "")?;
				self.builder.build_store(source, target)?;
				(slot, Some(error))
			}
			None => (self.pointer_type().const_null(), None),
		};
		self.call_runtime(
			"fa_array_sort",
			&[
				sort_target.into(),
				size.into(),
				comparator.as_global_value().as_pointer_value().into(),
				context.into(),
			],
		)?;
		// a comparator that threw: the sort throws its error
		if let (Some(error), Some(&callback_node)) = (error, arguments.first())
			&& self.callback_may_throw(callback_node)
		{
			let error = self
				.builder
				.build_load(self.pointer_type(), error, "")?
				.into_pointer_value();
			self.raise_if_error(error)?;
		}
		if staged {
			self.apply_sort_permutation(target, item, sort_target)?;
		}
		Ok(sorted)
	}

	/// Apply destination-to-source indices with ownership transfers, using
	/// one item register per cycle. No item is copied or destroyed here.
	fn apply_sort_permutation(
		&mut self,
		array: PointerValue<'ctx>,
		item: Repr,
		permutation: PointerValue<'ctx>,
	) -> Result<(), Error> {
		let (_, length) = self.array_parts(array)?;
		let index_repr = Repr::Int(IntRepr::I64);
		self.for_each_index(length, |this, start| {
			let first = this.array_item_slot(array, item, start)?;
			let saved = this.load(first)?;
			let cursor = this.alloca(index_repr, "sort_cycle")?;
			this.builder.build_store(cursor.pointer, start)?;
			let test = this.append_block("sort_cycle_test");
			let shift = this.append_block("sort_cycle_shift");
			let finish = this.append_block("sort_cycle_finish");
			this.builder.build_unconditional_branch(test)?;
			this.builder.position_at_end(test);
			let position = this.load(cursor)?.int();
			let mapping = this.array_item_slot(permutation, index_repr, position)?;
			let source = this.load(mapping)?.int();
			let done = this
				.builder
				.build_int_compare(IntPredicate::EQ, source, start, "")?;
			this.builder.build_conditional_branch(done, finish, shift)?;
			this.builder.position_at_end(shift);
			let from = this.array_item_slot(array, item, source)?;
			let value = this.load(from)?;
			let to = this.array_item_slot(array, item, position)?;
			if let Some(value) = value.value {
				this.builder.build_store(to.pointer, value)?;
			}
			this.builder.build_store(mapping.pointer, position)?;
			this.builder.build_store(cursor.pointer, source)?;
			this.builder.build_unconditional_branch(test)?;
			this.builder.position_at_end(finish);
			let last = this.array_item_slot(array, item, position)?;
			if let Some(saved) = saved.value {
				this.builder.build_store(last.pointer, saved)?;
			}
			this.builder.build_store(mapping.pointer, position)?;
			Ok(())
		})
	}

	/// A C-convention comparator for `fa_array_sort`: `(context, a, b) -> i32`
	/// (negative, zero or positive), calling the Fa comparator in the context
	/// or comparing in the natural order.
	fn comparator(
		&mut self,
		item: Repr,
		callback: Option<Val<'ctx>>,
		callback_node: Option<NodeId>,
		staged: bool,
		node: NodeId,
	) -> Result<inkwell::values::FunctionValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let i32 = self.context.i32_type();
		let fn_type = i32.fn_type(&[pointer.into(), pointer.into(), pointer.into()], false);
		let function = self.module.add_function(
			"fa.comparator",
			fn_type,
			Some(inkwell::module::Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		// lowered as a function of its own
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, Repr::Void, None);
		let mut a_pointer = function.get_nth_param(1).expect("a").into_pointer_value();
		let mut b_pointer = function.get_nth_param(2).expect("b").into_pointer_value();
		if staged {
			let callback = callback.expect("a staged comparator has a callback");
			let context = function
				.get_nth_param(0)
				.expect("a context")
				.into_pointer_value();
			let structure = self.context.struct_type(
				&[
					self.llvm_type(callback.repr),
					pointer.into(),
					pointer.into(),
				],
				false,
			);
			let source = self.builder.build_struct_gep(structure, context, 2, "")?;
			let array = self
				.builder
				.build_load(pointer, source, "")?
				.into_pointer_value();
			let a = self
				.builder
				.build_load(self.context.i64_type(), a_pointer, "")?
				.into_int_value();
			let b = self
				.builder
				.build_load(self.context.i64_type(), b_pointer, "")?
				.into_int_value();
			a_pointer = self.array_item_slot(array, item, a)?.pointer;
			b_pointer = self.array_item_slot(array, item, b)?.pointer;
		}
		let a = self.load(Slot {
			pointer: a_pointer,
			repr: item,
		})?;
		let b = self.load(Slot {
			pointer: b_pointer,
			repr: item,
		})?;
		let result = match (callback, callback_node) {
			(Some(callback), Some(callback_node)) => {
				let context = function
					.get_nth_param(0)
					.expect("a context")
					.into_pointer_value();
				let structure = self.context.struct_type(
					&[
						self.llvm_type(callback.repr),
						self.pointer_type().into(),
						self.pointer_type().into(),
					],
					false,
				);
				let closure_pointer = self.builder.build_struct_gep(structure, context, 0, "")?;
				let error_pointer = self.builder.build_struct_gep(structure, context, 1, "")?;
				let closure = self.load(Slot {
					pointer: closure_pointer,
					repr: callback.repr,
				})?;
				// a comparator that threw: the error kept for the sort, which
				// ends comparing everything as equal
				let throws = self.callback_may_throw(callback_node);
				let failed_block = self.append_block("comparator_failed");
				if throws {
					let previous = self
						.builder
						.build_load(self.pointer_type(), error_pointer, "")?
						.into_pointer_value();
					let failed = self.builder.build_is_not_null(previous, "")?;
					let compare = self.append_block("compare");
					self.builder
						.build_conditional_branch(failed, failed_block, compare)?;
					self.builder.position_at_end(compare);
					let mark = self.mark();
					self.frame_mut()
						.catches
						.push((failed_block, error_pointer, mark));
				}
				let order = self.call_collection_callback(
					closure,
					callback_node,
					&[],
					&[
						Slot {
							pointer: a_pointer,
							repr: item,
						},
						Slot {
							pointer: b_pointer,
							repr: item,
						},
					],
					&[],
				);
				if throws {
					self.frame_mut().catches.pop();
				}
				let order = order?;
				let resume = self.builder.get_insert_block();
				self.builder.position_at_end(failed_block);
				self.builder
					.build_return(Some(&self.context.i32_type().const_zero()))?;
				if let Some(resume) = resume {
					self.builder.position_at_end(resume);
				}
				match order.repr {
					Repr::Int(int) => {
						let zero = self.int_type(int.bits).const_zero();
						let negative = self.builder.build_int_compare(
							IntPredicate::SLT,
							order.int(),
							zero,
							"",
						)?;
						let positive = self.builder.build_int_compare(
							IntPredicate::SGT,
							order.int(),
							zero,
							"",
						)?;
						self.sign(negative, positive)?
					}
					_ => {
						let order = order.value.expect("a float").into_float_value();
						let zero = order.get_type().const_zero();
						let negative = self.builder.build_float_compare(
							inkwell::FloatPredicate::OLT,
							order,
							zero,
							"",
						)?;
						let positive = self.builder.build_float_compare(
							inkwell::FloatPredicate::OGT,
							order,
							zero,
							"",
						)?;
						self.sign(negative, positive)?
					}
				}
			}
			_ => {
				let less = self.values_order(a, b, IntPredicate::SLT, node)?;
				let greater = self.values_order(b, a, IntPredicate::SLT, node)?;
				self.sign(less, greater)?
			}
		};
		self.builder.build_return(Some(&result))?;
		let frame = self.frame.take().expect("the comparator");
		self.frame = Some(frame);
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		Ok(function)
	}

	/// -1, 0 or 1 from two conditions.
	fn sign(
		&self,
		negative: IntValue<'ctx>,
		positive: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i32 = self.context.i32_type();
		let positive = self
			.builder
			.build_select(positive, i32.const_int(1, false), i32.const_zero(), "")?
			.into_int_value();
		Ok(self
			.builder
			.build_select(negative, i32.const_all_ones(), positive, "")?
			.into_int_value())
	}
}
