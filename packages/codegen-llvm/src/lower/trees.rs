//! Regions for recursively built objects and named unions. The proof is
//! closed over every unit (`fa_analyzer::regions`): no generic or asynchronous code, finite passive payloads
//! and edges to the same root type. Every constructor belongs to a root builder;
//! roots cannot escape through captures, fields, containers or global writes.
//! Node fields stay immutable, while builders may replace their own root locals.
//! Union builders can throw; their wrapper frees partial trees on failure.
//!
//! Ordinary nodes need no destruction walk; an owning string or collection adds one
//! cleanup record to the region. Copies outside a builder get their own region.
//! Other recursive shapes retain the general ownership implementation.

use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use fa_analyzer::types::{CollKind, TypeKind};
use inkwell::builder::Builder;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue};
use rustc_hash::FxHashSet;

use super::values::{Repr, Scalar};
use super::{FAST_CALLING_CONVENTION, Lowering};
use crate::error::Error;

pub(super) const ARENA_CHUNK_BYTES: u64 = 4096;

#[derive(Default)]
pub(super) struct TreeArenas<'ctx> {
	/// Resolved claims for a whole-module build. Split builds consult their
	/// shared ProgramFacts directly, recording the answer in the object cache.
	facts: std::collections::BTreeSet<String>,
	/// The links of the nodes allocated through them (`through_links`), by
	/// address. A link dominates every use of its address: both are made
	/// where the node is allocated.
	links: std::cell::RefCell<
		rustc_hash::FxHashMap<PointerValue<'ctx>, inkwell::values::IntValue<'ctx>>,
	>,
	objects: FxHashSet<TypeId>,
	pub(super) unions: FxHashSet<NominalId>,
	/// The arena object types whose recursive edges are compressed: 32-bit
	/// offsets, in units of 8 bytes, from the runtime's `fa_arena_base`, 0
	/// for an absent edge, in memory and as values (`design/decisions.md`,
	/// "Compressed links for arena-allocated recursive values").
	compressed: FxHashSet<TypeId>,
}

impl TreeArenas<'_> {
	/// Whether some edges are compressed: the runtime takes the arena pages
	/// from the range of `fa_arena_base` (`fa_compressed_links`).
	pub(super) fn compressed_any(&self) -> bool {
		!self.compressed.is_empty()
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Arena nodes must fit one runtime page, including FaArena's five-pointer
	/// header (rounded to 16 bytes) and the allocator's eight-byte size rounding.
	fn arena_type_fits(&self, ty: inkwell::types::BasicTypeEnum<'ctx>) -> bool {
		let data = self.target_data();
		let header = (5 * u64::from(self.usize.get_bit_width() / 8)).next_multiple_of(16);
		let align = u64::from(data.get_abi_alignment(&ty));
		let offset = header.next_multiple_of(align);
		let size = data.get_abi_size(&ty);
		const PAGE_SIZE: u64 = ARENA_CHUNK_BYTES;
		offset <= PAGE_SIZE && size <= PAGE_SIZE && size.next_multiple_of(8) <= PAGE_SIZE - offset
	}

	/// Passive owning fields of a recursive member. Recursive
	/// edges belong to the region and must never be followed by its cleanup.
	fn arena_payload_slots(
		&self,
		b: &Builder<'ctx>,
		node: PointerValue<'ctx>,
		layout: inkwell::types::StructType<'ctx>,
		fields: &[Repr],
		owner: NominalId,
	) -> Result<Vec<(PointerValue<'ctx>, Repr, bool)>, Error> {
		let mut slots = Vec::new();
		for (index, repr) in fields.iter().enumerate() {
			let Repr::Object(ty) = repr else {
				continue;
			};
			let data = b.build_struct_gep(layout, node, index as u32 + 1, "")?;
			for (index, field) in self.object_fields(*ty).iter().enumerate() {
				if self.needs_drop(*field) && *field != Repr::Union {
					slots.push((
						b.build_struct_gep(self.object_type(*ty), data, index as u32, "")?,
						*field,
						self.arena_array_nominal(*field) == Some(owner),
					));
				}
			}
		}
		Ok(slots)
	}

	pub(super) fn register_arena_payloads(
		&self,
		b: &Builder<'ctx>,
		id: u64,
		node: PointerValue<'ctx>,
		layout: inkwell::types::StructType<'ctx>,
		fields: &[Repr],
	) -> Result<(), Error> {
		let owner = self
			.union_ids
			.borrow()
			.iter()
			.find_map(|((nominal, _), member)| (*member == id).then_some(*nominal))
			.expect("an arena member id");
		let slots = self.arena_payload_slots(b, node, layout, fields, owner)?;
		if slots.is_empty() {
			return Ok(());
		}
		let mut owning = self.context.bool_type().const_zero();
		for (slot, repr, child) in slots {
			if child {
				let array = b.build_load(self.pointer_type(), slot, "")?;
				let region = b
					.build_call(
						self.runtime.function("fa_array_is_region")?,
						&[array.into()],
						"",
					)?
					.try_as_basic_value()
					.basic()
					.expect("a region marker")
					.into_int_value();
				let external = b.build_int_compare(
					inkwell::IntPredicate::EQ,
					region,
					region.get_type().const_zero(),
					"",
				)?;
				owning = b.build_or(owning, external, "")?;
				continue;
			}
			if repr != Repr::Str {
				owning = self.context.bool_type().const_int(1, false);
				break;
			}
			let cap_ptr = b.build_struct_gep(self.string_type, slot, 2, "")?;
			let cap = b.build_load(self.usize, cap_ptr, "")?.into_int_value();
			let kind = b.build_and(cap, self.simple_flag().const_not(), "")?;
			let nonzero =
				b.build_int_compare(inkwell::IntPredicate::NE, kind, self.usize.const_zero(), "")?;
			let heap = b.build_int_compare(
				inkwell::IntPredicate::ULT,
				kind,
				self.string_kind_mask(),
				"",
			)?;
			let tag = b.build_and(kind, self.usize.const_int(1, false), "")?;
			let even =
				b.build_int_compare(inkwell::IntPredicate::EQ, tag, self.usize.const_zero(), "")?;
			let heap = b.build_and(heap, even, "")?;
			owning = b.build_or(owning, b.build_and(nonzero, heap, "")?, "")?;
		}
		let name = format!("fa.arena.payloads.{id}");
		let drop = match self.module.get_function(&name) {
			Some(drop) => drop,
			None => {
				let ty = self
					.context
					.void_type()
					.fn_type(&[self.pointer_type().into()], false);
				let (drop, cleanup) = self.helper_function(&name, ty);
				let node = drop.get_nth_param(0).expect("a node").into_pointer_value();
				for (slot, repr, child) in
					self.arena_payload_slots(&cleanup, node, layout, fields, owner)?
				{
					let value = cleanup.build_load(self.llvm_type(repr), slot, "")?;
					if child {
						let drop = self.child_array_drop_function(repr)?;
						cleanup.build_call(drop, &[value.into()], "")?;
					} else {
						self.call_drop(&cleanup, repr, value)?;
					}
				}
				cleanup.build_return(None)?;
				drop
			}
		};
		let function = b
			.get_insert_block()
			.expect("block")
			.get_parent()
			.expect("function");
		let register = self.context.append_basic_block(function, "owning_payload");
		let end = self
			.context
			.append_basic_block(function, "payload_cleanup_registered");
		b.build_conditional_branch(owning, register, end)?;
		b.position_at_end(register);
		b.build_call(
			self.runtime.function("fa_arena_cleanup")?,
			&[
				node.into(),
				drop.as_global_value().as_pointer_value().into(),
			],
			"",
		)?;
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		Ok(())
	}

	pub(super) fn arena_child_array_type(&self, repr: Repr) -> bool {
		self.arena_array_nominal(repr).is_some()
	}

	/// Only fresh arrays of the proven builder may share its active region.
	/// The same Array<Root> outside that builder remains an ordinary owner.
	pub(super) fn region_child_array(&self, node: NodeId, repr: Repr) -> bool {
		if !self.frame().arena || self.arena_array_nominal(repr).is_none() {
			return false;
		}
		let Some(function) = self.functions.iter().find_map(|(&node, function)| {
			(function.arena_body == Some(self.frame().function)).then_some(node)
		}) else {
			return false;
		};
		let Some(function) = fa_analyzer::regions::function_key(self.session, self.file, function)
		else {
			return false;
		};
		let Some(array) = crate::facts::stable_type_name(self.types(), self.analysis.type_of(node))
		else {
			return false;
		};
		self.region_fact(&fa_analyzer::regions::child_array_key(&function, &array))
	}

	/// Root arrays outside builders own one independent region per element.
	/// A field array of the same proven root instead contains edges into its
	/// owner's region: its cleanup frees only the array storage.
	pub(super) fn arena_array_nominal(&self, repr: Repr) -> Option<NominalId> {
		let Repr::Array(item) = repr else { return None };
		let item = self.types().rec_target(item).unwrap_or(item);
		match self.types().kind(item) {
			TypeKind::NamedUnion(nominal, _) if self.arena_member(nominal) => Some(nominal),
			_ => None,
		}
	}

	fn child_array_drop_function(
		&self,
		repr: Repr,
	) -> Result<inkwell::values::FunctionValue<'ctx>, Error> {
		let Repr::Array(item) = repr else {
			unreachable!("an array of child roots")
		};
		let key = format!("child-array {}", self.shape_key(repr));
		if let Some(function) = self.drop_functions.borrow().get(&key).copied() {
			return Ok(function);
		}
		let signature = self
			.context
			.void_type()
			.fn_type(&[self.pointer_type().into()], false);
		let (function, builder) = self.helper_function("fa.drop.child_array", signature);
		self.drop_functions.borrow_mut().insert(key, function);
		let array = function
			.get_nth_param(0)
			.expect("an array")
			.into_pointer_value();
		let present = builder.build_is_not_null(array, "")?;
		let nonempty = self.context.append_basic_block(function, "present");
		let absent = self.context.append_basic_block(function, "absent");
		builder.build_conditional_branch(present, nonempty, absent)?;
		builder.position_at_end(absent);
		builder.build_return(None)?;
		builder.position_at_end(nonempty);
		let item = self.item_repr(item, NodeId::new(0))?;
		builder.build_call(
			self.runtime.function("fa_array_free")?,
			&[array.into(), self.item_size_const(item).into()],
			"",
		)?;
		builder.build_return(None)?;
		Ok(function)
	}

	/// Whether the recursive edges to this type are compressed links.
	pub(crate) fn compressed(&self, target: TypeId) -> bool {
		self.tree_arenas.compressed.contains(&target)
	}

	/// Whether a representation is an edge (or an optional edge) held as a
	/// compressed link: an `i32`.
	pub(crate) fn compressed_edge(&self, repr: Repr) -> bool {
		matches!(repr, Repr::Rec(target) | Repr::Optional(Scalar::Rec(target)) if self.compressed(target))
	}

	/// The address an edge value of this representation designates (null for
	/// an absent optional edge): the value itself, or a compressed link
	/// decoded (the address divided by 8: the runtime keeps the arena nodes
	/// in the first 32 GiB, and link 0, address 0, is the absent edge).
	pub(crate) fn edge_address(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
		raw: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		if !self.compressed_edge(repr) {
			return Ok(raw.into_pointer_value());
		}
		let i64 = self.context.i64_type();
		let link = b.build_int_z_extend(raw.into_int_value(), i64, "")?;
		let address = b.build_int_nuw_mul(link, i64.const_int(8, false), "")?;
		Ok(b.build_int_to_ptr(address, self.pointer_type(), "node")?)
	}

	/// The edge value of this representation designating an address (null
	/// for an absent optional edge): the address, or its compressed link.
	pub(crate) fn address_edge(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
		address: PointerValue<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if !self.compressed_edge(repr) {
			return Ok(address.into());
		}
		if let Some(link) = self.tree_arenas.links.borrow().get(&address) {
			return Ok((*link).into());
		}
		let i64 = self.context.i64_type();
		let at = b.build_ptr_to_int(address, i64, "")?;
		let link = b.build_right_shift(at, i64.const_int(3, false), false, "")?;
		Ok(b.build_int_truncate(link, self.context.i32_type(), "link")?
			.into())
	}

	/// New arena nodes of this representation (at `block`, plus each of
	/// `units` of 8 bytes), addressed through their compressed links when
	/// edges to them are compressed: the code keeps one link alive across the
	/// calls that build the nodes' fields, instead of addresses and links both.
	pub(crate) fn through_links(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
		block: PointerValue<'ctx>,
		units: &[u64],
	) -> Result<Vec<PointerValue<'ctx>>, Error> {
		let edge = match repr {
			Repr::Object(target) => Repr::Rec(target),
			_ => Repr::Void,
		};
		if !self.compressed_edge(edge) {
			return Ok(units
				.iter()
				.map(|&units| unsafe {
					b.build_in_bounds_gep(
						self.context.i8_type(),
						block,
						&[self.usize.const_int(units * 8, false)],
						"",
					)
				})
				.collect::<Result<_, _>>()?);
		}
		let i64 = self.context.i64_type();
		let node = block;
		let address = b.build_ptr_to_int(node, i64, "")?;
		let link = b.build_right_shift(address, i64.const_int(3, false), false, "")?;
		// an identity the optimizer can't see through: without it, LLVM
		// rewrites the address as `node & 0x7fffffff8` and keeps both again
		let opaque = self.context.create_inline_asm(
			i64.fn_type(&[i64.into()], false),
			String::new(),
			"=r,0".to_string(),
			false,
			false,
			None,
			false,
		);
		let link = b
			.build_indirect_call(
				i64.fn_type(&[i64.into()], false),
				opaque,
				&[link.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a link")
			.into_int_value();
		units
			.iter()
			.map(|&units| {
				let link = b.build_int_nuw_add(link, i64.const_int(units, false), "")?;
				let address = b.build_int_nuw_mul(link, i64.const_int(8, false), "")?;
				let node = b.build_int_to_ptr(address, self.pointer_type(), "node")?;
				// its link, without a shift back that LLVM can't fold (`address_edge`)
				let link = b.build_int_truncate(link, self.context.i32_type(), "link")?;
				self.tree_arenas.links.borrow_mut().insert(node, link);
				Ok(node)
			})
			.collect()
	}

	/// The address held by a slot of an edge (`Repr::Rec`, or an optional
	/// edge narrowed to its value), decoded when it holds a compressed link.
	pub(crate) fn load_edge(&self, slot: super::Slot<'ctx>) -> Result<PointerValue<'ctx>, Error> {
		let raw = self
			.builder
			.build_load(self.llvm_type(slot.repr), slot.pointer, "")?;
		self.edge_address(&self.builder, slot.repr, raw)
	}

	pub(super) fn arena_object(&self, ty: TypeId) -> bool {
		self.tree_arenas.objects.contains(&ty)
	}

	pub(super) fn arena_member(&self, nominal: NominalId) -> bool {
		self.tree_arenas.unions.contains(&nominal)
	}

	pub(super) fn arena_box(&self, id: u64) -> bool {
		self.union_ids
			.borrow()
			.iter()
			.any(|((nominal, _), member_id)| *member_id == id && self.arena_member(*nominal))
	}

	pub(super) fn arena_repr(&self, repr: Repr) -> bool {
		match repr {
			Repr::Object(ty) | Repr::Rec(ty) => self.arena_object(ty),
			_ => false,
		}
	}

	/// Whether storage of this type may contain a root. Functions do not
	/// own their parameter/result types; captures are checked separately.
	fn tree_value_type(&self, ty: TypeId, root: TypeId) -> bool {
		let mut seen = FxHashSet::default();
		let mut stack = vec![ty];
		while let Some(at) = stack.pop() {
			if at == root
				|| matches!((self.types().kind(at), self.types().kind(root)),
				(TypeKind::Member(a, _) | TypeKind::NamedUnion(a, _), TypeKind::NamedUnion(b, _)) if a == b)
			{
				return true;
			}
			if !seen.insert(at) {
				continue;
			}
			match self.types().kind(at) {
				TypeKind::Rec(_) => stack.extend(self.types().rec_target(at)),
				TypeKind::Object(_) => {
					stack.extend(self.types().fields_of(at).iter().map(|f| f.ty))
				}
				TypeKind::Union(list) | TypeKind::Tuple(list) => {
					stack.extend(self.types().list_items(list))
				}
				TypeKind::Member(n, i) => stack.push(self.types().payload(n, i)),
				TypeKind::NamedUnion(n, _) => {
					for i in 0..self.types().sym_list(self.types().nominal(n).members).len() {
						stack.push(self.types().payload(n, i as u16));
					}
				}
				TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
					stack.push(item);
					stack.push(key);
				}
				TypeKind::Pointer(item) | TypeKind::Sync(_, item) => stack.push(item),
				_ => {}
			}
		}
		false
	}

	/// Finite owned data whose destruction has no user-visible effects.
	/// No recursive edges, custom destructors, closures or relation owners:
	/// delaying its release until the region ends only changes storage usage.
	fn arena_payload_type(&self, ty: TypeId) -> bool {
		let mut seen = FxHashSet::default();
		let mut stack = vec![ty];
		while let Some(at) = stack.pop() {
			if !seen.insert(at) {
				continue;
			}
			if self.repr_of(at, 0).is_ok_and(|r| !self.needs_drop(r)) {
				continue;
			}
			match self.types().kind(at) {
				TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => {}
				TypeKind::Object(_) if self.destroy_function(at).is_none() => {
					stack.extend(self.types().fields_of(at).iter().map(|f| f.ty));
				}
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					stack.extend(self.types().list_items(list));
				}
				TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => {
					stack.push(item);
				}
				_ => return false,
			}
		}
		true
	}

	/// Chooses the recursive types built in regions, and wraps their builders.
	pub(super) fn prepare_tree_arenas(&mut self) -> Result<(), Error> {
		if self.tasks.asynchronous || self.c_library.is_some() {
			return Ok(());
		}
		let mut files: Vec<_> = self.other_files.keys().copied().collect();
		files.push(self.file);
		files.sort_by_key(|&file| self.session.module(file));
		files.dedup();
		if self.program_facts.is_none() {
			self.tree_arenas.facts = fa_analyzer::regions::resolve_claims(
				fa_analyzer::regions::unit_claims(self.session, &files),
			);
		}
		let mut wraps = Vec::new();
		for &file in &files {
			self.switch_file(file);
			for ty in fa_analyzer::regions::candidate_types(self.session, file) {
				let Some(type_key) = fa_analyzer::regions::type_key(self.types(), ty) else {
					continue;
				};
				if !self.region_fact(&type_key) {
					continue;
				}
				let Some((nominal, _)) = self.tree_shapes(ty) else {
					continue;
				};
				if let Some(nominal) = nominal {
					self.tree_arenas.unions.insert(nominal);
				} else {
					self.tree_arenas.objects.insert(ty);
				}
				let root = crate::facts::stable_type_name(self.types(), ty).expect("a proven type");
				for &function in self.functions.keys() {
					if !self.is_reached(function) {
						continue;
					}
					let Some(name) =
						fa_analyzer::regions::function_key(self.session, file, function)
					else {
						continue;
					};
					if self.region_fact(&fa_analyzer::regions::builder_key(&name, &root)) {
						wraps.push((file, function, nominal));
					}
				}
			}
		}
		self.switch_file(self.entry);
		self.compress_tree_links()?;
		wraps.sort_by_key(|&(file, function, _)| (file, function));
		wraps.dedup_by_key(|item| (item.0, item.1));
		for (file, key, nominal) in wraps {
			self.switch_file(file);
			self.wrap_tree_builder(key, nominal)?;
		}
		self.switch_file(self.entry);
		Ok(())
	}

	pub(super) fn region_fact(&self, key: &str) -> bool {
		match &self.program_facts {
			Some(facts) => facts
				.consulted(crate::facts::ROOT_REGIONS, self.fact_log.as_deref())
				.contains(key),
			None => self.tree_arenas.facts.contains(key),
		}
	}

	/// Compressed links (`design/decisions.md`, "Compressed links for
	/// arena-allocated recursive values"): where the runtime reserves the
	/// range of the arenas (64-bit Linux), the edges of the arena object
	/// types are 32-bit offsets. Their layouts change, so the functions
	/// declared with the previous layouts are declared again.
	fn compress_tree_links(&mut self) -> Result<(), Error> {
		if !crate::runtime::Runtime::reserves_arena_space(self.target)
			|| self.is_split()
			|| std::env::var_os("FA_COMPRESSED_LINKS").is_some_and(|value| value == "0")
		{
			return Ok(());
		}
		let mut objects: Vec<TypeId> = self.tree_arenas.objects.iter().copied().collect();
		objects.sort();
		for ty in objects {
			if self
				.object_fields(ty)
				.iter()
				.any(|field| matches!(field, Repr::Optional(Scalar::Rec(target)) if *target == ty))
			{
				self.tree_arenas.compressed.insert(ty);
			}
		}
		if self.tree_arenas.compressed.is_empty() {
			return Ok(());
		}
		self.object_types.borrow_mut().clear();
		let mut files: Vec<_> = self.other_files.keys().copied().collect();
		files.push(self.file);
		files.sort();
		files.dedup();
		for file in files {
			self.switch_file(file);
			let mut keys: Vec<NodeId> = self.functions.keys().copied().collect();
			keys.sort();
			for key in keys {
				let old = self.functions[&key].clone();
				let name = old.value.get_name().to_string_lossy().into_owned();
				old.value.as_global_value().set_name("");
				let mut function = self.declare_function(key, old.info.clone(), "")?;
				function.value.as_global_value().set_name(&name);
				function.captures = old.captures;
				unsafe { old.value.delete() };
				self.functions.insert(key, function);
			}
		}
		self.switch_file(self.entry);
		Ok(())
	}

	/// The node types of a recursive object or named union whose nodes can
	/// live in a region (its named union, and the object types of its nodes):
	/// finite passive payloads, edges to the same root type, a node fitting a page.
	fn tree_shapes(&self, ty: TypeId) -> Option<(Option<NominalId>, FxHashSet<TypeId>)> {
		let mut shapes = FxHashSet::default();
		let mut recursive = false;
		let (nominal, valid) = match self.types().kind(ty) {
			TypeKind::Object(_) if self.destroy_function(ty).is_none() => {
				shapes.insert(ty);
				let valid = self.arena_type_fits(self.object_type(ty).into())
					&& self.object_fields(ty).iter().all(|r| {
						if matches!(r, Repr::Optional(Scalar::Rec(target)) if *target == ty) {
							recursive = true;
							true
						} else {
							!self.needs_drop(*r)
						}
					});
				(None, valid)
			}
			TypeKind::NamedUnion(n, _) => {
				let count = self.types().sym_list(self.types().nominal(n).members).len();
				let valid = (0..count).all(|i| {
					// a member without data is a constant box, outside every region
					let Some(data) = self.member_data(n, i as u16) else {
						return true;
					};
					if !matches!(self.types().kind(data), TypeKind::Object(_))
						|| self.destroy_function(data).is_some()
						|| !self.arena_type_fits(self.box_type(Some(Repr::Object(data))).into())
					{
						return false;
					}
					shapes.insert(data);
					self.types().fields_of(data).iter().all(|f| {
						let union = self.repr_of(f.ty, 0).is_ok_and(|r| r == Repr::Union);
						if self.tree_value_type(f.ty, ty) {
							let children = matches!(self.types().kind(f.ty), TypeKind::Collection(CollKind::Array, item, _) if item == ty || self.types().rec_target(item) == Some(ty));
							recursive |= union || children;
							union || children
						} else {
							// Direct Union fields are reserved for recursive root
							// edges by arena_payload_slots; unrelated boxes need
							// the general ownership path to release their payload.
							!union && self.arena_payload_type(f.ty)
						}
					})
				});
				(Some(n), valid)
			}
			_ => return None,
		};
		(valid && recursive).then_some((nominal, shapes))
	}

	/// Moves a builder's body into `<name>.arena`, which its recursive calls
	/// use, and makes the function itself enter a region, call the body and
	/// leave the region with the root.
	fn wrap_tree_builder(&mut self, key: NodeId, nominal: Option<NominalId>) -> Result<(), Error> {
		let f = self.functions[&key].clone();
		let name = format!("{}.arena", f.value.get_name().to_string_lossy());
		let body = self.module.add_function(
			&name,
			f.value.get_type(),
			Some(inkwell::module::Linkage::Internal),
		);
		self.runtime.add_attributes(body, &["nounwind"]);
		body.set_call_conventions(FAST_CALLING_CONVENTION);
		self.functions.get_mut(&key).expect("builder").arena_body = Some(body);
		let b = self.context.create_builder();
		b.position_at_end(self.context.append_basic_block(f.value, "region"));
		b.build_call(self.runtime.function("fa_arena_enter")?, &[], "")?;
		let arguments: Vec<BasicMetadataValueEnum> =
			f.value.get_param_iter().map(Into::into).collect();
		let call = b.build_call(body, &arguments, "root")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		let root = call.try_as_basic_value().basic().expect("root");
		let value = if f.throws {
			b.build_extract_value(root.into_struct_value(), 0, "")?
		} else {
			root
		};
		let pointer = match nominal {
			Some(nominal) => self.union_root_pointer(&b, nominal, value)?,
			None => self.tree_root_pointer(&b, f.ret, value)?,
		};
		b.build_call(
			self.runtime.function("fa_arena_leave")?,
			&[pointer.into()],
			"",
		)?;
		b.build_return(Some(&root))?;
		Ok(())
	}

	pub(super) fn tree_root_pointer(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		if matches!(repr, Repr::Rec(_) | Repr::Union) {
			return self.edge_address(b, repr, value);
		}
		let Repr::Object(ty) = repr else {
			unreachable!("an arena root")
		};
		let mut pointer = self.pointer_type().const_null();
		for (index, field) in self.object_fields(ty).iter().enumerate() {
			if matches!(field, Repr::Optional(Scalar::Rec(_))) {
				let edge = b.build_extract_value(value.into_struct_value(), index as u32, "")?;
				let edge = self.edge_address(b, *field, edge)?;
				pointer = b
					.build_select(b.build_is_null(pointer, "")?, edge, pointer, "")?
					.into_pointer_value();
			}
		}
		Ok(pointer)
	}

	/// The box of a union root, or null for a member without data: its
	/// constant box belongs to no region.
	fn union_root_pointer(
		&self,
		b: &Builder<'ctx>,
		nominal: NominalId,
		value: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let pointer = value.into_pointer_value();
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len() as u16;
		let constants: Vec<u64> = (0..count)
			.filter(|&index| self.member_data(nominal, index).is_none())
			.map(|index| self.member_id(nominal, index))
			.collect();
		if constants.is_empty() {
			return Ok(pointer);
		}
		// a throwing builder returns null with its error
		let entry = b.get_insert_block().expect("a block");
		let function = entry.get_parent().expect("a function");
		let present = self.context.append_basic_block(function, "root_present");
		let done = self.context.append_basic_block(function, "root_member");
		b.build_conditional_branch(b.build_is_null(pointer, "")?, done, present)?;
		b.position_at_end(present);
		let i64 = self.context.i64_type();
		let id = b.build_load(i64, pointer, "id")?.into_int_value();
		let mut constant = self.context.bool_type().const_zero();
		for member in constants {
			let equal = b.build_int_compare(
				inkwell::IntPredicate::EQ,
				id,
				i64.const_int(member, false),
				"",
			)?;
			constant = b.build_or(constant, equal, "")?;
		}
		let null = self.pointer_type().const_null();
		let selected = b.build_select(constant, null, pointer, "")?;
		b.build_unconditional_branch(done)?;
		b.position_at_end(done);
		let root = b.build_phi(self.pointer_type(), "root")?;
		root.add_incoming(&[(&null, entry), (&selected, present)]);
		Ok(root.as_basic_value().into_pointer_value())
	}

	pub(super) fn allocate_tree_node(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
	) -> Result<PointerValue<'ctx>, Error> {
		self.allocate_tree_type(b, self.llvm_type(repr))
	}

	pub(super) fn allocate_tree_type(
		&self,
		b: &Builder<'ctx>,
		ty: inkwell::types::BasicTypeEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let size = ty.size_of().expect("a node size");
		let size = b.build_int_cast(size, self.usize, "")?;
		let align = b.build_int_cast(ty.get_alignment(), self.usize, "")?;
		Ok(b.build_call(
			self.runtime.function("fa_arena_alloc")?,
			&[size.into(), align.into()],
			"",
		)?
		.try_as_basic_value()
		.basic()
		.expect("a node")
		.into_pointer_value())
	}
}
