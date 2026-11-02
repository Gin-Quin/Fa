//! `std.concurrency` without async (advanced brief §14.3, D31): the
//! thread-safe types `AtomicInteger`, `AtomicBoolean`, `Mutex(T)`,
//! `Channel(T)`, the guard of a locked mutex and `Ordering`.
//!
//! Their values are `TypeKind::Sync`. Their methods don't declare `mutates
//! self` (they modify the shared value from an immutable reference), so
//! they produce no effect. `Mutex#lock`, `Channel#send` and
//! `Channel#receive` wait: they can only be called blocking on `native`
//! (`CONC010` elsewhere, see [`waits`]).
//!
//! The names are visible like the prelude's; `use { Mutex } =
//! std.concurrency` is checked (its members exist) and names nothing more
//! (see `check/concurrency.rs`).

use crate::builtins::BuiltinKind::{BuiltType, Function, Namespace, Property, Value};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;
use crate::types::{SyncKind, TypeKind, TypeStore};

const INTEGER: Ty = Ty::Is(TypeId::INTEGER);
const BOOLEAN: Ty = Ty::Is(TypeId::BOOLEAN);
const VOID: Ty = Ty::Is(TypeId::VOID);
const ITEM: Ty = Ty::SyncItem;

const ORDERING: Param = Param {
	name: "ordering",
	ty: Ty::Ordering,
	default: true,
};

const fn param(name: &'static str, ty: Ty) -> Param {
	Param {
		name,
		ty,
		default: false,
	}
}

macro_rules! method {
	($params:expr, $ret:expr $(,)?) => {
		Function(&Signature {
			params: $params,
			variadic: false,
			ret: $ret,
			throws: None,
		})
	};
}

/// The members of `std.concurrency`.
pub const MEMBERS: &[&str] = &[
	"AtomicInteger",
	"AtomicBoolean",
	"Mutex",
	"Channel",
	"Ordering",
	"ChannelClosed",
];

/// The types, their methods, and the orderings.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "std",
		kind: Namespace,
	},
	Builtin {
		path: "std.concurrency",
		kind: Namespace,
	},
	Builtin {
		path: "AtomicInteger",
		kind: BuiltType(atomic_integer),
	},
	Builtin {
		path: "AtomicBoolean",
		kind: BuiltType(atomic_boolean),
	},
	Builtin {
		path: "Mutex",
		kind: BuiltType(|types| sync(types, SyncKind::Mutex, TypeId::UNKNOWN)),
	},
	Builtin {
		path: "Channel",
		kind: BuiltType(|types| sync(types, SyncKind::Channel, TypeId::UNKNOWN)),
	},
	Builtin {
		path: "Ordering",
		kind: BuiltType(ordering),
	},
	Builtin {
		path: "Ordering.relaxed",
		kind: Value(ordering),
	},
	Builtin {
		path: "Ordering.acquire",
		kind: Value(ordering),
	},
	Builtin {
		path: "Ordering.release",
		kind: Value(ordering),
	},
	Builtin {
		path: "Ordering.acquireRelease",
		kind: Value(ordering),
	},
	Builtin {
		path: "Ordering.sequential",
		kind: Value(ordering),
	},
	// atomics: every method returns the new value (or the previous one for `swap`)
	Builtin {
		path: "AtomicInteger#load",
		kind: method!(&[ORDERING], INTEGER),
	},
	Builtin {
		path: "AtomicInteger#store",
		kind: method!(&[param("value", INTEGER), ORDERING], VOID),
	},
	Builtin {
		path: "AtomicInteger#increment",
		kind: method!(&[ORDERING], INTEGER),
	},
	Builtin {
		path: "AtomicInteger#decrement",
		kind: method!(&[ORDERING], INTEGER),
	},
	Builtin {
		path: "AtomicInteger#add",
		kind: method!(&[param("amount", INTEGER), ORDERING], INTEGER),
	},
	Builtin {
		path: "AtomicInteger#swap",
		kind: method!(&[param("value", INTEGER), ORDERING], INTEGER),
	},
	Builtin {
		path: "AtomicInteger#compareAndSwap",
		kind: method!(
			&[
				param("expected", INTEGER),
				param("value", INTEGER),
				ORDERING,
			],
			BOOLEAN,
		),
	},
	Builtin {
		path: "AtomicBoolean#load",
		kind: method!(&[ORDERING], BOOLEAN),
	},
	Builtin {
		path: "AtomicBoolean#store",
		kind: method!(&[param("value", BOOLEAN), ORDERING], VOID),
	},
	Builtin {
		path: "AtomicBoolean#swap",
		kind: method!(&[param("value", BOOLEAN), ORDERING], BOOLEAN),
	},
	Builtin {
		path: "AtomicBoolean#compareAndSwap",
		kind: method!(
			&[
				param("expected", BOOLEAN),
				param("value", BOOLEAN),
				ORDERING,
			],
			BOOLEAN,
		),
	},
	// a mutex and its guard
	Builtin {
		path: "Mutex#lock",
		kind: method!(&[], Ty::Guard),
	},
	Builtin {
		path: "Mutex#tryLock",
		kind: method!(&[], Ty::Optional(&Ty::Guard)),
	},
	Builtin {
		path: "Guard#value",
		kind: Property(ITEM),
	},
	// a channel
	Builtin {
		path: "Channel#send",
		kind: Function(&Signature {
			params: &[param("value", ITEM)],
			variadic: false,
			ret: VOID,
			throws: Some(Ty::Error("ChannelClosed")),
		}),
	},
	Builtin {
		path: "Channel#receive",
		kind: method!(&[], Ty::Optional(&ITEM)),
	},
	Builtin {
		path: "Channel#trySend",
		kind: method!(&[param("value", ITEM)], BOOLEAN),
	},
	Builtin {
		path: "Channel#tryReceive",
		kind: method!(&[], Ty::Optional(&ITEM)),
	},
	Builtin {
		path: "Channel#close",
		kind: method!(&[], VOID),
	},
	// a worker: `stop()` consumes the handle
	Builtin {
		path: "Worker#stop",
		kind: method!(&[], VOID),
	},
];

/// A type of `std.concurrency`.
pub fn sync(types: &mut TypeStore, kind: SyncKind, item: TypeId) -> TypeId {
	types.intern(TypeKind::Sync(kind, item))
}

fn atomic_integer(types: &mut TypeStore) -> TypeId {
	sync(types, SyncKind::AtomicInteger, TypeId::NONE)
}

fn atomic_boolean(types: &mut TypeStore) -> TypeId {
	sync(types, SyncKind::AtomicBoolean, TypeId::NONE)
}

/// `Ordering`.
pub fn ordering(types: &mut TypeStore) -> TypeId {
	sync(types, SyncKind::Ordering, TypeId::NONE)
}

/// Whether a built-in method waits (`lock`, `send`, `receive`): it can only
/// be called blocking on `native` (D31, `CONC010` elsewhere).
pub fn waits(path: &str) -> bool {
	matches!(path, "Mutex#lock" | "Channel#send" | "Channel#receive")
}

/// The fields of the literal building a thread-safe value: their names,
/// types (given the `T` of a mutex or a channel) and whether they may be
/// omitted (advanced brief §14.3: `Channel(T) { capacity = 16 }`).
pub fn literal_fields(kind: SyncKind, item: TypeId) -> &'static [(&'static str, bool)] {
	let _ = item;
	match kind {
		SyncKind::AtomicInteger | SyncKind::AtomicBoolean | SyncKind::Mutex => &[("value", false)],
		SyncKind::Channel => &[("capacity", true)],
		SyncKind::Guard | SyncKind::Ordering | SyncKind::Worker => &[],
	}
}
