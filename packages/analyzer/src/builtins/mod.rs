//! The prelude as Rust tables (decision A12): one file per area, combined
//! into one registry. A [`BuiltinId`] is an index into [`all`].
//!
//! A built-in has a path:
//!
//! - `Integer`, `console`, `arguments`: a top-level name, looked up with [`lookup`];
//! - `console.log`, `Integer.parse`: a member of a namespace or an associated
//!   function of a type, looked up with [`member`];
//! - `String#includes`, `String#length`: a method or a property of the values
//!   of a built-in type, looked up with [`instance_member`]. `Character` values
//!   share the members of `String`.
//!
//! Backends recognize a built-in by its path (`Resolution::Builtin(id)` on the
//! name or member node, then [`get`]`(id).path`). Types are built in the
//! session's store on demand: use [`denoted_type`] and [`value_type`] rather
//! than matching on [`BuiltinKind`], which grows with the prelude.

pub mod array;
pub mod console;
pub mod effects;
pub mod errors;
pub mod numbers;
pub mod program;
pub mod ranges;
pub mod reactivity;
pub mod scalars;
pub mod strings;

use std::sync::OnceLock;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BuiltinId, TypeId};
use crate::types::{FnSig, ParamSig, TypeStore};

/// What a built-in is.
#[derive(Copy, Clone, Debug)]
pub enum BuiltinKind {
	/// A predefined type name (`Integer`): the type it denotes.
	Type(TypeId),
	/// A type built in the session's store (the built-in errors).
	BuiltType(fn(&mut TypeStore) -> TypeId),
	/// A namespace of members (`console`): not a value by itself.
	Namespace,
	/// A value: its type, built in the session's store.
	Value(fn(&mut TypeStore) -> TypeId),
	/// A function, or a method of a built-in type (`String#includes`): its
	/// signature with parameter names and defaults. A method's type is its
	/// bound type, without the receiver.
	Function(&'static Signature),
	/// A property of the values of a built-in type (`String#length`).
	Property(Ty),
}

/// A built-in of the prelude.
#[derive(Copy, Clone, Debug)]
pub struct Builtin {
	/// The path: `console.log`, `String#includes`.
	pub path: &'static str,
	/// What it is.
	pub kind: BuiltinKind,
}

impl Builtin {
	/// The last segment of the path: `log`, `includes`.
	pub fn name(&self) -> &'static str {
		self.path.rsplit(['.', '#']).next().unwrap_or(self.path)
	}

	/// The signature of a function or method.
	pub fn signature(&self) -> Option<&'static Signature> {
		match self.kind {
			BuiltinKind::Function(signature) => Some(signature),
			_ => None,
		}
	}
}

/// A type in a built-in signature, built in the session's store.
#[derive(Copy, Clone, Debug)]
pub enum Ty {
	/// A predefined type.
	Is(TypeId),
	/// `Integer(min = 0)`: lengths, counts.
	Natural,
	/// `T?`
	Optional(&'static Ty),
	/// `Array(T)`
	Array(&'static Ty),
	/// A built-in error type, by name (`ParseError`).
	Error(&'static str),
	/// Any value with a `toString` (printing): `UNKNOWN` for the walk, which
	/// accepts everything; the printable check is done by the caller.
	Printable,
	/// The type of the receiver of a method of number values (the base type
	/// of an integer): see [`member_type`]. `UNKNOWN` without a receiver.
	Receiver,
}

impl Ty {
	/// The type as written, without building it (hover, completions).
	pub fn describe(self, types: &TypeStore) -> String {
		match self {
			Ty::Is(ty) => types.display(ty),
			Ty::Natural => "Integer(min = 0)".to_string(),
			Ty::Optional(item) => format!("{}?", item.describe(types)),
			Ty::Array(item) => format!("Array({})", item.describe(types)),
			Ty::Error(name) => name.to_string(),
			Ty::Printable => "Printable".to_string(),
			Ty::Receiver => "Self".to_string(),
		}
	}

	/// The type in the session's store.
	pub fn build(self, types: &mut TypeStore) -> TypeId {
		self.build_for(types, TypeId::UNKNOWN)
	}

	/// The type in the session's store, [`Ty::Receiver`] being `receiver`.
	pub fn build_for(self, types: &mut TypeStore, receiver: TypeId) -> TypeId {
		match self {
			Ty::Receiver => receiver,
			Ty::Is(ty) => ty,
			Ty::Natural => {
				let storage = crate::types::IntStorage::Default;
				types.int_range(ethnum::I256::ZERO, storage.range().hi, storage)
			}
			Ty::Optional(item) => {
				let item = item.build_for(types, receiver);
				types.optional(item)
			}
			Ty::Array(item) => {
				let item = item.build_for(types, receiver);
				types.array(item)
			}
			Ty::Error(name) => errors::error_type(types, name).unwrap_or(TypeId::UNKNOWN),
			Ty::Printable => TypeId::UNKNOWN,
		}
	}
}

/// A parameter of a built-in function.
#[derive(Copy, Clone, Debug)]
pub struct Param {
	/// Its name, for named arguments and messages.
	pub name: &'static str,
	/// Its type.
	pub ty: Ty,
	/// Whether it has a default value (it may be omitted).
	pub default: bool,
}

/// The signature of a built-in function or method.
#[derive(Copy, Clone, Debug)]
pub struct Signature {
	/// The parameters, without the receiver of a method.
	pub params: &'static [Param],
	/// Whether the last parameter is variadic (`console.log(...values)`).
	pub variadic: bool,
	/// The return type.
	pub ret: Ty,
	/// The error type it throws, if any.
	pub throws: Option<Ty>,
}

impl Signature {
	/// The number of parameters that must be given.
	pub fn required(&self) -> usize {
		self.params
			.iter()
			.filter(|param| !param.default)
			.count()
			.saturating_sub(usize::from(self.variadic))
	}

	/// The function type, in the session's store.
	pub fn function_type(&self, types: &mut TypeStore) -> TypeId {
		self.function_type_for(types, TypeId::UNKNOWN)
	}

	/// The function type, [`Ty::Receiver`] being `receiver`.
	pub fn function_type_for(&self, types: &mut TypeStore, receiver: TypeId) -> TypeId {
		let last = self.params.len().saturating_sub(1);
		let params: smallvec::SmallVec<[ParamSig; 4]> = self
			.params
			.iter()
			.enumerate()
			.map(|(index, param)| ParamSig {
				ty: param.ty.build_for(types, receiver),
				variadic: self.variadic && index == last,
			})
			.collect();
		let ret = self.ret.build_for(types, receiver);
		let throws = self
			.throws
			.map_or(TypeId::NEVER, |ty| ty.build_for(types, receiver));
		let sig = FnSig {
			params: types.param_list(&params),
			..types.simple_sig(&[], ret, throws)
		};
		types.function(sig)
	}
}

struct Registry {
	all: Vec<Builtin>,
	by_path: FxHashMap<&'static str, BuiltinId>,
	/// The paths that have `.` members: namespaces, types with associated functions.
	with_members: FxHashSet<&'static str>,
}

fn registry() -> &'static Registry {
	static REGISTRY: OnceLock<Registry> = OnceLock::new();
	REGISTRY.get_or_init(|| {
		let all: Vec<Builtin> = [
			scalars::BUILTINS,
			array::BUILTINS,
			console::BUILTINS,
			errors::BUILTINS,
			numbers::BUILTINS,
			program::BUILTINS,
			ranges::BUILTINS,
			reactivity::BUILTINS,
			strings::BUILTINS,
		]
		.into_iter()
		.flatten()
		.copied()
		.collect();
		let by_path = all
			.iter()
			.enumerate()
			.map(|(index, builtin)| (builtin.path, BuiltinId::new(index)))
			.collect();
		let with_members = all
			.iter()
			.filter_map(|builtin| builtin.path.rsplit_once('.').map(|(parent, _)| parent))
			.collect();
		Registry {
			all,
			by_path,
			with_members,
		}
	})
}

/// Every built-in, indexed by [`BuiltinId`].
pub fn all() -> &'static [Builtin] {
	&registry().all
}

/// A built-in.
pub fn get(id: BuiltinId) -> &'static Builtin {
	&registry().all[id.index()]
}

/// The built-in with this exact path (`console.log`, `String#length`).
pub fn by_path(path: &str) -> Option<BuiltinId> {
	registry().by_path.get(path).copied()
}

/// The top-level built-in named `name`.
pub fn lookup(name: &str) -> Option<BuiltinId> {
	if name.contains(['.', '#']) {
		return None;
	}
	by_path(name)
}

/// The member `name` of the built-in namespace or type `parent`
/// (`console.log`, `Integer.parse`).
pub fn member(parent: BuiltinId, name: &str) -> Option<BuiltinId> {
	let path = format!("{}.{name}", get(parent).path);
	by_path(&path)
}

/// Whether a built-in has `.` members: a namespace (`console`) or a type
/// with associated functions (`Integer`, `String`).
pub fn has_members(id: BuiltinId) -> bool {
	registry().with_members.contains(get(id).path)
}

/// The method or property `name` of the values of a built-in type
/// (`"hello".length`, `x.sqrt`, `range.start`): `String` and `Character`
/// values, numbers (`Integer#…` for every integer type, `Float#…` for both
/// float types), ranges.
pub fn instance_member(types: &TypeStore, receiver: TypeId, name: &str) -> Option<BuiltinId> {
	use crate::types::TypeKind;
	let owner = match types.kind(receiver) {
		TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => "String",
		TypeKind::Range => "Range",
		TypeKind::Subscription => "Subscription",

		_ => numbers::number_owner(types, receiver)?.0,
	};
	by_path(&format!("{owner}#{name}"))
}

/// The type of a member of the values of a built-in type, read from a value
/// of type `receiver` ([`Ty::Receiver`] in its signature is the receiver's
/// type, the base type of an integer).
pub fn member_type(types: &mut TypeStore, id: BuiltinId, receiver: TypeId) -> Option<TypeId> {
	let receiver = numbers::number_owner(types, receiver).map_or(receiver, |(_, ty)| ty);
	match get(id).kind {
		BuiltinKind::Function(signature) => Some(signature.function_type_for(types, receiver)),
		BuiltinKind::Property(ty) => Some(ty.build_for(types, receiver)),
		_ => value_type(types, id),
	}
}

/// The type a built-in type name denotes (`Integer`, `IndexError`).
pub fn denoted_type(types: &mut TypeStore, id: BuiltinId) -> Option<TypeId> {
	match get(id).kind {
		BuiltinKind::Type(ty) => Some(ty),
		BuiltinKind::BuiltType(build) => Some(build(types)),
		_ => None,
	}
}

/// The type of a built-in value, function or property (`console.log`,
/// `arguments`, `String#length`).
pub fn value_type(types: &mut TypeStore, id: BuiltinId) -> Option<TypeId> {
	match get(id).kind {
		BuiltinKind::Value(build) => Some(build(types)),
		BuiltinKind::Function(signature) => Some(signature.function_type(types)),
		BuiltinKind::Property(ty) => Some(ty.build(types)),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::TypeKind;

	#[test]
	fn paths_are_unique_and_found() {
		let mut paths: Vec<_> = all().iter().map(|builtin| builtin.path).collect();
		paths.sort();
		let count = paths.len();
		paths.dedup();
		assert_eq!(paths.len(), count, "a built-in path is registered twice");
		let console = lookup("console").unwrap();
		let log = member(console, "log").unwrap();
		assert_eq!(get(log).name(), "log");
		assert!(lookup("console.log").is_none());
		assert!(lookup("String#length").is_none());
		assert!(matches!(
			get(lookup("Int8").unwrap()).kind,
			BuiltinKind::Type(TypeId::INT8)
		));
		assert!(has_members(console));
		assert!(has_members(lookup("Integer").unwrap()));
		assert!(has_members(lookup("Int8").unwrap()));
		assert!(!has_members(lookup("Boolean").unwrap()));
	}

	#[test]
	fn every_built_in_type_and_value_builds() {
		let mut types = TypeStore::new();
		for (index, builtin) in all().iter().enumerate() {
			let id = BuiltinId::new(index);
			let built = denoted_type(&mut types, id).or_else(|| value_type(&mut types, id));
			match builtin.kind {
				BuiltinKind::Namespace => assert!(built.is_none()),
				_ => assert!(
					built.is_some_and(|ty| !ty.is_unset() && ty != TypeId::UNKNOWN),
					"{} doesn't build",
					builtin.path
				),
			}
		}
	}

	#[test]
	fn instance_members_of_strings_and_characters() {
		let mut types = TypeStore::new();
		let literal = types.string_literal("abc");
		for receiver in [TypeId::STRING, TypeId::CHARACTER, literal] {
			let length = instance_member(&types, receiver, "length").unwrap();
			assert_eq!(get(length).path, "String#length");
		}
		assert!(instance_member(&types, TypeId::STRING, "push").is_none());
		assert!(instance_member(&types, TypeId::INTEGER, "length").is_none());
		assert!(instance_member(&types, TypeId::INTEGER, "toString").is_some());
		let includes = instance_member(&types, TypeId::STRING, "includes").unwrap();
		let ty = value_type(&mut types, includes).unwrap();
		let TypeKind::Function(sig) = types.kind(ty) else {
			panic!("not a function")
		};
		let sig = types.sig(sig);
		assert_eq!(types.params(sig.params).len(), 1);
		assert_eq!(sig.ret, TypeId::BOOLEAN);
	}

	#[test]
	fn defaults_lower_the_required_count() {
		let pad = get(by_path("String#padStart").unwrap())
			.signature()
			.unwrap();
		assert_eq!((pad.params.len(), pad.required()), (2, 1));
		let log = get(by_path("console.log").unwrap()).signature().unwrap();
		assert_eq!(log.required(), 0);
	}
}
