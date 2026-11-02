//! Relations (advanced brief §8, `relations.md`; lowering §8.8): the sides
//! are fields of the elements, which live in pools and never move.
//!
//! - A side holding one element (a direct side `(T in c)?`, a reverse
//!   one-to-one side, a symmetric side, an owner field) is a
//!   [`Repr::Link`]: the address of the target, null for `none`. Reading it
//!   gives an optional view of the target.
//! - A side holding several (a many-to-many direct side, a `Pool` reverse
//!   side) is a [`Repr::Links`]: a pointer to a set of addresses of the
//!   runtime (`FaLinks`, insertion order), null while empty.
//! - A write of a direct side (`Analysis::relation_writes`) is the
//!   two-sided update of §8.5, in the runtime (`fa_relate_one`,
//!   `fa_relate_add`, `fa_relate_delete`, `fa_relate_clear`), given the
//!   byte offsets of both sides; an owner field is a plain store. A literal
//!   given to `add` that sets direct sides is linked once the element is
//!   stored.
//! - A removal from a related collection (`Analysis::relation_unlinks`)
//!   first unlinks each removed element (`fa_unlink`, per relation and
//!   role of the element).
//! - A copy of an element has no links (D18); a copy of an owner of related
//!   collections relinks the copies to each other ([`Lowering::build_relink`]).
//!   Printing, `==` and hashing skip the sides (D21).

use fa_analyzer::analysis::{
	Analysis, Argument, CallInfo, CollectionRef, Relation, RelationKind, RelationSide,
};
use fa_analyzer::ids::{NodeId, Sym, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, StructValue};
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

/// The relation sides of the object types of a program: per object type,
/// the representation of each side field.
pub(crate) type RelationSides = FxHashMap<TypeId, FxHashMap<Sym, Repr>>;

/// The relation sides declared by the files of a program.
pub(crate) fn relation_sides<'a>(analyses: impl Iterator<Item = &'a Analysis>) -> RelationSides {
	let mut sides = RelationSides::default();
	for analysis in analyses {
		let type_of = |decl: NodeId| analysis.type_decls.get(&decl).map(|decl| decl.ty);
		for relation in &analysis.relations {
			let (Some(holder), Some(target)) =
				(type_of(relation.holder_type), type_of(relation.target_type))
			else {
				continue;
			};
			let direct = match relation.kind {
				RelationKind::ManyToMany => Repr::Links(target),
				_ => Repr::Link(target),
			};
			sides
				.entry(holder)
				.or_default()
				.insert(relation.direct, direct);
			let Some(reverse) = relation.reverse else {
				continue;
			};
			let reverse_repr = match relation.kind {
				RelationKind::OneToMany | RelationKind::ManyToMany => Repr::Links(holder),
				_ => Repr::Link(holder),
			};
			sides
				.entry(target)
				.or_default()
				.insert(reverse, reverse_repr);
		}
	}
	sides
}

/// An entry of the unlinking of a removed element: the relation's kind
/// code, whether the element holds the direct side, the offsets of both
/// sides, and for an owner field the address of the field (or null).
#[derive(Copy, Clone)]
struct Unlink<'ctx> {
	kind: u64,
	holder: bool,
	direct: u64,
	reverse: u64,
	owner: PointerValue<'ctx>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	// ── layouts ─────────────────────────────────────────────────────────

	/// The representation of a field of an object type when it is a
	/// relation side.
	pub(crate) fn side_repr(&self, object: TypeId, name: Sym) -> Option<Repr> {
		self.relation_sides.get(&object)?.get(&name).copied()
	}

	/// Whether an object type has relation sides.
	pub(crate) fn has_sides(&self, object: TypeId) -> bool {
		self.relation_sides.contains_key(&object)
	}

	/// The representation of the field at `index` of an object type.
	pub(crate) fn field_repr(
		&self,
		object: TypeId,
		index: usize,
		node: usize,
	) -> Result<Repr, Error> {
		let field = self.types().fields_of(object)[index];
		match self.side_repr(object, field.name) {
			Some(repr) => Ok(repr),
			None => self.repr_of(field.ty, node),
		}
	}

	/// Whether a type owns a collection of elements with relation sides: a
	/// copy of it relinks the copies.
	pub(crate) fn owns_related_elements(&self, ty: TypeId) -> bool {
		!self.relation_sides.is_empty()
			&& self
				.object_fields(ty)
				.into_iter()
				.any(|repr| self.related_pool(repr).is_some())
	}

	/// The element type of a pool whose elements have relation sides.
	fn related_pool(&self, repr: Repr) -> Option<TypeId> {
		let Repr::Pool(item) = repr else {
			return None;
		};
		match self.repr_of(item, 0) {
			Ok(Repr::Object(element)) if self.has_sides(element) => Some(element),
			_ => None,
		}
	}

	/// The byte offset of a field in the struct of an object type.
	fn field_offset(&self, object: TypeId, name: Sym) -> Result<u64, Error> {
		let Some(index) = self
			.types()
			.fields_of(object)
			.iter()
			.position(|field| field.name == name)
		else {
			return Err(Error::Llvm("a relation side that isn't a field".into()));
		};
		let data = inkwell::targets::TargetData::create(
			self.module
				.get_data_layout()
				.as_str()
				.to_str()
				.unwrap_or(""),
		);
		data.offset_of_element(&self.object_type(object), index as u32)
			.ok_or_else(|| Error::Llvm("a field without an offset".into()))
	}

	/// The object type a type declaration declares.
	fn declared_type(&self, decl: NodeId) -> Result<TypeId, Error> {
		self.analysis
			.type_decls
			.get(&decl)
			.map(|decl| decl.ty)
			.ok_or_else(|| Error::Llvm("a relation without its type".into()))
	}

	/// The offsets of the direct and reverse sides of a relation (the
	/// direct one twice without a reverse side).
	fn side_offsets(&self, relation: &Relation) -> Result<(u64, u64), Error> {
		let holder = self.declared_type(relation.holder_type)?;
		let direct = self.field_offset(holder, relation.direct)?;
		let reverse = match relation.reverse {
			Some(reverse) => {
				let target = self.declared_type(relation.target_type)?;
				self.field_offset(target, reverse)?
			}
			None => direct,
		};
		Ok((direct, reverse))
	}

	fn offset_value(&self, offset: u64) -> IntValue<'ctx> {
		self.context.i64_type().const_int(offset, false)
	}

	// ── reading ─────────────────────────────────────────────────────────

	/// A side holding one element, read: an optional view of the element.
	pub(crate) fn link_view(&mut self, link: Val<'ctx>, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Repr::Link(target) = link.repr else {
			return Ok(link);
		};
		let pointer = link.value.expect("a link").into_pointer_value();
		self.optional_element(pointer, Repr::Object(target), node)
	}

	/// The number of elements of a side holding several.
	fn links_size(&self, links: PointerValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let present = self.builder.build_is_not_null(links, "")?;
		let start = self.builder.get_insert_block().expect("a block");
		let read = self.append_block("links_size");
		let end = self.append_block("links_sized");
		self.builder.build_conditional_branch(present, read, end)?;
		self.builder.position_at_end(read);
		let size = self
			.builder
			.build_load(i64, links, "size")?
			.into_int_value();
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let phi = self.builder.build_phi(i64, "")?;
		phi.add_incoming(&[(&i64.const_zero(), start), (&size, read)]);
		Ok(phi.as_basic_value().into_int_value())
	}

	/// `side.size` of a side holding several elements.
	pub(crate) fn links_size_value(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let links = value.value.expect("links").into_pointer_value();
		Ok(Val::new(Repr::Int(IntRepr::I64), self.links_size(links)?))
	}

	/// The address of the element a reference expression designates (the
	/// value given to a relation side): `none` is null, a relation side is
	/// the address it holds, a reference (`use`) its element.
	fn link_address(&mut self, node: NodeId) -> Result<PointerValue<'ctx>, Error> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		if let View::None = self.view(inner) {
			return Ok(self.pointer_type().const_null());
		}
		if !self.is_place(inner) {
			return Err(self.unsupported(node.index(), "this value as a relation target"));
		}
		let slot = self.place(inner)?;
		match slot.repr {
			Repr::Link(_) => Ok(self
				.builder
				.build_load(self.pointer_type(), slot.pointer, "")?
				.into_pointer_value()),
			Repr::Object(_) => Ok(slot.pointer),
			_ => Err(self.unsupported(node.index(), "this value as a relation target")),
		}
	}

	/// The place of the holder of a side, and the side's member node:
	/// `holder.side` (`skip` more members after the side: `holder.side.add`).
	fn side_holder(
		&mut self,
		access: NodeId,
		skip: usize,
	) -> Result<(PointerValue<'ctx>, NodeId), Error> {
		let View::Access { target, members } = self.view(access) else {
			return Err(self.unsupported(access.index(), "this relation side"));
		};
		let members: Vec<NodeId> = members.iter().collect();
		if members.len() < skip + 1 {
			return Err(self.unsupported(access.index(), "this relation side"));
		}
		let side = members[members.len() - 1 - skip];
		let before = &members[..members.len() - 1 - skip];
		let slot = self.place(target)?;
		let slot = self.place_members(slot, self.type_of(target), before.iter().copied())?;
		if !matches!(slot.repr, Repr::Object(_)) {
			return Err(self.unsupported(access.index(), "this relation side"));
		}
		Ok((slot.pointer, side))
	}

	// ── writing ─────────────────────────────────────────────────────────

	/// `holder.side = value` of a relation side (the assignment `node` is in
	/// `Analysis::relation_writes`): the two-sided update.
	pub(crate) fn relation_assignment(
		&mut self,
		node: NodeId,
		target: NodeId,
		value: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let id = self.analysis.relation_writes[&node][0];
		let relation = self.analysis.relations[id as usize].clone();
		let mut inner = target;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let (holder, _) = self.side_holder(inner, 0)?;
		let address = self.link_address(value)?;
		self.relate_one(&relation, holder, address, node)?;
		Ok(Val::void())
	}

	/// Links `holder` to `target` through a side holding one element.
	fn relate_one(
		&mut self,
		relation: &Relation,
		holder: PointerValue<'ctx>,
		target: PointerValue<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		let (direct, reverse) = self.side_offsets(relation)?;
		match relation.kind {
			RelationKind::OwnerField => {
				let field = unsafe {
					self.builder.build_in_bounds_gep(
						self.context.i8_type(),
						holder,
						&[self.offset_value(direct)],
						"",
					)?
				};
				self.builder.build_store(field, target)?;
			}
			RelationKind::ManyToMany => {
				return Err(self.unsupported(node.index(), "assigning a many-to-many side"));
			}
			kind => {
				let many = self
					.context
					.i32_type()
					.const_int(u64::from(kind == RelationKind::OneToMany), false);
				self.call_runtime(
					"fa_relate_one",
					&[
						holder.into(),
						self.offset_value(direct).into(),
						self.offset_value(reverse).into(),
						target.into(),
						many.into(),
					],
				)?;
			}
		}
		Ok(())
	}

	/// Links an element just stored by `add` (the call `node`, in
	/// `Analysis::relation_writes`) to the targets its literal gave to its
	/// direct sides (`pending`, collected by the literal).
	pub(crate) fn link_added(
		&mut self,
		node: NodeId,
		element: PointerValue<'ctx>,
		element_type: TypeId,
		pending: Vec<(Sym, Vec<PointerValue<'ctx>>)>,
	) -> Result<(), Error> {
		let Some(ids) = self.analysis.relation_writes.get(&node).cloned() else {
			return Ok(());
		};
		let Some(decl) = self
			.analysis
			.type_decls
			.iter()
			.find(|(_, decl)| decl.ty == element_type)
			.map(|(node, _)| *node)
		else {
			return Ok(());
		};
		for (name, targets) in pending {
			let Some(&(id, RelationSide::Direct)) =
				self.analysis.relation_fields.get(&(decl, name))
			else {
				continue;
			};
			if !ids.contains(&id) {
				continue;
			}
			let relation = self.analysis.relations[id as usize].clone();
			if relation.kind == RelationKind::ManyToMany {
				let (direct, reverse) = self.side_offsets(&relation)?;
				for target in targets {
					self.call_value(
						self.runtime.function("fa_relate_add")?,
						&[
							element.into(),
							self.offset_value(direct).into(),
							self.offset_value(reverse).into(),
							target.into(),
						],
					)?;
				}
			} else if let Some(&target) = targets.first() {
				self.relate_one(&relation, element, target, node)?;
			}
		}
		Ok(())
	}

	/// The value of a relation side given in an object literal: the
	/// addresses of its targets, kept for [`Lowering::link_added`], the side
	/// starting empty.
	pub(crate) fn literal_side(
		&mut self,
		name: Sym,
		value: NodeId,
		repr: Repr,
	) -> Result<Val<'ctx>, Error> {
		let mut inner = value;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let targets = match (repr, self.view(inner)) {
			(Repr::Links(_), View::Array { items }) => {
				let items: Vec<NodeId> = items.iter().collect();
				let mut targets = Vec::new();
				for item in items {
					targets.push(self.link_address(item)?);
				}
				targets
			}
			(Repr::Link(_), _) => vec![self.link_address(value)?],
			_ => return Err(self.unsupported(value.index(), "this relation side")),
		};
		self.pending_links.push((name, targets));
		Ok(Val::new(repr, self.pointer_type().const_null()))
	}

	// ── the pools of relation sides ─────────────────────────────────────

	/// The methods of a side holding several elements (`RelationPool#…`):
	/// the writes of a many-to-many side, and the reads.
	pub(crate) fn relation_pool_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let method = path.trim_start_matches("RelationPool#");
		let arguments: Vec<NodeId> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		let View::Call { callee, .. } = self.view(node) else {
			return Err(self.unsupported(node.index(), path));
		};
		if let Some(&id) = self
			.analysis
			.relation_writes
			.get(&node)
			.and_then(|ids| ids.first())
		{
			let relation = self.analysis.relations[id as usize].clone();
			let (direct, reverse) = self.side_offsets(&relation)?;
			let (holder, _) = self.side_holder(callee, 1)?;
			let offsets = [
				self.offset_value(direct).into(),
				self.offset_value(reverse).into(),
			];
			return match (method, arguments.as_slice()) {
				("add" | "delete", [target]) => {
					let target = self.link_address(*target)?;
					let function = if method == "add" {
						"fa_relate_add"
					} else {
						"fa_relate_delete"
					};
					let changed = self
						.call_value(
							self.runtime.function(function)?,
							&[holder.into(), offsets[0], offsets[1], target.into()],
						)?
						.into_int_value();
					let changed = self.builder.build_int_compare(
						inkwell::IntPredicate::NE,
						changed,
						changed.get_type().const_zero(),
						"",
					)?;
					Ok(Val::new(Repr::Bool, changed))
				}
				("clear", []) => {
					self.call_runtime("fa_relate_clear", &[holder.into(), offsets[0], offsets[1]])?;
					Ok(Val::void())
				}
				_ => Err(self.unsupported(node.index(), path)),
			};
		}
		let side = self.receiver(node)?;
		if side.repr == Repr::Never {
			return Ok(side);
		}
		let Repr::Links(element_type) = side.repr else {
			return Err(self.unsupported(node.index(), path));
		};
		let links = side.value.expect("links").into_pointer_value();
		match (method, arguments.as_slice()) {
			("isEmpty", []) => {
				let size = self.links_size(links)?;
				let empty = self.builder.build_int_compare(
					inkwell::IntPredicate::EQ,
					size,
					size.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, empty))
			}
			("includes", [target]) => {
				let target = self.link_address(*target)?;
				let found = self
					.call_value(
						self.runtime.function("fa_links_has")?,
						&[links.into(), target.into()],
					)?
					.into_int_value();
				let found = self.builder.build_int_compare(
					inkwell::IntPredicate::NE,
					found,
					found.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, found))
			}
			// the other reads go through a view of the targets as an array
			_ => {
				let mut values = Vec::new();
				for &argument in &arguments {
					let value = self.expr(argument)?;
					if value.repr == Repr::Never {
						return Ok(value);
					}
					values.push(value);
				}
				// the item type of the side (a recursive edge, or the element)
				let item = match self.view(callee) {
					View::Access { members, .. } if members.len() >= 2 => {
						let members: Vec<NodeId> = members.iter().collect();
						match self.types().kind(self.type_of(members[members.len() - 2])) {
							fa_analyzer::types::TypeKind::Collection(_, item, _) => item,
							_ => element_type,
						}
					}
					_ => element_type,
				};
				let view = self.links_view(links, item)?;
				self.array_method_more(node, &format!("Array#{method}"), view, &values, &arguments)
			}
		}
	}

	/// A view of the targets of a side as an array of items of type `item`
	/// (the side's), in a block that doesn't own them, freed at the end of
	/// the statement: the elements copied bytewise, or their addresses when
	/// the items are recursive edges (which point at their value).
	fn links_view(&mut self, links: PointerValue<'ctx>, item: TypeId) -> Result<Val<'ctx>, Error> {
		let repr = self.item_repr(item, NodeId::new(0))?;
		let size = self.links_size(links)?;
		let zero = self.context.i64_type().const_zero();
		let view = self.new_array(repr, zero, size)?;
		let item_size = self.item_size_const(repr);
		self.links_each(links, |this, address| {
			let slot = this
				.call_value(
					this.runtime.function("fa_array_push_slot")?,
					&[view.into(), item_size.into()],
				)?
				.into_pointer_value();
			let value: BasicValueEnum<'ctx> = match repr {
				Repr::Rec(_) => address.into(),
				_ => this.builder.build_load(this.llvm_type(repr), address, "")?,
			};
			this.builder.build_store(slot, value)?;
			Ok(())
		})?;
		self.cleanups
			.borrow_mut()
			.push(super::ownership::Cleanup::Shallow(view, item_size));
		Ok(Val::new(Repr::Array(item), view))
	}

	/// A loop over the addresses of a side holding several elements.
	fn links_each(
		&mut self,
		links: PointerValue<'ctx>,
		mut body: impl FnMut(&mut Self, PointerValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let counter = self.alloca(Repr::Int(IntRepr::I64), "links_position")?;
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("links_each");
		let block = self.append_block("links_each_body");
		let exit = self.append_block("end_links_each");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let (found, end) = self.seek(&self.builder, "fa_links_seek", links, position)?;
		self.builder.build_conditional_branch(end, exit, block)?;
		self.builder.position_at_end(block);
		let address = self.at(&self.builder, "fa_links_at", links, found)?;
		let next = self
			.builder
			.build_int_add(found, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		body(self, address)?;
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(())
	}

	/// `<seek>(collection, position)` and whether it reached the end.
	pub(crate) fn seek(
		&self,
		builder: &Builder<'ctx>,
		function: &str,
		collection: PointerValue<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let found = builder
			.build_call(
				self.runtime.function(function)?,
				&[collection.into(), position.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a position")
			.into_int_value();
		let end = builder.build_int_compare(
			inkwell::IntPredicate::EQ,
			found,
			self.context.i64_type().const_all_ones(),
			"",
		)?;
		Ok((found, end))
	}

	/// `<at>(collection, position)`: the address of an element.
	pub(crate) fn at(
		&self,
		builder: &Builder<'ctx>,
		function: &str,
		collection: PointerValue<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(builder
			.build_call(
				self.runtime.function(function)?,
				&[collection.into(), position.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an element")
			.into_pointer_value())
	}

	// ── removals ────────────────────────────────────────────────────────

	/// The unlinking of the elements a removal (the call `node`, in
	/// `Analysis::relation_unlinks`) takes out of its collection: per
	/// relation, the role of the elements of the receiver's collection.
	fn unlinks(&mut self, node: NodeId) -> Result<Vec<Unlink<'ctx>>, Error> {
		let Some(ids) = self.analysis.relation_unlinks.get(&node).cloned() else {
			return Ok(Vec::new());
		};
		let View::Call { callee, .. } = self.view(node) else {
			return Ok(Vec::new());
		};
		// the collection the receiver names, and its owner
		let (owner, collection) = match self.view(callee) {
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				match members.len() {
					0 | 1 => (None, target),
					count => (
						Some((target, members[..count - 2].to_vec())),
						members[count - 2],
					),
				}
			}
			_ => return Ok(Vec::new()),
		};
		let matches = |this: &Self, reference: CollectionRef| match reference {
			CollectionRef::Field { name, .. } => {
				this.types().syms.resolve(name) == this.tree.text(collection.index())
			}
			CollectionRef::Module { declaration } => match this.analysis.resolution(collection) {
				Resolution::Local(symbol) => {
					let decl = this.analysis.symbols.symbol(symbol).decl_node;
					decl == declaration
						|| this.analysis.symbols.symbol_of_decl(declaration) == Some(symbol)
				}
				_ => false,
			},
		};
		let mut entries = Vec::new();
		let null = self.pointer_type().const_null();
		for id in ids {
			let relation = self.analysis.relations[id as usize].clone();
			let (direct, reverse) = self.side_offsets(&relation)?;
			let kind = match relation.kind {
				RelationKind::OneToMany => 0,
				RelationKind::OneToOne => 1,
				RelationKind::ManyToMany => 2,
				RelationKind::Symmetric => 3,
				RelationKind::OwnerField => 4,
			};
			let (holder, target) = (
				matches(self, relation.holder),
				matches(self, relation.target),
			);
			let (holder, target) = if holder || target {
				(holder, target)
			} else {
				(true, true)
			};
			let entry = |holder: bool, owner: PointerValue<'ctx>| Unlink {
				kind,
				holder,
				direct,
				reverse,
				owner,
			};
			match relation.kind {
				RelationKind::OwnerField => {
					// the owner field of the collection's owner
					let field = match &owner {
						Some((target, prefix)) => {
							let slot = self.place(*target)?;
							let slot = self.place_members(
								slot,
								self.type_of(*target),
								prefix.iter().copied(),
							)?;
							unsafe {
								self.builder.build_in_bounds_gep(
									self.context.i8_type(),
									slot.pointer,
									&[self.offset_value(direct)],
									"",
								)?
							}
						}
						None => null,
					};
					entries.push(entry(false, field));
				}
				RelationKind::Symmetric => entries.push(entry(false, null)),
				_ => {
					if target {
						entries.push(entry(false, null));
					}
					if holder {
						entries.push(entry(true, null));
					}
				}
			}
		}
		Ok(entries)
	}

	/// Unlinks an element about to leave its collection.
	fn unlink_element(
		&self,
		entries: &[Unlink<'ctx>],
		element: PointerValue<'ctx>,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		for entry in entries {
			self.call_runtime(
				"fa_unlink",
				&[
					element.into(),
					i32.const_int(entry.kind, false).into(),
					i32.const_int(u64::from(entry.holder), false).into(),
					self.offset_value(entry.direct).into(),
					self.offset_value(entry.reverse).into(),
					entry.owner.into(),
				],
			)?;
		}
		Ok(())
	}

	/// The unlinking of a removal, prepared before its elements are reached.
	pub(crate) fn prepare_unlinks(&mut self, node: NodeId) -> Result<Unlinks<'ctx>, Error> {
		Ok(Unlinks(self.unlinks(node)?))
	}

	/// Unlinks `element` as `unlinks` says.
	pub(crate) fn unlink(
		&self,
		unlinks: &Unlinks<'ctx>,
		element: PointerValue<'ctx>,
	) -> Result<(), Error> {
		self.unlink_element(&unlinks.0, element)
	}

	/// An element taken out of its collection, without its links: its sides
	/// are emptied (D18).
	pub(crate) fn without_links(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let Repr::Object(object) = value.repr else {
			return Ok(value);
		};
		if !self.has_sides(object) {
			return Ok(value);
		}
		let mut structure = value.value.expect("an object").into_struct_value();
		for (index, repr) in self.object_fields(object).into_iter().enumerate() {
			match repr {
				Repr::Link(_) => {}
				Repr::Links(_) => {
					let links = self
						.builder
						.build_extract_value(structure, index as u32, "")?;
					self.call_runtime("fa_links_free", &[links.into()])?;
				}
				_ => continue,
			}
			structure = self
				.builder
				.build_insert_value(
					structure,
					self.pointer_type().const_null(),
					index as u32,
					"",
				)?
				.into_struct_value();
		}
		Ok(Val::new(value.repr, structure))
	}

	// ── loops ───────────────────────────────────────────────────────────

	/// `for element in holder.side`: the variable designates each target.
	pub(crate) fn for_links(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		side: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Links(element_type) = side.repr else {
			return Err(self.unsupported(node.index(), "this loop"));
		};
		let links = side.value.expect("links").into_pointer_value();
		self.for_slots(
			node,
			pattern,
			links,
			Repr::Object(element_type),
			super::pools::Cursor::Seek("fa_links_seek", "fa_links_at"),
			body,
		)
	}

	// ── copies ──────────────────────────────────────────────────────────

	/// The copy of an owner of related collections, its fields copied (the
	/// elements without links): every link between elements of the
	/// original joins their copies, and an owner field points at the copy
	/// of its target (§8.5, copies). A link to an element outside the owner
	/// is left out.
	pub(crate) fn build_relink(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		ty: TypeId,
		original: StructValue<'ctx>,
		copy: StructValue<'ctx>,
	) -> Result<StructValue<'ctx>, Error> {
		let fields = self.object_fields(ty);
		let pools: Vec<(u32, TypeId)> = fields
			.iter()
			.enumerate()
			.filter_map(|(index, repr)| Some((index as u32, self.related_pool(*repr)?)))
			.collect();
		let pointer = |value: BasicValueEnum<'ctx>| value.into_pointer_value();
		let i64 = self.context.i64_type();
		// the map from the elements to their copies
		let mut count = i64.const_zero();
		for &(index, _) in &pools {
			let pool = pointer(builder.build_extract_value(original, index, "")?);
			let size = builder.build_load(i64, pool, "")?.into_int_value();
			count = builder.build_int_add(count, size, "")?;
		}
		let map = builder
			.build_call(
				self.runtime.function("fa_address_map_new")?,
				&[count.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a map")
			.into_pointer_value();
		for &(index, _) in &pools {
			let from = pointer(builder.build_extract_value(original, index, "")?);
			let to = pointer(builder.build_extract_value(copy, index, "")?);
			self.helper_pool_pairs(builder, function, from, to, |builder, a, b| {
				builder.build_call(
					self.runtime.function("fa_address_map_put")?,
					&[map.into(), a.into(), b.into()],
					"",
				)?;
				Ok(())
			})?;
		}
		// the sides of the copies
		for &(index, element) in &pools {
			let structure = self.object_type(element);
			let sides: Vec<(u32, Repr)> = self
				.object_fields(element)
				.into_iter()
				.enumerate()
				.filter(|(_, repr)| matches!(repr, Repr::Link(_) | Repr::Links(_)))
				.map(|(index, repr)| (index as u32, repr))
				.collect();
			let from = pointer(builder.build_extract_value(original, index, "")?);
			let to = pointer(builder.build_extract_value(copy, index, "")?);
			self.helper_pool_pairs(builder, function, from, to, |builder, a, b| {
				for &(field, repr) in &sides {
					let source = builder.build_struct_gep(structure, a, field, "")?;
					let destination = builder.build_struct_gep(structure, b, field, "")?;
					let side = builder.build_load(self.pointer_type(), source, "")?;
					let relinked = self.relink_side(builder, repr, side, map)?;
					builder.build_store(destination, relinked)?;
				}
				Ok(())
			})?;
		}
		// the owner fields of the owner itself
		let mut copy = copy;
		for (index, repr) in fields.into_iter().enumerate() {
			if let Repr::Link(_) = repr {
				let side = builder.build_extract_value(original, index as u32, "")?;
				let relinked = self.relink_side(builder, repr, side, map)?;
				copy = builder
					.build_insert_value(copy, relinked, index as u32, "")?
					.into_struct_value();
			}
		}
		builder.build_call(
			self.runtime.function("fa_address_map_free")?,
			&[map.into()],
			"",
		)?;
		Ok(copy)
	}

	/// A side of an original element, for its copy: through the map.
	fn relink_side(
		&self,
		builder: &Builder<'ctx>,
		repr: Repr,
		side: BasicValueEnum<'ctx>,
		map: PointerValue<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let function = match repr {
			Repr::Links(_) => "fa_links_relink",
			_ => "fa_address_map_get",
		};
		let (first, second) = match repr {
			Repr::Links(_) => (side, map.into()),
			_ => (map.into(), side),
		};
		Ok(builder
			.build_call(
				self.runtime.function(function)?,
				&[first.into(), second.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an address"))
	}

	/// A loop over the elements of two pools in step (an original and its
	/// copy, which holds the same number of elements in the same order).
	fn helper_pool_pairs(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		first: PointerValue<'ctx>,
		second: PointerValue<'ctx>,
		mut body: impl FnMut(
			&Builder<'ctx>,
			PointerValue<'ctx>,
			PointerValue<'ctx>,
		) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let before = builder.get_insert_block().expect("a block");
		let test = self.context.append_basic_block(function, "pairs");
		let block = self.context.append_basic_block(function, "pairs_body");
		let end = self.context.append_basic_block(function, "pairs_end");
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(test);
		let a = builder.build_phi(i64, "a")?;
		let b = builder.build_phi(i64, "b")?;
		a.add_incoming(&[(&i64.const_zero(), before)]);
		b.add_incoming(&[(&i64.const_zero(), before)]);
		let (found_a, done) = self.seek(
			builder,
			"fa_pool_seek",
			first,
			a.as_basic_value().into_int_value(),
		)?;
		builder.build_conditional_branch(done, end, block)?;
		builder.position_at_end(block);
		let (found_b, _) = self.seek(
			builder,
			"fa_pool_seek",
			second,
			b.as_basic_value().into_int_value(),
		)?;
		let element_a = self.at(builder, "fa_pool_at", first, found_a)?;
		let element_b = self.at(builder, "fa_pool_at", second, found_b)?;
		body(builder, element_a, element_b)?;
		let one = i64.const_int(1, false);
		let next_a = builder.build_int_add(found_a, one, "")?;
		let next_b = builder.build_int_add(found_b, one, "")?;
		let last = builder.get_insert_block().expect("a block");
		a.add_incoming(&[(&next_a, last)]);
		b.add_incoming(&[(&next_b, last)]);
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(end);
		Ok(())
	}
}

/// The unlinking of the elements of one removal (see
/// [`Lowering::prepare_unlinks`]).
pub(crate) struct Unlinks<'ctx>(Vec<Unlink<'ctx>>);

impl Unlinks<'_> {
	/// Whether the removal unlinks nothing.
	pub(crate) fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}
