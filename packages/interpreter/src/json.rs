//! `@parseJson(text)` (advanced brief §12.5): JSON to a value whose exact
//! type the analyzer computes. Objects keep their key order, integral
//! numbers written without a fraction or an exponent are integers, other
//! numbers floats, `null` is `none`. Malformed JSON is a thrown
//! `ParseError` (`META005` at the call).

use std::sync::Arc;

use ethnum::I256;

use crate::eval::Error;
use crate::value::{Object, Value};

/// Parses a JSON text.
pub fn parse(text: &str) -> Result<Value, Error> {
	let mut parser = Parser {
		bytes: text.as_bytes(),
		text,
		position: 0,
		depth: 0,
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
}

impl Parser<'_> {
	fn error(&self, message: &str) -> Error {
		let line = self.text[..self.position.min(self.text.len())]
			.matches('\n')
			.count() + 1;
		Error::Thrown(Value::error(
			"ParseError",
			vec![(
				"message",
				Value::String(Arc::from(format!(
					"malformed JSON at line {line}: {message}"
				))),
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
			Some(b'"') => self.string().map(|text| Value::String(Arc::from(text))),
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
		let mut object = Object::default();
		self.skip_whitespace();
		if self.peek() == Some(b'}') {
			self.position += 1;
			return Ok(Value::Object(Arc::new(object)));
		}
		loop {
			self.skip_whitespace();
			let key = self.string()?;
			self.skip_whitespace();
			self.expect(b':')?;
			self.skip_whitespace();
			let value = self.value()?;
			object.set(Arc::from(key), value);
			self.skip_whitespace();
			match self.peek() {
				Some(b',') => self.position += 1,
				Some(b'}') => {
					self.position += 1;
					return Ok(Value::Object(Arc::new(object)));
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

	fn string(&mut self) -> Result<String, Error> {
		self.expect(b'"')?;
		let mut out = String::new();
		loop {
			let start = self.position;
			while !matches!(self.peek(), Some(b'"' | b'\\') | None) {
				if self.peek().is_some_and(|byte| byte < 0x20) {
					return Err(self.error("control character in a string"));
				}
				self.position += 1;
			}
			out.push_str(&self.text[start..self.position]);
			match self.peek() {
				Some(b'"') => {
					self.position += 1;
					return Ok(out);
				}
				Some(b'\\') => {
					self.position += 1;
					let escape = self.peek().ok_or_else(|| self.error("unfinished escape"))?;
					self.position += 1;
					match escape {
						b'"' => out.push('"'),
						b'\\' => out.push('\\'),
						b'/' => out.push('/'),
						b'b' => out.push('\u{8}'),
						b'f' => out.push('\u{c}'),
						b'n' => out.push('\n'),
						b'r' => out.push('\r'),
						b't' => out.push('\t'),
						b'u' => {
							let mut code = self.hex4()?;
							// a surrogate pair
							if (0xD800..0xDC00).contains(&code)
								&& self.bytes[self.position..].starts_with(b"\\u")
							{
								self.position += 2;
								let low = self.hex4()?;
								code =
									0x10000 + ((code - 0xD800) << 10) + (low.wrapping_sub(0xDC00));
							}
							out.push(
								char::from_u32(code)
									.ok_or_else(|| self.error("invalid unicode escape"))?,
							);
						}
						_ => return Err(self.error("unknown escape")),
					}
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
