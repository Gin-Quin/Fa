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
	InlineUnion(NominalId),
	Object(TypeId),
	Array(TypeId),
	Tuple(TypeId),
	Range,
	Rec(TypeId),
	Map(TypeId),
	Set(TypeId),
	Growing(TypeId),
	Pointer,
	Sync(TypeId),
	Links(TypeId),
}

impl Scalar {
	/// Boxes and recursive edges are always non-null when present. Their
	/// optional uses the otherwise unused null pointer as its absence tag.
	pub(crate) fn nullable(self) -> bool {
		matches!(self, Scalar::Rec(_) | Scalar::Union)
	}

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
			Scalar::InlineUnion(nominal) => Repr::InlineUnion(nominal),
			Scalar::Object(ty) => Repr::Object(ty),
			Scalar::Array(item) => Repr::Array(item),
			Scalar::Tuple(ty) => Repr::Tuple(ty),
			Scalar::Range => Repr::Range,
			Scalar::Rec(ty) => Repr::Rec(ty),
			Scalar::Map(ty) => Repr::Map(ty),
			Scalar::Set(ty) => Repr::Set(ty),
			Scalar::Growing(item) => Repr::Growing(item),
			Scalar::Pointer => Repr::Pointer,
			Scalar::Sync(ty) => Repr::Sync(ty),
			Scalar::Links(element) => Repr::Links(element),
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
			Repr::InlineUnion(nominal) => Scalar::InlineUnion(nominal),
			Repr::Object(ty) => Scalar::Object(ty),
			Repr::Array(item) => Scalar::Array(item),
			Repr::Tuple(ty) => Scalar::Tuple(ty),
			Repr::Range => Scalar::Range,
			Repr::Rec(ty) => Scalar::Rec(ty),
			Repr::Map(ty) => Scalar::Map(ty),
			Repr::Set(ty) => Scalar::Set(ty),
			Repr::Growing(item) => Scalar::Growing(item),
			Repr::Pointer => Scalar::Pointer,
			Repr::Sync(ty) => Scalar::Sync(ty),
			Repr::Links(element) => Scalar::Links(element),
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
	/// `T?`: a nullable pointer for boxes/recursive edges, otherwise `{ i1, T }`.
	Optional(Scalar),
	/// A member of an enumeration: its index, an `i32`.
	Enum(NominalId),
	/// A value of a named union or `Errors` type (an error): a pointer to a
	/// box holding the global id of its member (`i64`), then its data.
	Union,
	/// A small, nonrecursive named union: tag and maximum payload inline.
	InlineUnion(NominalId),
	/// An object, by value: a struct of its fields in the order of its shape.
	Object(TypeId),
	/// An array of items of this type: a pointer to `{ ptr items, i64 length, i64 capacity }`.
	Array(TypeId),
	/// A function value of this type: `{ ptr function, ptr environment }`.
	Function(TypeId),
	/// A tuple, by value: a struct of its items.
	Tuple(TypeId),
	/// A range: `{ i64 start, i64 end, i8 flags }`.
	Range,
	/// The recursive edge of a type to this type (`next: Node?` in `Node`):
	/// a pointer to a heap block holding the value, owned by its parent
	/// (`TypeKind::Rec`). A named union is a box already: its edge is the box.
	Rec(TypeId),
	/// A map of this collection type: a pointer to a table (`tables.rs`).
	Map(TypeId),
	/// A set of this collection type: a pointer to a table.
	Set(TypeId),
	/// A `GrowingArray` of items of this type: a pointer to segments that
	/// never move (`growing.rs`).
	Growing(TypeId),
	/// A `Pool` or `GrowingPool` of elements of this type: a pointer to a
	/// slab whose elements never move (`pools.rs`).
	Pool(TypeId),
	/// A `Pointer(T)` or a `CString` of foreign code: a `ptr` Fa doesn't
	/// manage (`lower/foreign.rs`).
	Pointer,
	/// A value of `std.concurrency` of this type (`parallel.rs`): a pointer
	/// to a reference-counted block (a guard: to its mutex's block).
	Sync(TypeId),
	/// A relation side holding one element of this object type (a direct
	/// side, a reverse one-to-one side, a symmetric side, an owner field):
	/// the element's address, null for `none` (`relations.rs`). It reads
	/// as an optional view of the element.
	Link(TypeId),
	/// A relation side holding several elements of this object type (a
	/// many-to-many direct side, a `Pool` reverse side): a pointer to a set
	/// of addresses of the runtime (`FaLinks`), null while empty.
	Links(TypeId),
	/// A `Columnar` or `GrowingColumnar` of objects of this item type, by
	/// columns: a pointer to a pool of 4-byte slots followed by one run per
	/// field in each chunk (`columns.rs`).
	Columnar(TypeId),
	/// The place of an element of a columnar, only in a [`Slot`](super::Slot):
	/// the address of its slot (`columns.rs`); loading it gathers the
	/// object of this type, storing it scatters it.
	Row(TypeId),
	/// A `Subscription`: the observer of a `watch` block (`reactivity.rs`),
	/// which lives until the end of the program.
	Watch,
	/// A `Task(T) throws E` of this type (`tasks.rs`): the frame of its
	/// coroutine or a finished block; 1 once cancelled.
	Task(TypeId),
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
				if self.inline_union_words(nominal).is_some() {
					Repr::InlineUnion(nominal)
				} else {
					Repr::Union
				}
			}
			TypeKind::Object(_) => Repr::Object(ty),
			TypeKind::Param(param) => match self.bindings.borrow().get(&param) {
				Some(repr) => *repr,
				None => {
					return Err(self.unsupported(node, "a generic type outside an instance"));
				}
			},
			TypeKind::Function(_) => Repr::Function(ty),
			TypeKind::Tuple(_) => Repr::Tuple(ty),
			TypeKind::Range => Repr::Range,
			TypeKind::Pointer(_) | TypeKind::CString => Repr::Pointer,
			TypeKind::Subscription => Repr::Watch,
			TypeKind::Task(..) => Repr::Task(ty),
			TypeKind::Rec(_) => {
				let Some(target) = types.rec_target(ty) else {
					return Err(self.unsupported(node, "this recursive type"));
				};
				match self.repr_of(target, node)? {
					// already a pointer
					repr @ (Repr::Union | Repr::Array(_)) => repr,
					_ => Repr::Rec(target),
				}
			}
			TypeKind::Collection(CollKind::Array, item, _) => Repr::Array(item),
			TypeKind::Collection(CollKind::GrowingArray, item, _) => Repr::Growing(item),
			// a columnar is stored like a pool for now, by rows (the column
			// layout of `collections/columnars.md` is not implemented); an
			// ordered columnar like an array
			TypeKind::Collection(CollKind::Pool | CollKind::GrowingPool, item, _) => {
				Repr::Pool(item)
			}
			// by columns, unless its elements have relation sides
			TypeKind::Collection(CollKind::Columnar | CollKind::GrowingColumnar, item, _) => {
				match self.columnar_element(self.repr_of(item, node)?) {
					Some(_) => Repr::Columnar(item),
					None => Repr::Pool(item),
				}
			}
			TypeKind::Collection(CollKind::OrderedColumnar, item, _) => Repr::Array(item),
			// a relation side holding several elements (`relations.rs`)
			TypeKind::Collection(CollKind::RelationPool(_), item, _) => {
				match self.repr_of(item, node)? {
					Repr::Object(element) | Repr::Rec(element) => Repr::Links(element),
					_ => return Err(self.unsupported(node, "this relation side")),
				}
			}
			// a reference is the value it designates (`T in pool`)
			TypeKind::Ref(target, _) => return self.repr_of(target, node),
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, _) => Repr::Map(ty),
			// a worker is its state (parallel.rs)
			TypeKind::Sync(fa_analyzer::types::SyncKind::Worker, state) => {
				return self.repr_of(state, node);
			}
			TypeKind::Sync(kind, _) if kind != fa_analyzer::types::SyncKind::Ordering => {
				Repr::Sync(ty)
			}
			TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, _, _) => Repr::Set(ty),
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
						// members of different kinds: a box
						_ => Repr::Union,
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
		let generic = !self.bindings.borrow().is_empty();
		if !generic && let Some(structure) = self.object_types.borrow().get(&ty) {
			return *structure;
		}
		let fields: Vec<BasicTypeEnum<'ctx>> = self
			.object_fields(ty)
			.into_iter()
			.map(|repr| self.llvm_type(repr))
			.collect();
		let structure = self.context.struct_type(&fields, false);
		if !generic {
			self.object_types.borrow_mut().insert(ty, structure);
		}
		structure
	}

	/// The IR type of a representation with values.
	pub(crate) fn llvm_type(&self, repr: Repr) -> BasicTypeEnum<'ctx> {
		match repr {
			Repr::Bool => self.context.bool_type().into(),
			Repr::Int(int) => self.int_type(int.bits).into(),
			Repr::F64 | Repr::Percentage => self.context.f64_type().into(),
			Repr::F32 => self.context.f32_type().into(),
			Repr::Str => self.string_type.into(),
			Repr::Enum(_) => self.context.i32_type().into(),
			Repr::InlineUnion(nominal) => self.inline_union_type(nominal).into(),
			Repr::Union
			| Repr::Array(_)
			| Repr::Rec(_)
			| Repr::Map(_)
			| Repr::Set(_)
			| Repr::Growing(_)
			| Repr::Pool(_)
			| Repr::Pointer
			| Repr::Sync(_)
			| Repr::Link(_)
			| Repr::Links(_)
			| Repr::Columnar(_)
			| Repr::Task(_)
			| Repr::Watch => self
				.context
				.ptr_type(inkwell::AddressSpace::default())
				.into(),
			Repr::Object(ty) | Repr::Row(ty) => self.object_type(ty).into(),
			Repr::Function(_) => self.closure_type().into(),
			Repr::Tuple(ty) => self.tuple_type(ty).into(),
			Repr::Range => self.range_type().into(),
			Repr::Optional(scalar) if scalar.nullable() => self.pointer_type().into(),
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

	/// Decode an optional using the given builder (also used in generated
	/// copy, drop and hash functions).
	pub(crate) fn split_optional(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		value: Val<'ctx>,
	) -> Result<(IntValue<'ctx>, Val<'ctx>), Error> {
		let Repr::Optional(scalar) = value.repr else {
			unreachable!("an optional")
		};
		let raw = value.value.expect("an optional");
		if scalar.nullable() {
			return Ok((
				builder.build_is_not_null(raw.into_pointer_value(), "")?,
				Val::new(scalar.repr(), raw),
			));
		}
		let pair = raw.into_struct_value();
		Ok((
			builder.build_extract_value(pair, 0, "")?.into_int_value(),
			Val::new(scalar.repr(), builder.build_extract_value(pair, 1, "")?),
		))
	}

	pub(crate) fn pack_optional(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		scalar: Scalar,
		present: IntValue<'ctx>,
		inner: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if scalar.nullable() {
			return Ok(builder.build_select(
				present,
				inner,
				self.pointer_type().const_null().into(),
				"",
			)?);
		}
		let ty = self.llvm_type(Repr::Optional(scalar)).into_struct_type();
		let pair = builder.build_insert_value(ty.get_undef(), present, 0, "")?;
		Ok(builder
			.build_insert_value(pair, inner, 1, "")?
			.into_struct_value()
			.into())
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

	/// Whether two representations have the same layout and field names, so
	/// that a value of one is a value of the other (two recursive types
	/// with the same unfolding, `Node` and `Link`). Recursive edges are
	/// compared by their layout: pointers.
	pub(crate) fn same_layout(&self, from: Repr, to: Repr) -> bool {
		let (from, to) = match (from, to) {
			(Repr::Rec(a), Repr::Rec(b)) => match (self.repr_of(a, 0), self.repr_of(b, 0)) {
				(Ok(a), Ok(b)) => (a, b),
				_ => return false,
			},
			(Repr::Optional(a), Repr::Optional(b)) => {
				return a != b && self.same_layout(a.repr(), b.repr());
			}
			pair => pair,
		};
		let (Repr::Object(a), Repr::Object(b)) = (from, to) else {
			return false;
		};
		if a == b {
			return true;
		}
		let types = self.types();
		let names =
			|ty: TypeId| -> Vec<_> { types.fields_of(ty).iter().map(|field| field.name).collect() };
		names(a) == names(b) && self.object_type(a) == self.object_type(b)
	}

	/// An optional converted to an optional of another representation (`Node?`
	/// stored as the recursive edge `Node?`): the value, when present. A
	/// temporary gives its value to the conversion when the conversion takes
	/// it (boxes it).
	fn convert_optional(
		&self,
		value: Val<'ctx>,
		from: Scalar,
		scalar: Scalar,
		node: usize,
	) -> Result<Val<'ctx>, Error> {
		let to = Repr::Optional(scalar);
		let (present, inner) = self.split_optional(&self.builder, value)?;
		let inner = inner.value.expect("an inner value");
		let ty = self.llvm_type(to);
		let start = self.builder.get_insert_block().expect("a block");
		let function = start.get_parent().expect("a function");
		let some = self.context.append_basic_block(function, "convert_some");
		let join = self.context.append_basic_block(function, "converted");
		self.builder.build_conditional_branch(present, some, join)?;
		self.builder.position_at_end(some);
		let moved = self.is_temporary(value);
		let inner = Val::new(from.repr(), inner);
		if moved {
			self.fresh(inner);
		}
		let converted = self.convert(inner, scalar.repr(), node)?;
		// whether the conversion took the value (and the optional gives it up)
		let inner_moved = moved && !self.is_temporary(inner);
		if moved && !inner_moved {
			self.take(inner, None)?;
		}
		let owned = self.is_temporary(converted);
		if owned {
			self.take(converted, None)?;
		}
		let wrapped = self.pack_optional(
			&self.builder,
			scalar,
			self.context.bool_type().const_int(1, false),
			converted.value.expect("a value"),
		)?;
		let some_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		let phi = self.builder.build_phi(ty, "")?;
		phi.add_incoming(&[(&super::zero_of(ty), start), (&wrapped, some_end)]);
		let result = Val::new(to, phi.as_basic_value());
		if inner_moved {
			self.take(value, None)?;
		}
		if owned {
			self.fresh(result);
		}
		Ok(result)
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
		if to == Repr::Union && value.repr != Repr::Union {
			return self.box_value(value, node);
		}
		// a relation side starts empty
		if let (Repr::None, Repr::Link(_) | Repr::Links(_)) = (value.repr, to) {
			return Ok(Val::new(to, self.pointer_type().const_null()));
		}
		if self.same_layout(value.repr, to) {
			// `Node` and `Link`, with the same unfolding: the same bits
			return Ok(Val::new(to, value.value.expect("a value")));
		}
		if let Repr::Rec(target) = value.repr
			&& !matches!(to, Repr::Optional(_))
		{
			// the value the edge holds (a view of it)
			let unfolded = self.unfold_rec(value, target, node)?;
			return self.convert(unfolded, to, node);
		}
		if let Repr::Rec(target) = to {
			let target_repr = self.repr_of(target, node)?;
			let value = self.convert(value, target_repr, node)?;
			return self.box_rec(value, to);
		}
		if let (Repr::Optional(from), Repr::Optional(scalar)) = (value.repr, to)
			&& from != scalar
		{
			return self.convert_optional(value, from, scalar, node);
		}
		if let Repr::Optional(scalar) = to {
			// `none`, or a value injected into an optional
			let ty = self.llvm_type(to);
			if value.repr == Repr::None {
				return Ok(Val::new(to, super::zero_of(ty)));
			}
			let inner = self.convert(value, scalar.repr(), node)?;
			let present = self.context.bool_type().const_int(1, false);
			let pair = self.pack_optional(
				&self.builder,
				scalar,
				present,
				inner.value.expect("a value"),
			)?;
			let wrapped = Val::new(to, pair);
			// the optional of a temporary is the temporary
			self.transfer(inner, wrapped);
			return Ok(wrapped);
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
			// items of the same representation (an instance's `Array(Item)`)
			(Repr::Array(a), Repr::Array(b))
				if self.repr_of(a, node)? == self.repr_of(b, node)?
					|| self.repr_of(a, node)? == Repr::Never =>
			{
				Val::new(to, raw)
			}
			(Repr::Growing(a), Repr::Growing(b))
				if self.repr_of(a, node)? == self.repr_of(b, node)?
					|| self.repr_of(a, node)? == Repr::Never =>
			{
				Val::new(to, raw)
			}
			(Repr::Array(a), Repr::Growing(b)) | (Repr::Growing(a), Repr::Array(b))
				if self.repr_of(a, node)? == self.repr_of(b, node)?
					|| self.repr_of(a, node)? == Repr::Never =>
			{
				self.convert_growing(value, to)?
			}
			(Repr::Function(_), Repr::Function(_)) => Val::new(to, raw),

			// tables of the same keys and values (`GrowingMap`, literal types)
			(Repr::Map(_), Repr::Map(_)) | (Repr::Set(_), Repr::Set(_)) => {
				let (a, b) = (self.table_of(value.repr)?, self.table_of(to)?);
				if a.key != b.key || a.value != b.value || a.growing != b.growing {
					return Err(self.unsupported(node, "converting this collection"));
				}
				Val::new(to, raw)
			}
			// structural typing: the fields of the target, by name
			(Repr::Object(from_type), Repr::Object(to_type)) => {
				let source = raw.into_struct_value();
				let target_struct = self.object_type(to_type);
				let mut value = target_struct.get_undef();
				let from_fields = self.types().fields_of(from_type).to_vec();
				let to_fields = self.types().fields_of(to_type).to_vec();
				for (index, field) in to_fields.iter().enumerate() {
					let Some(position) =
						from_fields.iter().position(|from| from.name == field.name)
					else {
						return Err(self.unsupported(node, "converting this object"));
					};
					let from_repr = self.repr_of(from_fields[position].ty, node)?;
					let to_repr = self.repr_of(field.ty, node)?;
					let extracted =
						self.builder
							.build_extract_value(source, position as u32, "")?;
					let converted = self.convert(Val::new(from_repr, extracted), to_repr, node)?;
					if let Some(converted) = converted.value {
						value = self
							.builder
							.build_insert_value(value, converted, index as u32, "")?
							.into_struct_value();
					}
				}
				Val::new(to, value)
			}
			(Repr::Union | Repr::InlineUnion(_), to) if to.has_value() => self.unbox(value, to)?,
			(from, to) => {
				return Err(
					self.unsupported(node, &format!("converting a value from {from:?} to {to:?}"))
				);
			}
		})
	}
}
