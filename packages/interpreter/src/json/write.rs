//! JSON serialization uses an explicit stack, including recursive Fa values.

use super::{kind, list_property, property};
use crate::eval::{Error, Interpreter};
use crate::text::Text;
use crate::value::{MemberKind, Value, format_float};

/// Write with the type schema, preserving field order and raw surrogate units.
pub(crate) fn stringify(
	interpreter: &mut Interpreter<'_>,
	value: &Value,
	schema: &Value,
) -> Result<Value, Error> {
	let mut writer = Writer {
		interpreter,
		output: String::new(),
		names: Vec::new(),
	};
	let mut pending = vec![Task::Value(value, schema)];
	while let Some(task) = pending.pop() {
		writer.interpreter.step().map_err(Error::from)?;
		match task {
			Task::RestoreNames(length) => writer.names.truncate(length),
			Task::Raw(text) => writer.push(text)?,
			Task::Key(text) => writer.quote(&text.into())?,
			Task::Text(text) => writer.quote(text)?,
			Task::Value(value, schema) => {
				writer.tasks(&mut pending, 2)?;
				pending.push(Task::RestoreNames(writer.names.len()));
				let schema = writer.resolve(value, schema);
				if kind(schema) == "Fields" {
					let bits = value.as_int().unwrap_or_default();
					writer.push("[")?;
					let mut separator = "";
					for (index, entry) in list_property(schema, "paths").iter().enumerate() {
						let Some(items) = entry.items() else { continue };
						let mask = items.get(1).and_then(Value::as_int).unwrap_or_default();
						// A one-child group also has a single bit: distinguish
						// groups by their descendants, not by their mask width.
						let name = items.first().and_then(Value::as_str).unwrap_or("");
						let group = list_property(schema, "paths")
							.get(index + 1)
							.and_then(Value::items)
							.and_then(|items| items.first())
							.and_then(Value::as_str)
							.is_some_and(|other| {
								other
									.strip_prefix(name)
									.is_some_and(|rest| rest.starts_with('.'))
							});
						if !group && bits & mask != ethnum::I256::ZERO {
							writer.push(separator)?;
							if let Some(Value::String(name)) = items.first() {
								writer.quote(name)?;
							}
							separator = ",";
						}
					}
					writer.push("]")?;
					continue;
				}
				match value {
					Value::None | Value::Function(_) => writer.push("null")?,
					Value::Boolean(value) => writer.push(if *value { "true" } else { "false" })?,
					Value::Integer(value) => writer.push(&value.to_string())?,
					Value::Float(value) | Value::Percentage(value) => {
						writer.push(&if value.is_finite() {
							format_float(*value)
						} else {
							"null".into()
						})?;
					}
					Value::String(text) => writer.quote(text)?,
					Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
						writer.tasks(
							&mut pending,
							items.len().saturating_mul(2).saturating_add(1),
						)?;
						writer.push("[")?;
						pending.push(Task::Raw("]"));
						for (index, item) in items.iter().enumerate().rev() {
							let item_schema = if kind(schema) == "Tuple" {
								list_property(schema, "items").get(index)
							} else {
								property(schema, "item")
							}
							.unwrap_or(&Value::None);
							pending.push(Task::Value(item, item_schema));
							if index > 0 {
								pending.push(Task::Raw(","));
							}
						}
					}
					Value::Object(object) => {
						writer.push("{")?;
						writer.fields(&mut pending, &object.fields, schema, false)?;
					}
					Value::Map(entries) => {
						writer.tasks(
							&mut pending,
							entries.len().saturating_mul(4).saturating_add(1),
						)?;
						writer.push("{")?;
						pending.push(Task::Raw("}"));
						let mut separator = false;
						for (key, item) in entries.iter().rev().filter(|(_, v)| !omitted(v)) {
							if separator {
								pending.push(Task::Raw(","));
							}
							pending.push(Task::Value(
								item,
								property(schema, "item").unwrap_or(&Value::None),
							));
							pending.push(Task::Raw(":"));
							if let Value::String(key) = key {
								pending.push(Task::Text(key));
							}
							separator = true;
						}
					}
					Value::Member(member) => {
						let name = member.name.rsplit('.').next().unwrap_or(&member.name);
						let entry = list_property(schema, "members").iter().find(|entry| {
							property(entry, "name").and_then(Value::as_str) == Some(name)
						});
						if member.kind == MemberKind::Enumeration {
							if property(schema, "pinned").and_then(Value::as_bool) == Some(true) {
								let pinned = entry
									.and_then(|entry| property(entry, "value"))
									.unwrap_or(&Value::None);
								pending.push(Task::Value(pinned, &Value::None));
							} else {
								writer.quote(&name.into())?;
							}
							continue;
						}
						let data_schema = entry
							.and_then(|entry| property(entry, "definition"))
							.unwrap_or(&Value::None);
						let literal = match kind(data_schema) {
							"StringLiteral" | "BooleanLiteral" => property(data_schema, "value"),
							"Integer"
								if property(data_schema, "min") == property(data_schema, "max") =>
							{
								property(data_schema, "min")
							}
							_ => None,
						};
						if let Some(literal) = literal {
							pending.push(Task::Value(literal, &Value::None));
							continue;
						}
						writer.push("{\"type\":")?;
						writer.quote(&name.into())?;
						match member.data.as_ref() {
							Some(Value::Object(object)) if object.get("type").is_none() => {
								let data_schema = writer.resolve(&Value::None, data_schema);
								writer.fields(&mut pending, &object.fields, data_schema, true)?;
							}
							Some(data) if !omitted(data) => {
								writer.push(",\"value\":")?;
								pending.push(Task::Raw("}"));
								pending.push(Task::Value(data, data_schema));
							}
							_ => writer.push("}")?,
						}
					}
					Value::Bytes(_) | Value::Range { .. } => {
						return Err(Error::Unsupported(
							"JSON serialization of this value".into(),
						));
					}
				}
			}
		}
	}
	Ok(Value::String(writer.output.into()))
}

fn omitted(value: &Value) -> bool {
	matches!(value, Value::None | Value::Function(_))
}

/// Anonymous object unions are distinguished by missing fields or disjoint
/// literal fields. Inspect those discriminants without walking recursive data.
fn discriminant_matches(value: &Value, schema: &Value) -> bool {
	match kind(schema) {
		"StringLiteral" | "BooleanLiteral" => property(schema, "value") == Some(value),
		"Integer" => value.as_int().is_some_and(|number| {
			property(schema, "min")
				.and_then(Value::as_int)
				.is_none_or(|min| number >= min)
				&& property(schema, "max")
					.and_then(Value::as_int)
					.is_none_or(|max| number <= max)
		}),
		"Union" => list_property(schema, "items")
			.iter()
			.any(|member| discriminant_matches(value, member)),
		"Named" => discriminant_matches(
			value,
			property(schema, "definition").unwrap_or(&Value::None),
		),
		_ => true,
	}
}

enum Task<'a> {
	RestoreNames(usize),
	Raw(&'static str),
	Key(&'a str),
	Text(&'a Text),
	Value(&'a Value, &'a Value),
}

struct Writer<'a, 'h, 's> {
	interpreter: &'a mut Interpreter<'h>,
	output: String,
	names: Vec<(&'s str, &'s Value)>,
}

impl<'s> Writer<'_, '_, 's> {
	fn tasks(&self, pending: &mut Vec<Task<'s>>, extra: usize) -> Result<(), Error> {
		let needed = pending.len().saturating_add(extra);
		if needed > pending.capacity() {
			let capacity = needed.checked_next_power_of_two().unwrap_or(usize::MAX);
			self.interpreter
				.reserve(capacity.saturating_mul(std::mem::size_of::<Task<'s>>()))?;
			pending.reserve_exact(capacity - pending.len());
		}
		Ok(())
	}

	fn push(&mut self, text: &str) -> Result<(), Error> {
		let size = self.output.len().saturating_add(text.len());
		self.interpreter.reserve_text(size.saturating_mul(2))?;
		self.output.push_str(text);
		Ok(())
	}

	fn quote(&mut self, text: &Text) -> Result<(), Error> {
		self.push("\"")?;
		let mut offset = 0;
		while let Some(code) = text.next_scalar(&mut offset) {
			self.interpreter.step().map_err(Error::from)?;
			match code {
				0x22 => self.push("\\\"")?,
				0x5c => self.push("\\\\")?,
				8 => self.push("\\b")?,
				9 => self.push("\\t")?,
				10 => self.push("\\n")?,
				12 => self.push("\\f")?,
				13 => self.push("\\r")?,
				0..=31 | 0xd800..=0xdfff => self.push(&format!("\\u{code:04x}"))?,
				_ => {
					let mut bytes = [0; 4];
					self.push(
						char::from_u32(code)
							.expect("a decoded scalar")
							.encode_utf8(&mut bytes),
					)?;
				}
			}
		}
		self.push("\"")
	}

	fn resolve(&mut self, value: &Value, mut schema: &'s Value) -> &'s Value {
		for _ in 0..64 {
			match kind(schema) {
				"Named" => {
					let name = property(schema, "name")
						.and_then(Value::as_str)
						.unwrap_or("");
					schema = property(schema, "definition").unwrap_or(&Value::None);
					self.names.push((name, schema));
				}
				"Reference" => {
					let name = property(schema, "name")
						.and_then(Value::as_str)
						.unwrap_or("");
					schema = self
						.names
						.iter()
						.rev()
						.find(|(n, _)| *n == name)
						.map(|(_, s)| *s)
						.unwrap_or(&Value::None);
				}
				"Union" => {
					let chosen = list_property(schema, "items")
						.iter()
						.find(|s| self.matches(value, s));
					schema = chosen.unwrap_or(&Value::None);
				}
				_ => break,
			}
		}
		schema
	}

	fn matches(&mut self, value: &Value, schema: &'s Value) -> bool {
		let names = self.names.len();
		let schema = self.resolve(value, schema);
		let matched = match kind(schema) {
			"None" => matches!(value, Value::None),
			"Boolean" => matches!(value, Value::Boolean(_)),
			"String" => matches!(value, Value::String(_)),
			"StringLiteral" | "BooleanLiteral" | "Integer" => discriminant_matches(value, schema),
			"Fields" => matches!(value, Value::Integer(_)),
			"Float" => matches!(value, Value::Float(_)),
			"Object" => {
				matches!(value, Value::Object(object) if list_property(schema, "properties").iter().all(|field| {
					let Some(name) = property(field, "name").and_then(Value::as_str) else { return false };
					object.get(name).is_some_and(|value| discriminant_matches(value, property(field, "definition").unwrap_or(&Value::None)))
				}))
			}
			"Tuple" => {
				matches!(value, Value::Tuple(items) if items.len() == list_property(schema, "items").len())
			}
			"Collection" => match property(schema, "kind").and_then(Value::as_str) {
				Some("Array" | "GrowingArray") => matches!(value, Value::Array(_)),
				Some("Set" | "GrowingSet") => matches!(value, Value::Set(_)),
				Some("Map" | "GrowingMap") => matches!(value, Value::Map(_)),
				_ => false,
			},
			"Enum" | "NamedUnion" => {
				matches!(value, Value::Member(m) if list_property(schema,"members").iter().any(|e| property(e,"name").and_then(Value::as_str) == m.name.rsplit('.').next()))
			}
			_ => false,
		};
		self.names.truncate(names);
		matched
	}

	fn fields(
		&mut self,
		pending: &mut Vec<Task<'s>>,
		fields: &'s [(std::sync::Arc<str>, Value)],
		schema: &'s Value,
		tagged: bool,
	) -> Result<(), Error> {
		self.tasks(pending, fields.len().saturating_mul(4).saturating_add(1))?;
		pending.push(Task::Raw("}"));
		let mut separator = false;
		let declared = list_property(schema, "properties")
			.iter()
			.rev()
			.filter_map(|field| {
				let name = property(field, "name")?.as_str()?;
				let (_, value) = fields.iter().find(|(f, _)| f.as_ref() == name)?;
				Some((
					name,
					value,
					property(field, "definition").unwrap_or(&Value::None),
				))
			});
		let fallback = fields
			.iter()
			.rev()
			.filter(|_| kind(schema) != "Object")
			.map(|(name, value)| (name.as_ref(), value, &Value::None));
		for (name, value, definition) in declared.chain(fallback).filter(|(_, v, _)| !omitted(v)) {
			if separator {
				pending.push(Task::Raw(","));
			}
			pending.push(Task::Value(value, definition));
			pending.push(Task::Raw(":"));
			pending.push(Task::Key(name));
			separator = true;
		}
		if tagged && separator {
			self.push(",")?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::eval::{Host, Limits};
	use crate::ir::{FnKey, Function, GlobalKey};
	use std::sync::Arc;

	struct NoHost;
	impl Host for NoHost {
		fn function(&mut self, _: FnKey) -> Result<Arc<Function>, Error> {
			unreachable!("serialization calls no Fa function")
		}
		fn global(&mut self, _: GlobalKey) -> Result<Value, Error> {
			unreachable!("serialization reads no globals")
		}
	}

	#[test]
	fn json_preserves_surrogates_and_omits_only_object_none() {
		let value = Value::object(vec![
			(
				"raw",
				Value::String(Text::from_units(&[0xd83d, 0, 10, 0xde00])),
			),
			("missing", Value::None),
			(
				"items",
				Value::array(vec![
					Value::None,
					Value::Float(f64::INFINITY),
					Value::Float(-0.0),
				]),
			),
		]);
		let mut host = NoHost;
		let mut interpreter = Interpreter::new(&mut host, Limits::default());
		let result = stringify(&mut interpreter, &value, &Value::None).unwrap();
		assert_eq!(
			result.as_str(),
			Some(r#"{"raw":"\ud83d\u0000\n\ude00","items":[null,null,0]}"#)
		);
	}

	#[test]
	fn serialization_obeys_step_and_memory_limits() {
		let value = Value::String("a".repeat(4096).into());
		for limits in [
			Limits {
				steps: 10,
				..Limits::default()
			},
			Limits {
				memory: 1024,
				..Limits::default()
			},
		] {
			let mut host = NoHost;
			let mut interpreter = Interpreter::new(&mut host, limits);
			let result = stringify(&mut interpreter, &value, &Value::None);
			assert!(
				matches!(result, Err(Error::Steps(10) | Error::Memory(1024))),
				"{result:?}"
			);
		}
	}
}
