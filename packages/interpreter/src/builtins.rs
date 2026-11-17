//! The built-in functions and methods, implemented natively (advanced brief
//! §12.6: not through the standard library's intrinsics), by prelude path.
//!
//! The subset of the prelude that compile-time code uses: strings, arrays,
//! sets, maps, numbers, ranges. A path the interpreter doesn't know is
//! [`Error::Unsupported`]; the analyzer lowers the impure ones (`console`,
//! the clock) to [`crate::ir::Expr::Impure`] instead.

use std::sync::Arc;

use ethnum::I256;

use crate::eval::{Error, Flow, Interpreter};
use crate::value::{Object, Value, contains, equals, key_equals, map_get, set_contains};

fn stop(message: impl Into<String>) -> Error {
	Error::Stop(message.into())
}

pub(crate) fn index_error(index: I256, length: usize) -> Error {
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

fn usize_of(value: I256) -> Option<usize> {
	usize::try_from(value).ok().filter(|_| value >= I256::ZERO)
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
	match entries
		.iter_mut()
		.find(|(other, _)| key_equals(other, &key))
	{
		Some(entry) => entry.1 = value,
		None => entries.push((key, value)),
	}
}

/// The number of items [`iterate`] gives, when it is known without building them.
pub fn count(value: &Value) -> Option<usize> {
	match value {
		Value::Array(items) | Value::Set(items) | Value::Tuple(items) => Some(items.len()),
		Value::Pool(pool) => Some(pool.len),
		Value::Map(entries) => Some(entries.len()),
		Value::String(text) => Some(text.len()),
		Value::Bytes(bytes) => Some(bytes.len()),
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
		Value::Pool(pool) => Ok(pool.values().cloned().collect()),
		Value::Map(entries) => Ok(entries
			.iter()
			.map(|(key, value)| Value::Tuple(Arc::new(vec![key.clone(), value.clone()])))
			.collect()),
		Value::String(text) => {
			let mut offset = 0;
			let mut items = Vec::with_capacity(text.code_point_count());
			while let Some((point, _)) = text.next_point(&mut offset) {
				items.push(Value::String(point));
			}
			Ok(items)
		}
		Value::Bytes(bytes) => Ok(bytes.iter().map(|b| Value::int(i64::from(*b))).collect()),
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
		Value::Array(items) | Value::Tuple(items) => Ok(contains(items, value)),
		Value::Set(items) => Ok(set_contains(items, value)),
		Value::Pool(pool) => Ok(pool.values().any(|item| equals(item, value))),
		Value::Map(entries) => Ok(map_get(entries, value).is_some()),
		Value::String(haystack) => match value {
			Value::String(needle) => Ok(haystack.find(needle, false).is_some()),
			_ => Ok(false),
		},
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
		Value::String(text) if name == "length" => Ok(Value::int(text.len() as i64)),
		Value::Bytes(bytes) if name == "length" => Ok(Value::int(bytes.len() as i64)),
		Value::Array(items) if name == "length" => Ok(Value::int(items.len() as i64)),
		Value::Pool(pool) if name == "size" || name == "capacity" => {
			Ok(Value::int(pool.len as i64))
		}
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

pub(crate) fn position(index: I256, length: usize) -> Result<usize, Error> {
	let resolved = if index < I256::ZERO {
		index + I256::from(length as u64)
	} else {
		index
	};
	usize_of(resolved)
		.filter(|&position| position < length)
		.ok_or_else(|| index_error(index, length))
}

/// The bounds of a slice `items[range]` over `length` items: from `-length`
/// to `length`, missing ones are the ends, an inverted range is empty.
pub(crate) fn slice_bounds(
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
			let bound = resolve(end)?;
			if inclusive {
				bound
					.checked_add(1)
					.filter(|end| *end <= length)
					.ok_or_else(|| index_error(end, length))?
			} else {
				bound
			}
		}
		None => length,
	};
	Ok((from, to.max(from)))
}

/// The storage an indexed read will allocate, before constructing it.
pub fn index_size(value: &Value, index: &Value) -> Result<usize, Error> {
	if let Value::Range {
		start,
		end,
		inclusive,
	} = index
	{
		let length = match value {
			Value::String(text) => text.len(),
			Value::Bytes(bytes) => bytes.len(),
			Value::Array(items) => items.len(),
			_ => return Ok(0),
		};
		let (from, to) = slice_bounds(*start, *end, *inclusive, length)?;
		let count = to - from;
		return Ok(match value {
			Value::String(text) => count.saturating_mul(if text.is_ascii() { 2 } else { 10 }),
			Value::Bytes(_) => count,
			Value::Array(_) => crate::memory::values_size(count),
			_ => 0,
		});
	}
	Ok(if matches!(value, Value::String(_)) {
		64
	} else {
		0
	})
}

/// The buffers a spread or collection constructor builds while iterating.
pub fn iteration_size(value: &Value) -> usize {
	let count = count(value).unwrap_or(0);
	match value {
		Value::String(text) => crate::memory::values_size(count)
			.saturating_add(count.saturating_mul(32))
			.saturating_add(text.utf8_len().saturating_mul(2)),
		Value::Map(_) => crate::memory::values_size(count.saturating_mul(3))
			.saturating_add(count.saturating_mul(32)),
		_ => crate::memory::values_size(count),
	}
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
				let (from, to) = slice_bounds(*start, *end, *inclusive, text.len())?;
				Ok(Value::String(text.slice(from, to)))
			}
			Value::Bytes(bytes) => {
				let (from, to) = slice_bounds(*start, *end, *inclusive, bytes.len())?;
				Ok(Value::Bytes(Arc::new(bytes[from..to].to_vec())))
			}
			other => Err(stop(format!("`{}` can't be sliced", other.to_text()))),
		};
	}
	if matches!(value, Value::Object(_) | Value::Member(_))
		&& let Some(path) = index.as_str()
	{
		let mut selected = value.clone();
		for name in path.split('.') {
			selected = field(&selected, name)?;
		}
		return Ok(selected);
	}
	match value {
		Value::Pool(pool) => {
			let index = integer(index)?;
			pool.slots
				.get(position(index, pool.slots.len())?)
				.and_then(Option::as_ref)
				.cloned()
				.ok_or_else(|| index_error(index, pool.slots.len()))
		}
		Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
			let index = integer(index)?;
			Ok(items[position(index, items.len())?].clone())
		}
		Value::Map(entries) => map_get(entries, index).cloned().ok_or_else(|| {
			Error::Thrown(Value::error(
				"KeyNotFound",
				vec![("key", Value::string(&index.to_text()))],
			))
		}),
		Value::String(text) => {
			let index = position(integer(index)?, text.len())?;
			Ok(Value::String(text.slice(index, index + 1)))
		}
		Value::Bytes(bytes) => Ok(Value::int(i64::from(
			bytes[position(integer(index)?, bytes.len())?],
		))),
		other => Err(stop(format!("`{}` can't be indexed", other.to_text()))),
	}
}

/// `value[index]`, to change it (a map gets the key).
pub fn index_mut(value: &mut Value, key: Value) -> Result<&mut Value, Flow> {
	match value {
		Value::Pool(pool) => {
			let pool = Arc::make_mut(pool);
			let index = integer(&key)?;
			let count = pool.slots.len();
			pool.slots
				.get_mut(position(index, count)?)
				.and_then(Option::as_mut)
				.ok_or_else(|| index_error(index, count).into())
		}
		Value::Array(items) | Value::Tuple(items) => {
			let items = Arc::make_mut(items);
			let index = integer(&key)?;
			let position = position(index, items.len())?;
			Ok(&mut items[position])
		}
		Value::Map(entries) => {
			let entries = Arc::make_mut(entries);
			let found = entries
				.iter()
				.position(|(other, _)| key_equals(other, &key));
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

/// Writes a selected item or a borrowed slice back into its owner. Binary
/// storage stays compact instead of allocating a `Value` for every byte.
pub fn set_index(value: &mut Value, key: &Value, selected: Value) -> Result<(), Error> {
	match (value, key, selected) {
		(
			Value::Bytes(bytes),
			Value::Range {
				start,
				end,
				inclusive,
			},
			Value::Bytes(source),
		) => {
			let (from, to) = slice_bounds(*start, *end, *inclusive, bytes.len())?;
			if to - from != source.len() {
				return Err(stop("a byte slice keeps its length"));
			}
			Arc::make_mut(bytes)[from..to].copy_from_slice(&source);
		}
		(
			Value::Array(items),
			Value::Range {
				start,
				end,
				inclusive,
			},
			Value::Array(source),
		) => {
			let (from, to) = slice_bounds(*start, *end, *inclusive, items.len())?;
			if to - from != source.len() {
				return Err(stop("a borrowed slice keeps its length"));
			}
			Arc::make_mut(items)[from..to].clone_from_slice(&source);
		}
		(Value::Bytes(bytes), key, Value::Integer(byte)) => {
			let index = position(integer(key)?, bytes.len())?;
			let byte = u8::try_from(byte).map_err(|_| stop("expected a Byte"))?;
			Arc::make_mut(bytes)[index] = byte;
		}
		_ => return Err(stop("this item cannot be assigned")),
	}
	Ok(())
}

/// The paths of the flags of a `Fields` value (`names`: the path of each bit).
fn fields_flags(bits: I256, names: &Value) -> Vec<Value> {
	names
		.items()
		.unwrap_or_default()
		.iter()
		.enumerate()
		.filter(|(index, _)| (bits >> *index as u32) & I256::ONE == I256::ONE)
		.map(|(_, name)| name.clone())
		.collect()
}

/// A set of flags as a key set, from the dotted paths of its flags (a
/// group's flags are next to each other): `{ read, nested { a, b } }`.
fn format_fields(paths: &[&str]) -> String {
	if paths.is_empty() {
		return "{}".to_string();
	}
	let mut entries: Vec<String> = Vec::new();
	let mut index = 0;
	while index < paths.len() {
		match paths[index].split_once('.') {
			None => {
				entries.push(paths[index].to_string());
				index += 1;
			}
			Some((group, _)) => {
				let prefix = format!("{group}.");
				let members: Vec<&str> = paths[index..]
					.iter()
					.take_while(|path| path.starts_with(&prefix))
					.map(|path| &path[prefix.len()..])
					.collect();
				index += members.len();
				entries.push(format!("{group} {}", format_fields(&members)));
			}
		}
	}
	format!("{{ {} }}", entries.join(", "))
}

fn list(items: Vec<Value>) -> Value {
	Value::Array(Arc::new(items))
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
				Value::string(&format!("`{text}` is not an integer")),
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
				Value::string(&format!("`{text}` is not a number")),
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
	if crate::callbacks::Callback::handles(path) {
		return interpreter.callback_builtin(path, arguments);
	}

	let receiver = arguments.first().cloned().unwrap_or(Value::None);
	let arg = |index: usize| argument(&arguments, index);
	Ok(match path {
		"Function#bind" => {
			let Value::Function(mut function) = receiver else {
				return Err(stop("only a function can bind generic arguments"));
			};
			Arc::make_mut(&mut function)
				.bound
				.extend(arguments.iter().skip(1).cloned().enumerate());
			Value::Function(function)
		}
		"Json.stringify" => crate::json::stringify(interpreter, &receiver, arg(1)?)?,
		"$Json.bindDefaults" => {
			interpreter.reserve(receiver.text_size().saturating_mul(2))?;
			let mut schema = receiver;
			crate::json::bind_defaults(&mut schema, arg(1)?.items().unwrap_or_default());
			schema
		}
		// String operations preserve raw code units, including lone surrogates.
		_ if path.starts_with("String") || path.starts_with("Bytes") => {
			return crate::strings::call(interpreter, path, &arguments);
		}
		"Integer#toString"
		| "Float#toString"
		| "BigInt#toString"
		| "Percentage#toString"
		| "Enum#toString" => Value::string(&receiver.to_text()),

		// ── `Fields` values: their bits (the lowering's own paths) ──
		"Fields#union" => Value::Integer(integer(&receiver)? | integer(arg(1)?)?),
		"Fields#difference" => Value::Integer(integer(&receiver)? & !integer(arg(1)?)?),
		// `value is in set`: the set first
		"Fields#includes" => {
			let (set, value) = (integer(&receiver)?, integer(arg(1)?)?);
			Value::Boolean(set & value == value)
		}
		// the paths of the flags of the set (`names`: the path of each bit)
		"Fields#toStringArray" => list(fields_flags(integer(&receiver)?, arg(1)?)),
		// the set as a key set: `{ read, nested { a } }`
		"Fields#format" => {
			let flags = fields_flags(integer(&receiver)?, arg(1)?);
			let paths: Vec<&str> = flags.iter().filter_map(Value::as_str).collect();
			Value::string(&format_fields(&paths))
		}

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
		"Array#join" => {
			let items = receiver.items().unwrap_or(&[]);
			let separator = match arguments.get(1) {
				Some(Value::String(text)) => text.clone(),
				_ => crate::text::Text::from(","),
			};
			let size = items
				.iter()
				.fold(0usize, |n, v| {
					n.saturating_add(match v {
						Value::String(text) => text.utf8_len(),
						_ => 0,
					})
				})
				.saturating_add(
					separator
						.utf8_len()
						.saturating_mul(items.len().saturating_sub(1)),
				);
			interpreter.reserve_text(size.saturating_mul(3))?;
			interpreter.reserve(
				items
					.len()
					.saturating_mul(std::mem::size_of::<crate::text::Text>())
					.saturating_mul(2),
			)?;
			let mut pieces = Vec::with_capacity(items.len().saturating_mul(2));
			for (i, item) in items.iter().enumerate() {
				if i > 0 {
					pieces.push(separator.clone());
				}
				match item {
					Value::String(text) => pieces.push(text.clone()),
					_ => return Err(stop("expected a string")),
				}
			}
			Value::String(crate::text::Text::concat(&pieces))
		}
		"Array#length"
		| "Array#capacity"
		| "Pool#size"
		| "Pool#capacity"
		| "Columnar#size"
		| "Columnar#capacity"
		| "OrderedColumnar#length"
		| "OrderedColumnar#capacity" => Value::int(count(&receiver).unwrap_or(0) as i64),
		"Set#size" | "Set#capacity" => {
			Value::int(receiver.items().map_or(0, <[Value]>::len) as i64)
		}
		"Map#size" | "Map#capacity" => match &receiver {
			Value::Map(entries) => Value::int(entries.len() as i64),
			_ => Value::int(0),
		},
		"Array#isEmpty"
		| "Set#isEmpty"
		| "Map#isEmpty"
		| "Pool#isEmpty"
		| "Columnar#isEmpty"
		| "OrderedColumnar#isEmpty" => Value::Boolean(count(&receiver).unwrap_or(0) == 0),
		"Array#get" => {
			let index = integer(arg(1)?)?;
			let items = receiver.items().unwrap_or(&[]);
			position(index, items.len())
				.ok()
				.and_then(|position| items.get(position))
				.cloned()
				.unwrap_or(Value::None)
		}
		"Map#get" => match &receiver {
			Value::Map(entries) => map_get(entries, arg(1)?).cloned().unwrap_or(Value::None),
			_ => Value::None,
		},
		"Array#includes"
		| "Set#includes"
		| "Map#includes"
		| "Pool#includes"
		| "Columnar#includes"
		| "OrderedColumnar#includes" => Value::Boolean(includes(&receiver, arg(1)?)?),
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
		"Array#sum" | "Set#sum" | "Pool#sum" | "Columnar#sum" | "OrderedColumnar#sum" => {
			sum(&iterate(&receiver)?)?
		}
		"Pool#values" | "Columnar#values" | "OrderedColumnar#values" => list(iterate(&receiver)?),
		"Array#reverse" => {
			let mut items = receiver.items().unwrap_or(&[]).to_vec();
			items.reverse();
			list(items)
		}
		"Set#union" | "Set#intersection" | "Set#difference" | "Set#isSubsetOf" => {
			let mine = receiver.items().unwrap_or(&[]).to_vec();
			let other = arg(1)?.items().unwrap_or(&[]).to_vec();
			match path {
				"Set#union" => {
					let mut union = mine;
					for item in other {
						if !set_contains(&union, &item) {
							union.push(item);
						}
					}
					Value::Set(Arc::new(union))
				}
				"Set#intersection" => Value::Set(Arc::new(
					mine.into_iter()
						.filter(|item| set_contains(&other, item))
						.collect(),
				)),
				"Set#difference" => Value::Set(Arc::new(
					mine.into_iter()
						.filter(|item| !set_contains(&other, item))
						.collect(),
				)),
				_ => Value::Boolean(mine.iter().all(|item| set_contains(&other, item))),
			}
		}
		"Array.from"
		| "GrowingArray.from"
		| "Pool.from"
		| "GrowingPool.from"
		| "Columnar.from"
		| "GrowingColumnar.from"
		| "OrderedColumnar.from" => {
			interpreter.reserve(iteration_size(&receiver))?;
			list(iterate(&receiver)?)
		}
		"Set.from" | "GrowingSet.from" => {
			let mut set = Vec::new();
			for item in iterate(&receiver)? {
				if !set_contains(&set, &item) {
					set.push(item);
				}
			}
			Value::Set(Arc::new(set))
		}
		"Map.from" | "GrowingMap.from" => {
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
	if crate::callbacks::Callback::mutating(path) && matches!(receiver, Value::Array(_)) {
		let mut inputs = Vec::with_capacity(arguments.len() + 1);
		inputs.push(receiver.clone());
		inputs.extend(arguments);
		let changed = interpreter.callback_builtin(path, inputs)?;
		*receiver = changed;
		return Ok(Value::None);
	}

	// a value of the interpreter grows on demand: `reserve` is a hint
	if path.ends_with("#reserve") {
		return Ok(Value::None);
	}
	let arg = |index: usize| argument(&arguments, index).cloned();
	match receiver {
		Value::Pool(pool) => {
			let pool = Arc::make_mut(pool);
			match path {
				"Pool#add" | "Columnar#add" => {
					let value = arg(0)?;
					reserve_growth(interpreter, pool.slots.len(), pool.slots.capacity())?;
					pool.slots.push(Some(value.clone()));
					pool.len += 1;
					Ok(value)
				}
				"Pool#delete" | "Pool#take" | "Columnar#delete" | "Columnar#take" => {
					let reference = arg(0)?;
					let index = match &reference {
						Value::Reference(reference) => {
							reference.keys.last().and_then(|key| match key {
								crate::references::Key::Index(index) => {
									index.as_int().and_then(usize_of)
								}
								_ => None,
							})
						}
						_ => None,
					}
					.ok_or_else(|| stop("removing a pool element needs its borrowed place"))?;
					let value = pool
						.slots
						.get_mut(index)
						.and_then(Option::take)
						.ok_or_else(|| stop("this pool element was already removed"))?;
					pool.len -= 1;
					Ok(if path.ends_with("#take") {
						value
					} else {
						Value::None
					})
				}
				"Pool#clear" | "Columnar#clear" => {
					pool.slots.clear();
					pool.len = 0;
					Ok(Value::None)
				}
				_ => Err(Error::Unsupported(format!("`{path}`"))),
			}
		}
		Value::Array(items) => {
			let list = Arc::make_mut(items);
			let result = match path {
				"Array#push" | "OrderedColumnar#add" => {
					let value = arg(0)?;
					if let Some(capacity) = arguments.get(1)
						&& I256::from(list.len() as u64) >= integer(capacity)?
					{
						return Err(Error::Thrown(Value::error(
							"CapacityError.InlineFull",
							vec![],
						)));
					}
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
				_ => return Err(Error::Unsupported(format!("`{path}`"))),
			};
			Ok(result)
		}
		Value::Set(items) => {
			let set = Arc::make_mut(items);
			let result = match path {
				"Set#add" => {
					let value = arg(0)?;
					let added = !set_contains(set, &value);
					if added {
						reserve_growth(interpreter, set.len(), set.capacity())?;
						set.push(value);
					}
					Value::Boolean(added)
				}
				"Set#delete" | "Set#take" => {
					let value = arg(0)?;
					match set.iter().position(|item| key_equals(item, &value)) {
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
					match map.iter().position(|(other, _)| key_equals(other, &key)) {
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
