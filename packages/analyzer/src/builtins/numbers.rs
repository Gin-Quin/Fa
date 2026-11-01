//! Associated functions of the number types: parsing (`design/decisions.md`,
//! "Standard library"; types brief §3.13).

use crate::builtins::BuiltinKind::Function;
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;

/// `Integer.parse(text: String): Integer throws ParseError`: an optional sign
/// and ASCII digits.
static PARSE_INTEGER: Signature = Signature {
	params: &[Param {
		name: "text",
		ty: Ty::Is(TypeId::STRING),
		default: false,
	}],
	variadic: false,
	ret: Ty::Is(TypeId::INTEGER),
	throws: Some(Ty::Error("ParseError")),
};

/// The associated functions of the number types.
pub static BUILTINS: &[Builtin] = &[Builtin {
	path: "Integer.parse",
	kind: Function(&PARSE_INTEGER),
}];
