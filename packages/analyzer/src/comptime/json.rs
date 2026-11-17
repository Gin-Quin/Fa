//! JSON's compile-time reader uses the public type schema, decorated with
//! runtime details erased by reflection (integer representation, pinned enums,
//! declaration order and lazy field defaults).

use std::sync::Arc;

use fa_interpreter::Value;
use fa_interpreter::ir::FnKey;
use fa_interpreter::value::Closure;

use crate::analysis::Analysis;
use crate::ids::{FileId, NodeId, TypeId};
use crate::types::{TypeKind, TypeStore};

/// Synthetic function keys for field-default expressions. Source nodes cannot
/// reach this bit under the compiler's memory limit.
pub(super) const DEFAULT_EXPRESSION: u32 = 1 << 31;

pub(super) fn schema(
	types: &TypeStore,
	analysis: &Analysis,
	file: FileId,
	ty: TypeId,
) -> (Value, Vec<FnKey>) {
	let mut schema = super::schema::schema(types, ty);
	let mut defaults = Vec::new();
	decorate(types, analysis, file, ty, &mut schema, &mut defaults, 0);
	(schema, defaults)
}

fn data(schema: &mut Value) -> Option<&mut fa_interpreter::value::Object> {
	let Value::Member(member) = schema else {
		return None;
	};
	let Some(Value::Object(object)) = &mut Arc::make_mut(member).data else {
		return None;
	};
	Some(Arc::make_mut(object))
}

fn decorate(
	types: &TypeStore,
	analysis: &Analysis,
	file: FileId,
	ty: TypeId,
	schema: &mut Value,
	defaults: &mut Vec<FnKey>,
	depth: u32,
) {
	if depth > 64 {
		return;
	}
	// Reflection describes whole nominal types; JSON also accepts a value
	// whose inferred type is one particular member, including inside containers.
	if let TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) = types.kind(ty)
	{
		use super::schema::member;
		let info = types.nominal(nominal);
		let name = Value::string(types.member_name(nominal, index));
		let definition = if info.kind == crate::types::NominalKind::Enum {
			let pinned = types
				.as_int(types.payload(nominal, index))
				.map(|int| int.min);
			member(
				"Enum",
				vec![
					("pinned", Value::Boolean(pinned.is_some())),
					(
						"members",
						Value::array(vec![Value::object(vec![
							("name", name),
							(
								"value",
								Value::Integer(pinned.unwrap_or_else(|| i64::from(index).into())),
							),
						])]),
					),
				],
			)
		} else {
			let payload = types.payload(nominal, index);
			let mut definition = if payload.is_unset() || payload == TypeId::NONE {
				Value::None
			} else {
				super::schema::schema(types, payload)
			};
			if !matches!(definition, Value::None) {
				decorate(
					types,
					analysis,
					file,
					payload,
					&mut definition,
					defaults,
					depth + 1,
				);
			}
			member(
				"NamedUnion",
				vec![
					(
						"errors",
						Value::Boolean(info.kind == crate::types::NominalKind::Errors),
					),
					(
						"members",
						Value::array(vec![Value::object(vec![
							("name", name),
							("definition", definition),
						])]),
					),
				],
			)
		};
		*schema = member(
			"Named",
			vec![
				("name", Value::string(types.decl_name(info.key))),
				("definition", definition),
			],
		);
		return;
	}
	if let Value::Member(member) = schema
		&& member.name.as_ref() == "Schema.Named"
	{
		if let Some(definition) = data(schema).and_then(|data| data.get_mut("definition")) {
			decorate(types, analysis, file, ty, definition, defaults, depth + 1);
		}
		return;
	}
	match types.kind(ty) {
		TypeKind::Int(_) => {
			if let (Some(integer), Some(data)) = (types.as_int(ty), data(schema)) {
				data.set(
					Arc::from("jsNumber"),
					Value::Boolean(!integer.storage.is_js_bigint()),
				);
			}
		}
		TypeKind::Enum(nominal, _) => {
			let count = types.sym_list(types.nominal(nominal).members).len() as u16;
			let pinned = (0..count).any(|i| types.as_int(types.payload(nominal, i)).is_some());
			if let Some(data) = data(schema) {
				data.set(Arc::from("pinned"), Value::Boolean(pinned));
			}
		}
		TypeKind::Fields(_) => {
			if let Some(data) = data(schema) {
				data.set(
					Arc::from("paths"),
					Value::array(
						types
							.fields_names(ty)
							.into_iter()
							.map(|(name, bits)| {
								Value::Tuple(Arc::new(vec![
									Value::string(&name),
									Value::Integer(bits.into()),
								]))
							})
							.collect(),
					),
				);
			}
		}
		TypeKind::Object(_) => {
			let declaration = analysis.type_decls.values().find(|d| d.ty == ty);
			let Some(Value::Array(properties)) = data(schema).and_then(|d| d.get_mut("properties"))
			else {
				return;
			};
			let properties = Arc::make_mut(properties);
			let order: Vec<_> = declaration
				.map(|d| d.fields.iter().map(|f| f.name).collect())
				.or_else(|| analysis.inline_field_orders.get(&ty).cloned())
				.unwrap_or_default();
			properties.sort_by_key(|property| match property {
				Value::Object(object) => object
					.get("name")
					.and_then(Value::as_str)
					.and_then(|name| {
						order
							.iter()
							.position(|&sym| types.syms.resolve(sym) == name)
					})
					.unwrap_or(usize::MAX),
				_ => usize::MAX,
			});
			for property in properties {
				let Value::Object(object) = property else {
					continue;
				};
				let object = Arc::make_mut(object);
				let name = object
					.get("name")
					.and_then(Value::as_str)
					.unwrap_or("")
					.to_string();
				let Some(field) = types
					.fields_of(ty)
					.iter()
					.find(|f| types.syms.resolve(f.name) == name)
				else {
					continue;
				};
				if let Some(definition) = object.get_mut("definition") {
					decorate(
						types,
						analysis,
						file,
						field.ty,
						definition,
						defaults,
						depth + 1,
					);
				}
				let declared =
					declaration.and_then(|d| d.fields.iter().find(|f| f.name == field.name));
				if let Some(default) = declared.and_then(|f| f.default) {
					let key = FnKey(file.0, default.0 | DEFAULT_EXPRESSION);
					defaults.push(key);
					object.set(
						Arc::from("initial"),
						Value::Function(Arc::new(Closure {
							function: key,
							captured: Vec::new(),
							bound: Vec::new(),
						})),
					);
				} else if let Some(index) = declared
					.and_then(|f| f.constant)
					.or_else(|| analysis.json_defaults.get(&(ty, field.name)).copied())
					&& let Some(value) = analysis.field_constants.get(index as usize)
				{
					object.set(Arc::from("initial"), value.clone());
				}
			}
		}
		TypeKind::Union(list) => {
			if let Some(Value::Array(items)) = data(schema).and_then(|d| d.get_mut("items")) {
				let members = types.list_items(list);
				let ordered = members
					.iter()
					.filter(|m| **m != TypeId::NONE)
					.chain(members.iter().filter(|m| **m == TypeId::NONE));
				for (&ty, item) in ordered.zip(Arc::make_mut(items)) {
					decorate(types, analysis, file, ty, item, defaults, depth + 1);
				}
			}
		}
		TypeKind::Collection(kind, item, capacity) => {
			if kind == crate::types::CollKind::InlineArray
				&& matches!(types.kind(capacity), TypeKind::Param(_))
				&& let Some(data) = data(schema)
			{
				data.set(Arc::from("$capacityType"), Value::int(capacity.0 as i64));
			}
			if let Some(definition) = data(schema).and_then(|d| d.get_mut("item")) {
				decorate(types, analysis, file, item, definition, defaults, depth + 1);
			}
		}
		TypeKind::NamedUnion(nominal, mask) => {
			if let Some(Value::Array(members)) = data(schema).and_then(|d| d.get_mut("members")) {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				for (index, member) in (0..count)
					.filter(|i| types.mask_has(mask, *i))
					.zip(Arc::make_mut(members))
				{
					if let Value::Object(member) = member
						&& let Some(definition) = Arc::make_mut(member).get_mut("definition")
					{
						decorate(
							types,
							analysis,
							file,
							types.payload(nominal, index),
							definition,
							defaults,
							depth + 1,
						);
					}
				}
			}
		}
		_ => {}
	}
}

pub(super) fn default_node(node: NodeId) -> Option<NodeId> {
	(node.0 & DEFAULT_EXPRESSION != 0).then_some(NodeId(node.0 & !DEFAULT_EXPRESSION))
}
