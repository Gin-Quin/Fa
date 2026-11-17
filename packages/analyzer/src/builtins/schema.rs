//! `Schema` (`types/types-at-runtime`, types brief §3.14, D48–D49): the
//! prelude named union that describes a type, the result of
//! `@typeToSchema(T)` and the argument of `TypeFromSchema(schema)`, with
//! `SchemaProperty`, `SchemaMember` and `SchemaFlag`.
//!
//! Declared in the module `prelude` like the built-in errors. The default
//! values of their fields (`signed: Boolean = true` of `Schema.Integer`) are
//! given by [`field_defaults`].

use crate::builtins::Builtin;
use crate::builtins::BuiltinKind::BuiltType;
use crate::builtins::errors::PRELUDE_MODULE;
use crate::ids::{ListId, TypeId};
use crate::types::{FieldEntry, FieldFlags, NominalInfo, NominalKind, TypeStore};

/// The members of `Schema`, in declaration order.
pub const MEMBERS: &[&str] = &[
	"Named",
	"Reference",
	"Boolean",
	"String",
	"Character",
	"Percentage",
	"BigInt",
	"None",
	"Never",
	"Void",
	"Integer",
	"Float",
	"BooleanLiteral",
	"IntegerLiteral",
	"FloatLiteral",
	"StringLiteral",
	"Object",
	"Union",
	"NamedUnion",
	"Enum",
	"Fields",
	"Tuple",
	"Collection",
	"Function",
	"InlineArray",
];

/// `Schema` and its companion object types.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Schema",
		kind: BuiltType(schema),
	},
	Builtin {
		path: "SchemaProperty",
		kind: BuiltType(property),
	},
	Builtin {
		path: "SchemaMember",
		kind: BuiltType(member),
	},
	Builtin {
		path: "SchemaFlag",
		kind: BuiltType(flag),
	},
];

/// An object type named `name`, its fields as `(name, type)`.
fn object(types: &mut TypeStore, name: &str, fields: &[(&str, TypeId)]) -> TypeId {
	let fields: Vec<FieldEntry> = fields
		.iter()
		.map(|&(field, ty)| FieldEntry {
			name: types.syms.intern(field),
			ty,
			flags: FieldFlags::empty(),
		})
		.collect();
	let ty = types.object(&fields, None);
	if !name.is_empty() {
		types.name_type(ty, name);
	}
	ty
}

/// `Schema`, with the data of its members (built once per store).
pub fn schema(types: &mut TypeStore) -> TypeId {
	let module = types.module(PRELUDE_MODULE);
	let key = types.decl_key(module, &["Schema"], 0);
	let names: Vec<crate::ids::Sym> = MEMBERS.iter().map(|name| types.syms.intern(name)).collect();
	let members = types.intern_sym_list(&names);
	let nominal = types.intern_nominal(NominalInfo {
		key,
		args: ListId::EMPTY,
		kind: NominalKind::Union,
		members,
	});
	let ty = types.nominal_type(nominal);
	if types.has_payloads(nominal) {
		return ty;
	}
	// set before the payloads are built: they refer to `Schema`
	types.set_payloads(nominal, vec![TypeId::UNSET; MEMBERS.len()]);
	let string = TypeId::STRING;
	let schemas = types.array(ty);
	let optional_schema = types.optional(ty);
	let count = types.int_range(
		ethnum::I256::ZERO,
		ethnum::I256::from(i64::MAX),
		crate::types::IntStorage::Default,
	);
	let optional_big = types.optional(TypeId::BIG_INT);
	let property = property(types);
	let properties = types.array(property);
	let member = member(types);
	let members_array = types.array(member);
	let flag = flag(types);
	let flags = types.array(flag);
	let enum_member = object(types, "", &[("name", string), ("value", TypeId::INTEGER)]);
	let enum_members = types.array(enum_member);
	let payloads: Vec<TypeId> = MEMBERS
		.iter()
		.map(|&name| match name {
			"Named" => object(types, "", &[("name", string), ("definition", ty)]),
			"Reference" => object(types, "", &[("name", string)]),
			"Integer" => object(
				types,
				"",
				&[
					("bits", TypeId::INTEGER),
					("signed", TypeId::BOOLEAN),
					("min", optional_big),
					("max", optional_big),
				],
			),
			"Float" => object(types, "", &[("bits", TypeId::INTEGER)]),
			"BooleanLiteral" => object(types, "", &[("value", TypeId::BOOLEAN)]),
			"IntegerLiteral" => object(types, "", &[("value", TypeId::BIG_INT)]),
			"FloatLiteral" => object(types, "", &[("value", TypeId::FLOAT)]),
			"StringLiteral" => object(types, "", &[("value", string)]),
			"Object" => object(types, "", &[("properties", properties)]),
			"Union" | "Tuple" => object(types, "", &[("items", schemas)]),
			"NamedUnion" => object(
				types,
				"",
				&[("errors", TypeId::BOOLEAN), ("members", members_array)],
			),
			"Enum" => object(types, "", &[("members", enum_members)]),
			"Fields" => object(types, "", &[("flags", flags)]),
			"Collection" => object(
				types,
				"",
				&[("kind", string), ("key", optional_schema), ("item", ty)],
			),
			"InlineArray" => object(types, "", &[("item", ty), ("capacity", count)]),
			"Function" => {
				let optional = optional_schema;
				object(
					types,
					"",
					&[
						("parameters", properties),
						("returns", ty),
						("throws", optional),
						("async", TypeId::BOOLEAN),
					],
				)
			}
			_ => TypeId::UNSET,
		})
		.collect();
	types.set_payloads(nominal, payloads);
	ty
}

/// The default values of the fields of an object type of `Schema` (a
/// payload of one of its members), as the page declares them: `signed:
/// Boolean = true` of `Schema.Integer`. Empty for any other type.
pub fn field_defaults(
	types: &mut TypeStore,
	ty: TypeId,
) -> Vec<(crate::ids::Sym, fa_interpreter::Value)> {
	let crate::types::TypeKind::Object(_) = types.kind(ty) else {
		return Vec::new();
	};
	let schema = schema(types);
	let crate::types::TypeKind::NamedUnion(nominal, _) = types.kind(schema) else {
		return Vec::new();
	};
	let integer = MEMBERS
		.iter()
		.position(|&name| name == "Integer")
		.expect("`Schema.Integer`");
	if types.payload(nominal, integer as u16) != ty {
		return Vec::new();
	}
	vec![(
		types.syms.intern("signed"),
		fa_interpreter::Value::Boolean(true),
	)]
}

/// `SchemaProperty = { name: String, definition: Schema, isReadonly: Boolean, default: Schema? }`
pub fn property(types: &mut TypeStore) -> TypeId {
	let schema = schema_id(types);
	let optional = types.optional(schema);
	object(
		types,
		"SchemaProperty",
		&[
			("name", TypeId::STRING),
			("definition", schema),
			("isReadonly", TypeId::BOOLEAN),
			("default", optional),
		],
	)
}

/// `SchemaMember = { name: String, definition: Schema? }`
pub fn member(types: &mut TypeStore) -> TypeId {
	let schema = schema_id(types);
	let optional = types.optional(schema);
	object(
		types,
		"SchemaMember",
		&[("name", TypeId::STRING), ("definition", optional)],
	)
}

/// `SchemaFlag = { name: String, flags: Array(SchemaFlag) }` (recursive).
pub fn flag(types: &mut TypeStore) -> TypeId {
	let module = types.module(PRELUDE_MODULE);
	let key = types.decl_key(module, &["SchemaFlag"], 0);
	let reference = types.recursive_reference(key, "SchemaFlag", &[], true);
	if let Some(target) = types.rec_target(reference) {
		return target;
	}
	let flags = types.array(reference);
	let ty = object(
		types,
		"SchemaFlag",
		&[("name", TypeId::STRING), ("flags", flags)],
	);
	types.set_rec_target(reference, ty);
	ty
}

/// The id of `Schema` without building its payloads (they are built by
/// [`schema`], which may be running).
fn schema_id(types: &mut TypeStore) -> TypeId {
	let module = types.module(PRELUDE_MODULE);
	let key = types.decl_key(module, &["Schema"], 0);
	let names: Vec<crate::ids::Sym> = MEMBERS.iter().map(|name| types.syms.intern(name)).collect();
	let members = types.intern_sym_list(&names);
	let nominal = types.intern_nominal(NominalInfo {
		key,
		args: ListId::EMPTY,
		kind: NominalKind::Union,
		members,
	});
	let ty = types.nominal_type(nominal);
	if !types.has_payloads(nominal) {
		return schema(types);
	}
	ty
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::TypeKind;

	#[test]
	fn schema_is_a_named_union_with_data() {
		let mut types = TypeStore::new();
		let ty = schema(&mut types);
		assert_eq!(schema(&mut types), ty, "built once");
		assert_eq!(types.display(ty), "Schema");
		let TypeKind::NamedUnion(nominal, _) = types.kind(ty) else {
			panic!("not a named union")
		};
		let named = types.payload(nominal, 0);
		assert_eq!(types.display(named), "{ definition: Schema, name: String }");
		assert!(
			types.payload(nominal, 2).is_unset(),
			"`Boolean` has no data"
		);
		let property = property(&mut types);
		assert_eq!(types.display(property), "SchemaProperty");
		let flag = flag(&mut types);
		assert_eq!(flag, super::flag(&mut types));
	}
}
