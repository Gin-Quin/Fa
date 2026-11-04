//! Small nonrecursive named unions own their payload inline. A temporary
//! pointer view has the existing box layout, so matching and formatting can
//! reuse their generic dispatch without allocating a box.

use fa_analyzer::ids::{NominalId, TypeId};
use fa_analyzer::types::{NominalKind, TypeKind};
use inkwell::builder::Builder;
use inkwell::targets::TargetData;
use inkwell::types::StructType;
use inkwell::values::{AsValueRef, BasicValueEnum, FunctionValue, PointerValue};

use super::Lowering;
use super::values::{Repr, Val};
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	fn contains_inline_union(&self, repr: Repr) -> bool {
		match repr {
			Repr::InlineUnion(_) => true,
			Repr::Optional(scalar) => self.contains_inline_union(scalar.repr()),
			Repr::Object(ty) | Repr::Row(ty) => self
				.object_fields(ty)
				.into_iter()
				.any(|r| self.contains_inline_union(r)),
			Repr::Tuple(ty) => self
				.tuple_reprs(ty)
				.into_iter()
				.any(|r| self.contains_inline_union(r)),
			_ => false,
		}
	}

	/// Mutable payload views can overwrite an object, making its padding
	/// undefined again. Freeze the enclosing machine-word representation
	/// before passing it by value; only the unused bits may be chosen freely.
	pub(super) fn freeze_union_padding(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> BasicValueEnum<'ctx> {
		if !self.contains_inline_union(repr) {
			return value;
		}
		// Inkwell does not yet expose LLVMBuildFreeze. Both references belong
		// to this context, the builder is positioned, and freeze has the same
		// basic type as its operand.
		unsafe {
			BasicValueEnum::new(llvm_sys::core::LLVMBuildFreeze(
				builder.as_mut_ptr(),
				value.as_value_ref(),
				c"".as_ptr(),
			))
		}
	}

	/// Initially restrict inline payloads to finite scalar/object shapes. The
	/// boxed path remains available for recursive, generic and larger shapes.
	fn inline_payload(&self, ty: TypeId, depth: usize) -> bool {
		if depth > 32 {
			return false;
		}
		match self.types().kind(ty) {
			TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::Percentage
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..) => true,
			TypeKind::Int(int) => self.types().int(int).storage.bits() <= 64,
			TypeKind::Object(_) => {
				self.destroy_function(ty).is_none()
					&& self
						.types()
						.fields_of(ty)
						.iter()
						.all(|f| self.inline_payload(f.ty, depth + 1))
			}
			_ => false,
		}
	}

	pub(super) fn inline_union_words(&self, nominal: NominalId) -> Option<u32> {
		let types = self.types();
		let info = types.nominal(nominal);
		if info.kind != NominalKind::Union
			|| !types.list_items(info.args).is_empty()
			|| !self.other_files.is_empty()
		{
			return None;
		}
		let count = types.sym_list(info.members).len() as u16;
		for index in 0..count {
			if let Some(ty) = self.member_data(nominal, index)
				&& !self.inline_payload(ty, 0)
			{
				return None;
			}
		}
		let layout = self.module.get_data_layout();
		let target = TargetData::create(layout.as_str().to_str().expect("a data layout"));
		let mut bytes = 0;
		for index in 0..count {
			if let Some(ty) = self.member_data(nominal, index) {
				let repr = self.repr_of(ty, 0).ok()?;
				bytes = bytes.max(target.get_abi_size(&self.llvm_type(repr)));
			}
		}
		// Keep a payload within the size of a string descriptor. Larger
		// maxima inflate every array element, even when most variants are
		// tiny; those need a compact collection layout before being inlined.
		let budget = 3 * u64::from(self.usize.get_bit_width() / 8);
		(bytes <= budget).then_some(bytes.div_ceil(8) as u32)
	}

	pub(super) fn inline_union_needs_drop(&self, nominal: NominalId) -> bool {
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len() as u16;
		(0..count)
			.filter_map(|index| self.member_data(nominal, index))
			.any(|ty| self.repr_of(ty, 0).is_ok_and(|repr| self.needs_drop(repr)))
	}

	pub(super) fn inline_union_type(&self, nominal: NominalId) -> StructType<'ctx> {
		let words = self.inline_union_words(nominal).expect("an inline union");
		let i64 = self.context.i64_type();
		self.context
			.struct_type(&[i64.into(), i64.array_type(words).into()], false)
	}

	/// A stack snapshot, also in generated helper functions. Allocate in the
	/// entry block so a view created inside a loop cannot grow the stack.
	pub(super) fn inline_union_slot(
		&self,
		builder: &Builder<'ctx>,
		value: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let function = builder
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let entry = function.get_first_basic_block().expect("an entry");
		let allocas = self.context.create_builder();
		match entry.get_first_instruction() {
			Some(first) => allocas.position_before(&first),
			None => allocas.position_at_end(entry),
		}
		let pointer = allocas.build_alloca(value.into_struct_value().get_type(), "union_view")?;
		builder.build_store(pointer, value)?;
		Ok(pointer)
	}

	pub(super) fn union_view(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		if matches!(value.repr, Repr::InlineUnion(_)) {
			let pointer = self.inline_union_slot(&self.builder, value.value.expect("a union"))?;
			Ok(Val::new(Repr::Union, pointer))
		} else {
			Ok(value)
		}
	}

	/// Preserve the initialized padding of the enclosing union. An aggregate
	/// object store may leave its padding undefined; those bytes become real
	/// integer bits when the union is subsequently loaded in machine words.
	pub(super) fn store_inline_payload(
		&self,
		builder: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		match repr {
			Repr::Object(ty) => {
				for (index, field) in self.object_fields(ty).into_iter().enumerate() {
					let raw =
						builder.build_extract_value(value.into_struct_value(), index as u32, "")?;
					let address = builder.build_struct_gep(
						self.object_type(ty),
						pointer,
						index as u32,
						"",
					)?;
					self.store_inline_payload(builder, address, field, raw)?;
				}
			}
			Repr::Bool => {
				let byte = builder.build_int_z_extend(
					value.into_int_value(),
					self.context.i8_type(),
					"",
				)?;
				builder.build_store(pointer, byte)?;
			}
			_ => {
				builder.build_store(pointer, value)?;
			}
		}
		Ok(())
	}

	/// A boxed member may occupy fewer bytes than the maximum payload. Read
	/// only that member's layout. The result borrows the box's data, just as
	/// an ordinary unboxed object does; storing it into an owner copies it.
	pub(super) fn boxed_to_inline(
		&self,
		value: Val<'ctx>,
		nominal: NominalId,
	) -> Result<Val<'ctx>, Error> {
		let source = value.value.expect("a box").into_pointer_value();
		let repr = Repr::InlineUnion(nominal);
		let out = self.alloca(repr, "inline_union")?;
		self.builder
			.build_store(out.pointer, self.inline_union_type(nominal).const_zero())?;
		let i64 = self.context.i64_type();
		let tag = self
			.builder
			.build_load(i64, source, "tag")?
			.into_int_value();
		let tag_ptr =
			self.builder
				.build_struct_gep(self.inline_union_type(nominal), out.pointer, 0, "")?;
		self.builder.build_store(tag_ptr, tag)?;
		let dispatch = self.builder.get_insert_block().expect("a block");
		let end = self.append_block("union_loaded");
		let mut cases = Vec::new();
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len() as u16;
		for index in 0..count {
			let id = self.member_id(nominal, index);
			let Some(ty) = self.member_data(nominal, index) else {
				continue;
			};
			let data_repr = self.repr_of(ty, 0)?;
			let layout = self.box_type(Some(data_repr));
			let block = self.append_block("load_union_payload");
			self.builder.position_at_end(block);
			let data = self.builder.build_struct_gep(layout, source, 1, "")?;
			let loaded = self
				.builder
				.build_load(self.llvm_type(data_repr), data, "")?;
			let target = self.builder.build_struct_gep(layout, out.pointer, 1, "")?;
			self.store_inline_payload(&self.builder, target, data_repr, loaded)?;
			self.builder.build_unconditional_branch(end)?;
			cases.push((i64.const_int(id, false), block));
		}
		self.builder.position_at_end(dispatch);
		self.builder.build_switch(tag, end, &cases)?;
		self.builder.position_at_end(end);
		self.load(out)
	}

	pub(super) fn build_inline_drop(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		nominal: NominalId,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		self.inline_payload_ownership(builder, function, nominal, value, false)?;
		Ok(())
	}

	pub(super) fn build_inline_copy(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		nominal: NominalId,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let pointer = self.inline_payload_ownership(builder, function, nominal, value, true)?;
		let loaded = builder.build_load(self.inline_union_type(nominal), pointer, "")?;
		Ok(self.freeze_union_padding(builder, Repr::InlineUnion(nominal), loaded))
	}

	fn inline_payload_ownership(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		nominal: NominalId,
		value: BasicValueEnum<'ctx>,
		copy: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let pointer = self.inline_union_slot(builder, value)?;
		let id = builder
			.build_extract_value(value.into_struct_value(), 0, "tag")?
			.into_int_value();
		let dispatch = builder.get_insert_block().expect("a block");
		let end = self.context.append_basic_block(function, "union_done");
		let mut cases = Vec::new();
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len() as u16;
		for index in 0..count {
			let id = self.member_id(nominal, index);
			let Some(ty) = self.member_data(nominal, index) else {
				continue;
			};
			let repr = self.repr_of(ty, 0)?;
			if !self.needs_drop(repr) {
				continue;
			}
			let block = self.context.append_basic_block(function, "union_payload");
			builder.position_at_end(block);
			let data = builder.build_struct_gep(self.box_type(Some(repr)), pointer, 1, "")?;
			let loaded = builder.build_load(self.llvm_type(repr), data, "")?;
			if copy {
				let copied = self.call_copy(builder, repr, loaded)?;
				self.store_inline_payload(builder, data, repr, copied)?;
			} else {
				self.call_drop(builder, repr, loaded)?;
			}
			builder.build_unconditional_branch(end)?;
			cases.push((self.context.i64_type().const_int(id, false), block));
		}
		builder.position_at_end(dispatch);
		builder.build_switch(id, end, &cases)?;
		builder.position_at_end(end);
		Ok(pointer)
	}
}
