//! How Fa values are represented in LLVM IR, and the conversions between
//! representations.

use fa_analyzer::ids::{NominalId, TypeId};
use fa_analyzer::types::{CollKind, FloatBits, IntStorage, NominalKind, TypeKind};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue};

use crate::error::Error;
use crate::lower::Lowering;

/// The representation of an integer: its width and signedness. An integer
/// node of an arithmetic tree is computed in its compute type (values brief
/// §6.7), so its representation can be wider than its type's storage.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct IntRepr {
	pub bits: u32,
	pub signed: bool,
}

impl IntRepr {
	pub const I64: IntRepr = IntRepr {
		bits: 64,
		signed: true,
	};

	/// The representation of an integer storage.
	pub fn of(storage: IntStorage) -> IntRepr {
		IntRepr {
			bits: storage.bits(),
			signed: storage.is_signed(),
		}
	}

	/// A representation holding every value of both.
	pub fn common(self, other: IntRepr) -> IntRepr {
		if self.signed == other.signed {
			return IntRepr {
				bits: self.bits.max(other.bits),
				signed: self.signed,
			};
		}
		let (signed, unsigned) = if self.signed {
			(self, other)
		} else {
			(other, self)
		};
		let bits = if unsigned.bits < signed.bits {
			signed.bits
		} else {
			(unsigned.bits * 2).min(128)
		};
		IntRepr { bits, signed: true }
	}
}

/// The representation of the value of an optional (`T?`).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Scalar {
	Bool,
	Int(IntRepr),
	F64,
	F32,
	Percentage,
	Str,
	Enum(NominalId),
	Union,
	Object(TypeId),
	Array(TypeId),
}

impl Scalar {
	pub fn repr(self) -> Repr {
		match self {
			Scalar::Bool => Repr::Bool,
			Scalar::Int(int) => Repr::Int(int),
			Scalar::F64 => Repr::F64,
			Scalar::F32 => Repr::F32,
			Scalar::Percentage => Repr::Percentage,
			Scalar::Str => Repr::Str,
			Scalar::Enum(nominal) => Repr::Enum(nominal),
			Scalar::Union => Repr::Union,
			Scalar::Object(ty) => Repr::Object(ty),
			Scalar::Array(item) => Repr::Array(item),
		}
	}

	pub fn of(repr: Repr) -> Option<Scalar> {
		Some(match repr {
			Repr::Bool => Scalar::Bool,
			Repr::Int(int) => Scalar::Int(int),
			Repr::F64 => Scalar::F64,
			Repr::F32 => Scalar::F32,
			Repr::Percentage => Scalar::Percentage,
			Repr::Str => Scalar::Str,
			Repr::Enum(nominal) => Scalar::Enum(nominal),
			Repr::Union => Scalar::Union,
			Repr::Object(ty) => Scalar::Object(ty),
			Repr::Array(item) => Scalar::Array(item),
			_ => return None,
		})
	}
}

/// The representation of a value.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Repr {
	/// No value: a statement.
	Void,
	/// No value: the code after it is unreachable.
	Never,
	/// `i1`
	Bool,
	/// `iN`
	Int(IntRepr),
	/// `double`
	F64,
	/// `float`
	F32,
	/// A `Percentage`: a `double` holding the fraction.
	Percentage,
	/// A string: `{ ptr, usize }`, UTF-8 bytes.
	Str,
	/// `none`: no value at run time.
	None,
	/// `T?`: `{ i1 present, T }`.
	Optional(Scalar),
	/// A member of an enumeration: its index, an `i32`.
	Enum(NominalId),
	/// A value of a named union or `Errors` type (an error): a pointer to a
	/// box holding the global id of its member (`i64`), then its data.
	Union,
	/// An object, by value: a struct of its fields in the order of its shape.
	Object(TypeId),
	/// An array of items of this type: a pointer to `{ ptr items, i64 length, i64 capacity }`.
	Array(TypeId),
}

impl Repr {
	/// Whether values of this representation exist at run time.
	pub fn has_value(self) -> bool {
		!matches!(self, Repr::Void | Repr::Never | Repr::None)
	}
}

/// A lowered value: its representation and, unless `Void` or `Never`, its IR value.
#[derive(Copy, Clone, Debug)]
pub struct Val<'ctx> {
	pub repr: Repr,
	pub value: Option<BasicValueEnum<'ctx>>,
}

impl<'ctx> Val<'ctx> {
	pub fn new(repr: Repr, value: impl Into<BasicValueEnum<'ctx>>) -> Self {
		Val {
			repr,
			value: Some(value.into()),
		}
	}

	pub fn void() -> Self {
		Val {
			repr: Repr::Void,
			value: None,
		}
	}

	pub fn never() -> Self {
		Val {
			repr: Repr::Never,
			value: None,
		}
	}

	/// The integer value (a bug of the lowering when it isn't one).
	pub fn int(&self) -> IntValue<'ctx> {
		self.value.expect("an integer value").into_int_value()
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The representation of the values of a type.
	pub(crate) fn repr_of(&self, ty: TypeId, node: usize) -> Result<Repr, Error> {
		let types = self.types();
		Ok(match types.kind(ty) {
			TypeKind::Never => Repr::Never,
			TypeKind::Void => Repr::Void,
			TypeKind::Boolean | TypeKind::BoolLit(_) => Repr::Bool,
			TypeKind::Int(id) => Repr::Int(IntRepr::of(types.int(id).storage)),
			TypeKind::Float(FloatBits::F64) | TypeKind::FloatLit(FloatBits::F64, _) => Repr::F64,
			TypeKind::Float(FloatBits::F32) | TypeKind::FloatLit(FloatBits::F32, _) => Repr::F32,
			TypeKind::Percentage => Repr::Percentage,
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => Repr::Str,
			TypeKind::None => Repr::None,
			TypeKind::Enum(nominal, _) | TypeKind::EnumMember(nominal, _) => Repr::Enum(nominal),
			TypeKind::ErrorTop => Repr::Union,
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _)
				if matches!(
					types.nominal(nominal).kind,
					NominalKind::Union | NominalKind::Errors
				) =>
			{
				Repr::Union
			}
			TypeKind::Object(_) => Repr::Object(ty),
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => {
				Repr::Array(item)
			}
			TypeKind::Union(list) => {
				// a union of literals of one kind (`0 | 1`, `"a" | "b"`), with `none` or not
				let items = types.list_items(list);
				let mut common: Option<Repr> = None;
				for &item in items.iter().filter(|&&item| item != TypeId::NONE) {
					let repr = self.repr_of(item, node)?;
					common = Some(match (common, repr) {
						(None, repr) => repr,
						(Some(Repr::Int(a)), Repr::Int(b)) => Repr::Int(a.common(b)),
						(Some(a), b) if a == b => a,
						_ => {
							return Err(self.unsupported(
								node,
								&format!("values of type `{}`", self.session.display_type(ty)),
							));
						}
					});
				}
				let Some(common) = common else {
					return Ok(Repr::None);
				};
				if !items.contains(&TypeId::NONE) {
					return Ok(common);
				}
				match Scalar::of(common) {
					Some(scalar) => Repr::Optional(scalar),
					None => {
						return Err(self.unsupported(
							node,
							&format!("values of type `{}`", self.session.display_type(ty)),
						));
					}
				}
			}
			_ => {
				return Err(self.unsupported(
					node,
					&format!("values of type `{}`", self.session.display_type(ty)),
				));
			}
		})
	}

	/// The integer type of `bits` bits.
	pub(crate) fn int_type(&self, bits: u32) -> inkwell::types::IntType<'ctx> {
		match bits {
			1 => self.context.bool_type(),
			8 => self.context.i8_type(),
			16 => self.context.i16_type(),
			32 => self.context.i32_type(),
			64 => self.context.i64_type(),
			128 => self.context.i128_type(),
			_ => panic!("no integer representation has {bits} bits"),
		}
	}

	/// The struct of an object type: its fields in the order of its shape.
	pub(crate) fn object_type(&self, ty: TypeId) -> inkwell::types::StructType<'ctx> {
		if let Some(structure) = self.object_types.borrow().get(&ty) {
			return *structure;
		}
		let fields: Vec<BasicTypeEnum<'ctx>> = self
			.types()
			.fields_of(ty)
			.iter()
			.map(|field| {
				let repr = self.repr_of(field.ty, 0).unwrap_or(Repr::Bool);
				if repr.has_value() {
					self.llvm_type(repr)
				} else {
					self.context.bool_type().into()
				}
			})
			.collect();
		let structure = self.context.struct_type(&fields, false);
		self.object_types.borrow_mut().insert(ty, structure);
		structure
	}

	/// The IR type of a representation with values.
	pub(crate) fn llvm_type(&self, repr: Repr) -> BasicTypeEnum<'ctx> {
		match repr {
			Repr::Bool => self.context.bool_type().into(),
			Repr::Int(int) => self.int_type(int.bits).into(),
			Repr::F64 | Repr::Percentage => self.context.f64_type().into(),
			Repr::F32 => self.context.f32_type().into(),
			Repr::Str => self.pair_type.into(),
			Repr::Enum(_) => self.context.i32_type().into(),
			Repr::Union | Repr::Array(_) => self
				.context
				.ptr_type(inkwell::AddressSpace::default())
				.into(),
			Repr::Object(ty) => self.object_type(ty).into(),
			Repr::Optional(scalar) => self
				.context
				.struct_type(
					&[
						self.context.bool_type().into(),
						self.llvm_type(scalar.repr()),
					],
					false,
				)
				.into(),
			Repr::Void | Repr::Never | Repr::None => panic!("{repr:?} has no IR type"),
		}
	}

	/// Converts an integer to another representation: extends by its own
	/// signedness, or truncates. The caller has proven that the value fits.
	pub(crate) fn int_to(
		&self,
		value: IntValue<'ctx>,
		from: IntRepr,
		to: IntRepr,
	) -> Result<IntValue<'ctx>, Error> {
		let target = self.int_type(to.bits);
		Ok(if to.bits == from.bits {
			value
		} else if to.bits > from.bits {
			if from.signed {
				self.builder.build_int_s_extend(value, target, "")?
			} else {
				self.builder.build_int_z_extend(value, target, "")?
			}
		} else {
			self.builder.build_int_truncate(value, target, "")?
		})
	}

	/// Converts a value to the representation of a sink whose type accepts it
	/// (the analyzer checked it, and range-checked what needed it).
	pub(crate) fn convert(
		&self,
		value: Val<'ctx>,
		to: Repr,
		node: usize,
	) -> Result<Val<'ctx>, Error> {
		if value.repr == to || !to.has_value() || value.repr == Repr::Never {
			return Ok(Val {
				repr: if to.has_value() { value.repr } else { to },
				value: if to.has_value() { value.value } else { None },
			});
		}
		if let Repr::Optional(scalar) = to {
			// `none`, or a value injected into an optional
			let ty = self.llvm_type(to).into_struct_type();
			if value.repr == Repr::None {
				return Ok(Val::new(to, ty.const_zero()));
			}
			let inner = self.convert(value, scalar.repr(), node)?;
			let present = self.context.bool_type().const_int(1, false);
			let pair = self
				.builder
				.build_insert_value(ty.get_undef(), present, 0, "")?;
			let pair =
				self.builder
					.build_insert_value(pair, inner.value.expect("a value"), 1, "")?;
			return Ok(Val::new(to, pair.into_struct_value()));
		}
		let Some(raw) = value.value else {
			return Err(self.unsupported(node, "a value of type `Void`"));
		};
		Ok(match (value.repr, to) {
			(Repr::Int(from), Repr::Int(int)) => {
				Val::new(to, self.int_to(raw.into_int_value(), from, int)?)
			}
			(Repr::Int(from), Repr::F64 | Repr::F32) => {
				let target = if to == Repr::F64 {
					self.context.f64_type()
				} else {
					self.context.f32_type()
				};
				let raw = raw.into_int_value();
				Val::new(
					to,
					if from.signed {
						self.builder.build_signed_int_to_float(raw, target, "")?
					} else {
						self.builder.build_unsigned_int_to_float(raw, target, "")?
					},
				)
			}
			(Repr::F32, Repr::F64) => Val::new(
				to,
				self.builder.build_float_ext(
					raw.into_float_value(),
					self.context.f64_type(),
					"",
				)?,
			),
			(Repr::F64, Repr::F32) => Val::new(
				to,
				self.builder.build_float_trunc(
					raw.into_float_value(),
					self.context.f32_type(),
					"",
				)?,
			),
			(from, to) => {
				return Err(
					self.unsupported(node, &format!("converting a value from {from:?} to {to:?}"))
				);
			}
		})
	}
}
