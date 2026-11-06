//! The built-in errors (types brief D50, values brief D-25, `design/decisions.md`).
//!
//! Each one is a prelude `Errors` type with one member of the same name,
//! declared in the module `prelude`: `IndexError` is
//! `Errors { IndexError { index: Integer, length: Integer } }`, so it is both
//! a type usable after `throws` and in `when`, and the value built by
//! `IndexError.IndexError { … }` (the language only builds them implicitly).
//! The payload of each member is [`payload`].

use crate::builtins::Builtin;
use crate::builtins::BuiltinKind::BuiltType;
use crate::ids::{ListId, TypeId};
use crate::types::{FieldEntry, FieldFlags, NominalInfo, NominalKind, TypeStore};

/// The module of the prelude's declarations (their `DeclKey`s).
pub const PRELUDE_MODULE: &str = "prelude";

/// A built-in error: its name and the fields of its payload.
pub struct BuiltinError {
	/// The name of the type and of its only member.
	pub name: &'static str,
	/// The payload fields, in declaration order (empty: a bare member).
	pub fields: &'static [(&'static str, TypeId)],
}

/// Every built-in error.
pub static ERRORS: &[BuiltinError] = &[
	// an index or a slice out of bounds
	BuiltinError {
		name: "IndexError",
		fields: &[("index", TypeId::INTEGER), ("length", TypeId::INTEGER)],
	},
	// a map key that is not present (the key's `toString()`)
	BuiltinError {
		name: "KeyNotFound",
		fields: &[("key", TypeId::STRING)],
	},
	// `//` or `%%` by zero
	BuiltinError {
		name: "DivisionByZero",
		fields: &[],
	},
	// a conversion or an operation outside the range of its type
	BuiltinError {
		name: "ArithmeticError",
		fields: &[("message", TypeId::STRING)],
	},
	// `Integer.parse` and the other parsing functions
	BuiltinError {
		name: "ParseError",
		fields: &[("input", TypeId::STRING), ("message", TypeId::STRING)],
	},
	// a cancelled task
	BuiltinError {
		name: "Cancelled",
		fields: &[],
	},
	// `send` on a closed channel (`std.concurrency`)
	BuiltinError {
		name: "ChannelClosed",
		fields: &[],
	},
	// a call of a worker pool that took longer than its `timeout` (`std.concurrency`)
	BuiltinError {
		name: "WorkerTimeout",
		fields: &[],
	},
	// `File.read` (`std.fs`): a missing or unreadable file
	BuiltinError {
		name: "FileError",
		fields: &[("path", TypeId::STRING), ("message", TypeId::STRING)],
	},
	// `Json.parse` (`std.json`): malformed JSON, or a value of another type
	BuiltinError {
		name: "JsonError",
		fields: &[("message", TypeId::STRING)],
	},
	// `std.net`: a socket that can't be opened, a connection refused or reset
	BuiltinError {
		name: "NetError",
		fields: &[("message", TypeId::STRING)],
	},
];

macro_rules! error_types {
	($($name:literal),*) => {
		/// The built-in error type names.
		pub static BUILTINS: &[Builtin] = &[$(Builtin {
			path: $name,
			kind: BuiltType(|types| error_type(types, $name).expect("a built-in error")),
		}),*, Builtin {
			path: "Error#toString",
			kind: crate::builtins::BuiltinKind::Function(&crate::builtins::Signature {
				params: &[], variadic: false,
				ret: crate::builtins::Ty::Is(TypeId::STRING), throws: None,
			}),
		}];
	};
}

error_types!(
	"IndexError",
	"KeyNotFound",
	"DivisionByZero",
	"ArithmeticError",
	"ParseError",
	"Cancelled",
	"ChannelClosed",
	"WorkerTimeout",
	"FileError",
	"JsonError",
	"NetError"
);

/// The built-in error named `name`, with its `Errors` type and member.
pub fn find(name: &str) -> Option<&'static BuiltinError> {
	ERRORS.iter().find(|error| error.name == name)
}

/// The `Errors` type of a built-in error, interned in the store (the same id
/// at every call).
pub fn error_type(types: &mut TypeStore, name: &str) -> Option<TypeId> {
	find(name)?;
	let module = types.module(PRELUDE_MODULE);
	let key = types.decl_key(module, &[name], 0);
	let member = types.syms.intern(name);
	let members = types.intern_sym_list(&[member]);
	let nominal = types.intern_nominal(NominalInfo {
		key,
		args: ListId::EMPTY,
		kind: NominalKind::Errors,
		members,
	});
	Some(types.nominal_type(nominal))
}

/// The payload of the member of a built-in error: an object type, or `None`
/// for a bare member (`DivisionByZero`).
pub fn payload(types: &mut TypeStore, name: &str) -> Option<TypeId> {
	let error = find(name)?;
	if error.fields.is_empty() {
		return None;
	}
	let fields: Vec<FieldEntry> = error
		.fields
		.iter()
		.map(|&(field, ty)| FieldEntry {
			name: types.syms.intern(field),
			ty,
			flags: FieldFlags::empty(),
		})
		.collect();
	Some(types.object(&fields, None))
}

#[cfg(test)]
mod tests {
	use super::*;

	/// `fa_runtime.c` (codegen-llvm) builds these errors with fixed ids: the
	/// position of each one in [`ERRORS`] plus one.
	#[test]
	fn builtin_error_ids_match_the_runtime() {
		let id = |name: &str| ERRORS.iter().position(|error| error.name == name).unwrap() + 1;
		assert_eq!(id("IndexError"), 1);
		assert_eq!(id("ParseError"), 5);
		assert_eq!(id("WorkerTimeout"), 8);
		assert_eq!(id("FileError"), 9);
		assert_eq!(id("NetError"), 11);
	}
	use crate::types::Mode;

	#[test]
	fn built_in_errors_are_errors() {
		let mut types = TypeStore::new();
		for error in ERRORS {
			let ty = error_type(&mut types, error.name).unwrap();
			assert_eq!(
				error_type(&mut types, error.name),
				Some(ty),
				"interned once"
			);
			assert!(types.assignable(ty, TypeId::ERROR, Mode::Copy).is_ok());
			assert_eq!(types.display(ty), error.name);
			assert_eq!(
				payload(&mut types, error.name).is_some(),
				!error.fields.is_empty()
			);
		}
		let index = error_type(&mut types, "IndexError").unwrap();
		let parse = error_type(&mut types, "ParseError").unwrap();
		assert_ne!(index, parse);
		assert!(types.assignable(index, parse, Mode::Copy).is_err());
		assert!(error_type(&mut types, "Nope").is_none());
		let payload = payload(&mut types, "ArithmeticError").unwrap();
		assert_eq!(types.display(payload), "{ message: String }");
	}

	#[test]
	fn every_error_has_a_type_name() {
		for error in ERRORS {
			assert!(BUILTINS.iter().any(|builtin| builtin.path == error.name));
		}
		assert_eq!(BUILTINS.len(), ERRORS.len());
	}
}
