//! Relative i16 links for private immutable nodes, with a local i32 fallback.
//! The common arena address domain and the owners retain their ordinary ABI.

use fa_analyzer::ids::NominalId;
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{IntValue, PointerValue};

use super::Lowering;
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn allocate_near_edges(
		&self,
		b: &Builder<'ctx>,
		nominal: NominalId,
		compact: BasicTypeEnum<'ctx>,
		wide: BasicTypeEnum<'ctx>,
		children: &[(PointerValue<'ctx>, IntValue<'ctx>)],
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		let constants = self
			.union_constants(nominal)
			.last()
			.map_or(0, |index| u64::from(*index) + 1);
		let mut first = self.usize.const_all_ones();
		let mut last = self.usize.const_zero();
		for &(pointer, link) in children {
			let address = b.build_ptr_to_int(pointer, self.usize, "")?;
			let constant = if constants == 0 {
				self.context.bool_type().const_zero()
			} else {
				b.build_int_compare(
					IntPredicate::ULE,
					link,
					link.get_type().const_int(constants, false),
					"",
				)?
			};
			let low = b
				.build_select(constant, self.usize.const_all_ones(), address, "")?
				.into_int_value();
			let high = b
				.build_select(constant, self.usize.const_zero(), address, "")?
				.into_int_value();
			first = b
				.build_select(
					b.build_int_compare(IntPredicate::ULT, low, first, "")?,
					low,
					first,
					"",
				)?
				.into_int_value();
			last = b
				.build_select(
					b.build_int_compare(IntPredicate::UGT, high, last, "")?,
					high,
					last,
					"",
				)?
				.into_int_value();
		}
		let data = self.target_data();
		let allocation = b
			.build_call(
				self.runtime.function("fa_arena_alloc_near")?,
				&[
					self.usize
						.const_int(data.get_abi_size(&compact), false)
						.into(),
					self.usize.const_int(data.get_abi_size(&wide), false).into(),
					self.usize
						.const_int(u64::from(data.get_abi_alignment(&wide)), false)
						.into(),
					first.into(),
					last.into(),
					self.usize.const_int(constants, false).into(),
				],
				"edge_allocation",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an allocation")
			.into_pointer_value();
		let tag = b
			.build_load(self.context.i16_type(), allocation, "")?
			.into_int_value();
		let flag = b.build_and(tag, self.context.i16_type().const_int(0x4000, false), "")?;
		let wide = b.build_int_compare(
			IntPredicate::NE,
			flag,
			self.context.i16_type().const_zero(),
			"wide_edges",
		)?;
		Ok((allocation, wide))
	}

	pub(super) fn encode_near_link(
		&self,
		b: &Builder<'ctx>,
		nominal: NominalId,
		node: PointerValue<'ctx>,
		child: PointerValue<'ctx>,
		link32: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let node = b.build_ptr_to_int(node, self.usize, "")?;
		let child = b.build_ptr_to_int(child, self.usize, "")?;
		let delta = b.build_int_sub(child, node, "")?;
		let scaled = b.build_right_shift(delta, self.usize.const_int(3, false), true, "")?;
		let link = b.build_int_truncate(scaled, self.context.i16_type(), "near_link")?;
		let constants = self
			.union_constants(nominal)
			.last()
			.map_or(0, |index| u64::from(*index) + 1);
		if constants == 0 {
			return Ok(link);
		}
		let constant = b.build_int_compare(
			IntPredicate::ULE,
			link32,
			link32.get_type().const_int(constants, false),
			"",
		)?;
		let code = b.build_int_truncate(link32, self.context.i16_type(), "")?;
		Ok(b.build_select(constant, code, link, "")?.into_int_value())
	}

	pub(super) fn decode_near_link(
		&self,
		b: &Builder<'ctx>,
		nominal: NominalId,
		node: PointerValue<'ctx>,
		link: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let delta = b.build_int_s_extend(link, self.usize, "")?;
		let bytes = b.build_left_shift(delta, self.usize.const_int(3, false), "")?;
		let base = b.build_ptr_to_int(node, self.usize, "")?;
		let address = b.build_int_add(base, bytes, "")?;
		let mut pointer = b.build_int_to_ptr(address, self.pointer_type(), "near_child")?;
		for index in self.union_constants(nominal) {
			let ty = self.box_type(None);
			let global = self.module.add_global(ty, None, "fa.arena.constant_member");
			global.set_initializer(
				&ty.const_named_struct(&[self
					.context
					.i64_type()
					.const_int(self.member_id(nominal, index), false)
					.into()]),
			);
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
}
