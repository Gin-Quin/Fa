//! The values of compile-time evaluation.
//!
//! Composite values use `Arc` storage with copy on write. A read may borrow
//! that storage; explicit Fa copies remain independent on mutation. Checked
//! last-use stores use `ir::Expr::Move`, transferring the value and clearing
//! its former slot so that a dead owner cannot force a later copy on write.
//! Values are `Send + Sync`: cached compile-time results can move between
//! analysis threads.
//!
//! Printing follows the runtime (`packages/codegen-js/runtime/format.js`): a
//! string at the top level is its text, nested it is quoted; integers and
//! floats print like JavaScript's `String(x)` (`20` for `20.0`); objects
//! print `{ x = 1, y = 2 }` in field order, members `Shape.Circle { … }`,
//! maps `[("a", 1)]`, sets `[1, 2]`, ranges `1..3`.

use std::fmt::Write as _;
use std::sync::Arc;

use ethnum::I256;

use crate::text::Text;

/// What a member value belongs to.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemberKind {
	/// A member of a named union (`Shape.Circle`).
	Union,
	/// A member of an enumeration (`Color.red`): prints as its name only.
	Enumeration,
	/// An error: a member of an `Errors` type, or a built-in error
	/// (`ParseError`, `IndexError`…).
	Error,
}

/// A member of a named union, an enumeration or an error type.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
	/// The qualified name (`Shape.Circle`, `ParseError`): its identity.
	pub name: Arc<str>,
	/// What it belongs to.
	pub kind: MemberKind,
	/// Its data (`Shape.Circle { radius = 1 }`, `UserId.User(12)`), if any.
	pub data: Option<Value>,
}

/// An object: its fields in declaration order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object {
	/// The fields, in declaration order (the order they print in).
	pub fields: Vec<(Arc<str>, Value)>,
}

impl Object {
	/// The value of a field.
	pub fn get(&self, name: &str) -> Option<&Value> {
		self.fields
			.iter()
			.find(|(field, _)| &**field == name)
			.map(|(_, value)| value)
	}

	/// The value of a field, to change it.
	pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
		self.fields
			.iter_mut()
			.find(|(field, _)| &**field == name)
			.map(|(_, value)| value)
	}

	/// Sets a field, adding it at the end when the object doesn't have it.
	pub fn set(&mut self, name: Arc<str>, value: Value) {
		match self.get_mut(&name) {
			Some(slot) => *slot = value,
			None => self.fields.push((name, value)),
		}
	}
}

/// A function value: a lowered function and the values it captured.
#[derive(Clone, Debug)]
pub struct Closure {
	/// The function.
	pub function: crate::ir::FnKey,
	/// The captured values, copied when the closure was created, in the
	/// order of the function's `captures`.
	pub captured: Vec<Value>,
	/// Arguments given in advance (`f with (y = 0)`), by parameter index.
	pub bound: Vec<(usize, Value)>,
	/// Machine-local captures are never published as cached values.
	pub(crate) environment: Option<Vec<crate::loans::Loan>>,
}

impl Closure {
	/// An owned environment, portable between evaluations.
	pub fn new(
		function: crate::ir::FnKey,
		captured: Vec<Value>,
		bound: Vec<(usize, Value)>,
	) -> Self {
		Self {
			function,
			captured,
			bound,
			environment: None,
		}
	}
}

/// Stable evaluator slots. Removing an element never shifts a surviving place.
#[derive(Clone, Debug, Default)]
pub struct Pool {
	pub slots: Vec<Option<Value>>,
	pub len: usize,
}

impl Pool {
	pub fn new(values: Vec<Value>) -> Self {
		Self {
			len: values.len(),
			slots: values.into_iter().map(Some).collect(),
		}
	}

	pub fn values(&self) -> impl Iterator<Item = &Value> {
		self.slots.iter().filter_map(Option::as_ref)
	}
}

/// A compile-time value.
#[derive(Clone, Debug)]
pub enum Value {
	/// `none` (also the value of a statement).
	None,
	/// `true`, `false`.
	Boolean(bool),
	/// Any integer (`Integer`, `Int8`…, `BigInt`): its range was checked by
	/// the operation that produced it.
	Integer(I256),
	/// `Float` (a `Float32` is rounded after every operation).
	Float(f64),
	/// `Percentage`: the stored fraction (`50%` is `0.5`).
	Percentage(f64),
	/// Text.
	String(Text),
	/// A fixed-length binary sequence, distinct from an array.
	Bytes(Arc<Vec<u8>>),
	/// `Array`, `GrowingArray`.
	Array(Arc<Vec<Value>>),
	/// Stable pool/columnar slots, with holes after deletion.
	Pool(Arc<Pool>),
	/// A tuple.
	Tuple(Arc<Vec<Value>>),
	/// An object.
	Object(Arc<Object>),
	/// A member of a named union, an enumeration or an error type.
	Member(Arc<Member>),
	/// `Map`, `GrowingMap`: entries in insertion order.
	Map(Arc<Vec<(Value, Value)>>),
	/// `Set`, `GrowingSet`: items in insertion order.
	Set(Arc<Vec<Value>>),
	/// `a..b`, `a..=b`, open bounds.
	Range {
		start: Option<I256>,
		end: Option<I256>,
		inclusive: bool,
	},
	/// A function.
	Function(Arc<Closure>),
}

impl PartialEq for Value {
	fn eq(&self, other: &Self) -> bool {
		equals(self, other)
	}
}

/// Fa's `==`: structural, numbers by value (`NaN != NaN`), sets and maps as
/// collections (the order doesn't matter), functions by identity of their code.
pub fn equals(a: &Value, b: &Value) -> bool {
	use Value::*;
	match (a, b) {
		(None, None) => true,
		(Boolean(a), Boolean(b)) => a == b,
		(Integer(a), Integer(b)) => a == b,
		(Float(a), Float(b)) | (Percentage(a), Percentage(b)) => a == b,
		(Integer(a), Float(b)) | (Float(b), Integer(a)) => a.as_f64() == *b,
		(String(a), String(b)) => a == b,
		(Bytes(a), Bytes(b)) => a == b,
		(Array(a), Array(b)) | (Tuple(a), Tuple(b)) => {
			a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| equals(a, b))
		}
		(Object(a), Object(b)) => {
			a.fields.len() == b.fields.len()
				&& a.fields
					.iter()
					.all(|(name, value)| b.get(name).is_some_and(|other| equals(value, other)))
		}
		(Member(a), Member(b)) => {
			a.name == b.name
				&& match (&a.data, &b.data) {
					(Option::None, Option::None) => true,
					(Some(a), Some(b)) => equals(a, b),
					_ => false,
				}
		}
		(Set(a), Set(b)) => a.len() == b.len() && a.iter().all(|item| contains(b, item)),
		(Pool(a), Pool(b)) => {
			if a.len != b.len {
				return false;
			}
			let mut unmatched: Vec<_> = b.values().collect();
			for value in a.values() {
				let Some(index) = unmatched.iter().position(|other| equals(value, other)) else {
					return false;
				};
				unmatched.swap_remove(index);
			}
			true
		}
		(Map(a), Map(b)) => {
			a.len() == b.len()
				&& a.iter()
					.all(|(key, value)| map_get(b, key).is_some_and(|other| equals(value, other)))
		}
		(
			Range {
				start: a,
				end: b,
				inclusive: c,
			},
			Range {
				start: d,
				end: e,
				inclusive: f,
			},
		) => a == d && b == e && c == f,
		(Function(a), Function(b)) => a.function == b.function,
		_ => false,
	}
}

/// Whether a collection's items include `value` (`==`).
pub fn contains(items: &[Value], value: &Value) -> bool {
	items.iter().any(|item| equals(item, value))
}

/// The value of a key in a map's entries.
pub fn map_get<'v>(entries: &'v [(Value, Value)], key: &Value) -> Option<&'v Value> {
	entries
		.iter()
		.find(|(other, _)| equals(other, key))
		.map(|(_, value)| value)
}

impl Value {
	/// A string value.
	pub fn string(text: &str) -> Value {
		Value::String(Text::from(text))
	}

	/// An integer value.
	pub fn int(value: i64) -> Value {
		Value::Integer(I256::from(value))
	}

	/// An array value.
	pub fn array(items: Vec<Value>) -> Value {
		Value::Array(Arc::new(items))
	}

	/// An object value, fields in order.
	pub fn object(fields: Vec<(&str, Value)>) -> Value {
		Value::Object(Arc::new(Object {
			fields: fields
				.into_iter()
				.map(|(name, value)| (Arc::from(name), value))
				.collect(),
		}))
	}

	/// An error value (a built-in error or an `Errors` member).
	pub fn error(name: &str, fields: Vec<(&str, Value)>) -> Value {
		let data = (!fields.is_empty()).then(|| Value::object(fields));
		Value::Member(Arc::new(Member {
			name: Arc::from(name),
			kind: MemberKind::Error,
			data,
		}))
	}

	/// The boolean, if the value is one.
	pub fn as_bool(&self) -> Option<bool> {
		match self {
			Value::Boolean(value) => Some(*value),
			_ => None,
		}
	}

	/// The integer, if the value is one.
	pub fn as_int(&self) -> Option<I256> {
		match self {
			Value::Integer(value) => Some(*value),
			_ => None,
		}
	}

	/// The number as a float (integers converted).
	pub fn as_f64(&self) -> Option<f64> {
		match self {
			Value::Integer(value) => Some(value.as_f64()),
			Value::Float(value) | Value::Percentage(value) => Some(*value),
			_ => None,
		}
	}

	/// The well-formed text of a string, without copying.
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Value::String(text) => text.as_str(),
			_ => None,
		}
	}

	/// The items of an array, a tuple or a set.
	pub fn items(&self) -> Option<&[Value]> {
		match self {
			Value::Array(items) | Value::Tuple(items) | Value::Set(items) => Some(items),
			_ => None,
		}
	}

	/// Whether the value contains a function (it can't be a constant of
	/// the output then).
	pub fn has_function(&self) -> bool {
		match self {
			Value::Function(_) => true,
			Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
				items.iter().any(Value::has_function)
			}
			Value::Object(object) => object.fields.iter().any(|(_, value)| value.has_function()),
			Value::Member(member) => member.data.as_ref().is_some_and(Value::has_function),
			Value::Map(entries) => entries
				.iter()
				.any(|(key, value)| key.has_function() || value.has_function()),
			_ => false,
		}
	}

	/// Whether a constant needs an emitter that understands raw UTF-16 units.
	pub fn has_lone_surrogates(&self) -> bool {
		match self {
			Value::String(text) => !text.is_well_formed(),
			Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
				items.iter().any(Self::has_lone_surrogates)
			}
			Value::Object(object) => object
				.fields
				.iter()
				.any(|(_, value)| value.has_lone_surrogates()),
			Value::Member(member) => member.data.as_ref().is_some_and(Self::has_lone_surrogates),
			Value::Map(entries) => entries
				.iter()
				.any(|(key, value)| key.has_lone_surrogates() || value.has_lone_surrogates()),
			_ => false,
		}
	}

	/// The internal `toString`, before an output boundary replaces lone
	/// surrogates. Formatting a composite must preserve its nested strings.
	pub fn to_text_value(&self) -> Text {
		match self {
			Value::String(text) => text.clone(),
			_ => {
				let mut out = TextBuffer::default();
				format_nested(self, &mut out);
				out.finish()
			}
		}
	}

	/// An upper bound for formatting, checked before allocating a template.
	pub fn text_size(&self) -> usize {
		match self {
			Value::String(text) => text.utf8_len().saturating_mul(6).saturating_add(2),
			Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
				items.iter().fold(2usize, |size, v| {
					size.saturating_add(v.text_size()).saturating_add(2)
				})
			}
			Value::Bytes(bytes) => bytes.len().saturating_mul(5).saturating_add(2),
			Value::Object(object) => object.fields.iter().fold(4usize, |size, (name, v)| {
				size.saturating_add(name.len())
					.saturating_add(v.text_size())
					.saturating_add(5)
			}),
			Value::Member(member) => member
				.name
				.len()
				.saturating_add(member.data.as_ref().map_or(0, Self::text_size))
				.saturating_add(4),
			Value::Map(entries) => entries.iter().fold(2usize, |size, (k, v)| {
				size.saturating_add(k.text_size())
					.saturating_add(v.text_size())
					.saturating_add(6)
			}),
			_ => 256,
		}
	}

	/// Text for an output: lone surrogates become U+FFFD here.
	pub fn to_text(&self) -> String {
		match self {
			Value::String(text) => text.to_utf8_lossy().into_owned(),
			_ => {
				let mut out = TextBuffer::default();
				format_nested(self, &mut out);
				out.output()
			}
		}
	}

	/// The Fa notation of the value, for an output or diagnostic.
	pub fn to_notation(&self) -> String {
		let mut out = TextBuffer::default();
		format_nested(self, &mut out);
		out.output()
	}
}

/// A float like JavaScript's `String(x)`: no fraction for an integral value,
/// exponents beyond `1e21` and below `1e-6`.
pub fn format_float(value: f64) -> String {
	if value.is_nan() {
		return "NaN".into();
	}
	if value.is_infinite() {
		return if value > 0.0 { "Infinity" } else { "-Infinity" }.into();
	}
	if value == 0.0 {
		return "0".into();
	}
	let magnitude = value.abs();
	if (1e-6..1e21).contains(&magnitude) {
		return format!("{value}");
	}
	// shortest digits, then JavaScript's exponent notation
	let text = format!("{value:e}");
	let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
	let exponent: i32 = exponent.parse().unwrap_or(0);
	let sign = if exponent < 0 { "-" } else { "+" };
	format!("{mantissa}e{sign}{}", exponent.abs())
}

/// A string in Fa literal notation (`faQuote` of the runtime).
pub fn quote(text: &str) -> String {
	let mut out = String::with_capacity(text.len() + 2);
	out.push('"');
	for c in text.chars() {
		quote_character(c, &mut out);
	}
	out.push('"');
	out
}

fn quote_character(c: char, out: &mut impl std::fmt::Write) {
	let escaped = match c {
		'\\' => Some("\\\\"),
		'"' => Some("\\\""),
		'\n' => Some("\\n"),
		'\r' => Some("\\r"),
		'\t' => Some("\\t"),
		'\0' => Some("\\0"),
		'{' => Some("{{"),
		'}' => Some("}}"),
		_ => None,
	};
	if let Some(escaped) = escaped {
		let _ = out.write_str(escaped);
	} else if (c as u32) < 0x20 || c as u32 == 0x7f {
		let _ = write!(out, "\\x{:02X}", c as u32);
	} else {
		let _ = out.write_char(c);
	}
}

/// Ordinary formatting stays UTF-8. The first raw unit switches to UTF-16;
/// the buffer is canonicalized only when complete, so joined pairs survive.
#[derive(Default)]
struct TextBuffer {
	text: String,
	units: Option<Vec<u16>>,
}

impl TextBuffer {
	fn push_str(&mut self, text: &str) {
		if let Some(units) = &mut self.units {
			units.extend(text.encode_utf16());
		} else {
			self.text.push_str(text);
		}
	}
	fn push(&mut self, c: char) {
		let mut buffer = [0; 4];
		self.push_str(c.encode_utf8(&mut buffer));
	}
	fn push_unit(&mut self, unit: u16) {
		self.units
			.get_or_insert_with(|| self.text.encode_utf16().collect())
			.push(unit);
	}
	fn quote(&mut self, text: &Text) {
		self.push('"');
		for scalar in char::decode_utf16(text.units()) {
			match scalar {
				Ok(c) => quote_character(c, self),
				Err(surrogate) => self.push_unit(surrogate.unpaired_surrogate()),
			}
		}
		self.push('"');
	}
	fn output(self) -> String {
		match self.units {
			Some(units) => String::from_utf16_lossy(&units),
			None => self.text,
		}
	}
	fn finish(self) -> Text {
		match self.units {
			Some(units) => Text::from_units(&units),
			None => Text::from(self.text),
		}
	}
}
impl std::fmt::Write for TextBuffer {
	fn write_str(&mut self, text: &str) -> std::fmt::Result {
		self.push_str(text);
		Ok(())
	}
}

fn format_list(items: &[Value], out: &mut TextBuffer) {
	out.push('[');
	for (index, item) in items.iter().enumerate() {
		if index > 0 {
			out.push_str(", ");
		}
		format_nested(item, out);
	}
	out.push(']');
}

fn format_object(object: &Object, out: &mut TextBuffer) {
	if object.fields.is_empty() {
		out.push_str("{}");
		return;
	}
	out.push_str("{ ");
	for (index, (name, value)) in object.fields.iter().enumerate() {
		if index > 0 {
			out.push_str(", ");
		}
		out.push_str(name);
		out.push_str(" = ");
		format_nested(value, out);
	}
	out.push_str(" }");
}

fn format_nested(value: &Value, out: &mut TextBuffer) {
	match value {
		Value::None => out.push_str("none"),
		Value::Boolean(value) => out.push_str(if *value { "true" } else { "false" }),
		Value::Integer(value) => {
			let _ = write!(out, "{value}");
		}
		Value::Float(value) => out.push_str(&format_float(*value)),
		Value::Percentage(value) => {
			out.push_str(&format_float(value * 100.0));
			out.push('%');
		}
		Value::String(text) => out.quote(text),
		Value::Bytes(bytes) => {
			out.push('[');
			for (index, byte) in bytes.iter().enumerate() {
				if index > 0 {
					out.push_str(", ");
				}
				let _ = write!(out, "{byte}");
			}
			out.push(']');
		}
		Value::Array(items) | Value::Set(items) => format_list(items, out),
		Value::Pool(pool) => format_list(&pool.values().cloned().collect::<Vec<_>>(), out),
		Value::Tuple(items) => {
			out.push('(');
			for (index, item) in items.iter().enumerate() {
				if index > 0 {
					out.push_str(", ");
				}
				format_nested(item, out);
			}
			out.push(')');
		}
		Value::Object(object) => format_object(object, out),
		Value::Member(member) => {
			let name = match member.kind {
				MemberKind::Enumeration => member.name.rsplit('.').next().unwrap_or(&member.name),
				_ => &member.name,
			};
			out.push_str(name);
			match &member.data {
				Some(Value::Object(object)) => {
					out.push(' ');
					format_object(object, out);
				}
				Some(data) => {
					out.push('(');
					format_nested(data, out);
					out.push(')');
				}
				None => {}
			}
		}
		Value::Map(entries) => {
			out.push('[');
			for (index, (key, value)) in entries.iter().enumerate() {
				if index > 0 {
					out.push_str(", ");
				}
				out.push('(');
				format_nested(key, out);
				out.push_str(", ");
				format_nested(value, out);
				out.push(')');
			}
			out.push(']');
		}
		Value::Range {
			start,
			end,
			inclusive,
		} => {
			if let Some(start) = start {
				let _ = write!(out, "{start}");
			}
			out.push_str(if *inclusive { "..=" } else { ".." });
			if let Some(end) = end {
				let _ = write!(out, "{end}");
			}
		}
		Value::Function(_) => out.push_str("<function>"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn values_print_like_the_runtime() {
		assert_eq!(Value::string("a").to_text(), "a");
		assert_eq!(
			Value::array(vec![Value::string("a"), Value::int(1)]).to_text(),
			"[\"a\", 1]"
		);
		assert_eq!(
			Value::object(vec![("x", Value::int(1)), ("y", Value::Float(2.5))]).to_text(),
			"{ x = 1, y = 2.5 }"
		);
		assert_eq!(Value::Float(20.0).to_text(), "20");
		assert_eq!(Value::Float(0.1 + 0.2).to_text(), "0.30000000000000004");
		assert_eq!(Value::Float(1e21).to_text(), "1e+21");
		assert_eq!(Value::Float(1.5e-7).to_text(), "1.5e-7");
		assert_eq!(
			Value::error("ParseError", vec![("message", Value::string("bad"))]).to_text(),
			"ParseError { message = \"bad\" }"
		);
		assert_eq!(quote("a{b}\n"), "\"a{{b}}\\n\"");
	}

	#[test]
	fn equality_is_structural() {
		let a = Value::object(vec![("x", Value::int(1)), ("y", Value::int(2))]);
		let b = Value::object(vec![("y", Value::int(2)), ("x", Value::int(1))]);
		assert_eq!(a, b);
		let s1 = Value::Set(Arc::new(vec![Value::int(1), Value::int(2)]));
		let s2 = Value::Set(Arc::new(vec![Value::int(2), Value::int(1)]));
		assert_eq!(s1, s2);
		assert_ne!(Value::Float(f64::NAN), Value::Float(f64::NAN));
	}
}
