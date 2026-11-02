//! Objects, named unions and errors.
//!
//! - An object is a struct, by value, of its fields in the order of its shape
//!   (sorted by name); it prints its fields in declaration order.
//! - A value of a named union or `Errors` type is a pointer to a box: the
//!   global id of its member (`i64`), then its data. A member without data
//!   is a constant box. The built-in errors have the fixed ids of the
//!   runtime (`fa_runtime.c`, "errors"); the other members get theirs on
//!   first use, and `fa_format_union`, generated last, prints any box.
//! - Errors are returned (design/llvm-backend-plan.md): a function that
//!   throws returns `{ result, ptr error }` (or the error alone), null when
//!   it succeeded. A raised error goes to the innermost `catch` of the
//!   function, else to the caller, else (top level) ends the program.

use fa_analyzer::analysis::FieldValue;
use fa_analyzer::builtins::errors::{ERRORS, PRELUDE_MODULE};
use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::TypeKind;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

/// The first id of the members that aren't built-in errors.
const FIRST_MEMBER_ID: u64 = 16;

/// The first id of the member types of anonymous unions.
const FIRST_TYPE_ID: u64 = 1 << 32;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn pointer_type(&self) -> inkwell::types::PointerType<'ctx> {
		self.context.ptr_type(AddressSpace::default())
	}

	/// The id of a built-in error type (1 for `IndexError`…), from its nominal.
	fn builtin_error_id(&self, nominal: NominalId) -> Option<u64> {
		let types = self.types();
		let key = types.nominal(nominal).key;
		if types.module_path(types.decl_key_data(key).module) != PRELUDE_MODULE {
			return None;
		}
		let name = types.decl_name(key);
		ERRORS
			.iter()
			.position(|error| error.name == name)
			.map(|index| index as u64 + 1)
	}

	/// The global id of a member of a named union (registered for printing).
	pub(crate) fn member_id(&mut self, nominal: NominalId, index: u16) -> u64 {
		if let Some(id) = self.builtin_error_id(nominal) {
			self.union_ids.insert((nominal, index), id);
			return id;
		}
		let next = FIRST_MEMBER_ID + self.union_ids.len() as u64;
		*self.union_ids.entry((nominal, index)).or_insert(next)
	}

	/// The global id of a member type of an anonymous union (`Integer` in
	/// `Integer | String`), registered for printing.
	pub(crate) fn type_id(&self, ty: TypeId) -> u64 {
		let mut ids = self.type_ids.borrow_mut();
		let next = FIRST_TYPE_ID + ids.len() as u64;
		*ids.entry(ty).or_insert(next)
	}

	/// The type a value of this representation is boxed as when the union
	/// item isn't known (a store into a union without a recorded coercion).
	fn canonical_type(&self, repr: Repr) -> Option<TypeId> {
		Some(match repr {
			Repr::Int(int) => {
				let storage = fa_analyzer::types::IntStorage::ALL
					.into_iter()
					.find(|storage| {
						storage.bits() == int.bits && storage.is_signed() == int.signed
					})?;
				TypeId::of_storage(storage)
			}
			Repr::Bool => TypeId::BOOLEAN,
			Repr::F64 => TypeId::FLOAT,
			Repr::F32 => TypeId::FLOAT32,
			Repr::Percentage => TypeId::PERCENTAGE,
			Repr::Str => TypeId::STRING,
			Repr::Object(ty) => ty,
			_ => return None,
		})
	}

	/// Boxes a value into a union, by the type of its representation.
	pub(crate) fn box_value(&self, value: Val<'ctx>, node: usize) -> Result<Val<'ctx>, Error> {
		let Some(ty) = self.canonical_type(value.repr) else {
			return Err(self.unsupported(node, "storing this value in a union"));
		};
		let id = self.context.i64_type().const_int(self.type_id(ty), false);
		let box_type = self.box_type(Some(value.repr));
		let pointer = self.allocate(box_type.size_of().expect("a sized box"))?;
		let id_pointer = self.builder.build_struct_gep(box_type, pointer, 0, "")?;
		self.builder.build_store(id_pointer, id)?;
		let data_pointer = self.builder.build_struct_gep(box_type, pointer, 1, "")?;
		// the box owns its data: a temporary moves into it
		let owned = self.is_temporary(value);
		let data = if owned { value } else { self.emit_copy(value)? };
		self.builder
			.build_store(data_pointer, data.value.expect("data"))?;
		let boxed = Val::new(Repr::Union, pointer);
		if owned {
			self.transfer(value, boxed);
			Ok(boxed)
		} else {
			Ok(self.fresh(boxed))
		}
	}

	/// The items of an anonymous union type (without `None`).
	fn union_items(&self, ty: TypeId) -> Option<Vec<TypeId>> {
		match self.types().kind(ty) {
			TypeKind::Union(list) => Some(
				self.types()
					.list_items(list)
					.iter()
					.copied()
					.filter(|&item| item != TypeId::NONE)
					.collect(),
			),
			_ => None,
		}
	}

	/// A value stored as a member of an anonymous union type: boxed with the
	/// id of the item its representation belongs to.
	pub(crate) fn inject(
		&mut self,
		value: Val<'ctx>,
		target: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if let TypeKind::NamedUnion(nominal, _) = self.types().kind(target)
			&& value.repr != Repr::Union
		{
			// a literal member (`Hello = "hello"`): the literal selects it
			let literal = self.type_of(node);
			if let TypeKind::Member(member_nominal, index) = self.types().kind(literal)
				&& member_nominal == nominal
			{
				return self.build_member(nominal, index, None, node);
			}
			let text = match self.view(node) {
				View::String(text) => Some(text.to_string()),
				_ => None,
			};
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len() as u16;
			let found = (0..count).find(|&index| {
				let payload = self.types().payload(nominal, index);
				payload == literal
					|| match (self.types().kind(payload), &text) {
						(TypeKind::StringLit(sym), Some(text)) => {
							self.types().syms.resolve(sym) == text
						}
						_ => false,
					}
			});
			if let Some(index) = found {
				return self.build_member(nominal, index, None, node);
			}
			return Err(self.unsupported(node.index(), "storing this value in a named union"));
		}
		let Some(items) = self.union_items(target) else {
			return Ok(value);
		};
		if matches!(value.repr, Repr::Union | Repr::None) {
			return Ok(value);
		}
		let mut chosen = None;
		for item in items {
			let repr = self.repr_of(item, node.index())?;
			let matches = match (repr, value.repr) {
				(Repr::Int(_), Repr::Int(_)) => true,
				(a, b) => a == b,
			};
			if matches {
				chosen = Some((item, repr));
				break;
			}
		}
		let Some((item, repr)) = chosen else {
			return Err(self.unsupported(node.index(), "storing this value in a union"));
		};
		let value = self.owned_as(value, repr, Some(node))?;
		let id = self.context.i64_type().const_int(self.type_id(item), false);
		let ty = self.box_type(Some(repr));
		let pointer = self.allocate(ty.size_of().expect("a sized box"))?;
		let id_pointer = self.builder.build_struct_gep(ty, pointer, 0, "")?;
		self.builder.build_store(id_pointer, id)?;
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		self.builder
			.build_store(data_pointer, value.value.expect("data"))?;
		Ok(self.fresh(Val::new(Repr::Union, pointer)))
	}

	/// The value of a box narrowed to a member type of its union.
	pub(crate) fn unbox(&self, value: Val<'ctx>, repr: Repr) -> Result<Val<'ctx>, Error> {
		let ty = self.box_type(Some(repr));
		let pointer = value.value.expect("a box").into_pointer_value();
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		let loaded = self
			.builder
			.build_load(self.llvm_type(repr), data_pointer, "")?;
		Ok(Val::new(repr, loaded))
	}

	/// The member a type designates: a member, or a named union of one member.
	pub(crate) fn member_of_type(&self, ty: TypeId) -> Option<(NominalId, u16)> {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Member(nominal, index) => Some((nominal, index)),
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				let mut members = (0..count).filter(|&index| types.mask_has(mask, index));
				match (members.next(), members.next()) {
					(Some(index), None) => Some((nominal, index)),
					_ => None,
				}
			}
			_ => None,
		}
	}

	/// The data of a member (`None`: a bare member).
	fn member_data(&self, nominal: NominalId, index: u16) -> Option<TypeId> {
		let data = self.types().payload(nominal, index);
		(!data.is_unset() && !self.types().is_literal(data)).then_some(data)
	}

	/// The struct of a box holding data of this representation.
	fn box_type(&self, data: Option<Repr>) -> StructType<'ctx> {
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i64_type().into()];
		if let Some(data) = data.filter(|data| data.has_value()) {
			fields.push(self.llvm_type(data));
		}
		self.context.struct_type(&fields, false)
	}

	/// A value of a member, with its data.
	pub(crate) fn build_member(
		&mut self,
		nominal: NominalId,
		index: u16,
		data: Option<Val<'ctx>>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let id = self
			.context
			.i64_type()
			.const_int(self.member_id(nominal, index), false);
		let Some(data) = data.filter(|data| data.repr.has_value()) else {
			// a constant box
			let ty = self.box_type(None);
			let global = self.module.add_global(ty, None, "fa.member");
			global.set_initializer(&ty.const_named_struct(&[id.into()]));
			global.set_constant(true);
			global.set_linkage(inkwell::module::Linkage::Private);
			return Ok(Val::new(Repr::Union, global.as_pointer_value()));
		};
		let data_type = self.member_data(nominal, index).unwrap_or(TypeId::UNKNOWN);
		let data_repr = self.repr_of(data_type, node.index())?;
		let data = self.owned_as(data, data_repr, None)?;
		let ty = self.box_type(Some(data_repr));
		let pointer = self.allocate(ty.size_of().expect("a sized box"))?;
		let id_pointer = self.builder.build_struct_gep(ty, pointer, 0, "")?;
		self.builder.build_store(id_pointer, id)?;
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		self.builder
			.build_store(data_pointer, data.value.expect("data"))?;
		Ok(self.fresh(Val::new(Repr::Union, pointer)))
	}

	/// `fa_alloc(size, 16)`.
	pub(crate) fn allocate(&self, size: IntValue<'ctx>) -> Result<PointerValue<'ctx>, Error> {
		let size = self.builder.build_int_cast(size, self.usize, "")?;
		let align = self.usize.const_int(16, false);
		let alloc = self.runtime.alloc()?;
		Ok(self
			.call_value(alloc, &[size.into(), align.into()])?
			.into_pointer_value())
	}

	/// The id of the member in a box.
	fn box_id(&self, value: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let pointer = value.value.expect("a box").into_pointer_value();
		Ok(self
			.builder
			.build_load(self.context.i64_type(), pointer, "")?
			.into_int_value())
	}

	/// Whether a box holds one of the members of `ty` (a member, a named
	/// union, `Error`).
	pub(crate) fn box_is(&mut self, value: Val<'ctx>, ty: TypeId) -> Result<IntValue<'ctx>, Error> {
		let types = self.types();
		let members: Vec<(NominalId, u16)> = match types.kind(ty) {
			TypeKind::Member(nominal, index) => vec![(nominal, index)],
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				(0..count)
					.filter(|&index| types.mask_has(mask, index))
					.map(|index| (nominal, index))
					.collect()
			}
			TypeKind::ErrorTop | TypeKind::Unknown => {
				return Ok(self.context.bool_type().const_int(1, false));
			}
			_ => {
				// a member type of an anonymous union, or a union of some of them
				let items = self.union_items(ty).unwrap_or_else(|| vec![ty]);
				let id = self.box_id(value)?;
				let mut result = self.context.bool_type().const_zero();
				for item in items {
					let expected = self.type_id(item);
					let equal = self.builder.build_int_compare(
						IntPredicate::EQ,
						id,
						self.context.i64_type().const_int(expected, false),
						"",
					)?;
					result = self.builder.build_or(result, equal, "")?;
				}
				return Ok(result);
			}
		};
		let id = self.box_id(value)?;
		let mut result = self.context.bool_type().const_zero();
		for (nominal, index) in members {
			let member = self.member_id(nominal, index);
			let equal = self.builder.build_int_compare(
				IntPredicate::EQ,
				id,
				self.context.i64_type().const_int(member, false),
				"",
			)?;
			result = self.builder.build_or(result, equal, "")?;
		}
		Ok(result)
	}

	/// The data of a box whose member is known (`ty`: a member type, or a
	/// named union of one member).
	pub(crate) fn box_data(
		&self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some((nominal, index)) = self.member_of_type(ty) else {
			return Err(
				self.unsupported(node.index(), "the data of a union whose member isn't known")
			);
		};
		let Some(data) = self.member_data(nominal, index) else {
			return Err(self.unsupported(node.index(), "the data of a member without data"));
		};
		let repr = self.repr_of(data, node.index())?;
		let ty = self.box_type(Some(repr));
		let pointer = value.value.expect("a box").into_pointer_value();
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		let loaded = self
			.builder
			.build_load(self.llvm_type(repr), data_pointer, "")?;
		Ok(Val::new(repr, loaded))
	}

	/// A pointer to the data of a box whose member is known.
	fn box_data_pointer(
		&self,
		pointer: PointerValue<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let Some((nominal, index)) = self.member_of_type(ty) else {
			return Err(
				self.unsupported(node.index(), "the data of a union whose member isn't known")
			);
		};
		let data = self.member_data(nominal, index).unwrap_or(TypeId::UNKNOWN);
		let repr = self.repr_of(data, node.index())?;
		let ty = self.box_type(Some(repr));
		let data_pointer = self.builder.build_struct_gep(ty, pointer, 1, "")?;
		Ok(Slot {
			pointer: data_pointer,
			repr,
		})
	}

	// ── errors ──────────────────────────────────────────────────────────

	/// Raises an error: to the innermost `catch` of the function, else to the
	/// caller, else (the top level) the program ends with it.
	pub(crate) fn raise(&mut self, error: PointerValue<'ctx>) -> Result<(), Error> {
		let frame = self.frame();
		if let Some(&(block, slot, mark)) = frame.catches.last() {
			self.drop_to(mark)?;
			self.builder.build_store(slot, error)?;
			self.builder.build_unconditional_branch(block)?;
		} else if self.frame().throws {
			self.drop_to(0)?;
			let frame = self.frame();
			let ret = frame.ret;
			let function_type = frame.function.get_type();
			match function_type.get_return_type() {
				Some(BasicTypeEnum::StructType(structure)) if ret.has_value() => {
					let pair = self.builder.build_insert_value(
						structure.get_undef(),
						super::zero_of(self.llvm_type(ret)),
						0,
						"",
					)?;
					let pair = self.builder.build_insert_value(pair, error, 1, "")?;
					self.builder.build_return(Some(&pair))?;
				}
				_ => {
					self.builder.build_return(Some(&error))?;
				}
			}
		} else {
			self.call_noreturn("fa_uncaught", &[error.into()])?;
		}
		self.continue_unreachable();
		Ok(())
	}

	/// Raises `error` when it isn't null.
	pub(crate) fn raise_if_error(&mut self, error: PointerValue<'ctx>) -> Result<(), Error> {
		let failed = self.builder.build_is_not_null(error, "")?;
		self.guard(failed, |this| this.raise(error))
	}

	/// Calls a runtime function returning an error, and raises it (in a cold block).
	pub(crate) fn raise_runtime(
		&mut self,
		name: &str,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<(), Error> {
		let function = self.runtime.function(name)?;
		let error = self.call_value(function, arguments)?.into_pointer_value();
		self.raise(error)
	}

	/// `expression catch error => handler`.
	pub(crate) fn catch_expression(
		&mut self,
		node: NodeId,
		expression: NodeId,
		handler: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// printed by `console.log`, a `catch` has no type: its operand's
		let repr = if self.type_of(node) == TypeId::UNKNOWN {
			self.node_repr(expression)?
		} else {
			self.node_repr(node)?
		};
		let slot = if repr.has_value() {
			Some(self.alloca(repr, "catch")?)
		} else {
			None
		};
		let error_slot = self.alloca(Repr::Union, "caught")?;
		let handler_block = self.append_block("catch");
		let end = self.append_block("end_catch");
		let mark = self.mark();
		self.frame_mut()
			.catches
			.push((handler_block, error_slot.pointer, mark));
		let owned = slot.is_some();
		let target = slot.map(|slot| slot.repr);
		let value = self.region(target, expression, |this| this.expr(expression));
		self.frame_mut().catches.pop();
		let value = value?;
		if value.repr != Repr::Never {
			if let Some(slot) = slot {
				self.store(slot, value, expression)?;
			}
			self.branch_to(end)?;
		}
		self.builder.position_at_end(handler_block);
		let handled = self.region(target, handler, |this| {
			// the handler owns the error
			let scope = this.mark();
			let error = this.load(error_slot)?;
			let error = this.fresh(error);
			if let Some(symbol) = this.analysis.symbols.symbol_of_decl(node) {
				let binding = this.slot_of(symbol, node)?;
				let error = this.take(error, None)?;
				this.store(binding, error, node)?;
				this.own_local(symbol, binding);
			}
			let handled = match this.view(handler) {
				View::Block { .. } => this.block(handler, owned)?,
				_ => this.expr(handler)?,
			};
			let handled = if owned && handled.repr.has_value() {
				this.own(handled, Some(handler))?
			} else {
				handled
			};
			this.end_scope(scope, Some(handled))?;
			Ok(handled)
		})?;
		if handled.repr != Repr::Never {
			if let Some(slot) = slot {
				self.store(slot, handled, handler)?;
			}
			self.branch_to(end)?;
		}
		self.builder.position_at_end(end);
		match slot {
			Some(slot) => Ok(self.fresh(self.load(slot)?)),
			None => Ok(Val::void()),
		}
	}

	/// `Integer.parse(text)`.
	pub(crate) fn parse_integer(&mut self, text: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let (bytes, length) = self.string_parts(text)?;
		let out = self.alloca(Repr::Int(super::values::IntRepr::I64), "parsed")?;
		let parse = self.runtime.function("fa_parse_integer")?;
		let error = self
			.call_value(parse, &[bytes.into(), length.into(), out.pointer.into()])?
			.into_pointer_value();
		self.raise_if_error(error)?;
		self.load(out)
	}

	// ── objects ─────────────────────────────────────────────────────────

	/// The shape index of a field of an object type.
	fn field_index(&self, ty: TypeId, name: fa_analyzer::ids::Sym) -> Option<u32> {
		self.types()
			.fields_of(ty)
			.iter()
			.position(|field| field.name == name)
			.map(|index| index as u32)
	}

	/// An object literal (`{ x = 1 }`, `Point { x = 1 }`, `Shape.Circle { … }`).
	pub(crate) fn object_literal(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.object(node).cloned() else {
			return Err(self.unsupported(node.index(), "this object"));
		};
		let structure = self.object_type(info.ty);
		// the given values in the written order, then the defaults
		let mut order: Vec<usize> = (0..info.fields.len()).collect();
		order.sort_by_key(|&position| match info.fields[position].1 {
			FieldValue::Given(value) | FieldValue::Spread(value) => {
				(0, self.tree.spans[value.index()].start)
			}
			_ => (1, position),
		});
		let mut lowered_fields = Vec::new();
		let mut spreads: rustc_hash::FxHashMap<NodeId, Val<'ctx>> = Default::default();
		for position in order {
			let (name, field) = info.fields[position];
			let Some(index) = self.field_index(info.ty, name) else {
				continue;
			};
			let field_type = self.types().fields_of(info.ty)[index as usize].ty;
			let repr = self.repr_of(field_type, node.index())?;
			let lowered = match field {
				FieldValue::Given(value) => {
					let lowered = self.expr(value)?;
					if lowered.repr == Repr::Never {
						return Ok(lowered);
					}
					lowered
				}
				FieldValue::Default(value) => self.expr(value)?,
				// a field copied from `...value`, evaluated once
				FieldValue::Spread(value) => {
					let source = match spreads.get(&value) {
						Some(source) => *source,
						None => {
							let source = self.expr(value)?;
							if source.repr == Repr::Never {
								return Ok(source);
							}
							spreads.insert(value, source);
							source
						}
					};
					let Repr::Object(source_type) = source.repr else {
						return Err(self.unsupported(value.index(), "spreading this value"));
					};
					let Some(source_index) = self.field_index(source_type, name) else {
						return Err(self.unsupported(value.index(), "spreading this value"));
					};
					let source_field =
						self.types().fields_of(source_type)[source_index as usize].ty;
					let source_repr = self.repr_of(source_field, value.index())?;
					let structure = source.value.expect("an object").into_struct_value();
					let raw = self
						.builder
						.build_extract_value(structure, source_index, "")?;
					// a field of the spread value: copied
					self.fresh(self.emit_copy(Val::new(source_repr, raw))?)
				}
				FieldValue::None => Val {
					repr: Repr::None,
					value: None,
				},
			};
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let source = match field {
				FieldValue::Given(value) | FieldValue::Default(value) => Some(value),
				_ => None,
			};
			lowered_fields.push((index, self.owned_as(lowered, repr, source)?));
		}
		let mut value = structure.get_undef();
		for (index, lowered) in lowered_fields {
			if let Some(raw) = lowered.value {
				value = self
					.builder
					.build_insert_value(value, raw, index, "")?
					.into_struct_value();
			}
		}
		let object = self.fresh(Val::new(Repr::Object(info.ty), value));
		match info.member.and_then(|member| self.member_of_type(member)) {
			Some((nominal, index)) => self.build_member(nominal, index, Some(object), node),
			None => Ok(object),
		}
	}

	/// The field order for printing: the declaration's, else the shape's.
	fn printed_fields(&self, ty: TypeId) -> Vec<(String, u32, TypeId)> {
		let types = self.types();
		let shape = types.fields_of(ty);
		let declared = self
			.analysis
			.type_decls
			.values()
			.find(|declaration| declaration.ty == ty)
			.map(|declaration| {
				declaration
					.fields
					.iter()
					.map(|field| field.name)
					.collect::<Vec<_>>()
			});
		// an anonymous type: the order of the first literal that builds it
		let literal = || {
			let mut literals: Vec<_> = self
				.analysis
				.objects
				.iter()
				.filter(|(_, info)| info.ty == ty && info.decl.is_none())
				.collect();
			literals.sort_by_key(|(node, _)| **node);
			literals.first().map(|(_, info)| {
				info.fields
					.iter()
					.map(|(name, _)| *name)
					.collect::<Vec<_>>()
			})
		};
		let order: Vec<fa_analyzer::ids::Sym> = declared
			.or_else(literal)
			.unwrap_or_else(|| shape.iter().map(|field| field.name).collect());
		order
			.into_iter()
			// private fields (`#secret`) don't print
			.filter(|&name| !types.syms.resolve(name).starts_with('#'))
			.filter_map(|name| {
				let index = shape.iter().position(|field| field.name == name)?;
				Some((
					types.syms.resolve(name).to_string(),
					index as u32,
					shape[index].ty,
				))
			})
			.collect()
	}

	/// Appends `{ x = 1, y = 2 }`.
	pub(crate) fn append_object(
		&mut self,
		value: Val<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<(), Error> {
		let fields = self.printed_fields(ty);
		if fields.is_empty() {
			let text = self.string_constant("{}");
			return self.append_value(text, node);
		}
		let structure = value.value.expect("an object").into_struct_value();
		for (position, (name, index, field_type)) in fields.into_iter().enumerate() {
			let prefix = if position == 0 {
				format!("{{ {name} = ")
			} else {
				format!(", {name} = ")
			};
			let prefix = self.string_constant(&prefix);
			self.append_value(prefix, node)?;
			let repr = self.repr_of(field_type, node.index())?;
			let raw = self.builder.build_extract_value(structure, index, "")?;
			self.append_nested(Val::new(repr, raw), node)?;
		}
		let suffix = self.string_constant(" }");
		self.append_value(suffix, node)
	}

	/// Appends a value nested in a composite: strings are quoted.
	pub(crate) fn append_nested(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		match value.repr {
			Repr::Str => {
				let (bytes, length) = self.string_parts(value)?;
				self.call_runtime("fa_b_quoted", &[bytes.into(), length.into()])
			}
			Repr::Optional(super::values::Scalar::Str) => self.append_optional(value, node, true),
			Repr::Union => {
				let nested = self.context.i32_type().const_int(1, false);
				self.call_runtime(
					"fa_format_union",
					&[value.value.expect("a box").into(), nested.into()],
				)
			}
			_ => self.append_value(value, node),
		}
	}

	/// `value.a.b`: fields of objects and of the data of union members,
	/// built-in properties, members of enumerations and unions.
	pub(crate) fn member_access(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let first = members.first().expect("a member");
		// `Color.red`, `E.Boom`: a member of a nominal type
		if let Resolution::Member(nominal, index) = self.analysis.resolution(first)
			&& members.len() == 1
		{
			return match self.node_repr(node)? {
				Repr::Enum(_) => Ok(Val::new(
					self.node_repr(node)?,
					self.context.i32_type().const_int(index as u64, false),
				)),
				_ => self.build_member(nominal, index, None, node),
			};
		}
		// `Integer.maxValue`, `Float.infinity`…
		if let Resolution::Builtin(id) = self.analysis.resolution(first)
			&& members.len() == 1
			&& let Some(value) = self.number_constant(fa_analyzer::builtins::get(id).path, node)?
		{
			return Ok(value);
		}
		// `Color.members`: every member of an enumeration, in order
		if let Resolution::Builtin(id) = self.analysis.resolution(first)
			&& fa_analyzer::builtins::get(id).path == "Enum.members"
			&& let Repr::Array(item_type) = self.node_repr(node)?
			&& let Repr::Enum(nominal) = self.repr_of(item_type, node.index())?
		{
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len() as u64;
			let length = self.context.i64_type().const_int(count, false);
			let array = self.new_array(Repr::Enum(nominal), length, length)?;
			let (items, _) = self.array_parts(array)?;
			for index in 0..count {
				let position = self.context.i64_type().const_int(index, false);
				let pointer = unsafe {
					self.builder.build_in_bounds_gep(
						self.context.i32_type(),
						items,
						&[position],
						"",
					)?
				};
				self.builder
					.build_store(pointer, self.context.i32_type().const_int(index, false))?;
			}
			return Ok(self.fresh(Val::new(Repr::Array(item_type), array)));
		}
		let value = self.expr(target)?;
		let count = members.len();
		let value = self.apply_members(node, value, self.type_of(target), members, count)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let narrowed = self.node_repr(node).unwrap_or(value.repr);
		self.narrow_to(value, narrowed, node)
	}

	/// The first `count` members of an access applied to a value: fields of
	/// objects and of the data of boxes, built-in properties.
	pub(crate) fn apply_members(
		&mut self,
		node: NodeId,
		mut value: Val<'ctx>,
		mut current_type: TypeId,
		members: NodeList<'_>,
		count: usize,
	) -> Result<Val<'ctx>, Error> {
		for member in members.iter().take(count) {
			if value.repr == Repr::Never {
				return Ok(value);
			}
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		Ok(value)
	}

	/// One member applied to a value: a field, or a built-in property.
	fn apply_member(
		&mut self,
		node: NodeId,
		mut value: Val<'ctx>,
		current_type: TypeId,
		member: NodeId,
	) -> Result<(Val<'ctx>, TypeId), Error> {
		match self.analysis.resolution(member) {
			Resolution::Field(_, index) => {
				if value.repr == Repr::Union {
					value = self.box_data(value, current_type, member)?;
				}
				if let Repr::Rec(target) = value.repr {
					value = self.unfold_rec(value, target, member.index())?;
				}
				let Repr::Object(object) = value.repr else {
					return Err(self.unsupported(member.index(), "this field access"));
				};
				let field_type = self.types().fields_of(object)[index as usize].ty;
				let repr = self.repr_of(field_type, member.index())?;
				let structure = value.value.expect("an object").into_struct_value();
				let raw = self
					.builder
					.build_extract_value(structure, index as u32, "")?;
				Ok((Val::new(repr, raw), field_type))
			}
			Resolution::Builtin(_) => {
				let value = self.builtin_property(node, value, member)?;
				Ok((value, self.type_of(member)))
			}
			// `Color.green` inside a longer access
			Resolution::Member(nominal, index) => {
				let ty = self.type_of(member);
				let value = match self.repr_of(ty, member.index())? {
					Repr::Enum(_) => Val::new(
						Repr::Enum(nominal),
						self.context.i32_type().const_int(index as u64, false),
					),
					_ => self.build_member(nominal, index, None, member)?,
				};
				Ok((value, ty))
			}
			_ => Err(self.unsupported(member.index(), "this member")),
		}
	}

	/// `value?.a.b`: `none` when the value (or an optional along the chain)
	/// is `none`, the member otherwise.
	pub(crate) fn optional_access(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let result = self.alloca(repr, "chain")?;
		let none = self.convert(
			Val {
				repr: Repr::None,
				value: None,
			},
			repr,
			node.index(),
		)?;
		if let Some(raw) = none.value {
			self.builder.build_store(result.pointer, raw)?;
		}
		let end = self.append_block("end_chain");
		let mut value = self.expr(target)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let mut current_type = self.type_of(target);
		for member in members.iter() {
			value = self.present_or_exit(value, end)?;
			current_type = self.without_none(current_type);
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		if let Repr::None = value.repr {
			self.builder.build_unconditional_branch(end)?;
		} else {
			self.store(result, value, node)?;
			self.builder.build_unconditional_branch(end)?;
		}
		self.builder.position_at_end(end);
		self.load(result)
	}

	/// `value?.a.method(…)`: `none` when the value (or an optional along the
	/// chain) is `none`, the result of the call otherwise.
	pub(crate) fn optional_call(
		&mut self,
		node: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let result = match repr.has_value() {
			true => Some(self.alloca(repr, "chain")?),
			false => None,
		};
		if let Some(result) = result {
			let none = self.convert(
				Val {
					repr: Repr::None,
					value: None,
				},
				repr,
				node.index(),
			)?;
			if let Some(raw) = none.value {
				self.builder.build_store(result.pointer, raw)?;
			}
		}
		let end = self.append_block("end_chain");
		let mut value = self.expr(target)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let mut current_type = self.type_of(target);
		let count = members.len();
		for member in members.iter().take(count - 1) {
			value = self.present_or_exit(value, end)?;
			current_type = self.without_none(current_type);
			(value, current_type) = self.apply_member(node, value, current_type, member)?;
		}
		let receiver = self.present_or_exit(value, end)?;
		let called = self.region(result.map(|_| repr), node, |this| {
			this.pending_receiver = Some(receiver);
			let called = this.call_present(node);
			this.pending_receiver = None;
			called
		})?;
		if let Some(result) = result
			&& called.repr.has_value()
		{
			self.store(result, called, node)?;
		}
		self.branch_to(end)?;
		self.builder.position_at_end(end);
		match result {
			Some(result) => Ok(self.fresh(self.load(result)?)),
			None => Ok(Val::void()),
		}
	}

	/// The value of an optional, continuing only when it is present
	/// (branching to `exit` otherwise).
	fn present_or_exit(
		&mut self,
		value: Val<'ctx>,
		exit: inkwell::basic_block::BasicBlock<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Optional(scalar) = value.repr else {
			return Ok(value);
		};
		let pair = value.value.expect("an optional").into_struct_value();
		let present = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_int_value();
		let inner = self.builder.build_extract_value(pair, 1, "")?;
		let next = self.append_block("chain_present");
		self.builder.build_conditional_branch(present, next, exit)?;
		self.builder.position_at_end(next);
		Ok(Val::new(scalar.repr(), inner))
	}

	/// A type without `none`.
	fn without_none(&self, ty: TypeId) -> TypeId {
		let types = self.types();
		if let TypeKind::Union(list) = types.kind(ty) {
			let items: Vec<TypeId> = types
				.list_items(list)
				.iter()
				.copied()
				.filter(|&item| item != TypeId::NONE)
				.collect();
			if items.len() == 1 {
				return items[0];
			}
		}
		ty
	}

	/// The receiver of a method call `a.b.method(…)`: `a.b`.
	pub(crate) fn receiver(&mut self, call: NodeId) -> Result<Val<'ctx>, Error> {
		if let Some(value) = self.pending_receiver.take() {
			return Ok(value);
		}
		let View::Call { callee, .. } = self.view(call) else {
			return Err(self.unsupported(call.index(), "this method call"));
		};
		let View::Access { target, members } = self.view(callee) else {
			return Err(self.unsupported(call.index(), "this method call"));
		};
		let value = self.expr(target)?;
		let count = members.len() - 1;
		self.apply_members(callee, value, self.type_of(target), members, count)
	}

	/// A place: a pointer to a local, a field of a place, or the data of a box.
	pub(crate) fn place(&mut self, node: NodeId) -> Result<Slot<'ctx>, Error> {
		match self.view(node) {
			View::Name(_) => {
				let Resolution::Local(symbol) = self.analysis.resolution(node) else {
					return Err(self.unsupported(node.index(), "this place"));
				};
				self.slot_of(symbol, node)
			}
			View::Group(inner) => self.place(inner),
			View::Index { target, index } => match self.tuple_place(target, index)? {
				Some(slot) => Ok(slot),
				None if matches!(self.node_repr(target)?, Repr::Map(_)) => {
					let map = self.expr(target)?;
					let key = self.expr(index)?;
					self.table_place(node, map, key)
				}
				None => self.index_place(node, target, index),
			},
			View::Access { target, members } => {
				let slot = self.place(target)?;
				self.place_members(slot, self.type_of(target), members.iter())
			}
			_ => Err(self.unsupported(node.index(), "this place")),
		}
	}

	/// The place of fields below a place (`.a.b`).
	pub(crate) fn place_members(
		&mut self,
		mut slot: Slot<'ctx>,
		mut current_type: TypeId,
		members: impl Iterator<Item = NodeId>,
	) -> Result<Slot<'ctx>, Error> {
		for member in members {
			let Resolution::Field(_, index) = self.analysis.resolution(member) else {
				return Err(self.unsupported(member.index(), "this place"));
			};
			if slot.repr == Repr::Union {
				let pointer = self
					.builder
					.build_load(self.pointer_type(), slot.pointer, "")?
					.into_pointer_value();
				slot = self.box_data_pointer(pointer, current_type, member)?;
			}
			if let Repr::Rec(target) = slot.repr {
				let pointer = self
					.builder
					.build_load(self.pointer_type(), slot.pointer, "")?
					.into_pointer_value();
				slot = Slot {
					pointer,
					repr: self.repr_of(target, member.index())?,
				};
			}
			let Repr::Object(object) = slot.repr else {
				return Err(self.unsupported(member.index(), "this place"));
			};
			let field_type = self.types().fields_of(object)[index as usize].ty;
			let structure = self.object_type(object);
			let pointer =
				self.builder
					.build_struct_gep(structure, slot.pointer, index as u32, "")?;
			slot = Slot {
				pointer,
				repr: self.repr_of(field_type, member.index())?,
			};
			current_type = field_type;
		}
		Ok(slot)
	}

	/// `target .= { field = value, … }`.
	pub(crate) fn update_fields(
		&mut self,
		target: NodeId,
		value: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let View::Object { items } = self.view(value) else {
			return Err(self.unsupported(value.index(), "this update"));
		};
		let slot = self.place(target)?;
		let Repr::Object(object) = slot.repr else {
			return Err(self.unsupported(target.index(), "this update"));
		};
		let structure = self.object_type(object);
		let mut updates = Vec::new();
		for item in items.iter() {
			let (name, field_value) = match self.view(item) {
				View::Assignment {
					target: name,
					value: Some(field_value),
					..
				} => (name, field_value),
				View::Name(_) => (item, item),
				_ => return Err(self.unsupported(item.index(), "this field update")),
			};
			let View::Name(name) = self.view(name) else {
				return Err(self.unsupported(item.index(), "this field update"));
			};
			let text = self.tree.name(name).to_string();
			let Some(index) = self
				.types()
				.fields_of(object)
				.iter()
				.position(|field| self.types().syms.resolve(field.name) == text)
			else {
				return Err(self.unsupported(item.index(), "this field update"));
			};
			let field_type = self.types().fields_of(object)[index].ty;
			let repr = self.repr_of(field_type, item.index())?;
			let lowered = self.expr(field_value)?;
			if lowered.repr == Repr::Never {
				return Ok(lowered);
			}
			let lowered = self.owned_as(lowered, repr, Some(field_value))?;
			updates.push((index, repr, lowered, field_value));
		}
		// every value is read before the first field changes
		for (index, repr, lowered, field_value) in updates {
			let pointer =
				self.builder
					.build_struct_gep(structure, slot.pointer, index as u32, "")?;
			let field = Slot { pointer, repr };
			let old = self.load(field)?;
			self.emit_drop(old)?;
			self.store(field, lowered, field_value)?;
		}
		Ok(Val::void())
	}

	// ── printing unions ─────────────────────────────────────────────────

	/// Defines `fa.drop_box` and `fa.copy_box` for every member id the program
	/// uses (`ownership.rs`): the layout of its data, none for a constant box.
	pub(crate) fn define_boxes(&mut self) -> Result<(), Error> {
		let mut cases = Vec::new();
		let mut members: Vec<((NominalId, u16), u64)> =
			self.union_ids.iter().map(|(key, id)| (*key, *id)).collect();
		members.sort_by_key(|(_, id)| *id);
		members.dedup_by_key(|(_, id)| *id);
		for ((nominal, index), id) in members {
			let fields: Vec<Repr> = match self.builtin_error_id(nominal) {
				Some(error) => ERRORS[error as usize - 1]
					.fields
					.iter()
					.map(|(_, ty)| self.repr_of(*ty, 0))
					.collect::<Result<_, _>>()?,
				None => match self.member_data(nominal, index) {
					Some(data) => vec![self.repr_of(data, 0)?],
					None => Vec::new(),
				},
			};
			let fields: Vec<Repr> = fields.into_iter().filter(|repr| repr.has_value()).collect();
			if fields.is_empty() {
				// a constant box
				cases.push((id, None, fields));
				continue;
			}
			let mut types: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i64_type().into()];
			types.extend(fields.iter().map(|repr| self.llvm_type(*repr)));
			let layout = self.context.struct_type(&types, false);
			cases.push((id, Some(layout), fields));
		}
		let mut types: Vec<(TypeId, u64)> = self
			.type_ids
			.borrow()
			.iter()
			.map(|(ty, id)| (*ty, *id))
			.collect();
		types.sort_by_key(|(_, id)| *id);
		for (ty, id) in types {
			let repr = self.repr_of(ty, 0)?;
			cases.push((id, Some(self.box_type(Some(repr))), vec![repr]));
		}
		self.define_box_functions(&cases)
	}

	/// Defines `fa_format_union(box, nested)`, which printing the values of
	/// unions call: a switch on the id of the member.
	pub(crate) fn define_format_union(&mut self) -> Result<(), Error> {
		// the built-in errors, which the runtime raises
		for error in ERRORS {
			if let Some(id) = fa_analyzer::builtins::lookup(error.name)
				&& let Some(ty) = self.session.builtin_type(id)
				&& let TypeKind::NamedUnion(nominal, _) = self.types().kind(ty)
			{
				self.member_id(nominal, 0);
			}
		}
		let pointer = self.pointer_type();
		let function = match self.module.get_function("fa_format_union") {
			Some(function) => function,
			None => self.module.add_function(
				"fa_format_union",
				self.context
					.void_type()
					.fn_type(&[pointer.into(), self.context.i32_type().into()], false),
				None,
			),
		};
		self.begin_function(function, Repr::Void, None);
		let value = Val::new(Repr::Union, function.get_nth_param(0).expect("a box"));
		let id = self.box_id(value)?;
		let end = self.append_block("end_format");
		let mut members: Vec<((NominalId, u16), u64)> =
			self.union_ids.iter().map(|(key, id)| (*key, *id)).collect();
		members.sort_by_key(|(_, id)| *id);
		members.dedup_by_key(|(_, id)| *id);
		let mut cases = Vec::new();
		let switch_block = self.builder.get_insert_block().expect("a block");
		let node = NodeId::new(0);
		for ((nominal, index), member_id) in members {
			let block = self.append_block("format_member");
			self.builder.position_at_end(block);
			let types = self.types();
			let info = types.nominal(nominal);
			let type_name = types.decl_name(info.key).to_string();
			let member_name = types
				.sym_list(info.members)
				.get(index as usize)
				.map(|&sym| types.syms.resolve(sym).to_string())
				.unwrap_or_default();
			let name = if self.builtin_error_id(nominal).is_some() {
				type_name
			} else {
				format!("{type_name}.{member_name}")
			};
			let name = self.string_constant(&name);
			self.append_value(name, node)?;
			let builtin_fields = self
				.builtin_error_id(nominal)
				.map(|id| ERRORS[id as usize - 1].fields)
				.filter(|fields| !fields.is_empty());
			if let Some(fields) = builtin_fields {
				// the box of the runtime: the id, then the fields
				let reprs: Vec<Repr> = fields
					.iter()
					.map(|(_, ty)| self.repr_of(*ty, 0))
					.collect::<Result<_, _>>()?;
				let mut types: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i64_type().into()];
				types.extend(reprs.iter().map(|repr| self.llvm_type(*repr)));
				let ty = self.context.struct_type(&types, false);
				let box_pointer = value.value.expect("a box").into_pointer_value();
				for (position, ((field, _), repr)) in fields.iter().zip(reprs).enumerate() {
					let prefix = if position == 0 {
						format!(" {{ {field} = ")
					} else {
						format!(", {field} = ")
					};
					let prefix = self.string_constant(&prefix);
					self.append_value(prefix, node)?;
					let pointer =
						self.builder
							.build_struct_gep(ty, box_pointer, position as u32 + 1, "")?;
					let loaded = self.load(Slot { pointer, repr })?;
					self.append_nested(loaded, node)?;
				}
				let suffix = self.string_constant(" }");
				self.append_value(suffix, node)?;
			} else if let Some(data) = self.member_data(nominal, index) {
				let repr = self.repr_of(data, 0)?;
				let ty = self.box_type(Some(repr));
				let box_pointer = value.value.expect("a box").into_pointer_value();
				let data_pointer = self.builder.build_struct_gep(ty, box_pointer, 1, "")?;
				let loaded = self
					.builder
					.build_load(self.llvm_type(repr), data_pointer, "")?;
				let loaded = Val::new(repr, loaded);
				if let Repr::Object(object) = repr {
					let space = self.string_constant(" ");
					self.append_value(space, node)?;
					self.append_object(loaded, object, node)?;
				} else {
					let open = self.string_constant("(");
					self.append_value(open, node)?;
					self.append_nested(loaded, node)?;
					let close = self.string_constant(")");
					self.append_value(close, node)?;
				}
			}
			self.builder.build_unconditional_branch(end)?;
			cases.push((self.context.i64_type().const_int(member_id, false), block));
		}
		// the member types of anonymous unions: their value, quoted when nested
		let nested = function.get_nth_param(1).expect("a flag").into_int_value();
		let mut types: Vec<(TypeId, u64)> = self
			.type_ids
			.borrow()
			.iter()
			.map(|(ty, id)| (*ty, *id))
			.collect();
		types.sort_by_key(|(_, id)| *id);
		for (ty, type_id) in types {
			let block = self.append_block("format_type");
			self.builder.position_at_end(block);
			let repr = self.repr_of(ty, 0)?;
			let data = self.unbox(value, repr)?;
			if repr == Repr::Str {
				let quoted = self.append_block("format_quoted");
				let raw = self.append_block("format_raw");
				let is_nested = self.builder.build_int_compare(
					IntPredicate::NE,
					nested,
					nested.get_type().const_zero(),
					"",
				)?;
				self.builder
					.build_conditional_branch(is_nested, quoted, raw)?;
				self.builder.position_at_end(quoted);
				self.append_nested(data, node)?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(raw);
				self.append_value(data, node)?;
			} else {
				self.append_nested(data, node)?;
			}
			self.builder.build_unconditional_branch(end)?;
			cases.push((self.context.i64_type().const_int(type_id, false), block));
		}
		self.builder.position_at_end(switch_block);
		self.builder.build_switch(id, end, &cases)?;
		self.builder.position_at_end(end);
		self.builder.build_return(None)?;
		self.end_function()
	}
}
