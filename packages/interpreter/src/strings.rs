//! String/bytes builtins. All positions are UTF-16 units; UTF-8 is used
//! only for well-formed fast paths and at explicit encoding boundaries.

use std::sync::Arc;

use ethnum::I256;
use unicode_segmentation::UnicodeSegmentation;

use crate::builtins::position;
use crate::eval::{Error, Interpreter};
use crate::text::Text;
use crate::value::Value;

fn stop(message: &str) -> Error {
	Error::Stop(message.into())
}

fn string(value: &Value) -> Result<&Text, Error> {
	match value {
		Value::String(text) => Ok(text),
		_ => Err(stop("expected a string")),
	}
}

fn bytes(value: &Value) -> Result<&[u8], Error> {
	match value {
		Value::Bytes(bytes) => Ok(bytes),
		_ => Err(stop("expected Bytes")),
	}
}

fn number(value: &Value) -> Result<I256, Error> {
	value.as_int().ok_or_else(|| stop("expected an integer"))
}

fn size(value: &Value) -> Result<usize, Error> {
	let n = number(value)?;
	Ok(if n <= I256::ZERO {
		0
	} else {
		usize::try_from(n).unwrap_or(usize::MAX)
	})
}

/// Units, their UTF-8 conversion, and the final shared buffer can coexist.
fn reserve_units(interpreter: &Interpreter<'_>, length: usize) -> Result<(), Error> {
	interpreter.reserve(length.saturating_mul(10))
}

/// The ECMAScript whitespace set, independent of Rust's `is_whitespace`
/// (which includes U+0085 and excludes the byte order mark U+FEFF).
fn whitespace(unit: u16) -> bool {
	matches!(unit, 0x0009..=0x000d | 0x0020 | 0x00a0 | 0x1680 | 0x2000..=0x200a
		| 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff)
}

fn trim(text: &Text, start: bool, end: bool) -> Text {
	let from = if start {
		text.units().take_while(|u| whitespace(*u)).count()
	} else {
		0
	};
	let to = if end {
		text.units()
			.enumerate()
			.filter(|(_, u)| !whitespace(*u))
			.map(|(i, _)| i + 1)
			.last()
			.unwrap_or(0)
	} else {
		text.len()
	};
	text.slice(from, to.max(from))
}

fn case(text: &Text, upper: bool) -> Text {
	let map = |s: &str| {
		if upper {
			s.to_uppercase()
		} else {
			s.to_lowercase()
		}
	};
	if let Some(text) = text.as_str() {
		return map(text).into();
	}
	// A lone surrogate separates runs of Unicode text; keep it verbatim.
	let mut units = Vec::new();
	let mut run = String::new();
	for scalar in char::decode_utf16(text.units()) {
		match scalar {
			Ok(c) => run.push(c),
			Err(surrogate) => {
				units.extend(map(&run).encode_utf16());
				run.clear();
				units.push(surrogate.unpaired_surrogate());
			}
		}
	}
	units.extend(map(&run).encode_utf16());
	Text::from_units(&units)
}

fn list(items: Vec<Value>) -> Value {
	Value::array(items)
}

pub(crate) fn call(
	interpreter: &mut Interpreter<'_>,
	path: &str,
	args: &[Value],
) -> Result<Value, Error> {
	let arg = |i| args.get(i).ok_or_else(|| stop("missing builtin argument"));
	// Bytes constructors and decoders don't have a string receiver.
	match path {
		"Bytes.from" => {
			let items = arg(0)?.items().ok_or_else(|| stop("expected an array"))?;
			interpreter.reserve(items.len())?;
			let bytes = items
				.iter()
				.map(|v| {
					number(v).and_then(|n| u8::try_from(n).map_err(|_| stop("expected a Byte")))
				})
				.collect::<Result<Vec<_>, _>>()?;
			return Ok(Value::Bytes(Arc::new(bytes)));
		}
		"Bytes.filled" => {
			let length = size(arg(0)?)?;
			interpreter.reserve(length)?;
			let fill = args.get(1).map(number).transpose()?.unwrap_or(I256::ZERO);
			let fill = u8::try_from(fill).map_err(|_| stop("expected a Byte"))?;
			return Ok(Value::Bytes(Arc::new(vec![fill; length])));
		}
		"String.fromUtf8" | "String.fromUtf8Lossy" => {
			let input = bytes(arg(0)?)?;
			interpreter.reserve_text(input.len().saturating_mul(3))?;
			let text = if path == "String.fromUtf8Lossy" {
				String::from_utf8_lossy(input)
			} else {
				std::borrow::Cow::Borrowed(std::str::from_utf8(input).map_err(|e| {
					Error::Thrown(Value::error(
						"Utf8Error",
						vec![("position", Value::int(e.valid_up_to() as i64))],
					))
				})?)
			};
			return Ok(Value::string(&text));
		}
		_ => {}
	}
	let receiver = arg(0)?;
	if path.starts_with("Bytes#") {
		let input = bytes(receiver)?;
		return Ok(match path {
			"Bytes#length" => Value::int(input.len() as i64),
			"Bytes#isEmpty" => Value::Boolean(input.is_empty()),
			"Bytes#get" => position(number(arg(1)?)?, input.len())
				.ok()
				.map(|i| Value::int(i64::from(input[i])))
				.unwrap_or(Value::None),
			"Bytes#toArray" => {
				interpreter.reserve(crate::memory::values_size(input.len()))?;
				list(input.iter().map(|b| Value::int(i64::from(*b))).collect())
			}
			"Bytes#toString" => {
				interpreter.reserve_text(input.len().saturating_mul(5).saturating_add(2))?;
				Value::string(&receiver.to_text())
			}
			_ => return Err(Error::Unsupported(format!("`{path}`"))),
		});
	}
	let text = string(receiver)?;
	Ok(match path {
		"String#length" | "String#capacity" => Value::int(text.len() as i64),
		"String.from" | "String#toString" => receiver.clone(),
		"String#isEmpty" => Value::Boolean(text.is_empty()),
		"String#isWellFormed" => Value::Boolean(text.is_well_formed()),
		"String#codePointCount" => Value::int(text.code_point_count() as i64),
		"String#utf8Length" => Value::int(text.utf8_len() as i64),
		"String#get" => position(number(arg(1)?)?, text.len())
			.ok()
			.map(|i| Value::String(text.slice(i, i + 1)))
			.unwrap_or(Value::None),
		"String#codeUnitAt" => {
			let i = position(number(arg(1)?)?, text.len())?;
			Value::int(i64::from(text.unit(i).expect("checked index")))
		}
		"String#includes" | "String#indexOf" | "String#lastIndexOf" => {
			let needle = string(arg(1)?)?;
			if !text.is_well_formed() || !needle.is_well_formed() {
				interpreter.reserve(text.len().saturating_add(needle.len()).saturating_mul(2))?;
			}
			let found = text.find(needle, path == "String#lastIndexOf");
			if path == "String#includes" {
				Value::Boolean(found.is_some())
			} else {
				found.map(|i| Value::int(i as i64)).unwrap_or(Value::None)
			}
		}
		"String#startsWith" | "String#endsWith" => {
			let needle = string(arg(1)?)?;
			let found = if let (Some(text), Some(needle)) = (text.as_str(), needle.as_str()) {
				if path == "String#startsWith" {
					text.starts_with(needle)
				} else {
					text.ends_with(needle)
				}
			} else {
				let offset = if path == "String#endsWith" {
					text.len().saturating_sub(needle.len())
				} else {
					0
				};
				needle.len() <= text.len()
					&& text
						.units()
						.skip(offset)
						.zip(needle.units())
						.all(|(a, b)| a == b)
			};
			Value::Boolean(found)
		}
		"String#toUpperCase" | "String#toLowerCase" => {
			reserve_units(interpreter, text.len().saturating_mul(3))?;
			Value::String(case(text, path == "String#toUpperCase"))
		}
		"String#trim" | "String#trimStart" | "String#trimEnd" => {
			reserve_units(interpreter, text.len())?;
			Value::String(trim(
				text,
				path != "String#trimEnd",
				path != "String#trimStart",
			))
		}
		"String#repeat" => {
			let count = size(arg(1)?)?;
			if text.is_empty() || count == 0 {
				Value::string("")
			} else if let Some(s) = text.as_str() {
				interpreter.reserve_text(s.len().saturating_mul(count))?;
				Value::String(Text::from_utf8(
					Arc::from(s.repeat(count)),
					text.len() * count,
				))
			} else {
				let length = text.len().saturating_mul(count);
				reserve_units(interpreter, length)?;
				let units: Vec<u16> = text.units().cycle().take(length).collect();
				Value::String(Text::from_units(&units))
			}
		}
		"String#padStart" | "String#padEnd" => {
			let target = size(arg(1)?)?;
			let default = Text::from(" ");
			let filler = args.get(2).map(string).transpose()?.unwrap_or(&default);
			if target <= text.len() || filler.is_empty() {
				receiver.clone()
			} else {
				let needed = target - text.len();
				if text.is_ascii() && filler.is_ascii() {
					interpreter.reserve_text(target)?;
					let fill = filler.as_str().expect("ASCII");
					let mut padding = fill.repeat(needed / fill.len());
					padding.push_str(&fill[..needed % fill.len()]);
					let pieces = [Text::from_utf8(Arc::from(padding), needed), text.clone()];
					Value::String(if path == "String#padStart" {
						Text::concat(&pieces)
					} else {
						Text::concat(&[pieces[1].clone(), pieces[0].clone()])
					})
				} else {
					reserve_units(interpreter, target)?;
					let padding = filler.units().cycle().take(needed);
					let units: Vec<u16> = if path == "String#padStart" {
						padding.chain(text.units()).collect()
					} else {
						text.units().chain(padding).collect()
					};
					Value::String(Text::from_units(&units))
				}
			}
		}
		"String#split" => split(interpreter, text, string(arg(1)?)?)?,
		"String#replace" | "String#replaceAll" => replace(
			interpreter,
			text,
			string(arg(1)?)?,
			string(arg(2)?)?,
			path == "String#replaceAll",
		)?,
		"String#characters" => {
			interpreter.reserve_text(text.utf8_len())?;
			let view = text.to_utf8_lossy();
			let count = view.graphemes(true).count();
			interpreter.reserve(
				crate::memory::values_size(count)
					.saturating_add(count.saturating_mul(32))
					.saturating_add(text.len().saturating_mul(10)),
			)?;
			let mut parts = Vec::with_capacity(count);
			if text.is_well_formed() {
				parts.extend(view.graphemes(true).map(Value::string));
			} else {
				// U+FFFD and a lone surrogate have GCB=Other and one code unit.
				let mut offset = 0;
				parts.extend(view.graphemes(true).map(|g| {
					let start = offset;
					offset += g.encode_utf16().count();
					Value::String(text.slice(start, offset))
				}));
			}
			list(parts)
		}
		"String#codePoints" => {
			interpreter.reserve(crate::memory::values_size(text.code_point_count()))?;
			let mut offset = 0;
			let mut points = Vec::with_capacity(text.code_point_count());
			while let Some(code) = text.next_scalar(&mut offset) {
				points.push(Value::int(i64::from(code)));
			}
			list(points)
		}
		"String#toUtf8" => {
			interpreter.reserve(text.utf8_len().saturating_mul(2))?;
			Value::Bytes(Arc::new(text.to_utf8_lossy().as_bytes().to_vec()))
		}
		"String.compareLocale" => {
			return Err(Error::Unsupported(
				"`String.compareLocale`: the result depends on the locale data of the machine that runs the program".into(),
			));
		}
		_ => return Err(Error::Unsupported(format!("`{path}`"))),
	})
}

fn split(interpreter: &Interpreter<'_>, text: &Text, separator: &Text) -> Result<Value, Error> {
	if separator.len() > text.len() {
		interpreter.reserve(crate::memory::values_size(1))?;
		return Ok(list(vec![Value::String(text.clone())]));
	}
	let count = if separator.is_empty() {
		text.len()
	} else if let (Some(text), Some(separator)) = (text.as_str(), separator.as_str()) {
		text.matches(separator).count().saturating_add(1)
	} else {
		text.len().saturating_add(1)
	};
	interpreter.reserve(
		crate::memory::values_size(count)
			.saturating_add(count.saturating_mul(32))
			.saturating_add(text.len().saturating_mul(10)),
	)?;
	if separator.is_empty() {
		let mut parts = Vec::with_capacity(count);
		parts.extend(text.units().map(|u| Value::String(Text::from_units(&[u]))));
		return Ok(list(parts));
	}
	if let (Some(text), Some(separator)) = (text.as_str(), separator.as_str()) {
		let ascii = text.is_ascii();
		let mut parts = Vec::with_capacity(count);
		parts.extend(text.split(separator).map(|part| {
			if ascii {
				Value::String(Text::from_utf8(Arc::from(part), part.len()))
			} else {
				Value::string(part)
			}
		}));
		return Ok(list(parts));
	}
	let mut units = Vec::with_capacity(text.len());
	units.extend(text.units());
	let mut needle = Vec::with_capacity(separator.len());
	needle.extend(separator.units());
	let mut parts = Vec::with_capacity(count);
	let (mut start, mut i) = (0, 0);
	while i + needle.len() <= units.len() {
		if units[i..].starts_with(&needle) {
			parts.push(Value::String(Text::from_units(&units[start..i])));
			i += needle.len();
			start = i;
		} else {
			i += 1;
		}
	}
	parts.push(Value::String(Text::from_units(&units[start..])));
	Ok(list(parts))
}

fn replace(
	interpreter: &Interpreter<'_>,
	text: &Text,
	pattern: &Text,
	replacement: &Text,
	all: bool,
) -> Result<Value, Error> {
	// Literal replacement, including '$' characters: Fa has no regex API.
	if text.find(pattern, false).is_none() {
		return Ok(Value::String(text.clone()));
	}
	let count = if all { text.len().saturating_add(1) } else { 1 };
	let maximum = text
		.len()
		.saturating_add(count.saturating_mul(replacement.len()));
	reserve_units(interpreter, maximum.saturating_add(pattern.len()))?;
	if (!pattern.is_empty() || text.is_ascii())
		&& let (Some(text), Some(pattern), Some(replacement)) =
			(text.as_str(), pattern.as_str(), replacement.as_str())
	{
		let output = if all {
			text.replace(pattern, replacement)
		} else {
			text.replacen(pattern, replacement, 1)
		};
		return Ok(Value::String(
			if text.is_ascii() && replacement.is_ascii() {
				let length = output.len();
				Text::from_utf8(Arc::from(output), length)
			} else {
				output.into()
			},
		));
	}
	let mut units = Vec::with_capacity(text.len());
	units.extend(text.units());
	let mut needle = Vec::with_capacity(pattern.len());
	needle.extend(pattern.units());
	let mut output = Vec::with_capacity(maximum);
	let mut i = 0;
	let mut replaced = false;
	while i <= units.len() {
		if (all || !replaced) && units[i..].starts_with(&needle) {
			output.extend(replacement.units());
			replaced = true;
			if !needle.is_empty() {
				i += needle.len();
				continue;
			}
		}
		if let Some(unit) = units.get(i) {
			output.push(*unit);
		} else {
			break;
		}
		i += 1;
	}
	Ok(Value::String(Text::from_units(&output)))
}
