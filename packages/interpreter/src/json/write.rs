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
			Task::Raw(text) => writer.push(text)?,
			Task::Key(text) => writer.quote(&text.into())?,
			Task::Text(text) => writer.quote(text)?,
			Task::Value(value, schema) => {
				let schema = writer.resolve(value, schema);
				if kind(schema) == "Fields" {
					let bits = value.as_int().unwrap_or_default();
					writer.push("[")?;
					let mut separator = "";
					for entry in list_property(schema, "paths") {
						let Some(items) = entry.items() else { continue };
						let mask = items.get(1).and_then(Value::as_int).unwrap_or_default();
						// A path can describe a group; JSON lists its leaf flags.
						if mask.count_ones() == 1 && bits & mask != ethnum::I256::ZERO {
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
						writer
							.interpreter
							.reserve(crate::memory::values_size(items.len()))?;
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
						writer
							.interpreter
							.reserve(crate::memory::values_size(entries.len()))?;
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

enum Task<'a> {
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
					if !self.names.iter().any(|(n, _)| *n == name) {
						self.names.push((name, schema));
					}
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
		let schema = self.resolve(value, schema);
		match kind(schema) {
			"None" => matches!(value, Value::None),
			"Boolean" => matches!(value, Value::Boolean(_)),
			"String" | "StringLiteral" => matches!(value, Value::String(_)),
			"Integer" | "Fields" => matches!(value, Value::Integer(_)),
			"Float" => matches!(value, Value::Float(_)),
			"Object" => matches!(value, Value::Object(_)),
			"Tuple" => matches!(value, Value::Tuple(_)),
			"Collection" => matches!(value, Value::Array(_) | Value::Set(_) | Value::Map(_)),
			"Enum" | "NamedUnion" => {
				matches!(value, Value::Member(m) if list_property(schema,"members").iter().any(|e| property(e,"name").and_then(Value::as_str) == m.name.rsplit('.').next()))
			}
			_ => false,
		}
	}

	fn fields(
		&mut self,
		pending: &mut Vec<Task<'s>>,
		fields: &'s [(std::sync::Arc<str>, Value)],
		schema: &'s Value,
		tagged: bool,
	) -> Result<(), Error> {
		self.interpreter
			.reserve(crate::memory::values_size(fields.len()))?;
		pending.push(Task::Raw("}"));
		let mut separator = false;
		for (name, value) in fields.iter().rev().filter(|(_, v)| !omitted(v)) {
			if separator {
				pending.push(Task::Raw(","));
			}
			let definition = list_property(schema, "properties")
				.iter()
				.find(|p| property(p, "name").and_then(Value::as_str) == Some(name))
				.and_then(|p| property(p, "definition"))
				.unwrap_or(&Value::None);
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
