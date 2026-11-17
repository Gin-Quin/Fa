//! Omitted arguments preserve the selected result's loan or temporary owner.
//! Only result paths participate; stores computing an argument keep their
//! ordinary ownership rules.

use fa_analyzer::ids::NodeId;
use fa_analyzer::syntax::View;
use inkwell::values::PointerValue;

use super::values::{Repr, Val};
use super::{Lowering, Slot};
use crate::error::Error;

#[derive(Clone, Copy)]
pub(super) struct DefaultInput<'ctx> {
	root: NodeId,
	value: Slot<'ctx>,
	address: Slot<'ctx>,
	owned: Option<PointerValue<'ctx>>,
	control: bool,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Register cleanup before the written/default branch. A written argument
	/// never initializes the temporary and leaves its ownership flag false.
	pub(super) fn prepare_default_input(
		&mut self,
		root: NodeId,
		repr: Repr,
	) -> Result<DefaultInput<'ctx>, Error> {
		let value = self.alloca(repr, "default_value")?;
		let address = self.alloca(Repr::Pointer, "default_address")?;
		let owned = if self.needs_drop(repr) {
			let flag = self.alloca(Repr::Bool, "default_owned")?;
			self.builder
				.build_store(flag.pointer, self.context.bool_type().const_zero())?;
			self.own_flagged(value, flag.pointer);
			Some(flag.pointer)
		} else {
			None
		};
		let input = DefaultInput {
			root,
			value,
			address,
			owned,
			control: false,
		};
		let mut pending = vec![root];
		while let Some(node) = pending.pop() {
			let children = match self.view(node) {
				View::Loop { .. } => Some(
					self.analysis
						.loop_results
						.get(&node)
						.cloned()
						.unwrap_or_default(),
				),
				_ => fa_analyzer::lower::copies::result_inputs(self.tree, node),
			};
			self.frame_mut().default_inputs.insert(
				node,
				DefaultInput {
					control: children.is_some(),
					..input
				},
			);
			pending.extend(children.into_iter().flatten());
		}
		Ok(input)
	}

	pub(super) fn bind_default_input(
		&mut self,
		node: NodeId,
		input: DefaultInput<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let result = self.expr(node);
		self.frame_mut()
			.default_inputs
			.retain(|_, saved| saved.root != input.root);
		self.frame_mut().default_places.clear();
		result?;
		Ok(self
			.load(input.address)?
			.value
			.expect("an argument address")
			.into_pointer_value())
	}

	/// Resolve a selected place once, including index expressions. The checked
	/// expression evaluator still performs its tracking and sink checks.
	pub(super) fn default_place_input(&mut self, node: NodeId) -> Result<Option<Val<'ctx>>, Error> {
		if !self
			.frame()
			.default_inputs
			.get(&node)
			.is_some_and(|input| !input.control)
		{
			return Ok(None);
		}
		let root = self.frame().default_inputs[&node].root;
		if fa_analyzer::lower::copies::argument_local_root(self.tree, self.analysis, root, node)
			.is_some()
		{
			return Ok(None);
		}
		if matches!(self.view(node), View::Call { .. })
			&& let Some((place, optional)) = self.collection_reference(node)?
		{
			self.frame_mut().default_places.insert(node, place);
			return Ok(Some(if optional {
				self.optional_element(place.pointer, place.repr, node)?
			} else {
				self.load(place)?
			}));
		}
		if !self.is_place(node) {
			return Ok(None);
		}
		let place = self.place(node)?;
		self.frame_mut().default_places.insert(node, place);
		Ok(Some(self.load(place)?))
	}

	pub(super) fn finish_default_input(
		&mut self,
		node: NodeId,
		value: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let Some(input) = self.frame().default_inputs.get(&node).copied() else {
			return Ok(value);
		};
		if value.repr == Repr::Never {
			return Ok(value);
		}
		if input.control {
			// Control-flow joins contain the selected value's bits, but ownership
			// remains with its original place or the guarded default temporary.
			self.unregister(value);
			return Ok(value);
		}
		let place = self.frame_mut().default_places.remove(&node);
		let mut fresh = self.is_temporary(value);
		let value = if fresh {
			self.take(value, None)?
		} else if fa_analyzer::lower::copies::argument_local_root(
			self.tree,
			self.analysis,
			input.root,
			node,
		)
		.is_some()
		{
			fresh = true;
			if matches!(self.view(node), View::Name(_)) {
				let local = self.place(node)?;
				self.give_up_place(node, local)?;
				value
			} else {
				// Ordinary data projected from a dying owner retains its value
				// semantics. Structure-field stores are checked by the analyzer.
				self.emit_copy(value)?
			}
		} else {
			value
		};
		if matches!(value.repr, Repr::Optional(_) | Repr::None)
			&& !matches!(input.value.repr, Repr::Optional(_) | Repr::None)
		{
			// A coalescing operand contributes only its present payload. It
			// must not be converted to the final type before the presence test.
			let (present, inner) = self.optional_split(value)?;
			if let Some(owned) = input.owned {
				self.builder
					.build_store(owned, self.context.bool_type().const_zero())?;
			}
			let yes = self.append_block("default_present");
			let end = self.append_block("default_optional");
			self.builder.build_conditional_branch(present, yes, end)?;
			self.builder.position_at_end(yes);
			if let Some(inner) = inner {
				let place = match place {
					Some(place) => match place.repr {
						Repr::Optional(scalar) if !scalar.nullable() => Some(Slot {
							pointer: self.builder.build_struct_gep(
								self.llvm_type(place.repr).into_struct_type(),
								place.pointer,
								1,
								"",
							)?,
							repr: scalar.repr(),
						}),
						Repr::Optional(scalar) => Some(Slot {
							repr: scalar.repr(),
							..place
						}),
						_ => Some(place),
					},
					None => None,
				};
				self.select_default_input(input, node, inner, place, fresh)?;
			}
			self.branch_to(end)?;
			self.builder.position_at_end(end);
		} else {
			self.select_default_input(input, node, value, place, fresh)?;
		}
		Ok(value)
	}

	fn select_default_input(
		&self,
		input: DefaultInput<'ctx>,
		node: NodeId,
		value: Val<'ctx>,
		place: Option<Slot<'ctx>>,
		fresh: bool,
	) -> Result<(), Error> {
		let pointer = match place {
			Some(place)
				if place.repr == input.value.repr
					|| self.same_layout(place.repr, input.value.repr) =>
			{
				place.pointer
			}
			_ => {
				let converted = self.convert(value, input.value.repr, node.index())?;
				self.store(input.value, converted, node)?;
				self.unregister(converted);
				input.value.pointer
			}
		};
		self.builder.build_store(input.address.pointer, pointer)?;
		if let Some(owned) = input.owned {
			self.builder.build_store(
				owned,
				self.context.bool_type().const_int(u64::from(fresh), false),
			)?;
		}
		Ok(())
	}
}
