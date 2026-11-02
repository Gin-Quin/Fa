//! The methods of `Pointer(T)` and `CString` (foreign code, advanced brief
//! §13.4). What a pointer designates is not checked: these only exist in
//! foreign files.

use crate::builtins::BuiltinKind::{Function, Property};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;

/// The members of pointers and C strings.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Pointer#value",
		kind: Property(Ty::Target),
	},
	Builtin {
		path: "Pointer#read",
		kind: Function(&Signature {
			params: &[],
			variadic: false,
			ret: Ty::Target,
			throws: None,
		}),
	},
	Builtin {
		path: "Pointer#write",
		kind: Function(&Signature {
			params: &[Param {
				name: "value",
				ty: Ty::Target,
				default: false,
			}],
			variadic: false,
			ret: Ty::Is(TypeId::VOID),
			throws: None,
		}),
	},
	Builtin {
		path: "Pointer#offset",
		kind: Function(&Signature {
			params: &[Param {
				name: "count",
				ty: Ty::Is(TypeId::INTEGER),
				default: false,
			}],
			variadic: false,
			ret: Ty::Receiver,
			throws: None,
		}),
	},
	Builtin {
		path: "Pointer#toArray",
		kind: Function(&Signature {
			params: &[Param {
				name: "count",
				ty: Ty::Natural,
				default: false,
			}],
			variadic: false,
			ret: Ty::Array(&Ty::Target),
			throws: None,
		}),
	},
	Builtin {
		path: "CString#toString",
		kind: Function(&Signature {
			params: &[],
			variadic: false,
			ret: Ty::Is(TypeId::STRING),
			throws: None,
		}),
	},
];
