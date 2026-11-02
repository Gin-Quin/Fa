//! The modules of `std` (advanced brief §22.2, D9): `std.concurrency`,
//! `std.time`, `std.fs` and `std.json`. Their names are **not** in the
//! prelude: a file sees them through `use { Mutex } = std.concurrency`, or
//! names them in full (`std.time.now()`, `std.concurrency.AtomicInteger`).
//! `std.c` is used through an alias (`use c = std.c`, `check/foreign.rs`).
//!
//! A member keeps its registry path (`Mutex`, `Mutex#lock`, `File.read`),
//! which the backends match on; [`gated`] tells the name lookup of the
//! prelude to skip it, and [`member`] finds it from its module.
//!
//! - `std.time.now(): Integer`: the milliseconds since the Unix epoch;
//!   impure (never at compile time, never in a derived value).
//! - `std.fs.File.read(path): String throws FileError`: the whole file as
//!   UTF-8; blocking; impure; not on `web` ([`missing_on`]).
//! - `std.json.Json.parse(text): T throws JsonError`: `T` is the expected
//!   type (`check/std_library.rs`); `Json.stringify(value): String`.

use crate::builtins::BuiltinKind::{Function, Namespace};
use crate::builtins::{Builtin, Param, Signature, Ty, by_path};
use crate::ids::{BuiltinId, TypeId};

/// A module of `std` whose names a `use` brings.
pub struct StdModule {
	/// `concurrency` in `std.concurrency`.
	pub name: &'static str,
	/// Its members, as `(name, registry path)`.
	pub members: &'static [(&'static str, &'static str)],
}

/// The modules of `std` known to the compiler (besides `std.c`).
pub static MODULES: &[StdModule] = &[
	StdModule {
		name: "concurrency",
		members: &[
			("AtomicInteger", "AtomicInteger"),
			("AtomicBoolean", "AtomicBoolean"),
			("Mutex", "Mutex"),
			("Channel", "Channel"),
			("Ordering", "Ordering"),
			("ChannelClosed", "ChannelClosed"),
		],
	},
	StdModule {
		name: "time",
		members: &[("now", "std.time.now"), ("sleep", "std.time.sleep")],
	},
	StdModule {
		name: "fs",
		members: &[("File", "File"), ("FileError", "FileError")],
	},
	StdModule {
		name: "json",
		members: &[("Json", "Json"), ("JsonError", "JsonError")],
	},
];

/// The module of `std` named `name`.
pub fn module(name: &str) -> Option<&'static StdModule> {
	MODULES.iter().find(|module| module.name == name)
}

/// The member `name` of the module `std.<module>`.
pub fn member(module_name: &str, name: &str) -> Option<BuiltinId> {
	let path = module(module_name)?
		.members
		.iter()
		.find(|(member, _)| *member == name)?
		.1;
	by_path(path)
}

/// Whether a registry path is a top-level name of a module of `std`, only
/// visible through a `use` (`Mutex`, `File`, not `Mutex#lock`).
pub fn gated(path: &str) -> bool {
	MODULES
		.iter()
		.any(|module| module.members.iter().any(|(_, member)| *member == path))
}

/// The module of `std` a registry path belongs to (`Mutex#lock` →
/// `concurrency`), for messages.
pub fn module_of(path: &str) -> Option<&'static str> {
	let owner = path.split(['#', '.']).next().unwrap_or(path);
	if let Some(rest) = path.strip_prefix("std.") {
		return rest
			.split('.')
			.next()
			.and_then(|name| module(name).map(|module| module.name));
	}
	MODULES
		.iter()
		.find(|module| module.members.iter().any(|(_, member)| *member == owner))
		.map(|module| module.name)
}

/// What an impure built-in does, for `META002` and `REA002`: printing, the
/// clock, the file system. `None` for a pure one.
pub fn impurity(path: &str) -> Option<&'static str> {
	if path.starts_with("console") {
		return Some("`console`");
	}
	match path {
		"std.time.now" => Some("the clock (`now` is not deterministic)"),
		"File.read" => Some("the file system (`File.read`; use `@readFile` at compile time)"),
		_ => None,
	}
}

/// The targets a built-in doesn't exist on (`File.read` on `web`).
pub fn missing_on(path: &str) -> &'static [&'static str] {
	match path {
		"File.read" => &["web"],
		_ => &[],
	}
}

const STRING: Ty = Ty::Is(TypeId::STRING);

/// The namespaces of `std` and the members of `std.time`, `std.fs` and `std.json`.
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
		path: "std.time",
		kind: Namespace,
	},
	Builtin {
		path: "std.fs",
		kind: Namespace,
	},
	Builtin {
		path: "std.json",
		kind: Namespace,
	},
	// the milliseconds since the Unix epoch
	Builtin {
		path: "std.time.now",
		kind: Function(&Signature {
			params: &[],
			variadic: false,
			ret: Ty::Is(TypeId::INTEGER),
			throws: None,
		}),
	},
	// waits (`design/async.md` §5): blocking on `native`, `bun` and `node`
	// only, awaited everywhere (`check/tasks.rs`)
	Builtin {
		path: "std.time.sleep",
		kind: Function(&Signature {
			params: &[Param {
				name: "milliseconds",
				ty: Ty::Is(TypeId::INTEGER),
				default: false,
			}],
			variadic: false,
			ret: Ty::Is(TypeId::VOID),
			throws: None,
		}),
	},
	Builtin {
		path: "File",
		kind: Namespace,
	},
	// the whole file, as UTF-8
	Builtin {
		path: "File.read",
		kind: Function(&Signature {
			params: &[Param {
				name: "path",
				ty: STRING,
				default: false,
			}],
			variadic: false,
			ret: STRING,
			throws: Some(Ty::Error("FileError")),
		}),
	},
	Builtin {
		path: "Json",
		kind: Namespace,
	},
	// the value of the expected type (`check/std_library.rs`)
	Builtin {
		path: "Json.parse",
		kind: Function(&Signature {
			params: &[Param {
				name: "text",
				ty: STRING,
				default: false,
			}],
			variadic: false,
			ret: Ty::Is(TypeId::UNKNOWN),
			throws: Some(Ty::Error("JsonError")),
		}),
	},
	// the JSON text of a value
	Builtin {
		path: "Json.stringify",
		kind: Function(&Signature {
			params: &[Param {
				name: "value",
				ty: Ty::Printable,
				default: false,
			}],
			variadic: false,
			ret: STRING,
			throws: None,
		}),
	},
];

#[cfg(test)]
mod tests {
	use super::*;
	use crate::builtins::{get, lookup};

	#[test]
	fn members_exist_and_are_gated() {
		for module in MODULES {
			for (name, path) in module.members {
				let id = member(module.name, name).unwrap_or_else(|| panic!("{path}"));
				assert_eq!(get(id).path, *path);
				assert!(gated(path));
				assert!(
					lookup(name).is_none(),
					"`{name}` is visible without a `use`"
				);
			}
		}
		assert!(lookup("console").is_some());
		assert!(lookup("std").is_some());
		assert_eq!(module_of("Mutex#lock"), Some("concurrency"));
		assert_eq!(module_of("std.time.now"), Some("time"));
		assert_eq!(module_of("File.read"), Some("fs"));
		assert_eq!(module_of("console.log"), None);
		assert!(impurity("std.time.now").is_some());
		assert!(impurity("Json.parse").is_none());
	}
}
