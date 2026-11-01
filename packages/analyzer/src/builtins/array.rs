//! The members of `Array` values (types brief §5.1) known in M1: `length` and
//! `push`. The other methods come with the collections (M2). The types
//! depend on the array's item type: the walk builds them (`check/objects.rs`);
//! these entries give them their identity (`Resolution::Builtin`). Also the
//! members every enumeration has (types brief §3.8): `members`, `toString`.

use crate::builtins::BuiltinKind::{Function, Property};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;

/// `push(item)`: the item type is the array's.
static PUSH: Signature = Signature {
	params: &[Param {
		name: "item",
		ty: Ty::Is(TypeId::UNKNOWN),
		default: false,
	}],
	variadic: false,
	ret: Ty::Is(TypeId::VOID),
	throws: None,
};

/// `toString()`: the name of the member.
static TO_STRING: Signature = Signature {
	params: &[],
	variadic: false,
	ret: Ty::Is(TypeId::STRING),
	throws: None,
};

/// The members of arrays and enumerations.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Enum#toString",
		kind: Function(&TO_STRING),
	},
	// `Color.members`: an `Array(Color)` in declaration order (typed by the walk)
	Builtin {
		path: "Enum.members",
		kind: Property(Ty::Array(&Ty::Is(TypeId::UNKNOWN))),
	},
	Builtin {
		path: "Array#length",
		kind: Property(Ty::Natural),
	},
	Builtin {
		path: "Array#push",
		kind: Function(&PUSH),
	},
];
