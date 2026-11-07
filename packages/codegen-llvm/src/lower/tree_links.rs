//! Collection storage for arena-owned named unions. Their values remain
//! ordinary pointers at function boundaries; array slots reuse Rec's i32
//! compressed links. Constant members live outside the arena range and use
//! small nonzero codes, keeping zero available for an absent optional.

use fa_analyzer::ids::{NominalId, TypeId};
use fa_analyzer::types::TypeKind;
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use super::Lowering;
use super::values::{Repr, Scalar};
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn compressed_union_item(&self, item: TypeId) -> Option<TypeId> {
		let item = self.types().rec_target(item).unwrap_or(item);
		let nominal = match self.types().kind(item) {
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => nominal,
			_ => return None,
		};
		self.tree_arenas.compressed_unions.get(&nominal).copied()
	}

	pub(super) fn compressed_union_link(&self, repr: Repr) -> Option<NominalId> {
		let target = match repr {
			Repr::Rec(target) | Repr::Optional(Scalar::Rec(target)) => target,
			_ => return None,
		};
		match self.types().kind(target) {
			TypeKind::NamedUnion(nominal, _)
				if self.tree_arenas.compressed_unions.get(&nominal) == Some(&target) =>
			{
				Some(nominal)
			}
			_ => None,
		}
	}

	fn union_constants(&self, nominal: NominalId) -> Vec<u16> {
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len();
		(0..count as u16)
			.filter(|&index| self.member_data(nominal, index).is_none())
			.collect()
	}

	pub(super) fn decode_union_link(
		&self,
		b: &Builder<'ctx>,
		nominal: NominalId,
		link: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let wide = b.build_int_z_extend(link, i64, "")?;
		let address = b.build_int_nuw_mul(wide, i64.const_int(8, false), "")?;
		let mut pointer = b.build_int_to_ptr(address, self.pointer_type(), "node")?;
		for index in self.union_constants(nominal) {
			let id = i64.const_int(self.member_id(nominal, index), false);
			let ty = self.box_type(None);
			let global = self.module.add_global(ty, None, "fa.arena.constant_member");
			global.set_initializer(&ty.const_named_struct(&[id.into()]));
			global.set_constant(true);
			global.set_linkage(inkwell::module::Linkage::Private);
			let constant = b.build_int_compare(
				IntPredicate::EQ,
				link,
				link.get_type().const_int(u64::from(index) + 1, false),
				"",
			)?;
			pointer = b
				.build_select(constant, global.as_pointer_value(), pointer, "")?
				.into_pointer_value();
		}
		Ok(pointer)
	}

	pub(super) fn encode_union_link(
		&self,
		b: &Builder<'ctx>,
		nominal: NominalId,
		pointer: PointerValue<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let address = b.build_ptr_to_int(pointer, i64, "")?;
		let shifted = b.build_right_shift(address, i64.const_int(3, false), false, "")?;
		let link = b.build_int_truncate(shifted, i32, "link")?;
		let constants = self.union_constants(nominal);
		if constants.is_empty() {
			return Ok(link.into());
		}
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let absent = self.context.append_basic_block(function, "null_union_link");
		let present = self.context.append_basic_block(function, "union_link");
		let end = self
			.context
			.append_basic_block(function, "encoded_union_link");
		b.build_conditional_branch(b.build_is_not_null(pointer, "")?, present, absent)?;
		b.position_at_end(absent);
		b.build_unconditional_branch(end)?;
		b.position_at_end(present);
		let id = self.union_id(b, pointer)?;
		let mut encoded = link;
		for index in constants {
			let constant = b.build_int_compare(
				IntPredicate::EQ,
				id,
				i64.const_int(self.member_id(nominal, index), false),
				"",
			)?;
			encoded = b
				.build_select(
					constant,
					i32.const_int(u64::from(index) + 1, false),
					encoded,
					"",
				)?
				.into_int_value();
		}
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		let result = b.build_phi(i32, "union_link")?;
		result.add_incoming(&[(&i32.const_zero(), absent), (&encoded, present)]);
		Ok(result.as_basic_value())
	}
}
