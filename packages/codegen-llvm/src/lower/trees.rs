//! Regions for recursively built objects and named unions. The proof is
//! closed: one file, no generic or asynchronous code, finite passive payloads
//! and edges to the same root type. Every constructor belongs to a root builder;
//! roots cannot escape through captures, fields, containers or global writes.
//! Node fields stay immutable, while builders may replace their own root locals.
//! Union builders can throw; their wrapper frees partial trees on failure.
//!
//! Ordinary nodes need no destruction walk; an owning string or collection adds one
//! cleanup record to the region. Copies outside a builder get their own region.
//! Other recursive shapes retain the general ownership implementation.

use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, PathRoot, TypeKind};
use inkwell::builder::Builder;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue};
use rustc_hash::FxHashSet;

use super::values::{Repr, Scalar};
use super::{FAST_CALLING_CONVENTION, Lowering};
use crate::error::Error;

#[derive(Default)]
pub(super) struct TreeArenas {
	objects: FxHashSet<TypeId>,
	unions: FxHashSet<NominalId>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Arena nodes must fit one runtime page, including FaArena's five-pointer
	/// header (rounded to 16 bytes) and the allocator's eight-byte size rounding.
	fn arena_type_fits(&self, ty: inkwell::types::BasicTypeEnum<'ctx>) -> bool {
		let layout = self.module.get_data_layout();
		let data =
			inkwell::targets::TargetData::create(layout.as_str().to_str().expect("a data layout"));
		let header = (5 * u64::from(self.usize.get_bit_width() / 8)).next_multiple_of(16);
		let align = u64::from(data.get_abi_alignment(&ty));
		let offset = header.next_multiple_of(align);
		let size = data.get_abi_size(&ty);
		const PAGE_SIZE: u64 = 65536;
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
	) -> Result<Vec<(PointerValue<'ctx>, Repr)>, Error> {
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
		let slots = self.arena_payload_slots(b, node, layout, fields)?;
		if slots.is_empty() {
			return Ok(());
		}
		let mut owning = self.context.bool_type().const_zero();
		for (slot, repr) in slots {
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
				for (slot, repr) in self.arena_payload_slots(&cleanup, node, layout, fields)? {
					let value = cleanup.build_load(self.llvm_type(repr), slot, "")?;
					self.call_drop(&cleanup, repr, value)?;
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

	/// Follow reference bindings as well as field/index syntax: a helper
	/// mutating `use label = leaf.text` still mutates the tree's payload.
	fn tree_place_contains(&self, node: NodeId, root: TypeId, shapes: &FxHashSet<TypeId>) -> bool {
		let mut seen = FxHashSet::default();
		let mut stack = vec![node.index()];
		while let Some(at) = stack.pop() {
			if !seen.insert(at) {
				continue;
			}
			let node = NodeId::new(at);
			let ty = self.type_of(node);
			if shapes.contains(&ty) || self.tree_value_type(ty, root) {
				return true;
			}
			if let Resolution::Local(symbol) = self.analysis.resolution(node) {
				let symbol = self.analysis.symbols.symbol(symbol);
				if matches!(symbol.kind, SymbolKind::Use | SymbolKind::MutableUse) {
					stack.push(symbol.decl_node.index());
				}
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		false
	}

	fn call_writes_tree(&self, node: NodeId, root: TypeId, shapes: &FxHashSet<TypeId>) -> bool {
		let Some(call) = self.analysis.call(node) else {
			return false;
		};
		let sig = match call.callee {
			Callee::Function(decl) => self.analysis.functions.get(&decl).and_then(|f| f.sig),
			_ => match self.view(node) {
				View::Call { callee, .. } => match self.types().kind(self.type_of(callee)) {
					TypeKind::Function(sig) => Some(sig),
					_ => None,
				},
				_ => None,
			},
		};
		let Some(sig) = sig else { return false };
		let (mutates, invalidates) = self.types().effects(self.types().sig(sig).effects);
		mutates.iter().chain(invalidates).any(|path| {
			let (path, _) = self.types().path_data(*path);
			let index = match path.root {
				PathRoot::Param(i) => i as usize,
				PathRoot::SelfParam => 0,
				_ => return false,
			};
			match call.arguments.get(index) {
				Some(Argument::Given(value) | Argument::Receiver(value)) => {
					self.tree_place_contains(*value, root, shapes)
				}
				_ => false,
			}
		})
	}

	pub(super) fn prepare_tree_arenas(&mut self) -> Result<(), Error> {
		if !self.other_files.is_empty()
			|| !self.analysis.extern_calls.is_empty()
			|| self.tasks.asynchronous
			|| self
				.analysis
				.functions
				.values()
				.any(|f| !f.generics.is_empty())
		{
			return Ok(());
		}
		let candidates: FxHashSet<TypeId> =
			self.analysis.type_decls.values().map(|d| d.ty).collect();
		for ty in candidates {
			let mut shapes = FxHashSet::default();
			let mut nominal = None;
			let mut recursive = false;
			let valid = match self.types().kind(ty) {
				TypeKind::Object(_) if self.destroy_function(ty).is_none() => {
					shapes.insert(ty);
					self.arena_type_fits(self.object_type(ty).into())
						&& self.object_fields(ty).iter().all(|r| {
							if matches!(r, Repr::Optional(Scalar::Rec(target)) if *target == ty) {
								recursive = true;
								true
							} else {
								!self.needs_drop(*r)
							}
						})
				}
				TypeKind::NamedUnion(n, _) => {
					nominal = Some(n);
					(0..self.types().sym_list(self.types().nominal(n).members).len()).all(|i| {
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
							if self.tree_value_type(f.ty, ty)
								&& self.repr_of(f.ty, 0).is_ok_and(|r| r == Repr::Union)
							{
								recursive = true;
								true
							} else {
								// Direct Union fields are reserved for recursive root
								// edges by arena_payload_slots; unrelated boxes need
								// the general ownership path to release their payload.
								!self.tree_value_type(f.ty, ty)
									&& self.repr_of(f.ty, 0).is_ok_and(|r| r != Repr::Union)
									&& self.arena_payload_type(f.ty)
							}
						})
					})
				}
				_ => false,
			};
			if !valid || !recursive {
				continue;
			}
			let root_repr = self.repr_of(ty, 0)?;

			// No secondary container/closure may retain a root created inside a
			// builder. Optional roots and function signatures are ordinary uses.
			let embedded = self
				.analysis
				.types
				.iter()
				.copied()
				.chain(self.analysis.coercions.values().copied())
				.chain(self.analysis.symbols.symbols().iter().map(|s| s.ty))
				.any(|at| match self.types().kind(at) {
					TypeKind::Object(_) if !shapes.contains(&at) => self
						.types()
						.fields_of(at)
						.iter()
						.any(|f| self.tree_value_type(f.ty, ty)),
					TypeKind::Collection(_, item, key) => {
						self.tree_value_type(item, ty) || self.tree_value_type(key, ty)
					}
					TypeKind::Tuple(list) => self
						.types()
						.list_items(list)
						.iter()
						.any(|t| self.tree_value_type(*t, ty)),
					TypeKind::Pointer(item) | TypeKind::Sync(_, item) | TypeKind::Task(item, _) => {
						self.tree_value_type(item, ty)
					}
					TypeKind::Member(n, index) if Some(n) != nominal => {
						self.tree_value_type(self.types().payload(n, index), ty)
					}
					TypeKind::NamedUnion(n, _) if Some(n) != nominal => {
						(0..self.types().sym_list(self.types().nominal(n).members).len())
							.any(|i| self.tree_value_type(self.types().payload(n, i as u16), ty))
					}
					_ => false,
				});
			let captured = self.analysis.functions.values().any(|f| {
				f.captures
					.iter()
					.any(|s| self.tree_value_type(self.analysis.symbols.symbol(*s).ty, ty))
			});

			let builders: Vec<NodeId> =
				self.functions
					.iter()
					.filter_map(|(key, f)| {
						if f.ret != root_repr
							|| !f.info.sig.is_some_and(|sig| {
								self.tree_value_type(self.types().sig(sig).ret, ty)
							}) || (f.throws && root_repr != Repr::Union)
							|| f.lambda || f.params.iter().any(|(_, d)| *d)
							|| f.info.params.iter().enumerate().any(|(i, p)| {
								f.by_pointer[i]
									&& !f.borrowed[i] && self
									.tree_value_type(self.analysis.symbols.symbol(p.symbol).ty, ty)
							}) {
							return None;
						}
						let mut stack = vec![f.info.body.index()];
						while let Some(at) = stack.pop() {
							if matches!(
								self.view(NodeId::new(at)),
								View::Function { .. } | View::Lambda { .. }
							) {
								return None;
							}
							self.tree.nodes[at].for_each_child(|child| stack.push(child));
						}
						Some(*key)
					})
					.collect();
			let written = (0..self.tree.nodes.len()).any(|at| {
				if self.call_writes_tree(NodeId::new(at), ty, &shapes) {
					return true;
				}
				// Parameter/field syntax and binding initializers have a value
				// type; only evaluated assignments have the statement type.
				if self.type_of(NodeId::new(at)) != TypeId::VOID {
					return false;
				}
				let target = match self.view(NodeId::new(at)) {
					View::Assignment { target, .. }
					| View::CompoundAssignment { target, .. }
					| View::UpdateFields { target, .. } => target,
					_ => return false,
				};
				// Fields, function signatures and named arguments also use
				// assignment syntax; only actual program places are writes.
				if !self.is_place(target) {
					return false;
				}

				if matches!(self.view(target), View::Name(_))
					&& let Resolution::Local(symbol) = self.analysis.resolution(target)
					&& self.analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable
				{
					let scope = self.analysis.symbols.symbol(symbol).scope;
					if self
						.owner_of_scope(scope)
						.is_some_and(|owner| builders.contains(&owner))
					{
						return false;
					}
				}

				self.tree_place_contains(target, ty, &shapes)
			});

			if embedded || captured || written {
				continue;
			}
			let mut found = false;
			let closed = self
				.analysis
				.objects
				.iter()
				.filter(|(_, o)| match (nominal, o.member) {
					(Some(n), Some(member)) => {
						matches!(self.types().kind(member), TypeKind::Member(m, _) if m == n)
					}
					(None, _) => o.ty == ty,
					_ => false,
				})
				.all(|(node, _)| {
					found = true;
					let span = self.tree.spans[node.index()];
					// The nearest function body containing this constructor must be
					// one of the proven builders (not a nested lambda or a default).
					self.functions
						.iter()
						.filter(|(_, f)| {
							let body = self.tree.spans[f.info.body.index()];
							body.start <= span.start && body.end >= span.end
						})
						.min_by_key(|(_, f)| {
							let body = self.tree.spans[f.info.body.index()];
							body.end - body.start
						})
						.is_some_and(|(key, _)| builders.contains(key))
				});
			if !found || !closed {
				continue;
			}
			if let Some(nominal) = nominal {
				self.tree_arenas.unions.insert(nominal);
			} else {
				self.tree_arenas.objects.insert(ty);
			}
			for key in builders {
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
			}
		}
		Ok(())
	}

	pub(super) fn tree_root_pointer(
		&self,
		b: &Builder<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		if matches!(repr, Repr::Rec(_) | Repr::Union) {
			return Ok(value.into_pointer_value());
		}
		let Repr::Object(ty) = repr else {
			unreachable!("an arena root")
		};
		let mut pointer = self.pointer_type().const_null();
		for (index, field) in self.object_fields(ty).iter().enumerate() {
			if matches!(field, Repr::Optional(Scalar::Rec(_))) {
				let edge = b
					.build_extract_value(value.into_struct_value(), index as u32, "")?
					.into_pointer_value();
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
		Ok(b.build_select(constant, null, pointer, "")?
			.into_pointer_value())
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
