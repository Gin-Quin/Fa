//! `console`: printing (values brief §1, §2.8). Every argument goes through
//! `toString`: the walk checks that each one has it (`E-STR-11`, `E-STR-12`).

use crate::builtins::BuiltinKind::{Function, Namespace};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;

/// `(...values) => Void`: the values printed separated by a space, then a newline.
static PRINT: Signature = Signature {
	params: &[Param {
		name: "values",
		ty: Ty::Printable,
		default: false,
	}],
	variadic: true,
	ret: Ty::Is(TypeId::VOID),
	throws: None,
};

/// `console` and its members.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "console",
		kind: Namespace,
	},
	// to the standard output
	Builtin {
		path: "console.log",
		kind: Function(&PRINT),
	},
	// to the standard error
	Builtin {
		path: "console.error",
		kind: Function(&PRINT),
	},
];

/// Whether a built-in prints its arguments (`console.log`, `console.error`).
pub fn prints(path: &str) -> bool {
	matches!(path, "console.log" | "console.error")
}
