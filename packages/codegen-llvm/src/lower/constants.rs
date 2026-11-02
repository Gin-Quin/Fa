//! Compile-time values (advanced brief §12, `Analysis::constant`): a node
//! the analysis evaluated is lowered as its value, built from the
//! interpreter's [`Value`]: scalars and strings as constants, arrays,
//! tuples and objects as new values; `@for` runs its body once per item of
//! `Analysis::unrolled`, the variable bound to the item.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::SymbolKind;
use fa_analyzer::types::TypeKind;
use fa_interpreter::Value;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	/// A compile-time value of type `ty`, owned and not registered (the
	/// caller owns it). `node` locates an error.
	pub(crate) fn constant_value(
		&mut self,
		value: &Value,
		ty: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.repr_of(ty, node.index())?;
		let unsupported = |this: &Self| this.unsupported(node.index(), "this compile-time value");
		Ok(match (value, repr) {
			(Value::Integer(integer), Repr::Int(int)) => {
				Val::new(repr, self.const_int(int.bits, *integer))
			}
			(Value::Integer(integer), Repr::F64 | Repr::Percentage) => {
				Val::new(repr, self.context.f64_type().const_float(integer.as_f64()))
			}
			(Value::Integer(integer), Repr::F32) => Val::new(
				repr,
				self.context
					.f32_type()
					.const_float(integer.as_f64() as f32 as f64),
			),
			(Value::Float(value) | Value::Percentage(value), Repr::F64 | Repr::Percentage) => {
				Val::new(repr, self.context.f64_type().const_float(*value))
			}
			(Value::Float(value), Repr::F32) => Val::new(
				repr,
				self.context.f32_type().const_float(*value as f32 as f64),
			),
			(Value::Boolean(value), Repr::Bool) => Val::new(
				repr,
				self.context.bool_type().const_int(*value as u64, false),
			),
			(Value::String(text), Repr::Str) => self.string_constant(text),
			(Value::None, Repr::None) => Val {
				repr: Repr::None,
				value: None,
			},
			(Value::None, Repr::Optional(_)) => {
				Val::new(repr, super::zero_of(self.llvm_type(repr)))
			}
			(value, Repr::Optional(_)) => {
				let TypeKind::Union(list) = self.types().kind(ty) else {
					return Err(unsupported(self));
				};
				let members: Vec<TypeId> = self
					.types()
					.list_items(list)
					.iter()
					.copied()
					.filter(|&member| member != TypeId::NONE)
					.collect();
				let [member] = members.as_slice() else {
					return Err(unsupported(self));
				};
				let inner = self.constant_value(value, *member, node)?;
				self.convert(inner, repr, node.index())?
			}
			// a union of literal types (the items of a static array of
			// objects): the member the value has, boxed
			(value, Repr::Union) if matches!(self.types().kind(ty), TypeKind::Union(_)) => {
				let TypeKind::Union(list) = self.types().kind(ty) else {
					return Err(unsupported(self));
				};
				let members = self.types().list_items(list).to_vec();
				let Some(member) = members
					.into_iter()
					.find(|&member| self.value_has_type(value, member))
				else {
					return Err(unsupported(self));
				};
				let inner = self.constant_value(value, member, node)?;
				let inner = if self.needs_drop(inner.repr) {
					self.fresh(inner)
				} else {
					inner
				};
				// the box is registered as a temporary: the caller owns it
				let boxed = self.inject(inner, ty, node)?;
				self.take(boxed, None)?
			}
			(Value::Array(items), Repr::Array(item_type) | Repr::Growing(item_type)) => {
				let item = self.item_repr(item_type, node)?;
				let count = self.context.i64_type().const_int(items.len() as u64, false);
				let array = self.new_array(item, count, count)?;
				let (pointer, _) = self.array_parts(array)?;
				for (index, value) in items.iter().enumerate() {
					let value = self.constant_value(value, item_type, node)?;
					let value = self.convert(value, item, node.index())?;
					let index = self.context.i64_type().const_int(index as u64, false);
					let slot = self.item_pointer(pointer, item, index)?;
					if let Some(raw) = value.value {
						self.builder.build_store(slot, raw)?;
					}
				}
				match repr {
					Repr::Growing(_) => Val::new(repr, self.growing_from_array(array, item)?),
					_ => Val::new(repr, array),
				}
			}
			(Value::Tuple(items), Repr::Tuple(_)) => {
				let item_types = self.tuple_items(ty);
				if item_types.len() != items.len() {
					return Err(unsupported(self));
				}
				let structure = self.tuple_type(ty);
				let mut tuple = structure.get_undef();
				for (index, (value, item_type)) in items.iter().zip(item_types).enumerate() {
					let item_repr = self.repr_of(item_type, node.index())?;
					let value = self.constant_value(value, item_type, node)?;
					let value = self.convert(value, item_repr, node.index())?;
					let raw = value
						.value
						.unwrap_or_else(|| self.context.bool_type().const_zero().into());
					tuple = self
						.builder
						.build_insert_value(tuple, raw, index as u32, "")?
						.into_struct_value();
				}
				Val::new(repr, tuple)
			}
			(Value::Object(object), Repr::Object(_)) => {
				let fields = self.types().fields_of(ty).to_vec();
				let structure = self.object_type(ty);
				let mut result = structure.get_undef();
				for (index, field) in fields.iter().enumerate() {
					let name = self.types().syms.resolve(field.name).to_string();
					let Some((_, value)) = object.fields.iter().find(|(key, _)| **key == *name)
					else {
						return Err(unsupported(self));
					};
					let field_repr = self.repr_of(field.ty, node.index())?;
					let value = self.constant_value(value, field.ty, node)?;
					let value = self.convert(value, field_repr, node.index())?;
					let raw = value
						.value
						.unwrap_or_else(|| self.context.bool_type().const_zero().into());
					result = self
						.builder
						.build_insert_value(result, raw, index as u32, "")?
						.into_struct_value();
				}
				Val::new(repr, result)
			}
			_ => return Err(unsupported(self)),
		})
	}

	/// Whether a compile-time value belongs to a type: literal types are
	/// compared with the value, the others accept it.
	fn value_has_type(&self, value: &Value, ty: TypeId) -> bool {
		let types = self.types();
		match (value, types.kind(ty)) {
			(Value::Integer(integer), TypeKind::Int(_)) => types
				.as_int(ty)
				.is_some_and(|int| int.min <= *integer && *integer <= int.max),
			(Value::String(text), TypeKind::StringLit(sym)) => types.syms.resolve(sym) == &**text,
			(Value::Boolean(value), TypeKind::BoolLit(literal)) => *value == literal,
			(Value::None, kind) => kind == TypeKind::None,
			(Value::Object(object), TypeKind::Object(_)) => {
				let fields = types.fields_of(ty).to_vec();
				fields.len() == object.fields.len()
					&& fields.iter().all(|field| {
						let name = types.syms.resolve(field.name);
						object
							.fields
							.iter()
							.find(|(key, _)| &**key == name)
							.is_some_and(|(_, value)| self.value_has_type(value, field.ty))
					})
			}
			(Value::Tuple(items), TypeKind::Tuple(list)) => {
				let item_types = types.list_items(list).to_vec();
				item_types.len() == items.len()
					&& items
						.iter()
						.zip(item_types)
						.all(|(value, ty)| self.value_has_type(value, ty))
			}
			(_, TypeKind::None) => false,
			_ => true,
		}
	}

	/// `@for item in items { … }`: the body once per item the analysis
	/// computed, the variable (and the index) bound to it; an ordinary loop
	/// when the items are known per call only.
	pub(crate) fn comptime_for(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(items) = self.analysis.unrolled.get(&node).cloned() else {
			return self.for_loop(node, pattern, has_index, iterable, body);
		};
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		let index = if has_index {
			self.analysis
				.symbols
				.symbols()
				.iter()
				.position(|symbol| {
					symbol.decl_node == node
						&& matches!(symbol.kind, SymbolKind::LoopVariable { copy: true })
				})
				.map(SymbolId::new)
		} else {
			None
		};
		for (position, item) in items.iter().enumerate() {
			let mark = self.mark();
			if let Some(symbol) = variable {
				let ty = self.analysis.symbols.symbol(symbol).ty;
				let value = self.constant_value(item, ty, node)?;
				let slot = self.slot_of(symbol, node)?;
				self.store(slot, value, node)?;
				if self.needs_drop(slot.repr) {
					self.cleanups
						.borrow_mut()
						.push(super::ownership::Cleanup::Local(slot));
				}
			}
			if let Some(symbol) = index {
				let slot = self.slot_of(symbol, node)?;
				let value = Val::new(
					Repr::Int(IntRepr::I64),
					self.context.i64_type().const_int(position as u64, false),
				);
				self.store(slot, value, node)?;
			}
			let result = self.block(body, false)?;
			if result.repr == Repr::Never {
				return Ok(result);
			}
			self.end_scope(mark, None)?;
		}
		Ok(Val::void())
	}
}
