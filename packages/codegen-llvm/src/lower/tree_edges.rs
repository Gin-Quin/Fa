//! Immutable direct Root fields use the same 32-bit links as child arrays.
//! Function values and reconstructed payloads retain their ordinary ABI.

use fa_analyzer::ids::{NominalId, TypeId};
use fa_analyzer::types::{NominalKind, TypeKind};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use super::Lowering;
use super::values::Repr;
use crate::error::Error;

#[derive(Clone, Copy)]
pub(super) struct EdgePayload {
	pub(super) data: TypeId,
	owner: NominalId,
	fields: u32,
}

#[derive(Clone, Copy)]
struct EdgeValue<'ctx> {
	value: BasicValueEnum<'ctx>,
	link: Option<IntValue<'ctx>>,
}

enum EdgeWidth {
	Fixed32,
	Near16,
	Far32,
}

impl<'ctx> Lowering<'_, 'ctx> {
	fn is_root_link(&self, owner: NominalId, ty: TypeId) -> bool {
		let unfolded = self.types().unfold(ty);
		matches!(self.types().kind(unfolded),
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) if nominal == owner)
			&& self.repr_of(ty, 0).is_ok_and(|repr| repr == Repr::Union)
	}

	fn edge_scalar(&self, ty: TypeId) -> bool {
		self.repr_of(ty, 0).is_ok_and(|repr| {
			matches!(
				repr,
				Repr::Int(_)
					| Repr::Bool | Repr::F32
					| Repr::F64 | Repr::Enum(_)
					| Repr::Percentage
			)
		})
	}

	/// Layout eligibility alone, also used before compressed links are enabled.
	/// A mixed payload keeps its ordinary layout until every field has a plan.
	pub(super) fn edge_payload_shape(&self, owner: NominalId, index: u16) -> Option<EdgePayload> {
		if !self.arena_member(owner) || self.types().nominal(owner).kind != NominalKind::Union {
			return None;
		}
		let data = self.member_data(owner, index)?;
		let fields = self.types().fields_of(data);
		if !fields
			.iter()
			.any(|field| self.is_root_link(owner, field.ty))
			|| !fields
				.iter()
				.all(|field| self.is_root_link(owner, field.ty) || self.edge_scalar(field.ty))
		{
			return None;
		}
		Some(EdgePayload {
			data,
			owner,
			fields: fields.len().try_into().ok()?,
		})
	}

	/// Root links occupy i32 slots; ordinary scalar fields keep their ABI
	/// alignment. Let LLVM lay out padding so an i128 or Float is never
	/// loaded through an under-aligned packed field.
	fn edge_layout(&self, plan: EdgePayload, compact: bool) -> BasicTypeEnum<'ctx> {
		let link = if compact {
			self.context.i16_type()
		} else {
			self.context.i32_type()
		};
		let mut fields = vec![link.as_basic_type_enum()];
		for field in self.types().fields_of(plan.data) {
			fields.push(if self.is_root_link(plan.owner, field.ty) {
				link.into()
			} else {
				self.llvm_type(self.repr_of(field.ty, 0).expect("a proven scalar"))
			});
		}
		if fields
			.iter()
			.all(|field| *field == link.as_basic_type_enum())
		{
			link.array_type(plan.fields + 1).into()
		} else {
			self.context.struct_type(&fields, false).into()
		}
	}

	fn near_edge_payload(&self, plan: EdgePayload) -> bool {
		// Relative mixed fields save bytes, but add work on the hot scalar
		// path. Keep their naturally aligned fixed-width layout; pure root
		// edges benefit from the smaller traversal working set.
		if !self.tree_arenas.near_links
			|| !self
				.types()
				.fields_of(plan.data)
				.iter()
				.all(|field| self.is_root_link(plan.owner, field.ty))
		{
			return false;
		}
		let data = self.target_data();
		data.get_abi_size(&self.edge_layout(plan, true))
			.next_multiple_of(8)
			< data
				.get_abi_size(&self.edge_layout(plan, false))
				.next_multiple_of(8)
	}

	fn edge_field_pointer(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		layout: BasicTypeEnum<'ctx>,
		index: u32,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(match layout {
			BasicTypeEnum::StructType(ty) => b.build_struct_gep(ty, pointer, index + 1, "")?,
			BasicTypeEnum::ArrayType(ty) => unsafe {
				b.build_in_bounds_gep(
					ty,
					pointer,
					&[
						self.usize.const_zero(),
						self.usize.const_int(u64::from(index + 1), false),
					],
					"",
				)?
			},
			_ => unreachable!("an edge payload is a struct or array"),
		})
	}

	pub(super) fn compact_edge_payload(&self, owner: NominalId, index: u16) -> Option<EdgePayload> {
		if !self.packed_headers() || !self.tree_arenas.compressed_unions.contains_key(&owner) {
			return None;
		}
		self.edge_payload_shape(owner, index)
	}

	pub(super) fn compact_edge_box(&self, id: u64) -> Option<EdgePayload> {
		let member = self
			.union_ids
			.borrow()
			.iter()
			.find_map(|(&member, &candidate)| (candidate == id).then_some(member))?;
		self.compact_edge_payload(member.0, member.1)
	}

	pub(super) fn compact_edge_data(&self, data: TypeId) -> Vec<(u64, EdgePayload)> {
		let mut members = Vec::new();
		for nominal in self.tree_arenas.unions.iter().copied() {
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len();
			for index in 0..count as u16 {
				if let Some(plan) = self.compact_edge_payload(nominal, index)
					&& plan.data == data
				{
					members.push((self.member_id(nominal, index), plan));
				}
			}
		}
		members.sort_by_key(|(id, _)| *id);
		members
	}

	pub(super) fn edge_payload_value(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		plan: EdgePayload,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if !self.near_edge_payload(plan) {
			return self.read_edge_payload(b, pointer, plan, false);
		}
		let tag = b
			.build_load(self.context.i16_type(), pointer, "edge_tag")?
			.into_int_value();
		let flag = b.build_and(tag, self.context.i16_type().const_int(0x4000, false), "")?;
		let wide = b.build_int_compare(
			IntPredicate::NE,
			flag,
			self.context.i16_type().const_zero(),
			"",
		)?;
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let narrow = self.context.append_basic_block(function, "near_edges");
		let far = self.context.append_basic_block(function, "far_edges");
		let end = self.context.append_basic_block(function, "edge_value");
		b.build_conditional_branch(wide, far, narrow)?;
		b.position_at_end(narrow);
		let small = self.read_edge_payload(b, pointer, plan, true)?;
		let small_end = b.get_insert_block().expect("a block");
		b.build_unconditional_branch(end)?;
		b.position_at_end(far);
		let large = self.read_edge_payload(b, pointer, plan, false)?;
		let large_end = b.get_insert_block().expect("a block");
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		let result = b.build_phi(
			self.object_type(plan.data).as_basic_type_enum(),
			"edge_payload",
		)?;
		result.add_incoming(&[(&small, small_end), (&large, large_end)]);
		Ok(result.as_basic_value())
	}

	fn read_edge_payload(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		plan: EdgePayload,
		compact: bool,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let mut value = self.object_type(plan.data).const_zero();
		let layout = self.edge_layout(plan, compact);
		for (index, field) in self.types().fields_of(plan.data).iter().enumerate() {
			let at = self.edge_field_pointer(b, pointer, layout, index as u32)?;
			let child = if self.is_root_link(plan.owner, field.ty) {
				let link = b
					.build_load(
						if compact {
							self.context.i16_type()
						} else {
							self.context.i32_type()
						},
						at,
						"",
					)?
					.into_int_value();
				if compact {
					self.decode_near_link(b, plan.owner, pointer, link)?.into()
				} else {
					self.decode_union_link(b, plan.owner, link)?.into()
				}
			} else {
				b.build_load(self.llvm_type(self.repr_of(field.ty, 0)?), at, "")?
			};
			value = b
				.build_insert_value(value, child, index as u32, "")?
				.into_struct_value();
		}
		Ok(value.into())
	}

	pub(super) fn freeze_edge_payload(
		&self,
		b: &Builder<'ctx>,
		id: u64,
		plan: EdgePayload,
		value: BasicValueEnum<'ctx>,
		copy: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let mut children = Vec::with_capacity(plan.fields as usize);
		let mut roots = Vec::new();
		for (index, field) in self.types().fields_of(plan.data).iter().enumerate() {
			let child = b.build_extract_value(value.into_struct_value(), index as u32, "")?;
			let (child, link) = if self.is_root_link(plan.owner, field.ty) {
				let child = if copy {
					self.call_copy(b, Repr::Union, child)?
				} else {
					child
				};
				let link = self
					.encode_union_link(b, plan.owner, child.into_pointer_value())?
					.into_int_value();
				roots.push((child.into_pointer_value(), link));
				(child, Some(link))
			} else {
				(child, None)
			};
			children.push(EdgeValue { value: child, link });
		}
		let layout = self.edge_layout(plan, false);
		if !self.near_edge_payload(plan) {
			let node = self.allocate_tree_type(b, layout)?;
			self.write_edge_payload(b, node, id, plan, EdgeWidth::Fixed32, &children)?;
			return Ok(node);
		}
		let (node, wide) =
			self.allocate_near_edges(b, plan.owner, self.edge_layout(plan, true), layout, &roots)?;
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let narrow = self
			.context
			.append_basic_block(function, "freeze_near_edges");
		let far = self
			.context
			.append_basic_block(function, "freeze_far_edges");
		let end = self.context.append_basic_block(function, "frozen_edges");
		b.build_conditional_branch(wide, far, narrow)?;
		b.position_at_end(narrow);
		self.write_edge_payload(b, node, id, plan, EdgeWidth::Near16, &children)?;
		b.build_unconditional_branch(end)?;
		b.position_at_end(far);
		self.write_edge_payload(b, node, id, plan, EdgeWidth::Far32, &children)?;
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		Ok(node)
	}

	fn write_edge_payload(
		&self,
		b: &Builder<'ctx>,
		node: PointerValue<'ctx>,
		id: u64,
		plan: EdgePayload,
		width: EdgeWidth,
		children: &[EdgeValue<'ctx>],
	) -> Result<(), Error> {
		let compact = matches!(width, EdgeWidth::Near16);
		let layout = self.edge_layout(plan, compact);
		if self.target_data().get_abi_size(&layout) < 8 {
			b.build_store(node, self.context.i64_type().const_zero())?;
		}
		b.build_store(node, super::zero_of(layout))?;
		self.store_compact_id(b, node, id)?;
		if matches!(width, EdgeWidth::Far32) {
			b.build_store(node, self.context.i16_type().const_int(id | 0xc000, false))?;
		}
		for (index, child) in children.iter().enumerate() {
			let value = child.value;
			let child = if let Some(link) = child.link {
				if compact {
					self.encode_near_link(b, plan.owner, node, value.into_pointer_value(), link)?
						.into()
				} else {
					link.into()
				}
			} else {
				value
			};
			b.build_store(
				self.edge_field_pointer(b, node, layout, index as u32)?,
				child,
			)?;
		}
		Ok(())
	}
}
