//! `Pool` and `GrowingPool`: a slab whose elements never move
//! (collections/pools.md, implementation notes).
//!
//! A pool is a pointer to a `FaPool` of the runtime (`fa_runtime.c`,
//! "pools"), whose first field is `size`. Its elements live in chunks
//! aligned on their size, found from an element's address; the runtime
//! hands out and takes back the slots (`fa_pool_add_slot`,
//! `fa_pool_remove`), and the generated code stores, drops and copies the
//! elements. Iteration is in slot order: `fa_pool_seek` gives the position
//! of the next element, `fa_pool_at` its address.
//!
//! A reference into a pool is the address of the element (a [`Slot`]):
//! `use john = pool.add(…)` designates the stored element, and
//! `use found = pool.find(…)` an element or nothing (a null address, read
//! as an optional: [`Lowering::nullable_references`]).

use fa_analyzer::analysis::{Argument, CallInfo, Callee};
use fa_analyzer::ids::NodeId;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::values::{FunctionValue, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// The representation of the elements of a pool.
	fn pool_element(&self, pool: Repr, node: NodeId) -> Result<Repr, Error> {
		let Repr::Pool(item) = pool else {
			return Err(self.unsupported(node.index(), "this pool"));
		};
		self.item_repr(item, node)
	}

	/// A new empty pool of elements of this representation.
	pub(crate) fn pool_new(&self, element: Repr) -> Result<PointerValue<'ctx>, Error> {
		let size = self.item_size_const(element);
		Ok(self
			.call_value(self.runtime.function("fa_pool_new")?, &[size.into()])?
			.into_pointer_value())
	}

	/// `pool.size`: the first field of the pool.
	pub(crate) fn pool_size(&self, pool: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let pointer = pool.value.expect("a pool").into_pointer_value();
		let size = self
			.builder
			.build_load(self.context.i64_type(), pointer, "size")?;
		Ok(Val::new(Repr::Int(IntRepr::I64), size))
	}

	/// `fa_pool_seek(pool, position)` and whether it found an element.
	pub(crate) fn pool_seek(
		&self,
		builder: &Builder<'ctx>,
		pool: PointerValue<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let found = builder
			.build_call(
				self.runtime.function("fa_pool_seek")?,
				&[pool.into(), position.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a position")
			.into_int_value();
		let end = builder.build_int_compare(
			IntPredicate::EQ,
			found,
			self.context.i64_type().const_all_ones(),
			"",
		)?;
		Ok((found, end))
	}

	/// The address of the element at a position `fa_pool_seek` returned.
	pub(crate) fn pool_at(
		&self,
		builder: &Builder<'ctx>,
		pool: PointerValue<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(builder
			.build_call(
				self.runtime.function("fa_pool_at")?,
				&[pool.into(), position.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an element")
			.into_pointer_value())
	}

	/// A loop over the elements of a pool in the function being lowered:
	/// `body` gets the address of each element and the exit block. The
	/// position moves past the element before the body runs, so that the
	/// body may remove it.
	fn pool_each(
		&mut self,
		pool: PointerValue<'ctx>,
		mut body: impl FnMut(&mut Self, PointerValue<'ctx>, BasicBlock<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let counter = self.alloca(Repr::Int(IntRepr::I64), "pool_position")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("pool_each");
		let block = self.append_block("pool_each_body");
		let exit = self.append_block("end_pool_each");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let (found, end) = self.pool_seek(&self.builder, pool, position)?;
		self.builder.build_conditional_branch(end, exit, block)?;
		self.builder.position_at_end(block);
		let element = self.pool_at(&self.builder, pool, found)?;
		let next = self
			.builder
			.build_int_add(found, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		let mark = self.mark();
		body(self, element, exit)?;
		self.end_statement(mark, None)?;
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(())
	}

	/// A loop over the elements of a pool in a helper (a drop, a copy).
	fn helper_pool_each(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pool: PointerValue<'ctx>,
		mut body: impl FnMut(&Builder<'ctx>, PointerValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let before = builder.get_insert_block().expect("a block");
		let test = self.context.append_basic_block(function, "pool_each");
		let block = self.context.append_basic_block(function, "pool_each_body");
		let end = self.context.append_basic_block(function, "pool_each_end");
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(test);
		let position = builder.build_phi(i64, "position")?;
		position.add_incoming(&[(&i64.const_zero(), before)]);
		let (found, done) =
			self.pool_seek(builder, pool, position.as_basic_value().into_int_value())?;
		builder.build_conditional_branch(done, end, block)?;
		builder.position_at_end(block);
		let element = self.pool_at(builder, pool, found)?;
		body(builder, element)?;
		let next = builder.build_int_add(found, i64.const_int(1, false), "")?;
		let last = builder.get_insert_block().expect("a block");
		position.add_incoming(&[(&next, last)]);
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(end);
		Ok(())
	}

	/// A view of a pool as an array: its elements copied bytewise into a
	/// block that doesn't own them, freed at the end of the statement (the
	/// read-only methods, printing).
	pub(crate) fn pool_view(&mut self, value: Val<'ctx>, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Repr::Pool(item_type) = value.repr else {
			return Ok(value);
		};
		let element = self.item_repr(item_type, node)?;
		let pool = value.value.expect("a pool").into_pointer_value();
		let size = self.pool_size(value)?.int();
		let zero = self.context.i64_type().const_zero();
		let view = self.new_array(element, zero, size)?;
		let element_size = self.item_size_const(element);
		self.pool_each(pool, |this, pointer, _| {
			let slot = this
				.call_value(
					this.runtime.function("fa_array_push_slot")?,
					&[view.into(), element_size.into()],
				)?
				.into_pointer_value();
			let loaded = this
				.builder
				.build_load(this.llvm_type(element), pointer, "")?;
			this.builder.build_store(slot, loaded)?;
			Ok(())
		})?;
		self.cleanups
			.borrow_mut()
			.push(super::ownership::Cleanup::Shallow(view, element_size));
		Ok(Val::new(Repr::Array(item_type), view))
	}

	/// `[]` and `[a, b]` as a pool: the elements added in order.
	pub(crate) fn pool_literal(&mut self, values: Vec<(Val<'ctx>, NodeId)>, repr: Repr, node: NodeId) -> Result<Val<'ctx>, Error> {
		let element = self.pool_element(repr, node)?;
		let pool = self.pool_new(element)?;
		for (value, _) in values {
			let slot = self
				.call_value(self.runtime.function("fa_pool_add_slot")?, &[pool.into()])?
				.into_pointer_value();
			self.builder
				.build_store(slot, value.value.expect("an element"))?;
		}
		Ok(self.fresh(Val::new(repr, pool)))
	}

	/// The given arguments of a call, in order.
	fn given_arguments(info: &CallInfo) -> Vec<NodeId> {
		info.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect()
	}

	/// `pool.add(value)`: the address of the new element.
	fn pool_add(
		&mut self,
		pool: Val<'ctx>,
		value: NodeId,
		node: NodeId,
	) -> Result<Option<Slot<'ctx>>, Error> {
		let element = self.pool_element(pool.repr, node)?;
		let lowered = self.expr(value)?;
		if lowered.repr == Repr::Never {
			return Ok(None);
		}
		let owned = self.owned_as(lowered, element, Some(value))?;
		let slot = self
			.call_value(
				self.runtime.function("fa_pool_add_slot")?,
				&[pool.value.expect("a pool").into()],
			)?
			.into_pointer_value();
		self.builder
			.build_store(slot, owned.value.expect("an element"))?;
		Ok(Some(Slot {
			pointer: slot,
			repr: element,
		}))
	}

	/// `pool.find(predicate)`: the address of the first element (in slot
	/// order) for which the predicate holds, or null.
	fn pool_find(
		&mut self,
		pool: Val<'ctx>,
		callback: NodeId,
		node: NodeId,
	) -> Result<Option<PointerValue<'ctx>>, Error> {
		let predicate = self.expr(callback)?;
		if predicate.repr == Repr::Never {
			return Ok(None);
		}
		let element = self.pool_element(pool.repr, node)?;
		let result = self.alloca_type(self.pointer_type().into(), "found")?;
		self.builder
			.build_store(result, self.pointer_type().const_null())?;
		let callback_type = self.type_of(callback);
		self.pool_each(
			pool.value.expect("a pool").into_pointer_value(),
			|this, pointer, exit| {
				let value = this.load(Slot {
					pointer,
					repr: element,
				})?;
				let holds = this.call_closure(predicate, callback_type, &[value], callback)?;
				let holds = this.convert(holds, Repr::Bool, callback.index())?.int();
				let found = this.append_block("pool_found");
				let next = this.append_block("pool_find_next");
				this.builder.build_conditional_branch(holds, found, next)?;
				this.builder.position_at_end(found);
				this.builder.build_store(result, pointer)?;
				this.builder.build_unconditional_branch(exit)?;
				this.builder.position_at_end(next);
				Ok(())
			},
		)?;
		Ok(Some(
			self.builder
				.build_load(self.pointer_type(), result, "")?
				.into_pointer_value(),
		))
	}

	/// An element as an optional value: `none` for a null address.
	pub(crate) fn optional_element(
		&mut self,
		pointer: PointerValue<'ctx>,
		element: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(scalar) = Scalar::of(element) else {
			return Err(self.unsupported(node.index(), "an optional reference to this value"));
		};
		let repr = Repr::Optional(scalar);
		let result = self.alloca(repr, "element")?;
		self.builder
			.build_store(result.pointer, super::zero_of(self.llvm_type(repr)))?;
		let present = self.builder.build_is_not_null(pointer, "")?;
		let then = self.append_block("element_present");
		let end = self.append_block("element_read");
		self.builder.build_conditional_branch(present, then, end)?;
		self.builder.position_at_end(then);
		let value = self.load(Slot {
			pointer,
			repr: element,
		})?;
		let value = self.convert(value, repr, node.index())?;
		self.builder
			.build_store(result.pointer, value.value.expect("an optional"))?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		self.load(result)
	}

	/// Whether a node is a call of `Pool#add` or `Pool#find` (a reference
	/// into the pool), and which.
	fn pool_reference_call(&self, node: NodeId) -> Option<(&'static str, CallInfo)> {
		let info = self.analysis.call(node)?;
		let Callee::Builtin(id) = info.callee else {
			return None;
		};
		let path = fa_analyzer::builtins::get(id).path;
		matches!(path, "Pool#add" | "Pool#find").then(|| (path, info.clone()))
	}

	/// The place `use x = pool.add(…)` or `use x = pool.find(…)` designates,
	/// and whether it may be null (`find`).
	pub(crate) fn pool_reference(
		&mut self,
		node: NodeId,
	) -> Result<Option<(Slot<'ctx>, bool)>, Error> {
		let Some((path, info)) = self.pool_reference_call(node) else {
			return Ok(None);
		};
		let pool = self.receiver(node)?;
		if pool.repr == Repr::Never {
			return Err(self.unsupported(node.index(), "a `use` of this value"));
		}
		let arguments = Self::given_arguments(&info);
		let [argument] = arguments.as_slice() else {
			return Err(self.unsupported(node.index(), path));
		};
		let element = self.pool_element(pool.repr, node)?;
		if path == "Pool#add" {
			let Some(slot) = self.pool_add(pool, *argument, node)? else {
				return Err(self.unsupported(node.index(), path));
			};
			return Ok(Some((slot, false)));
		}
		let Some(pointer) = self.pool_find(pool, *argument, node)? else {
			return Err(self.unsupported(node.index(), path));
		};
		Ok(Some((
			Slot {
				pointer,
				repr: element,
			},
			true,
		)))
	}

	/// The address of the element a reference argument designates
	/// (`pool.delete(john)`).
	fn element_address(&mut self, argument: NodeId) -> Result<PointerValue<'ctx>, Error> {
		let mut inner = argument;
		while let fa_analyzer::syntax::View::Group(group) = self.view(inner) {
			inner = group;
		}
		if !self.is_place(inner) {
			return Err(self.unsupported(argument.index(), "removing this value from a pool"));
		}
		Ok(self.place(inner)?.pointer)
	}

	/// The methods of pools.
	pub(crate) fn pool_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let pool = self.receiver(node)?;
		if pool.repr == Repr::Never {
			return Ok(pool);
		}
		let element = self.pool_element(pool.repr, node)?;
		let pointer = pool.value.expect("a pool").into_pointer_value();
		let arguments = Self::given_arguments(info);
		let method = path.split('#').nth(1).unwrap_or(path);
		match (method, arguments.as_slice()) {
			("isEmpty", []) => {
				let size = self.pool_size(pool)?.int();
				let empty = self.builder.build_int_compare(
					IntPredicate::EQ,
					size,
					size.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, empty))
			}
			("add", [value]) => match self.pool_add(pool, *value, node)? {
				// a view of the stored element
				Some(slot) => self.load(slot),
				None => Ok(Val::never()),
			},
			("find", [callback]) => {
				let Some(found) = self.pool_find(pool, *callback, node)? else {
					return Ok(Val::never());
				};
				self.optional_element(found, element, node)
			}
			("delete" | "take", [reference]) => {
				let address = self.element_address(*reference)?;
				let value = self.load(Slot {
					pointer: address,
					repr: element,
				})?;
				self.call_runtime("fa_pool_remove", &[pointer.into(), address.into()])?;
				if method == "take" {
					// the element leaves the pool: owned by the statement
					Ok(self.fresh(value))
				} else {
					self.emit_drop(value)?;
					Ok(Val::void())
				}
			}
			("deleteWhere", [callback]) => {
				let predicate = self.expr(*callback)?;
				if predicate.repr == Repr::Never {
					return Ok(predicate);
				}
				let callback_type = self.type_of(*callback);
				let callback_node = *callback;
				self.pool_each(pointer, |this, address, _| {
					let value = this.load(Slot {
						pointer: address,
						repr: element,
					})?;
					let holds =
						this.call_closure(predicate, callback_type, &[value], callback_node)?;
					let holds = this.convert(holds, Repr::Bool, callback_node.index())?.int();
					let remove = this.append_block("pool_remove");
					let keep = this.append_block("pool_keep");
					this.builder.build_conditional_branch(holds, remove, keep)?;
					this.builder.position_at_end(remove);
					let value = this.load(Slot {
						pointer: address,
						repr: element,
					})?;
					this.emit_drop(value)?;
					this.call_runtime("fa_pool_remove", &[pointer.into(), address.into()])?;
					this.builder.build_unconditional_branch(keep)?;
					this.builder.position_at_end(keep);
					Ok(())
				})?;
				Ok(Val::void())
			}
			("clear", []) => {
				if self.needs_drop(element) {
					self.pool_each(pointer, |this, address, _| {
						let value = this.load(Slot {
							pointer: address,
							repr: element,
						})?;
						this.emit_drop(value)
					})?;
				}
				self.call_runtime("fa_pool_clear", &[pointer.into()])?;
				Ok(Val::void())
			}
			// the other methods read a view of the elements
			(_, _) if !matches!(self.node_repr(node)?, Repr::Pool(_)) => {
				let mut values = Vec::new();
				for &argument in &arguments {
					let value = self.expr(argument)?;
					if value.repr == Repr::Never {
						return Ok(value);
					}
					values.push(value);
				}
				let view = self.pool_view(pool, node)?;
				let array_path = format!("Array#{method}");
				self.array_method_more(node, &array_path, view, &values, &arguments)
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// `for element in pool`: the variable designates each element, in slot
	/// order.
	pub(crate) fn for_pool(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		pool: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let element = self.pool_element(pool.repr, node)?;
		let pointer = pool.value.expect("a pool").into_pointer_value();
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		let destructured = pattern.and_then(|pattern| self.analysis.pattern(pattern).cloned());
		let copied = variable.is_some_and(|symbol| {
			matches!(
				self.analysis.symbols.symbol(symbol).kind,
				fa_analyzer::symbols::SymbolKind::LoopVariable { copy: true }
			)
		});
		let copy_slot = match (variable, copied) {
			(Some(symbol), true) => Some(self.slot_of(symbol, node)?),
			_ => None,
		};
		let counter = self.alloca(Repr::Int(IntRepr::I64), "pool_position")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("for_element");
		let body_block = self.append_block("for_element_body");
		let exit = self.append_block("end_for_element");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let (found, end) = self.pool_seek(&self.builder, pointer, position)?;
		self.builder.build_conditional_branch(end, exit, body_block)?;
		self.builder.position_at_end(body_block);
		let address = self.pool_at(&self.builder, pointer, found)?;
		let next = self
			.builder
			.build_int_add(found, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		let element_slot = Slot {
			pointer: address,
			repr: element,
		};
		match (variable, copy_slot) {
			_ if destructured.is_some() => {
				self.bind_item_pattern(
					destructured.as_ref().expect("a pattern"),
					element_slot,
					node,
				)?;
			}
			(Some(symbol), None) => {
				self.slots.insert(symbol, element_slot);
			}
			(_, Some(copy)) => {
				let value = self.load(element_slot)?;
				let value = self.emit_copy(value)?;
				self.store(copy, value, node)?;
			}
			_ => {}
		}
		let mark = self.mark();
		if let Some(copy) = copy_slot {
			self.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Local(copy));
		}
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: test,
			value: None,
			mark,
		});
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.end_scope(mark, None)?;
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// The drop of a pool: its elements, its chunks, its header.
	pub(crate) fn build_pool_drop(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		pool: PointerValue<'ctx>,
	) -> Result<(), Error> {
		let element = self.pool_element(repr, NodeId::new(0))?;
		let present = builder.build_is_not_null(pool, "")?;
		let then = self.context.append_basic_block(function, "pool");
		let none = self.context.append_basic_block(function, "no_pool");
		builder.build_conditional_branch(present, then, none)?;
		builder.position_at_end(none);
		builder.build_return(None)?;
		builder.position_at_end(then);
		if self.needs_drop(element) {
			let element_type = self.llvm_type(element);
			self.helper_pool_each(builder, function, pool, |builder, address| {
				let loaded = builder.build_load(element_type, address, "")?;
				self.call_drop(builder, element, loaded)
			})?;
		}
		builder.build_call(
			self.runtime.function("fa_pool_free")?,
			&[pool.into()],
			"",
		)?;
		Ok(())
	}

	/// The copy of a pool: a new pool with copies of the elements.
	pub(crate) fn build_pool_copy(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		pool: PointerValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let element = self.pool_element(repr, NodeId::new(0))?;
		let present = builder.build_is_not_null(pool, "")?;
		let then = self.context.append_basic_block(function, "pool");
		let none = self.context.append_basic_block(function, "no_pool");
		builder.build_conditional_branch(present, then, none)?;
		builder.position_at_end(none);
		builder.build_return(Some(&pool))?;
		builder.position_at_end(then);
		let size = self.item_size_const(element);
		let copy = builder
			.build_call(self.runtime.function("fa_pool_new")?, &[size.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a pool")
			.into_pointer_value();
		let element_type = self.llvm_type(element);
		self.helper_pool_each(builder, function, pool, |builder, address| {
			let loaded = builder.build_load(element_type, address, "")?;
			let copied = self.call_copy(builder, element, loaded)?;
			let slot = builder
				.build_call(
					self.runtime.function("fa_pool_add_slot")?,
					&[copy.into()],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a slot")
				.into_pointer_value();
			builder.build_store(slot, copied)?;
			Ok(())
		})?;
		Ok(copy)
	}
}
