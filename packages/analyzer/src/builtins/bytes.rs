//! Fixed-length binary data (`values/bytes.md`). No method mutates the length.
use crate::builtins::BuiltinKind::{Function, Property};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;

const BYTE: Ty = Ty::Is(TypeId::BYTE);
const BYTES: Ty = Ty::Is(TypeId::BYTES);
const fn method(params: &'static [Param], ret: Ty) -> Signature {
	Signature {
		params,
		variadic: false,
		ret,
		throws: None,
	}
}
const fn param(name: &'static str, ty: Ty, default: bool) -> Param {
	Param { name, ty, default }
}

pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Bytes.from",
		kind: Function(&method(&[param("items", Ty::Array(&BYTE), false)], BYTES)),
	},
	Builtin {
		path: "Bytes.filled",
		kind: Function(&method(
			&[
				param("length", Ty::Natural, false),
				param("value", BYTE, true),
			],
			BYTES,
		)),
	},
	Builtin {
		path: "Bytes#length",
		kind: Property(Ty::Natural),
	},
	Builtin {
		path: "Bytes#isEmpty",
		kind: Function(&method(&[], Ty::Is(TypeId::BOOLEAN))),
	},
	Builtin {
		path: "Bytes#get",
		kind: Function(&method(
			&[param("index", Ty::Is(TypeId::INTEGER), false)],
			Ty::Optional(&BYTE),
		)),
	},
	Builtin {
		path: "Bytes#toArray",
		kind: Function(&method(&[], Ty::Array(&BYTE))),
	},
	Builtin {
		path: "Bytes#toString",
		kind: Function(&method(&[], Ty::Is(TypeId::STRING))),
	},
];
