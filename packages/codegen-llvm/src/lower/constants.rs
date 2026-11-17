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
	/// Preserve every UTF-16 unit of a compile-time value. Ordinary literals
	/// keep their UTF-8 fast path; lone surrogates use canonical WTF-8.
	pub(crate) fn text_constant(&mut self, text: &fa_interpreter::text::Text) -> Val<'ctx> {
		if let Some(text) = text.as_str() {
			return self.string_constant(text);
		}
		let mut bytes = Vec::with_capacity(text.utf8_len());
		for point in char::decode_utf16(text.units()) {
			match point {
				Ok(point) => {
					let mut encoded = [0; 4];
					bytes.extend_from_slice(point.encode_utf8(&mut encoded).as_bytes());
				}
				Err(error) => {
					let unit = error.unpaired_surrogate();
					bytes.extend_from_slice(&[
						0xE0 | (unit >> 12) as u8,
						0x80 | ((unit >> 6) & 63) as u8,
						0x80 | (unit & 63) as u8,
					]);
				}
			}
		}
		let pointer = self.constant_bytes(&bytes);
		// This tag also denotes counted views. Treating malformed static
		// data conservatively as borrowed preserves lifetime and copy rules.
		let bits = self.usize.get_bit_width();
		let capacity = (1u64 << (bits - 4)) | (1u64 << (bits - 2)) | text.len() as u64;
		let value = self.string_type.const_named_struct(&[
			pointer.into(),
			self.usize.const_int(bytes.len() as u64, false).into(),
			self.usize.const_int(capacity, false).into(),
		]);
		Val::new(Repr::Str, value)
	}

	/// A literal array's length survives immutable bindings and copies. Its
	/// receiver is still evaluated normally, including every initializer.
	pub(crate) fn constant_array_length(&self, mut node: NodeId) -> Option<usize> {
		use fa_analyzer::symbols::Resolution;
		use fa_analyzer::syntax::View;
		for _ in 0..32 {
			match self.view(node) {
				View::Group(inner) => node = inner,
				View::Array { items } => {
					return (!items
						.iter()
						.any(|item| matches!(self.view(item), View::Spread(_))))
					.then_some(items.len());
				}
				View::Name(_) => {
					let Resolution::Local(symbol) = self.analysis.resolution(node) else {
						return None;
					};
					let symbol = self.analysis.symbols.symbol(symbol);
					if !matches!(symbol.kind, SymbolKind::Let | SymbolKind::Static) {
						return None;
					}
					let View::Declaration {
						value: Some(value), ..
					} = self.view(symbol.decl_node)
					else {
						return None;
					};
					node = value;
				}
				_ => return None,
			}
		}
		None
	}

	/// Text known without executing user code. Immutable aliases are safe to
	/// follow; mutable values and references may change and are never folded.
	pub(crate) fn constant_text(&self, mut node: NodeId) -> Option<String> {
		for _ in 0..32 {
			if let Some(Value::String(text)) = self.comptime_value(node) {
				return text.as_str().map(str::to_string);
			}
			match self.view(node) {
				fa_analyzer::syntax::View::String(text) => return Some(text.to_string()),
				fa_analyzer::syntax::View::Group(inner) => node = inner,
				fa_analyzer::syntax::View::Name(_) => {
					let fa_analyzer::symbols::Resolution::Local(symbol) =
						self.analysis.resolution(node)
					else {
						return None;
					};
					let symbol = self.analysis.symbols.symbol(symbol);
					if !matches!(symbol.kind, SymbolKind::Let | SymbolKind::Static) {
						return None;
					}
					let fa_analyzer::syntax::View::Declaration {
						value: Some(value), ..
					} = self.view(symbol.decl_node)
					else {
						return None;
					};
					node = value;
				}
				_ => return None,
			}
		}
		None
	}

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
			(Value::String(text), Repr::Str) => self.text_constant(text),
			(Value::Bytes(bytes), Repr::Bytes) => {
				let pointer = self.constant_bytes(bytes);
				let i64 = self.context.i64_type();
				let count = i64.const_int(bytes.len() as u64, false);
				let header = self.context.const_struct(
					&[
						pointer.into(),
						count.into(),
						count.into(),
						i64.const_int(super::arrays::ARRAY_STATIC_MARKER, false)
							.into(),
					],
					false,
				);
				Val::new(
					Repr::Bytes,
					self.readonly_global(header.into(), "fa.bytes.static"),
				)
			}
			(Value::None, Repr::None) => Val::none(),
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
			// an empty pool or columnar (a field default of a type of another file)
			(Value::Array(items), Repr::Pool(item_type) | Repr::Columnar(item_type))
				if items.is_empty() =>
			{
				let element = self.item_repr(item_type, node)?;
				let pool = self.collection_new(&self.builder, repr, element)?;
				Val::new(repr, pool)
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
			(Value::Array(items), Repr::Tuple(inline)) if self.inline_info(inline).is_some() => {
				let (item_type, capacity) = self.inline_info(inline).expect("an inline array");
				if items.len() > capacity as usize {
					return Err(self.unsupported(
						node.index(),
						"an inline-array constant exceeding its declared capacity",
					));
				}
				let slot = self.alloca(repr, "inline.constant")?;
				for (index, value) in items.iter().enumerate() {
					let destination = self.inline_item(
						&self.builder,
						slot,
						inline,
						self.context.i64_type().const_int(index as u64, false),
					)?;
					let value = self.constant_value(value, item_type, node)?;
					self.store(destination, value, node)?;
				}
				self.set_inline_length(
					slot,
					inline,
					self.context.i64_type().const_int(items.len() as u64, false),
				)?;
				self.load(slot)?
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
			// a member of an enumeration: its index
			(Value::Member(member), Repr::Enum(nominal)) => {
				let Some(index) = self.member_index(nominal, &member.name) else {
					return Err(unsupported(self));
				};
				Val::new(
					repr,
					self.context.i32_type().const_int(u64::from(index), false),
				)
			}
			// a member of a named union or of an `Errors` type: a box
			(Value::Member(member), Repr::Union | Repr::InlineUnion(_)) => {
				let nominal = match self.types().kind(ty) {
					TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => nominal,
					_ => return Err(unsupported(self)),
				};
				let Some(index) = self.member_index(nominal, &member.name) else {
					return Err(unsupported(self));
				};
				let data = match (&member.data, self.member_data(nominal, index)) {
					(Some(data), Some(data_type)) => {
						let value = self.constant_value(data, data_type, node)?;
						Some(if self.needs_drop(value.repr) {
							self.fresh(value)
						} else {
							value
						})
					}
					(None, None) => None,
					_ => return Err(unsupported(self)),
				};
				let boxed = self.build_member(nominal, index, data, node)?;
				// the caller owns it
				if self.is_temporary(boxed) {
					self.take(boxed, None)?
				} else {
					boxed
				}
			}
			(
				Value::Range {
					start,
					end,
					inclusive,
				},
				Repr::Range,
			) => {
				let i64 = self.context.i64_type();
				let mut flags = if *inclusive {
					super::tuples::INCLUSIVE
				} else {
					0
				};
				let mut bound = |bound: &Option<ethnum::I256>, flag: u64| match bound {
					Some(value) => {
						flags |= flag;
						self.const_int(64, *value)
					}
					None => i64.const_zero(),
				};
				let start = bound(start, super::tuples::HAS_START);
				let end = bound(end, super::tuples::HAS_END);
				let flags = self.context.i8_type().const_int(flags, false);
				let structure = self.range_type();
				Val::new(
					repr,
					structure.const_named_struct(&[start.into(), end.into(), flags.into()]),
				)
			}
			(Value::Map(entries), Repr::Map(_)) => {
				let table = self.table_of(repr)?;
				let TypeKind::Collection(_, value_type, key_type) = self.types().kind(ty) else {
					return Err(unsupported(self));
				};
				let capacity = self
					.context
					.i64_type()
					.const_int(entries.len() as u64, false);
				let pointer = self.table_new(table, capacity)?;
				for (key, value) in entries.iter() {
					let key = self.constant_value(key, key_type, node)?;
					let key = self.convert(key, table.key, node.index())?;
					let value = self.constant_value(value, value_type, node)?;
					let value =
						self.convert(value, table.value.unwrap_or(Repr::Bool), node.index())?;
					self.table_insert(pointer, table, key, Some(value), node)?;
				}
				Val::new(repr, pointer)
			}
			(Value::Set(items), Repr::Set(_)) => {
				let table = self.table_of(repr)?;
				let TypeKind::Collection(_, item_type, _) = self.types().kind(ty) else {
					return Err(unsupported(self));
				};
				let capacity = self.context.i64_type().const_int(items.len() as u64, false);
				let pointer = self.table_new(table, capacity)?;
				for item in items.iter() {
					let item = self.constant_value(item, item_type, node)?;
					let item = self.convert(item, table.key, node.index())?;
					self.table_insert(pointer, table, item, None, node)?;
				}
				Val::new(repr, pointer)
			}
			_ => return Err(unsupported(self)),
		})
	}

	/// The index of a member of a nominal type, from its qualified name
	/// (`Color.red`, `Shape.Circle`).
	fn member_index(&self, nominal: fa_analyzer::ids::NominalId, name: &str) -> Option<u16> {
		let types = self.types();
		let short = name.rsplit('.').next().unwrap_or(name);
		types
			.sym_list(types.nominal(nominal).members)
			.iter()
			.position(|&member| types.syms.resolve(member) == short)
			.map(|index| index as u16)
	}

	/// Whether a compile-time value belongs to a type: literal types are
	/// compared with the value, the others accept it.
	fn value_has_type(&self, value: &Value, ty: TypeId) -> bool {
		let types = self.types();
		match (value, types.kind(ty)) {
			(Value::Integer(integer), TypeKind::Int(_)) => types
				.as_int(ty)
				.is_some_and(|int| int.min <= *integer && *integer <= int.max),
			(Value::String(text), TypeKind::StringLit(sym)) => {
				text.as_str() == Some(types.syms.resolve(sym))
			}
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
		// over a key set: its keys, and the body's analysis for each (fields.rs)
		let key_loop = self.analysis.key_loops.contains(&node);
		let key_analyses = if key_loop {
			self.key_loop_analyses(node, iterable)
		} else {
			None
		};
		let items = match (&key_analyses, self.analysis.unrolled.get(&node)) {
			(Some((keys, _)), _) => keys.iter().map(|key| Value::string(key)).collect(),
			(None, Some(items)) => items.clone(),
			(None, None) if key_loop => {
				return Err(self.unsupported(node.index(), "this `@for` over a key set"));
			}
			(None, None) => return self.for_loop(node, pattern, has_index, iterable, body),
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
		let results = self.analysis.unrolled_items.get(&node).cloned();
		for (position, item) in items.iter().enumerate() {
			// the branches and values of this item (an `@for` analyzed per item)
			let per_item = results
				.as_ref()
				.and_then(|results| results.get(position))
				.cloned();
			let overlay = per_item.is_some();
			if let Some(per_item) = per_item {
				self.item_results.push(per_item);
			}
			let mark = self.mark();
			if key_loop
				&& let Some(symbol) = variable
				&& let Some(key) = item.as_str()
			{
				self.current_keys.insert(symbol, key.to_string());
				// the locals of the body may have another type for this key
				self.forget_body_slots(body);
			}
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
			// the body as analyzed for this key
			let main = self.analysis;
			if let Some((_, analyses)) = &key_analyses
				&& let Some(analysis) = analyses.get(position)
			{
				self.analysis = analysis;
			}
			let result = self.block(body, false);
			self.analysis = main;
			if overlay {
				self.item_results.pop();
			}
			let result = result?;
			if result.repr == Repr::Never {
				return Ok(result);
			}
			self.end_scope(mark, None)?;
		}
		Ok(Val::void())
	}

	/// The branch an `@if` keeps: the item's being lowered, else the analysis'.
	pub(crate) fn comptime_branch(&self, node: NodeId) -> Option<bool> {
		self.item_results
			.iter()
			.rev()
			.find_map(|results| results.branches.get(&node).copied())
			.or_else(|| self.analysis.comptime_branch(node))
	}

	/// The compile-time value of a node: the item's being lowered, else the analysis'.
	pub(crate) fn comptime_value(&self, node: NodeId) -> Option<Value> {
		self.item_results
			.iter()
			.rev()
			.find_map(|results| results.constants.get(&node).cloned())
			.or_else(|| self.analysis.constant(node).cloned())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Target;
	use fa_analyzer::Session;
	use fa_analyzer::types::CollKind;
	use inkwell::context::Context;

	#[test]
	fn oversized_inline_constants_are_rejected_before_storage_writes() {
		for target in [Target::Native, Target::Wasm, Target::NativeFreestanding] {
			let mut session = Session::new();
			let file = session.set_source("main", std::sync::Arc::from("none"));
			session.analyze_file(file);
			let capacity = session.types.int_literal(1.into());
			let ty = session
				.types
				.collection(CollKind::InlineArray, TypeId::INTEGER, capacity);
			let context = Context::create();
			let module = context.create_module("invalid-inline-constant");
			let function =
				module.add_function("test", context.void_type().fn_type(&[], false), None);
			let usize = if target == Target::Wasm {
				context.i32_type()
			} else {
				context.i64_type()
			};
			let mut lowering = Lowering::new(
				&session,
				&[(file, "test.fa".into())],
				file,
				vec![],
				&context,
				&module,
				target,
				usize,
			);
			lowering
				.builder
				.position_at_end(context.append_basic_block(function, "entry"));
			let result = lowering.constant_value(
				&Value::array(vec![Value::int(7), Value::int(8)]),
				ty,
				NodeId::new(0),
			);
			assert!(
				matches!(result, Err(Error::Unsupported { message, .. }) if message.contains("exceeding its declared capacity"))
			);
			assert!(
				!module
					.print_to_string()
					.to_string()
					.contains("inline.constant")
			);
		}
	}
}
