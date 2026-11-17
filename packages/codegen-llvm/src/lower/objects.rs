//! Objects, named unions and errors.
//!
//! - An object is a struct, by value, of its fields in the order of its shape
//!   (sorted by name); it prints its fields in declaration order.
//! - A value of a named union or `Errors` type is a pointer to a box: the
//!   global id of its member (`i64`), then its data. A member without data
//!   is a constant box. The built-in errors have the fixed ids of the
//!   runtime (`fa_runtime.c`, "errors"); the other members get theirs on
//!   first use, and `fa_format_union`, generated last, prints any box.
//! - Errors are returned (design/llvm-backend-plan.md): a function that
//!   throws returns `{ result, ptr error }` (or the error alone), null when
//!   it succeeded. A raised error goes to the innermost `catch` of the
//!   function, else to the caller, else (top level) ends the program.

use fa_analyzer::analysis::{FieldValue, NodeFlags};
use fa_analyzer::builtins::errors::{ERRORS, PRELUDE_MODULE};
use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::TypeKind;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

/// The first id of the members that aren't built-in errors.
const FIRST_MEMBER_ID: u64 = 16;

/// The first id of the member types of anonymous unions.
const FIRST_TYPE_ID: u64 = 1 << 32;

/// Ordinary ids never resemble a marked region header, including ids
/// allocated after lowering has interned additional concrete types.
fn unmarked_id(counter: u64) -> u64 {
	let high = (counter & !0x7fff)
		.checked_mul(2)
		.expect("too many union ids");
	high | (counter & 0x7fff)
}

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn pointer_type(&self) -> inkwell::types::PointerType<'ctx> {
		self.context.ptr_type(AddressSpace::default())
	}

	/// The id of a built-in error type (1 for `IndexError`…), from its nominal.
	fn builtin_error_id(&self, nominal: NominalId) -> Option<u64> {
		let types = self.types();
		let key = types.nominal(nominal).key;
		if types.module_path(types.decl_key_data(key).module) != PRELUDE_MODULE {
			return None;
		}
		let name = types.decl_name(key);
		ERRORS
			.iter()
			.position(|error| error.name == name)
			.map(|index| index as u64 + 1)
	}

	/// Runtime errors and source-built errors share the ordinary object layout.
	/// Declaration order is retained separately when formatting their fields.
	fn builtin_error_fields(&self, nominal: NominalId) -> Option<Vec<(&'static str, TypeId)>> {
		let id = self.builtin_error_id(nominal)?;
		let mut fields = ERRORS[id as usize - 1].fields.to_vec();
		fields.sort_unstable_by_key(|(name, _)| *name);
		Some(fields)
	}

	/// The global id of a member of a named union (registered for printing).
	pub(crate) fn member_id(&self, nominal: NominalId, index: u16) -> u64 {
		let mut ids = self.union_ids.borrow_mut();
		if let Some(id) = self.builtin_error_id(nominal) {
			ids.insert((nominal, index), id);
			return id;
		}
		let next = match self.is_split() {
			// the same in every unit (`units.rs`)
			true => self.stable_member_id(nominal, index),
			false => unmarked_id(
				FIRST_MEMBER_ID
					.checked_add(ids.len() as u64)
					.expect("too many named union ids"),
			),
		};
		*ids.entry((nominal, index)).or_insert(next)
	}

	/// The global id of a member type of an anonymous union (`Integer` in
	/// `Integer | String`), registered for printing.
	pub(crate) fn type_id(&self, ty: TypeId) -> u64 {
		let mut ids = self.type_ids.borrow_mut();
		let next = match self.is_split() {
			true => self.stable_type_id(ty),
			false => FIRST_TYPE_ID
				.checked_add(unmarked_id(ids.len() as u64))
				.expect("too many anonymous union ids"),
		};
		*ids.entry(ty).or_insert(next)
	}

	/// The type a value of this representation is boxed as when the union
	/// item isn't known (a store into a union without a recorded coercion).
	fn canonical_type(&self, repr: Repr) -> Option<TypeId> {
		Some(match repr {
			Repr::Int(int) => {
				let storage = fa_analyzer::types::IntStorage::ALL
					.into_iter()
					.find(|storage| {
						storage.bits() == int.bits && storage.is_signed() == int.signed
					})?;
				TypeId::of_storage(storage)
			}
			Repr::Bool => TypeId::BOOLEAN,
			Repr::F64 => TypeId::FLOAT,
			Repr::F32 => TypeId::FLOAT32,
			Repr::Percentage => TypeId::PERCENTAGE,
			Repr::Str => TypeId::STRING,
			Repr::Bytes => TypeId::BYTES,
			Repr::Object(ty) => ty,
			_ => return None,
		})
	}

	/// Boxes a value into a union, by the type of its representation.
	pub(crate) fn box_value(&self, value: Val<'ctx>, node: usize) -> Result<Val<'ctx>, Error> {
		if let Repr::InlineUnion(nominal) = value.repr {
			let copied =
				self.copy_inline_box(&self.builder, nominal, value.value.expect("a union"))?;
			return Ok(self.fresh(Val::new(Repr::Union, copied)));
		}
		let Some(ty) = self.canonical_type(value.repr) else {
			return Err(self.unsupported(node, "storing this value in a union"));
		};
		let id = self.context.i64_type().const_int(self.type_id(ty), false);
		let box_type = self.box_type(Some(value.repr));
		let pointer = self.allocate_for(box_type.into())?;
		let id_pointer = self.builder.build_struct_gep(box_type, pointer, 0, "")?;
		self.builder.build_store(id_pointer, id)?;
		let data_pointer = self.builder.build_struct_gep(box_type, pointer, 1, "")?;
		// the box owns its data: a temporary moves into it
		let owned = self.is_temporary(value);
		let data = if owned { value } else { self.emit_copy(value)? };
		self.builder
			.build_store(data_pointer, data.value.expect("data"))?;
		let boxed = Val::new(Repr::Union, pointer);
		if owned {
			self.transfer(value, boxed);
			Ok(boxed)
		} else {
			Ok(self.fresh(boxed))
		}
	}

	/// Materialize an independent box, including static storage for bare
	/// members. Usable from an ownership helper without a caller frame.
	pub(super) fn copy_inline_box(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		nominal: NominalId,
		value: inkwell::values::BasicValueEnum<'ctx>,
	) -> Result<inkwell::values::BasicValueEnum<'ctx>, Error> {
		let function = builder
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		// copy_box returns its input for a bare member, because ordinary
		// bare boxes are static. A stack view must never take that path:
		// use a real static box, and copy only members carrying data.
		let mut pointer = self.pointer_type().const_null();
		let mut any_bare = self.context.bool_type().const_zero();
		let mut has_bare = false;
		let tag = builder
			.build_extract_value(value.into_struct_value(), 0, "")?
			.into_int_value();
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len() as u16;
		for index in 0..count {
			if self.member_data(nominal, index).is_some() {
				continue;
			}
			has_bare = true;
			let id = self
				.context
				.i64_type()
				.const_int(self.member_id(nominal, index), false);
			let ty = self.box_type(None);
			let global = self.module.add_global(ty, None, "fa.member");
			global.set_initializer(&ty.const_named_struct(&[id.into()]));
			global.set_constant(true);
			global.set_linkage(inkwell::module::Linkage::Private);
			let bare = builder.build_int_compare(IntPredicate::EQ, tag, id, "")?;
			any_bare = builder.build_or(any_bare, bare, "")?;
			pointer = builder
				.build_select(bare, global.as_pointer_value(), pointer, "")?
				.into_pointer_value();
		}
		let bare_end = builder.get_insert_block().expect("a block");
		let end = if has_bare {
			let copy = self.context.append_basic_block(function, "box_payload");
			let end = self.context.append_basic_block(function, "boxed_union");
			builder.build_conditional_branch(any_bare, end, copy)?;
			builder.position_at_end(copy);
			Some(end)
		} else {
			None
		};
		let view = self.inline_union_slot(builder, value)?;
		let copied = builder
			.build_call(self.box_copy_function(), &[view.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a copied box");
		let result = if let Some(end) = end {
			let copied_end = builder.get_insert_block().expect("a block");
			builder.build_unconditional_branch(end)?;
			builder.position_at_end(end);
			let result = builder.build_phi(self.pointer_type(), "box")?;
			result.add_incoming(&[
				(&pointer, bare_end),
				(&copied.into_pointer_value(), copied_end),
			]);
			result.as_basic_value()
		} else {
			copied
		};
		Ok(result)
	}

	/// The items of an anonymous union type (without `None`).
	pub(crate) fn union_items(&self, ty: TypeId) -> Option<Vec<TypeId>> {
		match self.types().kind(ty) {
			TypeKind::Union(list) => Some(
				self.types()
					.list_items(list)
					.iter()
					.copied()
					.filter(|&item| item != TypeId::NONE)
					.collect(),
			),
			_ => None,
		}
	}

	/// A value stored as a member of an anonymous union type: boxed with the
	/// id of the item its representation belongs to.
	pub(crate) fn inject(
		&mut self,
		value: Val<'ctx>,
		target: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if self.repr_of(target, node.index())? == value.repr {
			return Ok(value);
		}
		if let TypeKind::NamedUnion(nominal, _) = self.types().kind(target)
			&& value.repr != Repr::Union
		{
			// a literal member (`Hello = "hello"`): the literal selects it
			let literal = self.type_of(node);
			if let TypeKind::Member(member_nominal, index) = self.types().kind(literal)
				&& member_nominal == nominal
			{
				return self.build_member(nominal, index, None, node);
			}
			let text = match self.view(node) {
				View::String(text) => Some(text.to_string()),
				_ => None,
			};
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len() as u16;
			let found = (0..count).find(|&index| {
				let payload = self.types().payload(nominal, index);
				payload == literal
					|| match (self.types().kind(payload), &text) {
						(TypeKind::StringLit(sym), Some(text)) => {
							self.types().syms.resolve(sym) == text
						}
						_ => false,
					}
			});
			if let Some(index) = found {
				return self.build_member(nominal, index, None, node);
			}
			return Err(self.unsupported(node.index(), "storing this value in a named union"));
		}
		let Some(items) = self.union_items(target) else {
			return Ok(value);
		};
		if matches!(value.repr, Repr::Union | Repr::None) {
			return Ok(value);
		}
		if matches!(value.repr, Repr::InlineUnion(_)) {
			return self.box_value(value, node.index());
		}
		let mut chosen = None;
		for item in items {
			let repr = self.repr_of(item, node.index())?;
			let matches = match (repr, value.repr) {
				(Repr::Int(_), Repr::Int(_)) => true,
				(a, b) => a == b || self.implicit_collection_view(b, a),
			};
			if matches {
				chosen = Some((item, repr));
				break;
			}
		}
		let Some((item, repr)) = chosen else {
			return Err(self.unsupported(node.index(), "storing this value in a union"));
		};
		let value = self.owned_as(value, repr, Some(node))?;
		let id = self.context.i64_type().const_int(self.type_id(item), false);
		let ty = self.box_type(Some(repr));
		let pointer = self.allocate_for(ty.into())?;
		let id_pointer = self.builder.build_struct_gep(ty, pointer, 0, "")?;
		self.builder.build_store(id_pointer, id)?;
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		self.builder
			.build_store(data_pointer, value.value.expect("data"))?;
		Ok(self.fresh(Val::new(Repr::Union, pointer)))
	}

	/// Select the language's collection alternative before considering layout.
	/// The representation keeps the source item even at a block whose checked
	/// result type is already the destination union.
	fn implicit_collection_view(&self, from: Repr, to: Repr) -> bool {
		let (source, target) = match (from, to) {
			(Repr::Array(a), Repr::Array(b))
			| (Repr::Growing(a), Repr::Growing(b))
			| (Repr::Pool(a), Repr::Pool(b))
			| (Repr::Columnar(a), Repr::Columnar(b)) => {
				let (a, b) = (self.concrete_type(a), self.concrete_type(b));
				if !self.types().same_structure_item(a, b) {
					return false;
				}
				(a, b)
			}
			(Repr::Map(a), Repr::Map(b)) | (Repr::Set(a), Repr::Set(b)) => (a, b),
			_ => return false,
		};
		let (source, target) = (self.concrete_type(source), self.concrete_type(target));
		self.same_layout(from, to)
			&& fa_analyzer::lower::copies::implicit_destination(
				self.types(),
				self.session.module_id(self.file),
				source,
				target,
			)
			.is_some()
	}

	/// The value of a box narrowed to a member type of its union.
	pub(crate) fn unbox(&self, value: Val<'ctx>, repr: Repr) -> Result<Val<'ctx>, Error> {
		if value.repr == repr || self.same_layout(value.repr, repr) {
			return Ok(Val { repr, ..value });
		}
		if let Repr::InlineUnion(nominal) = repr {
			return self.boxed_to_inline(self.union_view(value)?, nominal);
		}
		let value = self.union_view(value)?;
		if let Repr::Object(data) = repr
			&& let Some(value) = self.unbox_compact_data(value, data)?
		{
			return Ok(value);
		}
		let ty = self.box_type(Some(repr));
		let pointer = value.value.expect("a box").into_pointer_value();
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		let loaded = self
			.builder
			.build_load(self.llvm_type(repr), data_pointer, "")?;
		Ok(Val::new(repr, loaded))
	}

	/// The member a type designates: a member, or a named union of one member.
	pub(crate) fn member_of_type(&self, ty: TypeId) -> Option<(NominalId, u16)> {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Member(nominal, index) => Some((nominal, index)),
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				let mut members = (0..count).filter(|&index| types.mask_has(mask, index));
				match (members.next(), members.next()) {
					(Some(index), None) => Some((nominal, index)),
					_ => None,
				}
			}
			_ => None,
		}
	}

	/// The data of a member (`None`: a bare member).
	pub(crate) fn member_data(&self, nominal: NominalId, index: u16) -> Option<TypeId> {
		let data = self.types().payload(nominal, index);
		(!data.is_unset() && !self.types().is_literal(data)).then_some(data)
	}

	/// The struct of a box holding data of this representation.
	pub(super) fn box_type(&self, data: Option<Repr>) -> StructType<'ctx> {
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i64_type().into()];
		if let Some(data) = data.filter(|data| data.has_value()) {
			fields.push(self.llvm_type(data));
		}
		self.context.struct_type(&fields, false)
	}

	/// A value of a member, with its data.
	pub(crate) fn build_member(
		&mut self,
		nominal: NominalId,
		index: u16,
		data: Option<Val<'ctx>>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let id = self
			.context
			.i64_type()
			.const_int(self.member_id(nominal, index), false);
		let Some(data) = data.filter(|data| data.repr.has_value()) else {
			if self.inline_union_words(nominal).is_some() {
				let ty = self.inline_union_type(nominal);
				let value = self
					.builder
					.build_insert_value(ty.const_zero(), id, 0, "")?;
				return Ok(Val::new(
					Repr::InlineUnion(nominal),
					value.into_struct_value(),
				));
			}
			// a constant box
			let ty = self.box_type(None);
			let global = self.module.add_global(ty, None, "fa.member");
			global.set_initializer(&ty.const_named_struct(&[id.into()]));
			global.set_constant(true);
			global.set_linkage(inkwell::module::Linkage::Private);
			return Ok(Val::new(Repr::Union, global.as_pointer_value()));
		};
		let data_type = self.member_data(nominal, index).unwrap_or(TypeId::UNKNOWN);
		let data_repr = self.repr_of(data_type, node.index())?;
		let data = self.owned_as(data, data_repr, None)?;
		if let Some(plan) = self.compact_array_payload(nominal, index) {
			let array = self
				.builder
				.build_extract_value(data.value.expect("a payload").into_struct_value(), 0, "")?
				.into_pointer_value();
			let pointer = self.freeze_array_payload(
				&self.builder,
				id.get_zero_extended_constant().expect("a member id"),
				plan,
				array,
				false,
			)?;
			return Ok(self.fresh(Val::new(Repr::Union, pointer)));
		}
		if self.compact_string_payload(nominal, index).is_some() {
			let text = self.builder.build_extract_value(
				data.value.expect("a payload").into_struct_value(),
				0,
				"",
			)?;
			let pointer = self.freeze_string_payload(
				&self.builder,
				id.get_zero_extended_constant().expect("a member id"),
				text,
				false,
			)?;
			return Ok(self.fresh(Val::new(Repr::Union, pointer)));
		}
		if self.compact_scalar_payload(nominal, index).is_some() {
			let pointer = self.freeze_scalar_payload(
				&self.builder,
				id.get_zero_extended_constant().expect("a member id"),
				data.value.expect("a payload"),
			)?;
			return Ok(self.fresh(Val::new(Repr::Union, pointer)));
		}
		if let Some(plan) = self.compact_edge_payload(nominal, index) {
			let pointer = self.freeze_edge_payload(
				&self.builder,
				id.get_zero_extended_constant().expect("a member id"),
				plan,
				data.value.expect("a payload"),
				false,
			)?;
			return Ok(self.fresh(Val::new(Repr::Union, pointer)));
		}
		let ty = self.box_type(Some(data_repr));
		if self.inline_union_words(nominal).is_some() {
			let repr = Repr::InlineUnion(nominal);
			let out = self.alloca(repr, "union")?;
			self.builder
				.build_store(out.pointer, self.inline_union_type(nominal).const_zero())?;
			let tag = self.builder.build_struct_gep(ty, out.pointer, 0, "")?;
			self.builder.build_store(tag, id)?;
			let payload = self.builder.build_struct_gep(ty, out.pointer, 1, "")?;
			self.store_inline_payload(
				&self.builder,
				payload,
				data_repr,
				data.value.expect("a payload"),
			)?;
			return Ok(self.fresh(self.load(out)?));
		}
		if self.packed_push_argument(node) {
			// copied by the push (`packed.rs`): a box on the stack, its data
			// dropped with the statement's temporaries
			let pointer = self.alloca_type(ty.into(), "packed_member")?;
			let id_pointer = self.builder.build_struct_gep(ty, pointer, 0, "")?;
			self.builder.build_store(id_pointer, id)?;
			let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
			self.builder
				.build_store(data_pointer, data.value.expect("data"))?;
			self.fresh(data);
			return Ok(Val::new(Repr::Union, pointer));
		}
		let pointer = if self.arena_member(nominal) {
			self.allocate_tree_type(&self.builder, ty.into())?
		} else {
			self.allocate_for(ty.into())?
		};
		let id_pointer = self.builder.build_struct_gep(ty, pointer, 0, "")?;
		let stored_id = if self.packed_headers() && self.arena_member(nominal) {
			self.context.i64_type().const_int(
				id.get_zero_extended_constant().expect("a member id") | 0x8000,
				false,
			)
		} else {
			id
		};
		self.builder.build_store(id_pointer, stored_id)?;
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		self.builder
			.build_store(data_pointer, data.value.expect("data"))?;
		if self.arena_member(nominal) {
			self.register_arena_payloads(
				&self.builder,
				id.get_zero_extended_constant().expect("a member id"),
				pointer,
				ty,
				&[data_repr],
			)?;
		}
		Ok(self.fresh(Val::new(Repr::Union, pointer)))
	}

	/// `fa_alloc(size, 16)`.
	pub(crate) fn allocate(&self, size: IntValue<'ctx>) -> Result<PointerValue<'ctx>, Error> {
		self.allocate_aligned(size, 16)
	}

	/// A heap block for a value of this LLVM type: aligned on 8 bytes, or 16
	/// when it holds a 128-bit integer, so that the allocator can give small
	/// blocks in steps of 8 bytes (a box of 24 bytes takes 24, not 32).
	pub(crate) fn allocate_for(
		&self,
		ty: inkwell::types::BasicTypeEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		self.allocate_for_with(&self.builder, ty)
	}

	pub(super) fn allocate_for_with(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		ty: inkwell::types::BasicTypeEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		fn wide(ty: inkwell::types::BasicTypeEnum<'_>) -> bool {
			match ty {
				BasicTypeEnum::IntType(int) => int.get_bit_width() > 64,
				BasicTypeEnum::FloatType(_) | BasicTypeEnum::PointerType(_) => false,
				BasicTypeEnum::StructType(structure) => {
					structure.get_field_types().into_iter().any(wide)
				}
				BasicTypeEnum::ArrayType(array) => wide(array.get_element_type()),
				_ => true,
			}
		}
		let size = inkwell::types::BasicType::size_of(&ty).expect("a sized value");
		let size = builder.build_int_cast(size, self.usize, "")?;
		let align = self.usize.const_int(if wide(ty) { 16 } else { 8 }, false);
		Ok(builder
			.build_call(self.runtime.alloc()?, &[size.into(), align.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a block")
			.into_pointer_value())
	}

	fn allocate_aligned(
		&self,
		size: IntValue<'ctx>,
		align: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		let size = self.builder.build_int_cast(size, self.usize, "")?;
		let align = self.usize.const_int(align, false);
		let alloc = self.runtime.alloc()?;
		Ok(self
			.call_value(alloc, &[size.into(), align.into()])?
			.into_pointer_value())
	}

	/// The id of the member in a box.
	pub(crate) fn box_id(&self, value: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		if matches!(value.repr, Repr::InlineUnion(_)) {
			return Ok(self
				.builder
				.build_extract_value(value.value.expect("a union").into_struct_value(), 0, "")?
				.into_int_value());
		}
		let pointer = value.value.expect("a box").into_pointer_value();
		self.union_id(&self.builder, pointer)
	}

	/// Whether a box holds one of the members of `ty` (a member, a named
	/// union, `Error`).
	pub(crate) fn box_is(&mut self, value: Val<'ctx>, ty: TypeId) -> Result<IntValue<'ctx>, Error> {
		self.box_is_typed(value, ty, None)
	}

	pub(super) fn box_is_typed(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		subject: Option<TypeId>,
	) -> Result<IntValue<'ctx>, Error> {
		let types = self.types();
		let members: Vec<(NominalId, u16)> = match types.kind(ty) {
			TypeKind::Member(nominal, index) => vec![(nominal, index)],
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				(0..count)
					.filter(|&index| types.mask_has(mask, index))
					.map(|index| (nominal, index))
					.collect()
			}
			TypeKind::ErrorTop | TypeKind::Unknown => {
				return Ok(self.context.bool_type().const_int(1, false));
			}
			_ => {
				// a member type of an anonymous union, or a union of some of them
				let items = self.union_items(ty).unwrap_or_else(|| vec![ty]);
				let id = self.box_id(value)?;
				let mut result = self.context.bool_type().const_zero();
				for item in items {
					let expected = self.type_id(item);
					let equal = self.builder.build_int_compare(
						IntPredicate::EQ,
						id,
						self.context.i64_type().const_int(expected, false),
						"",
					)?;
					result = self.builder.build_or(result, equal, "")?;
				}
				return Ok(result);
			}
		};
		// A statically named subject cannot contain an anonymous type id.
		// Compare its exact stored tag, including the compact marker, instead
		// of decoding a general union's full-width discriminant.
		let subject_nominal = subject.and_then(|ty| {
			let ty = self.without_none(ty);
			let ty = self.types().rec_target(ty).unwrap_or(ty);
			match self.types().kind(ty) {
				TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => Some(nominal),
				_ => None,
			}
		});
		let narrow = self.packed_headers()
			&& value.repr == Repr::Union
			&& subject_nominal.is_some_and(|nominal| self.arena_member(nominal));
		let byte_tag = narrow
			&& subject_nominal.is_some_and(|nominal| {
				members.iter().all(|(owner, _)| *owner == nominal) && self.byte_root_tags(nominal)
			});
		let mut id = if narrow {
			self.builder
				.build_load(
					if byte_tag {
						self.context.i8_type()
					} else {
						self.context.i16_type()
					},
					value.value.expect("a box").into_pointer_value(),
					"member_tag",
				)?
				.into_int_value()
		} else {
			self.box_id(value)?
		};
		if narrow && !byte_tag && self.tree_arenas.near_links {
			id = self
				.builder
				.build_and(id, id.get_type().const_int(0xbfff, false), "")?;
		}
		let mut result = self.context.bool_type().const_zero();
		for (nominal, index) in members {
			let mut member = self.member_id(nominal, index);
			if narrow && self.arena_member(nominal) && self.member_data(nominal, index).is_some() {
				member |= 0x8000;
			}
			let equal = self.builder.build_int_compare(
				IntPredicate::EQ,
				id,
				id.get_type().const_int(member, false),
				"",
			)?;
			result = self.builder.build_or(result, equal, "")?;
		}
		Ok(result)
	}

	/// The data of a box whose member is known (`ty`: a member type, or a
	/// named union of one member).
	pub(crate) fn box_data(
		&self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let value = self.union_view(value)?;
		let Some((nominal, index)) = self.member_of_type(ty) else {
			return self.unbox(value, self.repr_of(ty, node.index())?);
		};
		let Some(data) = self.member_data(nominal, index) else {
			return Err(self.unsupported(node.index(), "the data of a member without data"));
		};
		let repr = self.repr_of(data, node.index())?;
		let pointer = value.value.expect("a box").into_pointer_value();
		let loaded = self.member_payload_value(&self.builder, pointer, nominal, index, repr)?;
		Ok(Val::new(repr, loaded))
	}

	/// A pointer to the data of a box whose member is known.
	fn box_data_pointer(
		&self,
		pointer: PointerValue<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let Some((nominal, index)) = self.member_of_type(ty) else {
			return Err(
				self.unsupported(node.index(), "the data of a union whose member isn't known")
			);
		};
		let data = self.member_data(nominal, index).unwrap_or(TypeId::UNKNOWN);
		let repr = self.repr_of(data, node.index())?;
		if let Some(plan) = self.compact_array_payload(nominal, index) {
			return self.compact_payload_place(pointer, plan);
		}
		if self.compact_string_payload(nominal, index).is_some() {
			let value = self.string_payload_value(&self.builder, pointer, data)?;
			let out = self.alloca(repr, "member_view")?;
			self.builder.build_store(out.pointer, value)?;
			return Ok(out);
		}
		if self.compact_scalar_payload(nominal, index).is_some() {
			return Ok(Slot {
				pointer: self.payload_offset(&self.builder, pointer, 4)?,
				repr,
			});
		}
		if let Some(plan) = self.compact_edge_payload(nominal, index) {
			let value = self.edge_payload_value(&self.builder, pointer, plan)?;
			let out = self.alloca(repr, "member_view")?;
			self.builder.build_store(out.pointer, value)?;
			return Ok(out);
		}
		let ty = self.box_type(Some(repr));
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		Ok(Slot {
			pointer: data_pointer,
			repr,
		})
	}

	// ── errors ──────────────────────────────────────────────────────────

	/// Raises an error: to the innermost `catch` of the function, else to the
	/// caller, else (the top level) the program ends with it.
	pub(crate) fn raise(&mut self, error: PointerValue<'ctx>) -> Result<(), Error> {
		let frame = self.frame();
		if let Some(&(block, slot, mark)) = frame.catches.last() {
			self.drop_to(mark)?;
			if let Some(slot) = slot {
				self.builder.build_store(slot, error)?;
			} else {
				self.call_drop(&self.builder, Repr::Union, error.into())?;
			}
			self.builder.build_unconditional_branch(block)?;
		} else if self.frame().throws {
			self.drop_to(0)?;
			let ret = self.frame().ret;
			match self.raw_return_type() {
				Some(BasicTypeEnum::StructType(structure)) if ret.has_value() => {
					let pair = self.builder.build_insert_value(
						structure.get_undef(),
						super::zero_of(self.llvm_type(ret)),
						0,
						"",
					)?;
					let pair = self.builder.build_insert_value(pair, error, 1, "")?;
					self.return_raw(Some(inkwell::values::BasicValue::as_basic_value_enum(
						&pair,
					)))?;
				}
				_ => {
					self.return_raw(Some(error.into()))?;
				}
			}
		} else {
			self.call_noreturn("fa_uncaught", &[error.into()])?;
		}
		self.continue_unreachable();
		Ok(())
	}

	/// Raises `error` when it isn't null.
	pub(crate) fn raise_if_error(&mut self, error: PointerValue<'ctx>) -> Result<(), Error> {
		let failed = self.builder.build_is_not_null(error, "")?;
		self.guard(failed, |this| this.raise(error))
	}

	/// Calls a runtime function returning an error, and raises it (in a cold block).
	pub(crate) fn raise_runtime(
		&mut self,
		name: &str,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<(), Error> {
		self.raise_runtime_with(name, |_| Ok(arguments.to_vec()))
	}

	/// Build diagnostic-only arguments only when the handler observes them.
	pub(crate) fn raise_runtime_with(
		&mut self,
		name: &str,
		arguments: impl FnOnce(&mut Self) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, Error>,
	) -> Result<(), Error> {
		// IndexError construction has no observable effect when the local
		// handler never reads its payload. User expressions are evaluated
		// before this call; retain all cleanups before entering the handler.
		if name == "fa_error_index"
			&& let Some(&(block, None, mark)) = self.frame().catches.last()
		{
			self.drop_to(mark)?;
			self.builder.build_unconditional_branch(block)?;
			self.continue_unreachable();
			return Ok(());
		}
		let arguments = arguments(self)?;
		let function = self.runtime.function(name)?;
		let error = self.call_value(function, &arguments)?.into_pointer_value();
		self.raise(error)
	}

	/// `expression catch error => handler`.
	pub(crate) fn catch_expression(
		&mut self,
		node: NodeId,
		expression: NodeId,
		handler: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// Unconstrained console arguments can have no inferred catch type.
		// Preserve both branches, rather than assuming the handler has the
		// successful operand's representation (e.g. String catch => Integer).
		let repr = if self.type_of(node) == TypeId::UNKNOWN {
			let success = self.node_repr(expression)?;
			let failure = self.node_repr(handler)?;
			let empty_array = |node| matches!(self.view(self.ungrouped(node)), View::Array { items } if items.iter().next().is_none());
			if success == failure || failure == Repr::Never || empty_array(handler) {
				success
			} else if success == Repr::Never || empty_array(expression) {
				failure
			} else if let (Repr::Int(a), Repr::Int(b)) = (success, failure) {
				Repr::Int(a.common(b))
			} else {
				Repr::Union
			}
		} else {
			self.node_repr(node)?
		};
		let slot = if repr.has_value() {
			Some(self.alloca(repr, "catch")?)
		} else {
			None
		};
		let binding = self.analysis.symbols.symbol_of_decl(node);
		let observes_error = binding.is_some_and(|symbol| !self.uses_of(symbol).is_empty());
		let error_slot = observes_error
			.then(|| self.alloca(Repr::Union, "caught"))
			.transpose()?;
		let handler_block = self.append_block("catch");
		let end = self.append_block("end_catch");
		let mark = self.mark();
		self.frame_mut()
			.catches
			.push((handler_block, error_slot.map(|slot| slot.pointer), mark));
		let owned = slot.is_some();
		let target = slot.map(|slot| slot.repr);
		let value = self.region(target, expression, |this| this.expr(expression));
		self.frame_mut().catches.pop();
		let value = value?;
		if value.repr != Repr::Never {
			if let Some(slot) = slot {
				self.store(slot, value, expression)?;
			}
			self.branch_to(end)?;
		}
		self.builder.position_at_end(handler_block);
		let handled = self.region(target, handler, |this| {
			// the handler owns the error
			let scope = this.mark();
			if let Some(error_slot) = error_slot {
				let error = this.load(error_slot)?;
				let error = this.fresh(error);
				let symbol = binding.expect("an observed error binding");
				let binding = this.slot_of(symbol, node)?;
				let error = this.take(error, None)?;
				this.store(binding, error, node)?;
				this.own_local(symbol, binding);
			}
			let handled = match this.view(handler) {
				View::Block { .. } => this.block(handler, owned)?,
				_ => this.expr(handler)?,
			};
			let handled = if owned && handled.repr.has_value() {
				this.own(handled, Some(handler))?
			} else {
				handled
			};
			this.end_scope(scope, Some(handled))?;
			Ok(handled)
		})?;
		if handled.repr != Repr::Never {
			if let Some(slot) = slot {
				self.store(slot, handled, handler)?;
			}
			self.branch_to(end)?;
		}
		self.builder.position_at_end(end);
		match slot {
			Some(slot) => Ok(self.fresh(self.load(slot)?)),
			None => Ok(Val::void()),
		}
	}

	/// `Integer.parse(text)`.
	pub(crate) fn parse_integer(&mut self, text: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let (bytes, length) = self.string_parts(text)?;
		let out = self.alloca(Repr::Int(super::values::IntRepr::I64), "parsed")?;
		let parse = self.runtime.function("fa_parse_integer")?;
		let error = self
			.call_value(parse, &[bytes.into(), length.into(), out.pointer.into()])?
			.into_pointer_value();
		self.raise_if_error(error)?;
		self.load(out)
	}

	// ── objects ─────────────────────────────────────────────────────────

	/// The shape index of a field of an object type.
	fn field_index(&self, ty: TypeId, name: fa_analyzer::ids::Sym) -> Option<u32> {
		self.types()
			.fields_of(ty)
			.iter()
			.position(|field| {
				self.types().syms.resolve(field.name) == self.types().syms.resolve(name)
			})
			.map(|index| index as u32)
	}

	/// A field's semantic resolution belongs to its declared shape (possibly
	/// a generic constraint). An instance may have additional fields before
	/// or after it, so only its name identifies the concrete layout slot.
	pub(crate) fn concrete_field_index(&self, owner: TypeId, member: NodeId) -> Result<u32, Error> {
		let Resolution::Field(shape, index) = self.analysis.resolution(member) else {
			return Err(self.unsupported(member.index(), "this field"));
		};
		let name = self.types().shape_fields(shape)[index as usize].name;
		if self
			.types()
			.fields_of(owner)
			.get(index as usize)
			.is_some_and(|field| {
				self.types().syms.resolve(field.name) == self.types().syms.resolve(name)
			}) {
			return Ok(u32::from(index));
		}
		self.field_index(owner, name)
			.ok_or_else(|| self.unsupported(member.index(), "this field of the concrete type"))
	}

	/// An object literal (`{ x = 1 }`, `Point { x = 1 }`, `Shape.Circle { … }`).
	pub(crate) fn object_literal(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// a worker of an asynchronous program: its thread (tasks.rs)
		if let Some(worker) = self.worker_literal(node)? {
			return Ok(worker);
		}
		let Some(info) = self.analysis.object(node).cloned() else {
			return Err(self.unsupported(node.index(), "this object"));
		};
		// `AtomicInteger { value = 0 }`, `Mutex(T) { … }` (parallel.rs)
		if self.sync_kind(info.ty).is_some() {
			return self.sync_literal(node, &info);
		}
		let structure = self.object_type(info.ty);
		let destination = if info.member.is_none() {
			self.destination_for(node, Repr::Object(info.ty))
		} else {
			None
		};
		// the given values in the written order, then the defaults
		let mut order: Vec<usize> = (0..info.fields.len()).collect();
		order.sort_by_key(|&position| match info.fields[position].1 {
			FieldValue::Given(value) | FieldValue::Spread(value) => {
				(0, self.tree.spans[value.index()].start)
			}
			_ => (1, position),
		});
		let mut lowered_fields = Vec::new();
		let mut spreads: rustc_hash::FxHashMap<NodeId, Val<'ctx>> = Default::default();
		let paired = self.paired_boxes(&info, &order, node)?;
		for position in order {
			let (name, field) = info.fields[position];
			let Some(index) = self.field_index(info.ty, name) else {
				continue;
			};
			let repr = self.field_repr(info.ty, index as usize, node.index())?;
			// the box of a recursive field is allocated before its value: a
			// tree built recursively is laid out in the order a walk visits
			// it (parent before children), as the hardware prefetcher likes
			// it. `box_rec` takes the block (an inner literal of the same
			// type may take it first), else it is freed.
			let early = match field {
				_ if paired.contains_key(&position) => paired.get(&position).copied(),
				FieldValue::Given(value) => self.early_box(repr, value)?,
				_ => None,
			};
			let outer = self.pending_box.replace(early);
			// a default of a generic type's field (`#items: Array<Item> = []`):
			// under the literal's type arguments
			let saved = if let FieldValue::Default(default) = field {
				if let Some((params, arguments)) = self.generic_type_arguments(info.ty, default) {
					let mut representations = self.bindings.borrow().clone();
					let mut types = self.type_bindings.borrow().clone();
					for (param, argument) in params.into_iter().zip(arguments) {
						representations.insert(param, self.repr_of(argument, node.index())?);
						types.insert(param, argument);
					}
					Some((
						self.bindings.replace(representations),
						self.type_bindings.replace(types),
					))
				} else {
					None
				}
			} else {
				None
			};
			let field_destination = destination
				.filter(|_| self.uses_return_slot(repr))
				.map(|slot| {
					self.builder
						.build_struct_gep(structure, slot.pointer, index, "result.field")
						.map(|pointer| Slot { pointer, repr })
				})
				.transpose()?;
			let lowered = match field {
				// a default value of a type of another file, evaluated there
				FieldValue::Constant(constant) => self.constant_field(&info, index, constant, node),
				FieldValue::Given(value)
					if early.is_some_and(|(_, repr)| self.uses_return_slot(repr)) =>
				{
					let (pointer, repr) = early.expect("an early box");
					self.expr_with_early_destination(value, Slot { pointer, repr })
				}
				FieldValue::Given(value) | FieldValue::Default(value)
					if field_destination.is_some() =>
				{
					self.expr_with_destination(value, field_destination)
				}
				_ => self.field_value(field, name, repr, &mut spreads),
			};
			let lowered = match lowered {
				Ok(lowered) if lowered.repr != Repr::Never => {
					let source = match field {
						FieldValue::Given(value) | FieldValue::Default(value) => Some(value),
						_ => None,
					};
					let owned = self.owned_as(lowered, repr, source)?;
					if info
						.member
						.and_then(|ty| self.member_of_type(ty))
						.is_some_and(|(nominal, index)| {
							self.compact_array_payload(nominal, index).is_some()
						}) {
						Ok(owned)
					} else {
						self.compact_moved_array(owned, source)
					}
				}
				other => other,
			};
			if let Some((representations, types)) = saved {
				self.bindings.replace(representations);
				self.type_bindings.replace(types);
			}
			// (a box of a pair lives in the arena: nothing to free)
			if let Some((unused, _)) = self.pending_box.replace(outer)
				&& !paired.contains_key(&position)
			{
				let one = self.usize.const_int(1, false);
				self.builder.build_call(
					self.runtime.free()?,
					&[unused.into(), one.into(), one.into()],
					"",
				)?;
			}
			let lowered = lowered?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			if let Some(slot) = destination {
				let pointer = match field_destination {
					Some(field) => field.pointer,
					None => self.builder.build_struct_gep(
						structure,
						slot.pointer,
						index,
						"result.field",
					)?,
				};
				self.store(Slot { pointer, repr }, lowered, node)?;
			}
			// Keep fields owned until the whole constructor succeeds.
			lowered_fields.push((index, self.fresh(lowered)));
		}
		let mut value = structure.get_undef();
		for (index, lowered) in lowered_fields {
			let lowered = self.take(lowered, None)?;
			if destination.is_none()
				&& let Some(raw) = lowered.value
			{
				value = self
					.builder
					.build_insert_value(value, raw, index, "")?
					.into_struct_value();
			}
		}
		let object = self.fresh(match destination {
			Some(slot) => self.load_destination(slot)?,
			None => Val::new(Repr::Object(info.ty), value),
		});
		match info.member.and_then(|member| self.member_of_type(member)) {
			Some((nominal, index)) => self.build_member(nominal, index, Some(object), node),
			None => Ok(object),
		}
	}

	/// The default value of field `index` of a type of another file, which
	/// its file evaluated at compile time (`FieldValue::Constant`): a fresh
	/// value at each creation.
	fn constant_field(
		&mut self,
		info: &fa_analyzer::analysis::ObjectInfo,
		index: u32,
		constant: u32,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(value) = self
			.analysis
			.field_constants
			.get(constant as usize)
			.cloned()
		else {
			return Err(self.unsupported(node.index(), "this default value"));
		};
		let ty = self.types().fields_of(info.ty)[index as usize].ty;
		let built = self.constant_value(&value, ty, node)?;
		Ok(if self.needs_drop(built.repr) {
			self.fresh(built)
		} else {
			built
		})
	}

	/// A new object of representation `to` holding copies of the fields of
	/// `source` it has, by name (`pick(value, { foo })`); a nested object of
	/// another shape is picked the same way. `source` is only read.
	pub(crate) fn picked_copy(
		&mut self,
		source: Val<'ctx>,
		to: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let (Repr::Object(from_type), Repr::Object(to_type)) = (source.repr, to) else {
			return Err(self.unsupported(node.index(), "this `pick` or `omit`"));
		};
		let raw = source.value.expect("an object").into_struct_value();
		let structure = self.object_type(to_type);
		let mut result = structure.get_undef();
		let fields = self.types().fields_of(to_type).to_vec();
		for field in fields {
			let (Some(from_index), Some(to_index)) = (
				self.field_index(from_type, field.name),
				self.field_index(to_type, field.name),
			) else {
				return Err(self.unsupported(node.index(), "this `pick` or `omit`"));
			};
			let from_field = self
				.types()
				.fields_of(from_type)
				.iter()
				.find(|other| other.name == field.name)
				.map(|other| other.ty)
				.expect("a field");
			let from_repr = self.repr_of(from_field, node.index())?;
			let to_repr = self.repr_of(field.ty, node.index())?;
			let extracted = self.builder.build_extract_value(raw, from_index, "")?;
			let value = Val::new(from_repr, extracted);
			let value = match (from_repr, to_repr) {
				(Repr::Object(_), Repr::Object(_)) if from_repr != to_repr => {
					self.picked_copy(value, to_repr, node)?
				}
				_ if self.needs_drop(from_repr) => {
					let copied = self.call_copy(&self.builder, from_repr, extracted)?;
					self.convert(Val::new(from_repr, copied), to_repr, node.index())?
				}
				_ => self.convert(value, to_repr, node.index())?,
			};
			if let Some(raw) = value.value {
				result = self
					.builder
					.build_insert_value(result, raw, to_index, "")?
					.into_struct_value();
			}
		}
		Ok(Val::new(to, result))
	}

	/// The declaration owning this default and its arguments. Structural
	/// instances may have no nominal identity; the default's source and the
	/// recorded application still retain the declaration and phantom values.
	fn generic_type_arguments(
		&self,
		ty: TypeId,
		default: NodeId,
	) -> Option<(Vec<fa_analyzer::ids::ParamId>, Vec<TypeId>)> {
		let ty = self.concrete_type(ty);
		let span = self.tree.spans[default.index()];
		let decl = self.analysis.type_decls.iter().find_map(|(node, decl)| {
			let owner = self.tree.spans[node.index()];
			(!decl.generics.is_empty()
				&& owner.start <= span.start
				&& span.end <= owner.end
				&& decl
					.fields
					.iter()
					.any(|field| field.default == Some(default)))
			.then_some(decl)
		})?;
		if let Some((name, arguments)) = self.types().application(ty)
			&& self.types().syms.resolve(name)
				== self
					.types()
					.syms
					.resolve(self.analysis.symbols.symbol(decl.symbol).name)
		{
			return Some((
				decl.generics.clone(),
				self.types().list_items(arguments).to_vec(),
			));
		}
		let mut found = rustc_hash::FxHashMap::default();
		self.match_params(decl.ty, ty, &mut found, 0);
		let arguments = decl
			.generics
			.iter()
			.map(|param| found.get(param).copied())
			.collect::<Option<Vec<_>>>()?;
		Some((decl.generics.clone(), arguments))
	}

	/// Collects the types that the parameters of `declared` stand for in
	/// `actual`, `declared` substituted.
	pub(crate) fn match_params(
		&self,
		declared: TypeId,
		actual: TypeId,
		found: &mut rustc_hash::FxHashMap<fa_analyzer::ids::ParamId, TypeId>,
		depth: usize,
	) {
		use fa_analyzer::types::{TypeFlags, TypeKind};
		let types = self.types();
		if depth > 32 || !types.flags(declared).contains(TypeFlags::HAS_PARAM) {
			return;
		}
		match (types.kind(declared), types.kind(actual)) {
			(TypeKind::Param(param), _) => {
				found.entry(param).or_insert(actual);
			}
			(TypeKind::Object(declared), TypeKind::Object(actual)) => {
				let actual = types.shape_fields(actual);
				for field in types.shape_fields(declared) {
					if let Some(other) = actual.iter().find(|other| {
						types.syms.resolve(other.name) == types.syms.resolve(field.name)
					}) {
						self.match_params(field.ty, other.ty, found, depth + 1);
					}
				}
			}
			(TypeKind::Tuple(declared), TypeKind::Tuple(actual))
			| (TypeKind::Union(declared), TypeKind::Union(actual)) => {
				let (declared, actual) = (types.list_items(declared), types.list_items(actual));
				if declared.len() == actual.len() {
					for (&declared, &actual) in declared.iter().zip(actual) {
						self.match_params(declared, actual, found, depth + 1);
					}
				}
			}
			(
				TypeKind::Collection(kind, item, key),
				TypeKind::Collection(other, actual_item, actual_key),
			) if kind == other => {
				self.match_params(item, actual_item, found, depth + 1);
				self.match_params(key, actual_key, found, depth + 1);
			}
			(TypeKind::Pointer(declared), TypeKind::Pointer(actual)) => {
				self.match_params(declared, actual, found, depth + 1);
			}
			(TypeKind::Function(declared), TypeKind::Function(actual)) => {
				let (declared, actual) = (types.sig(declared), types.sig(actual));
				let (params, others) = (types.params(declared.params), types.params(actual.params));
				if params.len() == others.len() {
					for (param, other) in params.iter().zip(others) {
						self.match_params(param.ty, other.ty, found, depth + 1);
					}
				}
				self.match_params(declared.ret, actual.ret, found, depth + 1);
			}
			_ => {}
		}
	}

	/// The value of a field of an object literal, before it is taken.
	fn field_value(
		&mut self,
		field: FieldValue,
		name: fa_analyzer::ids::Sym,
		repr: Repr,
		spreads: &mut rustc_hash::FxHashMap<NodeId, Val<'ctx>>,
	) -> Result<Val<'ctx>, Error> {
		{
			let lowered = match field {
				// a relation side: linked by the `add` that stores the element
				FieldValue::Given(value) if matches!(repr, Repr::Link(_) | Repr::Links(_)) => {
					self.literal_side(name, value, repr)?
				}
				FieldValue::Given(value) => self.expr(value)?,
				FieldValue::Default(value) => self.expr(value)?,
				// a field copied from `...value`, evaluated once
				FieldValue::Spread(value) => {
					let source = match spreads.get(&value) {
						Some(source) => *source,
						None => {
							let source = self.expr(value)?;
							if source.repr == Repr::Never {
								return Ok(source);
							}
							spreads.insert(value, source);
							source
						}
					};
					let Repr::Object(source_type) = source.repr else {
						return Err(self.unsupported(value.index(), "spreading this value"));
					};
					let Some(source_index) = self.field_index(source_type, name) else {
						return Err(self.unsupported(value.index(), "spreading this value"));
					};
					let source_repr =
						self.field_repr(source_type, source_index as usize, value.index())?;
					let structure = source.value.expect("an object").into_struct_value();
					let raw = self
						.builder
						.build_extract_value(structure, source_index, "")?;
					// a field of the spread value: copied
					self.fresh(self.emit_copy(Val::new(source_repr, raw))?)
				}
				FieldValue::None => Val::none(),
				// lowered by the caller (`constant_field`)
				FieldValue::Constant(_) => {
					return Err(self.unsupported(0, "this default value"));
				}
			};
			Ok(lowered)
		}
	}

	/// For a field holding a recursive edge (`left: Node?`) given a value of
	/// the target type that can't leave the expression early (no `try`,
	/// `throw`, `return`, `break`, `continue`): a block for its box, and the
	/// representation of the value.
	/// The boxes of the recursive fields of a literal whose values are calls
	/// or literals (see [`Self::early_box`]), when at least two of the same
	/// type live in a root's arena: one allocation for all of them, side by
	/// side in the order of the fields, so that the children of a node share
	/// a cache line and an allocation check (`binary-trees`), keyed by the
	/// field's position in the literal.
	fn paired_boxes(
		&self,
		info: &fa_analyzer::analysis::ObjectInfo,
		order: &[usize],
		node: NodeId,
	) -> Result<rustc_hash::FxHashMap<usize, (PointerValue<'ctx>, Repr)>, Error> {
		let mut paired = rustc_hash::FxHashMap::default();
		let mut candidates = Vec::new();
		for &position in order {
			let (name, FieldValue::Given(value)) = info.fields[position] else {
				continue;
			};
			let Some(index) = self.field_index(info.ty, name) else {
				continue;
			};
			let repr = self.field_repr(info.ty, index as usize, node.index())?;
			if let Some(target) = self.early_box_target(repr, value)?
				&& self.arena_repr(target)
			{
				candidates.push((position, target));
			}
		}
		let Some(&(_, target)) = candidates.first() else {
			return Ok(paired);
		};
		if candidates.len() < 2 || candidates.iter().any(|(_, other)| *other != target) {
			return Ok(paired);
		}
		let ty = self.llvm_type(target);
		// every node on 8 bytes, as `fa_arena_alloc` places them: a compressed
		// link counts in units of 8 bytes
		let size = self.target_data().get_abi_size(&ty).next_multiple_of(8);
		let align = u64::from(self.target_data().get_abi_alignment(&ty));
		let block = self
			.call_value(
				self.runtime.function("fa_arena_alloc")?,
				&[
					self.usize
						.const_int(size * candidates.len() as u64, false)
						.into(),
					self.usize.const_int(align, false).into(),
				],
			)?
			.into_pointer_value();
		let units: Vec<u64> = (0..candidates.len() as u64)
			.map(|rank| size / 8 * rank)
			.collect();
		let nodes = self.through_links(&self.builder, target, block, &units)?;
		for ((position, target), pointer) in candidates.into_iter().zip(nodes) {
			paired.insert(position, (pointer, target));
		}
		Ok(paired)
	}

	/// The representation of the box [`Self::early_box`] allocates, if any.
	fn early_box_target(&self, field: Repr, value: NodeId) -> Result<Option<Repr>, Error> {
		let (Repr::Rec(target) | Repr::Optional(super::values::Scalar::Rec(target))) = field else {
			return Ok(None);
		};
		let target_repr = self.repr_of(target, value.index())?;
		if !matches!(target_repr, Repr::Object(_))
			|| self.node_repr(value)? != target_repr
			|| !matches!(
				self.view(value),
				View::Call { .. } | View::TypedObject { .. } | View::Object { .. }
			) {
			return Ok(None);
		}
		let mut stack = vec![value.index()];
		while let Some(current) = stack.pop() {
			if matches!(
				self.view(NodeId::new(current)),
				View::Try(_)
					| View::Throw(_)
					| View::Return(_)
					| View::Break(_)
					| View::Continue
					| View::Lambda { .. }
			) {
				return Ok(None);
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		Ok(Some(target_repr))
	}

	fn early_box(
		&self,
		field: Repr,
		value: NodeId,
	) -> Result<Option<(PointerValue<'ctx>, Repr)>, Error> {
		let (Repr::Rec(target) | Repr::Optional(super::values::Scalar::Rec(target))) = field else {
			return Ok(None);
		};
		let target_repr = self.repr_of(target, value.index())?;
		if !matches!(target_repr, Repr::Object(_))
			|| self.node_repr(value)? != target_repr
			|| !matches!(
				self.view(value),
				View::Call { .. } | View::TypedObject { .. } | View::Object { .. }
			) {
			return Ok(None);
		}
		let mut stack = vec![value.index()];
		while let Some(current) = stack.pop() {
			if matches!(
				self.view(NodeId::new(current)),
				View::Try(_)
					| View::Throw(_)
					| View::Return(_)
					| View::Break(_)
					| View::Continue
					| View::Lambda { .. }
			) {
				return Ok(None);
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		Ok(Some((self.alloc_rec(target_repr)?, target_repr)))
	}

	/// The field order for printing: the declaration's, else the shape's.
	pub(crate) fn printed_fields(&self, ty: TypeId) -> Vec<(String, u32, TypeId)> {
		let types = self.types();
		let shape = types.fields_of(ty);
		let declared = self
			.analysis
			.type_decls
			.values()
			.find(|declaration| declaration.ty == ty)
			.map(|declaration| {
				declaration
					.fields
					.iter()
					.map(|field| field.name)
					.collect::<Vec<_>>()
			})
			.filter(|fields| !fields.is_empty());
		// an anonymous type: the order of the first literal that builds it
		let literal = || {
			let mut literals: Vec<_> = self
				.analysis
				.objects
				.iter()
				.filter(|(_, info)| info.ty == ty && info.decl.is_none())
				.collect();
			literals.sort_by_key(|(node, _)| **node);
			literals.first().map(|(_, info)| {
				info.fields
					.iter()
					.map(|(name, _)| *name)
					.collect::<Vec<_>>()
			})
		};
		// a member's inline data, a type built by `Pick` or `Omit`
		let inline = || self.analysis.inline_field_orders.get(&ty).cloned();
		let order: Vec<fa_analyzer::ids::Sym> = declared
			.or_else(inline)
			.or_else(literal)
			.unwrap_or_else(|| shape.iter().map(|field| field.name).collect());
		order
			.into_iter()
			// private fields (`#secret`) and relation sides (D21) don't print
			.filter(|&name| !types.syms.resolve(name).starts_with('#'))
			.filter(|&name| self.side_repr(ty, name).is_none())
			.filter_map(|name| {
				let index = shape.iter().position(|field| {
					self.types().syms.resolve(field.name) == self.types().syms.resolve(name)
				})?;
				Some((
					types.syms.resolve(name).to_string(),
					index as u32,
					shape[index].ty,
				))
			})
			.collect()
	}

	/// Appends `{ x = 1, y = 2 }`.
	pub(crate) fn append_object(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		debug_assert_eq!(value.repr, Repr::Object(ty));
		self.append_aggregate(value, node)
	}

	pub(super) fn append_object_fields(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		let fields = self.printed_fields(ty);
		self.append_ordered_fields(value, fields, node)
	}

	fn append_ordered_fields(
		&mut self,
		value: Val<'ctx>,
		fields: Vec<(String, u32, TypeId)>,
		node: NodeId,
	) -> Result<(), Error> {
		if fields.is_empty() {
			let text = self.string_constant("{}");
			return self.append_value(text, node);
		}
		let structure = value.value.expect("an object").into_struct_value();
		for (position, (name, index, field_type)) in fields.into_iter().enumerate() {
			let prefix = if position == 0 {
				format!("{{ {name} = ")
			} else {
				format!(", {name} = ")
			};
			let prefix = self.string_constant(&prefix);
			self.append_value(prefix, node)?;
			let repr = self.repr_of(field_type, node.index())?;
			let raw = self.builder.build_extract_value(structure, index, "")?;
			self.append_nested_of(Val::new(repr, raw), field_type, node)?;
		}
		let suffix = self.string_constant(" }");
		self.append_value(suffix, node)
	}

	/// Appends a value nested in a composite: strings are quoted.
	pub(crate) fn append_nested(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		match value.repr {
			Repr::Str => {
				let (bytes, length) = self.string_parts(value)?;
				self.call_runtime("fa_b_quoted", &[bytes.into(), length.into()])
			}
			// an optional string or union: its value quoted the same
			Repr::Optional(
				super::values::Scalar::Str
				| super::values::Scalar::Union
				| super::values::Scalar::InlineUnion(_),
			) => self.append_optional(value, node, true),
			Repr::Union | Repr::InlineUnion(_) => {
				let value = self.union_view(value)?;
				let nested = self.context.i32_type().const_int(1, false);
				self.call_runtime(
					"fa_format_union",
					&[value.value.expect("a box").into(), nested.into()],
				)
			}
			_ => self.append_value(value, node),
		}
	}

	/// `value.a.b`: fields of objects and of the data of union members,
	/// built-in properties, members of enumerations and unions.
	pub(crate) fn member_access(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		// `Palette.Color.red`, `Self.Shade.Dark`: the types declared inside
		// types first; the rest reads like a member of the last of them
		let nested = members
			.iter()
			.take_while(|&member| {
				self.analysis
					.has_flags(member, fa_analyzer::analysis::NodeFlags::TYPE_EXPR)
			})
			.count();
		if nested > 0
			&& nested < members.len()
			&& let Some(inner) = members.get(nested - 1)
		{
			return self.member_access(node, inner, members.skip(nested));
		}
		let first = members.first().expect("a member");
		if members.len() == 1
			&& let Resolution::Builtin(property) = self.analysis.resolution(first)
			&& fa_analyzer::builtins::get(property).path == "String#length"
			&& matches!(self.view(target), View::Call { .. })
			&& self.node_repr(target)? == Repr::Str
			&& let Some(info) = self.analysis.call(target)
			&& let fa_analyzer::analysis::Callee::Builtin(method) = info.callee
			&& fa_analyzer::builtins::get(method).path == "Array#join"
		{
			let arguments: Vec<_> = info
				.arguments
				.iter()
				.filter_map(|argument| match argument {
					fa_analyzer::analysis::Argument::Given(node) => Some(*node),
					_ => None,
				})
				.collect();
			if arguments.len() == 1 {
				// The ordinary method lowering preserves receiver/argument order.
				return self.array_method(target, "Array#joinedLength", &arguments);
			}
		}
		if let Some(value) = self.text_count_prefix(node, target, first)? {
			return self.apply_members(
				node,
				value,
				self.type_of(first),
				members.skip(1),
				members.len() - 1,
			);
		}

		// `value?.a.b`: the members after an optional access continue its chain
		if let View::OptionalAccess {
			target: inner,
			members: before,
		} = self.view(target)
			&& matches!(self.node_repr(node)?, Repr::Optional(_))
		{
			let chain: Vec<NodeId> = before.iter().chain(members.iter()).collect();
			return self.optional_chain(node, inner, &chain);
		}
		// `Color.red`, `E.Boom`: a member of a nominal type; after types
		// declared inside types (`Database.OpenError.Failed`,
		// `library.Farm.Error.Empty`), the path before it names types only
		let nested = members.len() > 1
			&& members.iter().take(members.len() - 1).any(|member| {
				self.analysis
					.has_flags(member, fa_analyzer::analysis::NodeFlags::TYPE_EXPR)
			});
		let first = if nested {
			members.last().unwrap_or(first)
		} else {
			first
		};
		if let Resolution::Member(nominal, index) = self.analysis.resolution(first)
			&& (members.len() == 1 || nested)
		{
			return match self.node_repr(node)? {
				Repr::Enum(_) => Ok(Val::new(
					self.node_repr(node)?,
					self.context.i32_type().const_int(index as u64, false),
				)),
				_ => self.build_member(nominal, index, None, node),
			};
		}
		// `Integer.maxValue`, `Float.infinity`…
		if let Resolution::Builtin(id) = self.analysis.resolution(first)
			&& members.len() == 1
			&& let Some(value) = self.number_constant(fa_analyzer::builtins::get(id).path, node)?
		{
			return Ok(value);
		}
		// `Color.members`: every member of an enumeration, in order
		if let Resolution::Builtin(id) = self.analysis.resolution(first)
			&& fa_analyzer::builtins::get(id).path == "Enum.members"
			&& let Repr::Array(item_type) = self.node_repr(first)?
			&& let Repr::Enum(nominal) = self.repr_of(item_type, node.index())?
		{
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len() as u64;
			let length = self.context.i64_type().const_int(count, false);
			if members.len() == 2
				&& let Some(last) = members.last()
				&& let Resolution::Builtin(property) = self.analysis.resolution(last)
				&& fa_analyzer::builtins::get(property).path == "Array#length"
			{
				return Ok(Val::new(Repr::Int(super::values::IntRepr::I64), length));
			}
			let array = self.new_array(Repr::Enum(nominal), length, length)?;
			let (items, _) = self.array_parts(array)?;
			for index in 0..count {
				let position = self.context.i64_type().const_int(index, false);
				let pointer = unsafe {
					self.builder.build_in_bounds_gep(
						self.context.i32_type(),
						items,
						&[position],
						"",
					)?
				};
				self.builder
					.build_store(pointer, self.context.i32_type().const_int(index, false))?;
			}
			let value = self.fresh(Val::new(Repr::Array(item_type), array));
			return self.apply_members(
				node,
				value,
				self.type_of(first),
				members.skip(1),
				members.len() - 1,
			);
		}
		// `library.counter.count`: a value of another file or of a namespace
		let list: Vec<NodeId> = members.iter().collect();
		if let Some(value) = self.namespace_access(node, target, &list)? {
			if value.repr == Repr::Never {
				return Ok(value);
			}
			let narrowed = self.node_repr(node).unwrap_or(value.repr);
			return self.narrow_to(value, narrowed, node);
		}
		// a move-only field moved out of its place: the place gives it up
		if self.analysis.moved_reads.contains(&node)
			&& self.is_place(node)
			&& let Ok(slot) = self.place(node)
		{
			let value = self.load(slot)?;
			if value.repr != Repr::Never {
				self.give_up_place(node, slot)?;
				let value = self.fresh(value);
				let narrowed = self.node_repr(node).unwrap_or(value.repr);
				return self.narrow_to(value, narrowed, node);
			}
		}
		// `self.text` read once at the entry of the method (`frozen.rs`)
		if !self.analysis.moved_reads.contains(&node)
			&& let Some(first) = members.first()
			&& let Some((mut value, mut current_type)) = self.frozen_value(target, first)
		{
			for member in members.iter().skip(1) {
				if value.repr == Repr::Never {
					return Ok(value);
				}
				(value, current_type) = self.apply_member(node, value, current_type, member)?;
			}
			if value.repr == Repr::Never {
				return Ok(value);
			}
			let narrowed = self.node_repr(node).unwrap_or(value.repr);
			return self.narrow_to(value, narrowed, node);
		}
		// `node.world.translation` of a local or a parameter object: load the
		// field alone, not the whole object (whose aggregate load LLVM may then
		// turn into an object passed by value, `sumWorld` of `scene`)
		if let Some(value) = self.field_read(node, target, members)? {
			return Ok(value);
		}
		let value = self.expr(target)?;
		let count = members.len();
		let value = self.apply_members(node, value, self.type_of(target), members, count)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let narrowed = self.node_repr(node).unwrap_or(value.repr);
		self.narrow_to(value, narrowed, node)
	}

	/// The first `count` members of an access applied to a value: fields of
	/// objects and of the data of boxes, built-in properties.
	pub(crate) fn apply_members(
		&mut self,
		node: NodeId,
		mut value: Val<'ctx>,
		mut current_type: TypeId,
		members: NodeList<'_>,
		count: usize,
	) -> Result<Val<'ctx>, Error> {
		for member in members.iter().take(count) {
			if value.repr == Repr::Never {
				return Ok(value);
			}
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		Ok(value)
	}

	/// One member applied to a value: a field, or a built-in property.
	pub(crate) fn apply_member(
		&mut self,
		node: NodeId,
		mut value: Val<'ctx>,
		current_type: TypeId,
		member: NodeId,
	) -> Result<(Val<'ctx>, TypeId), Error> {
		match self.analysis.resolution(member) {
			Resolution::Field(..) => {
				if matches!(value.repr, Repr::Union | Repr::InlineUnion(_)) {
					value = self.box_data(value, current_type, member)?;
				}
				if let Repr::Rec(target) = value.repr {
					value = self.unfold_rec(value, target, member.index())?;
				}
				let Repr::Object(object) = value.repr else {
					return Err(self.unsupported(member.index(), "this field access"));
				};
				let index = self.concrete_field_index(object, member)?;
				let field_type = self.types().fields_of(object)[index as usize].ty;
				let repr = self.field_repr(object, index as usize, member.index())?;
				let structure = value.value.expect("an object").into_struct_value();
				let raw = self.builder.build_extract_value(structure, index, "")?;
				// a move-only field (a task, a resource) moves out of a
				// temporary object: its other fields are dropped now, the
				// field is a temporary itself
				if self.types().is_unique(field_type)
					&& self.destroy_function(object).is_none()
					&& self.is_temporary(value)
				{
					self.drop_all_but(value, index as usize)?;
					return Ok((self.fresh(Val::new(repr, raw)), field_type));
				}
				// a relation side holding one element reads as a view of it
				let value = self.link_view(Val::new(repr, raw), member)?;
				// a field narrowed by a test (`if h.items is not none`)
				if self.flags(member).contains(NodeFlags::NARROWED) {
					let narrowed = self.type_of(member);
					let repr = self.repr_of(narrowed, member.index())?;
					return Ok((self.narrow_to(value, repr, member)?, narrowed));
				}
				Ok((value, field_type))
			}
			Resolution::Builtin(_) => {
				let value = self.builtin_property(node, value, member)?;
				Ok((value, self.type_of(member)))
			}
			// `Color.green` inside a longer access
			Resolution::Member(nominal, index) => {
				let ty = self.type_of(member);
				let value = match self.repr_of(ty, member.index())? {
					Repr::Enum(_) => Val::new(
						Repr::Enum(nominal),
						self.context.i32_type().const_int(index as u64, false),
					),
					_ => self.build_member(nominal, index, None, member)?,
				};
				Ok((value, ty))
			}
			_ => Err(self.unsupported(member.index(), "this member")),
		}
	}

	/// `value?.a.b`: `none` when the value (or an optional along the chain)
	/// is `none`, the member otherwise.
	pub(crate) fn optional_access(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let members: Vec<NodeId> = members.iter().collect();
		self.optional_chain(node, target, &members)
	}

	/// The members of an optional chain applied to its target.
	fn optional_chain(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let result = self.alloca(repr, "chain")?;
		let none = self.convert(Val::none(), repr, node.index())?;
		if let Some(raw) = none.value {
			self.builder.build_store(result.pointer, raw)?;
		}
		let end = self.append_block("end_chain");
		let mut value = self.expr(target)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		// a target narrowed to `none`: the chain is `none`
		if value.repr == Repr::None {
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(end);
			return self.load(result);
		}
		let mut current_type = self.type_of(target);
		for &member in members {
			value = self.present_or_exit(value, end)?;
			current_type = self.without_none(current_type);
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		if let Repr::None = value.repr {
			self.builder.build_unconditional_branch(end)?;
		} else {
			self.store(result, value, node)?;
			self.builder.build_unconditional_branch(end)?;
		}
		self.builder.position_at_end(end);
		self.load(result)
	}

	/// `value?.a.method(…)`: `none` when the value (or an optional along the
	/// chain) is `none`, the result of the call otherwise.
	pub(crate) fn optional_call(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let result = match repr.has_value() {
			true => Some(self.alloca(repr, "chain")?),
			false => None,
		};
		if let Some(result) = result {
			let none = self.convert(Val::none(), repr, node.index())?;
			if let Some(raw) = none.value {
				self.builder.build_store(result.pointer, raw)?;
			}
		}
		let end = self.append_block("end_chain");
		let mut value = self.expr(target)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let mut current_type = self.type_of(target);
		let count = members.len();
		for member in members.iter().take(count - 1) {
			value = self.present_or_exit(value, end)?;
			current_type = self.without_none(current_type);
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		let receiver = self.present_or_exit(value, end)?;
		let called = self.region(result.map(|_| repr), node, |this| {
			this.pending_receiver = Some(receiver);
			let called = this.call_present(node);
			this.pending_receiver = None;
			called
		})?;
		if let Some(result) = result
			&& called.repr.has_value()
		{
			self.store(result, called, node)?;
		}
		self.branch_to(end)?;
		self.builder.position_at_end(end);
		match result {
			Some(result) => Ok(self.fresh(self.load(result)?)),
			None => Ok(Val::void()),
		}
	}

	/// The value of an optional, continuing only when it is present
	/// (branching to `exit` otherwise).
	pub(crate) fn present_or_exit(
		&mut self,
		value: Val<'ctx>,
		exit: inkwell::basic_block::BasicBlock<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Optional(_) = value.repr else {
			return Ok(value);
		};
		let (present, inner) = self.split_optional(&self.builder, value)?;
		let next = self.append_block("chain_present");
		self.builder.build_conditional_branch(present, next, exit)?;
		self.builder.position_at_end(next);
		Ok(inner)
	}

	/// A type without `none`.
	pub(super) fn without_none(&self, ty: TypeId) -> TypeId {
		let types = self.types();
		if let TypeKind::Union(list) = types.kind(ty) {
			let items: Vec<TypeId> = types
				.list_items(list)
				.iter()
				.copied()
				.filter(|&item| item != TypeId::NONE)
				.collect();
			if items.len() == 1 {
				return items[0];
			}
		}
		ty
	}

	/// A direct or virtual text collection followed by `length`. Flattened
	/// method receivers must consume this fact before evaluating the array.
	fn text_count_prefix(
		&mut self,
		node: NodeId,
		target: NodeId,
		first: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		if self.tree.text(first.index()) != "length" {
			return Ok(None);
		}
		let call = self.virtual_code_points_call(target).or_else(|| {
			self.code_points_call(target).filter(|&call| {
				self.is_bytes_call(call)
					|| self.analysis.has_flags(node, NodeFlags::CODE_POINT_COUNT)
					|| self.analysis.has_flags(first, NodeFlags::CODE_POINT_COUNT)
			})
		});
		let Some(call) = call else {
			return Ok(None);
		};
		let text = self.receiver(call)?;
		if text.repr == Repr::Never {
			return Ok(Some(text));
		}
		let count = self.text_view_length(call, text)?;
		Ok(Some(Val::new(
			Repr::Int(super::values::IntRepr::I64),
			count,
		)))
	}

	/// The receiver of a method call `a.b.method(…)`: `a.b`.
	pub(crate) fn receiver(&mut self, call: NodeId) -> Result<Val<'ctx>, Error> {
		if let Some(value) = self.pending_receiver.take() {
			return Ok(value);
		}
		let View::Call { mut callee, .. } = self.view(call) else {
			return Err(self.unsupported(call.index(), "this method call"));
		};
		while let View::Group(inner) = self.view(callee) {
			callee = inner;
		}
		let View::Access { target, members } = self.view(callee) else {
			return Err(self.unsupported(call.index(), "this method call"));
		};
		let count = members.len() - 1;
		// a value of another file reached through namespaces (`library.people`)
		let list: Vec<NodeId> = members.iter().collect();
		if count > 0
			&& let Some(slot) = self.namespace_place(callee, target, &list[..count])?
		{
			return self.load(slot);
		}
		if count > 0
			&& let Some(first) = members.first()
			&& let Some(value) = self.text_count_prefix(callee, target, first)?
		{
			return self.apply_members(
				callee,
				value,
				self.type_of(first),
				members.skip(1),
				count - 1,
			);
		}

		// `self.text.toUtf8()`: the field read once by the method (`frozen.rs`)
		if count >= 1
			&& let Some(first) = members.first()
			&& let Some((mut value, mut current_type)) = self.frozen_value(target, first)
		{
			for member in members.iter().take(count).skip(1) {
				if value.repr == Repr::Never {
					return Ok(value);
				}
				(value, current_type) = self.apply_member(callee, value, current_type, member)?;
			}
			return Ok(value);
		}
		let value = self.expr(target)?;
		self.apply_members(callee, value, self.type_of(target), members, count)
	}

	/// A place: a pointer to a local, a field of a place, or the data of a box.
	/// The fields `a.b.c` of an object held by a local or a parameter of the
	/// function (its slot, not a reference, a requirement, a global, a
	/// reactive or shared value, nor a narrowed union): a load of the field
	/// through its address.
	fn field_read(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Option<Val<'ctx>>, Error> {
		if !matches!(self.view(target), View::Name(_))
			|| self.flags(target).contains(NodeFlags::NARROWED)
			|| !members
				.iter()
				.all(|member| matches!(self.analysis.resolution(member), Resolution::Field(..)))
		{
			return Ok(None);
		}
		let Resolution::Local(symbol) = self.analysis.resolution(target) else {
			return Ok(None);
		};
		if self.references.contains_key(&symbol)
			|| self.reactive_sources.contains_key(&symbol)
			|| self.derived_nodes.contains_key(&symbol)
			|| self.shared.contains_key(&symbol)
			|| self.param_sources.contains_key(&symbol)
		{
			return Ok(None);
		}
		let Some(&slot) = self.slots.get(&symbol) else {
			return Ok(None);
		};
		if !matches!(slot.repr, Repr::Object(_)) || slot.repr != self.node_repr(target)? {
			return Ok(None);
		}
		let field = self.place_members(slot, self.type_of(target), members.iter())?;
		let repr = self.node_repr(node)?;
		if field.repr != repr || !repr.has_value() {
			return Ok(None);
		}
		Ok(Some(self.load(field)?))
	}

	pub(crate) fn place(&mut self, node: NodeId) -> Result<Slot<'ctx>, Error> {
		match self.view(node) {
			View::Name(_) => {
				let resolution = self.analysis.resolution(node);
				if let Some(reached) = self.external(resolution)
					&& let Some(slot) = self.reached_slot(reached, node)?
				{
					return Ok(slot);
				}
				let Resolution::Local(symbol) = resolution else {
					return Err(self.unsupported(node.index(), "this place"));
				};
				let slot = self.slot_of(symbol, node)?;
				if self.flags(node).contains(NodeFlags::NARROWED) {
					// a narrowed local designates its payload (patterns.rs)
					let narrowed = self.node_repr(node)?;
					return self.payload_place(slot, narrowed, node);
				}
				Ok(slot)
			}
			View::Group(inner) => self.place(inner),
			View::Index { target, index } if self.analysis.key_reads.contains(&node) => {
				self.key_place(node, target, index)
			}
			View::Index { target, index } => match self.tuple_place(node, target, index)? {
				Some(slot) => Ok(slot),
				None if matches!(self.node_repr(target)?, Repr::Map(_)) => {
					let map = self.expr(target)?;
					let key = self.expr(index)?;
					self.table_place(node, map, key)
				}
				None => self.index_place(node, target, index),
			},
			View::Access { target, members } => {
				let list: Vec<NodeId> = members.iter().collect();
				if let Some(slot) = self.namespace_place(node, target, &list)? {
					return Ok(slot);
				}
				let slot = self.place(target)?;
				self.place_members(slot, self.type_of(target), members.iter())
			}
			// a guard computed by the statement (`(await mutex.lock()).value`):
			// a temporary, so the lock is released at the end of the statement
			_ if matches!(self.node_repr(node), Ok(Repr::Sync(ty))
				if matches!(self.types().kind(ty), fa_analyzer::types::TypeKind::Sync(fa_analyzer::types::SyncKind::Guard, _))) =>
			{
				let value = self.expr(node)?;
				let Some(raw) = value.value else {
					return Err(self.unsupported(node.index(), "this place"));
				};
				let slot = self.alloca(value.repr, "guard")?;
				self.builder.build_store(slot.pointer, raw)?;
				Ok(slot)
			}
			_ => Err(self.unsupported(node.index(), "this place")),
		}
	}

	/// The place of fields below a place (`.a.b`).
	pub(crate) fn place_members(
		&mut self,
		mut slot: Slot<'ctx>,
		mut current_type: TypeId,
		members: impl Iterator<Item = NodeId>,
	) -> Result<Slot<'ctx>, Error> {
		for member in members {
			// `guard.value`: the value of the locked mutex (parallel.rs)
			if let Resolution::Builtin(builtin) = self.analysis.resolution(member)
				&& fa_analyzer::builtins::get(builtin).path == "Guard#value"
				&& let Repr::Sync(guard_type) = slot.repr
			{
				let guard = self
					.builder
					.build_load(self.pointer_type(), slot.pointer, "")?
					.into_pointer_value();
				slot = self.guard_value_slot(guard, guard_type, member)?;
				if let fa_analyzer::types::TypeKind::Sync(_, item) = self.types().kind(guard_type) {
					current_type = item;
				}
				continue;
			}
			// `pointer.value`: the target's storage. C scalars and CStructs
			// use the same layout; a C function pointer differs from a Fa
			// function pair and cannot be borrowed as one.
			if let Resolution::Builtin(builtin) = self.analysis.resolution(member)
				&& fa_analyzer::builtins::get(builtin).path == "Pointer#value"
				&& slot.repr == Repr::Pointer
				&& let Some(target) = self.types().pointer_target(current_type)
			{
				let repr = self.repr_of(target, member.index())?;
				if let Ok(c) = self.c_type(target, member)
					&& self.c_llvm_type(c) != Some(self.llvm_type(repr))
				{
					return Err(self.unsupported(member.index(), "a view of this C value"));
				}
				let pointer = self
					.builder
					.build_load(self.pointer_type(), slot.pointer, "")?
					.into_pointer_value();
				slot = Slot { pointer, repr };
				current_type = target;
				continue;
			}
			let Resolution::Field(..) = self.analysis.resolution(member) else {
				return Err(self.unsupported(member.index(), "this place"));
			};
			if matches!(slot.repr, Repr::Union | Repr::InlineUnion(_)) {
				let pointer = if matches!(slot.repr, Repr::InlineUnion(_)) {
					slot.pointer
				} else {
					self.builder
						.build_load(self.pointer_type(), slot.pointer, "")?
						.into_pointer_value()
				};
				slot = self.box_data_pointer(pointer, current_type, member)?;
			}
			if let Repr::Rec(target) = slot.repr {
				let pointer = self.load_edge(slot)?;
				slot = Slot {
					pointer,
					repr: self.repr_of(target, member.index())?,
				};
			}
			// a field of an element of a columnar: in its column
			if let Repr::Row(element) = slot.repr {
				let index = self.concrete_field_index(element, member)?;
				let field_type = self.types().fields_of(element)[index as usize].ty;
				slot = Slot {
					pointer: self.row_field(
						&self.builder,
						slot.pointer,
						element,
						index as usize,
					)?,
					repr: self.field_repr(element, index as usize, member.index())?,
				};
				current_type = field_type;
				continue;
			}
			let Repr::Object(object) = slot.repr else {
				return Err(self.unsupported(member.index(), "this place"));
			};
			let index = self.concrete_field_index(object, member)?;
			let field_type = self.types().fields_of(object)[index as usize].ty;
			let structure = self.object_type(object);
			let pointer = self
				.builder
				.build_struct_gep(structure, slot.pointer, index, "")?;
			slot = Slot {
				pointer,
				repr: self.field_repr(object, index as usize, member.index())?,
			};
			current_type = field_type;
			// a narrowed field designates its payload (patterns.rs)
			if self.flags(member).contains(NodeFlags::NARROWED) {
				current_type = self.type_of(member);
				let narrowed = self.repr_of(current_type, member.index())?;
				slot = self.payload_place(slot, narrowed, member)?;
			}
		}
		Ok(slot)
	}

	/// `target .= { field = value, … }`.
	pub(crate) fn update_fields(
		&mut self,
		target: NodeId,
		value: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let View::Object { items } = self.view(value) else {
			return Err(self.unsupported(value.index(), "this update"));
		};
		let slot = self.place(target)?;
		let Repr::Object(object) = slot.repr else {
			return Err(self.unsupported(target.index(), "this update"));
		};
		let structure = self.object_type(object);
		let mut updates = Vec::new();
		for item in items.iter() {
			let (name, field_value) = match self.view(item) {
				View::Assignment {
					target: name,
					value: Some(field_value),
					..
				} => (name, field_value),
				View::Name(_) => (item, item),
				_ => return Err(self.unsupported(item.index(), "this field update")),
			};
			let View::Name(name) = self.view(name) else {
				return Err(self.unsupported(item.index(), "this field update"));
			};
			let text = self.tree.name(name).to_string();
			let Some(index) = self
				.types()
				.fields_of(object)
				.iter()
				.position(|field| self.types().syms.resolve(field.name) == text)
			else {
				return Err(self.unsupported(item.index(), "this field update"));
			};
			let repr = self.field_repr(object, index, item.index())?;
			let lowered = self.expr(field_value)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let lowered = self.owned_as(lowered, repr, Some(field_value))?;
			updates.push((index, repr, lowered, field_value));
		}
		// every value is read before the first field changes
		for (index, repr, lowered, field_value) in updates {
			let pointer =
				self.builder
					.build_struct_gep(structure, slot.pointer, index as u32, "")?;
			let field = Slot { pointer, repr };
			let old = self.load(field)?;
			self.emit_drop(old)?;
			self.store(field, lowered, field_value)?;
		}
		Ok(Val::void())
	}

	// ── printing unions ─────────────────────────────────────────────────

	/// Defines `fa.drop_box` and `fa.copy_box` for every member id the program
	/// uses (`ownership.rs`): the layout of its data, none for a constant box.
	pub(crate) fn define_boxes(&mut self) -> Result<(), Error> {
		let mut cases = Vec::new();
		let mut members: Vec<((NominalId, u16), u64)> = self
			.union_ids
			.borrow()
			.iter()
			.map(|(key, id)| (*key, *id))
			.collect();
		members.sort_by_key(|(_, id)| *id);
		members.dedup_by_key(|(_, id)| *id);
		for ((nominal, index), id) in members {
			let fields: Vec<Repr> = match self.builtin_error_fields(nominal) {
				Some(fields) => fields
					.iter()
					.map(|(_, ty)| self.repr_of(*ty, 0))
					.collect::<Result<_, _>>()?,
				None => match self.member_data(nominal, index) {
					Some(data) => vec![self.repr_of(data, 0)?],
					None => Vec::new(),
				},
			};
			let fields: Vec<Repr> = fields.into_iter().filter(|repr| repr.has_value()).collect();
			if fields.is_empty() {
				// a constant box
				cases.push((id, None, fields));
				continue;
			}
			let mut types: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i64_type().into()];
			types.extend(fields.iter().map(|repr| self.llvm_type(*repr)));
			let layout = self.context.struct_type(&types, false);
			cases.push((id, Some(layout), fields));
		}
		let mut types: Vec<(TypeId, u64)> = self
			.type_ids
			.borrow()
			.iter()
			.map(|(ty, id)| (*ty, *id))
			.collect();
		types.sort_by_key(|(_, id)| *id);
		for (ty, id) in types {
			let repr = self.repr_of(ty, 0)?;
			// `none` in a union: an id without data
			if repr.has_value() {
				cases.push((id, Some(self.box_type(Some(repr))), vec![repr]));
			} else {
				cases.push((id, None, Vec::new()));
			}
		}
		self.define_box_functions(&cases)
	}

	/// `fa.equal_box(ptr a, ptr b) -> i32`: `==` of two values of unions
	/// (boxes), defined last ([`Lowering::define_equal_box`]).
	pub(crate) fn box_equal_function(&self) -> inkwell::values::FunctionValue<'ctx> {
		self.module.get_function("fa.equal_box").unwrap_or_else(|| {
			let pointer = self.pointer_type();
			let r#type = self
				.context
				.i32_type()
				.fn_type(&[pointer.into(), pointer.into()], false);
			let function = self.module.add_function(
				"fa.equal_box",
				r#type,
				Some(inkwell::module::Linkage::Internal),
			);
			self.runtime.add_attributes(function, &["nounwind"]);
			function
		})
	}

	/// `a == b` on values of unions: the same member, and equal data (like
	/// `faEquals` on JavaScript). A value of a member type is boxed first.
	pub(crate) fn unions_equal(
		&mut self,
		left: Val<'ctx>,
		right: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let union = |repr: Repr| matches!(repr, Repr::Union | Repr::InlineUnion(_));
		let (left, right) = if union(left.repr) {
			(left, right)
		} else {
			(right, left)
		};
		let right = if union(right.repr) {
			right
		} else {
			self.convert(right, left.repr, node.index())?
		};
		let left = self.union_view(left)?;
		let right = self.union_view(right)?;
		let function = self.box_equal_function();
		let equal = self
			.builder
			.build_call(
				function,
				&[
					left.value.expect("a box").into(),
					right.value.expect("a box").into(),
				],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("equality")
			.into_int_value();
		Ok(self.builder.build_int_compare(
			IntPredicate::NE,
			equal,
			self.context.i32_type().const_zero(),
			"",
		)?)
	}

	/// Defines `fa.equal_box` when a comparison declared it: the same box,
	/// or the same member id and equal data (a switch on the id).
	pub(crate) fn define_equal_box(&mut self) -> Result<(), Error> {
		// per unit, always: the entry object's switch calls each unit's
		let function = match self.module.get_function("fa.equal_box") {
			Some(function) => function,
			None if self.split.is_some() => self.box_equal_function(),
			None => return Ok(()),
		};
		if function.count_basic_blocks() > 0 {
			return Ok(());
		}
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, Repr::Void, None);
		let i32 = self.context.i32_type();
		let i64 = self.context.i64_type();
		let a = function.get_nth_param(0).expect("a").into_pointer_value();
		let b = function.get_nth_param(1).expect("b").into_pointer_value();
		let yes = self.append_block("equal_boxes");
		let no = self.append_block("different_boxes");
		let same = self.builder.build_int_compare(IntPredicate::EQ, a, b, "")?;
		let next = self.append_block("other_boxes");
		self.builder.build_conditional_branch(same, yes, next)?;
		self.builder.position_at_end(next);
		let a_null = self.builder.build_is_null(a, "")?;
		let b_null = self.builder.build_is_null(b, "")?;
		let null = self.builder.build_or(a_null, b_null, "")?;
		let next = self.append_block("present_boxes");
		self.builder.build_conditional_branch(null, no, next)?;
		self.builder.position_at_end(next);
		let a_id = self.union_id(&self.builder, a)?;
		let b_id = self.union_id(&self.builder, b)?;
		let same_id = self
			.builder
			.build_int_compare(IntPredicate::EQ, a_id, b_id, "")?;
		let switch_block = self.append_block("same_member");
		self.builder
			.build_conditional_branch(same_id, switch_block, no)?;
		// the members with data: their layout, the representation of each part
		let mut cases: Vec<(u64, StructType<'ctx>, Vec<Repr>)> = Vec::new();
		let mut members: Vec<((NominalId, u16), u64)> = self
			.union_ids
			.borrow()
			.iter()
			.map(|(key, id)| (*key, *id))
			.collect();
		members.sort_by_key(|(_, id)| *id);
		members.dedup_by_key(|(_, id)| *id);
		for ((nominal, index), id) in members {
			let fields: Vec<Repr> = match self.builtin_error_fields(nominal) {
				Some(fields) => fields
					.iter()
					.map(|(_, ty)| self.repr_of(*ty, 0))
					.collect::<Result<_, _>>()?,
				None => match self.member_data(nominal, index) {
					Some(data) => vec![self.repr_of(data, 0)?],
					None => Vec::new(),
				},
			};
			let fields: Vec<Repr> = fields.into_iter().filter(|repr| repr.has_value()).collect();
			if fields.is_empty() {
				continue;
			}
			let mut types: Vec<BasicTypeEnum<'ctx>> = vec![i64.into()];
			types.extend(fields.iter().map(|repr| self.llvm_type(*repr)));
			cases.push((id, self.context.struct_type(&types, false), fields));
		}
		let mut types: Vec<(TypeId, u64)> = self
			.type_ids
			.borrow()
			.iter()
			.map(|(ty, id)| (*ty, *id))
			.collect();
		types.sort_by_key(|(_, id)| *id);
		for (ty, id) in types {
			let repr = self.repr_of(ty, 0)?;
			if repr.has_value() {
				cases.push((id, self.box_type(Some(repr)), vec![repr]));
			}
		}
		let node = NodeId::new(0);
		let mut blocks = Vec::with_capacity(cases.len());
		for (id, layout, fields) in cases {
			let block = self.append_block("compare_member");
			self.builder.position_at_end(block);
			let mut equal = self.context.bool_type().const_int(1, false);
			for (position, repr) in fields.into_iter().enumerate() {
				let index = position as u32 + 1;
				let (left, right) = if let Some(plan) = self.compact_array_box(id) {
					(
						self.compact_payload_value(&self.builder, a, plan)?,
						self.compact_payload_value(&self.builder, b, plan)?,
					)
				} else if let Some(data) = self.compact_string_box(id) {
					(
						self.string_payload_value(&self.builder, a, data)?,
						self.string_payload_value(&self.builder, b, data)?,
					)
				} else if let Some(data) = self.compact_scalar_box(id) {
					(
						self.scalar_payload_value(&self.builder, a, data)?,
						self.scalar_payload_value(&self.builder, b, data)?,
					)
				} else if let Some(plan) = self.compact_edge_box(id) {
					(
						self.edge_payload_value(&self.builder, a, plan)?,
						self.edge_payload_value(&self.builder, b, plan)?,
					)
				} else {
					let left = self.builder.build_struct_gep(layout, a, index, "")?;
					let right = self.builder.build_struct_gep(layout, b, index, "")?;
					(
						self.builder.build_load(self.llvm_type(repr), left, "")?,
						self.builder.build_load(self.llvm_type(repr), right, "")?,
					)
				};
				let mark = self.mark();
				let part =
					self.values_equal_any(Val::new(repr, left), Val::new(repr, right), node)?;
				self.end_scope(mark, None)?;
				equal = self.builder.build_and(equal, part, "")?;
			}
			let result = self.builder.build_int_z_extend(equal, i32, "")?;
			self.builder.build_return(Some(&result))?;
			blocks.push((i64.const_int(id, false), block));
		}
		self.builder.position_at_end(switch_block);
		// a member without data: the same id is the same value
		self.builder.build_switch(a_id, yes, &blocks)?;
		self.builder.position_at_end(yes);
		self.builder.build_return(Some(&i32.const_int(1, false)))?;
		self.builder.position_at_end(no);
		self.builder.build_return(Some(&i32.const_zero()))?;
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		Ok(())
	}

	/// Defines `fa_format_union(box, nested)`, which printing the values of
	/// unions call: a switch on the id of the member.
	pub(crate) fn define_format_union(&mut self) -> Result<(), Error> {
		// the built-in errors, which the runtime raises (per unit: by the
		// entry's unit, and the units that build them)
		for error in ERRORS.iter().filter(|_| self.registers_builtin_errors()) {
			if let Some(id) = fa_analyzer::builtins::by_path(error.name)
				&& let Some(ty) = self.session.builtin_type(id)
				&& let TypeKind::NamedUnion(nominal, _) = self.types().kind(ty)
			{
				self.member_id(nominal, 0);
			}
		}
		let pointer = self.pointer_type();
		let function = match self.module.get_function("fa_format_union") {
			Some(function) => function,
			None => self.module.add_function(
				"fa_format_union",
				self.context
					.void_type()
					.fn_type(&[pointer.into(), self.context.i32_type().into()], false),
				None,
			),
		};
		self.begin_function(function, Repr::Void, None);
		let value = Val::new(Repr::Union, function.get_nth_param(0).expect("a box"));
		let id = self.box_id(value)?;
		let end = self.append_block("end_format");
		let mut members: Vec<((NominalId, u16), u64)> = self
			.union_ids
			.borrow()
			.iter()
			.map(|(key, id)| (*key, *id))
			.collect();
		members.sort_by_key(|(_, id)| *id);
		members.dedup_by_key(|(_, id)| *id);
		let mut cases = Vec::new();
		let switch_block = self.builder.get_insert_block().expect("a block");
		let node = NodeId::new(0);
		for ((nominal, index), member_id) in members {
			let block = self.append_block("format_member");
			self.builder.position_at_end(block);
			let types = self.types();
			let info = types.nominal(nominal);
			let type_name = types.decl_name(info.key).to_string();
			let member_name = types
				.sym_list(info.members)
				.get(index as usize)
				.map(|&sym| types.syms.resolve(sym).to_string())
				.unwrap_or_default();
			let name = if self.builtin_error_id(nominal).is_some() && type_name != "CapacityError" {
				type_name
			} else {
				format!("{type_name}.{member_name}")
			};
			let name = self.string_constant(&name);
			self.append_value(name, node)?;
			let builtin_fields = self
				.builtin_error_id(nominal)
				.map(|id| ERRORS[id as usize - 1].fields)
				.filter(|fields| !fields.is_empty());
			if let Some(fields) = builtin_fields {
				// Storage is sorted by name; output follows declaration order.
				let layout = self.builtin_error_fields(nominal).expect("a builtin error");
				let reprs: Vec<Repr> = layout
					.iter()
					.map(|(_, ty)| self.repr_of(*ty, 0))
					.collect::<Result<_, _>>()?;
				let mut types: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i64_type().into()];
				types.extend(reprs.iter().map(|repr| self.llvm_type(*repr)));
				let ty = self.context.struct_type(&types, false);
				let box_pointer = value.value.expect("a box").into_pointer_value();
				for (position, (field, _)) in fields.iter().enumerate() {
					let index = layout
						.iter()
						.position(|(name, _)| name == field)
						.expect("a payload field");
					let repr = reprs[index];
					let prefix = if position == 0 {
						format!(" {{ {field} = ")
					} else {
						format!(", {field} = ")
					};
					let prefix = self.string_constant(&prefix);
					self.append_value(prefix, node)?;
					let pointer =
						self.builder
							.build_struct_gep(ty, box_pointer, index as u32 + 1, "")?;
					let loaded = self.load(Slot { pointer, repr })?;
					self.append_nested(loaded, node)?;
				}
				let suffix = self.string_constant(" }");
				self.append_value(suffix, node)?;
			} else if let Some(data) = self.member_data(nominal, index) {
				let repr = self.repr_of(data, 0)?;
				let box_pointer = value.value.expect("a box").into_pointer_value();
				let loaded =
					self.member_payload_value(&self.builder, box_pointer, nominal, index, repr)?;
				let loaded = Val::new(repr, loaded);
				if let Repr::Object(object) = repr {
					let space = self.string_constant(" ");
					self.append_value(space, node)?;
					// Errors carry their declared field formatter; ordinary
					// union payloads use their canonical value representation.
					let fields = if self.types().nominal(nominal).kind
						== fa_analyzer::types::NominalKind::Errors
					{
						self.printed_fields(object)
					} else {
						self.types()
							.fields_of(object)
							.iter()
							.enumerate()
							.filter(|(_, field)| {
								!self.types().syms.resolve(field.name).starts_with('#')
							})
							.map(|(index, field)| {
								(
									self.types().syms.resolve(field.name).to_string(),
									index as u32,
									field.ty,
								)
							})
							.collect()
					};
					self.append_ordered_fields(loaded, fields, node)?;
				} else {
					let open = self.string_constant("(");
					self.append_value(open, node)?;
					self.append_nested(loaded, node)?;
					let close = self.string_constant(")");
					self.append_value(close, node)?;
				}
			}
			self.builder.build_unconditional_branch(end)?;
			cases.push((self.context.i64_type().const_int(member_id, false), block));
		}
		// the member types of anonymous unions: their value, quoted when nested
		let nested = function.get_nth_param(1).expect("a flag").into_int_value();
		let mut types: Vec<(TypeId, u64)> = self
			.type_ids
			.borrow()
			.iter()
			.map(|(ty, id)| (*ty, *id))
			.collect();
		types.sort_by_key(|(_, id)| *id);
		for (ty, type_id) in types {
			let block = self.append_block("format_type");
			self.builder.position_at_end(block);
			let repr = self.repr_of(ty, 0)?;
			// `none` boxed in a union has no data
			let data = if repr.has_value() {
				self.unbox(value, repr)?
			} else {
				Val {
					destination: None,
					repr,
					value: None,
				}
			};
			if repr == Repr::Str {
				let quoted = self.append_block("format_quoted");
				let raw = self.append_block("format_raw");
				let is_nested = self.builder.build_int_compare(
					IntPredicate::NE,
					nested,
					nested.get_type().const_zero(),
					"",
				)?;
				self.builder
					.build_conditional_branch(is_nested, quoted, raw)?;
				self.builder.position_at_end(quoted);
				self.append_nested(data, node)?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(raw);
				self.append_value(data, node)?;
			} else {
				self.append_nested(data, node)?;
			}
			self.builder.build_unconditional_branch(end)?;
			cases.push((self.context.i64_type().const_int(type_id, false), block));
		}
		self.builder.position_at_end(switch_block);
		self.builder.build_switch(id, end, &cases)?;
		self.builder.position_at_end(end);
		self.builder.build_return(None)?;
		self.end_function()
	}
}

#[cfg(test)]
mod id_tests {
	use super::{FIRST_MEMBER_ID, FIRST_TYPE_ID, unmarked_id};

	#[test]
	fn ordinary_union_ids_stay_disjoint_from_compact_tags() {
		let counters = [
			0,
			FIRST_MEMBER_ID,
			32766,
			32767,
			32768,
			32769,
			65535,
			65536,
			1 << 32,
		];
		let mut previous = None;
		for counter in counters {
			let id = unmarked_id(counter);
			assert_eq!(id & 0x8000, 0);
			assert_eq!((FIRST_TYPE_ID + id) & 0x8000, 0);
			if let Some(previous) = previous {
				assert!(id > previous);
			}
			previous = Some(id);
		}
	}
}
