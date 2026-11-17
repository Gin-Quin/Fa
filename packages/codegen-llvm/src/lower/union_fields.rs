//! Common fields of anonymous unions dispatch on the actual payload layout.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::View;

use super::values::{Repr, Scalar, Val};
use super::{Lowering, Slot};
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	/// A field has no single shape resolution when its receiver is a union.
	fn union_field_layouts(&self, ty: TypeId, member: NodeId) -> Option<Vec<(TypeId, usize)>> {
		let items = self.union_items(self.concrete_type(ty))?;
		let View::Name(name) = self.view(member) else {
			return None;
		};
		let name = self.tree.name(name);
		items
			.into_iter()
			.map(|item| {
				self.types()
					.fields_of(item)
					.iter()
					.position(|field| self.types().syms.resolve(field.name) == name)
					.map(|index| (item, index))
			})
			.collect()
	}

	fn union_field_slot(
		&self,
		value: Val<'ctx>,
		ty: TypeId,
		index: usize,
		member: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let payload = self.box_data_pointer(
			value.value.expect("a union").into_pointer_value(),
			ty,
			member,
		)?;
		let Repr::Object(object) = payload.repr else {
			return Err(self.unsupported(member.index(), "this common union field"));
		};
		Ok(Slot {
			pointer: self.builder.build_struct_gep(
				self.object_type(object),
				payload.pointer,
				index as u32,
				"union_field",
			)?,
			repr: self.field_repr(object, index, member.index())?,
		})
	}

	fn union_field_type(&self, layouts: &[(TypeId, usize)]) -> TypeId {
		let fields: Vec<_> = layouts
			.iter()
			.map(|&(ty, index)| self.shared_type(self.types().fields_of(ty)[index].ty))
			.collect();
		let mut scratch = fa_analyzer::types::TypeStore::view(self.types().global());
		let joined = scratch.union(&fields);
		scratch.merger(None).ty(joined)
	}

	/// Equal representations keep the actual field address, so reads, writes
	/// and aliases use the same dispatch. Different representations merge an
	/// owned result after converting each selected field to the union type.
	pub(super) fn common_union_field(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		member: NodeId,
		place: bool,
	) -> Result<Option<(Slot<'ctx>, bool)>, Error> {
		let Some(layouts) = self.union_field_layouts(ty, member) else {
			return Ok(None);
		};
		// The member expression may already be narrowed. Locate storage using
		// its complete declared type; narrowing applies after loading it.
		let field_type = self.union_field_type(&layouts);
		let repr = self.repr_of(field_type, member.index())?;
		let same = layouts.iter().all(|&(ty, index)| {
			self.field_repr(ty, index, member.index())
				.is_ok_and(|field| field == repr || self.same_layout(field, repr))
		});
		if place && !same {
			return Err(self.unsupported(
				member.index(),
				"a writable union field with different layouts",
			));
		}
		let result = self.alloca(
			if same { Repr::Pointer } else { repr },
			"union_field_result",
		)?;
		let end = self.append_block("union_field_end");
		for (item, index) in layouts {
			let holds = self.box_is_typed(value, item, Some(ty))?;
			let selected = self.append_block("union_field_member");
			let next = self.append_block("union_field_next");
			self.builder
				.build_conditional_branch(holds, selected, next)?;
			self.builder.position_at_end(selected);
			let mark = self.mark();
			let field = self.union_field_slot(value, item, index, member)?;
			let selected = if same {
				Val::new(Repr::Pointer, field.pointer)
			} else {
				let field = self.load(field)?;
				let field = if matches!(
					repr,
					Repr::Union
						| Repr::InlineUnion(_)
						| Repr::Optional(Scalar::Union | Scalar::InlineUnion(_))
				) {
					self.inject(field, field_type, member)?
				} else {
					field
				};
				self.owned_as(field, repr, Some(member))?
			};
			self.store(result, selected, member)?;
			self.end_scope(mark, None)?;
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(next);
		}
		// The checker proves that one of the declared alternatives is present.
		self.builder.build_unreachable()?;
		self.builder.position_at_end(end);
		Ok(Some((
			if same {
				Slot {
					pointer: self
						.load(result)?
						.value
						.expect("a field address")
						.into_pointer_value(),
					repr,
				}
			} else {
				result
			},
			!same,
		)))
	}
}
