//! A package built as a C library (`build.cLibrary`, foreign-code.md,
//! "Exposing a library to C"): the functions of the namespace exported by
//! the API file, the C types allowed at the boundary, and the checks of the
//! API (`FFI012`, `FFI013`, `FFI014`, `REQ006`). The backend
//! (`fa-codegen-llvm`) reads the same functions to generate the header and
//! the exported wrappers.
//!
//! - Every function of the exported namespace becomes the C function
//!   `<prefix>_<name in snake case>`, in declaration order.
//! - Its parameters and its result have C types ([`boundary_type`]):
//!   numbers of a fixed width, `Boolean`, `Pointer(T)` (`T` any type: a Fa
//!   value is an opaque `struct` for C), `Pointer(T)?`, `CString`,
//!   `CString?` and `CStruct` types by value.
//! - A function that `throws` throws the members of one `Errors` type
//!   ([`error_type`]): its status is the position of the member, from 1.

use crate::configuration::CLibraryOptions;
use crate::ids::{DeclKey, NodeId, NominalId, TypeId};
use crate::modules::Interface;
use crate::symbols::SymbolKind;
use crate::types::{FloatBits, NominalKind, TypeKind, TypeStore};

/// A function of the C interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiFunction {
	/// Its Fa name (`createFarm`).
	pub name: String,
	/// Its C name (`farm_create_farm`).
	pub symbol: String,
	/// Its declaration key.
	pub key: DeclKey,
	/// Its declaration node.
	pub decl: NodeId,
	/// Its function type.
	pub ty: TypeId,
}

/// A type at the C boundary.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoundaryType {
	/// No result.
	Void,
	/// `bool`.
	Bool,
	/// An integer of this width.
	Int { bits: u32, signed: bool },
	/// `float`.
	Float32,
	/// `double`.
	Float64,
	/// `const char *` (`CString`, `CString?`).
	CString { nullable: bool },
	/// `T *` (`Pointer(T)`, `Pointer(T)?`).
	Pointer { target: TypeId, nullable: bool },
	/// A `CStruct` by value.
	Struct(TypeId),
}

/// The `build.cLibrary` options of the merged configuration of a package
/// (the value of the static `configuration`), if it builds a C library.
pub fn configured(configuration: Option<&fa_interpreter::Value>) -> Option<CLibraryOptions> {
	configured_with_targets(configuration).map(|(options, _)| options)
}

/// [`configured`], and the `targets` of the package.
pub fn configured_with_targets(
	configuration: Option<&fa_interpreter::Value>,
) -> Option<(CLibraryOptions, Vec<String>)> {
	let value = crate::configuration::from_value_without_functions(configuration?)?;
	let configuration = crate::configuration::PackageConfiguration::from_value(&value);
	Some((configuration.build.c_library?, configuration.targets))
}

/// `createFarm` → `create_farm`, `parseHTTPRequest` → `parse_http_request`.
pub fn snake_case(name: &str) -> String {
	let characters: Vec<char> = name.chars().collect();
	let mut snake = String::new();
	for (index, &character) in characters.iter().enumerate() {
		if character.is_uppercase() {
			let previous = index.checked_sub(1).map(|at| characters[at]);
			let next = characters.get(index + 1);
			let starts_word = previous.is_some_and(|previous| {
				previous.is_lowercase()
					|| previous.is_ascii_digit()
					|| (previous.is_uppercase() && next.is_some_and(|next| next.is_lowercase()))
			});
			if starts_word && !snake.ends_with('_') {
				snake.push('_');
			}
			snake.extend(character.to_lowercase());
		} else {
			snake.push(character);
		}
	}
	snake
}

/// The functions of the namespace the API file exports, in declaration
/// order (`None` when its export isn't a namespace).
pub fn api_functions(
	types: &TypeStore,
	interface: &Interface,
	prefix: &str,
) -> Option<Vec<ApiFunction>> {
	let export = interface.export?;
	if export.kind != SymbolKind::Namespace {
		return None;
	}
	let block = export.members?;
	let mut members: Vec<(u32, ApiFunction)> = interface
		.members
		.iter()
		.filter(|((owner, _), member)| *owner == block && member.kind == SymbolKind::Function)
		.map(|((_, name), member)| {
			let name = types.syms.resolve(*name).to_string();
			let start = interface
				.keys
				.get(&member.key)
				.map_or(member.decl.0, |keyed| keyed.span.start);
			let function = ApiFunction {
				symbol: format!("{prefix}_{}", snake_case(&name)),
				name,
				key: member.key,
				decl: member.decl,
				ty: member.ty,
			};
			(start, function)
		})
		.collect();
	members.sort_by_key(|(start, _)| *start);
	Some(members.into_iter().map(|(_, function)| function).collect())
}

/// The C type of a parameter (or of the result: `result`) of an API
/// function; `None` when the type can't cross the boundary.
pub fn boundary_type(types: &TypeStore, ty: TypeId, result: bool) -> Option<BoundaryType> {
	Some(match types.kind(ty) {
		TypeKind::Void if result => BoundaryType::Void,
		TypeKind::Boolean => BoundaryType::Bool,
		TypeKind::Int(id) => {
			let int = types.int(id);
			if !int.is_full() || ty == TypeId::INTEGER || int.storage.bits() > 64 {
				return None;
			}
			BoundaryType::Int {
				bits: int.storage.bits(),
				signed: int.storage.is_signed(),
			}
		}
		TypeKind::Float(FloatBits::F32) => BoundaryType::Float32,
		TypeKind::Float(FloatBits::F64) => BoundaryType::Float64,
		TypeKind::CString => BoundaryType::CString { nullable: false },
		TypeKind::Pointer(target) => BoundaryType::Pointer {
			target,
			nullable: false,
		},
		TypeKind::Union(list) => {
			let items = types.list_items(list);
			let other = match items {
				[first, second] if *first == TypeId::NONE => *second,
				[first, second] if *second == TypeId::NONE => *first,
				_ => return None,
			};
			match types.kind(other) {
				TypeKind::CString => BoundaryType::CString { nullable: true },
				TypeKind::Pointer(target) => BoundaryType::Pointer {
					target,
					nullable: true,
				},
				_ => return None,
			}
		}
		TypeKind::Object(shape) => {
			let nominal = types.shape(shape).nominal?;
			if types.nominal(nominal).kind != NominalKind::CStruct {
				return None;
			}
			BoundaryType::Struct(ty)
		}
		_ => return None,
	})
}

/// A function of the C library throws errors of several types, or of no
/// `Errors` type: C can't tell them by their position.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NotOneErrorsType;

/// The `Errors` type whose members a function throws (`Ok(None)` for a
/// function that doesn't throw).
pub fn error_type(
	types: &TypeStore,
	throws: TypeId,
) -> Result<Option<NominalId>, NotOneErrorsType> {
	if throws == TypeId::NEVER {
		return Ok(None);
	}
	let nominal = |ty: TypeId| match types.kind(ty) {
		TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => Some(nominal),
		_ => None,
	};
	let found = match types.kind(throws) {
		TypeKind::Union(list) => {
			let mut found: Option<NominalId> = None;
			for &item in types.list_items(list) {
				let item = nominal(item).ok_or(NotOneErrorsType)?;
				if found.is_some_and(|found| found != item) {
					return Err(NotOneErrorsType);
				}
				found = Some(item);
			}
			found
		}
		_ => nominal(throws),
	};
	let found = found.ok_or(NotOneErrorsType)?;
	if types.nominal(found).kind != NominalKind::Errors {
		return Err(NotOneErrorsType);
	}
	Ok(Some(found))
}

/// The members of an `Errors` type, in declaration order (their status is
/// their position, from 1).
pub fn error_members(types: &TypeStore, errors: NominalId) -> Vec<String> {
	types
		.sym_list(types.nominal(errors).members)
		.iter()
		.map(|&member| types.syms.resolve(member).to_string())
		.collect()
}

/// The name of a declared type for C: its path joined (`Farm.Error` →
/// `FarmError`).
pub fn nominal_name(types: &TypeStore, nominal: NominalId) -> String {
	let key = types.nominal(nominal).key;
	types
		.sym_list(types.decl_key_data(key).names)
		.iter()
		.map(|&name| types.syms.resolve(name))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn snake_case_splits_words() {
		assert_eq!(snake_case("createFarm"), "create_farm");
		assert_eq!(snake_case("animalCount"), "animal_count");
		assert_eq!(snake_case("parseHTTPRequest"), "parse_http_request");
		assert_eq!(snake_case("count"), "count");
		assert_eq!(snake_case("toUTF8"), "to_utf8");
		assert_eq!(snake_case("vector3D"), "vector3_d");
	}
}
