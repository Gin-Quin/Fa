//! The scalar type names of the prelude.

use crate::builtins::Builtin;
use crate::builtins::BuiltinKind::Type;
use crate::ids::TypeId;

macro_rules! types {
	($($name:literal => $ty:ident),* $(,)?) => {
		/// The scalar type names.
		pub static BUILTINS: &[Builtin] = &[$(Builtin { path: $name, kind: Type(TypeId::$ty) }),*];
	};
}

types! {
	"Never" => NEVER,
	"Void" => VOID,
	"Boolean" => BOOLEAN,
	"Integer" => INTEGER,
	"Int8" => INT8,
	"Int16" => INT16,
	"Int32" => INT32,
	"Int64" => INT64,
	"Int128" => INT128,
	"UInt8" => UINT8,
	"Byte" => BYTE,
	"UInt16" => UINT16,
	"UInt32" => UINT32,
	"UInt64" => UINT64,
	"UInt128" => UINT128,
	"Float" => FLOAT,
	"Float32" => FLOAT32,
	"BigInt" => BIG_INT,
	"Percentage" => PERCENTAGE,
	"String" => STRING,
	"Character" => CHARACTER,
	"Range" => RANGE,
}
