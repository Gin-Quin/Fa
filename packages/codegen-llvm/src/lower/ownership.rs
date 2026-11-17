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

/// The drop flag of a field that may move out of a local: its path of
/// field indices from the local, and the flag (set while the local owns it).
#[derive(Clone, Debug)]
pub(crate) struct FieldFlag<'ctx> {
	pub(crate) path: Vec<u32>,
	pub(crate) flag: PointerValue<'ctx>,
}

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
	/// Retain the memory ABI of wide helper calls through optimization.
	/// Inlining their aggregate loads/stores expands thousands of unused
	/// slots into SSA values, even when the initialized prefix is tiny.
	pub(super) fn keep_aggregate_helper(&self, function: FunctionValue<'ctx>, repr: Repr) {
		if self.wide_aggregate(repr) {
			self.runtime.add_attributes(function, &["noinline"]);
		}
	}

	pub(super) fn wide_aggregate(&self, repr: Repr) -> bool {
		if !repr.has_value() {
			return false;
		}
		let ty = self.llvm_type(repr);
		ty.is_struct_type() && self.collection_data_size(ty) > 256
	}
	/// Helper functions borrow aggregates by address, independently of their
	/// width. In particular wasm must not flatten capacity into parameter count.
	pub(super) fn helper_argument(
		&self,
		builder: &Builder<'ctx>,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if !value.is_struct_value() {
			return Ok(value);
		}
		if let Some(pointer) = crate::optimize::aggregate_snapshot(value) {
			return Ok(pointer.into());
		}
		let slot = self.helper_slot(builder, value.get_type())?;
		builder.build_store(slot, value)?;
		Ok(slot.into())
	}

	fn helper_slot(
		&self,
		builder: &Builder<'ctx>,
		ty: inkwell::types::BasicTypeEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let entry = builder
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function")
			.get_first_basic_block()
			.expect("an entry");
		let allocations = self.context.create_builder();
		match entry.get_first_instruction() {
			Some(first) => allocations.position_before(&first),
			None => allocations.position_at_end(entry),
		}
		Ok(allocations.build_alloca(ty, "aggregate_argument")?)
	}

	pub(super) fn helper_parameter(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if self.llvm_type(repr).is_struct_type() {
			let mut pointer = value.into_pointer_value();
			let snapshot = self.wide_aggregate(repr);
			if snapshot {
				let target = self.helper_slot(builder, self.llvm_type(repr))?;
				builder.build_memcpy(
					target,
					1,
					pointer,
					1,
					self.usize
						.const_int(self.collection_data_size(self.llvm_type(repr)), false),
				)?;
				pointer = target;
			}
			let loaded = builder.build_load(self.llvm_type(repr), pointer, "")?;
			if snapshot {
				crate::optimize::mark_aggregate_snapshot(loaded)?;
			}
			Ok(self.freeze_union_padding(builder, repr, loaded))
		} else {
			Ok(value)
		}
	}

	// ── shapes ──────────────────────────────────────────────────────────

	/// Whether values of this representation may own heap memory.
	pub(crate) fn needs_drop(&self, repr: Repr) -> bool {
		match repr {
			Repr::Str
			| Repr::Bytes
			| Repr::BigInt
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
			Repr::InlineUnion(nominal) => self.inline_union_needs_drop(nominal),
			Repr::Optional(scalar) => self.needs_drop(scalar.repr()),
			Repr::Object(ty) => {
				self.destroy_function(ty).is_some()
					|| self
						.object_fields(ty)
						.iter()
						.any(|repr| self.needs_drop(*repr))
			}
			Repr::Tuple(ty) if self.inline_info(ty).is_some() => {
				let (item, capacity) = self.inline_info(ty).expect("an inline array");
				capacity != 0
					&& self
						.item_repr(item, NodeId::new(0))
						.is_ok_and(|item| self.needs_drop(item))
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
	pub(super) fn shape_key(&self, repr: Repr) -> String {
		let bindings = self.bindings.borrow();
		let types = self.type_bindings.borrow();
		if bindings.is_empty() && types.is_empty() {
			return format!("{repr:?}");
		}
		let mut bindings: Vec<String> = bindings
			.iter()
			.map(|(param, repr)| format!("{param:?}={repr:?}"))
			.collect();
		// Equal scalar representations can have different map exclusions
		// (`Integer` and `Int64`, for example). Helpers must retain the exact
		// instance whose table layout they compiled, also in nested owners.
		bindings.extend(
			types
				.iter()
				.map(|(param, ty)| format!("{param:?}:type={ty:?}")),
		);
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
		let pointer = self.edge_address(
			&self.builder,
			Repr::Rec(target),
			value.value.expect("an edge"),
		)?;
		if self.compressed_union_link(Repr::Rec(target)).is_some() {
			return Ok(Val::new(Repr::Union, pointer));
		}
		let loaded = self.builder.build_load(self.llvm_type(repr), pointer, "")?;
		Ok(Val::new(repr, loaded))
	}

	/// A value stored behind a recursive edge (`to`): a temporary moves into
	/// the new block, a place read is copied.
	pub(crate) fn box_rec(&self, value: Val<'ctx>, to: Repr) -> Result<Val<'ctx>, Error> {
		if self.compressed_union_link(to).is_some() {
			let raw = self.address_edge(
				&self.builder,
				to,
				value.value.expect("a root").into_pointer_value(),
			)?;
			let result = Val::new(to, raw);
			self.transfer(value, result);
			return Ok(result);
		}
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
		if data.destination != Some(pointer) {
			self.builder
				.build_store(pointer, data.value.expect("a value"))?;
		}
		if region {
			self.builder.build_call(
				self.runtime.function("fa_arena_leave")?,
				&[pointer.into()],
				"",
			)?;
		}
		let edge = Val::new(to, self.address_edge(&self.builder, to, pointer)?);
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
			let node = self.allocate_tree_node(&self.builder, repr)?;
			return Ok(self.through_links(&self.builder, repr, node, &[0])?[0]);
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
		self.invalidate_dropped_map_probes(value.repr)?;
		let Some(raw) = value.value else {
			return Ok(());
		};
		if !self.needs_drop(value.repr) {
			return Ok(());
		}
		self.call_drop(&self.builder, value.repr, raw)
	}

	/// A copy of a value that owns its own memory.
	pub(crate) fn emit_copy(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let Some(raw) = value.value else {
			return Ok(value);
		};
		if !self.needs_drop(value.repr) {
			return Ok(value);
		}
		let copied = self.call_copy(&self.builder, value.repr, raw)?;
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
		let argument = if ty.is_struct_type() {
			self.pointer_type().into()
		} else {
			ty
		};
		let r#type = self.context.void_type().fn_type(&[argument.into()], false);
		let (function, builder) = self.helper_function("fa.drop", r#type);
		self.keep_aggregate_helper(function, repr);
		self.drop_functions.borrow_mut().insert(key, function);
		let value = function.get_nth_param(0).expect("a value");
		if let Repr::Tuple(ty) = repr
			&& self.inline_info(ty).is_some()
		{
			self.inline_ownership(&builder, function, ty, value.into_pointer_value(), None)?;
			builder.build_return(None)?;
			return Ok(function);
		}
		let value = self.helper_parameter(&builder, repr, value)?;
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
		let argument = if ty.is_struct_type() {
			self.pointer_type().into()
		} else {
			ty
		};
		let r#type = if ty.is_struct_type() {
			self.context
				.void_type()
				.fn_type(&[self.pointer_type().into(), argument.into()], false)
		} else {
			ty.fn_type(&[argument.into()], false)
		};
		let (function, builder) = self.helper_function("fa.copy", r#type);
		self.keep_aggregate_helper(function, repr);
		if ty.is_struct_type() {
			self.add_return_slot_attribute(function, repr, false, 0);
		}
		if repr == Repr::Str {
			self.runtime.add_attributes(function, &["alwaysinline"]);
		}
		self.copy_functions.borrow_mut().insert(key, function);
		let value = function
			.get_nth_param(u32::from(ty.is_struct_type()))
			.expect("a value");
		if let Repr::Tuple(ty) = repr
			&& self.inline_info(ty).is_some()
		{
			let out = function
				.get_nth_param(0)
				.expect("a destination")
				.into_pointer_value();
			self.inline_ownership(
				&builder,
				function,
				ty,
				value.into_pointer_value(),
				Some(out),
			)?;
			builder.build_return(None)?;
			return Ok(function);
		}
		let value = self.helper_parameter(&builder, repr, value)?;
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
		if ty.is_struct_type() {
			let out = function
				.get_nth_param(0)
				.expect("a destination")
				.into_pointer_value();
			builder.build_store(out, copied)?;
			builder.build_return(None)?;
		} else {
			builder.build_return(Some(&copied))?;
		}
		Ok(function)
	}

	/// Drop a value already in a place without loading a wide aggregate.
	pub(super) fn drop_at(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		source: PointerValue<'ctx>,
	) -> Result<(), Error> {
		if !self.needs_drop(repr) {
			return Ok(());
		}
		let argument = if self.llvm_type(repr).is_struct_type() {
			source.into()
		} else {
			builder.build_load(self.llvm_type(repr), source, "")?
		};
		builder.build_call(self.drop_function(repr)?, &[argument.into()], "")?;
		Ok(())
	}

	/// Construct an owning copy directly in its destination.
	pub(super) fn copy_into(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		source: PointerValue<'ctx>,
		destination: PointerValue<'ctx>,
	) -> Result<(), Error> {
		if self.needs_drop(repr) && self.llvm_type(repr).is_struct_type() {
			builder.build_call(
				self.copy_function(repr)?,
				&[destination.into(), source.into()],
				"",
			)?;
		} else {
			let value = builder.build_load(self.llvm_type(repr), source, "")?;
			let value = self.call_copy(builder, repr, value)?;
			builder.build_store(destination, value)?;
		}
		Ok(())
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
			let value = self.freeze_union_padding(builder, repr, value);
			let value = self.helper_argument(builder, value)?;
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
		let value = self.freeze_union_padding(builder, repr, value);
		let value = self.helper_argument(builder, value)?;
		if self.llvm_type(repr).is_struct_type() {
			let out = self.helper_slot(builder, self.llvm_type(repr))?;
			builder.build_call(function, &[out.into(), value.into()], "")?;
			let value = builder.build_load(self.llvm_type(repr), out, "")?;
			if self.wide_aggregate(repr) {
				crate::optimize::mark_aggregate_snapshot(value)?;
			}
			return Ok(value);
		}
		Ok(builder
			.build_call(function, &[value.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a copy"))
	}

	/// Branches to a new block when `condition` holds, else returns: the
	/// builder continues in the new block.
	pub(crate) fn helper_when(
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
		if self.compressed_union_link(repr).is_some() {
			let pointer = self.edge_address(builder, repr, value)?;
			self.call_drop(builder, Repr::Union, pointer.into())?;
			return Ok(());
		}
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
			// the runtime frees a block, never an inline value nor a constant
			Repr::BigInt => {
				builder.build_call(
					self.runtime.function("fa_bigint_drop")?,
					&[value.into()],
					"",
				)?;
			}
			Repr::Str => {
				let pair = value.into_struct_value();
				let capacity = builder.build_extract_value(pair, 2, "")?.into_int_value();
				let kind = builder.build_and(capacity, self.string_flags().const_not(), "")?;
				let nonzero = builder.build_int_compare(
					IntPredicate::NE,
					kind,
					self.usize.const_zero(),
					"",
				)?;
				let heap = builder.build_int_compare(
					IntPredicate::ULT,
					kind,
					self.string_kind_mask(),
					"",
				)?;
				let tag = builder.build_and(kind, self.usize.const_int(1, false), "")?;
				let even = builder.build_int_compare(
					IntPredicate::EQ,
					tag,
					self.usize.const_zero(),
					"",
				)?;
				let owned = builder.build_and(builder.build_and(nonzero, heap, "")?, even, "")?;
				self.helper_when(builder, function, owned, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				let bytes = builder.build_extract_value(pair, 0, "")?;
				builder.build_call(
					self.runtime.function("fa_string_free")?,
					&[bytes.into(), capacity.into()],
					"",
				)?;
			}
			Repr::Bytes => {
				builder.build_call(self.runtime.function("fa_bytes_free")?, &[value.into()], "")?;
			}
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, NodeId::new(0))?;
				let array = value.into_pointer_value();
				let present = builder.build_is_not_null(array, "")?;
				self.helper_when(builder, function, present, |builder| {
					builder.build_return(None)?;
					Ok(())
				})?;
				if self.packed_layout(item_type).is_some() {
					// its items own nothing (`packed.rs`)
					builder.build_call(
						self.runtime.function("fa_packed_free")?,
						&[array.into()],
						"",
					)?;
					return Ok(());
				}
				let storage = builder
					.build_call(
						self.runtime.function("fa_array_storage")?,
						&[array.into()],
						"",
					)?
					.try_as_basic_value()
					.basic()
					.expect("storage")
					.into_int_value();
				let owned = builder.build_int_compare(
					IntPredicate::NE,
					storage,
					storage.get_type().const_int(2, false),
					"",
				)?;
				self.helper_when(builder, function, owned, |builder| {
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
			Repr::InlineUnion(nominal) => {
				self.build_inline_drop(builder, function, nominal, value)?
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
			Repr::Tuple(ty) if self.inline_info(ty).is_some() => {
				unreachable!("inline arrays use place helpers");
			}
			Repr::Object(_) | Repr::Tuple(_) => {
				let fields = match repr {
					Repr::Object(ty) => self.object_fields(ty),
					Repr::Tuple(ty) => self.tuple_reprs(ty),
					_ => unreachable!(),
				};
				// a resource of foreign code: its `destroy` first (foreign.rs);
				// a resource that moved out is never dropped (its drop flag,
				// `own_local`), so every value here is owned, even all zero
				if let Repr::Object(ty) = repr
					&& let Some(destroy) = self.destroy_function(ty)
				{
					self.effectful_drops.set(true);
					let argument = if destroy.get_type().get_param_types()[0].is_pointer_type() {
						function.get_nth_param(0).expect("a resource place")
					} else {
						value
					};
					let call = builder.build_call(destroy, &[argument.into()], "")?;
					call.set_call_convention(super::FAST_CALLING_CONVENTION);
				}
				let structure = if let Repr::Object(ty) = repr
					&& self.destroy_function(ty).is_some()
				{
					// `destroy` may clear/replace inline fields. Drop the prefix
					// it leaves initialized, from the same borrowed owner.
					let source = function
						.get_nth_param(0)
						.expect("a resource place")
						.into_pointer_value();
					builder
						.build_load(self.llvm_type(repr), source, "")?
						.into_struct_value()
				} else {
					value.into_struct_value()
				};
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

	/// Copy fast paths use the same result-place convention as the main
	/// path, including strings and closure handles with an early return.
	fn return_copy_value(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		if function.get_type().get_return_type().is_none() {
			let out = function
				.get_nth_param(0)
				.expect("a destination")
				.into_pointer_value();
			builder.build_store(out, value)?;
			builder.build_return(None)?;
		} else {
			builder.build_return(Some(&value))?;
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
		if self.compressed_union_link(repr).is_some() {
			let pointer = self.edge_address(builder, repr, value)?;
			let copied = self
				.call_copy(builder, Repr::Union, pointer.into())?
				.into_pointer_value();
			return self.address_edge(builder, repr, copied);
		}
		Ok(match repr {
			Repr::BigInt => builder
				.build_call(
					self.runtime.function("fa_bigint_copy")?,
					&[value.into()],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("a copy"),
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
					self.return_copy_value(builder, function, pair.into())?;
					Ok(())
				})?;
				// Only immortal literal storage can be shared. An empty owner
				// may still own a reserved heap block; the slow copy makes it inline.
				let shifted =
					builder.build_left_shift(capacity, self.usize.const_int(3, false), "")?;
				let before = builder.build_int_sub(shifted, self.usize.const_int(1, false), "")?;
				let literal = builder.build_int_compare(
					IntPredicate::SLT,
					before,
					self.usize.const_zero(),
					"",
				)?;
				// Counted views use LITERAL together with SURROGATES; unlike
				// immortal well-formed literals, their bytes must be copied.
				let surrogates = builder.build_and(capacity, self.string_surrogates_flag(), "")?;
				let well_formed_kind = builder.build_int_compare(
					IntPredicate::EQ,
					surrogates,
					self.usize.const_zero(),
					"",
				)?;
				let literal = builder.build_and(literal, well_formed_kind, "")?;
				let not_shared = builder.build_not(literal, "")?;
				self.helper_when(builder, function, not_shared, |builder| {
					self.return_copy_value(builder, function, pair.into())?;
					Ok(())
				})?;
				let slow = self.copy_string_slow()?;
				builder
					.build_call(slow, &[value.into()], "")?
					.try_as_basic_value()
					.basic()
					.expect("a copied string")
			}
			Repr::Array(item_type) if self.packed_layout(item_type).is_some() => {
				// `packed::packable_items` admits no copy
				return Err(Error::Llvm("a copy of a packed array".into()));
			}
			Repr::Bytes => builder
				.build_call(self.runtime.function("fa_bytes_copy")?, &[value.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("Bytes copy"),
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, NodeId::new(0))?;
				let array = value.into_pointer_value();
				let present = builder.build_is_not_null(array, "")?;
				self.helper_when(builder, function, present, |builder| {
					self.return_copy_value(builder, function, array.into())?;
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
						self.return_copy_value(builder, function, copied.into())?;
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
					self.return_copy_value(builder, function, array.into())?;
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
			Repr::InlineUnion(nominal) => {
				self.build_inline_copy(builder, function, nominal, value)?
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
				// an edge, or a compressed link (`trees.rs`)
				let optional_edge = Repr::Optional(super::values::Scalar::Rec(target));
				let edge_type = self.llvm_type(repr);
				let first = self.edge_address(builder, optional_edge, value)?;
				let present = builder.build_is_not_null(first, "")?;
				self.helper_when(builder, function, present, |builder| {
					if self.arena_object(target) {
						builder.build_call(
							self.runtime.function("fa_arena_leave")?,
							&[first.into()],
							"",
						)?;
					}
					self.return_copy_value(builder, function, value)?;
					Ok(())
				})?;
				// where the next copy goes: a slot holding the first copy, then
				// the edge field of the last copy
				let root = builder.build_alloca(edge_type, "root")?;
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
				builder.build_store(
					destination_pointer,
					self.address_edge(builder, optional_edge, copy)?,
				)?;
				let loaded = builder.build_load(target_type, source_pointer, "")?;
				let (next, next_destination) = match self.last_edge(target_repr) {
					Some(index) => {
						let structure = loaded.into_struct_value();
						let edge = builder.build_extract_value(structure, index as u32, "")?;
						let Repr::Object(object) = target_repr else {
							unreachable!("an edge field of an object")
						};
						let field = self.object_fields(object)[index];
						let edge = self.edge_address(builder, field, edge)?;
						let rest = builder.build_insert_value(
							structure,
							super::zero_of(self.llvm_type(field)),
							index as u32,
							"",
						)?;
						let copied =
							self.call_copy(builder, target_repr, rest.into_struct_value().into())?;
						builder.build_store(copy, copied)?;
						let field =
							builder.build_struct_gep(target_type, copy, index as u32, "")?;
						(edge, field)
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
				builder.build_load(edge_type, root, "")?
			}
			Repr::Function(_) => {
				let pair = value.into_struct_value();
				let environment = builder
					.build_extract_value(pair, 1, "")?
					.into_pointer_value();
				let present = builder.build_is_not_null(environment, "")?;
				self.helper_when(builder, function, present, |builder| {
					self.return_copy_value(builder, function, value)?;
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
					self.return_copy_value(builder, function, value)?;
					Ok(())
				})?;
				let inner = inner.value.expect("an optional value");
				let copied = self.call_copy(builder, scalar.repr(), inner)?;
				self.pack_optional(builder, scalar, present, copied)?
			}
			Repr::Tuple(ty) if self.inline_info(ty).is_some() => {
				unreachable!("inline arrays use place helpers");
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
	/// and even a static ASCII string pays a call and several spills.
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
	pub(super) fn helper_array_parts(
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
			let id = self.union_id(&builder, pointer)?;
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
			let id = self.union_id(&builder, pointer)?;
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
				if let Some(plan) = self.compact_array_box(*id) {
					let data = self
						.compact_payload_value(&builder, pointer, plan)?
						.into_struct_value();
					let array = builder
						.build_extract_value(data, 0, "")?
						.into_pointer_value();
					let copied = self.freeze_array_payload(&builder, *id, plan, array, true)?;
					builder.build_call(
						self.runtime.function("fa_arena_leave")?,
						&[copied.into()],
						"",
					)?;
					builder.build_return(Some(&copied))?;
					blocks.push((i64.const_int(*id, false), block));
					continue;
				}
				if let Some(data) = self.compact_string_box(*id) {
					let data = self
						.string_payload_value(&builder, pointer, data)?
						.into_struct_value();
					let text = builder.build_extract_value(data, 0, "")?;
					let copied = self.freeze_string_payload(&builder, *id, text, true)?;
					builder.build_call(
						self.runtime.function("fa_arena_leave")?,
						&[copied.into()],
						"",
					)?;
					builder.build_return(Some(&copied))?;
					blocks.push((i64.const_int(*id, false), block));
					continue;
				}
				if let Some(data) = self.compact_scalar_box(*id) {
					let value = self.scalar_payload_value(&builder, pointer, data)?;
					let copied = self.freeze_scalar_payload(&builder, *id, value)?;
					builder.build_call(
						self.runtime.function("fa_arena_leave")?,
						&[copied.into()],
						"",
					)?;
					builder.build_return(Some(&copied))?;
					blocks.push((i64.const_int(*id, false), block));
					continue;
				}
				if let Some(plan) = self.compact_edge_box(*id) {
					let value = self.edge_payload_value(&builder, pointer, plan)?;
					let copied = self.freeze_edge_payload(&builder, *id, plan, value, true)?;
					builder.build_call(
						self.runtime.function("fa_arena_leave")?,
						&[copied.into()],
						"",
					)?;
					builder.build_return(Some(&copied))?;
					blocks.push((i64.const_int(*id, false), block));
					continue;
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
				let stored_id = if region && self.packed_headers() {
					*id | 0x8000
				} else {
					*id
				};
				builder.build_store(copied, i64.const_int(stored_id, false))?;
				for (index, field) in fields.iter().enumerate() {
					let from = builder.build_struct_gep(*layout, pointer, index as u32 + 1, "")?;
					let to = builder.build_struct_gep(*layout, copied, index as u32 + 1, "")?;
					let loaded = builder.build_load(self.llvm_type(*field), from, "")?;
					let deep = self.call_copy(&builder, *field, loaded)?;
					builder.build_store(to, deep)?;
				}
				if region {
					self.register_arena_payloads(&builder, *id, copied, *layout, fields)?;
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
	pub(crate) fn unregister(&self, value: Val<'ctx>) -> bool {
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
	fn escape_owned_storage(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		if value.repr == Repr::Bytes {
			let owned = self.call_value(
				self.runtime.function("fa_bytes_own")?,
				&[value.value.expect("Bytes").into()],
			)?;
			Ok(Val::new(Repr::Bytes, owned))
		} else {
			self.escape_frame_string(value)
		}
	}

	pub(crate) fn take(&self, value: Val<'ctx>, node: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		if value.value.is_none() || !self.needs_drop(value.repr) {
			return Ok(value);
		}
		if self.unregister(value) {
			return self.escape_owned_storage(value);
		}
		if let Some(node) = node
			&& self.is_coalesced_return_read(node, value.repr)
		{
			// The local already occupies the caller's result slot. Keep its
			// bits intact; normal-return cleanup excludes this owner, while
			// error cleanup still destroys it. No run-time drop flag is needed.
			return Ok(value);
		}
		if let Some(node) = node
			&& let Some(slot) = self.movable_local(node, value.repr)
		{
			let value = self.move_owned_slot(slot, value)?;
			return self.escape_owned_storage(value);
		}
		self.emit_copy(value)
	}

	/// Commit a proven whole-owner transfer after its input reads finish.
	/// This does not rescue borrowed backing by copying it.
	pub(super) fn move_owned_slot(
		&self,
		slot: Slot<'ctx>,
		value: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		self.builder
			.build_store(slot.pointer, self.llvm_type(slot.repr).const_zero())?;
		if let Some(&flag) = self.resource_flags.borrow().get(&slot.pointer) {
			self.builder
				.build_store(flag, self.context.bool_type().const_zero())?;
		}
		Ok(value)
	}

	/// A local read for the last time (`node`), given up so that the caller
	/// may change its value in place, without a copy; `None` otherwise.
	pub(crate) fn take_local(
		&self,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		if value.value.is_none() || !self.needs_drop(value.repr) {
			return Ok(None);
		}
		match self.movable_local(node, value.repr) {
			Some(slot)
				if slot.repr == value.repr
					&& !self.resource_flags.borrow().contains_key(&slot.pointer) =>
			{
				self.builder
					.build_store(slot.pointer, self.llvm_type(slot.repr).const_zero())?;
				Ok(Some(value))
			}
			_ => Ok(None),
		}
	}

	/// Takes a value, then registers it as a temporary of the enclosing
	/// statement: the value of a block, of an `if`.
	pub(crate) fn own(&self, value: Val<'ctx>, node: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		if node.is_some_and(|node| self.frame().default_inputs.contains_key(&node)) {
			return Ok(value);
		}
		let owned = self.take(value, node)?;
		Ok(self.fresh(owned))
	}

	/// Starts owning a local whose slot was just initialized.
	pub(crate) fn own_local(&mut self, symbol: SymbolId, slot: Slot<'ctx>) {
		if !self.needs_drop(slot.repr) {
			return;
		}
		// a value holding a resource of foreign code: a drop flag, cleared
		// when the local moves out (`let second = first`, a returned value),
		// and one per field that may move out of it: a resource may be all
		// zero, so a moved one is known by its flag, never by its bytes
		if self
			.types()
			.contains_resource(self.analysis.symbols.symbol(symbol).ty)
			&& let Ok(flag) = self.set_owner_flag(symbol, slot, "owns_resource")
		{
			self.cleanups
				.borrow_mut()
				.push(Cleanup::Flagged(slot, flag));
			self.resource_flags.borrow_mut().insert(slot.pointer, flag);
			let mut fields = Vec::new();
			for path in self.moved_field_paths(symbol, slot.repr) {
				if let Ok(flag) =
					self.set_owner_flag(symbol, slot, &format!("owns_field.{}", fields.len()))
				{
					fields.push(FieldFlag { path, flag });
				}
			}
			if !fields.is_empty() {
				self.field_flags.borrow_mut().insert(slot.pointer, fields);
			}
			self.owned_locals.insert(symbol, slot);
			return;
		}
		self.cleanups.borrow_mut().push(Cleanup::Local(slot));
		self.owned_locals.insert(symbol, slot);
	}

	/// A new drop flag, set here.
	fn set_flag(&self, name: &str) -> Result<PointerValue<'ctx>, Error> {
		let flag = self.alloca(Repr::Bool, name)?;
		self.builder
			.build_store(flag.pointer, self.context.bool_type().const_int(1, false))?;
		Ok(flag.pointer)
	}

	/// Module values outlive their initializer, including a C library's
	/// constructor. Their ownership flags must outlive that frame too.
	fn set_owner_flag(
		&self,
		symbol: SymbolId,
		slot: Slot<'ctx>,
		name: &str,
	) -> Result<PointerValue<'ctx>, Error> {
		if !self
			.global_slots
			.values()
			.any(|global| global.pointer == slot.pointer)
		{
			return self.set_flag(name);
		}
		let name = format!("fa.{name}.{}.{}", self.path, symbol.index());
		let global = self.module.get_global(&name).unwrap_or_else(|| {
			let global = self
				.module
				.add_global(self.context.bool_type(), None, &name);
			global.set_initializer(&self.context.bool_type().const_zero());
			global.set_linkage(Linkage::Internal);
			global
		});
		self.builder.build_store(
			global.as_pointer_value(),
			self.context.bool_type().const_int(1, false),
		)?;
		Ok(global.as_pointer_value())
	}

	/// The paths of the fields holding a resource that move out of a local
	/// somewhere (`Analysis::moved_reads`), through objects stored inline.
	fn moved_field_paths(&self, symbol: SymbolId, repr: Repr) -> Vec<Vec<u32>> {
		let mut nodes: Vec<NodeId> = self.analysis.moved_reads.iter().copied().collect();
		nodes.sort_unstable_by_key(|node| node.index());
		let mut paths: Vec<Vec<u32>> = Vec::new();
		for node in nodes {
			if !self.types().contains_resource(self.type_of(node)) {
				continue;
			}
			if let Some((root, path)) = self.field_path(node)
				&& root == symbol
				&& !path.is_empty()
				&& self.inline_path(repr, &path).is_some()
				&& !paths.contains(&path)
			{
				paths.push(path);
			}
		}
		paths
	}

	/// The local at the root of a place made of fields (`holder.a.b`) and
	/// the indices of those fields.
	pub(crate) fn field_path(&self, node: NodeId) -> Option<(SymbolId, Vec<u32>)> {
		match self.view(node) {
			super::View::Group(inner) => self.field_path(inner),
			super::View::Name(_) => match self.analysis.resolution(node) {
				Resolution::Local(symbol) => Some((symbol, Vec::new())),
				_ => None,
			},
			super::View::Access { target, members } => {
				let (symbol, mut path) = self.field_path(target)?;
				let mut repr = match self.slots.get(&symbol) {
					Some(slot) => slot.repr,
					None => self
						.repr_of(self.analysis.symbols.symbol(symbol).ty, node.index())
						.ok()?,
				};
				for &index in &path {
					let Repr::Object(owner) = repr else {
						return None;
					};
					repr = self.field_repr(owner, index as usize, node.index()).ok()?;
				}
				for member in members.iter() {
					let Repr::Object(owner) = repr else {
						return None;
					};
					let index = self.concrete_field_index(owner, member).ok()?;
					path.push(index);
					repr = self
						.field_repr(owner, index as usize, member.index())
						.ok()?;
				}
				Some((symbol, path))
			}
			_ => None,
		}
	}

	/// The representation at the end of a path of fields stored inline
	/// (objects of objects), if every step is one.
	fn inline_path(&self, mut repr: Repr, path: &[u32]) -> Option<Repr> {
		for &index in path {
			let Repr::Object(object) = repr else {
				return None;
			};
			if self.destroy_function(object).is_some() {
				return None;
			}
			repr = *self.object_fields(object).get(index as usize)?;
		}
		Some(repr)
	}

	/// A place whose move-only value moved out (`node`, a local or a field
	/// of one): a zero (a task's null, which its drop skips), and its drop
	/// flag cleared when it holds a resource, which a zero can't mark (a
	/// resource may be all zero).
	pub(crate) fn give_up_place(&mut self, node: NodeId, slot: Slot<'ctx>) -> Result<(), Error> {
		self.builder
			.build_store(slot.pointer, self.llvm_type(slot.repr).const_zero())?;
		if let Some(&flag) = self.resource_flags.borrow().get(&slot.pointer) {
			self.builder
				.build_store(flag, self.context.bool_type().const_zero())?;
			return Ok(());
		}
		if !self.types().contains_resource(self.type_of(node)) {
			return Ok(());
		}
		let flag = self.field_path(node).and_then(|(symbol, path)| {
			let local = self.slots.get(&symbol)?;
			let flags = self.field_flags.borrow();
			flags
				.get(&local.pointer)?
				.iter()
				.find(|field| field.path == path)
				.map(|field| field.flag)
		});
		match flag {
			Some(flag) => {
				self.builder
					.build_store(flag, self.context.bool_type().const_zero())?;
				Ok(())
			}
			None => Err(self.unsupported(node.index(), "a resource moved out of this place")),
		}
	}

	/// Drops the value of a place before it is replaced (`target = value`),
	/// but the parts that moved out of it (their drop flags): the flags to
	/// set once the new value is stored.
	pub(crate) fn drop_replaced(
		&mut self,
		target: NodeId,
		slot: Slot<'ctx>,
	) -> Result<Vec<PointerValue<'ctx>>, Error> {
		let mut flags: Vec<FieldFlag<'ctx>> = Vec::new();
		if let Some((symbol, path)) = self.field_path(target)
			&& let Some(local) = self.slots.get(&symbol).copied()
		{
			if path.is_empty()
				&& let Some(&flag) = self.resource_flags.borrow().get(&local.pointer)
			{
				flags.push(FieldFlag {
					path: Vec::new(),
					flag,
				});
			}
			if let Some(fields) = self.field_flags.borrow().get(&local.pointer) {
				for field in fields {
					if field.path.starts_with(&path) {
						flags.push(FieldFlag {
							path: field.path[path.len()..].to_vec(),
							flag: field.flag,
						});
					}
				}
			}
		}
		if flags.is_empty() {
			let old = self.load(slot)?;
			self.emit_drop(old)?;
			return Ok(Vec::new());
		}
		self.drop_place(slot, &flags)?;
		Ok(flags.into_iter().map(|field| field.flag).collect())
	}

	/// Drops the value of a place but the parts whose flag is clear
	/// (`flags`, by path from the place; an empty path is the whole).
	fn drop_place(&self, slot: Slot<'ctx>, flags: &[FieldFlag<'ctx>]) -> Result<(), Error> {
		if flags.is_empty() {
			if self.needs_drop(slot.repr) {
				let value = self.load_for_drop(slot)?;
				self.emit_drop(value)?;
			}
			return Ok(());
		}
		if let Some(whole) = flags.iter().find(|field| field.path.is_empty()) {
			let rest: Vec<FieldFlag<'ctx>> = flags
				.iter()
				.filter(|field| !field.path.is_empty())
				.cloned()
				.collect();
			let set = self
				.builder
				.build_load(self.context.bool_type(), whole.flag, "")?
				.into_int_value();
			let function = self.builder.get_insert_block().expect("a block");
			let function = function.get_parent().expect("a function");
			let then = self.context.append_basic_block(function, "drop_owned");
			let end = self.context.append_basic_block(function, "dropped_owned");
			self.builder.build_conditional_branch(set, then, end)?;
			self.builder.position_at_end(then);
			self.drop_place(slot, &rest)?;
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(end);
			return Ok(());
		}
		// the fields one by one, in the order of the drop function
		let Repr::Object(object) = slot.repr else {
			return Err(self.unsupported(0, "a resource moved out of this place"));
		};
		let structure = self.object_type(object);
		let fields = self.object_fields(object);
		for (index, repr) in fields.into_iter().enumerate().rev() {
			let below: Vec<FieldFlag<'ctx>> = flags
				.iter()
				.filter(|field| field.path[0] == index as u32)
				.map(|field| FieldFlag {
					path: field.path[1..].to_vec(),
					flag: field.flag,
				})
				.collect();
			if below.is_empty() && !self.needs_drop(repr) {
				continue;
			}
			let pointer =
				self.builder
					.build_struct_gep(structure, slot.pointer, index as u32, "")?;
			self.drop_place(Slot { pointer, repr }, &below)?;
		}
		Ok(())
	}

	/// Drops the fields of a temporary object but one, which moves out of
	/// it (a task, a resource): the object is no longer a temporary.
	pub(crate) fn drop_all_but(&self, value: Val<'ctx>, kept: usize) -> Result<(), Error> {
		self.unregister(value);
		let Repr::Object(object) = value.repr else {
			return Ok(());
		};
		let structure = value.value.expect("an object").into_struct_value();
		for (index, repr) in self.object_fields(object).into_iter().enumerate().rev() {
			if index != kept && self.needs_drop(repr) {
				let raw = self
					.builder
					.build_extract_value(structure, index as u32, "")?;
				self.emit_drop(Val::new(repr, raw))?;
			}
		}
		Ok(())
	}

	/// Whether a parameter is moved in by its caller, so the function owns
	/// it: a parameter (not `self`, which a method borrows) of a move-only
	/// type (`TypeStore::owns_unique`).
	pub(crate) fn is_moved_parameter(&self, symbol: SymbolId) -> bool {
		let data = self.analysis.symbols.symbol(symbol);
		matches!(data.kind, SymbolKind::Parameter { .. })
			&& self
				.types()
				.parameter_can_be_consumed(self.concrete_type(data.ty))
	}

	/// Stops dropping a slot registered with `Cleanup::Local` (a value built
	/// by a loop whose callbacks may throw, owned by its result now).
	pub(crate) fn forget_local(&self, slot: Slot<'ctx>) {
		let mut cleanups = self.cleanups.borrow_mut();
		if let Some(position) = cleanups.iter().rposition(
			|cleanup| matches!(cleanup, Cleanup::Local(local) if local.pointer == slot.pointer),
		) {
			cleanups.remove(position);
		}
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
				if matches!(slot.repr, Repr::Row(_)) {
					self.emit_drop(self.load_for_drop(slot)?)
				} else {
					self.drop_at(&self.builder, slot.repr, slot.pointer)
				}
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
				// the whole flag, then the flags of its fields
				let mut flags = vec![FieldFlag {
					path: Vec::new(),
					flag,
				}];
				if let Some(fields) = self.field_flags.borrow().get(&slot.pointer) {
					flags.extend(fields.iter().cloned());
				}
				self.drop_place(slot, &flags)
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
		let destination = keep.and_then(|value| value.destination);
		let entries = {
			let mut cleanups = self.cleanups.borrow_mut();
			let mark = mark.min(cleanups.len());
			cleanups.split_off(mark)
		};
		let keep = keep.and_then(|value| value.value);
		let mut kept = None;
		for entry in entries.into_iter().rev() {
			if self.preserves_return_owner(entry, destination) {
				continue;
			}
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
		self.drop_to_returning(mark, None)
	}

	pub(super) fn drop_to_returning(
		&self,
		mark: usize,
		keep: Option<Val<'ctx>>,
	) -> Result<(), Error> {
		let entries: Vec<Cleanup<'ctx>> = {
			let cleanups = self.cleanups.borrow();
			cleanups[mark.min(cleanups.len())..].to_vec()
		};
		for entry in entries.into_iter().rev() {
			if self.preserves_return_owner(entry, keep.and_then(|value| value.destination)) {
				continue;
			}
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
		if let Some(node) = node
			&& self.frame().default_inputs.contains_key(&node)
		{
			return self.convert(value, repr, node.index());
		}
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
	pub(crate) fn movable_local(&self, node: NodeId, repr: Repr) -> Option<Slot<'ctx>> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		if self.analysis.has_flags(inner, NodeFlags::BORROWED_OWNER) {
			return None;
		}
		if !matches!(self.view(inner), View::Name(_)) {
			return None;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(inner) else {
			return None;
		};
		let slot = *self.owned_locals.get(&symbol)?;
		// Callee-frame locals and unconditional read-only globals cannot
		// change owner. Caller-provided return storage is deliberately absent
		// from this set: its pointer may move back to that caller.
		if self
			.frame()
			.collection_locals
			.contains(&(self.file, symbol))
		{
			return None;
		}
		if self.shared.contains_key(&symbol)
			&& !(self.analysis.has_flags(inner, NodeFlags::MOVE) && self.closed_module_transfers())
		{
			return None;
		}
		// the whole value, or the value of an optional
		let whole = slot.repr == repr
			|| self.same_layout(slot.repr, repr)
			|| matches!(slot.repr, Repr::Optional(scalar) if scalar.repr() == repr);
		if !whole {
			return None;
		}
		let data = self.analysis.symbols.symbol(symbol);
		// A catch binding owns the error. A consuming parameter is moved
		// into this frame by its caller and owns its value as well.
		if !matches!(
			data.kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::CatchBinding
		) && !self.is_moved_parameter(symbol)
		{
			return None;
		}
		// views of a text's bytes must not leave their local (`views.rs`)
		if self.holds_views(symbol) {
			return None;
		}
		if self.analysis.flags_of(inner).contains(NodeFlags::MOVE) {
			return Some(slot);
		}
		if self.is_returned_local(symbol, inner, repr)
			|| self.is_last_use(symbol, inner, data.decl_node)
		{
			Some(slot)
		} else {
			None
		}
	}

	/// A closed synchronous initializer accounts for its future exports in
	/// the move graph. Other deferred observers still need their lifetime
	/// proof before physical module slots can transfer.
	fn closed_module_transfers(&self) -> bool {
		use fa_analyzer::analysis::{Callee, FunctionCaptureState};
		if self.analysis.escapes.iter().any(|function| {
			self.analysis.function_capture_states.get(function)
				!= Some(&FunctionCaptureState::Empty)
		}) || !self.analysis.starts.is_empty()
			|| !self.analysis.watches.is_empty()
			|| !self.analysis.type_decls.is_empty()
			|| !self.analysis.function_owners.is_empty()
			|| !self.analysis.foreign_args.is_empty()
			|| self.analysis.ownership_source_graph.entries.is_empty()
			|| self
				.analysis
				.ownership_source_graph
				.entries
				.values()
				.any(|entry| entry.pending)
		{
			return false;
		}
		if self.analysis.symbols.symbols().iter().any(|symbol| {
			matches!(
				symbol.kind,
				SymbolKind::Use
					| SymbolKind::MutableUse
					| SymbolKind::Reactive
					| SymbolKind::Derived
					| SymbolKind::Namespace
			)
		}) {
			return false;
		}
		self.analysis.functions.values().all(|info| {
			info.sig.is_some_and(|sig| !self.types().sig(sig).is_async)
				&& info.declaration.is_some_and(|node| {
					matches!(
						self.view(node),
						View::Function { .. } | View::ExportFunction { .. }
					)
				})
		}) && self.analysis.calls.values().all(|call| match call.callee {
			Callee::Builtin(_) | Callee::Conversion(_) => true,
			Callee::Function(node) => self.analysis.function(node).is_some(),
			_ => false,
		}) && (0..self.tree.nodes.len()).all(|index| {
			!matches!(
				self.view(NodeId::new(index)),
				View::ExportNamespace(_) | View::Await(_)
			)
		})
	}

	/// Immutable global loads cannot promise invariance when their owner
	/// slot (or a supported unique field) is cleared by a later transfer.
	pub(super) fn binding_may_move(&self, symbol: SymbolId) -> bool {
		self.analysis
			.flags
			.iter()
			.enumerate()
			.any(|(index, flags)| {
				let node = NodeId::new(index);
				(flags.contains(NodeFlags::MOVE) || self.analysis.moved_reads.contains(&node))
					&& fa_analyzer::lower::copies::place_root(self.tree, self.analysis, node)
						== Some(symbol)
			})
	}

	/// A direct return ends this local's lifetime even when another branch
	/// reads it later in the source. Keep the proof closed to passive values
	/// with no references, captures, foreign exposure or observer cleanups.
	pub(super) fn is_returned_local(&self, symbol: SymbolId, node: NodeId, repr: Repr) -> bool {
		let Some(owner) = self.frame().owner else {
			return false;
		};
		let Some(function) = self.functions.get(&owner) else {
			return false;
		};
		if function.lambda
			|| self.tasks.asynchronous
			|| !self.analysis.foreign_args.is_empty()
			|| self
				.functions
				.values()
				.any(|f| f.captures.contains(&symbol))
		{
			return false;
		}
		let mut scope = self.analysis.symbols.symbol(symbol).scope;
		loop {
			if let Some(declared_owner) = self.scope_owners.get(&scope) {
				if *declared_owner != Some(owner) {
					return false;
				}
				break;
			}
			let Some(parent) = self.analysis.symbols.scope(scope).parent else {
				return false;
			};
			scope = parent;
		}
		let parents = self.parents();
		let mut current = node.index();
		while let Some(&parent) = parents.get(current) {
			if parent == usize::MAX {
				return false;
			}
			match self.view(NodeId::new(parent)) {
				View::Group(_) => current = parent,
				View::Return(Some(value)) if value.index() == current => break,
				_ => return false,
			}
		}
		let mut stack = vec![function.info.body.index()];
		while let Some(at) = stack.pop() {
			if matches!(
				self.view(NodeId::new(at)),
				View::Use { .. } | View::Reuse { .. } | View::Provide { .. } | View::Watch(_)
			) {
				return false;
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		let mut seen = rustc_hash::FxHashSet::default();
		let mut stack = vec![repr];
		while let Some(at) = stack.pop() {
			if !seen.insert(at) {
				continue;
			}
			match at {
				Repr::Str | Repr::BigInt => {}
				Repr::Object(ty) if self.destroy_function(ty).is_none() => {
					stack.extend(self.object_fields(ty));
				}
				Repr::Array(ty) | Repr::Rec(ty) => match self.repr_of(ty, node.index()) {
					Ok(repr) => stack.push(repr),
					Err(_) => return false,
				},
				Repr::Tuple(ty) if self.inline_info(ty).is_some() => {
					let (item, capacity) = self.inline_info(ty).expect("an inline array");
					if capacity != 0 {
						match self.item_repr(item, node) {
							Ok(item) => stack.push(item),
							Err(_) => return false,
						}
					}
				}
				Repr::Tuple(ty) => stack.extend(self.tuple_reprs(ty)),
				Repr::Optional(scalar) => stack.push(scalar.repr()),
				_ if !self.needs_drop(at) => {}
				_ => return false,
			}
		}
		true
	}

	pub(super) fn is_last_use(&self, symbol: SymbolId, node: NodeId, declaration: NodeId) -> bool {
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
	pub(crate) fn referenced_locals(&self) -> std::cell::Ref<'_, rustc_hash::FxHashSet<SymbolId>> {
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
	pub(crate) fn uses_of(&self, symbol: SymbolId) -> Vec<NodeId> {
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
