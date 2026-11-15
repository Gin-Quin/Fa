//! Caller-owned result slots. Coroutine promises keep their separate ABI.

use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::types::AnyType;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue};

use crate::error::Error;
use crate::lower::ownership::Cleanup;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	fn return_value_leaves(&self, root: NodeId, leaves: &mut Vec<NodeId>) {
		let mut pending = vec![root];
		while let Some(node) = pending.pop() {
			match self.view(node) {
				View::Group(inner) | View::Try(inner) | View::Do(inner) => pending.push(inner),
				View::Block { statements } => {
					if let Some(last) = statements.iter().last() {
						pending.push(last);
					}
				}
				View::If {
					then_body,
					else_body: Some(other),
					..
				} => pending.extend([then_body, other]),
				View::When { arms, .. } => pending.extend(arms.iter().map(|arm| arm.value)),
				View::Return(_) | View::Throw(_) => {}
				_ if self
					.node_repr(node)
					.is_ok_and(|repr| matches!(repr, Repr::Void | Repr::Never)) => {}
				_ => leaves.push(node),
			}
		}
	}

	/// Every successful exit must give up the same immutable owner. Alias,
	/// capture and move facts come from the existing ownership proofs.
	pub(super) fn returned_local_candidate(&mut self, body: NodeId) -> Option<SymbolId> {
		let mut leaves = Vec::new();
		self.return_value_leaves(body, &mut leaves);
		let mut pending = vec![body];
		while let Some(node) = pending.pop() {
			match self.view(node) {
				View::Lambda { .. } | View::Function { .. } => continue,
				View::Use { .. } | View::Reuse { .. } | View::Provide { .. } | View::Watch(_) => {
					return None;
				}
				View::Return(Some(value)) => self.return_value_leaves(value, &mut leaves),
				_ => {}
			}
			self.tree.nodes[node.index()].for_each_child(|child| pending.push(NodeId::new(child)));
		}
		let Resolution::Local(symbol) = self.analysis.resolution(*leaves.first()?) else {
			return None;
		};
		if !leaves
			.iter()
			.all(|node| self.analysis.resolution(*node) == Resolution::Local(symbol))
			|| self.referenced_locals().contains(&symbol)
			|| !self.collection_returned_binding(symbol)
		{
			return None;
		}
		Some(symbol)
	}

	pub(super) fn is_coalesced_return_read(&self, node: NodeId, repr: Repr) -> bool {
		let Some(symbol) = self.frame().nrvo_local else {
			return false;
		};
		repr == self.frame().ret
			&& self.analysis.resolution(self.ungrouped(node)) == Resolution::Local(symbol)
	}

	pub(super) fn preserves_return_owner(
		&self,
		entry: Cleanup<'ctx>,
		destination: Option<inkwell::values::PointerValue<'ctx>>,
	) -> bool {
		if self.frame().nrvo_local.is_none()
			|| destination != self.frame().return_slot.map(|slot| slot.pointer)
		{
			return false;
		}
		match entry {
			Cleanup::Local(slot) | Cleanup::Flagged(slot, _) => Some(slot.pointer) == destination,
			_ => false,
		}
	}

	pub(super) fn uses_return_slot(&self, repr: Repr) -> bool {
		!self.tasks.asynchronous && matches!(repr, Repr::Object(_) | Repr::Tuple(_))
	}

	pub(super) fn add_return_slot_attribute(
		&self,
		function: FunctionValue<'ctx>,
		repr: Repr,
		throws: bool,
		index: u32,
	) {
		// LLVM sret requires a void result. Throwing functions instead return
		// just their error pointer and use the same plain destination parameter.
		if !throws {
			let attribute = self.context.create_type_attribute(
				Attribute::get_named_enum_kind_id("sret"),
				self.llvm_type(repr).as_any_type_enum(),
			);
			function.add_attribute(AttributeLoc::Param(index), attribute);
		}
	}

	fn destination_node(&self, mut node: NodeId) -> NodeId {
		while let View::Group(inner) | View::Try(inner) = self.view(node) {
			node = inner;
		}
		node
	}

	pub(super) fn destination_for(&self, node: NodeId, repr: Repr) -> Option<Slot<'ctx>> {
		self.destination_at(node).filter(|slot| slot.repr == repr)
	}

	pub(super) fn destination_at(&self, node: NodeId) -> Option<Slot<'ctx>> {
		let (target, slot) = self.frame().destination?;
		(self.destination_node(target) == self.destination_node(node)).then_some(slot)
	}

	pub(super) fn with_destination<T>(
		&mut self,
		node: NodeId,
		slot: Option<Slot<'ctx>>,
		lower: impl FnOnce(&mut Self) -> Result<T, Error>,
	) -> Result<T, Error> {
		let outer = self.frame().destination;
		self.frame_mut().destination = slot.map(|slot| (node, slot));
		let result = lower(self);
		self.frame_mut().destination = outer;
		result
	}

	pub(super) fn expr_with_destination(
		&mut self,
		node: NodeId,
		slot: Option<Slot<'ctx>>,
	) -> Result<Val<'ctx>, Error> {
		self.with_destination(node, slot, |this| this.expr(node))
	}

	pub(super) fn load_destination(&self, slot: Slot<'ctx>) -> Result<Val<'ctx>, Error> {
		let mut value = self.load(slot)?;
		value.destination = Some(slot.pointer);
		Ok(value)
	}

	pub(super) fn call_return_slot(
		&mut self,
		node: NodeId,
		repr: Repr,
	) -> Result<Slot<'ctx>, Error> {
		match self.destination_for(node, repr) {
			Some(slot) => Ok(slot),
			None => self.alloca(repr, "return.result"),
		}
	}

	pub(super) fn slot_call_result(
		&mut self,
		raw: Option<BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
		slot: Option<Slot<'ctx>>,
	) -> Result<Val<'ctx>, Error> {
		let Some(slot) = slot else {
			return self.call_result(raw, ret, throws);
		};
		if throws {
			self.raise_if_error(raw.expect("an error pointer").into_pointer_value())?;
		}
		Ok(self.fresh(self.load_destination(slot)?))
	}

	/// Adapt to the existing task-promise/arena-wrapper raw result. An error
	/// must never read an unfinished result slot or expose its abandoned fields.
	pub(super) fn raw_from_return_slot(
		&self,
		builder: &Builder<'ctx>,
		slot: Slot<'ctx>,
		error: Option<BasicValueEnum<'ctx>>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let Some(error) = error else {
			return Ok(builder.build_load(self.llvm_type(slot.repr), slot.pointer, "result")?);
		};
		let function = builder
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let failed = builder.get_insert_block().expect("a block");
		let success = self.context.append_basic_block(function, "result_success");
		let join = self.context.append_basic_block(function, "result_ready");
		let okay = builder.build_is_null(error.into_pointer_value(), "")?;
		builder.build_conditional_branch(okay, success, join)?;
		builder.position_at_end(success);
		let value = builder.build_load(self.llvm_type(slot.repr), slot.pointer, "result")?;
		builder.build_unconditional_branch(join)?;
		builder.position_at_end(join);
		let result = builder.build_phi(self.llvm_type(slot.repr), "result")?;
		result.add_incoming(&[
			(&super::zero_of(self.llvm_type(slot.repr)), failed),
			(&value, success),
		]);
		let ty = self.context.struct_type(
			&[self.llvm_type(slot.repr), self.pointer_type().into()],
			false,
		);
		let pair = builder.build_insert_value(ty.get_undef(), result.as_basic_value(), 0, "")?;
		Ok(builder
			.build_insert_value(pair, error, 1, "")?
			.as_basic_value_enum())
	}
}
