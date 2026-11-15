//! `@typeToSchema(T)` (`types/types-at-runtime`): a type as a `Schema`
//! value, a named union whose members are `Schema.Named`, `Schema.Object`,
//! `Schema.Integer { bits, signed, min, max }`…
//!
//! The `Schema` type is in the prelude (`builtins/schema.rs`).

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

/// Sets the `default` of the properties of the object a schema describes
/// (through `Schema.Named`): the schemas of the literal default values,
/// which only the declaration knows; and puts the properties in the
/// declaration's order (`order`, the field names).
pub fn with_defaults(schema: Value, defaults: &[(String, Value)], order: &[String]) -> Value {
	if defaults.is_empty() && order.is_empty() {
		return schema;
	}
	let Value::Member(member) = &schema else {
		return schema;
	};
	let Some(Value::Object(data)) = &member.data else {
		return schema;
	};
	let mut data = (**data).clone();
	match &*member.name {
		"Schema.Named" => {
			if let Some(definition) = data.get("definition").cloned() {
				data.set(
					Arc::from("definition"),
					with_defaults(definition, defaults, order),
				);
			}
		}
		"Schema.Object" => {
			let Some(Value::Array(properties)) = data.get("properties").cloned() else {
				return schema;
			};
			let position = |property: &Value| {
				let name = match property {
					Value::Object(object) => object.get("name").and_then(|name| name.as_str()),
					_ => None,
				};
				name.and_then(|name| order.iter().position(|field| field == name))
					.unwrap_or(usize::MAX)
			};
			let mut properties: Vec<Value> = properties.iter().cloned().collect();
			properties.sort_by_key(position);
			let properties: Vec<Value> = properties
				.iter()
				.map(|property| {
					let Value::Object(object) = property else {
						return property.clone();
					};
					let name = object
						.get("name")
						.and_then(|name| name.as_str())
						.unwrap_or("");
					match defaults.iter().find(|(field, _)| field == name) {
						Some((_, default)) => {
							let mut object = (**object).clone();
							object.set(Arc::from("default"), default.clone());
							Value::Object(Arc::new(object))
						}
						None => property.clone(),
					}
				})
				.collect();
			data.set(Arc::from("properties"), Value::array(properties));
		}
		_ => return schema,
	}
	Value::Member(Arc::new(Member {
		name: member.name.clone(),
		kind: member.kind,
		data: Some(Value::Object(Arc::new(data))),
	}))
}

/// The `SchemaFlag`s of a `Fields` type: `{ name, flags }`, `flags` empty
/// for a leaf.
fn schema_flags(types: &TypeStore, ty: TypeId, depth: u32) -> Value {
	let flags = types
		.fields_keys(ty)
		.into_iter()
		.map(|(name, children)| {
			let members = if children.is_unset() || depth > 32 {
				Value::array(Vec::new())
			} else {
				schema_flags(types, children, depth + 1)
			};
			Value::object(vec![("name", Value::string(&name)), ("flags", members)])
		})
		.collect();
	Value::array(flags)
}

/// The schema of a type.
pub fn schema(types: &TypeStore, ty: TypeId) -> Value {
	describe(types, ty, 0, true)
}

fn describe(types: &TypeStore, ty: TypeId, depth: u32, named: bool) -> Value {
	if depth > 32 {
		return member("Never", Vec::new());
	}
	// a named union or an enumeration with all its members: used through its name
	let nominal_name = match types.kind(ty) {
		TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
			let info = types.nominal(nominal);
			let count = types.sym_list(info.members).len() as u16;
			(0..count)
				.all(|index| types.mask_has(mask, index))
				.then(|| types.decl_name(info.key))
		}
		_ => None,
	};
	if named
		&& let Some(name) = types.type_name(ty).or(nominal_name)
		&& !matches!(
			types.kind(ty),
			TypeKind::Int(_)
				| TypeKind::Float(_)
				| TypeKind::String
				| TypeKind::Bytes
				| TypeKind::Boolean
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
		// a recursive use inside its own definition
		TypeKind::Rec(rec) => {
			let name = types.syms.resolve(types.rec_info(rec).name);
			member("Reference", vec![("name", Value::string(name))])
		}
		TypeKind::Boolean => member("Boolean", Vec::new()),
		TypeKind::String => member("String", Vec::new()),
		TypeKind::Bytes => member("Bytes", Vec::new()),
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
			// every field of `Schema.Integer`: a value built in Fa has them all
			if int.min != range.lo || int.max != range.hi {
				fields.push(("min", Value::Integer(int.min)));
				fields.push(("max", Value::Integer(int.max)));
			} else {
				fields.push(("min", Value::None));
				fields.push(("max", Value::None));
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
		// `none` last, as `T?` writes it
		TypeKind::Union(list) => member(
			"Union",
			vec![(
				"items",
				Value::array(
					types
						.list_items(list)
						.iter()
						.filter(|&&item| item != TypeId::NONE)
						.chain(
							types
								.list_items(list)
								.iter()
								.filter(|&&item| item == TypeId::NONE),
						)
						.map(|&item| next(item))
						.collect(),
				),
			)],
		),
		// its flags in order, a group with its own flags
		TypeKind::Fields(_) => member("Fields", vec![("flags", schema_flags(types, ty, depth))]),
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
		// its parameters (named by position: a function type keeps no names),
		// result, errors and `async`; not its effects
		TypeKind::Function(sig) => {
			let sig = types.sig(sig);
			let parameters: Vec<Value> = types
				.params(sig.params)
				.iter()
				.enumerate()
				.map(|(index, parameter)| {
					Value::object(vec![
						("name", Value::string(&format!("parameter{}", index + 1))),
						("definition", next(parameter.ty)),
						("isReadonly", Value::Boolean(false)),
						("default", Value::None),
					])
				})
				.collect();
			let throws = if sig.throws == TypeId::NEVER {
				Value::None
			} else {
				next(sig.throws)
			};
			member(
				"Function",
				vec![
					("parameters", Value::array(parameters)),
					("returns", next(sig.ret)),
					("throws", throws),
					("async", Value::Boolean(sig.is_async)),
				],
			)
		}
		TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
			let info = types.nominal(nominal);
			let count = types.sym_list(info.members).len();
			let members: Vec<Value> = (0..count as u16)
				.filter(|&index| types.mask_has(mask, index))
				.map(|index| {
					// an enumeration member: its pinned value (its literal
					// payload), else its position
					if info.kind == crate::types::NominalKind::Enum {
						let pinned = types
							.as_int(types.payload(nominal, index))
							.filter(|int| int.min == int.max)
							.map(|int| int.min);
						let value = match pinned {
							Some(pinned) => Value::Integer(pinned),
							None => Value::int(i64::from(index)),
						};
						return Value::object(vec![
							("name", Value::string(types.member_name(nominal, index))),
							("value", value),
						]);
					}
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
