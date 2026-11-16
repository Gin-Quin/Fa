//! `@parseJson(text)` (advanced brief §12.5): JSON to a value whose exact
//! type the analyzer computes. Anonymous objects use name order, integral
//! numbers written without a fraction or an exponent are integers, other
//! numbers floats, `null` is `none`. Malformed JSON is a thrown
//! `ParseError` (`META005` at the call).

use std::sync::Arc;

use ethnum::I256;

use crate::eval::Error;
use crate::text::Text;
use crate::value::{Object, Value};

mod write;
pub(crate) use write::stringify;

/// Parses a JSON text.
pub fn parse(text: &str) -> Result<Value, Error> {
	parse_inner(text, false)
}

fn parse_inner(text: &str, maps: bool) -> Result<Value, Error> {
	let mut parser = Parser {
		bytes: text.as_bytes(),
		text,
		position: 0,
		depth: 0,
		maps,
	};
	parser.skip_whitespace();
	let value = parser.value()?;
	parser.skip_whitespace();
	if parser.position < parser.bytes.len() {
		return Err(parser.error("unexpected text after the value"));
	}
	Ok(value)
}

struct Parser<'t> {
	bytes: &'t [u8],
	text: &'t str,
	position: usize,
	depth: u32,
	maps: bool,
}

impl Parser<'_> {
	fn error(&self, message: &str) -> Error {
		let line = self.bytes[..self.position.min(self.bytes.len())]
			.iter()
			.filter(|b| **b == b'\n')
			.count() + 1;
		Error::Thrown(Value::error(
			"ParseError",
			vec![(
				"message",
				Value::String(format!("malformed JSON at line {line}: {message}").into()),
			)],
		))
	}

	fn peek(&self) -> Option<u8> {
		self.bytes.get(self.position).copied()
	}

	fn skip_whitespace(&mut self) {
		while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
			self.position += 1;
		}
	}

	fn expect(&mut self, byte: u8) -> Result<(), Error> {
		if self.peek() == Some(byte) {
			self.position += 1;
			Ok(())
		} else {
			Err(self.error(&format!("expected `{}`", byte as char)))
		}
	}

	fn keyword(&mut self, word: &str, value: Value) -> Result<Value, Error> {
		if self.bytes[self.position..].starts_with(word.as_bytes()) {
			self.position += word.len();
			Ok(value)
		} else {
			Err(self.error("unexpected character"))
		}
	}

	fn value(&mut self) -> Result<Value, Error> {
		self.depth += 1;
		if self.depth > 512 {
			return Err(self.error("too deeply nested"));
		}
		let value = match self.peek() {
			Some(b'{') => self.object(),
			Some(b'[') => self.array(),
			Some(b'"') => self.string().map(Value::String),
			Some(b't') => self.keyword("true", Value::Boolean(true)),
			Some(b'f') => self.keyword("false", Value::Boolean(false)),
			Some(b'n') => self.keyword("null", Value::None),
			Some(b'-' | b'0'..=b'9') => self.number(),
			Some(_) => Err(self.error("unexpected character")),
			None => Err(self.error("unexpected end of the text")),
		};
		self.depth -= 1;
		value
	}

	fn object(&mut self) -> Result<Value, Error> {
		self.expect(b'{')?;
		let mut fields = Vec::new();
		self.skip_whitespace();
		if self.peek() == Some(b'}') {
			self.position += 1;
			return self.object_value(fields);
		}
		loop {
			self.skip_whitespace();
			let key = self.string()?;
			self.skip_whitespace();
			self.expect(b':')?;
			self.skip_whitespace();
			let value = self.value()?;
			if let Some((_, slot)) = fields.iter_mut().find(|(name, _)| *name == key) {
				*slot = value;
			} else {
				fields.push((key, value));
			}
			self.skip_whitespace();
			match self.peek() {
				Some(b',') => self.position += 1,
				Some(b'}') => {
					self.position += 1;
					return self.object_value(fields);
				}
				_ => return Err(self.error("expected `,` or `}`")),
			}
		}
	}

	fn array(&mut self) -> Result<Value, Error> {
		self.expect(b'[')?;
		let mut items = Vec::new();
		self.skip_whitespace();
		if self.peek() == Some(b']') {
			self.position += 1;
			return Ok(Value::Array(Arc::new(items)));
		}
		loop {
			self.skip_whitespace();
			items.push(self.value()?);
			self.skip_whitespace();
			match self.peek() {
				Some(b',') => self.position += 1,
				Some(b']') => {
					self.position += 1;
					return Ok(Value::Array(Arc::new(items)));
				}
				_ => return Err(self.error("expected `,` or `]`")),
			}
		}
	}

	fn object_value(&self, fields: Vec<(Text, Value)>) -> Result<Value, Error> {
		if self.maps {
			return Ok(Value::Map(Arc::new(
				fields
					.into_iter()
					.map(|(key, v)| (Value::String(key), v))
					.collect(),
			)));
		}
		let mut fields = fields
			.into_iter()
			.map(|(key, v)| {
				let key = key.as_str().ok_or_else(|| {
					self.error("a compile-time object field name must be well-formed text")
				})?;
				Ok((Arc::<str>::from(key), v))
			})
			.collect::<Result<Vec<_>, Error>>()?;
		fields.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
		Ok(Value::Object(Arc::new(Object { fields })))
	}

	fn string(&mut self) -> Result<Text, Error> {
		self.expect(b'"')?;
		let mut out = Vec::new();
		loop {
			let start = self.position;
			while !matches!(self.peek(), Some(b'"' | b'\\') | None) {
				if self.peek().is_some_and(|byte| byte < 0x20) {
					return Err(self.error("control character in a string"));
				}
				self.position += 1;
			}
			out.extend(self.text[start..self.position].encode_utf16());
			match self.peek() {
				Some(b'"') => {
					self.position += 1;
					return Ok(Text::from_units(&out));
				}
				Some(b'\\') => {
					self.position += 1;
					let escape = self.peek().ok_or_else(|| self.error("unfinished escape"))?;
					self.position += 1;
					let unit = match escape {
						b'"' => 0x22,
						b'\\' => 0x5c,
						b'/' => 0x2f,
						b'b' => 8,
						b'f' => 12,
						b'n' => 10,
						b'r' => 13,
						b't' => 9,
						b'u' => self.hex4()? as u16,
						_ => return Err(self.error("unknown escape")),
					};
					out.push(unit);
				}
				_ => return Err(self.error("unfinished string")),
			}
		}
	}

	fn hex4(&mut self) -> Result<u32, Error> {
		let digits = self
			.text
			.get(self.position..self.position + 4)
			.ok_or_else(|| self.error("invalid unicode escape"))?;
		let code =
			u32::from_str_radix(digits, 16).map_err(|_| self.error("invalid unicode escape"))?;
		self.position += 4;
		Ok(code)
	}

	fn number(&mut self) -> Result<Value, Error> {
		let start = self.position;
		if self.peek() == Some(b'-') {
			self.position += 1;
		}
		let digits = |parser: &mut Self| {
			let start = parser.position;
			while parser.peek().is_some_and(|byte| byte.is_ascii_digit()) {
				parser.position += 1;
			}
			parser.position > start
		};
		if !digits(self) {
			return Err(self.error("expected a digit"));
		}
		let digits_start = start + usize::from(self.bytes[start] == b'-');
		if self.bytes[digits_start] == b'0' && self.position > digits_start + 1 {
			return Err(self.error("leading zero in a number"));
		}
		let mut integral = true;
		if self.peek() == Some(b'.') {
			self.position += 1;
			integral = false;
			if !digits(self) {
				return Err(self.error("expected a digit"));
			}
		}
		if matches!(self.peek(), Some(b'e' | b'E')) {
			self.position += 1;
			integral = false;
			if matches!(self.peek(), Some(b'+' | b'-')) {
				self.position += 1;
			}
			if !digits(self) {
				return Err(self.error("expected a digit"));
			}
		}
		let literal = &self.text[start..self.position];
		if integral && let Ok(value) = I256::from_str_radix(literal, 10) {
			return Ok(Value::Integer(value));
		}
		literal
			.parse::<f64>()
			.map(Value::Float)
			.map_err(|_| self.error("invalid number"))
	}
}

/// Supplies lexical captures for lazy field defaults. The schema itself
/// remains ordinary constant metadata, shared until this binding step.
pub(crate) fn bind_defaults(value: &mut Value, closures: &[Value]) {
	match value {
		Value::Function(function) => {
			if let Some(closure) = closures.iter().find(
				|value| matches!(value,Value::Function(other) if other.function == function.function),
			) {
				*value = closure.clone();
			}
		}
		Value::Array(items) | Value::Tuple(items) => {
			for item in Arc::make_mut(items) {
				bind_defaults(item, closures);
			}
		}
		Value::Object(object) => {
			for (_, field) in &mut Arc::make_mut(object).fields {
				bind_defaults(field, closures);
			}
		}
		Value::Member(member) => {
			if let Some(data) = &mut Arc::make_mut(member).data {
				bind_defaults(data, closures);
			}
		}
		_ => {}
	}
}

/// JSON inputs use the same parser on a borrowed UTF-8 view of bytes. A
/// malformed Fa string is escaped losslessly before parsing; no U+FFFD is
/// introduced into a string value by the parser.
fn read(input: &Value) -> Result<Value, Error> {
	let parsed = match input {
		Value::Bytes(bytes) => {
			let text = std::str::from_utf8(bytes)
				.map_err(|e| json_error(&format!("invalid UTF-8 at byte {}", e.valid_up_to())))?;
			parse_inner(text, true)
		}
		Value::String(text) => match text.as_str() {
			Some(text) => parse_inner(text, true),
			None => parse_inner(&escaped_input(text)?, true),
		},
		_ => return Err(json_error("expected a String or Bytes")),
	}
	.map_err(|error| match error {
		Error::Thrown(value) => json_error(&value.to_text()),
		other => other,
	})?;
	Ok(parsed)
}

/// A JSON conversion suspends when an omitted field needs its Fa default.
/// The machine calls that function on its ordinary explicit call stack.
/// Completed defaults are memoized by their input object and property identity
/// while conversion resumes; map keys may contain arbitrary code units.
pub(crate) struct Reader {
	value: Value,
	schema: Value,
	js: bool,
	defaults: Vec<((usize, usize), Value)>,
	pending: Option<(usize, usize)>,
}

pub(crate) enum ReadProgress {
	Call(Value),
	Done(Value),
}

enum Failure {
	Error(Error),
	Default(Box<((usize, usize), Value)>),
}
impl From<Error> for Failure {
	fn from(error: Error) -> Self {
		Self::Error(error)
	}
}

impl Reader {
	pub fn new(input: &Value, schema: Value, js: bool) -> Result<Self, Error> {
		Ok(Self {
			value: read(input)?,
			schema,
			js,
			defaults: Vec::new(),
			pending: None,
		})
	}
	pub fn advance(
		&mut self,
		returned: Option<Value>,
		mut step: impl FnMut() -> Result<(), Error>,
	) -> Result<ReadProgress, Error> {
		if let Some(path) = self.pending.take() {
			self.defaults
				.push((path, returned.expect("the JSON default returned")));
		}
		match convert(
			&self.value,
			&self.schema,
			self.js,
			"the value",
			ShapeContext {
				names: &[],
				owner: "",
				depth: 0,
			},
			&self.defaults,
			&mut step,
		) {
			Ok(value) => Ok(ReadProgress::Done(value)),
			Err(Failure::Error(error)) => Err(error),
			Err(Failure::Default(request)) => {
				let (path, function) = *request;
				self.pending = Some(path);
				Ok(ReadProgress::Call(function))
			}
		}
	}
}

fn escaped_input(text: &Text) -> Result<String, Error> {
	use std::fmt::Write as _;
	let mut output = String::new();
	let (mut quoted, mut escaped) = (false, false);
	for scalar in char::decode_utf16(text.units()) {
		match scalar {
			Ok(c) => {
				output.push(c);
				if escaped {
					escaped = false;
				} else if c == '\\' && quoted {
					escaped = true;
				} else if c == '"' {
					quoted = !quoted;
				}
			}
			Err(surrogate) => {
				if !quoted || escaped {
					return Err(json_error("unexpected lone surrogate in JSON"));
				}
				let _ = write!(output, "\\u{:04X}", surrogate.unpaired_surrogate());
			}
		}
	}
	Ok(output)
}

fn json_error(message: &str) -> Error {
	Error::Thrown(Value::error(
		"JsonError",
		vec![("message", Value::string(message))],
	))
}

fn property<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
	match value {
		Value::Object(object) => object.get(name),
		Value::Map(entries) => crate::value::map_get(entries, &Value::string(name)),
		Value::Member(member) => member.data.as_ref().and_then(|data| property(data, name)),
		_ => None,
	}
}

fn kind(schema: &Value) -> &str {
	match schema {
		Value::Member(member) => member.name.strip_prefix("Schema.").unwrap_or(""),
		_ => "",
	}
}

fn list_property<'a>(value: &'a Value, name: &str) -> &'a [Value] {
	property(value, name)
		.and_then(Value::items)
		.unwrap_or_default()
}

#[derive(Clone, Copy)]
struct ShapeContext<'a> {
	names: &'a [(&'a str, &'a Value)],
	owner: &'a str,
	depth: u32,
}

fn convert(
	value: &Value,
	schema: &Value,
	js: bool,
	at: &str,
	context: ShapeContext<'_>,
	defaults: &[((usize, usize), Value)],
	step: &mut dyn FnMut() -> Result<(), Error>,
) -> Result<Value, Failure> {
	step()?;
	let ShapeContext {
		names,
		owner,
		depth,
	} = context;
	if depth > 512 {
		return Err(json_error("JSON is nested too deeply").into());
	}
	let mut convert = |v, s, at: &str| {
		self::convert(
			v,
			s,
			js,
			at,
			ShapeContext {
				depth: depth + 1,
				..context
			},
			defaults,
			step,
		)
	};
	let field = |name| property(schema, name).unwrap_or(&Value::None);
	let fail = |expected: &str| {
		let found = match value {
			Value::None => "null",
			Value::Boolean(_) => "boolean",
			Value::Integer(_) | Value::Float(_) => "number",
			Value::String(_) => "string",
			Value::Array(_) => "an array",
			_ => "object",
		};
		Failure::Error(json_error(&format!(
			"{at} should be {expected}, not {found}"
		)))
	};
	Ok(match kind(schema) {
		"Named" => {
			let name = field("name").as_str().unwrap_or("");
			let definition = field("definition");
			let mut nested = names.to_vec();
			nested.push((name, definition));
			return self::convert(
				value,
				definition,
				js,
				at,
				ShapeContext {
					names: &nested,
					owner: name,
					depth: depth + 1,
				},
				defaults,
				step,
			);
		}
		"Reference" => {
			let name = field("name").as_str().unwrap_or("");
			let definition = names
				.iter()
				.rev()
				.find(|(n, _)| *n == name)
				.map(|(_, s)| *s)
				.ok_or_else(|| json_error("unknown recursive JSON type"))?;
			return convert(value, definition, at);
		}
		"String" if matches!(value, Value::String(_)) => value.clone(),
		"Boolean" if matches!(value, Value::Boolean(_)) => value.clone(),
		"None" if matches!(value, Value::None) => Value::None,
		"Float" => {
			let mut number = value.as_f64().ok_or_else(|| fail("a number"))?;
			if field("bits").as_int() == Some(32.into()) {
				number = number as f32 as f64;
			}
			Value::Float(number)
		}
		"Integer" => {
			let number = match value {
				Value::Integer(number) => *number,
				Value::Float(number) if number.is_finite() && number.fract() == 0.0 => {
					format!("{number:.0}")
						.parse::<I256>()
						.map_err(|_| fail("an integer in range"))?
				}
				_ => return Err(fail("an integer")),
			};
			let bits = field("bits").as_int().unwrap_or(64.into()).as_u32();
			let signed = field("signed").as_bool().unwrap_or(true);
			if js
				&& field("jsNumber").as_bool().unwrap_or(true)
				&& number.abs() > I256::from(9_007_199_254_740_991i64)
			{
				return Err(
					json_error(&format!("{at} doesn't fit in an Integer on JavaScript")).into(),
				);
			}
			let lo = if signed {
				-(I256::ONE << (bits - 1))
			} else {
				I256::ZERO
			};
			let hi = (I256::ONE << (bits - u32::from(signed))) - I256::ONE;
			let lo = field("min").as_int().unwrap_or(lo);
			let hi = field("max").as_int().unwrap_or(hi);
			if number < lo {
				return Err(
					json_error(&format!("{at} should be at least {lo}, not {number}")).into(),
				);
			}
			if number > hi {
				return Err(
					json_error(&format!("{at} should be at most {hi}, not {number}")).into(),
				);
			}
			Value::Integer(number)
		}
		"StringLiteral" | "IntegerLiteral" | "FloatLiteral" | "BooleanLiteral" => {
			if value != field("value") {
				return Err(fail(&field("value").to_notation()));
			}
			value.clone()
		}
		"Union" => {
			for member in list_property(schema, "items") {
				match convert(value, member, at) {
					Ok(value) => return Ok(value),
					Err(suspended @ Failure::Default(..)) => return Err(suspended),
					Err(Failure::Error(Error::Thrown(ref error))) if matches!(error,Value::Member(m) if m.name.as_ref() == "JsonError") =>
						{}
					Err(error) => return Err(error),
				}
			}
			return Err(fail("one of the members of its union"));
		}
		"Collection" => match field("kind").as_str().unwrap_or("") {
			"Array" | "GrowingArray" => {
				let Value::Array(items) = value else {
					return Err(fail("an array"));
				};
				Value::array(
					items
						.iter()
						.enumerate()
						.map(|(i, v)| convert(v, field("item"), &format!("{at}[{i}]")))
						.collect::<Result<_, _>>()?,
				)
			}
			"Map" | "GrowingMap" => {
				let Value::Map(entries) = value else {
					return Err(fail("an object"));
				};
				Value::Map(Arc::new(
					entries
						.iter()
						.map(|(k, v)| {
							Ok((
								k.clone(),
								convert(v, field("item"), &format!("{at}.{}", k.to_text()))?,
							))
						})
						.collect::<Result<_, Failure>>()?,
				))
			}
			_ => return Err(fail("a JSON collection")),
		},
		"Object" => {
			if !matches!(value, Value::Map(_)) {
				return Err(fail("an object"));
			}
			let mut object = Object::default();
			for field in list_property(schema, "properties") {
				let name = property(field, "name")
					.and_then(Value::as_str)
					.unwrap_or("");
				let ty = property(field, "definition").unwrap_or(&Value::None);
				let converted = match property(value, name) {
					Some(value) => convert(value, ty, &format!("{at}.{name}"))?,
					None => {
						if let Some(initial) = property(field, "initial") {
							if matches!(initial, Value::Function(_)) {
								let path = (
									value as *const Value as usize,
									field as *const Value as usize,
								);
								match defaults.iter().find(|(p, _)| *p == path) {
									Some((_, value)) => value.clone(),
									None => {
										return Err(Failure::Default(Box::new((
											path,
											initial.clone(),
										))));
									}
								}
							} else {
								initial.clone()
							}
						} else {
							match convert(&Value::None, ty, &format!("{at}.{name}")) {
								Ok(none) => none,
								Err(Failure::Error(Error::Thrown(_))) => {
									return Err(
										json_error(&format!("{at} has no field `{name}`")).into()
									);
								}
								Err(error) => return Err(error),
							}
						}
					}
				};
				object.set(Arc::from(name), converted);
			}
			Value::Object(Arc::new(object))
		}
		"NamedUnion" => {
			let members = list_property(schema, "members");
			for member in members {
				let definition = property(member, "definition").unwrap_or(&Value::None);
				if kind(definition).ends_with("Literal")
					&& let Ok(result) = convert(value, definition, at)
				{
					return Ok(result);
				}
			}
			let tag = property(value, "type")
				.and_then(Value::as_str)
				.ok_or_else(|| fail("one of the members of its union"))?;
			let member = members
				.iter()
				.find(|m| property(m, "name").and_then(Value::as_str) == Some(tag))
				.ok_or_else(|| fail("one of the members of its union"))?;
			let definition = property(member, "definition").unwrap_or(&Value::None);
			let definition = if kind(definition) == "Named" {
				property(definition, "definition").unwrap_or(definition)
			} else {
				definition
			};
			let data = if matches!(definition, Value::None) {
				None
			} else {
				let flat = kind(definition) == "Object"
					&& !list_property(definition, "properties")
						.iter()
						.any(|f| property(f, "name").and_then(Value::as_str) == Some("type"));
				let input = if flat {
					value
				} else {
					property(value, "value")
						.ok_or_else(|| json_error(&format!("{at} has no field `value`")))?
				};
				Some(convert(input, definition, at)?)
			};
			Value::Member(Arc::new(crate::value::Member {
				name: Arc::from(format!("{owner}.{tag}")),
				kind: if field("errors").as_bool() == Some(true) {
					crate::value::MemberKind::Error
				} else {
					crate::value::MemberKind::Union
				},
				data,
			}))
		}
		"Fields" => {
			let Value::Array(flags) = value else {
				return Err(fail("an array of flags"));
			};
			let mut bits = I256::ZERO;
			for flag in flags.iter() {
				let Some(found) = list_property(schema, "paths")
					.iter()
					.find(|entry| entry.items().and_then(|items| items.first()) == Some(flag))
				else {
					return Err(fail("a known flag"));
				};
				bits |= found
					.items()
					.and_then(|items| items.get(1))
					.and_then(Value::as_int)
					.unwrap_or(I256::ZERO);
			}
			Value::Integer(bits)
		}
		"Enum" => {
			let pinned = field("pinned").as_bool().unwrap_or(false);
			let member = list_property(schema, "members")
				.iter()
				.find(|m| {
					if pinned {
						property(m, "value") == Some(value)
					} else {
						property(m, "name") == Some(value)
					}
				})
				.ok_or_else(|| fail("one of the members of its enumeration"))?;
			let name = property(member, "name")
				.and_then(Value::as_str)
				.unwrap_or("");
			Value::Member(Arc::new(crate::value::Member {
				name: Arc::from(format!("{owner}.{name}")),
				kind: crate::value::MemberKind::Enumeration,
				data: None,
			}))
		}
		_ => {
			return Err(fail(match kind(schema) {
				"String" => "a string",
				"Boolean" => "a boolean",
				"None" => "null",
				_ => "a JSON value",
			}));
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn json_becomes_values() {
		let value = parse(
			r#"{"port": 8080, "debug": false, "ratio": 0.5, "tags": ["a", "é"], "none": null}"#,
		)
		.unwrap();
		assert_eq!(
			value.to_text(),
			"{ port = 8080, debug = false, ratio = 0.5, tags = [\"a\", \"é\"], none = none }"
		);
		assert!(matches!(parse("[1, 2"), Err(Error::Thrown(_))));
		assert!(matches!(parse("{} x"), Err(Error::Thrown(_))));
		assert_eq!(parse("-12").unwrap(), Value::int(-12));
		assert_eq!(parse("1e2").unwrap(), Value::Float(100.0));
	}
}
