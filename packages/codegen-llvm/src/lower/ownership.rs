//! Owning values: copies, moves, drops (advanced brief §6, memory
//! management page).
//!
//! A value **owns** heap memory when it is a string with a heap block (its
//! capacity word is neither 0, a literal, nor `usize::MAX`, a view), an
//! array (its header and its items), a box (a named union, an error, an
//! anonymous union), a function value with an environment, or an object,
//! tuple or optional holding one of them. Each owning value has one owner:
//! a local, a field, an item, a box, an environment, or the statement that
//! computed it (a temporary).
//!
//! - **Generated functions**: `fa.drop.<shape>` and `fa.copy.<shape>`, one
//!   per representation, made on first use ([`Lowering::emit_drop`],
//!   [`Lowering::emit_copy`]). Every drop accepts a zero value (a null
//!   pointer, a string of capacity 0), which is how a moved-from local, a
//!   `none` and an uninitialized slot look: a local needs no drop flag.
//!   Boxes go through `fa.drop_box` / `fa.copy_box`, generated last with a
//!   case per member id (a member without data is a constant, never freed).
//! - **The cleanup stack** of the function being lowered: the owned locals
//!   in declaration order, and the temporaries of the statements being
//!   lowered. A block drops its locals in reverse order at its end; a
//!   statement drops its temporaries at its end; `break`, `continue`,
//!   `return` and a raised error drop what they leave
//!   ([`Lowering::drop_to`]). Code evaluated conditionally (a branch, the
//!   right operand of `and`) is a region of its own, so that a drop is
//!   always dominated by the value it drops.
//! - **Taking**: a value stored into an owner (a binding, an assignment, a
//!   field, an item, a box, a result) is taken ([`Lowering::take`]): a
//!   temporary is moved (unregistered), a local read for the last time is
//!   moved (its slot is zeroed), anything else (a place read) is copied.

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

/// An entry of the cleanup stack.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Cleanup<'ctx> {
	/// An owned local, dropped at the end of its scope.
	Local(Slot<'ctx>),
	/// An owned local that holds an owned value only when its flag is set
	/// (a parameter given its default value).
	Flagged(Slot<'ctx>, PointerValue<'ctx>),
	/// A temporary, dropped at the end of its statement.
	Temp(Val<'ctx>),
	/// A view of a `GrowingArray` as an array (its items not owned): its
	/// block and header freed at the end of its statement, with this item size.
	Shallow(PointerValue<'ctx>, IntValue<'ctx>),
	/// A `watch` block, disposed at the end of its scope.
	Dispose(PointerValue<'ctx>),
}

impl<'ctx> Lowering<'_, 'ctx> {
	// ── shapes ──────────────────────────────────────────────────────────

	/// Whether values of this representation may own heap memory.
	pub(crate) fn needs_drop(&self, repr: Repr) -> bool {
		match repr {
			Repr::Str
			| Repr::Array(_)
			| Repr::Union
			| Repr::Function(_)
			| Repr::Rec(_)
			| Repr::Map(_)
			| Repr::Set(_)
			| Repr::Growing(_)
			| Repr::Pool(_)
			| Repr::Sync(_)
			| Repr::Link(_)
			| Repr::Links(_)
			| Repr::Columnar(_)
			| Repr::Task(_) => true,
			Repr::Optional(scalar) => self.needs_drop(scalar.repr()),
			Repr::Object(ty) => {
				self.destroy_function(ty).is_some()
					|| self
						.object_fields(ty)
						.iter()
						.any(|repr| self.needs_drop(*repr))
			}
			Repr::Tuple(ty) => self
				.tuple_reprs(ty)
				.iter()
				.any(|repr| self.needs_drop(*repr)),
			_ => false,
		}
	}

	/// The representations of the fields of an object type, in shape order.
	/// A relation side is a [`Repr::Link`] or a [`Repr::Links`].
	pub(crate) fn object_fields(&self, ty: TypeId) -> Vec<Repr> {
		self.types()
			.fields_of(ty)
			.iter()
			.map(|field| {
				self.side_repr(ty, field.name)
					.unwrap_or_else(|| self.repr_of(field.ty, 0).unwrap_or(Repr::Bool))
			})
			.map(|repr| if repr.has_value() { repr } else { Repr::Bool })
			.collect()
	}

	/// The representations of the items of a tuple type.
	pub(crate) fn tuple_reprs(&self, ty: TypeId) -> Vec<Repr> {
		self.tuple_items(ty)
			.iter()
			.map(|item| self.repr_of(*item, 0).unwrap_or(Repr::Bool))
			.map(|repr| if repr.has_value() { repr } else { Repr::Bool })
			.collect()
	}

	/// The key of a representation in the caches of generated functions:
	/// with the bindings of the instance being lowered, since an object type
	/// with generic parameters has the representation of its bindings.
	fn shape_key(&self, repr: Repr) -> String {
		let bindings = self.bindings.borrow();
		if bindings.is_empty() {
			return format!("{repr:?}");
		}
		let mut bindings: Vec<String> = bindings
			.iter()
			.map(|(param, repr)| format!("{param:?}={repr:?}"))
			.collect();
		bindings.sort();
		format!("{repr:?}|{}", bindings.join(","))
	}

	// ── recursive edges ─────────────────────────────────────────────────

	/// The value a recursive edge holds: a view of it (the edge owns it).
	pub(crate) fn unfold_rec(
		&self,
		value: Val<'ctx>,
		target: TypeId,
		node: usize,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.repr_of(target, node)?;
		let pointer = value.value.expect("an edge").into_pointer_value();
		let loaded = self.builder.build_load(self.llvm_type(repr), pointer, "")?;
		Ok(Val::new(repr, loaded))
	}

	/// A value stored behind a recursive edge (`to`): a temporary moves into
	/// the new block, a place read is copied.
	pub(crate) fn box_rec(&self, value: Val<'ctx>, to: Repr) -> Result<Val<'ctx>, Error> {
		let region = self.arena_repr(value.repr) && !self.frame().arena;
		let owned = self.is_temporary(value);
		if region {
			if owned {
				let pointer =
					self.tree_root_pointer(&self.builder, value.repr, value.value.expect("root"))?;
				self.builder.build_call(
					self.runtime.function("fa_arena_enter_owned")?,
					&[pointer.into()],
					"",
				)?;
			} else {
				self.builder
					.build_call(self.runtime.function("fa_arena_enter")?, &[], "")?;
			}
		}
		let pointer = match self.pending_box.get() {
			Some((pointer, repr)) if repr == value.repr => {
				self.pending_box.set(None);
				pointer
			}
			_ => self.alloc_rec(value.repr)?,
		};
		let data = if owned { value } else { self.emit_copy(value)? };
		self.builder
			.build_store(pointer, data.value.expect("a value"))?;
		if region {
			self.builder.build_call(
				self.runtime.function("fa_arena_leave")?,
				&[pointer.into()],
				"",
			)?;
		}
		let edge = Val::new(to, pointer);
		if owned {
			self.transfer(value, edge);
			Ok(edge)
		} else {
			Ok(self.fresh(edge))
		}
	}

	/// A block for the box of a recursive edge to a value of this representation.
	pub(crate) fn alloc_rec(&self, repr: Repr) -> Result<PointerValue<'ctx>, Error> {
		if self.arena_repr(repr) {
			return self.allocate_tree_node(&self.builder, repr);
		}
		self.allocate_for(self.llvm_type(repr))
	}

	/// The recursive edges inside a value of this representation that a drop
	/// or a copy follows by iteration rather than recursion (the last one,
	/// directly in a field or in an optional field): its field index.
	fn last_edge(&self, repr: Repr) -> Option<usize> {
		let Repr::Object(ty) = repr else {
			return None;
		};
		self.object_fields(ty)
			.into_iter()
			.enumerate()
			.rev()
			.find_map(|(index, field)| match field {
				Repr::Rec(_) | Repr::Optional(super::values::Scalar::Rec(_)) => Some(index),
				_ => None,
			})
	}

	// ── drops and copies ────────────────────────────────────────────────

	/// Drops a value (nothing for a value that owns no memory).
	pub(crate) fn emit_drop(&self, value: Val<'ctx>) -> Result<(), Error> {
		let Some(raw) = value.value else {
			return Ok(());
		};
		if !self.needs_drop(value.repr) {
			return Ok(());
		}
		let function = self.drop_function(value.repr)?;
		self.builder.build_call(function, &[raw.into()], "")?;
		Ok(())
	}

	/// A copy of a value that owns its own memory.
	pub(crate) fn emit_copy(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let Some(raw) = value.value else {
			return Ok(value);
		};
		if !self.needs_drop(value.repr) {
			return Ok(value);
		}
		let function = self.copy_function(value.repr)?;
		let copied = self
			.builder
			.build_call(function, &[raw.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a copy");
		Ok(Val::new(value.repr, copied))
	}

	/// An internal function of the ownership code, with a builder at its entry.
	pub(crate) fn helper_function(
		&self,
		name: &str,
		r#type: inkwell::types::FunctionType<'ctx>,
	) -> (FunctionValue<'ctx>, Builder<'ctx>) {
		let function = self
			.module
			.add_function(name, r#type, Some(Linkage::Internal));
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		(function, builder)
	}

	/// `fa.drop.<shape>(value)`.
	pub(crate) fn drop_function(&self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		let key = self.shape_key(repr);
		if let Some(function) = self.drop_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let ty = self.llvm_type(repr);
		let r#type = self.context.void_type().fn_type(&[ty.into()], false);
		let (function, builder) = self.helper_function("fa.drop", r#type);
		self.drop_functions.borrow_mut().insert(key, function);
		let value = function.get_nth_param(0).expect("a value");
		self.build_drop_body(&builder, function, repr, value)?;
		builder.build_return(None)?;
		Ok(function)
	}

	/// `fa.copy.<shape>(value) -> value`.
	pub(crate) fn copy_function(&self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		let key = self.shape_key(repr);
		if let Some(function) = self.copy_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let ty = self.llvm_type(repr);
		let r#type = ty.fn_type(&[ty.into()], false);
		let (function, builder) = self.helper_function("fa.copy", r#type);
		if repr == Repr::Str {
			self.runtime.add_attributes(function, &["alwaysinline"]);
		}
		self.copy_functions.borrow_mut().insert(key, function);
		let value = function.get_nth_param(0).expect("a value");
		if self.arena_repr(repr) {
			builder.build_call(self.runtime.function("fa_arena_enter")?, &[], "")?;
		}
		let copied = self.build_copy_body(&builder, function, repr, value)?;
		if self.arena_repr(repr) {
			let pointer = self.tree_root_pointer(&builder, repr, copied)?;
			builder.build_call(
				self.runtime.function("fa_arena_leave")?,
				&[pointer.into()],
				"",
			)?;
		}
		builder.build_return(Some(&copied))?;
		Ok(function)
	}

	/// Calls the drop of a representation from a helper's builder.
	pub(crate) fn call_drop(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		if self.needs_drop(repr) {
			let function = self.drop_function(repr)?;
			builder.build_call(function, &[value.into()], "")?;
		}
		Ok(())
	}

	/// Calls the copy of a representation from a helper's builder.
	pub(crate) fn call_copy(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if !self.needs_drop(repr) {
			return Ok(value);
		}
		let function = self.copy_function(repr)?;
		Ok(builder
			.build_call(function, &[value.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a copy"))
	}

	/// Branches to a new block when `condition` holds, else returns: the
	/// builder continues in the new block.
	fn helper_when(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		condition: IntValue<'ctx>,
		otherwise: impl FnOnce(&Builder<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let then = self.context.append_basic_block(function, "then");
		let other = self.context.append_basic_block(function, "otherwise");
		builder.build_conditional_branch(condition, then, other)?;
		builder.position_at_end(other);
		otherwise(builder)?;
		builder.position_at_end(then);
		Ok(())
	}

	/// A loop over `0..length` in a helper (`body` gets the index).
	pub(crate) fn helper_loop(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		length: IntValue<'ctx>,
		mut body: impl FnMut(&Builder<'ctx>, IntValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let before = builder.get_insert_block().expect("a block");
		let test = self.context.append_basic_block(function, "each");
		let block = self.context.append_basic_block(function, "each_body");
		let end = self.context.append_basic_block(function, "each_end");
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(test);
		let index = builder.build_phi(i64, "index")?;
		index.add_incoming(&[(&i64.const_zero(), before)]);
		let current = index.as_basic_value().into_int_value();
		let more = builder.build_int_compare(IntPredicate::SLT, current, length, "")?;
		builder.build_conditional_branch(more, block, end)?;
		builder.position_at_end(block);
		body(builder, current)?;
		let next = builder.build_int_nsw_add(current, i64.const_int(1, false), "")?;
		let last = builder.get_insert_block().expect("a block");
		index.add_incoming(&[(&next, last)]);
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(end);
		Ok(())
	}

	fn build_drop_body(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		if self.arena_repr(repr) {
			let pointer = self.tree_root_pointer(builder, repr, value)?;
			builder.build_call(
				self.runtime.function("fa_arena_drop")?,
				&[pointer.into()],
				"",
			)?;
			return Ok(());
		}
		match repr {
			Repr::Str => {
				let pair = value.into_struct_value();
				let capacity = builder.build_extract_value(pair, 2, "")?.into_int_value();
				let kind = builder.build_and(capacity, self.string_kind_mask(), "")?;
				let inline = builder.build_and(capacity, self.string_inline_flag(), "")?;
				let external = builder.build_int_compare(
					IntPredicate::EQ,
					inline,
					self.usize.const_zero(),
					"",
				)?;
				let nonzero = builder.build_int_compare(
					IntPredicate::NE,
					kind,
					self.usize.const_zero(),
					"",
				)?;
				let not_view = builder.build_int_compare(
					IntPredicate::NE,
					kind,
					self.string_kind_mask(),
					"",
				)?;
				let owned = builder.build_and(nonzero, not_view, "")?;
				let owned = builder.build_and(owned, external, "")?;
				self.helper_when(builder, function, owned, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				let bytes = builder.build_extract_value(pair, 0, "")?;
				let one = self.usize.const_int(1, false);
				builder.build_call(
					self.runtime.free()?,
					&[bytes.into(), capacity.into(), one.into()],
					"",
				)?;
			}
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, NodeId::new(0))?;
				let array = value.into_pointer_value();
				let present = builder.build_is_not_null(array, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				if item == Repr::Str {
					let cap_pointer = builder.build_struct_gep(self.header_type(), array, 2, "")?;
					let cap = builder
						.build_load(self.context.i64_type(), cap_pointer, "")?
						.into_int_value();
					let ordinary = builder.build_int_compare(
						IntPredicate::SGE,
						cap,
						cap.get_type().const_zero(),
						"",
					)?;
					self.helper_when(builder, function, ordinary, |builder| {
						builder.build_call(
							self.runtime.function("fa_array_free")?,
							&[array.into(), self.item_size_const(item).into()],
							"",
						)?;
						builder.build_return(None)?;
						Ok(())
					})?;
				}
				if self.needs_drop(item) {
					let (items, length) = self.helper_array_parts(builder, array)?;
					let item_type = self.llvm_type(item);
					self.helper_loop(builder, function, length, |builder, index| {
						let pointer =
							unsafe { builder.build_in_bounds_gep(item_type, items, &[index], "")? };
						let loaded = builder.build_load(item_type, pointer, "")?;
						self.call_drop(builder, item, loaded)
					})?;
				}
				let size = self.item_size_const(item);
				builder.build_call(
					self.runtime.function("fa_array_free")?,
					&[array.into(), size.into()],
					"",
				)?;
			}
			Repr::Growing(item_type) => {
				let array = value.into_pointer_value();
				let present = builder.build_is_not_null(array, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				self.build_growing_drop(builder, function, item_type, array)?;
			}
			Repr::Union => {
				let pointer = value.into_pointer_value();
				builder.build_call(self.box_drop_function(), &[pointer.into()], "")?;
			}
			Repr::Map(_) | Repr::Set(_) => {
				self.build_table_drop(builder, function, repr, value)?;
			}
			Repr::Pool(_) | Repr::Columnar(_) => {
				self.build_pool_drop(builder, function, repr, value.into_pointer_value())?;
			}
			// a handle of `std.concurrency`, a guard (parallel.rs)
			Repr::Sync(ty) => {
				self.effectful_drops.set(true);
				self.build_sync_drop(builder, ty, value)?
			}
			// a task: cancelled when not done (tasks.rs)
			Repr::Task(_) => {
				self.effectful_drops.set(true);
				self.build_task_drop(builder, value.into_pointer_value())?
			}
			// a side's set of addresses (it owns no element)
			Repr::Links(_) => {
				builder.build_call(self.runtime.function("fa_links_free")?, &[value.into()], "")?;
			}
			Repr::Rec(target) => {
				// a loop along the last edge of each value, so that a long
				// list doesn't recurse as deep as it is long (advanced brief §6.4)
				let target_repr = self.repr_of(target, 0)?;
				let target_type = self.llvm_type(target_repr);
				let size = target_type.size_of().expect("a sized value");
				let size = builder.build_int_cast(size, self.usize, "")?;
				let align = self.usize.const_int(16, false);
				let before = builder.get_insert_block().expect("a block");
				let test = self.context.append_basic_block(function, "edge");
				let body = self.context.append_basic_block(function, "edge_body");
				let end = self.context.append_basic_block(function, "edge_end");
				builder.build_unconditional_branch(test)?;
				builder.position_at_end(test);
				let current = builder.build_phi(self.pointer_type(), "node")?;
				current.add_incoming(&[(&value, before)]);
				let pointer = current.as_basic_value().into_pointer_value();
				let present = builder.build_is_not_null(pointer, "")?;
				builder.build_conditional_branch(present, body, end)?;
				builder.position_at_end(body);
				let loaded = builder.build_load(target_type, pointer, "")?;
				let (rest, next) = match self.last_edge(target_repr) {
					Some(index) => {
						let structure = loaded.into_struct_value();
						let edge = builder.build_extract_value(structure, index as u32, "")?;
						let next = edge.into_pointer_value();
						let cleared: BasicValueEnum<'ctx> = self.pointer_type().const_null().into();
						let rest =
							builder.build_insert_value(structure, cleared, index as u32, "")?;
						(rest.into_struct_value().into(), next)
					}
					None => (loaded, self.pointer_type().const_null()),
				};
				self.call_drop(builder, target_repr, rest)?;
				builder.build_call(
					self.runtime.free()?,
					&[pointer.into(), size.into(), align.into()],
					"",
				)?;
				let last = builder.get_insert_block().expect("a block");
				current.add_incoming(&[(&next, last)]);
				builder.build_unconditional_branch(test)?;
				builder.position_at_end(end);
			}
			Repr::Function(_) => {
				let pair = value.into_struct_value();
				let environment = builder
					.build_extract_value(pair, 1, "")?
					.into_pointer_value();
				let present = builder.build_is_not_null(environment, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				// the environment starts with its drop and its copy
				let drop = builder
					.build_load(self.pointer_type(), environment, "")?
					.into_pointer_value();
				let r#type = self
					.context
					.void_type()
					.fn_type(&[self.pointer_type().into()], false);
				builder.build_indirect_call(r#type, drop, &[environment.into()], "")?;
			}
			Repr::Optional(scalar) => {
				let (present, inner) = self.split_optional(builder, Val::new(repr, value))?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				let inner = inner.value.expect("an optional value");
				self.call_drop(builder, scalar.repr(), inner)?;
			}
			Repr::Object(_) | Repr::Tuple(_) => {
				let fields = match repr {
					Repr::Object(ty) => self.object_fields(ty),
					Repr::Tuple(ty) => self.tuple_reprs(ty),
					_ => unreachable!(),
				};
				// a resource of foreign code: its `destroy` first (foreign.rs)
				if let Repr::Object(ty) = repr
					&& let Some(destroy) = self.destroy_function(ty)
				{
					self.effectful_drops.set(true);
					let argument = if destroy.get_type().get_param_types()[0].is_pointer_type() {
						let slot = builder.build_alloca(value.get_type(), "destroyed")?;
						builder.build_store(slot, value)?;
						slot.into()
					} else {
						value
					};
					let call = builder.build_call(destroy, &[argument.into()], "")?;
					call.set_call_convention(super::FAST_CALLING_CONVENTION);
				}
				let structure = value.into_struct_value();
				for (index, field) in fields.into_iter().enumerate().rev() {
					if self.needs_drop(field) {
						let raw = builder.build_extract_value(structure, index as u32, "")?;
						self.call_drop(builder, field, raw)?;
					}
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn build_copy_body(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		Ok(match repr {
			Repr::Str => {
				let pair = value.into_struct_value();
				let capacity = builder.build_extract_value(pair, 2, "")?.into_int_value();
				let inline = builder.build_and(capacity, self.string_inline_flag(), "")?;
				let external = builder.build_int_compare(
					IntPredicate::EQ,
					inline,
					self.usize.const_zero(),
					"",
				)?;
				self.helper_when(builder, function, external, |builder| {
					builder.build_return(Some(&pair))?;
					Ok(())
				})?;
				let length = builder.build_extract_value(pair, 1, "")?.into_int_value();
				let flag = builder.build_and(capacity, self.simple_flag(), "")?;
				// a literal (kind 0, whatever its `SIMPLE` flag) is shared, and
				// so is an empty string
				let kind =
					builder.build_left_shift(capacity, self.usize.const_int(1, false), "")?;
				let literal = builder.build_int_compare(
					IntPredicate::EQ,
					kind,
					self.usize.const_zero(),
					"",
				)?;
				let empty = builder.build_int_compare(
					IntPredicate::EQ,
					length,
					self.usize.const_zero(),
					"",
				)?;
				let shared = builder.build_or(literal, empty, "")?;
				let not_shared = builder.build_not(shared, "")?;
				self.helper_when(builder, function, not_shared, |builder| {
					let zeroed = builder.build_insert_value(pair, flag, 2, "")?;
					builder.build_return(Some(&zeroed))?;
					Ok(())
				})?;
				let slow = self.copy_string_slow()?;
				builder
					.build_call(slow, &[value.into()], "")?
					.try_as_basic_value()
					.basic()
					.expect("a copied string")
			}
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, NodeId::new(0))?;
				let array = value.into_pointer_value();
				let present = builder.build_is_not_null(array, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(Some(&array))?;
					Ok(())
				})?;
				let size = self.item_size_const(item);
				let copied = builder
					.build_call(
						self.runtime.function("fa_array_copy")?,
						&[array.into(), size.into()],
						"",
					)?
					.try_as_basic_value()
					.basic()
					.expect("an array")
					.into_pointer_value();
				if item == Repr::Str {
					let cap_pointer =
						builder.build_struct_gep(self.header_type(), copied, 2, "")?;
					let cap = builder
						.build_load(self.context.i64_type(), cap_pointer, "")?
						.into_int_value();
					let ordinary = builder.build_int_compare(
						IntPredicate::SGE,
						cap,
						cap.get_type().const_zero(),
						"",
					)?;
					self.helper_when(builder, function, ordinary, |builder| {
						builder.build_return(Some(&copied))?;
						Ok(())
					})?;
				}
				if self.needs_drop(item) {
					let (items, length) = self.helper_array_parts(builder, copied)?;
					let item_type = self.llvm_type(item);
					self.helper_loop(builder, function, length, |builder, index| {
						let pointer =
							unsafe { builder.build_in_bounds_gep(item_type, items, &[index], "")? };
						let loaded = builder.build_load(item_type, pointer, "")?;
						let deep = self.call_copy(builder, item, loaded)?;
						builder.build_store(pointer, deep)?;
						Ok(())
					})?;
				}
				copied.into()
			}
			Repr::Pool(_) | Repr::Columnar(_) => self
				.build_pool_copy(builder, function, repr, value.into_pointer_value())?
				.into(),
			// a copy of a handle designates the same value (parallel.rs)
			Repr::Sync(_) => self.build_sync_copy(builder, value)?,
			// a task is moved, never copied (the analyzer's `MOVE_ONLY`)
			Repr::Task(_) => value,
			// a copy has no links (D18)
			Repr::Link(_) | Repr::Links(_) => self.pointer_type().const_null().into(),
			Repr::Growing(item_type) => {
				let array = value.into_pointer_value();
				let present = builder.build_is_not_null(array, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(Some(&array))?;
					Ok(())
				})?;
				self.build_growing_copy(builder, function, item_type, array)?
					.into()
			}
			Repr::Union => {
				let pointer = value.into_pointer_value();
				builder
					.build_call(self.box_copy_function(), &[pointer.into()], "")?
					.try_as_basic_value()
					.basic()
					.expect("a box")
			}
			Repr::Map(_) | Repr::Set(_) => self.build_table_copy(builder, function, repr, value)?,
			Repr::Rec(target) => {
				// a loop along the last edge, as the drop: each copied value
				// gets its last edge from the next round
				let target_repr = self.repr_of(target, 0)?;
				let target_type = self.llvm_type(target_repr);
				let size = target_type.size_of().expect("a sized value");
				let size = builder.build_int_cast(size, self.usize, "")?;
				let align = self.usize.const_int(16, false);
				let pointer_type = self.pointer_type();
				let first = value.into_pointer_value();
				let present = builder.build_is_not_null(first, "")?;
				self.helper_when(builder, function, present, |builder| {
					if self.arena_object(target) {
						builder.build_call(
							self.runtime.function("fa_arena_leave")?,
							&[first.into()],
							"",
						)?;
					}
					builder.build_return(Some(&first))?;
					Ok(())
				})?;
				// where the next copy goes: a slot holding the first copy, then
				// the edge field of the last copy
				let root = builder.build_alloca(pointer_type, "root")?;
				let before = builder.get_insert_block().expect("a block");
				let test = self.context.append_basic_block(function, "edge");
				let body = self.context.append_basic_block(function, "edge_body");
				let end = self.context.append_basic_block(function, "edge_end");
				builder.build_unconditional_branch(test)?;
				builder.position_at_end(test);
				let source = builder.build_phi(pointer_type, "source")?;
				let destination = builder.build_phi(pointer_type, "destination")?;
				source.add_incoming(&[(&first, before)]);
				destination.add_incoming(&[(&root, before)]);
				let source_pointer = source.as_basic_value().into_pointer_value();
				let destination_pointer = destination.as_basic_value().into_pointer_value();
				let present = builder.build_is_not_null(source_pointer, "")?;
				builder.build_conditional_branch(present, body, end)?;
				builder.position_at_end(body);
				let copy = if self.arena_object(target) {
					self.allocate_tree_node(builder, target_repr)?
				} else {
					builder
						.build_call(self.runtime.alloc()?, &[size.into(), align.into()], "")?
						.try_as_basic_value()
						.basic()
						.expect("a block")
						.into_pointer_value()
				};
				builder.build_store(destination_pointer, copy)?;
				let loaded = builder.build_load(target_type, source_pointer, "")?;
				let (next, next_destination) = match self.last_edge(target_repr) {
					Some(index) => {
						let structure = loaded.into_struct_value();
						let edge = builder.build_extract_value(structure, index as u32, "")?;
						let rest = builder.build_insert_value(
							structure,
							pointer_type.const_null(),
							index as u32,
							"",
						)?;
						let copied =
							self.call_copy(builder, target_repr, rest.into_struct_value().into())?;
						builder.build_store(copy, copied)?;
						let field =
							builder.build_struct_gep(target_type, copy, index as u32, "")?;
						(edge.into_pointer_value(), field)
					}
					None => {
						let copied = self.call_copy(builder, target_repr, loaded)?;
						builder.build_store(copy, copied)?;
						(pointer_type.const_null(), root)
					}
				};
				let last = builder.get_insert_block().expect("a block");
				source.add_incoming(&[(&next, last)]);
				destination.add_incoming(&[(&next_destination, last)]);
				builder.build_unconditional_branch(test)?;
				builder.position_at_end(end);
				builder.build_load(pointer_type, root, "")?
			}
			Repr::Function(_) => {
				let pair = value.into_struct_value();
				let environment = builder
					.build_extract_value(pair, 1, "")?
					.into_pointer_value();
				let present = builder.build_is_not_null(environment, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(Some(&value))?;
					Ok(())
				})?;
				let copy_pointer =
					builder.build_struct_gep(self.environment_header(), environment, 1, "")?;
				let copy = builder
					.build_load(self.pointer_type(), copy_pointer, "")?
					.into_pointer_value();
				let r#type = self
					.pointer_type()
					.fn_type(&[self.pointer_type().into()], false);
				let copied = builder
					.build_indirect_call(r#type, copy, &[environment.into()], "")?
					.try_as_basic_value()
					.basic()
					.expect("an environment");
				builder
					.build_insert_value(pair, copied, 1, "")?
					.into_struct_value()
					.into()
			}
			Repr::Optional(scalar) => {
				let (present, inner) = self.split_optional(builder, Val::new(repr, value))?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(Some(&value))?;
					Ok(())
				})?;
				let inner = inner.value.expect("an optional value");
				let copied = self.call_copy(builder, scalar.repr(), inner)?;
				self.pack_optional(builder, scalar, present, copied)?
			}
			Repr::Object(_) | Repr::Tuple(_) => {
				let fields = match repr {
					Repr::Object(ty) => self.object_fields(ty),
					Repr::Tuple(ty) => self.tuple_reprs(ty),
					_ => unreachable!(),
				};
				let mut structure = value.into_struct_value();
				for (index, field) in fields.into_iter().enumerate() {
					if self.needs_drop(field) {
						let raw = builder.build_extract_value(structure, index as u32, "")?;
						let copied = self.call_copy(builder, field, raw)?;
						structure = builder
							.build_insert_value(structure, copied, index as u32, "")?
							.into_struct_value();
					}
				}
				// the links between the copied elements join the copies
				if let Repr::Object(ty) = repr
					&& self.owns_related_elements(ty)
				{
					structure = self.build_relink(
						builder,
						function,
						ty,
						value.into_struct_value(),
						structure,
					)?;
				}
				structure.into()
			}
			_ => value,
		})
	}

	/// Keep allocation and memcpy off the literal-copy path. Otherwise
	/// inlining the allocator makes the copy helper too large to inline,
	/// and even a static ASCII Character pays a call and several spills.
	fn copy_string_slow(&self) -> Result<FunctionValue<'ctx>, Error> {
		if let Some(function) = self.module.get_function("fa.copy_string_slow") {
			return Ok(function);
		}
		let ty = self.llvm_type(Repr::Str);
		let (function, builder) =
			self.helper_function("fa.copy_string_slow", ty.fn_type(&[ty.into()], false));
		self.runtime.add_attributes(function, &["noinline"]);
		let pair = function
			.get_nth_param(0)
			.expect("a string")
			.into_struct_value();
		let length = builder.build_extract_value(pair, 1, "")?.into_int_value();
		let capacity = builder.build_extract_value(pair, 2, "")?.into_int_value();
		let source = builder.build_extract_value(pair, 0, "")?;
		let out = builder.build_alloca(ty, "owned_string")?;
		builder.build_call(
			self.runtime.function("fa_string_own")?,
			&[source.into(), length.into(), capacity.into(), out.into()],
			"",
		)?;
		let copy = builder.build_load(ty, out, "")?;
		builder.build_return(Some(&copy))?;
		Ok(function)
	}

	/// The items and the length of an array, from a helper's builder.
	fn helper_array_parts(
		&self,
		builder: &Builder<'ctx>,
		array: PointerValue<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		let header = self.header_type();
		let items_pointer = builder.build_struct_gep(header, array, 0, "")?;
		let items = builder
			.build_load(self.pointer_type(), items_pointer, "items")?
			.into_pointer_value();
		let length_pointer = builder.build_struct_gep(header, array, 1, "")?;
		let length = builder
			.build_load(self.context.i64_type(), length_pointer, "length")?
			.into_int_value();
		Ok((items, length))
	}

	/// The size of an item as an `i64` constant.
	pub(crate) fn item_size_const(&self, item: Repr) -> IntValue<'ctx> {
		let size = self.llvm_type(item).size_of().expect("a sized item");
		// `LLVMSizeOf` is an i64 constant
		size
	}

	/// `{ ptr drop, ptr copy }`: the start of every closure environment.
	pub(crate) fn environment_header(&self) -> inkwell::types::StructType<'ctx> {
		let pointer = self.pointer_type();
		self.context
			.struct_type(&[pointer.into(), pointer.into()], false)
	}

	/// `fa.drop_box(ptr)`, defined last ([`Lowering::define_box_functions`]).
	pub(crate) fn box_drop_function(&self) -> FunctionValue<'ctx> {
		self.module.get_function("fa.drop_box").unwrap_or_else(|| {
			let r#type = self
				.context
				.void_type()
				.fn_type(&[self.pointer_type().into()], false);
			let function = self
				.module
				.add_function("fa.drop_box", r#type, Some(Linkage::Internal));
			self.runtime.add_attributes(function, &["nounwind"]);
			function
		})
	}

	/// `fa.copy_box(ptr) -> ptr`, defined last.
	pub(crate) fn box_copy_function(&self) -> FunctionValue<'ctx> {
		self.module.get_function("fa.copy_box").unwrap_or_else(|| {
			let r#type = self
				.pointer_type()
				.fn_type(&[self.pointer_type().into()], false);
			let function = self
				.module
				.add_function("fa.copy_box", r#type, Some(Linkage::Internal));
			self.runtime.add_attributes(function, &["nounwind"]);
			function
		})
	}

	/// Defines `fa.drop_box` and `fa.copy_box`: a switch on the id of the
	/// box, a case per member with data (and per member type of an anonymous
	/// union) that drops or copies the data and frees or allocates the box.
	/// A member without data is a constant box: neither freed nor copied.
	pub(crate) fn define_box_functions(
		&self,
		cases: &[(u64, Option<inkwell::types::StructType<'ctx>>, Vec<Repr>)],
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let drop = self.box_drop_function();
		if drop.count_basic_blocks() == 0 {
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(drop, "entry"));
			let pointer = drop.get_nth_param(0).expect("a box").into_pointer_value();
			let present = builder.build_is_not_null(pointer, "")?;
			self.helper_when(&builder, drop, present, |builder| {
				builder.build_return(None)?;
				Ok(())
			})?;
			let id = builder.build_load(i64, pointer, "id")?.into_int_value();
			let switch_block = builder.get_insert_block().expect("a block");
			let end = self.context.append_basic_block(drop, "end");
			let mut blocks = Vec::new();
			for (id, layout, fields) in cases {
				let Some(layout) = layout else { continue };
				let block = self.context.append_basic_block(drop, "drop_member");
				builder.position_at_end(block);
				if self.arena_box(*id) {
					builder.build_call(
						self.runtime.function("fa_arena_drop")?,
						&[pointer.into()],
						"",
					)?;
					builder.build_unconditional_branch(end)?;
					blocks.push((i64.const_int(*id, false), block));
					continue;
				}
				for (index, field) in fields.iter().enumerate().rev() {
					if self.needs_drop(*field) {
						let field_pointer =
							builder.build_struct_gep(*layout, pointer, index as u32 + 1, "")?;
						let loaded =
							builder.build_load(self.llvm_type(*field), field_pointer, "")?;
						self.call_drop(&builder, *field, loaded)?;
					}
				}
				let size = layout.size_of().expect("a sized box");
				let size = builder.build_int_cast(size, self.usize, "")?;
				let align = self.usize.const_int(16, false);
				builder.build_call(
					self.runtime.free()?,
					&[pointer.into(), size.into(), align.into()],
					"",
				)?;
				builder.build_unconditional_branch(end)?;
				blocks.push((i64.const_int(*id, false), block));
			}
			builder.position_at_end(switch_block);
			builder.build_switch(id, end, &blocks)?;
			builder.position_at_end(end);
			builder.build_return(None)?;
		}
		let copy = self.box_copy_function();
		if copy.count_basic_blocks() == 0 {
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(copy, "entry"));
			let pointer = copy.get_nth_param(0).expect("a box").into_pointer_value();
			let present = builder.build_is_not_null(pointer, "")?;
			self.helper_when(&builder, copy, present, |builder| {
				builder.build_return(Some(&pointer))?;
				Ok(())
			})?;
			let id = builder.build_load(i64, pointer, "id")?.into_int_value();
			let switch_block = builder.get_insert_block().expect("a block");
			let end = self.context.append_basic_block(copy, "shared");
			let mut blocks = Vec::new();
			for (id, layout, fields) in cases {
				let Some(layout) = layout else { continue };
				let block = self.context.append_basic_block(copy, "copy_member");
				builder.position_at_end(block);
				let region = self.arena_box(*id);
				if region {
					builder.build_call(self.runtime.function("fa_arena_enter")?, &[], "")?;
				}
				let size = layout.size_of().expect("a sized box");
				let size = builder.build_int_cast(size, self.usize, "")?;
				let align = self.usize.const_int(16, false);
				let copied = if region {
					self.allocate_tree_type(&builder, (*layout).into())?
				} else {
					builder
						.build_call(self.runtime.alloc()?, &[size.into(), align.into()], "")?
						.try_as_basic_value()
						.basic()
						.expect("a box")
						.into_pointer_value()
				};
				builder.build_store(copied, i64.const_int(*id, false))?;
				for (index, field) in fields.iter().enumerate() {
					let from = builder.build_struct_gep(*layout, pointer, index as u32 + 1, "")?;
					let to = builder.build_struct_gep(*layout, copied, index as u32 + 1, "")?;
					let loaded = builder.build_load(self.llvm_type(*field), from, "")?;
					let deep = self.call_copy(&builder, *field, loaded)?;
					builder.build_store(to, deep)?;
				}
				if region {
					self.register_arena_characters(&builder, *id, copied, *layout, fields)?;
					builder.build_call(
						self.runtime.function("fa_arena_leave")?,
						&[copied.into()],
						"",
					)?;
				}
				builder.build_return(Some(&copied))?;
				blocks.push((i64.const_int(*id, false), block));
			}
			builder.position_at_end(switch_block);
			builder.build_switch(id, end, &blocks)?;
			builder.position_at_end(end);
			builder.build_return(Some(&pointer))?;
		}
		Ok(())
	}

	// ── the cleanup stack ───────────────────────────────────────────────

	/// The depth of the cleanup stack: where a scope or a statement starts.
	pub(crate) fn mark(&self) -> usize {
		self.cleanups.borrow().len()
	}

	/// Registers a value computed by the statement being lowered: it is
	/// dropped at the end of the statement unless taken before.
	pub(crate) fn fresh(&self, value: Val<'ctx>) -> Val<'ctx> {
		if value.value.is_some() && self.needs_drop(value.repr) {
			self.cleanups.borrow_mut().push(Cleanup::Temp(value));
		}
		value
	}

	/// Unregisters a temporary: whether the value was one.
	fn unregister(&self, value: Val<'ctx>) -> bool {
		let Some(raw) = value.value else {
			return false;
		};
		let mut cleanups = self.cleanups.borrow_mut();
		let position = cleanups
			.iter()
			.rposition(|cleanup| matches!(cleanup, Cleanup::Temp(temp) if temp.value == Some(raw)));
		match position {
			Some(position) => {
				cleanups.remove(position);
				true
			}
			None => false,
		}
	}

	/// Whether a value is a registered temporary.
	pub(crate) fn is_temporary(&self, value: Val<'ctx>) -> bool {
		let Some(raw) = value.value else {
			return false;
		};
		self.cleanups
			.borrow()
			.iter()
			.any(|cleanup| matches!(cleanup, Cleanup::Temp(temp) if temp.value == Some(raw)))
	}

	/// Replaces the registration of a temporary by another value that took
	/// its memory (an optional or a box made of it).
	pub(crate) fn transfer(&self, from: Val<'ctx>, to: Val<'ctx>) {
		if self.unregister(from) {
			self.fresh(to);
		}
	}

	/// Takes a value to store it into an owner: moves a temporary or a local
	/// read for the last time (`node`), copies anything else.
	pub(crate) fn take(&self, value: Val<'ctx>, node: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		if value.value.is_none() || !self.needs_drop(value.repr) {
			return Ok(value);
		}
		if self.unregister(value) {
			return Ok(value);
		}
		if let Some(node) = node
			&& let Some(slot) = self.movable_local(node, value.repr)
		{
			// the local gives its value up: its drop sees a zero
			self.builder
				.build_store(slot.pointer, self.llvm_type(slot.repr).const_zero())?;
			if let Some(&flag) = self.resource_flags.borrow().get(&slot.pointer) {
				self.builder
					.build_store(flag, self.context.bool_type().const_zero())?;
			}
			return Ok(value);
		}
		self.emit_copy(value)
	}

	/// Takes a value, then registers it as a temporary of the enclosing
	/// statement: the value of a block, of an `if`.
	pub(crate) fn own(&self, value: Val<'ctx>, node: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		let owned = self.take(value, node)?;
		Ok(self.fresh(owned))
	}

	/// Starts owning a local whose slot was just initialized.
	pub(crate) fn own_local(&mut self, symbol: SymbolId, slot: Slot<'ctx>) {
		if !self.needs_drop(slot.repr) {
			return;
		}
		// a resource of foreign code: a drop flag, cleared when the local
		// moves out (`let second = first`, a returned value), since its
		// `destroy` must not run on the zeroed slot
		if let Repr::Object(ty) = slot.repr
			&& self.destroy_function(ty).is_some()
			&& let Ok(flag) = self.alloca(Repr::Bool, "owns_resource")
			&& self
				.builder
				.build_store(flag.pointer, self.context.bool_type().const_int(1, false))
				.is_ok()
		{
			self.cleanups
				.borrow_mut()
				.push(Cleanup::Flagged(slot, flag.pointer));
			self.resource_flags
				.borrow_mut()
				.insert(slot.pointer, flag.pointer);
			self.owned_locals.insert(symbol, slot);
			return;
		}
		self.cleanups.borrow_mut().push(Cleanup::Local(slot));
		self.owned_locals.insert(symbol, slot);
	}

	/// Starts owning a local when `flag` is set at run time.
	pub(crate) fn own_flagged(&self, slot: Slot<'ctx>, flag: PointerValue<'ctx>) {
		if self.needs_drop(slot.repr) {
			self.cleanups
				.borrow_mut()
				.push(Cleanup::Flagged(slot, flag));
		}
	}

	/// Emits the drop of a cleanup entry.
	fn drop_entry(&self, entry: Cleanup<'ctx>) -> Result<(), Error> {
		match entry {
			Cleanup::Temp(value) => self.emit_drop(value),
			Cleanup::Shallow(view, size) => {
				self.builder.build_call(
					self.runtime.function("fa_array_free")?,
					&[view.into(), size.into()],
					"",
				)?;
				Ok(())
			}
			Cleanup::Local(slot) => {
				let value = self.load(slot)?;
				self.emit_drop(value)
			}
			Cleanup::Dispose(observer) => {
				self.effectful_drops.set(true);
				self.builder.build_call(
					self.runtime.function("fa_dispose")?,
					&[observer.into()],
					"",
				)?;
				Ok(())
			}
			Cleanup::Flagged(slot, flag) => {
				let set = self
					.builder
					.build_load(self.context.bool_type(), flag, "")?
					.into_int_value();
				let function = self.builder.get_insert_block().expect("a block");
				let function = function.get_parent().expect("a function");
				let then = self.context.append_basic_block(function, "drop_flagged");
				let end = self.context.append_basic_block(function, "dropped_flagged");
				self.builder.build_conditional_branch(set, then, end)?;
				self.builder.position_at_end(then);
				let value = self.load(slot)?;
				self.emit_drop(value)?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				Ok(())
			}
		}
	}

	/// Whether the current block already ended (after a `return`…).
	pub(crate) fn terminated(&self) -> bool {
		self.builder
			.get_insert_block()
			.is_none_or(|block| block.get_terminator().is_some())
	}

	/// Ends a scope: drops what it registered after `mark`, in reverse order,
	/// but `keep` (the value of the scope), which stays registered.
	pub(crate) fn end_scope(&self, mark: usize, keep: Option<Val<'ctx>>) -> Result<(), Error> {
		let entries = {
			let mut cleanups = self.cleanups.borrow_mut();
			let mark = mark.min(cleanups.len());
			cleanups.split_off(mark)
		};
		let keep = keep.and_then(|value| value.value);
		let mut kept = None;
		for entry in entries.into_iter().rev() {
			if let Cleanup::Temp(temp) = entry
				&& keep.is_some()
				&& temp.value == keep
				&& kept.is_none()
			{
				kept = Some(temp);
				continue;
			}
			if !self.terminated() {
				self.drop_entry(entry)?;
			}
		}
		if let Some(kept) = kept {
			self.cleanups.borrow_mut().push(Cleanup::Temp(kept));
		}
		Ok(())
	}

	/// Ends a statement: drops its temporaries (but `keep`), and keeps the
	/// locals it declared for the end of their block.
	pub(crate) fn end_statement(&self, mark: usize, keep: Option<Val<'ctx>>) -> Result<(), Error> {
		let entries = {
			let mut cleanups = self.cleanups.borrow_mut();
			let mark = mark.min(cleanups.len());
			cleanups.split_off(mark)
		};
		let keep = keep.and_then(|value| value.value);
		let mut kept = Vec::new();
		for entry in entries.into_iter().rev() {
			match entry {
				Cleanup::Temp(temp) if keep.is_none() || temp.value != keep => {
					if !self.terminated() {
						self.drop_entry(entry)?;
					}
				}
				Cleanup::Shallow(..) => {
					if !self.terminated() {
						self.drop_entry(entry)?;
					}
				}

				other => kept.push(other),
			}
		}
		kept.reverse();
		self.cleanups.borrow_mut().extend(kept);
		Ok(())
	}

	/// Emits the drops of everything registered after `mark`, for a jump out
	/// of the scopes (`break`, `return`, a raised error); the entries stay
	/// registered for the code that follows in the scopes.
	pub(crate) fn drop_to(&self, mark: usize) -> Result<(), Error> {
		let entries: Vec<Cleanup<'ctx>> = {
			let cleanups = self.cleanups.borrow();
			cleanups[mark.min(cleanups.len())..].to_vec()
		};
		for entry in entries.into_iter().rev() {
			if self.terminated() {
				break;
			}
			self.drop_entry(entry)?;
		}
		Ok(())
	}

	/// Lowers code evaluated conditionally (a branch, a right operand) as a
	/// region: the temporaries it registers are dropped at its end, but its
	/// value when it is kept (`target`: converted to that representation and
	/// taken, so the caller owns it).
	pub(crate) fn region(
		&mut self,
		target: Option<Repr>,
		node: NodeId,
		body: impl FnOnce(&mut Self) -> Result<Val<'ctx>, Error>,
	) -> Result<Val<'ctx>, Error> {
		let mark = self.mark();
		let value = body(self)?;
		let value = match target {
			Some(repr) if value.repr.has_value() => self.owned_as(value, repr, Some(node))?,
			_ => value,
		};
		self.end_statement(mark, None)?;
		Ok(value)
	}

	/// A value converted to the representation of the owner it is stored
	/// into, and taken: owned first (a temporary or a local read for the last
	/// time moves, a place read is copied), then converted (a conversion that
	/// boxes or wraps moves what it is given).
	pub(crate) fn owned_as(
		&self,
		value: Val<'ctx>,
		repr: Repr,
		node: Option<NodeId>,
	) -> Result<Val<'ctx>, Error> {
		if !repr.has_value() || value.repr == Repr::Never {
			return self.take(value, node);
		}
		let index = node.map_or(0, NodeId::index);
		if !self.needs_drop(value.repr) {
			let converted = self.convert(value, repr, index)?;
			return self.take(converted, None);
		}
		let owned = self.own(value, node)?;
		let converted = self.convert(owned, repr, index)?;
		self.take(converted, None)
	}

	// ── last uses ───────────────────────────────────────────────────────

	/// The slot of an owned local whose read at `node` is its last use, so
	/// that taking it moves it (analyzer's `MOVE` flag, or a conservative
	/// local analysis: the last reference in the source, not in a loop or a
	/// lambda that its declaration is outside of, not in an assignment whose
	/// target reads it).
	fn movable_local(&self, node: NodeId, repr: Repr) -> Option<Slot<'ctx>> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		if !matches!(self.view(inner), View::Name(_)) {
			return None;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(inner) else {
			return None;
		};
		let slot = *self.owned_locals.get(&symbol)?;
		if self.shared.contains_key(&symbol) {
			return None;
		}
		// the whole value, or the value of an optional
		let whole = slot.repr == repr
			|| matches!(slot.repr, Repr::Optional(scalar) if scalar.repr() == repr);
		if !whole {
			return None;
		}
		let data = self.analysis.symbols.symbol(symbol);
		if !matches!(data.kind, SymbolKind::Let | SymbolKind::Mutable) {
			return None;
		}
		if self.analysis.flags_of(inner).contains(NodeFlags::MOVE) {
			return Some(slot);
		}
		if self.is_last_use(symbol, inner, data.decl_node) {
			Some(slot)
		} else {
			None
		}
	}

	fn is_last_use(&self, symbol: SymbolId, node: NodeId, declaration: NodeId) -> bool {
		// a reference into the local (`use ana = town.people.add(…)`) may
		// still read it after its last textual read
		if self.referenced_locals().contains(&symbol) {
			return false;
		}
		let uses = self.uses_of(symbol);
		let start = self.tree.spans[node.index()].start;
		if self.reassigned_after(symbol, node, &uses) {
			return true;
		}
		// a later read in another branch of an `if` or a `when` never runs
		// after this one
		if uses.iter().any(|other| {
			*other != node
				&& self.tree.spans[other.index()].start > start
				&& !self.exclusive(node, *other)
		}) {
			return false;
		}
		let declared = self.tree.spans[declaration.index()];
		let parents = self.parents();
		let mut current = node.index();
		while let Some(&parent) = parents.get(current) {
			if parent == usize::MAX {
				break;
			}
			let span = self.tree.spans[parent];
			if span.start <= declared.start && declared.end <= span.end {
				break;
			}
			match self.view(NodeId::new(parent)) {
				View::While { .. }
				| View::WhileLet { .. }
				| View::For { .. }
				| View::Loop(_)
				| View::Lambda { .. } => {
					return false;
				}
				View::Function { .. } => return false,
				View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
					// `items[s.length] = s`: the target is evaluated after the value
					let reads = uses.iter().any(|other| {
						let span = self.tree.spans[other.index()];
						let target = self.tree.spans[target.index()];
						target.start <= span.start && span.end <= target.end
					});
					let plain = matches!(self.view(target), View::Name(_));
					if reads && !plain {
						return false;
					}
				}
				_ => {}
			}
			current = parent;
		}
		true
	}

	/// Whether two nodes are in different branches of an `if` or of a `when`
	/// (their closest common ancestor), so that at most one of them runs.
	fn exclusive(&self, first: NodeId, second: NodeId) -> bool {
		let parents = self.parents();
		let chain = |node: usize| {
			let mut chain = vec![node];
			let mut current = node;
			while let Some(&parent) = parents.get(current) {
				if parent == usize::MAX {
					break;
				}
				chain.push(parent);
				current = parent;
			}
			chain
		};
		let first_chain = chain(first.index());
		let second_chain = chain(second.index());
		// the closest common ancestor, and the child of it above each node
		let Some((position, &common)) = first_chain
			.iter()
			.enumerate()
			.find(|(_, ancestor)| second_chain.contains(ancestor))
		else {
			return false;
		};
		let Some(second_position) = second_chain.iter().position(|&node| node == common) else {
			return false;
		};
		if position == 0 || second_position == 0 {
			return false;
		}
		let first_child = first_chain[position - 1];
		let second_child = second_chain[second_position - 1];
		match self.view(NodeId::new(common)) {
			View::If {
				then_body,
				else_body: Some(else_body),
				..
			} => {
				let branches = [then_body.index(), else_body.index()];
				branches.contains(&first_child)
					&& branches.contains(&second_child)
					&& first_child != second_child
			}
			View::When { arms, .. } => {
				let values: Vec<usize> = arms.iter().map(|arm| arm.value.index()).collect();
				values.contains(&first_child)
					&& values.contains(&second_child)
					&& first_child != second_child
			}
			_ => false,
		}
	}

	/// `x = f(x)`, `head = Link { next = head }`: the read is the last one
	/// of the value of an assignment to the local itself, which gives the
	/// local a new value right after (in a loop too).
	fn reassigned_after(&self, symbol: SymbolId, node: NodeId, uses: &[NodeId]) -> bool {
		let start = self.tree.spans[node.index()].start;
		let parents = self.parents();
		let mut current = node.index();
		while let Some(&parent) = parents.get(current) {
			if parent == usize::MAX {
				return false;
			}
			match self.view(NodeId::new(parent)) {
				View::While { .. }
				| View::WhileLet { .. }
				| View::For { .. }
				| View::Loop(_)
				| View::Lambda { .. }
				| View::Function { .. } => return false,
				View::Assignment {
					target,
					value: Some(value),
					..
				} if value.index() == current => {
					let same = matches!(self.view(target), View::Name(_))
						&& self.analysis.resolution(target) == Resolution::Local(symbol);
					if !same {
						// `{ next = head }`: a field of an object literal
						let literal = parents.get(parent).is_some_and(|&literal| {
							literal != usize::MAX
								&& matches!(
									self.view(NodeId::new(literal)),
									View::Object { .. } | View::TypedObject { .. }
								)
						});
						if literal {
							current = parent;
							continue;
						}
						return false;
					}
					let span = self.tree.spans[value.index()];
					// no later read in the value
					return !uses.iter().any(|other| {
						let other = self.tree.spans[other.index()];
						other.start > start && other.end <= span.end
					});
				}
				_ => {}
			}
			current = parent;
		}
		false
	}

	/// The locals that a reference designates a part of (`use`, `reuse`,
	/// `provide` of a place below them).
	fn referenced_locals(&self) -> std::cell::Ref<'_, rustc_hash::FxHashSet<SymbolId>> {
		if self.referenced.borrow().is_none() {
			let mut referenced = rustc_hash::FxHashSet::default();
			for index in 0..self.tree.nodes.len() {
				let value = match self.view(NodeId::new(index)) {
					View::Use { value, .. }
					| View::Reuse { value, .. }
					| View::Provide { value, .. } => value,
					_ => continue,
				};
				// the root of the place
				let mut current = value;
				loop {
					current = match self.view(current) {
						View::Group(inner) => inner,
						View::Access { target, .. } | View::Index { target, .. } => target,
						View::Call { callee, .. } => callee,
						_ => break,
					};
				}
				if let Resolution::Local(symbol) = self.analysis.resolution(current) {
					referenced.insert(symbol);
				}
			}
			*self.referenced.borrow_mut() = Some(referenced);
		}
		std::cell::Ref::map(self.referenced.borrow(), |referenced| {
			referenced.as_ref().expect("computed above")
		})
	}

	/// The nodes that read a local.
	fn uses_of(&self, symbol: SymbolId) -> Vec<NodeId> {
		let mut uses = self.local_uses.borrow_mut();
		if uses.is_empty() {
			for (index, resolution) in self.analysis.resolutions.iter().enumerate() {
				if let Resolution::Local(symbol) = resolution {
					uses.entry(*symbol).or_default().push(NodeId::new(index));
				}
			}
		}
		uses.get(&symbol).cloned().unwrap_or_default()
	}

	/// The parent of every node (`usize::MAX` for the root).
	pub(crate) fn parents(&self) -> std::cell::Ref<'_, Vec<usize>> {
		if self.node_parents.borrow().is_empty() {
			let mut parents = vec![usize::MAX; self.tree.nodes.len()];
			for (index, node) in self.tree.nodes.iter().enumerate() {
				node.for_each_child(|child| {
					if child < parents.len() {
						parents[child] = index;
					}
				});
			}
			*self.node_parents.borrow_mut() = parents;
		}
		self.node_parents.borrow()
	}
}
