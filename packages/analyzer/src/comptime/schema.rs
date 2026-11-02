//! `@typeToSchema(T)` (`types/types-at-runtime`): a type as a `Schema`
//! value, a named union whose members are `Schema.Named`, `Schema.Object`,
//! `Schema.Integer { bits, signed, min, max }`…
//!
//! The `Schema` type itself is not in the analyzer's prelude yet: the value
//! is computed (the backends embed it, compile-time code compares it), but
//! its static type is unknown to the walk.

use std::sync::Arc;

use fa_interpreter::Value;
use fa_interpreter::value::{Member, MemberKind};

use crate::ids::TypeId;
use crate::types::{CollKind, FloatBits, IntStorage, TypeKind, TypeStore};

fn member(name: &str, fields: Vec<(&str, Value)>) -> Value {
	let data = (!fields.is_empty()).then(|| Value::object(fields));
	Value::Member(Arc::new(Member {
		name: Arc::from(format!("Schema.{name}")),
		kind: MemberKind::Union,
		data,
	}))
}

/// The schema of a type.
pub fn schema(types: &TypeStore, ty: TypeId) -> Value {
	describe(types, ty, 0, true)
}

fn describe(types: &TypeStore, ty: TypeId, depth: u32, named: bool) -> Value {
	if depth > 32 {
		return member("Never", Vec::new());
	}
	if named
		&& let Some(name) = types.type_name(ty)
		&& !matches!(
			types.kind(ty),
			TypeKind::Int(_) | TypeKind::Float(_) | TypeKind::String | TypeKind::Boolean
		) {
		return member(
			"Named",
			vec![
				("name", Value::string(name)),
				("definition", describe(types, ty, depth + 1, false)),
			],
		);
	}
	let next = |ty: TypeId| describe(types, ty, depth + 1, true);
	match types.kind(ty) {
		TypeKind::Boolean => member("Boolean", Vec::new()),
		TypeKind::String => member("String", Vec::new()),
		TypeKind::Character => member("Character", Vec::new()),
		TypeKind::Percentage => member("Percentage", Vec::new()),
		TypeKind::BigInt => member("BigInt", Vec::new()),
		TypeKind::None => member("None", Vec::new()),
		TypeKind::Never => member("Never", Vec::new()),
		TypeKind::Void => member("Void", Vec::new()),
		TypeKind::BoolLit(value) => {
			member("BooleanLiteral", vec![("value", Value::Boolean(value))])
		}
		TypeKind::StringLit(text) => member(
			"StringLiteral",
			vec![("value", Value::string(types.syms.resolve(text)))],
		),
		TypeKind::Int(_) => {
			let Some(int) = types.as_int(ty) else {
				return member("Never", Vec::new());
			};
			if int.min == int.max {
				return member("IntegerLiteral", vec![("value", Value::Integer(int.min))]);
			}
			let storage = int.storage;
			let range = storage.range();
			let bits = match storage {
				IntStorage::I8 | IntStorage::U8 => 8,
				IntStorage::I16 | IntStorage::U16 => 16,
				IntStorage::I32 | IntStorage::U32 => 32,
				IntStorage::I128 | IntStorage::U128 => 128,
				_ => 64,
			};
			let signed =
				!matches!(
					storage,
					IntStorage::U8
						| IntStorage::U16 | IntStorage::U32
						| IntStorage::U64 | IntStorage::U128
				);
			let mut fields = vec![
				("bits", Value::int(bits)),
				("signed", Value::Boolean(signed)),
			];
			if int.min != range.lo || int.max != range.hi {
				fields.push(("min", Value::Integer(int.min)));
				fields.push(("max", Value::Integer(int.max)));
			}
			member("Integer", fields)
		}
		TypeKind::Float(bits) | TypeKind::FloatLit(bits, _) => member(
			"Float",
			vec![(
				"bits",
				Value::int(if bits == FloatBits::F32 { 32 } else { 64 }),
			)],
		),
		TypeKind::Object(shape) => {
			let properties = types
				.shape_fields(shape)
				.iter()
				.map(|field| {
					Value::object(vec![
						("name", Value::string(types.syms.resolve(field.name))),
						("definition", next(field.ty)),
						(
							"isReadonly",
							Value::Boolean(
								field.flags.contains(crate::types::FieldFlags::READONLY),
							),
						),
						("default", Value::None),
					])
				})
				.collect();
			member("Object", vec![("properties", Value::array(properties))])
		}
		TypeKind::Union(list) => member(
			"Union",
			vec![(
				"items",
				Value::array(
					types
						.list_items(list)
						.iter()
						.map(|&item| next(item))
						.collect(),
				),
			)],
		),
		TypeKind::Tuple(list) => member(
			"Tuple",
			vec![(
				"items",
				Value::array(
					types
						.list_items(list)
						.iter()
						.map(|&item| next(item))
						.collect(),
				),
			)],
		),
		TypeKind::Collection(kind, item, key) => {
			let name = match kind {
				CollKind::Array => "Array",
				CollKind::GrowingArray => "GrowingArray",
				CollKind::Set => "Set",
				CollKind::GrowingSet => "GrowingSet",
				CollKind::Map => "Map",
				CollKind::GrowingMap => "GrowingMap",
				_ => "Collection",
			};
			let key = if key == TypeId::NONE || key.is_unset() {
				Value::None
			} else {
				next(key)
			};
			member(
				"Collection",
				vec![
					("kind", Value::string(name)),
					("key", key),
					("item", next(item)),
				],
			)
		}
		TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
			let info = types.nominal(nominal);
			let count = types.sym_list(info.members).len();
			let members: Vec<Value> = (0..count as u16)
				.filter(|&index| types.mask_has(mask, index))
				.map(|index| {
					let payload = types.payload(nominal, index);
					Value::object(vec![
						("name", Value::string(types.member_name(nominal, index))),
						(
							"definition",
							if payload.is_unset() || payload == TypeId::NONE {
								Value::None
							} else {
								next(payload)
							},
						),
					])
				})
				.collect();
			match info.kind {
				crate::types::NominalKind::Enum => {
					member("Enum", vec![("members", Value::array(members))])
				}
				kind => member(
					"NamedUnion",
					vec![
						(
							"errors",
							Value::Boolean(kind == crate::types::NominalKind::Errors),
						),
						("members", Value::array(members)),
					],
				),
			}
		}
		_ => member("Never", Vec::new()),
	}
}
