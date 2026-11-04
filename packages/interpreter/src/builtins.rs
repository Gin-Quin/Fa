//! The built-in functions and methods, implemented natively (advanced brief
//! §12.6: not through the standard library's intrinsics), by prelude path.
//!
//! The subset of the prelude that compile-time code uses: strings, arrays,
//! sets, maps, numbers, ranges. A path the interpreter doesn't know is
//! [`Error::Unsupported`]; the analyzer lowers the impure ones (`console`,
//! the clock) to [`crate::ir::Expr::Impure`] instead.

use std::sync::Arc;

use ethnum::I256;
use unicode_segmentation::UnicodeSegmentation;

use crate::eval::{Error, Flow, Interpreter};
use crate::value::{Object, Value, contains, equals, map_get};

/// The number of grapheme clusters of a text: the `length` of a string.
pub fn grapheme_count(text: &str) -> usize {
	if text.is_ascii() {
		return text.len() - text.matches("\r\n").count();
	}
	text.graphemes(true).count()
}

fn stop(message: impl Into<String>) -> Error {
	Error::Stop(message.into())
}

fn index_error(index: I256, length: usize) -> Error {
	Error::Thrown(Value::error(
		"IndexError",
		vec![
			("index", Value::Integer(index)),
			("length", Value::int(length as i64)),
		],
	))
}

fn argument(arguments: &[Value], index: usize) -> Result<&Value, Error> {
	arguments
		.get(index)
		.ok_or_else(|| stop(format!("missing argument {index}")))
}

fn text(value: &Value) -> Result<&str, Error> {
	value
		.as_str()
		.ok_or_else(|| stop(format!("`{}` is not a string", value.to_text())))
}

fn integer(value: &Value) -> Result<I256, Error> {
	value
		.as_int()
		.ok_or_else(|| stop(format!("`{}` is not an integer", value.to_text())))
}

fn boolean(value: Value) -> Result<bool, Error> {
	value
		.as_bool()
		.ok_or_else(|| stop(format!("`{}` is not a `Boolean`", value.to_text())))
}

fn usize_of(value: I256) -> Option<usize> {
	usize::try_from(value.as_i128())
		.ok()
		.filter(|_| value >= I256::ZERO)
}

/// A tuple of two values (a map entry).
pub fn pair(value: Value) -> Result<(Value, Value), Error> {
	match value {
		Value::Tuple(items) if items.len() == 2 => Ok((items[0].clone(), items[1].clone())),
		other => Err(stop(format!("`{}` is not a pair", other.to_text()))),
	}
}

/// Inserts or replaces an entry of a map.
pub fn map_insert(entries: &mut Vec<(Value, Value)>, key: Value, value: Value) {
	match entries.iter_mut().find(|(other, _)| equals(other, &key)) {
		Some(entry) => entry.1 = value,
		None => entries.push((key, value)),
	}
}

/// The number of items [`iterate`] gives, when it is known without building them.
pub fn count(value: &Value) -> Option<usize> {
	match value {
		Value::Array(items) | Value::Set(items) | Value::Tuple(items) => Some(items.len()),
		Value::Map(entries) => Some(entries.len()),
		Value::String(text) => Some(text.len()),
		Value::Range {
			start: Some(start),
			end: Some(end),
			inclusive,
		} => {
			let end = if *inclusive { *end + I256::ONE } else { *end };
			let count = (end - *start).max(I256::ZERO);
			Some(usize::try_from(count.min(I256::from(u64::MAX))).unwrap_or(usize::MAX))
		}
		_ => None,
	}
}

/// The items a `for` loop or a spread goes through.
pub fn iterate(value: &Value) -> Result<Vec<Value>, Error> {
	match value {
		Value::Array(items) | Value::Set(items) | Value::Tuple(items) => Ok(items.to_vec()),
		Value::Map(entries) => Ok(entries
			.iter()
			.map(|(key, value)| Value::Tuple(Arc::new(vec![key.clone(), value.clone()])))
			.collect()),
		Value::String(text) => Ok(text
			.graphemes(true)
			.map(|grapheme| Value::String(Arc::from(grapheme)))
			.collect()),
		Value::Range {
			start,
			end: Some(end),
			inclusive,
		} => {
			let Some(start) = start else {
				return Err(stop("a range without a start can't be iterated"));
			};
			let end = if *inclusive { *end + I256::ONE } else { *end };
			let count = (end - *start).max(I256::ZERO);
			// the items are built at once: they must fit in the memory budget
			if count > I256::from(crate::eval::DEFAULT_MEMORY as u64)
				|| crate::memory::values_size(count.as_usize()) > crate::eval::DEFAULT_MEMORY
			{
				return Err(Error::Memory(crate::eval::DEFAULT_MEMORY));
			}
			let mut items = Vec::with_capacity(count.as_usize());
			let mut current = *start;
			while current < end {
				items.push(Value::Integer(current));
				current += I256::ONE;
			}
			Ok(items)
		}
		other => Err(stop(format!("`{}` can't be iterated", other.to_text()))),
	}
}

/// `value is in collection`.
pub fn includes(collection: &Value, value: &Value) -> Result<bool, Error> {
	match collection {
		Value::Array(items) | Value::Set(items) | Value::Tuple(items) => Ok(contains(items, value)),
		Value::Map(entries) => Ok(map_get(entries, value).is_some()),
		Value::String(haystack) => Ok(haystack.contains(text(value)?)),
		Value::Range {
			start,
			end,
			inclusive,
		} => {
			let Value::Integer(value) = value else {
				return Ok(false);
			};
			let above = start.is_none_or(|start| start <= *value);
			let below = end.is_none_or(|end| {
				if *inclusive {
					*value <= end
				} else {
					*value < end
				}
			});
			Ok(above && below)
		}
		other => Err(stop(format!("`{}` is not a collection", other.to_text()))),
	}
}

/// `value.field`.
pub fn field(value: &Value, name: &str) -> Result<Value, Error> {
	match value {
		Value::Object(object) => object
			.get(name)
			.cloned()
			.ok_or_else(|| stop(format!("`{}` has no field `{name}`", value.to_text()))),
		Value::Member(member) => match &member.data {
			Some(data) => field(data, name),
			None => Err(stop(format!("`{}` has no field `{name}`", value.to_text()))),
		},
		Value::String(text) if name == "length" => Ok(Value::int(grapheme_count(text) as i64)),
		Value::Array(items) if name == "length" => Ok(Value::int(items.len() as i64)),
		Value::Set(items) if name == "size" => Ok(Value::int(items.len() as i64)),
		Value::Map(entries) if name == "size" => Ok(Value::int(entries.len() as i64)),
		Value::Range {
			start,
			end,
			inclusive,
		} => match name {
			"start" => Ok(start.map_or(Value::None, Value::Integer)),
			"end" => Ok(end.map_or(Value::None, Value::Integer)),
			"includesEnd" => Ok(Value::Boolean(*inclusive)),
			_ => Err(stop(format!("a range has no field `{name}`"))),
		},
		Value::None => Err(stop(format!("`none` has no field `{name}`"))),
		other => Err(stop(format!("`{}` has no field `{name}`", other.to_text()))),
	}
}

/// `value.field`, to change it.
pub fn field_mut<'v>(value: &'v mut Value, name: &str) -> Result<&'v mut Value, Flow> {
	match value {
		Value::Object(object) => {
			let object = Arc::make_mut(object);
			if object.get(name).is_none() {
				object.fields.push((Arc::from(name), Value::None));
			}
			Ok(object.get_mut(name).expect("the field was just added"))
		}
		Value::Member(member) => match &mut Arc::make_mut(member).data {
			Some(data) => field_mut(data, name),
			None => Err(Flow::Error(stop(format!("no field `{name}`")))),
		},
		other => Err(Flow::Error(stop(format!(
			"`{}` has no field `{name}`",
			other.to_text()
		)))),
	}
}

fn position(index: I256, length: usize) -> Result<usize, Error> {
	usize_of(index)
		.filter(|&position| position < length)
		.ok_or_else(|| index_error(index, length))
}

/// The bounds of a slice `items[range]` over `length` items: from `-length`
/// to `length`, missing ones are the ends, an inverted range is empty.
fn slice_bounds(
	start: Option<I256>,
	end: Option<I256>,
	inclusive: bool,
	length: usize,
) -> Result<(usize, usize), Error> {
	let resolve = |bound: I256| -> Result<usize, Error> {
		let length_value = I256::from(length as u64);
		if bound < -length_value || bound > length_value {
			return Err(index_error(bound, length));
		}
		let resolved = if bound < I256::ZERO {
			bound + length_value
		} else {
			bound
		};
		Ok(resolved.as_usize())
	};
	let from = start.map_or(Ok(0), resolve)?;
	let to = match end {
		Some(end) => {
			let end = if inclusive { end + I256::ONE } else { end };
			resolve(end)?
		}
		None => length,
	};
	Ok((from, to.max(from)))
}

/// `value[index]`: an item, a map value, a character, a slice.
pub fn index(value: &Value, index: &Value) -> Result<Value, Error> {
	if let Value::Range {
		start,
		end,
		inclusive,
	} = index
	{
		return match value {
			Value::Array(items) => {
				let (from, to) = slice_bounds(*start, *end, *inclusive, items.len())?;
				Ok(Value::Array(Arc::new(items[from..to].to_vec())))
			}
			Value::String(text) => {
				let graphemes: Vec<&str> = text.graphemes(true).collect();
				let (from, to) = slice_bounds(*start, *end, *inclusive, graphemes.len())?;
				Ok(Value::String(Arc::from(graphemes[from..to].concat())))
			}
			other => Err(stop(format!("`{}` can't be sliced", other.to_text()))),
		};
	}
	match value {
		Value::Array(items) | Value::Tuple(items) => {
			let index = integer(index)?;
			Ok(items[position(index, items.len())?].clone())
		}
		Value::Map(entries) => map_get(entries, index).cloned().ok_or_else(|| {
			Error::Thrown(Value::error(
				"KeyNotFound",
				vec![("key", Value::String(Arc::from(index.to_text())))],
			))
		}),
		Value::String(text) => {
			let index = integer(index)?;
			let graphemes: Vec<&str> = text.graphemes(true).collect();
			let position = position(index, graphemes.len())?;
			Ok(Value::String(Arc::from(graphemes[position])))
		}
		other => Err(stop(format!("`{}` can't be indexed", other.to_text()))),
	}
}

/// `value[index]`, to change it (a map gets the key).
pub fn index_mut(value: &mut Value, key: Value) -> Result<&mut Value, Flow> {
	match value {
		Value::Array(items) | Value::Tuple(items) => {
			let items = Arc::make_mut(items);
			let index = integer(&key)?;
			let position = position(index, items.len())?;
			Ok(&mut items[position])
		}
		Value::Map(entries) => {
			let entries = Arc::make_mut(entries);
			let found = entries.iter().position(|(other, _)| equals(other, &key));
			let position = match found {
				Some(position) => position,
				None => {
					entries.push((key, Value::None));
					entries.len() - 1
				}
			};
			Ok(&mut entries[position].1)
		}
		other => Err(Flow::Error(stop(format!(
			"`{}` can't be indexed",
			other.to_text()
		)))),
	}
}

fn callback(
	interpreter: &mut Interpreter<'_>,
	function: &Value,
	arguments: Vec<Value>,
) -> Result<Value, Error> {
	interpreter.call_value(function, arguments)
}

fn list(items: Vec<Value>) -> Value {
	Value::Array(Arc::new(items))
}

fn sorted(
	interpreter: &mut Interpreter<'_>,
	mut items: Vec<Value>,
	compare: Option<&Value>,
) -> Result<Vec<Value>, Error> {
	// insertion sort: stable, and every comparison may call Fa code
	for next in 1..items.len() {
		let mut position = next;
		while position > 0 {
			let order = match compare {
				Some(function) => integer(&callback(
					interpreter,
					function,
					vec![items[position - 1].clone(), items[position].clone()],
				)?)?,
				None => {
					if crate::eval::compare(
						crate::ir::CompareOp::Greater,
						&items[position - 1],
						&items[position],
					)? {
						I256::ONE
					} else {
						I256::ZERO
					}
				}
			};
			if order <= I256::ZERO {
				break;
			}
			items.swap(position - 1, position);
			position -= 1;
		}
	}
	Ok(items)
}

fn sum(items: &[Value]) -> Result<Value, Error> {
	if items.iter().all(|item| matches!(item, Value::Integer(_))) {
		let mut total = I256::ZERO;
		for item in items {
			total = total
				.checked_add(integer(item)?)
				.ok_or_else(|| Error::Overflow("the sum overflows at compile time".into()))?;
		}
		return Ok(Value::Integer(total));
	}
	let mut total = 0.0;
	for item in items {
		total += item
			.as_f64()
			.ok_or_else(|| stop(format!("`{}` is not a number", item.to_text())))?;
	}
	Ok(Value::Float(total))
}

/// Integer.parse: an optional sign and ASCII digits.
fn parse_integer(text: &str) -> Result<Value, Error> {
	let parse_error = || {
		Error::Thrown(Value::error(
			"ParseError",
			vec![(
				"message",
				Value::String(Arc::from(format!("`{text}` is not an integer"))),
			)],
		))
	};
	let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
	if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
		return Err(parse_error());
	}
	let value: i128 = text.parse().map_err(|_| parse_error())?;
	if i64::try_from(value).is_err() {
		return Err(parse_error());
	}
	Ok(Value::Integer(I256::from(value)))
}

fn parse_float(text: &str) -> Result<Value, Error> {
	text.trim().parse::<f64>().map(Value::Float).map_err(|_| {
		Error::Thrown(Value::error(
			"ParseError",
			vec![(
				"message",
				Value::String(Arc::from(format!("`{text}` is not a number"))),
			)],
		))
	})
}

/// A built-in that doesn't change its arguments (a method's receiver first).
pub fn call(
	interpreter: &mut Interpreter<'_>,
	path: &str,
	arguments: Vec<Value>,
) -> Result<Value, Error> {
	let receiver = arguments.first().cloned().unwrap_or(Value::None);
	let arg = |index: usize| argument(&arguments, index);
	Ok(match path {
		// ── strings ──
		"String#length" => Value::int(grapheme_count(text(&receiver)?) as i64),
		// the room of a string, in bytes: exactly its bytes at compile time
		"String#capacity" => Value::int(text(&receiver)?.len() as i64),
		"String.from" => Value::String(Arc::from(text(&receiver)?)),
		// a value of the interpreter grows on demand: `reserve` is a hint
		_ if path.ends_with("#reserve") => Value::None,
		"String#isEmpty" => Value::Boolean(text(&receiver)?.is_empty()),
		"String#get" => {
			let index = integer(arg(1)?)?;
			let graphemes: Vec<&str> = text(&receiver)?.graphemes(true).collect();
			match usize_of(index).and_then(|position| graphemes.get(position)) {
				Some(grapheme) => Value::string(grapheme),
				None => Value::None,
			}
		}
		"String#includes" => Value::Boolean(text(&receiver)?.contains(text(arg(1)?)?)),
		"String#startsWith" => Value::Boolean(text(&receiver)?.starts_with(text(arg(1)?)?)),
		"String#endsWith" => Value::Boolean(text(&receiver)?.ends_with(text(arg(1)?)?)),
		"String#indexOf" | "String#lastIndexOf" => {
			let haystack = text(&receiver)?;
			let needle = text(arg(1)?)?;
			let found = if path == "String#indexOf" {
				haystack.find(needle)
			} else {
				haystack.rfind(needle)
			};
			match found {
				Some(byte) => Value::int(grapheme_count(&haystack[..byte]) as i64),
				None => Value::None,
			}
		}
		"String#toUpperCase" => Value::String(Arc::from(text(&receiver)?.to_uppercase())),
		"String#toLowerCase" => Value::String(Arc::from(text(&receiver)?.to_lowercase())),
		"String#trim" => Value::string(text(&receiver)?.trim()),
		"String#trimStart" => Value::string(text(&receiver)?.trim_start()),
		"String#trimEnd" => Value::string(text(&receiver)?.trim_end()),
		"String#padStart" | "String#padEnd" => {
			let value = text(&receiver)?;
			let length = usize_of(integer(arg(1)?)?).unwrap_or(0);
			let fill = match arguments.get(2) {
				Some(Value::String(fill)) => fill.to_string(),
				_ => " ".to_string(),
			};
			let current = grapheme_count(value);
			if current >= length || fill.is_empty() {
				Value::string(value)
			} else {
				interpreter.reserve_text(
					value
						.len()
						.saturating_add(length.saturating_mul(fill.len())),
				)?;
				let padding: String = fill.chars().cycle().take(length - current).collect();
				let padded = if path == "String#padStart" {
					format!("{padding}{value}")
				} else {
					format!("{value}{padding}")
				};
				Value::String(Arc::from(padded))
			}
		}
		"String#repeat" => {
			let count = usize_of(integer(arg(1)?)?).unwrap_or(0);
			let value = text(&receiver)?;
			interpreter.reserve_text(value.len().saturating_mul(count))?;
			Value::String(Arc::from(value.repeat(count)))
		}
		"String#replace" => Value::String(Arc::from(text(&receiver)?.replacen(
			text(arg(1)?)?,
			text(arg(2)?)?,
			1,
		))),
		"String#replaceAll" => {
			let (value, pattern, replacement) = (text(&receiver)?, text(arg(1)?)?, text(arg(2)?)?);
			let count = if pattern.is_empty() {
				value.chars().count() + 1
			} else {
				value.matches(pattern).count()
			};
			interpreter.reserve_text(
				value
					.len()
					.saturating_add(count.saturating_mul(replacement.len())),
			)?;
			Value::String(Arc::from(value.replace(pattern, replacement)))
		}
		"String#split" => {
			let separator = text(arg(1)?)?;
			let value = text(&receiver)?;
			let parts: Vec<Value> = if separator.is_empty() {
				value.graphemes(true).map(Value::string).collect()
			} else {
				value.split(separator).map(Value::string).collect()
			};
			list(parts)
		}
		"String#characters" => list(
			text(&receiver)?
				.graphemes(true)
				.map(Value::string)
				.collect(),
		),
		"String#codePoints" => list(
			text(&receiver)?
				.chars()
				.map(|c| Value::int(i64::from(u32::from(c))))
				.collect(),
		),
		"String#toString"
		| "Integer#toString"
		| "Float#toString"
		| "BigInt#toString"
		| "Percentage#toString"
		| "Enum#toString" => Value::String(Arc::from(receiver.to_text())),

		// ── numbers ──
		"Integer.parse" => parse_integer(text(&receiver)?)?,
		"Float.parse" => parse_float(text(&receiver)?)?,
		"Integer#abs" | "BigInt#abs" => Value::Integer(integer(&receiver)?.abs()),
		"Float#abs" | "Percentage#abs" => match receiver {
			Value::Percentage(value) => Value::Percentage(value.abs()),
			other => Value::Float(other.as_f64().unwrap_or(f64::NAN).abs()),
		},
		"Integer#clamp" => {
			let value = integer(&receiver)?;
			Value::Integer(value.clamp(integer(arg(1)?)?, integer(arg(2)?)?))
		}
		"Float#clamp" => {
			let value = receiver.as_f64().unwrap_or(f64::NAN);
			let low = arg(1)?.as_f64().unwrap_or(f64::NAN);
			let high = arg(2)?.as_f64().unwrap_or(f64::NAN);
			Value::Float(value.clamp(low, high))
		}
		"Float#sqrt" => Value::Float(receiver.as_f64().unwrap_or(f64::NAN).sqrt()),
		"Float#round" => {
			// halves away from zero
			Value::Float(receiver.as_f64().unwrap_or(f64::NAN).round())
		}
		"Float#floor" => Value::Float(receiver.as_f64().unwrap_or(f64::NAN).floor()),
		"Float#ceil" => Value::Float(receiver.as_f64().unwrap_or(f64::NAN).ceil()),
		"Float#truncate" => Value::Float(receiver.as_f64().unwrap_or(f64::NAN).trunc()),
		"Float#isNaN" => Value::Boolean(receiver.as_f64().is_some_and(f64::is_nan)),
		"Float#isFinite" => Value::Boolean(receiver.as_f64().is_some_and(f64::is_finite)),
		"Integer.round" | "Integer.floor" | "Integer.ceil" | "Integer.truncate" => {
			let value = receiver.as_f64().unwrap_or(f64::NAN);
			let rounded = match path {
				"Integer.round" => value.round(),
				"Integer.floor" => value.floor(),
				"Integer.ceil" => value.ceil(),
				_ => value.trunc(),
			};
			if !rounded.is_finite() || rounded.abs() > 9.2e18 {
				return Err(Error::Thrown(Value::error(
					"ArithmeticError",
					vec![(
						"message",
						Value::string("the value can't be converted to an integer"),
					)],
				)));
			}
			Value::Integer(I256::from(rounded as i64))
		}
		"Math.max" | "Math.min" => {
			let (a, b) = (arg(0)?.clone(), arg(1)?.clone());
			let greater = crate::eval::compare(crate::ir::CompareOp::Greater, &a, &b)?;
			if (path == "Math.max") == greater {
				a
			} else {
				b
			}
		}

		// ── arrays, sets, maps (reading) ──
		"Array#length" | "Array#capacity" => {
			Value::int(receiver.items().map_or(0, <[Value]>::len) as i64)
		}
		"Set#size" | "Set#capacity" => {
			Value::int(receiver.items().map_or(0, <[Value]>::len) as i64)
		}
		"Map#size" | "Map#capacity" => match &receiver {
			Value::Map(entries) => Value::int(entries.len() as i64),
			_ => Value::int(0),
		},
		"Array#isEmpty" | "Set#isEmpty" | "Map#isEmpty" => Value::Boolean(match &receiver {
			Value::Map(entries) => entries.is_empty(),
			other => other.items().is_none_or(<[Value]>::is_empty),
		}),
		"Array#get" => {
			let index = integer(arg(1)?)?;
			let items = receiver.items().unwrap_or(&[]);
			usize_of(index)
				.and_then(|position| items.get(position))
				.cloned()
				.unwrap_or(Value::None)
		}
		"Map#get" => match &receiver {
			Value::Map(entries) => map_get(entries, arg(1)?).cloned().unwrap_or(Value::None),
			_ => Value::None,
		},
		"Array#includes" | "Set#includes" | "Map#includes" => {
			Value::Boolean(includes(&receiver, arg(1)?)?)
		}
		"Array#indexOf" => {
			let value = arg(1)?;
			match receiver
				.items()
				.unwrap_or(&[])
				.iter()
				.position(|item| equals(item, value))
			{
				Some(position) => Value::int(position as i64),
				None => Value::None,
			}
		}
		"Array#find" | "Set#find" | "Map#find" => {
			let function = arg(1)?.clone();
			for item in iterate(&receiver)? {
				if boolean(callback(interpreter, &function, vec![item.clone()])?)? {
					return Ok(item);
				}
			}
			Value::None
		}
		"Array#filter" | "Set#filter" | "Map#filter" => {
			let function = arg(1)?.clone();
			let mut kept = Vec::new();
			for (position, item) in iterate(&receiver)?.into_iter().enumerate() {
				let arguments = vec![item.clone(), Value::int(position as i64)];
				if boolean(callback(interpreter, &function, arguments)?)? {
					kept.push(item);
				}
			}
			match receiver {
				Value::Set(_) => Value::Set(Arc::new(kept)),
				Value::Map(_) => {
					let mut entries = Vec::new();
					for item in kept {
						let (key, value) = pair(item)?;
						entries.push((key, value));
					}
					Value::Map(Arc::new(entries))
				}
				_ => list(kept),
			}
		}
		"Array#map" | "Set#map" | "Map#map" => {
			let function = arg(1)?.clone();
			let mut mapped = Vec::new();
			for (position, item) in iterate(&receiver)?.into_iter().enumerate() {
				let arguments = vec![item, Value::int(position as i64)];
				mapped.push(callback(interpreter, &function, arguments)?);
			}
			match receiver {
				Value::Set(_) => {
					let mut set = Vec::new();
					for item in mapped {
						if !contains(&set, &item) {
							set.push(item);
						}
					}
					Value::Set(Arc::new(set))
				}
				_ => list(mapped),
			}
		}
		"Array#every" | "Set#every" | "Map#every" | "Array#some" | "Set#some" | "Map#some" => {
			let every = path.ends_with("every");
			let function = arg(1)?.clone();
			for item in iterate(&receiver)? {
				if boolean(callback(interpreter, &function, vec![item])?)? != every {
					return Ok(Value::Boolean(!every));
				}
			}
			Value::Boolean(every)
		}
		"Array#reduce" | "Set#reduce" | "Map#reduce" => {
			let function = arg(1)?.clone();
			let mut total = arg(2)?.clone();
			for (position, item) in iterate(&receiver)?.into_iter().enumerate() {
				let arguments = vec![total, item, Value::int(position as i64)];
				total = callback(interpreter, &function, arguments)?;
			}
			total
		}
		"Array#sum" | "Set#sum" => sum(receiver.items().unwrap_or(&[]))?,
		"Array#reverse" => {
			let mut items = receiver.items().unwrap_or(&[]).to_vec();
			items.reverse();
			list(items)
		}
		"Array#sort" => {
			let items = receiver.items().unwrap_or(&[]).to_vec();
			list(sorted(interpreter, items, arguments.get(1))?)
		}
		"Set#union" | "Set#intersection" | "Set#difference" | "Set#isSubsetOf" => {
			let mine = receiver.items().unwrap_or(&[]).to_vec();
			let other = arg(1)?.items().unwrap_or(&[]).to_vec();
			match path {
				"Set#union" => {
					let mut union = mine;
					for item in other {
						if !contains(&union, &item) {
							union.push(item);
						}
					}
					Value::Set(Arc::new(union))
				}
				"Set#intersection" => Value::Set(Arc::new(
					mine.into_iter()
						.filter(|item| contains(&other, item))
						.collect(),
				)),
				"Set#difference" => Value::Set(Arc::new(
					mine.into_iter()
						.filter(|item| !contains(&other, item))
						.collect(),
				)),
				_ => Value::Boolean(mine.iter().all(|item| contains(&other, item))),
			}
		}
		"Array.from" => list(iterate(&receiver)?),
		"Set.from" => {
			let mut set = Vec::new();
			for item in iterate(&receiver)? {
				if !contains(&set, &item) {
					set.push(item);
				}
			}
			Value::Set(Arc::new(set))
		}
		"Map.from" => {
			let mut entries = Vec::new();
			for item in iterate(&receiver)? {
				let (key, value) = pair(item)?;
				map_insert(&mut entries, key, value);
			}
			Value::Map(Arc::new(entries))
		}
		"Range#start" | "Range#end" | "Range#includesEnd" => {
			field(&receiver, path.rsplit('#').next().unwrap_or(path))?
		}
		_ => return Err(Error::Unsupported(format!("`{path}`"))),
	})
}

/// Checks the memory budget before a push into a full collection, which
/// moves its items into a buffer twice as large.
fn reserve_growth(
	interpreter: &Interpreter<'_>,
	length: usize,
	capacity: usize,
) -> Result<(), Error> {
	if length == capacity {
		interpreter.reserve(crate::memory::values_size(
			capacity.max(4).saturating_mul(2),
		))?;
	}
	Ok(())
}

/// A built-in method changing its receiver in place (copied first only when
/// another value shares it): the method's result. On an error, the receiver
/// is unchanged.
pub fn mutate(
	interpreter: &mut Interpreter<'_>,
	path: &str,
	receiver: &mut Value,
	arguments: Vec<Value>,
) -> Result<Value, Error> {
	let arg = |index: usize| argument(&arguments, index).cloned();
	match receiver {
		Value::Array(items) => {
			let list = Arc::make_mut(items);
			let result = match path {
				"Array#push" => {
					let value = arg(0)?;
					reserve_growth(interpreter, list.len(), list.capacity())?;
					list.push(value);
					Value::None
				}
				"Array#pop" => list.pop().unwrap_or(Value::None),
				"Array#insert" => {
					let index = integer(&arg(0)?)?;
					let position = usize_of(index)
						.filter(|&position| position <= list.len())
						.ok_or_else(|| index_error(index, list.len()))?;
					list.insert(position, arg(1)?);
					Value::None
				}
				"Array#delete" | "Array#take" => {
					let index = integer(&arg(0)?)?;
					let position = position(index, list.len())?;
					let removed = list.remove(position);
					if path == "Array#take" {
						removed
					} else {
						Value::None
					}
				}
				"Array#clear" => {
					list.clear();
					Value::None
				}
				"Array#reverseInPlace" => {
					list.reverse();
					Value::None
				}
				"Array#sortInPlace" => {
					let sorted_items = sorted(interpreter, list.clone(), arguments.first())?;
					*list = sorted_items;
					Value::None
				}
				"Array#deleteWhere" => {
					let function = arg(0)?;
					let mut deleted = Vec::with_capacity(list.len());
					for item in list.iter() {
						deleted.push(boolean(callback(
							interpreter,
							&function,
							vec![item.clone()],
						)?)?);
					}
					let mut deleted = deleted.into_iter();
					list.retain(|_| !deleted.next().unwrap_or(false));
					Value::None
				}
				_ => return Err(Error::Unsupported(format!("`{path}`"))),
			};
			Ok(result)
		}
		Value::Set(items) => {
			let set = Arc::make_mut(items);
			let result = match path {
				"Set#add" => {
					let value = arg(0)?;
					let added = !contains(set, &value);
					if added {
						reserve_growth(interpreter, set.len(), set.capacity())?;
						set.push(value);
					}
					Value::Boolean(added)
				}
				"Set#delete" | "Set#take" => {
					let value = arg(0)?;
					match set.iter().position(|item| equals(item, &value)) {
						Some(position) => {
							let removed = set.remove(position);
							if path == "Set#take" {
								removed
							} else {
								Value::Boolean(true)
							}
						}
						None if path == "Set#take" => Value::None,
						None => Value::Boolean(false),
					}
				}
				"Set#clear" => {
					set.clear();
					Value::None
				}
				_ => return Err(Error::Unsupported(format!("`{path}`"))),
			};
			Ok(result)
		}
		Value::Map(entries) => {
			let map = Arc::make_mut(entries);
			let result = match path {
				"Map#delete" | "Map#take" => {
					let key = arg(0)?;
					match map.iter().position(|(other, _)| equals(other, &key)) {
						Some(position) => {
							let (_, value) = map.remove(position);
							if path == "Map#take" {
								value
							} else {
								Value::Boolean(true)
							}
						}
						None if path == "Map#take" => Value::None,
						None => Value::Boolean(false),
					}
				}
				"Map#getOrInsert" => {
					let key = arg(0)?;
					match map_get(map, &key) {
						Some(value) => value.clone(),
						None => {
							let value = arg(1)?;
							map.push((key, value.clone()));
							value
						}
					}
				}
				"Map#clear" => {
					map.clear();
					Value::None
				}
				_ => return Err(Error::Unsupported(format!("`{path}`"))),
			};
			Ok(result)
		}
		other => Err(Error::Unsupported(format!(
			"`{path}` on `{}`",
			other.to_text()
		))),
	}
}

/// An empty object (the start of a literal built field by field).
pub fn empty_object() -> Value {
	Value::Object(Arc::new(Object::default()))
}
