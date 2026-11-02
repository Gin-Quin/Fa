//! The fields of `Range` values (values brief §12, `values/ranges.md`): a
//! range is `{ start: Integer?, end: Integer?, includesEnd: Boolean }`,
//! readable, and writable on a `mutable` range. `1..3` is
//! `{ start = 1, end = 3, includesEnd = false }`; a missing bound is `none`.

use crate::builtins::BuiltinKind::Property;
use crate::builtins::{Builtin, Ty};
use crate::ids::TypeId;

const BOUND: Ty = Ty::Optional(&Ty::Is(TypeId::INTEGER));

/// The fields of ranges.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Range#start",
		kind: Property(BOUND),
	},
	Builtin {
		path: "Range#end",
		kind: Property(BOUND),
	},
	Builtin {
		path: "Range#includesEnd",
		kind: Property(Ty::Is(TypeId::BOOLEAN)),
	},
];
