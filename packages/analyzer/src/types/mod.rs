//! Types: interned, compared by id.
//!
//! A [`TypeId`] is a `u32` index into the [`TypeStore`]; structurally equal
//! types have the same id, so type equality is an integer comparison. A
//! [`TypeKind`] is `Copy` and at most 16 bytes: variable-size data (integer
//! bounds, shapes, signatures, lists) lives in hash-consed side tables of the
//! store. Facts (exact values, intervals, narrowings) are never part of a type
//! (types brief §2).

pub mod display;
pub mod global;
pub mod int;
pub mod layer;
pub mod merge;
pub mod ownership;
pub mod relate;
pub mod store;
pub mod subst;
pub mod union;

use bitflags::bitflags;
use ethnum::I256;

pub use crate::ids::TypeId;
use crate::ids::{
	ConstId, DeclKey, EffectSetId, IntId, ListId, MaskId, ModuleId, NominalId, ParamId,
	ParamListId, PathId, RecId, ReqListId, ShapeId, SigId, StepListId, Sym, SymIntId, SymListId,
};
pub use int::{ComputeType, IntStorage, IntType, Interval};
pub use ownership::{Ownership, StoreProjection};
pub use relate::{Mode, Reason};
pub use store::TypeStore;
pub use subst::{Bindings, Candidates};

/// The kind of a type. Variable data lives in the side tables of the [`TypeStore`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum TypeKind {
	/// The bottom type: `return`, `throw`, `break`… Satisfies every type.
	Never,
	/// The result of a statement: can't be bound, stored or compared.
	Void,
	/// The type of `none`.
	None,
	/// The built-in `Error`: any error.
	ErrorTop,
	/// Error recovery: satisfies and accepts everything, never reported twice.
	Unknown,
	/// `Boolean`.
	Boolean,
	/// `true` or `false` as a type.
	BoolLit(bool),
	/// An integer type: bounds and storage in the side table; a literal has `min == max`.
	Int(IntId),
	/// An integer whose bounds are affine in value parameters (generic bodies only, M2).
	SymInt(SymIntId),
	/// `Float` (64 bits) or `Float32`.
	Float(FloatBits),
	/// A float literal type.
	FloatLit(FloatBits, ConstId),
	/// `BigInt`.
	BigInt,
	/// A `BigInt` literal type.
	BigIntLit(ConstId),
	/// `Percentage`.
	Percentage,
	/// `String`.
	String,
	/// A string literal type.
	StringLit(Sym),
	/// `Character`.
	Character,
	/// An object type.
	Object(ShapeId),
	/// A tuple type.
	Tuple(ListId),
	/// An anonymous union: normalized, at least 2 members, sorted by id. `T?` is `Union[T, None]`.
	Union(ListId),
	/// A named union or `Errors` type, or a subset of its members.
	NamedUnion(NominalId, MaskId),
	/// One member of a named union or `Errors` type.
	Member(NominalId, u16),
	/// An enumeration, or a subset of its members.
	Enum(NominalId, MaskId),
	/// One member of an enumeration.
	EnumMember(NominalId, u16),
	/// A `Fields` flag tree, a key set (`Keys<T>`): its keys in order, a
	/// leaf a string literal type of its name, a group a tuple of its name
	/// and the `Fields` type of its children.
	Fields(ListId),
	/// A C structure (M3).
	CStruct(NominalId),
	/// `Pointer(T)` (foreign code): a `T` in memory that Fa doesn't manage,
	/// never `NULL`; `Pointer(Void)` is C's `void *`. `Pointer(T)?` is `NULL`able.
	Pointer(TypeId),
	/// `CString` (foreign code): a pointer to zero-terminated bytes.
	CString,
	/// `CArray(T, n)`: a fixed-size C array, as a field of a `CStruct`.
	CArray(TypeId, u32),
	/// A function type.
	Function(SigId),
	/// A collection: kind, item type, key type (`NONE` except for maps).
	Collection(CollKind, TypeId, TypeId),
	/// An iterator over a collection place (M2).
	Iterator(TypeId, PathId),
	/// A reference type `T in path` (M2).
	Ref(TypeId, PathId),
	/// `Range`.
	Range,
	/// A task: result and error types (out of the first version's scope).
	Task(TypeId, TypeId),
	/// `Subscription`: what `let subscription = watch { … }` keeps, to stop
	/// the `watch` block (advanced brief §11.3, D25). Move-only.
	Subscription,
	/// A type of `std.concurrency` (advanced brief §14.3): a thread-safe
	/// handle (`AtomicInteger`, `AtomicBoolean`, `Mutex(T)`, `Channel(T)`),
	/// the guard of a locked mutex (move-only), or `Ordering`. The type is
	/// the `T` of a mutex, a channel or a guard (`NONE` otherwise).
	Sync(SyncKind, TypeId),
	/// A type parameter in a generic body (M2).
	Param(ParamId),
	/// `Type(parameter)` of a structural parameter (M2).
	ImplicitParam(DeclKey, u16),
	/// A compile-time value given as a type argument (M2).
	Const(ConstId),
	/// A node of a canonical recursive type graph (M2).
	Rec(RecId),
}

/// The types of `std.concurrency` (advanced brief §14.3).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SyncKind {
	AtomicInteger,
	AtomicBoolean,
	/// `Mutex(T)`
	Mutex,
	/// `Channel(T)`
	Channel,
	/// What `lock()` returns: the lock is held while it lives.
	Guard,
	/// The memory orderings of atomics.
	Ordering,
	/// The handle of a worker (`*.worker.fa`): its type is the worker's
	/// exported type. Move-only; only its methods can be called.
	Worker,
	/// `std.net.TcpListener`: a listening socket (a resource: move-only,
	/// closed when dropped or by `close()`).
	TcpListener,
	/// `std.net.TcpStream`: a connected socket (a resource, like a listener).
	TcpStream,
	/// `WorkerPool(T)`: workers of the worker type `T` (its state type),
	/// started with copies of one state. Move-only.
	WorkerPool,
}

impl SyncKind {
	/// The name of the type, as written.
	pub fn name(self) -> &'static str {
		match self {
			SyncKind::AtomicInteger => "AtomicInteger",
			SyncKind::AtomicBoolean => "AtomicBoolean",
			SyncKind::Mutex => "Mutex",
			SyncKind::Channel => "Channel",
			SyncKind::Guard => "Guard",
			SyncKind::Ordering => "Ordering",
			SyncKind::Worker => "Worker",
			SyncKind::TcpListener => "TcpListener",
			SyncKind::TcpStream => "TcpStream",
			SyncKind::WorkerPool => "WorkerPool",
		}
	}

	/// The kind named `name` (the names of `std.concurrency`).
	pub fn from_name(name: &str) -> Option<SyncKind> {
		Some(match name {
			"AtomicInteger" => SyncKind::AtomicInteger,
			"AtomicBoolean" => SyncKind::AtomicBoolean,
			"Mutex" => SyncKind::Mutex,
			"Channel" => SyncKind::Channel,
			"Ordering" => SyncKind::Ordering,
			"WorkerPool" => SyncKind::WorkerPool,
			_ => return None,
		})
	}

	/// Whether it takes a type argument (`Mutex(T)`, `Channel(T)`).
	pub fn is_generic(self) -> bool {
		matches!(
			self,
			SyncKind::Mutex | SyncKind::Channel | SyncKind::Guard | SyncKind::WorkerPool
		)
	}

	/// Whether its values are thread-safe handles: copies share the value.
	pub fn is_handle(self) -> bool {
		matches!(
			self,
			SyncKind::AtomicInteger | SyncKind::AtomicBoolean | SyncKind::Mutex | SyncKind::Channel
		)
	}

	/// Whether it is a resource of `std.net` (a socket): move-only, closed
	/// when dropped.
	pub fn is_socket(self) -> bool {
		matches!(self, SyncKind::TcpListener | SyncKind::TcpStream)
	}

	pub(crate) fn stable_index(self) -> u32 {
		self as u32
	}
}

/// The width of a float type.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub enum FloatBits {
	/// `Float32`.
	F32,
	/// `Float` (`Float64`).
	F64,
}

/// The collection constructors.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum CollKind {
	Array,
	GrowingArray,
	Set,
	GrowingSet,
	Map,
	GrowingMap,
	Pool,
	GrowingPool,
	Columnar,
	GrowingColumnar,
	OrderedColumnar,
	/// The pool of a relation (M3).
	RelationPool(u16),
}

impl CollKind {
	/// The name of the constructor, as written.
	pub fn name(self) -> &'static str {
		match self {
			CollKind::Array => "Array",
			CollKind::GrowingArray => "GrowingArray",
			CollKind::Set => "Set",
			CollKind::GrowingSet => "GrowingSet",
			CollKind::Map => "Map",
			CollKind::GrowingMap => "GrowingMap",
			CollKind::Pool | CollKind::RelationPool(_) => "Pool",
			CollKind::GrowingPool => "GrowingPool",
			CollKind::Columnar => "Columnar",
			CollKind::GrowingColumnar => "GrowingColumnar",
			CollKind::OrderedColumnar => "OrderedColumnar",
		}
	}

	/// A number identifying the constructor, stable across sessions.
	pub fn stable_index(self) -> u32 {
		match self {
			CollKind::Array => 0,
			CollKind::GrowingArray => 1,
			CollKind::Set => 2,
			CollKind::GrowingSet => 3,
			CollKind::Map => 4,
			CollKind::GrowingMap => 5,
			CollKind::Pool => 6,
			CollKind::GrowingPool => 7,
			CollKind::Columnar => 8,
			CollKind::GrowingColumnar => 9,
			CollKind::OrderedColumnar => 10,
			CollKind::RelationPool(relation) => 100 + relation as u32,
		}
	}

	/// The runtime kind of its values, for union discrimination (types brief §3.4.2).
	pub fn runtime_kind(self) -> RuntimeKind {
		match self {
			CollKind::Array | CollKind::GrowingArray | CollKind::OrderedColumnar => {
				RuntimeKind::Array
			}
			CollKind::Set | CollKind::GrowingSet => RuntimeKind::Set,
			CollKind::Map | CollKind::GrowingMap => RuntimeKind::Map,
			CollKind::Pool | CollKind::GrowingPool | CollKind::RelationPool(_) => RuntimeKind::Pool,
			CollKind::Columnar | CollKind::GrowingColumnar => RuntimeKind::Columnar,
		}
	}
}

/// The runtime kind of a value, which decides whether union members can be
/// told apart (types brief §3.4.2). Stored in 4 bits of [`TypeFlags`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum RuntimeKind {
	/// No runtime value of its own (`Never`, `Void`, `Unknown`, parameters, unions).
	Other = 0,
	Number = 1,
	BigInt = 2,
	String = 3,
	Boolean = 4,
	None = 5,
	Function = 6,
	Range = 7,
	Task = 8,
	/// Arrays, growing arrays, ordered columnars and tuples (all JavaScript arrays).
	Array = 9,
	Set = 10,
	Map = 11,
	Pool = 12,
	Columnar = 13,
	/// Objects, named unions, C structures.
	Object = 14,
}

impl RuntimeKind {
	fn from_bits(bits: u32) -> Self {
		match bits {
			1 => RuntimeKind::Number,
			2 => RuntimeKind::BigInt,
			3 => RuntimeKind::String,
			4 => RuntimeKind::Boolean,
			5 => RuntimeKind::None,
			6 => RuntimeKind::Function,
			7 => RuntimeKind::Range,
			8 => RuntimeKind::Task,
			9 => RuntimeKind::Array,
			10 => RuntimeKind::Set,
			11 => RuntimeKind::Map,
			12 => RuntimeKind::Pool,
			13 => RuntimeKind::Columnar,
			14 => RuntimeKind::Object,
			_ => RuntimeKind::Other,
		}
	}
}

bitflags! {
	/// Per-type flags computed once at interning (types brief §2.3).
	#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
	pub struct TypeFlags: u32 {
		/// Contains a `Param`, `ImplicitParam` or `SymInt`: substitution can't short-circuit.
		const HAS_PARAM = 1 << 0;
		/// Invalidation-free: no reference into it can be invalidated by a write.
		const FLAT = 1 << 1;
		/// Owns no memory: copied with `memcpy`, never dropped.
		const TRIVIALLY_COPYABLE = 1 << 2;
		/// `==` is defined.
		const COMPARABLE = 1 << 3;
		/// `<` is defined.
		const ORDERED = 1 << 4;
		/// Contains a task or a type with `destroy`: copies are moves.
		const MOVE_ONLY = 1 << 5;
		/// Contains `Void` in a data position (`TY071`).
		const HAS_VOID = 1 << 6;
		/// Part of a recursive graph.
		const RECURSIVE = 1 << 7;
		/// Represented as a JavaScript `bigint`.
		const JS_BIGINT = 1 << 8;
		/// Contains `Unknown`: diagnostics about it are suppressed (no cascades).
		const HAS_UNKNOWN = 1 << 9;
		/// Contains `Pointer` or `CString` in its public shape: only foreign
		/// files can use it (`FFI002`, decision D29).
		const HAS_FOREIGN = 1 << 10;
		/// The 4 bits of the [`RuntimeKind`].
		const KIND = 0b1111 << 16;
	}
}

impl TypeFlags {
	/// The runtime kind stored in the flags.
	pub fn runtime_kind(self) -> RuntimeKind {
		RuntimeKind::from_bits((self & TypeFlags::KIND).bits() >> 16)
	}

	/// The flags with the runtime kind replaced.
	pub fn with_runtime_kind(self, kind: RuntimeKind) -> Self {
		(self - TypeFlags::KIND) | TypeFlags::from_bits_retain((kind as u32) << 16)
	}
}

bitflags! {
	/// Flags of an object field.
	#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Ord, PartialOrd)]
	pub struct FieldFlags: u8 {
		/// `readonly`: written only at creation.
		const READONLY = 1 << 0;
		/// `reactive` (M3).
		const REACTIVE = 1 << 1;
		/// A direct or reverse relation side (M3).
		const RELATION = 1 << 2;
		/// A `#private` field: kept for the layout, ignored by structural comparison.
		const PRIVATE = 1 << 3;
		/// A method required by a structural type (`{ describe: (self) => String }`,
		/// a constraint): the field type is the method without `self`; satisfied by a
		/// function of the actual type, not by a data field (GN013).
		const METHOD = 1 << 4;
	}
}

/// A field of an object shape.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct FieldEntry {
	/// The field name.
	pub name: Sym,
	/// The field type.
	pub ty: TypeId,
	/// `readonly`, `reactive`, relation, private.
	pub flags: FieldFlags,
}

/// An object shape: public data fields sorted by name, and the nominal
/// identity of a type with `#private` fields.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Shape {
	/// Index into the field lists of the store (see [`TypeStore::shape_fields`]).
	pub fields: u32,
	/// Set for a type with `#private` fields: only that declaration satisfies it.
	pub nominal: Option<NominalId>,
}

/// What a nominal type is.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum NominalKind {
	/// A `Union { … }` type.
	Union,
	/// An `Errors { … }` type: every member satisfies `Error`.
	Errors,
	/// An enumeration.
	Enum,
	/// A C structure.
	CStruct,
	/// An object type with `#private` fields.
	Private,
	/// An object type of a foreign file that declares `destroy`: nominal
	/// and move-only (advanced brief §13.5).
	Resource,
}

/// A nominal type: its declaration, its type arguments and its member names.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct NominalInfo {
	/// The declaration.
	pub key: DeclKey,
	/// The type arguments of a generic declaration.
	pub args: ListId,
	/// Union, errors, enumeration…
	pub kind: NominalKind,
	/// The member names in declaration order (unions, errors, enumerations).
	pub members: SymListId,
}

/// The parts of a [`DeclKey`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct DeclKeyData {
	/// The module of the declaration.
	pub module: ModuleId,
	/// The names of the enclosing declarations, then the declaration's own name.
	pub names: SymListId,
	/// Tells apart declarations with the same names (local functions of different blocks).
	pub disambiguator: u16,
}

/// A parameter of a function signature.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ParamSig {
	/// The parameter type.
	pub ty: TypeId,
	/// A variadic parameter (`...values: Array(T)`).
	pub variadic: bool,
}

/// A requirement of a signature: its name and type.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Requirement {
	/// The name the body uses.
	pub name: Sym,
	/// The required type.
	pub ty: TypeId,
}

/// A function signature. Parameter names are not part of the identity.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct FnSig {
	/// The parameters.
	pub params: ParamListId,
	/// The return type.
	pub ret: TypeId,
	/// The errors it may throw: `NEVER` for none.
	pub throws: TypeId,
	/// The requirements.
	pub requires: ReqListId,
	/// `mutates` / `invalidates` (empty in M1).
	pub effects: EffectSetId,
	/// Bit `i`: the function-typed parameter `i` escapes (advanced brief §5.11).
	pub stored: u32,
	/// `async` (out of the first version's scope).
	pub is_async: bool,
	/// How many trailing parameters a function given for this type may leave
	/// out: the `index` of the callbacks of collection methods (D36), so that
	/// `names.map(identity)` is valid. A backend calling such a function value
	/// passes the arguments it declares (JavaScript ignores the extra ones).
	pub ignorable: u8,
	/// The generic parameters of a generic function type, `<T>(value: T) => T`
	/// (`Param` types, in order; empty for a function that isn't generic):
	/// bound by the type, so they are not free parameters of the types
	/// containing it. A call or a conversion to a function type that isn't
	/// generic instantiates them (decision "Generic functions carry their
	/// list on the function value", phase 2).
	pub generics: ListId,
}

/// A compile-time value.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ConstValue {
	/// `true` or `false`.
	Bool(bool),
	/// An integer.
	Int(I256),
	/// A float, by its bits (`Float32` values are stored widened).
	Float(u64),
	/// A string.
	String(Sym),
	/// A character.
	Character(Sym),
	/// A big integer, by its decimal text.
	BigInt(Sym),
	/// A member of an enumeration.
	EnumMember(NominalId, u16),
	/// An immutable local of the file being analyzed, by symbol index: a map
	/// key proven present (types brief D32), never the value of a type.
	Binding(u32),
}

/// The root of an effect path (M2).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum PathRoot {
	/// A parameter, by index.
	Param(u16),
	/// `self`.
	SelfParam,
	/// A module-level value.
	ModuleValue(DeclKey),
	/// A requirement, by index.
	Requirement(u16),
}

/// A step of an effect path (M2).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum PathStep {
	/// `.field`.
	Field(Sym),
	/// `.#field`.
	Private(Sym),
	/// `[parameter]`, by index.
	IndexParam(u16),
	/// `[literal]`.
	IndexConst(ConstId),
	/// `[parameter..]`, by index: the elements from that position on.
	IndexFrom(u16),
	/// `[root.length - 1]`: the last element.
	Last,
	/// `[..]`.
	All,
	/// `.length`.
	Length,
	/// `.size`.
	Size,
	/// `.capacity` (`reserve` of a collection whose elements never move).
	Capacity,
}

/// An effect path, interned into a [`PathId`] (M2).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Path {
	/// Where the path starts.
	pub root: PathRoot,
	/// The steps below the root.
	pub steps: StepListId,
}

/// The `mutates` and `invalidates` paths of a signature (M2).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct EffectSet {
	/// `mutates` paths.
	pub mutates: crate::ids::PathListId,
	/// `invalidates` paths.
	pub invalidates: crate::ids::PathListId,
}

/// A type parameter of a generic declaration (M2).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ParamInfo {
	/// The generic declaration.
	pub owner: DeclKey,
	/// The position among its parameters.
	pub index: u16,
	/// The name.
	pub name: Sym,
	/// The constraint (`UNKNOWN` when none); for a value parameter, the type
	/// of its value.
	pub constraint: TypeId,
	/// The default (`UNSET` when none).
	pub default: TypeId,
	/// A compile-time value parameter (`rows: Integer`): bound to the literal
	/// type of its value (`Matrix(Float, 4, 4)` binds `rows` to `4`).
	pub value: bool,
}

/// A reference to a recursive `type` declaration from inside its own type
/// (types brief §3.6): `TypeKind::Rec`. The edge it sits on is an
/// indirection (a union member, an optional, a collection element): the
/// native backend boxes it. `target` is the declared type it stands for.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RecInfo {
	/// The declaration.
	pub key: DeclKey,
	/// Its name, for display.
	pub name: Sym,
	/// The type arguments of a generic declaration (its own parameters
	/// inside the declaration).
	pub args: ListId,
	/// The type it stands for (`UNSET` while the declaration is being built).
	pub target: TypeId,
	/// Whether the declaration is an object type (its runtime kind).
	pub object: bool,
}

/// A type as written: name and arguments, for messages, hover and method resolution.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Spelling {
	/// The name as written (`UserId`, `Point`).
	pub name: Sym,
	/// The arguments as written.
	pub args: ListId,
	/// The declaration the name resolved to.
	pub decl: Option<DeclKey>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn type_kind_fits_in_16_bytes() {
		assert!(std::mem::size_of::<TypeKind>() <= 16);
	}

	#[test]
	fn runtime_kind_round_trips_through_flags() {
		let flags = TypeFlags::COMPARABLE.with_runtime_kind(RuntimeKind::Map);
		assert_eq!(flags.runtime_kind(), RuntimeKind::Map);
		assert!(flags.contains(TypeFlags::COMPARABLE));
		assert_eq!(
			flags.with_runtime_kind(RuntimeKind::Number).runtime_kind(),
			RuntimeKind::Number
		);
		assert_eq!(TypeFlags::empty().runtime_kind(), RuntimeKind::Other);
	}
}
