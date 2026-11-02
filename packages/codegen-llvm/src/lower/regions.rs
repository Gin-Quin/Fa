//! Regions: the boxes of recursive values (`Tree?` edges, members of a
//! recursive named union) that a statement allocates are bumped out of a
//! region, released at once when the statement ends, when the compiler
//! proves that none of them outlives the statement (`runtime/fa_runtime.c`,
//! "regions").
//!
//! The proof is by types. A value holding a box has a *recursive type* (a
//! type whose value can hold a box: one with a recursive edge or a member of
//! a recursive named union). A statement is a region when:
//!
//! - what it leaves behind has no recursive type: the local it declares, the
//!   place it assigns, the value of a block it ends;
//! - it names no value of a recursive type declared outside of it (so it
//!   neither moves an older box into a new value, nor stores a new box into
//!   an older value, through itself or through the functions it calls,
//!   which only reach the values they are given);
//! - it has no lambda, no `use`, no `return` nor `break` with a value;
//! - and the program has no mutable value of a recursive type that a
//!   function reads from outside of it, no error type holding a recursive
//!   type, no thread nor task, and is a single file.
//!
//! The functions it calls allocate their boxes in the region too (it is the
//! thread's current region): every box of the statement dies with it. A box
//! freed early is left in its region (`fa_heap_free` ignores it), and the
//! drop of a recursive object whose fields own nothing but its edges stops
//! at a box of a region: the whole value is in it.

use fa_analyzer::analysis::Callee;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{NominalKind, TypeKind};
use inkwell::builder::Builder;
use inkwell::values::{IntValue, PointerValue};
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{Repr, Scalar};
use crate::target::Target;

/// What the region analysis computed once per program.
#[derive(Default)]
pub(crate) struct Regions {
	/// Whether the program may use regions at all.
	enabled: std::cell::OnceCell<bool>,
	/// Whether a type may hold a box (with the types the analysis doesn't
	/// look into, or without them), by type.
	recursive: std::cell::RefCell<FxHashMap<(TypeId, bool), bool>>,
	/// Whether a function may allocate a box, by its declaration node.
	allocating: std::cell::RefCell<FxHashMap<NodeId, bool>>,
	/// The recursive named unions whose boxes may be in a region, and
	/// whether the data of their members owns nothing but such boxes.
	nominals: std::cell::RefCell<FxHashMap<fa_analyzer::ids::NominalId, bool>>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether the program may use regions (see the module documentation).
	pub(crate) fn regions_enabled(&self) -> bool {
		*self
			.regions
			.enabled
			.get_or_init(|| self.compute_regions_enabled())
	}

	fn compute_regions_enabled(&self) -> bool {
		// a leak-checked native build allocates with `malloc`: no page headers
		if self.target == Target::Native && self.leak_check {
			return false;
		}
		if !self.other_files.is_empty() {
			return false;
		}
		let types = self.types();
		let functions: Vec<(usize, usize)> = self
			.analysis
			.functions
			.values()
			.map(|info| {
				let span = self.tree.spans[info.lambda.index()];
				(span.start, span.end)
			})
			.collect();
		let innermost = |index: usize| -> Option<(usize, usize)> {
			let span = self.tree.spans[index];
			functions
				.iter()
				.filter(|(start, end)| *start <= span.start && span.end <= *end)
				.min_by_key(|(start, end)| end - start)
				.copied()
		};
		let mut any_recursive = false;
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			let ty = self.type_of(node);
			match types.kind(ty) {
				TypeKind::Sync(..) | TypeKind::Task(..) => return false,
				TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _)
					if types.nominal(nominal).kind == NominalKind::Errors
						&& self.is_recursive_type(ty) =>
				{
					return false;
				}
				_ => {}
			}
			// threads, and the observers of reactivity, which keep what they
			// capture
			if matches!(
				self.view(node),
				View::For {
					parallel: Some(_),
					..
				}
			) || matches!(
				self.tree.nodes[index],
				fa_parser::Node::Reactive { .. }
					| fa_parser::Node::Derived { .. }
					| fa_parser::Node::Watch { .. }
			) {
				return false;
			}
			if !any_recursive && self.holds_boxes(ty) {
				any_recursive = true;
			}
			// a mutable value of a recursive type read by a function from
			// outside of it
			if let Resolution::Local(symbol) = self.analysis.resolution(node) {
				let declared = self.analysis.symbols.symbol(symbol);
				if matches!(
					declared.kind,
					SymbolKind::Mutable
						| SymbolKind::Static
						| SymbolKind::MutableUse
						| SymbolKind::Reactive
				) && self.is_recursive_type(declared.ty)
					&& innermost(index) != innermost(declared.decl_node.index())
				{
					return false;
				}
			}
		}
		if self
			.analysis
			.functions
			.values()
			.filter_map(|info| info.sig)
			.any(|sig| types.sig(sig).is_async)
		{
			return false;
		}
		any_recursive
	}

	/// Whether a value of this type may hold a box of a recursive value (or
	/// the type is one the analysis doesn't look into: functions, generics…).
	pub(crate) fn is_recursive_type(&self, ty: TypeId) -> bool {
		self.walk_memo(ty, false)
	}

	/// Whether a value of this type holds a box of a recursive value, the
	/// types the analysis doesn't look into counting as not: what tells that
	/// code builds boxes.
	fn holds_boxes(&self, ty: TypeId) -> bool {
		!ty.is_unset() && self.walk_memo(ty, true)
	}

	fn walk_memo(&self, ty: TypeId, precise: bool) -> bool {
		if let Some(&known) = self.regions.recursive.borrow().get(&(ty, precise)) {
			return known;
		}
		let mut nominals = Vec::new();
		let result = self.recursive_walk(ty, &mut nominals, 0, precise);
		self.regions
			.recursive
			.borrow_mut()
			.insert((ty, precise), result);
		result
	}

	fn recursive_walk(
		&self,
		ty: TypeId,
		nominals: &mut Vec<fa_analyzer::ids::NominalId>,
		depth: u32,
		precise: bool,
	) -> bool {
		if depth > 24 {
			return !precise;
		}
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Rec(_) => true,
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				if nominals.contains(&nominal) {
					return true;
				}
				let info = types.nominal(nominal);
				if !matches!(info.kind, NominalKind::Union | NominalKind::Errors) {
					return false;
				}
				nominals.push(nominal);
				let count = types.sym_list(info.members).len();
				let found = (0..count).any(|index| {
					self.member_data(nominal, index as u16)
						.is_some_and(|data| self.recursive_walk(data, nominals, depth + 1, precise))
				});
				nominals.pop();
				found
			}
			TypeKind::Object(_) => types
				.fields_of(ty)
				.iter()
				.any(|field| self.recursive_walk(field.ty, nominals, depth + 1, precise)),
			TypeKind::Tuple(list) | TypeKind::Union(list) => types
				.list_items(list)
				.iter()
				.any(|&item| self.recursive_walk(item, nominals, depth + 1, precise)),
			TypeKind::Collection(_, item, key) => {
				self.recursive_walk(item, nominals, depth + 1, precise)
					|| (key != TypeId::NONE
						&& self.recursive_walk(key, nominals, depth + 1, precise))
			}
			TypeKind::Function(_)
			| TypeKind::Task(..)
			| TypeKind::Sync(..)
			| TypeKind::Param(_)
			| TypeKind::ImplicitParam(..)
			| TypeKind::Iterator(..)
			| TypeKind::Ref(..)
			| TypeKind::Pointer(_)
			| TypeKind::Subscription
			| TypeKind::Unknown
			| TypeKind::Fields(_) => !precise,
			_ => false,
		}
	}

	/// Whether a named union holds itself (or another recursive value) in
	/// the data of one of its members: its boxes may go to a region.
	pub(crate) fn is_recursive_nominal(&self, nominal: fa_analyzer::ids::NominalId) -> bool {
		let types = self.types();
		let info = types.nominal(nominal);
		if info.kind != NominalKind::Union {
			return false;
		}
		let count = types.sym_list(info.members).len();
		(0..count).any(|index| {
			self.member_data(nominal, index as u16)
				.is_some_and(|data| self.is_recursive_type(data))
		})
	}

	/// Notes that the boxes of a recursive named union may be in a region
	/// (`build_member`), for `fa.copy_box` and `fa.drop_box`.
	pub(crate) fn note_region_nominal(&self, nominal: fa_analyzer::ids::NominalId) {
		if self.regions.nominals.borrow().contains_key(&nominal) {
			return;
		}
		let types = self.types();
		let count = types.sym_list(types.nominal(nominal).members).len();
		let pure = (0..count).all(|index| {
			let Some(data) = self.member_data(nominal, index as u16) else {
				return true;
			};
			let fields: Vec<TypeId> = match types.kind(data) {
				TypeKind::Object(_) => types.fields_of(data).iter().map(|field| field.ty).collect(),
				_ => vec![data],
			};
			fields.into_iter().all(|field| match types.kind(field) {
				TypeKind::NamedUnion(other, _) | TypeKind::Member(other, _) => other == nominal,
				_ => match self.repr_of(field, 0) {
					Ok(Repr::Rec(target) | Repr::Optional(Scalar::Rec(target))) => {
						self.drop_stops_at_region(target)
					}
					Ok(repr) => !self.needs_drop(repr),
					Err(_) => false,
				},
			})
		});
		self.regions.nominals.borrow_mut().insert(nominal, pure);
	}

	/// The ids of the members whose boxes may be in a region.
	pub(crate) fn region_member_ids(&self) -> Vec<u64> {
		let nominals = self.regions.nominals.borrow();
		self.union_ids
			.iter()
			.filter(|((nominal, _), _)| nominals.contains_key(nominal))
			.map(|(_, &id)| id)
			.collect()
	}

	/// Whether `fa.drop_box` can stop at a box of a region: every box that
	/// can be in one owns nothing but boxes of the region.
	pub(crate) fn drop_box_stops_at_region(&self) -> bool {
		let nominals = self.regions.nominals.borrow();
		self.regions_enabled() && !nominals.is_empty() && nominals.values().all(|&pure| pure)
	}

	/// Whether a statement is a region (see the module documentation);
	/// `kept`: its value is the value of its block.
	pub(crate) fn is_region_statement(&self, statement: NodeId, kept: bool) -> bool {
		if !self.regions_enabled() {
			return false;
		}
		let span = self.tree.spans[statement.index()];
		if kept && self.is_recursive_type(self.type_of(statement)) {
			return false;
		}
		let mut allocates = false;
		let mut stack = vec![statement.index()];
		while let Some(current) = stack.pop() {
			let node = NodeId::new(current);
			match self.view(node) {
				View::Lambda { .. }
				| View::Use { .. }
				| View::Reuse { .. }
				| View::Provide { .. }
				| View::Return(Some(_))
				| View::Break(Some(_)) => return false,
				_ => {}
			}
			let ty = self.type_of(node);
			let recursive = !ty.is_unset() && self.is_recursive_type(ty);
			match self.analysis.resolution(node) {
				Resolution::Local(symbol) => {
					let declared = self.analysis.symbols.symbol(symbol);
					let named_value = !matches!(
						declared.kind,
						SymbolKind::Function
							| SymbolKind::Type | SymbolKind::Namespace
							| SymbolKind::TypeParameter
							| SymbolKind::Test
					);
					let outside = {
						let at = self.tree.spans[declared.decl_node.index()];
						at.start < span.start || at.end > span.end
					};
					if named_value && (recursive || self.is_recursive_type(declared.ty)) {
						// a declaration of the statement that outlives it, or
						// a value from outside
						if outside || declared.decl_node == statement {
							return false;
						}
					}
				}
				Resolution::External(_) if recursive => return false,
				_ => {}
			}
			if self.holds_boxes(ty) {
				allocates = true;
			}
			if let Some(info) = self.analysis.call(node) {
				match info.callee {
					Callee::Function(function) => {
						allocates = allocates || self.function_allocates(function, 0);
					}
					Callee::Value => allocates = true,
					Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_) => {}
				}
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		// the local a declaration statement declares outlives it
		if let Some(symbol) = self.analysis.symbols.symbol_of_decl(statement)
			&& self.is_recursive_type(self.analysis.symbols.symbol(symbol).ty)
		{
			return false;
		}
		allocates
	}

	/// Whether a function of this file (its declaration node) may allocate a
	/// box, itself or through the functions it calls.
	fn function_allocates(&self, function: NodeId, depth: u32) -> bool {
		if let Some(&known) = self.regions.allocating.borrow().get(&function) {
			return known;
		}
		if depth > 64 {
			return true;
		}
		// a cycle counts as not allocating until shown otherwise
		self.regions.allocating.borrow_mut().insert(function, false);
		let info = self
			.analysis
			.functions
			.values()
			.find(|info| info.declaration == Some(function) || info.lambda == function);
		let result = match info {
			None => true,
			Some(info) => {
				let mut found = false;
				let mut stack = vec![info.body.index()];
				while let Some(current) = stack.pop() {
					let node = NodeId::new(current);
					let ty = self.type_of(node);
					if self.holds_boxes(ty) {
						found = true;
						break;
					}
					if let Some(call) = self.analysis.call(node) {
						let allocates = match call.callee {
							Callee::Function(callee) => self.function_allocates(callee, depth + 1),
							Callee::Value => true,
							_ => false,
						};
						if allocates {
							found = true;
							break;
						}
					}
					self.tree.nodes[current].for_each_child(|child| stack.push(child));
				}
				found
			}
		};
		self.regions
			.allocating
			.borrow_mut()
			.insert(function, result);
		result
	}

	/// Starts the region of a statement: its end is a cleanup of the
	/// statement, run however it ends.
	pub(crate) fn enter_region(&self) -> Result<(), Error> {
		let pointer = self.pointer_type();
		let mark_type = self
			.context
			.struct_type(&[pointer.into(), pointer.into(), pointer.into()], false);
		let mark = self.function_slot(&self.builder, mark_type.into(), "region")?;
		self.call_runtime("fa_region_enter", &[mark.into()])?;
		self.cleanups
			.borrow_mut()
			.push(super::ownership::Cleanup::Region(mark));
		Ok(())
	}

	/// Ends a region (the cleanup of [`Lowering::enter_region`]).
	pub(crate) fn exit_region(&self, mark: PointerValue<'ctx>) -> Result<(), Error> {
		self.call_runtime("fa_region_exit", &[mark.into()])
	}

	/// A block for a box of a recursive value: from the current region if
	/// there is one, else from the heap.
	pub(crate) fn allocate_box(
		&self,
		builder: &Builder<'ctx>,
		size: IntValue<'ctx>,
		align: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		let size = builder.build_int_cast(size, self.usize, "")?;
		let align = self.usize.const_int(align, false);
		let alloc = self.runtime.alloc()?;
		if !self.regions_enabled() {
			return Ok(builder
				.build_call(alloc, &[size.into(), align.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("a block")
				.into_pointer_value());
		}
		let region = self.runtime.function("fa_region_alloc")?;
		let bumped = builder
			.build_call(region, &[size.into(), align.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a block")
			.into_pointer_value();
		let function = builder
			.get_insert_block()
			.and_then(|block| block.get_parent())
			.expect("a function");
		let start = builder.get_insert_block().expect("a block");
		let heap = self.context.append_basic_block(function, "box_heap");
		let done = self.context.append_basic_block(function, "box_allocated");
		let missing = builder.build_is_null(bumped, "")?;
		builder.build_conditional_branch(missing, heap, done)?;
		builder.position_at_end(heap);
		let allocated = builder
			.build_call(alloc, &[size.into(), align.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a block")
			.into_pointer_value();
		let heap_end = builder.get_insert_block().expect("a block");
		builder.build_unconditional_branch(done)?;
		builder.position_at_end(done);
		let phi = builder.build_phi(self.pointer_type(), "box")?;
		phi.add_incoming(&[(&bumped, start), (&allocated, heap_end)]);
		Ok(phi.as_basic_value().into_pointer_value())
	}

	/// Whether the drop of a box of this recursive object can stop at a box
	/// of a region: its fields own nothing but edges to the same type.
	pub(crate) fn drop_stops_at_region(&self, target: TypeId) -> bool {
		if !self.regions_enabled() {
			return false;
		}
		let Ok(Repr::Object(object)) = self.repr_of(target, 0) else {
			return false;
		};
		self.object_fields(object)
			.into_iter()
			.all(|field| match field {
				Repr::Rec(edge) | Repr::Optional(Scalar::Rec(edge)) => edge == target,
				other => !self.needs_drop(other),
			})
	}

	/// `fa_in_region(box) != 0`, at `builder`.
	pub(crate) fn in_region(
		&self,
		builder: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let check = self.runtime.function("fa_in_region")?;
		let result = builder
			.build_call(check, &[pointer.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a flag")
			.into_int_value();
		Ok(builder.build_int_compare(
			inkwell::IntPredicate::NE,
			result,
			result.get_type().const_zero(),
			"in_region",
		)?)
	}
}
