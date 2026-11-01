//! `console`: printing.

use crate::builtins::Builtin;
use crate::builtins::BuiltinKind::{Namespace, Value};
use crate::ids::TypeId;
use crate::types::{FnSig, ParamSig, TypeStore};

/// `console` and its members.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "console",
		kind: Namespace,
	},
	Builtin {
		path: "console.log",
		kind: Value(print),
	},
	Builtin {
		path: "console.error",
		kind: Value(print),
	},
];

/// `(...values) => Void`: any printable value (`UNKNOWN` accepts everything).
fn print(types: &mut TypeStore) -> TypeId {
	let params = types.param_list(&[ParamSig {
		ty: TypeId::UNKNOWN,
		variadic: true,
	}]);
	let sig = FnSig {
		params,
		..types.simple_sig(&[], TypeId::VOID, TypeId::NEVER)
	};
	types.function(sig)
}
