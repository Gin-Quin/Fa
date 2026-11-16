//! `std.json` natively; the runtime part is the JSON section of
//! `runtime/fa_runtime.c`.
//!
//! - **`Json.parse(text)`** passes the text and the schema of the expected
//!   type (`json_schema`, the format is described in the runtime) to
//!   `fa_json_parse`, which reads the text into a tree and checks it with
//!   the messages of the JavaScript runtime's `faJsonParse`, then calls the
//!   reader of the type (`fa.json.read`, one function per type), which
//!   builds the value from the tree: it fits, so building never fails. A
//!   member of a union is the first that fits (`fa_json_fits`).
//! - **`Json.stringify(value)`** appends the value to the runtime's text
//!   builder: the writer of each object and collection type is a function
//!   (`fa.json.write`). As `faJsonStringify`: `none` is `null`, except for a
//!   field or a map entry, which is left out; a number that isn't finite is
//!   `null`; maps are objects.

use ethnum::I256;
use fa_analyzer::analysis::CallInfo;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::{CollKind, TypeKind};
use inkwell::IntPredicate;
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

/// The deepest type a schema describes (as `json_schema` of `fa-codegen-js`).
const MAX_DEPTH: u32 = 32;

impl<'ctx> Lowering<'_, 'ctx> {
	/// `Json.parse(text)` and `Json.stringify(value)`.
	pub(crate) fn json_builtin(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		match path {
			"Json.parse" => self.json_parse(node, info),
			_ => self.json_stringify(node, info),
		}
	}

	fn json_parse(&mut self, node: NodeId, info: &CallInfo) -> Result<Val<'ctx>, Error> {
		let ty = self.type_of(node);
		let mut schema = Vec::new();
		if !self.json_schema(ty, 0, &mut schema) {
			return Err(self.unsupported(node.index(), "`Json.parse` of this type"));
		}
		let Some(values) = self.io_arguments(info)? else {
			return Ok(Val::never());
		};
		let Some(&text) = values.first() else {
			return Err(self.unsupported(node.index(), "this `Json.parse`"));
		};
		schema.push(0);
		let schema = self.constant_bytes(&schema);
		let (bytes, length) = self.string_parts(text)?;
		let out = self.alloca_type(self.pointer_type().into(), "json")?;
		let error = self
			.call_value(
				self.runtime.function("fa_json_parse")?,
				&[bytes.into(), length.into(), schema.into(), out.into()],
			)?
			.into_pointer_value();
		self.raise_if_error(error)?;
		let document = self
			.builder
			.build_load(self.pointer_type(), out, "")?
			.into_pointer_value();
		// the root is the document's first field
		let value = self.json_value(document, ty, node)?;
		self.call_runtime("fa_json_free", &[document.into()])?;
		Ok(self.fresh(value))
	}

	fn json_stringify(&mut self, node: NodeId, info: &CallInfo) -> Result<Val<'ctx>, Error> {
		let arguments = Self::io_given(info);
		let Some(&argument) = arguments.first() else {
			return Err(self.unsupported(node.index(), "this `Json.stringify`"));
		};
		let value = self.expr(argument)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let ty = self.type_of(argument);
		let repr = self.repr_of(ty, node.index())?;
		let value = if value.repr == repr || !repr.has_value() {
			value
		} else {
			self.convert(value, repr, node.index())?
		};
		// every part is evaluated: the builder is free
		self.call_runtime("fa_b_reset", &[])?;
		self.json_write(value, ty, node)?;
		let out = self.alloca(Repr::Str, "json")?;
		self.call_runtime("fa_b_take", &[out.pointer.into()])?;
		let text = self.load(out)?;
		Ok(self.fresh(text))
	}

	// ── schemas

	/// Writes the schema of `ty` (the format of the runtime's JSON section);
	/// false when JSON can't describe it.
	fn json_schema(&self, ty: TypeId, depth: u32, out: &mut Vec<u8>) -> bool {
		if depth > MAX_DEPTH {
			return false;
		}
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Boolean => out.push(b'b'),
			TypeKind::BoolLit(value) => out.push(if value { b'T' } else { b'F' }),
			TypeKind::String | TypeKind::Character => out.push(b's'),
			TypeKind::StringLit(sym) => {
				let text = types.syms.resolve(sym);
				out.extend_from_slice(format!("S{};", text.len()).as_bytes());
				out.extend_from_slice(text.as_bytes());
			}
			TypeKind::Float(_) | TypeKind::FloatLit(..) => out.push(b'f'),
			TypeKind::None => out.push(b'n'),
			TypeKind::Int(id) => {
				let int = *types.int(id);
				let lowest = I256::from(i64::MIN);
				let highest = I256::from(i64::MAX);
				if int.min == int.max {
					if int.min < lowest || int.min > highest {
						return false;
					}
					out.extend_from_slice(format!("I{};", int.min).as_bytes());
					return true;
				}
				// the type's bounds, within its storage's (and an `Int64`'s)
				let range = int.storage.range();
				let min = int.min.max(range.lo);
				let max = int.max.min(range.hi);
				out.push(b'i');
				if min > lowest {
					out.extend_from_slice(min.min(highest).to_string().as_bytes());
				}
				out.push(b';');
				if max < highest {
					out.extend_from_slice(max.max(lowest).to_string().as_bytes());
				}
				out.push(b';');
			}
			TypeKind::Collection(CollKind::InlineArray, item, _) => {
				let Some((_, capacity)) = types.inline_array(ty) else {
					return false;
				};
				out.extend_from_slice(format!("A{capacity};").as_bytes());
				return self.json_schema(item, depth + 1, out);
			}
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => {
				out.push(b'a');
				return self.json_schema(item, depth + 1, out);
			}
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, value, _) => {
				out.push(b'm');
				return self.json_schema(value, depth + 1, out);
			}
			TypeKind::Union(list) => {
				let members = types.list_items(list).to_vec();
				out.extend_from_slice(format!("u{};", members.len()).as_bytes());
				for member in members {
					if !self.json_schema(member, depth + 1, out) {
						return false;
					}
				}
			}
			// a named union: its members tagged by `"type"` (`fa_runtime.c`, `JsonMember`)
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _)
				if matches!(
					types.nominal(nominal).kind,
					fa_analyzer::types::NominalKind::Union
						| fa_analyzer::types::NominalKind::Errors
				) =>
			{
				let members = self.json_members(ty);
				out.extend_from_slice(format!("N{};", members.len()).as_bytes());
				for (nominal, index) in members {
					let name = self.types().member_name(nominal, index).to_string();
					out.extend_from_slice(format!("{};", name.len()).as_bytes());
					out.extend_from_slice(name.as_bytes());
					let kind = self.json_member_kind(nominal, index);
					out.push(kind);
					if kind != b'e' {
						let data = self.types().payload(nominal, index);
						if !self.json_schema(data, depth + 1, out) {
							return false;
						}
					}
				}
			}
			// a set of flags: an array of its accepted names, checked as the
			// names of an enumeration (fields.rs)
			TypeKind::Fields(_) => {
				out.push(b'a');
				out.extend_from_slice(&fields_names_schema(types, ty));
			}
			// an enumeration: `E<count>;` then `<length>;<name>` and `p<value>;`
			// (pinned) or `n` (written as its name)
			TypeKind::Enum(..) | TypeKind::EnumMember(..) => {
				let members = self.json_enum_members(ty);
				out.extend_from_slice(format!("E{};", members.len()).as_bytes());
				for (nominal, index) in members {
					let name = types.member_name(nominal, index);
					out.extend_from_slice(format!("{};", name.len()).as_bytes());
					out.extend_from_slice(name.as_bytes());
					match self.json_pinned(nominal, index) {
						Some(value) => out.extend_from_slice(format!("p{value};").as_bytes()),
						None => out.push(b'n'),
					}
				}
			}
			TypeKind::Object(_) => {
				let fields = self.json_fields(ty);
				out.extend_from_slice(format!("o{};", fields.len()).as_bytes());
				for (name, field_type) in fields {
					out.extend_from_slice(format!("{};", name.len()).as_bytes());
					out.extend_from_slice(name.as_bytes());
					// may be absent: optional, or with a default value
					let optional = types.union_members(&field_type).contains(&TypeId::NONE)
						|| self.json_default(ty, &name).is_some()
						|| types
							.fields_of(ty)
							.iter()
							.find(|field| types.syms.resolve(field.name) == name)
							.is_some_and(|field| {
								self.analysis.json_defaults.contains_key(&(ty, field.name))
							});
					out.push(if optional { b'1' } else { b'0' });
					if !self.json_schema(field_type, depth + 1, out) {
						return false;
					}
				}
			}
			_ => return false,
		}
		true
	}

	/// The members of an enumeration type (or the one of a member).
	fn json_enum_members(&self, ty: TypeId) -> Vec<(fa_analyzer::ids::NominalId, u16)> {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::EnumMember(nominal, index) => vec![(nominal, index)],
			TypeKind::Enum(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				(0..count)
					.filter(|&index| types.mask_has(mask, index))
					.map(|index| (nominal, index))
					.collect()
			}
			_ => Vec::new(),
		}
	}

	/// The pinned value of a member of an enumeration (its literal payload).
	fn json_pinned(&self, nominal: fa_analyzer::ids::NominalId, index: u16) -> Option<I256> {
		self.types()
			.as_int(self.types().payload(nominal, index))
			.filter(|int| int.min == int.max)
			.map(|int| int.min)
	}

	/// The members of a named union type (or the one of a member type).
	fn json_members(&self, ty: TypeId) -> Vec<(fa_analyzer::ids::NominalId, u16)> {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Member(nominal, index) => vec![(nominal, index)],
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				(0..count)
					.filter(|&index| types.mask_has(mask, index))
					.map(|index| (nominal, index))
					.collect()
			}
			_ => Vec::new(),
		}
	}

	/// How a member is written: `e` without data, `l` a literal, `r` an
	/// object next to `"type"`, `v` data under `"value"` (an object with
	/// its own `type` field too).
	fn json_member_kind(&self, nominal: fa_analyzer::ids::NominalId, index: u16) -> u8 {
		let types = self.types();
		let data = types.payload(nominal, index);
		if data.is_unset() {
			return b'e';
		}
		if types.is_literal(data) {
			return b'l';
		}
		match types.kind(data) {
			TypeKind::Object(_)
				if !types
					.fields_of(data)
					.iter()
					.any(|field| types.syms.resolve(field.name) == "type") =>
			{
				b'r'
			}
			_ => b'v',
		}
	}

	/// The default value of a field of an object type declared in this
	/// file, given to a value read without the field.
	fn json_default(&self, ty: TypeId, name: &str) -> Option<NodeId> {
		let declaration = self
			.analysis
			.type_decls
			.values()
			.find(|decl| decl.ty == ty)?;
		declaration
			.fields
			.iter()
			.find(|field| self.types().syms.resolve(field.name) == name)?
			.default
	}

	/// The fields of an object type: the declaration's order, then the
	/// others (the shape's order).
	fn json_fields(&self, ty: TypeId) -> Vec<(String, TypeId)> {
		let types = self.types();
		let shape = types.fields_of(ty);
		let mut order: Vec<usize> = Vec::new();
		let declared: Vec<fa_analyzer::ids::Sym> = match self
			.analysis
			.type_decls
			.values()
			.find(|declaration| declaration.ty == ty)
		{
			Some(declaration) => declaration.fields.iter().map(|field| field.name).collect(),
			// an object written inline as a member's data: as written
			None => self
				.analysis
				.inline_field_orders
				.get(&ty)
				.cloned()
				.unwrap_or_default(),
		};
		for name in declared {
			if let Some(index) = shape.iter().position(|entry| entry.name == name)
				&& !order.contains(&index)
			{
				order.push(index);
			}
		}
		for index in 0..shape.len() {
			if !order.contains(&index) {
				order.push(index);
			}
		}
		order
			.into_iter()
			.map(|index| {
				(
					types.syms.resolve(shape[index].name).to_string(),
					shape[index].ty,
				)
			})
			.collect()
	}

	// ── reading

	/// The value of type `ty` that a node of the tree holds (it fits): owned,
	/// not registered.
	fn json_value(
		&mut self,
		tree: PointerValue<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.repr_of(ty, call.index())?;
		if !repr.has_value() {
			return Ok(Val::none());
		}
		let reader = self.json_reader(ty, repr, call)?;
		let value = self.call_value(reader, &[tree.into()])?;
		Ok(Val::new(repr, value))
	}

	/// The reader of a type: `fa.json.read(node) -> value`.
	fn json_reader(
		&mut self,
		ty: TypeId,
		repr: Repr,
		call: NodeId,
	) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("json read {:?} {ty:?}", self.file);
		if let Some(function) = self.drop_functions.borrow().get(&key).copied() {
			return Ok(function);
		}
		let signature = self
			.llvm_type(repr)
			.fn_type(&[self.pointer_type().into()], false);
		let function = self
			.module
			.add_function("fa.json.read", signature, Some(Linkage::Internal));
		self.runtime.add_attributes(function, &["nounwind"]);
		self.drop_functions.borrow_mut().insert(key, function);
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, repr, None);
		let tree = function
			.get_nth_param(0)
			.expect("a node")
			.into_pointer_value();
		let built = self.json_build(tree, ty, repr, call);
		let result = built.and_then(|value| {
			self.end_scope(0, None)?;
			self.builder
				.build_return(Some(&value.value.expect("a value")))?;
			Ok(())
		});
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		result?;
		Ok(function)
	}

	/// A value of a named union: the member the tree is (`fa_json_member`),
	/// with its data read from the tree or its `"value"`.
	fn json_named(
		&mut self,
		tree: PointerValue<'ctx>,
		ty: TypeId,
		repr: Repr,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let mut schema = Vec::new();
		if !self.json_schema(ty, 0, &mut schema) {
			return Err(self.unsupported(call.index(), "`Json.parse` of this union"));
		}
		schema.push(0);
		let schema = self.constant_bytes(&schema);
		let position = self
			.call_value(
				self.runtime.function("fa_json_member")?,
				&[tree.into(), schema.into()],
			)?
			.into_int_value();
		let slot = self.alloca(repr, "json_member")?;
		let join = self.append_block("json_member_read");
		for (at, (nominal, index)) in self.json_members(ty).into_iter().enumerate() {
			let is = self.builder.build_int_compare(
				IntPredicate::EQ,
				position,
				position.get_type().const_int(at as u64, false),
				"",
			)?;
			let read = self.append_block("json_member");
			let next = self.append_block("json_next_member");
			self.builder.build_conditional_branch(is, read, next)?;
			self.builder.position_at_end(read);
			let data = match self.json_member_kind(nominal, index) {
				b'e' | b'l' => None,
				kind => {
					let source = if kind == b'r' {
						tree
					} else {
						let name = self.constant_bytes(b"value");
						let length = self.usize.const_int(5, false);
						self.call_value(
							self.runtime.function("fa_json_field")?,
							&[tree.into(), name.into(), length.into()],
						)?
						.into_pointer_value()
					};
					let data_type = self.types().payload(nominal, index);
					Some(self.json_value(source, data_type, call)?)
				}
			};
			let member = self.build_member(nominal, index, data, call)?;
			let member = self.take(member, None)?;
			let member = self.convert(member, repr, call.index())?;
			self.builder
				.build_store(slot.pointer, member.value.expect("a member"))?;
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(next);
		}
		self.builder.build_unreachable()?;
		self.builder.position_at_end(join);
		self.load(slot)
	}

	/// The body of a reader.
	fn json_build(
		&mut self,
		tree: PointerValue<'ctx>,
		ty: TypeId,
		repr: Repr,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		if !self.json_members(ty).is_empty() {
			return self.json_named(tree, ty, repr, call);
		}
		// a set of flags: the bits of the names of the array (fields.rs)
		if self.is_fields(ty) {
			return self.json_read_fields(tree, ty, repr);
		}
		// an enumeration: the member the tree names (`fa_json_enum`)
		if let [(nominal, _), ..] = self.json_enum_members(ty).as_slice() {
			let nominal = *nominal;
			let mut schema = Vec::new();
			if !self.json_schema(ty, 0, &mut schema) {
				return Err(self.unsupported(call.index(), "`Json.parse` of this enumeration"));
			}
			schema.push(0);
			let schema = self.constant_bytes(&schema);
			let position = self
				.call_value(
					self.runtime.function("fa_json_enum")?,
					&[tree.into(), schema.into()],
				)?
				.into_int_value();
			// the position among the members of the type → the member's index
			let i32 = self.context.i32_type();
			let mut value = i32.const_zero();
			for (at, (_, index)) in self.json_enum_members(ty).into_iter().enumerate().rev() {
				let is = self.builder.build_int_compare(
					IntPredicate::EQ,
					position,
					position.get_type().const_int(at as u64, false),
					"",
				)?;
				value = self
					.builder
					.build_select(is, i32.const_int(u64::from(index), false), value, "")?
					.into_int_value();
			}
			let member = Val::new(Repr::Enum(nominal), value);
			return self.convert(member, repr, call.index());
		}
		match self.types().kind(ty) {
			TypeKind::Boolean | TypeKind::BoolLit(_) => {
				let flag = self
					.call_value(self.runtime.function("fa_json_boolean")?, &[tree.into()])?
					.into_int_value();
				let value = self.builder.build_int_compare(
					IntPredicate::NE,
					flag,
					flag.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, value))
			}
			TypeKind::Int(_) => {
				let Repr::Int(int) = repr else {
					return Err(self.unsupported(call.index(), "`Json.parse` of this integer"));
				};
				let value = self
					.call_value(self.runtime.function("fa_json_integer")?, &[tree.into()])?
					.into_int_value();
				let value = self.int_to(value, IntRepr::I64, int)?;
				Ok(Val::new(repr, value))
			}
			TypeKind::Float(_) | TypeKind::FloatLit(..) => {
				let value = self
					.call_value(self.runtime.function("fa_json_number")?, &[tree.into()])?
					.into_float_value();
				Ok(match repr {
					Repr::F32 => Val::new(
						repr,
						self.builder
							.build_float_trunc(value, self.context.f32_type(), "")?,
					),
					_ => Val::new(Repr::F64, value),
				})
			}
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => {
				let out = self.alloca(Repr::Str, "json_string")?;
				self.call_runtime("fa_json_string", &[tree.into(), out.pointer.into()])?;
				self.load(out)
			}
			TypeKind::Collection(CollKind::InlineArray, item_type, _) => {
				let count = self
					.call_value(self.runtime.function("fa_json_count")?, &[tree.into()])?
					.into_int_value();
				let slot = self.alloca(repr, "inline.json")?;
				self.set_inline_length(slot, ty, self.context.i64_type().const_zero())?;
				self.for_each_index(count, |this, index| {
					let child = this
						.call_value(
							this.runtime.function("fa_json_item")?,
							&[tree.into(), index.into()],
						)?
						.into_pointer_value();
					let value = this.json_value(child, item_type, call)?;
					let destination = this.inline_item(&this.builder, slot, ty, index)?;
					let value = this.json_adapt(value, destination.repr, call)?;
					this.store(destination, value, call)
				})?;
				self.set_inline_length(slot, ty, count)?;
				self.load(slot)
			}
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item_type, _) => {
				let item = self.item_repr(item_type, call)?;
				if !item.has_value() {
					return Err(self.unsupported(call.index(), "`Json.parse` of this array"));
				}
				let count = self
					.call_value(self.runtime.function("fa_json_count")?, &[tree.into()])?
					.into_int_value();
				let array = self.new_array(item, count, count)?;
				let (items, _) = self.array_parts(array)?;
				let item_function = self.runtime.function("fa_json_item")?;
				self.for_each_index(count, |this, index| {
					let child = this
						.call_value(item_function, &[tree.into(), index.into()])?
						.into_pointer_value();
					let value = this.json_value(child, item_type, call)?;
					let value = this.json_adapt(value, item, call)?;
					let slot = this.item_pointer(items, item, index)?;
					this.builder
						.build_store(slot, value.value.expect("an item"))?;
					Ok(())
				})?;
				if let Repr::Growing(_) = repr {
					let growing = self.growing_from_array(array, item)?;
					return Ok(Val::new(repr, growing));
				}
				Ok(Val::new(repr, array))
			}
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, value_type, _) => {
				let table = self.table_of(repr)?;
				let Some(value_repr) = table.value.filter(|value| value.has_value()) else {
					return Err(self.unsupported(call.index(), "`Json.parse` of this map"));
				};
				let count = self
					.call_value(self.runtime.function("fa_json_count")?, &[tree.into()])?
					.into_int_value();
				let pointer = self.table_new(table, i64.const_zero())?;
				let item_function = self.runtime.function("fa_json_item")?;
				let key_function = self.runtime.function("fa_json_key")?;
				self.for_each_index(count, |this, index| {
					let member = this
						.call_value(item_function, &[tree.into(), index.into()])?
						.into_pointer_value();
					let key = this.alloca(Repr::Str, "json_key")?;
					this.builder.build_call(
						key_function,
						&[member.into(), key.pointer.into()],
						"",
					)?;
					let key = this.load(key)?;
					let key = this.json_adapt(key, table.key, call)?;
					let value = this.json_value(member, value_type, call)?;
					let value = this.json_adapt(value, value_repr, call)?;
					this.table_insert(pointer, table, key, Some(value), call)?;
					Ok(())
				})?;
				Ok(Val::new(repr, pointer))
			}
			TypeKind::Object(_) => self.json_object(tree, ty, call),
			TypeKind::Union(_) => self.json_union(tree, ty, repr, call),
			_ => Err(self.unsupported(call.index(), "`Json.parse` of this type")),
		}
	}

	/// An object: its fields, read from the members of their names (a
	/// missing optional field is `none`).
	fn json_object(
		&mut self,
		tree: PointerValue<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let structure = self.object_type(ty);
		let shape: Vec<(fa_analyzer::ids::Sym, TypeId)> = self
			.types()
			.fields_of(ty)
			.iter()
			.map(|field| (field.name, field.ty))
			.collect();
		let field_function = self.runtime.function("fa_json_field")?;
		let mut object = structure.get_undef();
		for (index, (name, field_type)) in shape.into_iter().enumerate() {
			let repr = self.field_repr(ty, index, call.index())?;
			if !repr.has_value() {
				continue;
			}
			let field_name = name;
			let name = self.types().syms.resolve(name).to_string();
			let bytes = self.constant_bytes(name.as_bytes());
			let length = self.usize.const_int(name.len() as u64, false);
			let member = self
				.call_value(field_function, &[tree.into(), bytes.into(), length.into()])?
				.into_pointer_value();
			let optional = self
				.types()
				.union_members(&field_type)
				.contains(&TypeId::NONE);
			let default = self.json_default(ty, &name);
			// a type of another file: the constant its file evaluated
			let constant = self.analysis.json_defaults.get(&(ty, field_name)).copied();
			let value = if optional || default.is_some() || constant.is_some() {
				// absent: its default value, else `none`
				let slot = self.alloca(repr, "json_field")?;
				let absent = self.builder.build_is_null(member, "")?;
				let missing = self.append_block("json_missing");
				let present = self.append_block("json_present");
				let join = self.append_block("json_field");
				self.builder
					.build_conditional_branch(absent, missing, present)?;
				self.builder.position_at_end(missing);
				let initial = match (default, constant) {
					(Some(default), _) => {
						let value = self.expr(default)?;
						self.owned_as(value, repr, Some(default))?
					}
					(None, Some(constant)) => {
						let Some(value) = self
							.analysis
							.field_constants
							.get(constant as usize)
							.cloned()
						else {
							return Err(self.unsupported(call.index(), "this default value"));
						};
						let built = self.constant_value(&value, field_type, call)?;
						self.owned_as(built, repr, None)?
					}
					(None, None) => {
						let none = Val::none();
						self.json_adapt(none, repr, call)?
					}
				};
				self.builder
					.build_store(slot.pointer, initial.value.expect("a field"))?;
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(present);
				let value = self.json_value(member, field_type, call)?;
				let value = self.json_adapt(value, repr, call)?;
				self.builder
					.build_store(slot.pointer, value.value.expect("a field"))?;
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(join);
				self.load(slot)?
			} else {
				let value = self.json_value(member, field_type, call)?;
				self.json_adapt(value, repr, call)?
			};
			object = self
				.builder
				.build_insert_value(object, value.value.expect("a field"), index as u32, "")?
				.into_struct_value();
		}
		Ok(Val::new(Repr::Object(ty), object))
	}

	/// A union: its first member that fits (the last one fits, if the others don't).
	fn json_union(
		&mut self,
		tree: PointerValue<'ctx>,
		ty: TypeId,
		repr: Repr,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let TypeKind::Union(list) = self.types().kind(ty) else {
			unreachable!("a union")
		};
		let members = self.types().list_items(list).to_vec();
		let slot = self.alloca(repr, "json_union")?;
		let join = self.append_block("json_union");
		let fits = self.runtime.function("fa_json_fits")?;
		for (position, &member) in members.iter().enumerate() {
			let next = if position + 1 < members.len() {
				let mut schema = Vec::new();
				if !self.json_schema(member, 0, &mut schema) {
					return Err(self.unsupported(call.index(), "`Json.parse` of this union"));
				}
				schema.push(0);
				let schema = self.constant_bytes(&schema);
				let flag = self
					.call_value(fits, &[tree.into(), schema.into()])?
					.into_int_value();
				let fit = self.builder.build_int_compare(
					IntPredicate::NE,
					flag,
					flag.get_type().const_zero(),
					"",
				)?;
				let taken = self.append_block("json_member");
				let next = self.append_block("json_next");
				self.builder.build_conditional_branch(fit, taken, next)?;
				self.builder.position_at_end(taken);
				Some(next)
			} else {
				None
			};
			let value = self.json_member(tree, ty, member, repr, call)?;
			self.builder
				.build_store(slot.pointer, value.value.expect("a member"))?;
			self.builder.build_unconditional_branch(join)?;
			if let Some(next) = next {
				self.builder.position_at_end(next);
			}
		}
		self.builder.position_at_end(join);
		self.load(slot)
	}

	/// A member of a union read as the union's representation.
	fn json_member(
		&mut self,
		tree: PointerValue<'ctx>,
		union: TypeId,
		member: TypeId,
		repr: Repr,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		match repr {
			Repr::Optional(scalar) => {
				let inner = scalar.repr();
				let (present, value): (_, BasicValueEnum<'ctx>) = if member == TypeId::NONE {
					(
						self.context.bool_type().const_zero(),
						super::zero_of(self.llvm_type(inner)),
					)
				} else {
					let value = self.json_member(tree, union, member, inner, call)?;
					(
						self.context.bool_type().const_int(1, false),
						value.value.expect("a member"),
					)
				};
				let packed = self.pack_optional(&self.builder, scalar, present, value)?;
				Ok(Val::new(repr, packed))
			}
			Repr::Union => {
				let value = self.json_value(tree, member, call)?;
				if value.repr == Repr::Union {
					return Ok(value);
				}
				// boxed as the first item of its representation (`inject`)
				let items = self.union_items(union).unwrap_or_default();
				let mut chosen = None;
				for item in items {
					let item_repr = self.repr_of(item, call.index())?;
					let matches = match (item_repr, value.repr) {
						(Repr::Int(_), Repr::Int(_)) => true,
						(a, b) => a == b,
					};
					if matches {
						chosen = Some((item, item_repr));
						break;
					}
				}
				let Some((item, item_repr)) = chosen else {
					return Err(self.unsupported(call.index(), "`Json.parse` of this union"));
				};
				let value = self.json_adapt(value, item_repr, call)?;
				let id = self.context.i64_type().const_int(self.type_id(item), false);
				let box_type = self.box_type(Some(item_repr));
				let pointer = self.allocate_for(box_type.into())?;
				let id_pointer = self.builder.build_struct_gep(box_type, pointer, 0, "")?;
				self.builder.build_store(id_pointer, id)?;
				let data_pointer = self.builder.build_struct_gep(box_type, pointer, 1, "")?;
				self.builder
					.build_store(data_pointer, value.value.expect("data"))?;
				Ok(Val::new(Repr::Union, pointer))
			}
			_ => {
				let value = self.json_value(tree, member, call)?;
				self.json_adapt(value, repr, call)
			}
		}
	}

	/// A value read as another representation (an integer of another
	/// width, `none` as an optional).
	fn json_adapt(
		&mut self,
		value: Val<'ctx>,
		repr: Repr,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if value.repr == repr {
			return Ok(value);
		}
		match (value.repr, repr) {
			(Repr::Int(from), Repr::Int(to)) => {
				Ok(Val::new(repr, self.int_to(value.int(), from, to)?))
			}
			(Repr::None, Repr::Optional(scalar)) => {
				let inner = super::zero_of(self.llvm_type(scalar.repr()));
				let absent = self.context.bool_type().const_zero();
				Ok(Val::new(
					repr,
					self.pack_optional(&self.builder, scalar, absent, inner)?,
				))
			}
			_ => self.convert(value, repr, call.index()),
		}
	}

	// ── writing

	/// Appends the JSON text of a value of type `ty` to the text builder.
	fn json_write(&mut self, value: Val<'ctx>, ty: TypeId, call: NodeId) -> Result<(), Error> {
		let value = if let Repr::Rec(target) = value.repr
			&& self.compressed_union_link(value.repr).is_some()
		{
			self.unfold_rec(value, target, call.index())?
		} else {
			value
		};
		match self.types().kind(ty) {
			TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character => self.json_write_scalar(value, call),
			TypeKind::None => self.json_text("null"),
			TypeKind::Rec(_) => {
				let Some(target) = self.types().rec_target(ty) else {
					return Err(self.unsupported(call.index(), "`Json.stringify` of this type"));
				};
				let value = match value.repr {
					Repr::Rec(_) => self.unfold_rec(value, target, call.index())?,
					_ => value,
				};
				self.json_write(value, target, call)
			}
			TypeKind::Union(_) => self.json_write_union(value, ty, call),
			TypeKind::Enum(..) | TypeKind::EnumMember(..) => self.json_write_enum(value, ty),
			TypeKind::Fields(_) => self.json_write_flags(value, ty),
			TypeKind::NamedUnion(..)
			| TypeKind::Member(..)
			| TypeKind::Object(_)
			| TypeKind::Collection(
				CollKind::Array | CollKind::GrowingArray | CollKind::Map | CollKind::GrowingMap,
				..,
			) => {
				let writer = self.json_writer(ty, value.repr, call)?;
				self.builder
					.build_call(writer, &[value.value.expect("a value").into()], "")?;
				Ok(())
			}
			_ => Err(self.unsupported(call.index(), "`Json.stringify` of this type")),
		}
	}

	/// A boolean, a number or a string.
	fn json_write_scalar(&mut self, value: Val<'ctx>, call: NodeId) -> Result<(), Error> {
		match value.repr {
			Repr::Bool | Repr::Int(_) => self.append_value(value, call),
			Repr::F64 => {
				self.call_runtime("fa_b_json_number", &[value.value.expect("a float").into()])
			}
			Repr::F32 => {
				let wide = self.builder.build_float_ext(
					value.value.expect("a float").into_float_value(),
					self.context.f64_type(),
					"",
				)?;
				self.call_runtime("fa_b_json_number", &[wide.into()])
			}
			Repr::Str => {
				let (bytes, length) = self.string_parts(value)?;
				self.call_runtime("fa_b_json_string", &[bytes.into(), length.into()])
			}
			_ => Err(self.unsupported(call.index(), "`Json.stringify` of this value")),
		}
	}

	fn json_text(&mut self, text: &str) -> Result<(), Error> {
		let bytes = self.constant_bytes(text.as_bytes());
		let length = self.usize.const_int(text.len() as u64, false);
		self.call_runtime("fa_b_bytes", &[bytes.into(), length.into()])
	}

	/// A value of a union: its member's text.
	fn json_write_union(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		match value.repr {
			Repr::None => self.json_text("null"),
			Repr::Optional(_) => {
				let (present, inner) = self.split_optional(&self.builder, value)?;
				let some = self.append_block("json_some");
				let none = self.append_block("json_none");
				let join = self.append_block("json_written");
				self.builder.build_conditional_branch(present, some, none)?;
				self.builder.position_at_end(some);
				let inner_type = self.without_none(ty);
				self.json_write(inner, inner_type, call)?;
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(none);
				self.json_text("null")?;
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(join);
				Ok(())
			}
			Repr::Union => {
				let items = self.union_items(ty).unwrap_or_default();
				let join = self.append_block("json_written");
				for item in items {
					let is = self.box_is(value, item)?;
					let taken = self.append_block("json_item");
					let next = self.append_block("json_next");
					self.builder.build_conditional_branch(is, taken, next)?;
					self.builder.position_at_end(taken);
					let item_repr = self.repr_of(item, call.index())?;
					let data = self.unbox(value, item_repr)?;
					self.json_write(data, item, call)?;
					self.builder.build_unconditional_branch(join)?;
					self.builder.position_at_end(next);
				}
				self.json_text("null")?;
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(join);
				Ok(())
			}
			_ => self.json_write_scalar(value, call),
		}
	}

	/// A member of an enumeration: its pinned value, else its name.
	fn json_write_enum(&mut self, value: Val<'ctx>, ty: TypeId) -> Result<(), Error> {
		let index = value.int();
		let join = self.append_block("json_written");
		for (nominal, member) in self.json_enum_members(ty) {
			let is = self.builder.build_int_compare(
				IntPredicate::EQ,
				index,
				index.get_type().const_int(u64::from(member), false),
				"",
			)?;
			let taken = self.append_block("json_enum");
			let next = self.append_block("json_next");
			self.builder.build_conditional_branch(is, taken, next)?;
			self.builder.position_at_end(taken);
			let text = match self.json_pinned(nominal, member) {
				Some(pinned) => pinned.to_string(),
				None => json_quoted(self.types().member_name(nominal, member)),
			};
			self.json_text(&text)?;
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(next);
		}
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// A member of a named union: tagged by `"type"`, its data next to it
	/// (an object) or under `"value"`, a literal member as itself.
	fn json_write_named(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		let members = self.json_members(ty);
		let id = match members.as_slice() {
			[_] => None,
			_ => Some(self.box_id(value)?),
		};
		let join = self.append_block("json_written");
		for (nominal, index) in members {
			let next = self.append_block("json_next");
			if let Some(id) = id {
				let expected = id
					.get_type()
					.const_int(self.member_id(nominal, index), false);
				let is = self
					.builder
					.build_int_compare(IntPredicate::EQ, id, expected, "")?;
				let taken = self.append_block("json_member");
				self.builder.build_conditional_branch(is, taken, next)?;
				self.builder.position_at_end(taken);
			}
			let name = self.types().member_name(nominal, index).to_string();
			let mut tag = String::from("{\"type\":");
			tag.push_str(&json_quoted(&name));
			let data_type = self.types().payload(nominal, index);
			match self.json_member_kind(nominal, index) {
				b'e' => self.json_text(&format!("{tag}}}"))?,
				b'l' => {
					let literal = match self.types().kind(data_type) {
						TypeKind::StringLit(text) => json_quoted(self.types().syms.resolve(text)),
						TypeKind::BoolLit(value) => value.to_string(),
						_ => self
							.types()
							.as_int(data_type)
							.map_or("null".to_string(), |int| int.min.to_string()),
					};
					self.json_text(&literal)?;
				}
				kind => {
					let repr = self.repr_of(data_type, call.index())?;
					let boxed = self.union_view(value)?;
					let pointer = boxed.value.expect("a box").into_pointer_value();
					let loaded =
						self.member_payload_value(&self.builder, pointer, nominal, index, repr)?;
					let data = Val::new(repr, loaded);
					if kind == b'r' {
						// the fields after `"type"`
						self.json_text(&tag)?;
						let first = self.json_first()?;
						self.builder
							.build_store(first, self.context.bool_type().const_zero())?;
						self.json_write_fields(first, data, data_type, call)?;
						self.json_text("}")?;
					} else {
						self.json_text(&format!("{tag},\"value\":"))?;
						self.json_write(data, data_type, call)?;
						self.json_text("}")?;
					}
				}
			}
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(next);
		}
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// Memoized writers stop recursive objects and named unions from expanding
	/// their schemas indefinitely while compiling: `fa.json.write(value)`.
	fn json_writer(
		&mut self,
		ty: TypeId,
		repr: Repr,
		call: NodeId,
	) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("json write {:?} {ty:?} {}", self.file, self.shape_key(repr));
		if let Some(function) = self.drop_functions.borrow().get(&key).copied() {
			return Ok(function);
		}
		let signature = self
			.context
			.void_type()
			.fn_type(&[self.llvm_type(repr).into()], false);
		let function =
			self.module
				.add_function("fa.json.write", signature, Some(Linkage::Internal));
		self.runtime.add_attributes(function, &["nounwind"]);
		self.drop_functions.borrow_mut().insert(key, function);
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, Repr::Void, None);
		let value = Val::new(repr, function.get_nth_param(0).expect("a value"));
		let written = self.json_write_body(value, ty, call).and_then(|()| {
			self.end_scope(0, None)?;
			self.builder.build_return(None)?;
			Ok(())
		});
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		written?;
		Ok(function)
	}

	fn json_write_body(&mut self, value: Val<'ctx>, ty: TypeId, call: NodeId) -> Result<(), Error> {
		match (self.types().kind(ty), value.repr) {
			(TypeKind::NamedUnion(..) | TypeKind::Member(..), _) => {
				self.json_write_named(value, ty, call)
			}
			(TypeKind::Object(_), Repr::Object(_)) => self.json_write_object(value, ty, call),
			(TypeKind::Collection(CollKind::InlineArray, item, _), Repr::Tuple(ty)) => {
				let view = self.inline_view(value, ty)?;
				self.json_write_array(view, item, call)
			}
			(TypeKind::Collection(_, item, _), Repr::Array(_)) => {
				self.json_write_array(value, item, call)
			}
			(TypeKind::Collection(_, item, _), Repr::Growing(_)) => {
				let mark = self.mark();
				let view = self.growing_view(value)?;
				self.json_write_array(view, item, call)?;
				self.end_scope(mark, None)
			}
			(TypeKind::Collection(_, value_type, _), Repr::Map(_)) => {
				self.json_write_map(value, value_type, call)
			}
			_ => Err(self.unsupported(call.index(), "`Json.stringify` of this value")),
		}
	}

	/// `"<name>":` after a comma unless `first` (an `i1` slot) is still set.
	fn json_key(
		&mut self,
		first: PointerValue<'ctx>,
		key: Option<Val<'ctx>>,
		name: &str,
	) -> Result<(), Error> {
		let bool_type = self.context.bool_type();
		let is_first = self
			.builder
			.build_load(bool_type, first, "")?
			.into_int_value();
		let comma = self.append_block("json_comma");
		let key_block = self.append_block("json_key");
		self.builder
			.build_conditional_branch(is_first, key_block, comma)?;
		self.builder.position_at_end(comma);
		self.json_text(",")?;
		self.builder.build_unconditional_branch(key_block)?;
		self.builder.position_at_end(key_block);
		self.builder.build_store(first, bool_type.const_zero())?;
		match key {
			Some(key) => {
				let (bytes, length) = self.string_parts(key)?;
				self.call_runtime("fa_b_json_string", &[bytes.into(), length.into()])?;
				self.json_text(":")
			}
			None => {
				let mut quoted = String::new();
				quoted.push('"');
				for character in name.chars() {
					match character {
						'"' => quoted.push_str("\\\""),
						'\\' => quoted.push_str("\\\\"),
						other => quoted.push(other),
					}
				}
				quoted.push_str("\":");
				self.json_text(&quoted)
			}
		}
	}

	/// A slot holding `true`: no member was written yet.
	fn json_first(&mut self) -> Result<PointerValue<'ctx>, Error> {
		let bool_type = self.context.bool_type();
		let first = self.alloca(Repr::Bool, "json_first")?;
		self.builder
			.build_store(first.pointer, bool_type.const_int(1, false))?;
		Ok(first.pointer)
	}

	/// A member `"<key>":<value>`, left out when the value is `none`.
	fn json_write_member(
		&mut self,
		first: PointerValue<'ctx>,
		key: Option<Val<'ctx>>,
		name: &str,
		value: Val<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		match value.repr {
			Repr::None => Ok(()),
			Repr::Optional(_) => {
				let (present, inner) = self.split_optional(&self.builder, value)?;
				let some = self.append_block("json_present");
				let join = self.append_block("json_member_end");
				self.builder.build_conditional_branch(present, some, join)?;
				self.builder.position_at_end(some);
				self.json_key(first, key, name)?;
				let inner_type = self.without_none(ty);
				self.json_write(inner, inner_type, call)?;
				self.builder.build_unconditional_branch(join)?;
				self.builder.position_at_end(join);
				Ok(())
			}
			_ => {
				self.json_key(first, key, name)?;
				self.json_write(value, ty, call)
			}
		}
	}

	fn json_write_object(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		self.json_text("{")?;
		let first = self.json_first()?;
		self.json_write_fields(first, value, ty, call)?;
		self.json_text("}")
	}

	/// The fields of an object, `"<name>":<value>` each (`first`: no comma yet).
	fn json_write_fields(
		&mut self,
		first: PointerValue<'ctx>,
		value: Val<'ctx>,
		ty: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		let structure = value.value.expect("an object").into_struct_value();
		// in declaration order (`json_fields`)
		let shape: Vec<fa_analyzer::ids::Sym> = self
			.types()
			.fields_of(ty)
			.iter()
			.map(|field| field.name)
			.collect();
		for (name, field_type) in self.json_fields(ty) {
			let Some(index) = shape
				.iter()
				.position(|&field| self.types().syms.resolve(field) == name)
			else {
				continue;
			};
			let repr = self.field_repr(ty, index, call.index())?;
			if !repr.has_value() {
				continue;
			}
			let raw = self
				.builder
				.build_extract_value(structure, index as u32, "")?;
			self.json_write_member(first, None, &name, Val::new(repr, raw), field_type, call)?;
		}
		Ok(())
	}

	fn json_write_array(
		&mut self,
		value: Val<'ctx>,
		item_type: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		let Repr::Array(_) = value.repr else {
			return Err(self.unsupported(call.index(), "`Json.stringify` of this array"));
		};
		let item = self.item_repr(item_type, call)?;
		self.json_text("[")?;
		let first = self.json_first()?;
		let array = value.value.expect("an array").into_pointer_value();
		let (_, length) = self.array_parts(array)?;
		self.for_each_item_value(array, item, length, |this, item_value| {
			let bool_type = this.context.bool_type();
			let is_first = this
				.builder
				.build_load(bool_type, first, "")?
				.into_int_value();
			let comma = this.append_block("json_comma");
			let item_block = this.append_block("json_item");
			this.builder
				.build_conditional_branch(is_first, item_block, comma)?;
			this.builder.position_at_end(comma);
			this.json_text(",")?;
			this.builder.build_unconditional_branch(item_block)?;
			this.builder.position_at_end(item_block);
			this.builder.build_store(first, bool_type.const_zero())?;
			this.json_write(item_value, item_type, call)
		})?;
		self.json_text("]")
	}

	fn json_write_map(
		&mut self,
		value: Val<'ctx>,
		value_type: TypeId,
		call: NodeId,
	) -> Result<(), Error> {
		let table = self.table_of(value.repr)?;
		let pointer = value.value.expect("a table").into_pointer_value();
		self.json_text("{")?;
		let first = self.json_first()?;
		self.for_each_entry(pointer, table, |this, entry, _| {
			let key = this.load(this.entry_slot(entry, table, false)?)?;
			let entry_value = this.load(this.entry_slot(entry, table, true)?)?;
			this.json_write_member(first, Some(key), "", entry_value, value_type, call)
		})?;
		self.json_text("}")
	}
}

/// A text as a JSON string (the names of members, literals).
fn json_quoted(text: &str) -> String {
	let mut quoted = String::from('"');
	for character in text.chars() {
		match character {
			'"' => quoted.push_str("\\\""),
			'\\' => quoted.push_str("\\\\"),
			'\n' => quoted.push_str("\\n"),
			'\t' => quoted.push_str("\\t"),
			'\r' => quoted.push_str("\\r"),
			other if (other as u32) < 0x20 => quoted.push_str(&format!("\\u{:04x}", other as u32)),
			other => quoted.push(other),
		}
	}
	quoted.push('"');
	quoted
}

/// The names a set of flags accepts in JSON, as the schema of an
/// enumeration written as its names: `E<count>;` then `<length>;<name>n`.
pub(crate) fn fields_names_schema(types: &fa_analyzer::types::TypeStore, ty: TypeId) -> Vec<u8> {
	let names = types.fields_names(ty);
	let mut out = format!("E{};", names.len()).into_bytes();
	for (name, _) in names {
		out.extend_from_slice(format!("{};", name.len()).as_bytes());
		out.extend_from_slice(name.as_bytes());
		out.push(b'n');
	}
	out
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// A set of flags from its JSON form, an array of names checked by the
	/// schema: the bits of each name (a group: all its flags).
	fn json_read_fields(
		&mut self,
		tree: PointerValue<'ctx>,
		ty: TypeId,
		repr: Repr,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Int(int) = repr else {
			return Ok(Val::new(repr, self.context.i8_type().const_zero()));
		};
		let names = self.types().fields_names(ty);
		let mut schema = fields_names_schema(self.types(), ty);
		schema.push(0);
		let schema = self.constant_bytes(&schema);
		let bits = self.alloca(repr, "json_flags")?;
		let width = self.int_type(int.bits);
		self.builder.build_store(bits.pointer, width.const_zero())?;
		let count = self
			.call_value(self.runtime.function("fa_json_count")?, &[tree.into()])?
			.into_int_value();
		let item_function = self.runtime.function("fa_json_item")?;
		let enum_function = self.runtime.function("fa_json_enum")?;
		self.for_each_index(count, |this, index| {
			let child = this
				.call_value(item_function, &[tree.into(), index.into()])?
				.into_pointer_value();
			let position = this
				.call_value(enum_function, &[child.into(), schema.into()])?
				.into_int_value();
			let mut mask = width.const_zero();
			for (at, (_, bits)) in names.iter().enumerate() {
				let is = this.builder.build_int_compare(
					IntPredicate::EQ,
					position,
					position.get_type().const_int(at as u64, false),
					"",
				)?;
				let value = this.const_int(int.bits, I256::from(*bits));
				mask = this
					.builder
					.build_select(is, value, mask, "")?
					.into_int_value();
			}
			let current = this.load(bits)?.int();
			let next = this.builder.build_or(current, mask, "")?;
			this.builder.build_store(bits.pointer, next)?;
			Ok(())
		})?;
		self.load(bits)
	}

	/// A set of flags as JSON: the array of the dotted paths of its flags.
	fn json_write_flags(&mut self, value: Val<'ctx>, ty: TypeId) -> Result<(), Error> {
		let bits = value.int();
		let width = bits.get_type().get_bit_width();
		self.json_text("[")?;
		for (index, path) in self.types().fields_leaves(ty).iter().enumerate() {
			let bit = self.const_int(width, I256::from(1u128 << index));
			let common = self.builder.build_and(bits, bit, "")?;
			let present = self.builder.build_int_compare(
				IntPredicate::NE,
				common,
				bits.get_type().const_zero(),
				"",
			)?;
			let taken = self.append_block("json_flag");
			let next = self.append_block("json_next");
			self.builder
				.build_conditional_branch(present, taken, next)?;
			self.builder.position_at_end(taken);
			if index > 0 {
				// a comma when a flag before it is written
				let before = self.const_int(
					width,
					I256::from(fa_analyzer::types::store::fields_mask(index as u32)),
				);
				let earlier = self.builder.build_and(bits, before, "")?;
				let separated = self.builder.build_int_compare(
					IntPredicate::NE,
					earlier,
					bits.get_type().const_zero(),
					"",
				)?;
				let comma = self.append_block("json_comma");
				let name = self.append_block("json_name");
				self.builder
					.build_conditional_branch(separated, comma, name)?;
				self.builder.position_at_end(comma);
				self.json_text(",")?;
				self.builder.build_unconditional_branch(name)?;
				self.builder.position_at_end(name);
			}
			self.json_text(&json_quoted(path))?;
			self.builder.build_unconditional_branch(next)?;
			self.builder.position_at_end(next);
		}
		self.json_text("]")
	}
}
