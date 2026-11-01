//! The implicit values of a program (advanced brief D4, `design/decisions.md`).

use crate::builtins::Builtin;
use crate::builtins::BuiltinKind::Value;
use crate::ids::TypeId;
use crate::types::TypeStore;

/// `arguments`: the arguments given to a standalone file or a script, after
/// the file: index `0` is the first user argument. Only standalone files see
/// it; module files will hide it when modules come (M2).
pub static BUILTINS: &[Builtin] = &[Builtin {
	path: "arguments",
	kind: Value(arguments),
}];

/// `Array(String)`
fn arguments(types: &mut TypeStore) -> TypeId {
	types.array(TypeId::STRING)
}
