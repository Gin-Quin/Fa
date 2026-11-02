//! `Subscription` (advanced brief §11.3, D25): the value of a `watch` block
//! used as an expression (`let subscription = watch { … }`). It can't be
//! copied, only moved, and `stop()` consumes it: it runs the cleanup and
//! stops the block (`MEM002` on a later use).

use crate::builtins::BuiltinKind::{BuiltType, Function};
use crate::builtins::{Builtin, Signature, Ty};
use crate::ids::TypeId;
use crate::types::{TypeKind, TypeStore};

/// `Subscription` and its method.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Subscription",
		kind: BuiltType(subscription),
	},
	Builtin {
		path: "Subscription#stop",
		kind: Function(&Signature {
			params: &[],
			variadic: false,
			ret: Ty::Is(TypeId::VOID),
			throws: None,
		}),
	},
];

/// The `Subscription` type.
pub fn subscription(types: &mut TypeStore) -> TypeId {
	types.intern(TypeKind::Subscription)
}
