//! References: parameters a function changes, `use`, `mutable use` and
//! `reuse`, and `for` variables over collections (advanced brief §4, §6.7,
//! the memory management page: parameters are references).
//!
//! A reference is a pointer to its place, with no runtime tracking: the
//! analyzer proved it valid wherever it is used.
//!
//! - A parameter that is the root of a `mutates` or `invalidates` path of
//!   its function is passed by pointer to the caller's place. When the
//!   place has another object type than the parameter (a structural view,
//!   `rename(player)` for `(scored: { name: String })`), the caller passes a
//!   copy of the fields the parameter has, and writes them back after the
//!   call: the alias rule makes this equivalent to a pointer into the place.
//! - `use x = place` binds `x` to the address of the place, computed once;
//!   `mutable use x` keeps that address in a slot of its own, which
//!   `reuse x = place` changes.

use fa_analyzer::analysis::Argument;
use fa_analyzer::ids::{NodeId, PathId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, PathRoot, PathStep, TypeKind};
use inkwell::values::PointerValue;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

/// What a call with a parameter passed by pointer does after the call.
pub(crate) enum WriteBack<'ctx> {
	/// The parameter was a view of `place`: its fields written back by name.
	Fields { place: Slot<'ctx>, view: Slot<'ctx> },
	/// The argument wasn't a place: the changed value is a temporary of the
	/// statement.
	Temporary(Slot<'ctx>),
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Determine borrowed-result expressions before lowering their function.
	/// Nested lambdas own their return ABI and are visited separately.
	pub(super) fn prepare_reference_returns(&mut self, body: NodeId) {
		let mut roots = rustc_hash::FxHashSet::default();
		let tail = match self.view(body) {
			View::Block { statements } => statements.last(),
			_ => Some(body),
		};
		if let Some(tail) = tail
			&& !matches!(self.view(tail), View::Return(_) | View::Throw(_))
		{
			roots.insert(tail);
		}
		let mut pending = vec![body];
		while let Some(node) = pending.pop() {
			match self.view(node) {
				View::Return(Some(value)) => {
					roots.insert(value);
				}
				View::Lambda { .. } | View::Function { .. } => continue,
				_ => pending.extend(fa_analyzer::syntax::children(self.tree, node)),
			}
		}
		let mut pending: Vec<_> = roots.into_iter().collect();
		let mut values = rustc_hash::FxHashSet::default();
		while let Some(node) = pending.pop() {
			values.insert(node);
			match self.view(node) {
				View::Group(inner) | View::Try(inner) | View::Do(inner) => pending.push(inner),
				View::Block { statements } => pending.extend(statements.last()),
				View::If {
					then_body,
					else_body,
					..
				} => {
					pending.push(then_body);
					pending.extend(else_body);
				}
				View::When { arms, .. } => pending.extend(arms.iter().map(|arm| arm.value)),
				View::Catch {
					expression,
					handler,
					..
				} => pending.extend([expression, handler]),
				_ => {}
			}
		}
		self.frame_mut().reference_returns = values;
	}

	/// The declared result of a user call, separated from its address ABI.
	pub(super) fn user_reference_result(
		&self,
		node: NodeId,
	) -> Option<(fa_analyzer::ids::TypeId, bool)> {
		let View::Call { callee, .. } = self.view(node) else {
			return None;
		};
		let fa_analyzer::types::TypeKind::Function(sig) = self.types().kind(self.type_of(callee))
		else {
			return None;
		};
		let sig = self.types().sig(sig);
		sig.reference?;
		let result = self.type_of(node);
		let members = self.types().union_members(&result);
		let optional = members.contains(&fa_analyzer::ids::TypeId::NONE);
		Some((result, optional))
	}

	/// Locate the collection's type without evaluating any argument again.
	fn reference_owner_type(&self, path: PathId, roots: &[TypeId]) -> Option<TypeId> {
		let (data, steps) = self.types().path_data(path);
		let mut ty = match data.root {
			PathRoot::Param(index) => *roots.get(index as usize)?,
			PathRoot::SelfParam => *roots.first()?,
			PathRoot::ModuleValue(key) => {
				let (file, symbol) = self.session.external(Resolution::External(key))?;
				self.session.analysis(file).symbols.symbol(symbol).ty
			}
			_ => return None,
		};
		for step in steps {
			ty = self.types().rec_target(ty).unwrap_or(ty);
			ty = match step {
				PathStep::Field(name) | PathStep::Private(name) => {
					self.types()
						.fields_of(ty)
						.iter()
						.find(|field| self.same_field_name(field.name, *name))?
						.ty
				}
				_ => return None,
			};
		}
		Some(self.concrete_type(ty))
	}

	/// Columnar elements keep their row locator, not an object-layout address.
	pub(super) fn reference_item_repr(&self, path: PathId, roots: &[TypeId], repr: Repr) -> Repr {
		if let Repr::Object(object) = repr
			&& let Some(owner) = self.reference_owner_type(path, roots)
			&& let TypeKind::Collection(CollKind::Columnar | CollKind::GrowingColumnar, item, _) =
				self.types().kind(owner)
			&& matches!(self.repr_of(owner, 0), Ok(Repr::Columnar(_)))
			&& self.same_concrete_type(self.types().rec_target(item).unwrap_or(item), object)
		{
			Repr::Row(object)
		} else {
			repr
		}
	}

	pub(super) fn reference_result_repr(
		&self,
		node: NodeId,
		element: TypeId,
	) -> Result<Repr, Error> {
		let repr = match self.repr_of(element, node.index())? {
			// Keep the complete item representation of an optional reference,
			// including every member of an anonymous union. Selecting one
			// member would read or write the slot with the wrong layout.
			Repr::Optional(item) => item.repr(),
			repr => repr,
		};
		let View::Call { callee, .. } = self.view(node) else {
			return Ok(repr);
		};
		let TypeKind::Function(sig) = self.types().kind(self.type_of(callee)) else {
			return Ok(repr);
		};
		let Some(path) = self.types().sig(sig).reference else {
			return Ok(repr);
		};
		let Some(call) = self.analysis.call(node) else {
			return Ok(repr);
		};
		let roots: Vec<_> = call
			.arguments
			.iter()
			.map(|argument| match argument {
				Argument::Given(node) => self.type_of(*node),
				Argument::Receiver(callee) => match self.view(*callee) {
					View::Access { target, members } => self.type_of(if members.len() == 1 {
						target
					} else {
						members.get(members.len() - 2).unwrap_or(target)
					}),
					_ => TypeId::UNKNOWN,
				},
				_ => TypeId::UNKNOWN,
			})
			.collect();
		Ok(self.reference_item_repr(path, &roots, repr))
	}

	/// Evaluate a result as a borrowed address, without a copy, temporary
	/// lifetime extension or a second evaluation of an insertion.
	pub(super) fn borrowed_result(&mut self, value: NodeId) -> Result<Val<'ctx>, Error> {
		if self.reference_control(value) {
			return self.expr_inner(value);
		}
		let mut node = value;
		while let View::Group(inner) | View::Try(inner) = self.view(node) {
			node = inner;
		}
		if matches!(self.view(node), View::None) {
			return Ok(Val::new(Repr::Pointer, self.pointer_type().const_null()));
		}
		if self.user_reference_result(node).is_some() {
			return self.call_present(node);
		}
		let slot = match self.collection_reference(node)? {
			Some((slot, _)) => slot,
			None => self.reference_place(node)?,
		};
		Ok(Val::new(Repr::Pointer, slot.pointer))
	}

	/// Structured reference values merge addresses, rather than loaded data.
	pub(super) fn reference_control(&self, node: NodeId) -> bool {
		self.frame().reference_returns.contains(&node)
			&& matches!(
				self.view(node),
				View::Group(_)
					| View::Try(_) | View::Do(_)
					| View::Block { .. }
					| View::If { .. }
					| View::When { .. }
					| View::Catch { .. }
			)
	}

	/// Materialize a borrowed call for an ordinary read; `use` takes the
	/// same returned address directly through `collection_reference`.
	pub(super) fn read_reference_result(
		&mut self,
		node: NodeId,
		raw: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let Some((element, optional)) = self.user_reference_result(node) else {
			return Ok(raw);
		};
		if raw.repr == Repr::Never {
			return Ok(raw);
		}
		let pointer = raw
			.value
			.expect("a reference result address")
			.into_pointer_value();
		let repr = self.reference_result_repr(node, element)?;
		if optional {
			self.optional_element(pointer, repr, node)
		} else {
			self.load(Slot { pointer, repr })
		}
	}

	/// Whether a node designates a place (a local, a field, an item).
	pub(crate) fn is_place(&self, node: NodeId) -> bool {
		let stored = |kind| {
			!matches!(
				kind,
				SymbolKind::Static
					| SymbolKind::Function
					| SymbolKind::Type
					| SymbolKind::TypeParameter
					| SymbolKind::Namespace
					| SymbolKind::Test
			)
		};
		match self.view(node) {
			View::Try(inner) | View::Group(inner) => self.is_place(inner),
			View::Call { .. } if self.user_reference_result(node).is_some() => true,
			View::Name(_) => match self.analysis.resolution(node) {
				Resolution::Local(symbol) => stored(self.analysis.symbols.symbol(symbol).kind),
				resolution @ Resolution::External(..) => match self.external(resolution) {
					Some(super::program::Reached::Value(file, symbol)) => {
						stored(self.session.analysis(file).symbols.symbol(symbol).kind)
					}
					Some(super::program::Reached::RemoteValue(_)) => true,
					_ => false,
				},
				_ => false,
			},
			View::Access { target, members } => {
				// A type's member is a constructed value, not a runtime slot.
				if members
					.last()
					.is_some_and(|m| matches!(self.analysis.resolution(m), Resolution::Member(..)))
				{
					return false;
				}
				let list: Vec<NodeId> = members.iter().collect();
				match self.namespace_reach(target, &list) {
					Some((super::program::Reached::Value(file, symbol), _)) => {
						stored(self.session.analysis(file).symbols.symbol(symbol).kind)
					}
					Some((super::program::Reached::RemoteValue(_), _)) => true,
					Some(_) => false,
					None => self.is_place(target),
				}
			}

			View::Index { target, .. } => self.is_place(target),
			_ => false,
		}
	}

	/// The pointer given for a parameter passed by pointer: the argument's
	/// place (`node`, or the receiver of the method call `call`), a view of
	/// it of the parameter's representation, or a temporary.
	pub(crate) fn argument_place(
		&mut self,
		call: NodeId,
		node: Option<NodeId>,
		repr: Repr,
		borrowed: bool,
	) -> Result<(PointerValue<'ctx>, Option<WriteBack<'ctx>>), Error> {
		if borrowed {
			// Taking an address is still a read for reactive dependencies.
			let read = node.or_else(|| match self.view(call) {
				View::Call { callee, .. } => match self.view(callee) {
					View::Access { target, .. } => Some(target),
					_ => None,
				},
				_ => None,
			});
			if let Some(read) = read {
				self.track_read(read)?;
			}
			// An optional recursive receiver is already the address of the
			// object. Preserve it instead of loading all its fields into a
			// stack temporary for the borrowed call.
			if node.is_none()
				&& let Some(value) = self.pending_receiver
				&& let Repr::Rec(target) = value.repr
				&& self.repr_of(target, call.index())? == repr
			{
				self.pending_receiver = None;
				let pointer =
					self.edge_address(&self.builder, value.repr, value.value.expect("an edge"))?;
				return Ok((pointer, None));
			}
		}
		let place = match node {
			Some(node) if self.is_place(node) => Some(self.place(node)?),
			Some(_) => None,
			None => self.receiver_place(call)?,
		};
		let Some(mut place) = place else {
			// a value: owned by a slot of its own, changed by the call
			let value = match node {
				Some(node) => self.expr(node)?,
				None => self.receiver(call)?,
			};
			let value = if borrowed {
				self.convert(value, repr, call.index())?
			} else {
				self.owned_as(value, repr, node)?
			};
			if borrowed && let Some(pointer) = value.destination {
				return Ok((pointer, None));
			}
			let slot = self.alloca(repr, "argument")?;
			self.store(slot, value, node.unwrap_or(call))?;
			return Ok((
				slot.pointer,
				(!borrowed).then_some(WriteBack::Temporary(slot)),
			));
		};
		if let Repr::Rec(target) = place.repr {
			let pointer = self.load_edge(place)?;
			place = Slot {
				pointer,
				repr: self.repr_of(target, call.index())?,
			};
		}
		if place.repr == repr || self.same_layout(place.repr, repr) {
			return Ok((place.pointer, None));
		}
		// an element of a columnar: gathered, then scattered back
		if let Repr::Row(_) = place.repr {
			let view = self.alloca(repr, "view")?;
			let value = self.load(place)?;
			let value = self.convert(value, repr, call.index())?;
			self.store(view, value, call)?;
			return Ok((
				view.pointer,
				(!borrowed).then_some(WriteBack::Fields { place, view }),
			));
		}
		// a view of another object type: its fields, written back after; or a
		// borrowed optional the analysis narrowed to its value
		let view = self.alloca(repr, "view")?;
		let value = self.load(place)?;
		let value = match (value.repr, repr) {
			(Repr::Optional(_), to) if !matches!(to, Repr::Optional(_)) => {
				self.narrow_to(value, repr, call)?
			}
			_ => value,
		};
		let value = self.convert(value, repr, call.index())?;
		self.store(view, value, call)?;
		Ok((
			view.pointer,
			(!borrowed).then_some(WriteBack::Fields { place, view }),
		))
	}

	/// After a call: the changes of a view written back into its place.
	pub(crate) fn write_back(&mut self, write_back: WriteBack<'ctx>) -> Result<(), Error> {
		match write_back {
			WriteBack::Temporary(slot) => {
				let value = self.load(slot)?;
				self.fresh(value);
			}
			WriteBack::Fields { place, view } if matches!(place.repr, Repr::Row(_)) => {
				let value = self.load(view)?;
				self.store(place, value, NodeId::new(0))?;
			}
			WriteBack::Fields { place, view } => {
				let (Repr::Object(place_type), Repr::Object(view_type)) = (place.repr, view.repr)
				else {
					return Ok(());
				};
				let types = self.types();
				let place_fields = types.fields_of(place_type).to_vec();
				let view_fields = types.fields_of(view_type).to_vec();
				let place_struct = self.object_type(place_type);
				let view_struct = self.object_type(view_type);
				for (index, field) in view_fields.iter().enumerate() {
					let Some(target) = place_fields
						.iter()
						.position(|other| self.same_field_name(other.name, field.name))
					else {
						continue;
					};
					let from_repr = self.repr_of(field.ty, 0)?;
					let to_repr = self.repr_of(place_fields[target].ty, 0)?;
					let from = self.builder.build_struct_gep(
						view_struct,
						view.pointer,
						index as u32,
						"",
					)?;
					let to = self.builder.build_struct_gep(
						place_struct,
						place.pointer,
						target as u32,
						"",
					)?;
					let value = self.load(Slot {
						pointer: from,
						repr: from_repr,
					})?;
					let value = self.convert(value, to_repr, 0)?;
					self.store(
						Slot {
							pointer: to,
							repr: to_repr,
						},
						value,
						NodeId::new(0),
					)?;
				}
			}
		}
		Ok(())
	}

	/// The place of the receiver of a method call (`a.b` in `a.b.change()`),
	/// or `None` when it isn't one.
	pub(crate) fn receiver_place(&mut self, call: NodeId) -> Result<Option<Slot<'ctx>>, Error> {
		let View::Call { callee, .. } = self.view(call) else {
			return Ok(None);
		};
		let View::Access { target, members } = self.view(callee) else {
			return Ok(None);
		};
		let count = members.len() - 1;
		// a value of another file reached through namespaces (`library.people`)
		let list: Vec<NodeId> = members.iter().collect();
		if count > 0
			&& let Some(slot) = self.namespace_place(callee, target, &list[..count])?
		{
			return Ok(Some(slot));
		}
		if !self.is_place(target) {
			return Ok(None);
		}
		let slot = self.place(target)?;
		let slot = self.place_members(slot, self.type_of(target), members.iter().take(count))?;
		Ok(Some(slot))
	}

	/// `use x = place`, `mutable use x = place`: the address of the place.
	pub(crate) fn use_declaration(
		&mut self,
		node: NodeId,
		value: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(node) else {
			return Err(self.unsupported(node.index(), "this `use`"));
		};
		// a name alias (`use c = std.c`): nothing at run time
		if self.analysis.symbols.symbol(symbol).kind == fa_analyzer::symbols::SymbolKind::NameAlias
		{
			return Ok(Val::void());
		}
		let place = match self.collection_reference(value)? {
			Some((place, nullable)) => {
				if nullable {
					self.nullable_references.insert(symbol, ());
				}
				place
			}
			None => {
				let place = self.reference_place(value)?;
				if matches!(self.node_repr(value)?, Repr::Optional(_))
					&& !matches!(place.repr, Repr::Optional(_))
				{
					self.nullable_references.insert(symbol, ());
				}
				place
			}
		};
		let data = self.analysis.symbols.symbol(symbol);
		if self.shared.contains_key(&symbol) {
			// a top-level reference that functions read
			let holder = self.reference_holder(symbol, place.repr);
			self.builder.build_store(holder, place.pointer)?;
		} else if matches!(data.kind, fa_analyzer::symbols::SymbolKind::MutableUse) {
			// reseated by `reuse`: the address lives in a slot
			let holder = self.alloca_type(self.pointer_type().into(), "reference")?;
			self.builder.build_store(holder, place.pointer)?;
			self.references.insert(symbol, (holder, place.repr));
		} else {
			self.slots.insert(symbol, place);
		}
		Ok(Val::void())
	}

	/// The global holding the address a top-level reference designates, which
	/// the functions reading the reference load (created on first use).
	pub(crate) fn reference_holder(&mut self, symbol: SymbolId, repr: Repr) -> PointerValue<'ctx> {
		if let Some(&(holder, _)) = self.references.get(&symbol) {
			return holder;
		}
		let global = self
			.module
			.add_global(self.pointer_type(), None, "fa.reference");
		global.set_linkage(inkwell::module::Linkage::Internal);
		global.set_initializer(&self.pointer_type().const_null());
		let holder = global.as_pointer_value();
		self.references.insert(symbol, (holder, repr));
		holder
	}

	/// `provide Key = value`: the place of the value, or a temporary owned by
	/// the block, which the calls below give to the functions that require it.
	pub(crate) fn provide(&mut self, node: NodeId, value: NodeId) -> Result<Val<'ctx>, Error> {
		let slot = if self.is_place(value) {
			self.place(value)?
		} else {
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let repr = lowered.repr;
			let owned = self.owned_as(lowered, repr, Some(value))?;
			let pointer = self.alloca_type(self.llvm_type(repr), "provided")?;
			let slot = Slot { pointer, repr };
			self.store(slot, owned, value)?;
			if self.needs_drop(repr) {
				self.cleanups
					.borrow_mut()
					.push(super::ownership::Cleanup::Local(slot));
			}
			slot
		};
		self.provided.insert(node, slot);
		Ok(Val::void())
	}

	/// The hidden arguments of a call to a function with requirements: a
	/// pointer to what provides each one.
	pub(crate) fn requirement_arguments(
		&mut self,
		call: NodeId,
		keys: &[fa_analyzer::ids::TypeId],
	) -> Result<Vec<PointerValue<'ctx>>, Error> {
		let Some(arguments) = self.analysis.requirement_args.get(&call).cloned() else {
			return Ok(Vec::new());
		};
		let mut pointers = Vec::new();
		for (argument, key) in arguments.into_iter().zip(keys) {
			let slot = match argument {
				fa_analyzer::analysis::RequirementArg::Provided(provide) => {
					match self.provided.get(&provide) {
						Some(slot) => *slot,
						None => return Err(self.unsupported(call.index(), "this requirement")),
					}
				}
				fa_analyzer::analysis::RequirementArg::Own(own) => {
					let found = self
						.frame()
						.requirements
						.iter()
						.find(|(_, key)| *key == own)
						.copied();
					let Some((pointer, key)) = found else {
						return Err(self.unsupported(call.index(), "this requirement"));
					};
					Slot {
						pointer,
						repr: self.repr_of(key, call.index())?,
					}
				}
				fa_analyzer::analysis::RequirementArg::Missing => {
					return Err(self.unsupported(call.index(), "a missing requirement"));
				}
			};
			let repr = self.repr_of(*key, call.index())?;
			if slot.repr == repr || self.same_layout(slot.repr, repr) {
				pointers.push(slot.pointer);
			} else {
				// a view of the provided value, of the required type
				let value = self.load(slot)?;
				let value = self.convert(value, repr, call.index())?;
				let view = self.alloca_type(self.llvm_type(repr), "requirement")?;
				self.builder
					.build_store(view, value.value.expect("a value"))?;
				pointers.push(view);
			}
		}
		Ok(pointers)
	}

	/// `reuse x = place`: `x` designates another place from there on.
	pub(crate) fn reuse(&mut self, node: NodeId, value: NodeId) -> Result<Val<'ctx>, Error> {
		let Resolution::Local(symbol) = self.analysis.resolution(node) else {
			return Err(self.unsupported(node.index(), "this `reuse`"));
		};
		let Some(&(holder, _)) = self.references.get(&symbol) else {
			return Err(self.unsupported(node.index(), "this `reuse`"));
		};
		let place = match self.collection_reference(value)? {
			Some((place, _)) => place,
			None => self.reference_place(value)?,
		};
		self.builder.build_store(holder, place.pointer)?;
		Ok(Val::void())
	}

	/// Array lookups return the existing slot address, including absent
	/// optionals. Ordinary value reads keep the existing lookup lowering.
	fn array_reference(&mut self, node: NodeId) -> Result<Option<(Slot<'ctx>, bool)>, Error> {
		let Some(info) = self.analysis.call(node).cloned() else {
			return Ok(None);
		};
		let fa_analyzer::analysis::Callee::Builtin(id) = info.callee else {
			return Ok(None);
		};
		let path = fa_analyzer::builtins::get(id).path;
		if !matches!(
			path,
			"Array#get"
				| "Array#find"
				| "OrderedColumnar#get"
				| "OrderedColumnar#find"
				| "OrderedColumnar#add"
		) {
			return Ok(None);
		}
		let array = self.receiver(node)?;
		let (Repr::Array(item_type) | Repr::Growing(item_type)) = array.repr else {
			return Ok(None);
		};
		let repr = self.item_repr(item_type, node)?;
		let pointer = array.value.expect("an array").into_pointer_value();
		let length = if matches!(array.repr, Repr::Growing(_)) {
			self.growing_length(&self.builder, pointer)?
		} else {
			self.array_parts(pointer)?.1
		};
		let argument = match info.arguments.first() {
			Some(fa_analyzer::analysis::Argument::Given(value)) => *value,
			_ => return Err(self.unsupported(node.index(), "this lookup")),
		};
		let argument_value = self.expr(argument)?;
		if path == "OrderedColumnar#add" {
			self.materialize_array(pointer, repr)?;
			let index = self.array_length(array)?.int();
			self.array_push(array, argument_value, node, Some(argument))?;
			return Ok(Some((
				self.array_reference_slot(array, repr, index)?,
				false,
			)));
		}
		let result = self.pointer_alloca("reference")?;
		self.builder
			.build_store(result, self.pointer_type().const_null())?;
		let end = self.append_block("reference_found");
		if matches!(path, "Array#get" | "OrderedColumnar#get") {
			let index = self
				.convert(
					argument_value,
					Repr::Int(super::values::IntRepr::I64),
					node.index(),
				)?
				.int();
			let index = self.counted_from_end(index, length)?;
			let inside =
				self.builder
					.build_int_compare(inkwell::IntPredicate::ULT, index, length, "")?;
			let present = self.append_block("reference_present");
			self.builder
				.build_conditional_branch(inside, present, end)?;
			self.builder.position_at_end(present);
			let slot = self.array_reference_slot(array, repr, index)?;
			self.builder.build_store(result, slot.pointer)?;
			self.builder.build_unconditional_branch(end)?;
		} else {
			self.for_each_index(length, |this, index| {
				let slot = this.array_reference_slot(array, repr, index)?;
				let index_value = Val::new(Repr::Int(super::values::IntRepr::I64), index);
				let holds = this.call_collection_callback(
					argument_value,
					argument,
					&[],
					&[slot],
					&[index_value],
				)?;
				let found = this.append_block("reference_present");
				let next = this.append_block("reference_next");
				this.builder
					.build_conditional_branch(holds.int(), found, next)?;
				this.builder.position_at_end(found);
				this.builder.build_store(result, slot.pointer)?;
				this.builder.build_unconditional_branch(end)?;
				this.builder.position_at_end(next);
				Ok(())
			})?;
			self.builder.build_unconditional_branch(end)?;
		}
		self.builder.position_at_end(end);
		let pointer = self
			.builder
			.build_load(self.pointer_type(), result, "")?
			.into_pointer_value();
		Ok(Some((Slot { pointer, repr }, true)))
	}

	fn array_reference_slot(
		&self,
		array: Val<'ctx>,
		repr: Repr,
		index: inkwell::values::IntValue<'ctx>,
	) -> Result<Slot<'ctx>, Error> {
		let pointer = array.value.expect("an array").into_pointer_value();
		if matches!(array.repr, Repr::Growing(_)) {
			Ok(Slot {
				pointer: self.growing_at(&self.builder, pointer, repr, index)?,
				repr,
			})
		} else {
			self.array_item_slot(pointer, repr, index)
		}
	}

	/// The element a reference to a collection's element designates, and
	/// whether it may be null: `pool.add(…)`, `pool.find(…)` (pools.rs),
	/// `map.get(key)` (tables.rs).
	fn collection_reference(&mut self, value: NodeId) -> Result<Option<(Slot<'ctx>, bool)>, Error> {
		let mut node = value;
		while let View::Group(inner) | View::Try(inner) = self.view(node) {
			node = inner;
		}
		if let Some((element, optional)) = self.user_reference_result(node) {
			let raw = self.call_present(node)?;
			let pointer = raw
				.value
				.ok_or_else(|| {
					self.unsupported(node.index(), "a reference result without a place")
				})?
				.into_pointer_value();
			let repr = self.reference_result_repr(node, element)?;
			return Ok(Some((Slot { pointer, repr }, optional)));
		}
		if let Some(reference) = self.array_reference(node)? {
			return Ok(Some(reference));
		}
		if let Some(found) = self.pool_reference(value)? {
			return Ok(Some(found));
		}
		Ok(self.map_get_reference(value)?.map(|slot| (slot, true)))
	}

	/// The place a reference designates: a place, or the result of a call
	/// returning one (a temporary of its own otherwise).
	fn reference_place(&mut self, value: NodeId) -> Result<Slot<'ctx>, Error> {
		let mut inner = value;
		while let View::Group(group) | View::Try(group) = self.view(inner) {
			inner = group;
		}
		if self.is_place(inner) {
			let place = self.place(inner)?;
			if let Repr::Link(target) = place.repr {
				let pointer = self
					.builder
					.build_load(self.pointer_type(), place.pointer, "relation_target")?
					.into_pointer_value();
				return Ok(Slot {
					pointer,
					repr: self.repr_of(target, inner.index())?,
				});
			}
			return Ok(place);
		}
		// `map.getOrInsert(key, value)`: the value in the map
		if let Some(info) = self.analysis.call(inner).cloned()
			&& let fa_analyzer::analysis::Callee::Builtin(id) = info.callee
			&& fa_analyzer::builtins::get(id).path == "Map#getOrInsert"
		{
			return self.get_or_insert_place(inner, &info);
		}
		Err(self.unsupported(value.index(), "a `use` of this value"))
	}
}
