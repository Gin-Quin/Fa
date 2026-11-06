//! Marked compact headers of proven root payloads in a whole module.
//! Normal unions, errors and ABI boundaries keep their full u64 ids.
//! The preparation guard bounds all named ids and anonymous counters, so
//! their low 16 bits never contain the compact marker (bit 15).

use fa_analyzer::ids::{NominalId, TypeId};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use super::Lowering;
use super::values::Repr;
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn packed_headers(&self) -> bool {
		self.tree_arenas.packed_headers
	}

	pub(super) fn payload_offset(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		bytes: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(unsafe {
			b.build_in_bounds_gep(
				self.context.i8_type(),
				pointer,
				&[self.usize.const_int(bytes, false)],
				"",
			)?
		})
	}

	pub(super) fn union_id(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let word = b.build_load(i64, pointer, "id")?.into_int_value();
		if !self.packed_headers() {
			return Ok(word);
		}
		let marker = b.build_and(word, i64.const_int(0x8000, false), "")?;
		let compact = b.build_int_compare(IntPredicate::NE, marker, i64.const_zero(), "")?;
		let id = b.build_and(word, i64.const_int(0x7fff, false), "")?;
		Ok(b.build_select(compact, id, word, "id")?.into_int_value())
	}

	pub(super) fn store_compact_id(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		id: u64,
	) -> Result<(), Error> {
		assert!(id < 32768, "compact member id exceeds its prepared bound");
		b.build_store(
			pointer,
			self.context.i16_type().const_int(id | 0x8000, false),
		)?;
		Ok(())
	}

	pub(super) fn compact_scalar_payload(&self, nominal: NominalId, index: u16) -> Option<TypeId> {
		if !self.packed_headers() || !self.arena_member(nominal) {
			return None;
		}
		let data = self.member_data(nominal, index)?;
		let fields = self.object_fields(data);
		let [field] = fields.as_slice() else {
			return None;
		};
		if self.needs_drop(*field) {
			return None;
		}
		let ty = self.llvm_type(Repr::Object(data));
		(self.target_data().get_abi_size(&ty) <= 4
			&& self.target_data().get_abi_alignment(&ty) <= 4)
			.then_some(data)
	}

	pub(super) fn compact_scalar_box(&self, id: u64) -> Option<TypeId> {
		let member = self
			.union_ids
			.borrow()
			.iter()
			.find_map(|(&member, &candidate)| (candidate == id).then_some(member))?;
		self.compact_scalar_payload(member.0, member.1)
	}

	pub(super) fn compact_scalar_data(&self, data: TypeId) -> Vec<u64> {
		let mut members = Vec::new();
		for nominal in self.tree_arenas.unions.iter().copied() {
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len();
			for index in 0..count as u16 {
				if self.compact_scalar_payload(nominal, index) == Some(data) {
					members.push(self.member_id(nominal, index));
				}
			}
		}
		members.sort_unstable();
		members
	}

	pub(super) fn scalar_payload_value(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		data: TypeId,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		Ok(b.build_load(
			self.object_type(data),
			self.payload_offset(b, pointer, 4)?,
			"",
		)?)
	}

	pub(super) fn freeze_scalar_payload(
		&self,
		b: &Builder<'ctx>,
		id: u64,
		value: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let node = self.allocate_tree_type(b, self.context.i64_type().into())?;
		b.build_store(node, self.context.i64_type().const_zero())?;
		self.store_compact_id(b, node, id)?;
		b.build_store(self.payload_offset(b, node, 4)?, value)?;
		Ok(node)
	}
}
