//! The implicit values of a program (advanced brief D4, `design/decisions.md`).

use crate::builtins::Builtin;
use crate::builtins::BuiltinKind::{BuiltType, Value};
use crate::ids::TypeId;
use crate::types::TypeStore;

/// `arguments`: the arguments given to a standalone file or a script, after
/// the file: index `0` is the first user argument. Only standalone files see
/// it; module files will hide it when modules come (M2).
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "arguments",
		kind: Value(arguments),
	},
	// the targets (`architecture/configuration`): the type of `target`
	Builtin {
		path: "Target",
		kind: BuiltType(target),
	},
];

/// `"web" | "bun" | "node" | "deno" | "native" | "wasm" | "rust"`
fn target(types: &mut TypeStore) -> TypeId {
	let members: Vec<TypeId> = crate::comptime::TARGETS
		.iter()
		.map(|target| types.string_literal(target))
		.collect();
	types.union(&members)
}

/// `Array(String)`
fn arguments(types: &mut TypeStore) -> TypeId {
	types.array(TypeId::STRING)
}
