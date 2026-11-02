//! The number API (`values/numbers.md`, `operations/arithmetic.md`, values
//! brief §6.10–§6.14, types brief §3.13):
//!
//! - associated functions: `Integer.parse`, `Float.parse`, the float to
//!   integer conversions `Integer.round(x)` / `floor` / `ceil` / `truncate`
//!   (`throws ArithmeticError`: NaN, an infinity, out of range);
//! - constants: `minValue` and `maxValue` of every integer type (their
//!   literal type holds the value), `Integer.jsMaxSafeInteger`, and
//!   `Float.maxValue`, `infinity`, `nan`, `epsilon` (also on `Float32`);
//! - the methods of the values: `Integer#…` for every integer type, `Float#…`
//!   for `Float` and `Float32`, `BigInt#…`, `Percentage#…`. [`Ty::Receiver`]
//!   in their signatures is the receiver's type (its base type for an
//!   integer): `x.sqrt()` on a `Float32` is a `Float32`;
//! - the `Math` namespace: `Math.max(a, b)` and `Math.min(a, b)`, typed by
//!   the walk (two numbers of the same kind, `check/math.rs`).

use ethnum::I256;

use crate::builtins::BuiltinKind::{Function, Namespace, Value};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;
use crate::types::{IntStorage, IntType, TypeStore};

const STRING: Ty = Ty::Is(TypeId::STRING);
const INTEGER: Ty = Ty::Is(TypeId::INTEGER);
const FLOAT: Ty = Ty::Is(TypeId::FLOAT);
const BOOLEAN: Ty = Ty::Is(TypeId::BOOLEAN);
const RECEIVER: Ty = Ty::Receiver;
const ARITHMETIC_ERROR: Ty = Ty::Error("ArithmeticError");

const fn param(name: &'static str, ty: Ty) -> Param {
	Param {
		name,
		ty,
		default: false,
	}
}

const fn function(params: &'static [Param], ret: Ty, throws: Option<Ty>) -> Signature {
	Signature {
		params,
		variadic: false,
		ret,
		throws,
	}
}

/// `Integer.parse(text: String): Integer throws ParseError`: an optional sign
/// and ASCII digits.
static PARSE_INTEGER: Signature = function(
	&[param("text", STRING)],
	INTEGER,
	Some(Ty::Error("ParseError")),
);
/// `Float.parse(text: String): Float throws ParseError`
static PARSE_FLOAT: Signature = function(
	&[param("text", STRING)],
	FLOAT,
	Some(Ty::Error("ParseError")),
);
/// `Integer.round(value: Float): Integer throws ArithmeticError` (a `Float32`
/// is accepted where a `Float` is expected).
static FLOAT_TO_INTEGER: Signature =
	function(&[param("value", FLOAT)], INTEGER, Some(ARITHMETIC_ERROR));
/// `Math.max(a, b)`: typed by the walk; the parameters give the names.
static MATH_PAIR: Signature = function(
	&[
		param("a", Ty::Is(TypeId::UNKNOWN)),
		param("b", Ty::Is(TypeId::UNKNOWN)),
	],
	Ty::Is(TypeId::UNKNOWN),
	None,
);

/// `toString()`
static TO_STRING: Signature = function(&[], STRING, None);
/// `round()`, `sqrt()`, `abs()`, `bitNot()`: the receiver's type.
static SAME: Signature = function(&[], RECEIVER, None);
/// `isNaN()`, `isFinite()`
static TEST: Signature = function(&[], BOOLEAN, None);
/// `clamp(min, max)`
static CLAMP: Signature = function(
	&[param("min", RECEIVER), param("max", RECEIVER)],
	RECEIVER,
	None,
);
/// `bitAnd(other)`, `wrappingAdd(other)`, `saturatingMul(other)`
static WITH_OTHER: Signature = function(&[param("other", RECEIVER)], RECEIVER, None);
/// `checkedAdd(other)`: `ArithmeticError` on overflow.
static CHECKED: Signature = function(
	&[param("other", RECEIVER)],
	RECEIVER,
	Some(ARITHMETIC_ERROR),
);
/// `wrappingPow(exponent)`, `saturatingPow(exponent)`: a natural exponent.
static WITH_EXPONENT: Signature = function(&[param("exponent", Ty::Natural)], RECEIVER, None);
/// `checkedPow(exponent)`
static CHECKED_POW: Signature = function(
	&[param("exponent", Ty::Natural)],
	RECEIVER,
	Some(ARITHMETIC_ERROR),
);
/// `shiftLeft(amount)`, `shiftRight(amount)`: an amount between 0 and the
/// width, excluded; not proven by the analyzer yet, so they may fail.
static SHIFT: Signature = function(
	&[param("amount", INTEGER)],
	RECEIVER,
	Some(ARITHMETIC_ERROR),
);

/// A constant of an integer type: its literal type, in the type's storage.
fn int_constant(types: &mut TypeStore, storage: IntStorage, value: I256) -> TypeId {
	types.int_type(IntType {
		min: value,
		max: value,
		storage,
	})
}

macro_rules! int_bounds {
	($($min:ident, $max:ident => $storage:ident),* $(,)?) => {
		$(
			fn $min(types: &mut TypeStore) -> TypeId {
				int_constant(types, IntStorage::$storage, IntStorage::$storage.range().lo)
			}
			fn $max(types: &mut TypeStore) -> TypeId {
				int_constant(types, IntStorage::$storage, IntStorage::$storage.range().hi)
			}
		)*
	};
}

int_bounds! {
	integer_min, integer_max => Default,
	int8_min, int8_max => I8,
	int16_min, int16_max => I16,
	int32_min, int32_max => I32,
	int64_min, int64_max => I64,
	int128_min, int128_max => I128,
	uint8_min, uint8_max => U8,
	uint16_min, uint16_max => U16,
	uint32_min, uint32_max => U32,
	uint64_min, uint64_max => U64,
	uint128_min, uint128_max => U128,
}

/// `Integer.jsMaxSafeInteger`: 2^53 − 1.
fn js_max_safe_integer(types: &mut TypeStore) -> TypeId {
	int_constant(types, IntStorage::Default, I256::from((1i64 << 53) - 1))
}

fn float(_: &mut TypeStore) -> TypeId {
	TypeId::FLOAT
}

fn float32(_: &mut TypeStore) -> TypeId {
	TypeId::FLOAT32
}

macro_rules! members {
	($($path:literal => $kind:expr),* $(,)?) => {
		/// The number API.
		pub static BUILTINS: &[Builtin] = &[$(Builtin { path: $path, kind: $kind }),*];
	};
}

members! {
	"Math" => Namespace,
	"Math.max" => Function(&MATH_PAIR),
	"Math.min" => Function(&MATH_PAIR),

	"Integer.parse" => Function(&PARSE_INTEGER),
	"Integer.round" => Function(&FLOAT_TO_INTEGER),
	"Integer.floor" => Function(&FLOAT_TO_INTEGER),
	"Integer.ceil" => Function(&FLOAT_TO_INTEGER),
	"Integer.truncate" => Function(&FLOAT_TO_INTEGER),
	"Integer.jsMaxSafeInteger" => Value(js_max_safe_integer),
	"Float.parse" => Function(&PARSE_FLOAT),

	"Integer.minValue" => Value(integer_min),
	"Integer.maxValue" => Value(integer_max),
	"Int8.minValue" => Value(int8_min),
	"Int8.maxValue" => Value(int8_max),
	"Int16.minValue" => Value(int16_min),
	"Int16.maxValue" => Value(int16_max),
	"Int32.minValue" => Value(int32_min),
	"Int32.maxValue" => Value(int32_max),
	"Int64.minValue" => Value(int64_min),
	"Int64.maxValue" => Value(int64_max),
	"Int128.minValue" => Value(int128_min),
	"Int128.maxValue" => Value(int128_max),
	"UInt8.minValue" => Value(uint8_min),
	"UInt8.maxValue" => Value(uint8_max),
	"Byte.minValue" => Value(uint8_min),
	"Byte.maxValue" => Value(uint8_max),
	"UInt16.minValue" => Value(uint16_min),
	"UInt16.maxValue" => Value(uint16_max),
	"UInt32.minValue" => Value(uint32_min),
	"UInt32.maxValue" => Value(uint32_max),
	"UInt64.minValue" => Value(uint64_min),
	"UInt64.maxValue" => Value(uint64_max),
	"UInt128.minValue" => Value(uint128_min),
	"UInt128.maxValue" => Value(uint128_max),

	"Float.maxValue" => Value(float),
	"Float.infinity" => Value(float),
	"Float.nan" => Value(float),
	"Float.epsilon" => Value(float),
	"Float32.maxValue" => Value(float32),
	"Float32.infinity" => Value(float32),
	"Float32.nan" => Value(float32),
	"Float32.epsilon" => Value(float32),

	"Integer#toString" => Function(&TO_STRING),
	"Integer#abs" => Function(&SAME),
	"Integer#clamp" => Function(&CLAMP),
	"Integer#bitAnd" => Function(&WITH_OTHER),
	"Integer#bitOr" => Function(&WITH_OTHER),
	"Integer#bitXor" => Function(&WITH_OTHER),
	"Integer#bitNot" => Function(&SAME),
	"Integer#shiftLeft" => Function(&SHIFT),
	"Integer#shiftRight" => Function(&SHIFT),
	"Integer#wrappingAdd" => Function(&WITH_OTHER),
	"Integer#wrappingSub" => Function(&WITH_OTHER),
	"Integer#wrappingMul" => Function(&WITH_OTHER),
	"Integer#wrappingPow" => Function(&WITH_EXPONENT),
	"Integer#saturatingAdd" => Function(&WITH_OTHER),
	"Integer#saturatingSub" => Function(&WITH_OTHER),
	"Integer#saturatingMul" => Function(&WITH_OTHER),
	"Integer#saturatingPow" => Function(&WITH_EXPONENT),
	"Integer#checkedAdd" => Function(&CHECKED),
	"Integer#checkedSub" => Function(&CHECKED),
	"Integer#checkedMul" => Function(&CHECKED),
	"Integer#checkedPow" => Function(&CHECKED_POW),

	"Float#toString" => Function(&TO_STRING),
	"Float#abs" => Function(&SAME),
	"Float#round" => Function(&SAME),
	"Float#floor" => Function(&SAME),
	"Float#ceil" => Function(&SAME),
	"Float#truncate" => Function(&SAME),
	"Float#sqrt" => Function(&SAME),
	"Float#clamp" => Function(&CLAMP),
	"Float#isNaN" => Function(&TEST),
	"Float#isFinite" => Function(&TEST),

	"BigInt#toString" => Function(&TO_STRING),
	"BigInt#abs" => Function(&SAME),

	"Percentage#toString" => Function(&TO_STRING),
	"Percentage#abs" => Function(&SAME),
}

/// The owner of the members of a number type's values (`Integer` for every
/// integer type, `Float` for `Float` and `Float32`), and the type
/// [`Ty::Receiver`] stands for: the base type of an integer, the widened
/// type otherwise.
pub fn number_owner(types: &TypeStore, ty: TypeId) -> Option<(&'static str, TypeId)> {
	use crate::types::TypeKind;
	match types.kind(ty) {
		TypeKind::Int(id) => Some(("Integer", TypeId::of_storage(types.int(id).storage))),
		TypeKind::Float(_) | TypeKind::FloatLit(..) => Some(("Float", types.widen(ty))),
		TypeKind::BigInt | TypeKind::BigIntLit(_) => Some(("BigInt", TypeId::BIG_INT)),
		TypeKind::Percentage => Some(("Percentage", TypeId::PERCENTAGE)),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::builtins::{by_path, get, member_type, value_type};
	use crate::types::TypeKind;

	#[test]
	fn integer_bounds_are_literals_of_their_storage() {
		let mut types = TypeStore::new();
		let max = value_type(&mut types, by_path("UInt64.maxValue").unwrap()).unwrap();
		let int = *types.as_int(max).unwrap();
		assert_eq!(int.storage, IntStorage::U64);
		assert_eq!(int.min, int.max);
		assert_eq!(int.max, IntStorage::U64.range().hi);
		let min = value_type(&mut types, by_path("Int8.minValue").unwrap()).unwrap();
		assert_eq!(types.as_int(min).unwrap().min, I256::from(-128));
		let safe = value_type(&mut types, by_path("Integer.jsMaxSafeInteger").unwrap()).unwrap();
		assert_eq!(
			types.as_int(safe).unwrap().min,
			I256::from(9_007_199_254_740_991i64)
		);
	}

	#[test]
	fn methods_take_the_receiver_type() {
		let mut types = TypeStore::new();
		let sqrt = by_path("Float#sqrt").unwrap();
		let ty = member_type(&mut types, sqrt, TypeId::FLOAT32).unwrap();
		let TypeKind::Function(sig) = types.kind(ty) else {
			panic!("not a function")
		};
		assert_eq!(types.sig(sig).ret, TypeId::FLOAT32);
		let clamp = by_path("Integer#clamp").unwrap();
		let ty = member_type(&mut types, clamp, TypeId::INT8).unwrap();
		let TypeKind::Function(sig) = types.kind(ty) else {
			panic!("not a function")
		};
		let sig = types.sig(sig);
		assert_eq!(sig.ret, TypeId::INT8);
		assert_eq!(types.params(sig.params)[0].ty, TypeId::INT8);
		assert_eq!(get(by_path("Integer.round").unwrap()).name(), "round");
	}

	#[test]
	fn owners_of_number_values() {
		let mut types = TypeStore::new();
		let ranged = types.int_range(I256::ZERO, I256::from(9), IntStorage::I16);
		assert_eq!(
			number_owner(&types, ranged),
			Some(("Integer", TypeId::INT16))
		);
		let literal = types.float_literal(crate::types::FloatBits::F64, 2.5);
		assert_eq!(
			number_owner(&types, literal),
			Some(("Float", TypeId::FLOAT))
		);
		assert_eq!(number_owner(&types, TypeId::STRING), None);
	}
}
