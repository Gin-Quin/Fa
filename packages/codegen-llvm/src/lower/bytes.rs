//! Fixed-length byte blocks and two-word borrowed views. Array mechanics
//! operate through a temporary borrowed header; a Bytes value keeps its
//! distinct representation and ownership throughout calls and storage.

use fa_analyzer::analysis::{Argument, CallInfo};
use fa_analyzer::ids::{NodeId, TypeId};
use inkwell::types::StructType;

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// Borrow a live input; consume only a temporary or a proven last read.
	pub(super) fn conversion_source(
		&self,
		value: Val<'ctx>,
		node: Option<NodeId>,
	) -> Result<(Val<'ctx>, bool), Error> {
		let moved = self.is_temporary(value)
			|| node.is_some_and(|node| self.movable_local(node, value.repr).is_some());
		Ok((
			if moved {
				self.take(value, node)?
			} else {
				value
			},
			moved,
		))
	}

	pub(super) fn string_decode_bytes(
		&mut self,
		node: NodeId,
		info: &CallInfo,
		lossy: bool,
	) -> Result<Val<'ctx>, Error> {
		let Some(Argument::Given(argument)) = info.arguments.first() else {
			return Err(self.unsupported(node.index(), "UTF-8 input"));
		};
		let input = self.expr(*argument)?;
		if input.repr == Repr::Never {
			return Ok(input);
		}
		let (input, moved) = self.conversion_source(input, Some(*argument))?;
		let out = self.alloca(Repr::Str, "decoded_utf8")?;
		let error = self
			.call_value(
				self.runtime.function("fa_string_decode_bytes")?,
				&[
					input.value.expect("Bytes").into(),
					self.context
						.i32_type()
						.const_int(moved as u64, false)
						.into(),
					self.context
						.i32_type()
						.const_int(lossy as u64, false)
						.into(),
					out.pointer.into(),
				],
			)?
			.into_pointer_value();
		if !lossy {
			self.raise_if_error(error)?;
		}
		let text = self.load(out)?;
		Ok(self.fresh(text))
	}

	pub(super) fn bytes_view_type(&self) -> StructType<'ctx> {
		self.context
			.struct_type(&[self.pointer_type().into(), self.usize.into()], false)
	}

	pub(super) fn bytes_reference_type(&self) -> StructType<'ctx> {
		self.context.struct_type(
			&[self.pointer_type().into(), self.bytes_view_type().into()],
			false,
		)
	}

	/// A stack header borrows the payload of either an owner or a view.
	pub(super) fn bytes_as_array(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let structure = self.context.struct_type(
			&[self.header_type().into(), self.context.i64_type().into()],
			false,
		);
		let header = self.alloca_type(structure.into(), "bytes_header")?;
		self.call_runtime(
			"fa_bytes_array_view",
			&[value.value.expect("Bytes").into(), header.into()],
		)?;
		Ok(Val::new(Repr::Array(TypeId::BYTE), header))
	}

	pub(super) fn bytes_index_place(
		&mut self,
		node: NodeId,
		bytes: Val<'ctx>,
		index: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let array = self.bytes_as_array(bytes)?;
		let position = self.expr(index)?;
		let byte = self.repr_of(TypeId::BYTE, node.index())?;
		let pointer = self.checked_item(
			node,
			array.value.expect("header").into_pointer_value(),
			byte,
			position,
			Some(index),
		)?;
		Ok(Slot {
			pointer,
			repr: byte,
		})
	}

	pub(super) fn bytes_slice(
		&mut self,
		_node: NodeId,
		target: NodeId,
		index: NodeId,
		view: bool,
	) -> Result<Val<'ctx>, Error> {
		let bytes = self.expr(target)?;
		let range = self.expr(index)?;
		if bytes.repr == Repr::Never || range.repr == Repr::Never {
			return Ok(Val::never());
		}
		let (start, end, flags) = self.range_parts(range)?;
		let out = if view {
			self.alloca_type(self.bytes_view_type().into(), "bytes_view")?
		} else {
			self.alloca(Repr::Bytes, "bytes_slice")?.pointer
		};
		let error = self
			.call_value(
				self.runtime.function(if view {
					"fa_bytes_slice_view"
				} else {
					"fa_bytes_slice"
				})?,
				&[
					bytes.value.expect("Bytes").into(),
					start.into(),
					end.into(),
					flags.into(),
					out.into(),
				],
			)?
			.into_pointer_value();
		self.raise_if_error(error)?;
		if view {
			let pointer = self.call_value(
				self.runtime.function("fa_bytes_view_pointer")?,
				&[out.into()],
			)?;
			Ok(Val::new(Repr::Bytes, pointer))
		} else {
			let bytes = self.load(Slot {
				pointer: out,
				repr: Repr::Bytes,
			})?;
			Ok(self.fresh(bytes))
		}
	}

	pub(super) fn bytes_builtin(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let arguments: Vec<_> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		if matches!(path, "Bytes#get" | "Bytes#isEmpty") {
			if path == "Bytes#get"
				&& let [index] = arguments.as_slice()
				&& let Some(text) = self.code_points_of(node)
			{
				return self.code_point_get(node, text, *index);
			}
			let bytes = self.receiver(node)?;
			if bytes.repr == Repr::Never {
				return Ok(bytes);
			}
			let array = self.bytes_as_array(bytes)?;
			return if let Some(&index) = arguments.first() {
				let index = self.expr(index)?;
				if index.repr == Repr::Never {
					return Ok(index);
				}
				self.array_get(array, index, node)
			} else {
				let length = self.array_length(array)?.int();
				let empty = self.builder.build_int_compare(
					inkwell::IntPredicate::EQ,
					length,
					length.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, empty))
			};
		}
		let receiver = if path.starts_with("Bytes#") {
			Some(self.receiver(node)?)
		} else {
			None
		};
		if receiver.is_some_and(|value| value.repr == Repr::Never) {
			return Ok(Val::never());
		}
		let mut values = Vec::new();
		for &argument in &arguments {
			let value = self.expr(argument)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			values.push(value);
		}
		let result = match path {
			"Bytes.from" => {
				let (source, moved) = self.conversion_source(values[0], Some(arguments[0]))?;
				self.call_value(
					self.runtime.function("fa_bytes_from_array")?,
					&[
						source.value.expect("array").into(),
						self.context
							.i32_type()
							.const_int(moved as u64, false)
							.into(),
					],
				)?
			}
			"Bytes.filled" => {
				let length = self
					.convert(values[0], Repr::Int(IntRepr::I64), node.index())?
					.int();
				let byte = self.repr_of(TypeId::BYTE, node.index())?;
				let value = match values.get(1) {
					Some(&value) => self.convert(value, byte, node.index())?.int(),
					None => self.context.i8_type().const_zero(),
				};
				self.call_value(
					self.runtime.function("fa_bytes_filled")?,
					&[length.into(), value.into()],
				)?
			}
			"Bytes#toArray" => {
				let (receiver, moved) = self
					.conversion_source(receiver.expect("receiver"), self.method_receiver(node))?;
				let pointer = self.call_value(
					self.runtime.function("fa_bytes_to_array")?,
					&[
						receiver.value.expect("Bytes").into(),
						self.context
							.i32_type()
							.const_int(moved as u64, false)
							.into(),
					],
				)?;
				return Ok(self.fresh(Val::new(Repr::Array(TypeId::BYTE), pointer)));
			}
			"Bytes#toString" => {
				self.call_runtime("fa_b_reset", &[])?;
				self.append_value(receiver.expect("receiver"), node)?;
				let text = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
				return Ok(self.fresh(text));
			}
			_ => return Err(self.unsupported(node.index(), path)),
		};
		Ok(self.fresh(Val::new(Repr::Bytes, result)))
	}
}
