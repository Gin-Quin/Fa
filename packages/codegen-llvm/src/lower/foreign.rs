//! Foreign code (`documentation/advanced/foreign-code.md`): calls of
//! `@extern` functions with the C calling convention, the conversions of
//! the passing rules (advanced brief §13.3), C callbacks, the methods of
//! `Pointer(T)` and `CString`, and `destroy`.
//!
//! - An `@extern` function is declared once, `ccc`, with C types: integers
//!   of their width (`signext`/`zeroext` below 32 bits), `float`/`double`,
//!   `ptr` for pointers and C strings (`Pointer(T)?` is `NULL` when absent),
//!   and a `CStruct` by value classified like clang does on x86-64 System V
//!   (up to two eightbytes in registers, else `byval`/`sret`).
//! - A `String` given to a `CString` is copied into a zero-terminated block
//!   of the runtime allocator, freed after the call; an `Array(T)` given to a
//!   `Pointer(T)` is the pointer to its items; a value given to a
//!   `Pointer(T)` is copied into a temporary of its C type, written back to
//!   its place after the call when the parameter is `mutates`.
//! - A callback is a `ccc` wrapper calling the Fa function (`fastcc`).

use fa_analyzer::analysis::{Argument, ExternCall, ForeignArg};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use fa_analyzer::types::{FloatBits, NominalKind, TypeKind};
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::module::Linkage;
use inkwell::targets::TargetData;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};
use crate::target::Target;

/// The C type of a parameter or a result.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CType {
	Void,
	Bool,
	Int(u32, bool),
	F32,
	F64,
	/// A pointer or a C string; `nullable` for `Pointer(T)?`.
	Pointer {
		nullable: bool,
	},
	/// A `CStruct` by value: its object type.
	Struct(TypeId),
}

/// How a C value crosses the ABI (x86-64 System V for structs).
enum Passing<'ctx> {
	/// As is.
	Direct(BasicTypeEnum<'ctx>),
	/// A struct loaded as these registers (one or two eightbytes).
	Coerced(Vec<BasicTypeEnum<'ctx>>),
	/// A struct in memory: a pointer (`byval` for a parameter, `sret` for a result).
	Memory(StructType<'ctx>),
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The C type of a Fa type at the C boundary.
	pub(crate) fn c_type(&self, ty: TypeId, node: NodeId) -> Result<CType, Error> {
		let types = self.types();
		Ok(match types.kind(ty) {
			TypeKind::Void | TypeKind::Never => CType::Void,
			TypeKind::Boolean | TypeKind::BoolLit(_) => CType::Bool,
			TypeKind::Int(id) => {
				let int = IntRepr::of(types.int(id).storage);
				CType::Int(int.bits, int.signed)
			}
			TypeKind::Float(FloatBits::F32) => CType::F32,
			TypeKind::Float(FloatBits::F64) => CType::F64,
			TypeKind::Pointer(_) | TypeKind::CString => CType::Pointer { nullable: false },
			TypeKind::Union(list)
				if types.pointer_target(ty).is_some()
					|| types
						.list_items(list)
						.iter()
						.any(|&item| types.kind(item) == TypeKind::CString) =>
			{
				CType::Pointer { nullable: true }
			}
			TypeKind::Function(_) => CType::Pointer { nullable: false },
			TypeKind::Object(shape)
				if types
					.shape(shape)
					.nominal
					.is_some_and(|nominal| types.nominal(nominal).kind == NominalKind::CStruct) =>
			{
				CType::Struct(ty)
			}
			_ => {
				return Err(self.unsupported(
					node.index(),
					&format!(
						"a value of type `{}` at the C boundary",
						self.session.display_type(ty)
					),
				));
			}
		})
	}

	/// The IR type of a C type.
	fn c_llvm_type(&self, c: CType) -> Option<BasicTypeEnum<'ctx>> {
		Some(match c {
			CType::Void => return None,
			CType::Bool => self.context.bool_type().into(),
			CType::Int(bits, _) => self.int_type(bits).into(),
			CType::F32 => self.context.f32_type().into(),
			CType::F64 => self.context.f64_type().into(),
			CType::Pointer { .. } => self.pointer_type().into(),
			CType::Struct(ty) => self.object_type(ty).into(),
		})
	}

	/// The target data of the module (sizes and offsets of structs).
	pub(super) fn target_data(&self) -> TargetData {
		let layout = self.module.get_data_layout();
		TargetData::create(layout.as_str().to_str().unwrap_or(""))
	}

	/// How a C value is passed (x86-64 System V, and every scalar elsewhere).
	fn passing(&self, c: CType, node: NodeId) -> Result<Option<Passing<'ctx>>, Error> {
		let CType::Struct(ty) = c else {
			return Ok(self.c_llvm_type(c).map(Passing::Direct));
		};
		if self.target != Target::Native {
			return Err(self.unsupported(
				node.index(),
				"a `CStruct` by value outside x86-64 System V (pass a `Pointer` to it)",
			));
		}
		let structure = self.object_type(ty);
		let data = self.target_data();
		let size = data.get_abi_size(&structure);
		if size > 16 {
			return Ok(Some(Passing::Memory(structure)));
		}
		// the scalar leaves with their offsets
		let mut leaves: Vec<(u64, BasicTypeEnum<'ctx>)> = Vec::new();
		self.struct_leaves(&data, structure, 0, &mut leaves);
		let mut registers = Vec::new();
		for eightbyte in 0..size.div_ceil(8) {
			let (start, end) = (eightbyte * 8, (eightbyte * 8 + 8).min(size));
			let inside: Vec<&(u64, BasicTypeEnum<'ctx>)> = leaves
				.iter()
				.filter(|(offset, _)| *offset >= start && *offset < end)
				.collect();
			let sse = !inside.is_empty() && inside.iter().all(|(_, ty)| ty.is_float_type());
			registers.push(if sse {
				match (inside.len(), end - start) {
					(1, _) => inside[0].1,
					_ => self.context.f32_type().vec_type(2).into(),
				}
			} else {
				self.int_type(match end - start {
					1 => 8,
					2 => 16,
					3 | 4 => 32,
					_ => 64,
				})
				.into()
			});
		}
		Ok(Some(Passing::Coerced(registers)))
	}

	/// The scalar leaves of a struct, with their offsets.
	fn struct_leaves(
		&self,
		data: &TargetData,
		structure: StructType<'ctx>,
		base: u64,
		leaves: &mut Vec<(u64, BasicTypeEnum<'ctx>)>,
	) {
		for (index, field) in structure.get_field_types().into_iter().enumerate() {
			let offset = base
				+ data
					.offset_of_element(&structure, index as u32)
					.unwrap_or_default();
			match field {
				BasicTypeEnum::StructType(inner) => self.struct_leaves(data, inner, offset, leaves),
				BasicTypeEnum::ArrayType(array) => {
					let item = array.get_element_type();
					let size = data.get_abi_size(&item);
					for position in 0..array.len() as u64 {
						leaves.push((offset + position * size, item));
					}
				}
				other => leaves.push((offset, other)),
			}
		}
	}

	/// The declaration of an `@extern` function (`ccc`), with the C types
	/// of its parameters and of its result.
	fn extern_function(
		&self,
		symbol: &str,
		function_type: TypeId,
		node: NodeId,
	) -> Result<(FunctionValue<'ctx>, Vec<CType>, CType), Error> {
		let Some(sig) = self.signature(function_type) else {
			return Err(self.unsupported(node.index(), "this C function"));
		};
		let types = self.types();
		let params: Vec<TypeId> = types
			.params(sig.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		let c_params = params
			.iter()
			.map(|&ty| self.c_type(ty, node))
			.collect::<Result<Vec<_>, _>>()?;
		let c_ret = self.c_type(sig.ret, node)?;
		if let Some(function) = self.module.get_function(symbol) {
			return Ok((function, c_params, c_ret));
		}
		let mut ir_params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
		let mut attributes: Vec<(u32, &str, Option<inkwell::types::AnyTypeEnum<'ctx>>)> =
			Vec::new();
		let ret_passing = self.passing(c_ret, node)?;
		if let Some(Passing::Memory(structure)) = &ret_passing {
			attributes.push((0, "sret", Some((*structure).into())));
			ir_params.push(self.pointer_type().into());
		}
		for &c in &c_params {
			let position = ir_params.len() as u32;
			match self.passing(c, node)? {
				Some(Passing::Direct(ty)) => {
					match c {
						CType::Int(bits, signed) if bits < 32 => {
							let name = if signed { "signext" } else { "zeroext" };
							attributes.push((position, name, None));
						}
						CType::Bool => attributes.push((position, "zeroext", None)),
						_ => {}
					}
					ir_params.push(ty.into());
				}
				Some(Passing::Coerced(registers)) => {
					ir_params.extend(registers.into_iter().map(BasicMetadataTypeEnum::from));
				}
				Some(Passing::Memory(structure)) => {
					attributes.push((position, "byval", Some(structure.into())));
					ir_params.push(self.pointer_type().into());
				}
				None => return Err(self.unsupported(node.index(), "a `Void` parameter")),
			}
		}
		let function_type = match &ret_passing {
			None | Some(Passing::Memory(_)) => self.context.void_type().fn_type(&ir_params, false),
			Some(Passing::Direct(ty)) => ty.fn_type(&ir_params, false),
			Some(Passing::Coerced(registers)) => {
				if registers.len() == 1 {
					registers[0].fn_type(&ir_params, false)
				} else {
					self.context
						.struct_type(registers, false)
						.fn_type(&ir_params, false)
				}
			}
		};
		let function = self
			.module
			.add_function(symbol, function_type, Some(Linkage::External));
		for (position, name, ty) in attributes {
			let kind = Attribute::get_named_enum_kind_id(name);
			let attribute = match ty {
				Some(ty) => self.context.create_type_attribute(kind, ty),
				None => self.context.create_enum_attribute(kind, 0),
			};
			function.add_attribute(AttributeLoc::Param(position), attribute);
		}
		if let CType::Int(bits, signed) = c_ret
			&& bits < 32
		{
			let kind =
				Attribute::get_named_enum_kind_id(if signed { "signext" } else { "zeroext" });
			function.add_attribute(
				AttributeLoc::Return,
				self.context.create_enum_attribute(kind, 0),
			);
		}
		Ok((function, c_params, c_ret))
	}

	/// The signature of a function type.
	fn signature(&self, ty: TypeId) -> Option<fa_analyzer::types::FnSig> {
		match self.types().kind(ty) {
			TypeKind::Function(sig) => Some(self.types().sig(sig)),
			_ => None,
		}
	}

	/// A Fa value converted to its C type.
	fn to_c(
		&self,
		value: Val<'ctx>,
		c: CType,
		_node: NodeId,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let raw = value.value.expect("a value");
		Ok(match (value.repr, c) {
			(Repr::Optional(_), CType::Pointer { .. }) => {
				let pair = raw.into_struct_value();
				let present = self
					.builder
					.build_extract_value(pair, 0, "")?
					.into_int_value();
				let pointer = self.builder.build_extract_value(pair, 1, "")?;
				self.builder.build_select(
					present,
					pointer,
					BasicValueEnum::from(self.pointer_type().const_null()),
					"",
				)?
			}
			(Repr::None, CType::Pointer { .. }) => self.pointer_type().const_null().into(),
			(Repr::Int(from), CType::Int(bits, signed)) if from.bits != bits => self
				.int_to(raw.into_int_value(), from, IntRepr { bits, signed })?
				.into(),
			_ => raw,
		})
	}

	/// A C value converted to the Fa representation of `ty`.
	fn c_to_fa(
		&self,
		raw: BasicValueEnum<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.repr_of(ty, node.index())?;
		Ok(match repr {
			Repr::Optional(_) => {
				let pointer = raw.into_pointer_value();
				let present = self.builder.build_is_not_null(pointer, "")?;
				let structure = self.llvm_type(repr).into_struct_type();
				let pair =
					self.builder
						.build_insert_value(structure.get_undef(), present, 0, "")?;
				let pair = self.builder.build_insert_value(pair, pointer, 1, "")?;
				Val::new(repr, pair.into_struct_value())
			}
			_ => Val::new(repr, raw),
		})
	}

	/// A C value, loaded from memory, converted to the Fa representation of `ty`.
	fn load_c(
		&self,
		pointer: PointerValue<'ctx>,
		ty: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let c = self.c_type(ty, node)?;
		let Some(ir) = self.c_llvm_type(c) else {
			return Err(self.unsupported(node.index(), "reading a `Void`"));
		};
		let raw = self.builder.build_load(ir, pointer, "")?;
		self.c_to_fa(raw, ty, node)
	}

	// ── calls ───────────────────────────────────────────────────────────

	/// A call of an `@extern` function.
	pub(crate) fn foreign_call(
		&mut self,
		node: NodeId,
		call: &ExternCall,
	) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.call(node).cloned() else {
			return Err(self.unsupported(node.index(), "this C call"));
		};
		let View::Call { callee, .. } = self.view(node) else {
			return Err(self.unsupported(node.index(), "this C call"));
		};
		let function_type = self.type_of(callee);
		let (function, c_params, c_ret) =
			self.extern_function(&call.symbol, function_type, node)?;
		let sig = self.signature(function_type).expect("a function type");
		let param_types: Vec<TypeId> = self
			.types()
			.params(sig.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
		let ret_passing = self.passing(c_ret, node)?;
		let sret = match &ret_passing {
			Some(Passing::Memory(structure)) => {
				let slot = self.alloca_type((*structure).into(), "sret")?;
				arguments.push(slot.into());
				Some(slot)
			}
			_ => None,
		};
		// after the call: strings to free, temporaries to write back
		let mut frees: Vec<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>)> = Vec::new();
		let mut write_backs: Vec<(PointerValue<'ctx>, NodeId, TypeId)> = Vec::new();
		for (index, argument) in info.arguments.iter().enumerate() {
			let Argument::Given(value) = argument else {
				return Err(self.unsupported(node.index(), "these C arguments"));
			};
			let value = *value;
			let param_ty = param_types[index];
			let c = c_params[index];
			let raw: BasicValueEnum<'ctx> = match self.analysis.foreign_args.get(&value).copied() {
				Some(ForeignArg::CString) => {
					let text = self.expr(value)?;
					if text.repr == Repr::Never {
						return Ok(text);
					}
					let (bytes, length) = self.string_parts(text)?;
					let size =
						self.builder
							.build_int_add(length, self.usize.const_int(1, false), "")?;
					let block = self.allocate(size)?;
					self.builder.build_memcpy(block, 1, bytes, 1, length)?;
					let end = unsafe {
						self.builder.build_in_bounds_gep(
							self.context.i8_type(),
							block,
							&[length],
							"",
						)?
					};
					self.builder
						.build_store(end, self.context.i8_type().const_zero())?;
					frees.push((block, size));
					block.into()
				}
				Some(ForeignArg::Array { .. }) => {
					let array = self.expr(value)?;
					if array.repr == Repr::Never {
						return Ok(array);
					}
					let (items, _) =
						self.array_parts(array.value.expect("an array").into_pointer_value())?;
					items.into()
				}
				Some(ForeignArg::Address { mutated }) => {
					let target = self
						.types()
						.pointer_target(param_ty)
						.filter(|&target| target != TypeId::VOID)
						.unwrap_or_else(|| self.type_of(value));
					let target_c = self.c_type(target, value)?;
					let Some(ir) = self.c_llvm_type(target_c) else {
						return Err(self.unsupported(value.index(), "the address of a `Void`"));
					};
					let temporary = self.alloca_type(ir, "c_argument")?;
					let lowered = self.expr(value)?;
					if lowered.repr == Repr::Never {
						return Ok(lowered);
					}
					let target_repr = self.repr_of(target, value.index())?;
					let lowered = self.convert(lowered, target_repr, value.index())?;
					let c_value = self.to_c(lowered, target_c, value)?;
					self.builder.build_store(temporary, c_value)?;
					if mutated && self.is_place(value) {
						write_backs.push((temporary, value, target));
					}
					temporary.into()
				}
				Some(ForeignArg::Callback) => {
					let wrapper = self.c_callback(value)?;
					wrapper.as_global_value().as_pointer_value().into()
				}
				None => {
					let lowered = self.expr(value)?;
					if lowered.repr == Repr::Never {
						return Ok(lowered);
					}
					let repr = self.repr_of(param_ty, value.index())?;
					let lowered = self.convert(lowered, repr, value.index())?;
					self.to_c(lowered, c, value)?
				}
			};
			match self.passing(c, value)? {
				Some(Passing::Coerced(registers)) => {
					// a struct loaded as its registers
					let structure = self.c_llvm_type(c).expect("a struct");
					let slot = self.alloca_type(structure, "c_struct")?;
					self.builder.build_store(slot, raw)?;
					let coerced = self.context.struct_type(&registers, false);
					for (position, register) in registers.iter().enumerate() {
						let pointer =
							self.builder
								.build_struct_gep(coerced, slot, position as u32, "")?;
						arguments.push(self.builder.build_load(*register, pointer, "")?.into());
					}
				}
				Some(Passing::Memory(_)) => {
					let structure = self.c_llvm_type(c).expect("a struct");
					let slot = self.alloca_type(structure, "c_struct")?;
					self.builder.build_store(slot, raw)?;
					arguments.push(slot.into());
				}
				_ => arguments.push(raw.into()),
			}
		}
		let result = self.builder.build_call(function, &arguments, "")?;
		for (block, size) in frees {
			let free = self.runtime.free()?;
			let align = self.usize.const_int(16, false);
			let size = self.builder.build_int_cast(size, self.usize, "")?;
			self.builder
				.build_call(free, &[block.into(), size.into(), align.into()], "")?;
		}
		for (temporary, place, target) in write_backs {
			let value = self.load_c(temporary, target, place)?;
			let slot = self.place(place)?;
			self.store(slot, value, place)?;
		}
		if sig.ret == TypeId::NEVER {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		let raw = match (ret_passing, sret) {
			(None, _) => return Ok(Val::void()),
			(Some(Passing::Memory(structure)), Some(slot)) => {
				self.builder.build_load(structure, slot, "")?
			}
			(Some(Passing::Coerced(registers)), _) => {
				let structure = self.c_llvm_type(c_ret).expect("a struct");
				let slot = self.alloca_type(structure, "c_result")?;
				let returned = result.try_as_basic_value().basic().expect("a result");
				self.builder.build_store(slot, returned)?;
				let _ = registers;
				self.builder.build_load(structure, slot, "")?
			}
			_ => result.try_as_basic_value().basic().expect("a result"),
		};
		self.c_to_fa(raw, sig.ret, node)
	}

	/// The `ccc` wrapper of a `function` given to C as a callback.
	fn c_callback(&mut self, value: NodeId) -> Result<FunctionValue<'ctx>, Error> {
		let node = match self.view(value) {
			View::Access { members, .. } => members.last().unwrap_or(value),
			View::Group(inner) => inner,
			_ => value,
		};
		let Resolution::Local(symbol) = self.analysis.resolution(node) else {
			return Err(self.unsupported(value.index(), "this callback"));
		};
		let declaration = self.analysis.symbols.symbol(symbol).decl_node;
		let name = format!("fa.callback.{}", declaration.index());
		if let Some(function) = self.module.get_function(&name) {
			return Ok(function);
		}
		let Some(callee) = self.functions.get(&declaration) else {
			return Err(self.unsupported(value.index(), "a callback of another file"));
		};
		if callee.lambda || callee.throws || callee.params.iter().any(|(_, default)| *default) {
			return Err(self.unsupported(value.index(), "this callback"));
		}
		let (target, params, ret) = (callee.value, callee.params.clone(), callee.ret);
		// C gives the Fa representations of these types directly
		let c_of = |repr: Repr| -> Option<BasicTypeEnum<'ctx>> {
			match repr {
				Repr::Bool
				| Repr::Int(_)
				| Repr::F32
				| Repr::F64
				| Repr::Pointer
				| Repr::Object(_) => Some(self.llvm_type(repr)),
				_ => None,
			}
		};
		let mut ir_params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
		for (repr, _) in &params {
			match c_of(*repr) {
				Some(ty) => ir_params.push(ty.into()),
				None => {
					return Err(self.unsupported(value.index(), "the parameters of this callback"));
				}
			}
		}
		let function_type = match ret {
			Repr::Void => self.context.void_type().fn_type(&ir_params, false),
			repr => match c_of(repr) {
				Some(ty) => ty.fn_type(&ir_params, false),
				None => return Err(self.unsupported(value.index(), "the result of this callback")),
			},
		};
		let (function, builder) = self.helper_function(&name, function_type);
		let arguments: Vec<BasicMetadataValueEnum<'ctx>> = function
			.get_param_iter()
			.map(BasicMetadataValueEnum::from)
			.collect();
		let call = builder.build_call(target, &arguments, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		match call.try_as_basic_value().basic() {
			Some(result) if ret != Repr::Void => builder.build_return(Some(&result))?,
			_ => builder.build_return(None)?,
		};
		Ok(function)
	}

	// ── pointers ────────────────────────────────────────────────────────

	/// `pointer.read()`, `pointer.write(value)`, `pointer.offset(count)`,
	/// `pointer.toArray(count)`, `text.toString()`.
	pub(crate) fn pointer_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &fa_analyzer::analysis::CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let receiver = self.receiver(node)?;
		if receiver.repr == Repr::Never {
			return Ok(receiver);
		}
		let View::Call { callee, .. } = self.view(node) else {
			return Err(self.unsupported(node.index(), "this pointer method"));
		};
		let View::Access { target, .. } = self.view(callee) else {
			return Err(self.unsupported(node.index(), "this pointer method"));
		};
		let pointer_type = self.type_of(target);
		let pointer = receiver.value.expect("a pointer").into_pointer_value();
		let argument = info.arguments.iter().find_map(|argument| match argument {
			Argument::Given(value) => Some(*value),
			_ => None,
		});
		let target_type = self.types().pointer_target(pointer_type);
		match (path, target_type, argument) {
			("Pointer#read", Some(target), _) => self.load_c(pointer, target, node),
			("Pointer#write", Some(target), Some(argument)) => {
				let value = self.expr(argument)?;
				if value.repr == Repr::Never {
					return Ok(value);
				}
				let repr = self.repr_of(target, argument.index())?;
				let value = self.convert(value, repr, argument.index())?;
				let c = self.c_type(target, argument)?;
				let raw = self.to_c(value, c, argument)?;
				self.builder.build_store(pointer, raw)?;
				Ok(Val::void())
			}
			("Pointer#offset", Some(target), Some(argument)) => {
				let count = self.expr(argument)?;
				let count = count.value.expect("a count").into_int_value();
				let c = self.c_type(target, node)?;
				let item = self
					.c_llvm_type(c)
					.unwrap_or_else(|| self.context.i8_type().into());
				let moved = unsafe { self.builder.build_gep(item, pointer, &[count], "")? };
				Ok(Val::new(Repr::Pointer, moved))
			}
			("Pointer#toArray", Some(target), Some(argument)) => {
				let count = self.expr(argument)?;
				let count = count.value.expect("a count").into_int_value();
				let count = self
					.builder
					.build_int_cast(count, self.context.i64_type(), "")?;
				let repr = self.repr_of(target, node.index())?;
				let c = self.c_type(target, node)?;
				if self.c_llvm_type(c) != Some(self.llvm_type(repr)) {
					return Err(self.unsupported(node.index(), "`toArray` of this pointer"));
				}
				let array = self.new_array(repr, count, count)?;
				let (items, _) = self.array_parts(array)?;
				let size = self.llvm_type(repr).size_of().expect("a sized item");
				let size = self
					.builder
					.build_int_cast(size, self.context.i64_type(), "")?;
				let bytes = self.builder.build_int_mul(size, count, "")?;
				self.builder.build_memcpy(items, 1, pointer, 1, bytes)?;
				Ok(self.fresh(Val::new(Repr::Array(target), array)))
			}
			("CString#toString", _, _) => {
				let strlen = match self.module.get_function("strlen") {
					Some(function) => function,
					None => self.module.add_function(
						"strlen",
						self.usize.fn_type(&[self.pointer_type().into()], false),
						Some(Linkage::External),
					),
				};
				let length = self.call_value(strlen, &[pointer.into()])?.into_int_value();
				let empty = self.pointer_type().const_null();
				let result = self.call_with_pair(
					"fa_concat",
					&[
						pointer.into(),
						length.into(),
						empty.into(),
						self.usize.const_zero().into(),
					],
					Repr::Str,
				)?;
				Ok(self.fresh(result))
			}
			_ => Err(self.unsupported(node.index(), "this pointer method")),
		}
	}

	/// `pointer.value`: the value it designates, of type `target` (a copy).
	pub(crate) fn pointer_value(
		&mut self,
		value: Val<'ctx>,
		target: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let pointer = value.value.expect("a pointer").into_pointer_value();
		self.load_c(pointer, target, node)
	}

	// ── destroy ─────────────────────────────────────────────────────────

	/// The `destroy` function of a resource type (foreign code), lowered.
	pub(crate) fn destroy_function(&self, ty: TypeId) -> Option<FunctionValue<'ctx>> {
		if !self.types().is_resource(ty) {
			return None;
		}
		let Some(declaration) = self.analysis.type_decls.values().find(|decl| decl.ty == ty) else {
			// a type of another file of the program
			return self
				.destroy_in_other_files(ty)
				.or_else(|| self.remote_destroy(ty));
		};
		let symbols = &self.analysis.symbols;
		declaration
			.functions
			.iter()
			.map(|&function| symbols.symbol(function))
			.find(|symbol| self.session.types().syms.resolve(symbol.name) == "destroy")
			.and_then(|symbol| self.functions.get(&symbol.decl_node))
			.map(|function| function.value)
	}
}
