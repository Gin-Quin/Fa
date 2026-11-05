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

/// How a `for` reaches the elements that never move of a collection.
#[derive(Copy, Clone)]
pub(crate) enum Cursor<'a> {
	/// The runtime's `seek` and `at` functions (the targets of a relation
	/// side).
	Seek(&'a str, &'a str),
	/// The chunks of a pool or a columnar, their occupancy words bit by bit
	/// ([`Lowering::chunk_loop`]).
	Chunks,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The representation of the elements of a pool.
	pub(crate) fn pool_element(&self, pool: Repr, node: NodeId) -> Result<Repr, Error> {
		let (Repr::Pool(item) | Repr::Columnar(item)) = pool else {
			return Err(self.unsupported(node.index(), "this pool"));
		};
		self.item_repr(item, node)
	}

	/// The representation of the place of an element of a pool or a
	/// columnar: the element, or a [`Repr::Row`] of its columns.
	pub(crate) fn element_slot_repr(
		&self,
		collection: Repr,
		item: fa_analyzer::ids::TypeId,
		node: NodeId,
	) -> Result<Repr, Error> {
		let element = self.item_repr(item, node)?;
		Ok(match (collection, element) {
			(Repr::Columnar(_), Repr::Object(object)) => Repr::Row(object),
			_ => element,
		})
	}

	/// The place representation of the elements of a pool or a columnar.
	pub(crate) fn pool_place(&self, pool: Repr, node: NodeId) -> Result<Repr, Error> {
		let (Repr::Pool(item) | Repr::Columnar(item)) = pool else {
			return Err(self.unsupported(node.index(), "this pool"));
		};
		self.element_slot_repr(pool, item, node)
	}

	/// A new empty pool or columnar (`collection`) of elements of this
	/// representation.
	pub(crate) fn collection_new(
		&self,
		builder: &Builder<'ctx>,
		collection: Repr,
		element: Repr,
	) -> Result<PointerValue<'ctx>, Error> {
		if let (Repr::Columnar(_), Repr::Object(object)) = (collection, element) {
			let layout = self.column_layout(object);
			let i64 = self.context.i64_type();
			return Ok(builder
				.build_call(
					self.runtime.function("fa_columnar_new")?,
					&[
						i64.const_int(layout.capacity, false).into(),
						i64.const_int(layout.pages, false).into(),
						i64.const_int(layout.slots_offset, false).into(),
					],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a columnar")
				.into_pointer_value());
		}
		let size = self.item_size_const(element);
		Ok(builder
			.build_call(self.runtime.function("fa_pool_new")?, &[size.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a pool")
			.into_pointer_value())
	}

	/// The element at an address of a pool, or gathered from the columns of
	/// a columnar (`place` is [`Lowering::pool_place`]'s).
	fn element_read(
		&self,
		builder: &Builder<'ctx>,
		place: Repr,
		address: PointerValue<'ctx>,
	) -> Result<inkwell::values::BasicValueEnum<'ctx>, Error> {
		match place {
			Repr::Row(object) => Ok(self.gather(builder, address, object)?.into()),
			_ => Ok(builder.build_load(self.llvm_type(place), address, "")?),
		}
	}

	/// Stores an element at an address of a pool, or into the columns of a
	/// columnar.
	pub(crate) fn element_write(
		&self,
		builder: &Builder<'ctx>,
		place: Repr,
		address: PointerValue<'ctx>,
		value: inkwell::values::BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		match place {
			Repr::Row(object) => self.scatter(builder, address, object, value),
			_ => {
				builder.build_store(address, value)?;
				Ok(())
			}
		}
	}

	/// `Pool.from(source, capacity = n)`, and the same of the growing pools
	/// and the columnars: a new collection of copies of the source's items
	/// (their elements never move: the capacity is a hint).
	pub(crate) fn pool_from(&mut self, node: NodeId, info: &CallInfo) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let Some(Argument::Given(source_node)) = info.arguments.first() else {
			return Err(self.unsupported(node.index(), "this `from`"));
		};
		let source = self.expr(*source_node)?;
		if source.repr == Repr::Never {
			return Ok(source);
		}
		// the capacity, for its effects
		if let Some(Argument::Given(capacity)) = info.arguments.get(1) {
			let capacity = self.expr(*capacity)?;
			if capacity.repr == Repr::Never {
				return Ok(capacity);
			}
		}
		let Repr::Array(item_type) = source.repr else {
			return Err(self.unsupported(source_node.index(), "this source of `from`"));
		};
		let item = self.item_repr(item_type, *source_node)?;
		let element = self.pool_element(repr, node)?;
		let place = self.pool_place(repr, node)?;
		let pool = self.collection_new(&self.builder, repr, element)?;
		let array = source.value.expect("an array").into_pointer_value();
		let (_, length) = self.array_parts(array)?;
		let add = self.runtime.function("fa_pool_add_slot")?;
		self.for_each_item_value(array, item, length, |this, value| {
			let copy = this.emit_copy(value)?;
			let copy = this.convert(copy, element, node.index())?;
			let slot = this.call_value(add, &[pool.into()])?.into_pointer_value();
			this.element_write(&this.builder, place, slot, copy.value.expect("an element"))
		})?;
		Ok(self.fresh(Val::new(repr, pool)))
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
	pub(crate) fn pool_each(
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
		let (Repr::Pool(item_type) | Repr::Columnar(item_type)) = value.repr else {
			return Ok(value);
		};
		let element = self.item_repr(item_type, node)?;
		let place = self.pool_place(value.repr, node)?;
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
			let loaded = this.element_read(&this.builder, place, pointer)?;
			this.builder.build_store(slot, loaded)?;
			Ok(())
		})?;
		self.cleanups
			.borrow_mut()
			.push(super::ownership::Cleanup::Shallow(view, element_size));
		Ok(Val::new(Repr::Array(item_type), view))
	}

	/// `[]` and `[a, b]` as a pool: the elements added in order.
	pub(crate) fn pool_literal(
		&mut self,
		values: Vec<(Val<'ctx>, NodeId)>,
		repr: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let element = self.pool_element(repr, node)?;
		let place = self.pool_place(repr, node)?;
		let pool = self.collection_new(&self.builder, repr, element)?;
		for (value, _) in values {
			let slot = self
				.call_value(self.runtime.function("fa_pool_add_slot")?, &[pool.into()])?
				.into_pointer_value();
			self.element_write(&self.builder, place, slot, value.value.expect("an element"))?;
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
		let place = self.pool_place(pool.repr, node)?;
		let outer_links = std::mem::take(&mut self.pending_links);
		let lowered = self.expr(value)?;
		let pending = std::mem::replace(&mut self.pending_links, outer_links);
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
		self.element_write(&self.builder, place, slot, owned.value.expect("an element"))?;
		// the sides the literal gave, linked now that the element is stored
		if let Repr::Object(element_type) = element
			&& !pending.is_empty()
		{
			self.link_added(node, slot, element_type, pending)?;
		}
		Ok(Some(Slot {
			pointer: slot,
			repr: place,
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
		let _ = self.pool_element(pool.repr, node)?;
		let place = self.pool_place(pool.repr, node)?;
		let result = self.alloca_type(self.pointer_type().into(), "found")?;
		self.builder
			.build_store(result, self.pointer_type().const_null())?;
		self.pool_each(
			pool.value.expect("a pool").into_pointer_value(),
			|this, pointer, exit| {
				let slot = Slot {
					pointer,
					repr: place,
				};
				let holds =
					this.call_collection_callback(predicate, callback, &[], &[slot], &[])?;
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
		let value_repr = match element {
			Repr::Row(object) => Repr::Object(object),
			repr => repr,
		};
		// an optional value (a map's `T?` values): absent reads as `none` too
		let repr = match value_repr {
			Repr::Optional(_) => value_repr,
			_ => match Scalar::of(value_repr) {
				Some(scalar) => Repr::Optional(scalar),
				None => {
					return Err(
						self.unsupported(node.index(), "an optional reference to this value")
					);
				}
			},
		};
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
		matches!(
			path,
			"Pool#add" | "Pool#find" | "Columnar#add" | "Columnar#find"
		)
		.then(|| (path, info.clone()))
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

		let place = self.pool_place(pool.repr, node)?;
		if path.ends_with("#add") {
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
				repr: place,
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
		let place = self.pool_place(pool.repr, node)?;
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
				self.optional_element(found, place, node)
			}
			("delete" | "take", [reference]) => {
				let unlinks = self.prepare_unlinks(node)?;
				let address = self.element_address(*reference)?;
				self.unlink(&unlinks, address)?;
				let value = self.load(Slot {
					pointer: address,
					repr: place,
				})?;
				self.call_runtime("fa_pool_remove", &[pointer.into(), address.into()])?;
				if method == "take" {
					// the element leaves the pool, without its links: owned
					// by the statement
					let value = self.without_links(value)?;
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
				let callback_node = *callback;
				let unlinks = self.prepare_unlinks(node)?;
				self.pool_each(pointer, |this, address, _| {
					let slot = Slot {
						pointer: address,
						repr: place,
					};
					let holds =
						this.call_collection_callback(predicate, callback_node, &[], &[slot], &[])?;
					let holds = this
						.convert(holds, Repr::Bool, callback_node.index())?
						.int();
					let remove = this.append_block("pool_remove");
					let keep = this.append_block("pool_keep");
					this.builder.build_conditional_branch(holds, remove, keep)?;
					this.builder.position_at_end(remove);
					this.unlink(&unlinks, address)?;
					let value = this.load(Slot {
						pointer: address,
						repr: place,
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
				// every element is unlinked before any is dropped
				let unlinks = self.prepare_unlinks(node)?;
				if !unlinks.is_empty() {
					self.pool_each(pointer, |this, address, _| this.unlink(&unlinks, address))?;
				}
				if self.needs_drop(element) {
					self.pool_each(pointer, |this, address, _| {
						let value = this.load(Slot {
							pointer: address,
							repr: place,
						})?;
						this.emit_drop(value)
					})?;
				}
				self.call_runtime("fa_pool_clear", &[pointer.into()])?;
				Ok(Val::void())
			}
			// a new collection of the same kind: copies of the elements kept
			("filter", [callback]) => {
				let predicate = self.expr(*callback)?;
				if predicate.repr == Repr::Never {
					return Ok(predicate);
				}
				let callback_node = *callback;
				let repr = self.node_repr(node)?;
				let kept_element = self.pool_element(repr, node)?;
				let kept_place = self.pool_place(repr, node)?;
				let kept = self.collection_new(&self.builder, repr, kept_element)?;
				let add = self.runtime.function("fa_pool_add_slot")?;
				self.pool_each(pointer, |this, address, _| {
					let slot = Slot {
						pointer: address,
						repr: place,
					};
					let holds =
						this.call_collection_callback(predicate, callback_node, &[], &[slot], &[])?;
					let holds = this
						.convert(holds, Repr::Bool, callback_node.index())?
						.int();
					let copy_block = this.append_block("pool_keep");
					let next = this.append_block("pool_next");
					this.builder
						.build_conditional_branch(holds, copy_block, next)?;
					this.builder.position_at_end(copy_block);
					let value = this.load(slot)?;
					let value = this.without_links(value)?;
					let copy = this.emit_copy(value)?;
					let copy = this.convert(copy, kept_element, node.index())?;
					let target = this.call_value(add, &[kept.into()])?.into_pointer_value();
					this.element_write(
						&this.builder,
						kept_place,
						target,
						copy.value.expect("an element"),
					)?;
					this.builder.build_unconditional_branch(next)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				Ok(self.fresh(Val::new(repr, kept)))
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
		let place = self.pool_place(pool.repr, node)?;
		let pointer = pool.value.expect("a pool").into_pointer_value();
		self.for_slots(node, pattern, pointer, place, Cursor::Chunks, body)
	}

	/// `for element in collection` over elements that never move (a pool, the
	/// targets of a relation side), found with the runtime's `seek` and `at`
	/// functions: the variable designates each element.
	pub(crate) fn for_slots(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		pointer: PointerValue<'ctx>,
		element: Repr,
		cursor: Cursor<'_>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
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
		let exit = self.append_block("end_for_element");
		let (test, address, chunk) = match cursor {
			Cursor::Seek(seek, at) => {
				let counter = self.alloca(Repr::Int(IntRepr::I64), "pool_position")?;
				let i64 = self.context.i64_type();
				self.builder
					.build_store(counter.pointer, i64.const_zero())?;
				let test = self.append_block("for_element");
				let body_block = self.append_block("for_element_body");
				self.builder.build_unconditional_branch(test)?;
				self.builder.position_at_end(test);
				let position = self.load(counter)?.int();
				let (found, end) = self.seek(&self.builder, seek, pointer, position)?;
				self.builder
					.build_conditional_branch(end, exit, body_block)?;
				self.builder.position_at_end(body_block);
				let address = self.at(&self.builder, at, pointer, found)?;
				let next = self
					.builder
					.build_int_add(found, i64.const_int(1, false), "")?;
				self.builder.build_store(counter.pointer, next)?;
				(test, address, None)
			}
			Cursor::Chunks => {
				let (test, address, chunk, slot) = self.chunk_loop(pointer, exit)?;
				(test, address, Some((chunk, slot)))
			}
		};
		let element_slot = Slot {
			pointer: address,
			repr: element,
		};
		match (variable, copy_slot) {
			// the fields of an element of a columnar, straight from the runs
			// of its chunk
			_ if destructured.is_some() && matches!(element, Repr::Row(_)) && chunk.is_some() => {
				let Repr::Row(object) = element else {
					unreachable!()
				};
				let (chunk, slot) = chunk.expect("a chunk");
				self.bind_columns(
					destructured.as_ref().expect("a pattern"),
					object,
					chunk,
					slot,
					node,
				)?;
			}
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
		self.coop_point()?;
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
		let place = self.pool_place(repr, NodeId::new(0))?;
		let present = builder.build_is_not_null(pool, "")?;
		let then = self.context.append_basic_block(function, "pool");
		let none = self.context.append_basic_block(function, "no_pool");
		builder.build_conditional_branch(present, then, none)?;
		builder.position_at_end(none);
		builder.build_return(None)?;
		builder.position_at_end(then);
		if self.needs_drop(element) {
			self.helper_pool_each(builder, function, pool, |builder, address| {
				let loaded = self.element_read(builder, place, address)?;
				self.call_drop(builder, element, loaded)
			})?;
		}
		builder.build_call(self.runtime.function("fa_pool_free")?, &[pool.into()], "")?;
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
		let place = self.pool_place(repr, NodeId::new(0))?;
		let present = builder.build_is_not_null(pool, "")?;
		let then = self.context.append_basic_block(function, "pool");
		let none = self.context.append_basic_block(function, "no_pool");
		builder.build_conditional_branch(present, then, none)?;
		builder.position_at_end(none);
		builder.build_return(Some(&pool))?;
		builder.position_at_end(then);
		let copy = self.collection_new(builder, repr, element)?;
		self.helper_pool_each(builder, function, pool, |builder, address| {
			let loaded = self.element_read(builder, place, address)?;
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
			self.element_write(builder, place, slot, copied)?;
			Ok(())
		})?;
		Ok(copy)
	}
}
