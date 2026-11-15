//! Structural projections of consumed values transfer their retained fields
//! and destroy the fields that the receiving type does not contain.

use fa_analyzer::ids::NodeId;
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, FunctionValue};

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// Keep a prepared argument owned until the invocation takes it. Taking
	/// the source before projection also protects it if a later argument fails.
	pub(crate) fn prepare_owned_projection(
		&mut self,
		value: Val<'ctx>,
		target: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if matches!(value.repr, Repr::Object(ty) if self.types().is_resource(ty))
			&& matches!(target, Repr::Object(ty) if !self.types().is_resource(ty))
		{
			return Err(self.unsupported(
				node.index(),
				"transferring a Resource payload into a plain consuming view",
			));
		}
		let value = self.take(value, Some(node))?;
		let projected = if value.repr == target {
			value
		} else {
			let function = self.owned_projection_function(value.repr, target, node)?;
			Val::new(
				target,
				self.call_value(function, &[value.value.expect("a value").into()])?,
			)
		};
		Ok(self.fresh(projected))
	}

	fn owned_projection_function(
		&mut self,
		source: Repr,
		target: Repr,
		node: NodeId,
	) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!(
			"owned projection {} to {}",
			self.shape_key(source),
			self.shape_key(target)
		);
		let cached = self.copy_functions.borrow().get(&key).copied();
		if let Some(function) = cached {
			return Ok(function);
		}
		let signature = self
			.llvm_type(target)
			.fn_type(&[self.llvm_type(source).into()], false);
		let function =
			self.module
				.add_function("fa.project.owned", signature, Some(Linkage::Internal));
		self.runtime.add_attributes(function, &["nounwind"]);
		self.copy_functions.borrow_mut().insert(key, function);
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, target, None);
		let argument = Val::new(
			source,
			function.get_nth_param(0).expect("a projection source"),
		);
		let projected = self.build_owned_projection(argument, target, node)?;
		self.unregister(projected);
		self.end_scope(0, None)?;
		self.builder.build_return(
			projected
				.value
				.as_ref()
				.map(|value| value as &dyn BasicValue<'ctx>),
		)?;
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		Ok(function)
	}

	fn project_owned_child(
		&mut self,
		value: Val<'ctx>,
		target: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if value.repr == target {
			return Ok(value);
		}
		let function = self.owned_projection_function(value.repr, target, node)?;
		Ok(Val::new(
			target,
			self.call_value(function, &[value.value.expect("a child").into()])?,
		))
	}

	fn build_owned_projection(
		&mut self,
		value: Val<'ctx>,
		target: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		match (value.repr, target) {
			(Repr::Object(source), Repr::Object(destination)) => {
				let source_fields = self.types().fields_of(source).to_vec();
				let target_fields = self.types().fields_of(destination).to_vec();
				let raw = value.value.expect("an object").into_struct_value();
				let mut result = self.object_type(destination).get_undef();
				for (index, field) in source_fields.iter().enumerate().rev() {
					if target_fields.iter().any(|target| {
						self.types().syms.resolve(target.name)
							== self.types().syms.resolve(field.name)
					}) {
						continue;
					}
					let repr = self.repr_of(field.ty, node.index())?;
					if self.needs_drop(repr) {
						let child = self.builder.build_extract_value(raw, index as u32, "")?;
						self.emit_drop(Val::new(repr, child))?;
					}
				}
				for (index, field) in target_fields.iter().enumerate() {
					let position = source_fields
						.iter()
						.position(|source| {
							self.types().syms.resolve(source.name)
								== self.types().syms.resolve(field.name)
						})
						.ok_or_else(|| {
							self.unsupported(node.index(), "this owning object projection")
						})?;
					let source = self.repr_of(source_fields[position].ty, node.index())?;
					let destination = self.repr_of(field.ty, node.index())?;
					if !destination.has_value() {
						continue;
					}
					let child = Val::new(
						source,
						self.builder.build_extract_value(raw, position as u32, "")?,
					);
					let child = self.project_owned_child(child, destination, node)?;
					result = self
						.builder
						.build_insert_value(
							result,
							child.value.expect("a field"),
							index as u32,
							"",
						)?
						.into_struct_value();
				}

				Ok(Val::new(target, result))
			}
			(Repr::Tuple(source), Repr::Tuple(destination)) => {
				let source_fields = self.tuple_items(source);
				let target_fields = self.tuple_items(destination);
				let raw = value.value.expect("a tuple").into_struct_value();
				let mut result = self.tuple_type(destination).get_undef();
				for (index, (source, destination)) in
					source_fields.into_iter().zip(target_fields).enumerate()
				{
					let source = self.repr_of(source, node.index())?;
					let destination = self.repr_of(destination, node.index())?;
					if !destination.has_value() {
						continue;
					}
					let child = Val::new(
						source,
						self.builder.build_extract_value(raw, index as u32, "")?,
					);
					let child = self.project_owned_child(child, destination, node)?;
					result = self
						.builder
						.build_insert_value(
							result,
							child.value.expect("an item"),
							index as u32,
							"",
						)?
						.into_struct_value();
				}
				Ok(Val::new(target, result))
			}
			(Repr::Array(source), Repr::Array(destination)) => {
				let source_repr = self.item_repr(source, node)?;
				let target_repr = self.item_repr(destination, node)?;
				if source_repr == target_repr {
					return self.convert(value, target, node.index());
				}
				let array = value.value.expect("an array").into_pointer_value();
				self.materialize_array(array, source_repr)?;
				let (source_items, length) = self.array_parts(array)?;
				let result = self.new_array(target_repr, length, length)?;
				let (target_items, _) = self.array_parts(result)?;
				self.for_each_index(length, |this, index| {
					let source = this.item_pointer(source_items, source_repr, index)?;
					let child = this.load(Slot {
						pointer: source,
						repr: source_repr,
					})?;
					let child = this.project_owned_child(child, target_repr, node)?;
					let target = this.item_pointer(target_items, target_repr, index)?;
					this.builder
						.build_store(target, child.value.expect("an item"))?;
					Ok(())
				})?;
				self.builder.build_call(
					self.runtime.function("fa_array_free")?,
					&[array.into(), self.item_size(source_repr)?.into()],
					"",
				)?;
				Ok(Val::new(target, result))
			}
			(Repr::Optional(_), Repr::Optional(destination)) => {
				let result = self.alloca(target, "projected.optional")?;
				self.builder
					.build_store(result.pointer, self.llvm_type(target).const_zero())?;
				let (present, child) = self.split_optional(&self.builder, value)?;
				self.guard(present, |this| {
					let child = this.project_owned_child(child, destination.repr(), node)?;
					let packed = this.pack_optional(
						&this.builder,
						destination,
						this.context.bool_type().const_int(1, false),
						child.value.expect("an optional value"),
					)?;
					this.builder.build_store(result.pointer, packed)?;
					Ok(())
				})?;
				self.load(result)
			}
			(_, Repr::Optional(destination)) => {
				if value.repr == Repr::None {
					return Ok(Val::new(target, self.llvm_type(target).const_zero()));
				}
				let child = self.project_owned_child(value, destination.repr(), node)?;
				let packed = self.pack_optional(
					&self.builder,
					destination,
					self.context.bool_type().const_int(1, false),
					child.value.expect("an optional value"),
				)?;
				Ok(Val::new(target, packed))
			}
			(Repr::Function(_), Repr::Function(_)) => {
				// This helper receives an owned callable. A signature adapter
				// adopts it instead of retaining an owner that would be lost.
				let value = self.fresh(value);
				self.convert(value, target, node.index())
			}
			_ => self.convert(value, target, node.index()),
		}
	}
}
