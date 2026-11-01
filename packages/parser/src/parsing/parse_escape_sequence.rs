use crate::context::Context;

/// Parse the escape sequence starting at the backslash at `index` (before `end`), push the
/// escaped character to `literal`, and return the index after the sequence. An invalid sequence
/// is reported and pushes nothing.
pub(crate) fn parse_escape_sequence(
	context: &mut Context<'_>,
	index: usize,
	end: usize,
	literal: &mut String,
) -> usize {
	let source = context.source;
	let bytes = source.as_bytes();
	let Some(&next) = bytes.get(index + 1).filter(|_| index + 1 < end) else {
		context.error(
			"A string can't end with a single `\\`: write `\\\\` for a backslash",
			index,
			index + 1,
		);
		return end;
	};

	let simple = match next {
		b'\\' => Some('\\'),
		b'"' => Some('"'),
		b'n' => Some('\n'),
		b'r' => Some('\r'),
		b't' => Some('\t'),
		b'0' => Some('\0'),
		_ => None,
	};
	if let Some(character) = simple {
		literal.push(character);
		return index + 2;
	}

	match next {
		b'x' => {
			let mut digits_end = (index + 4).min(end);
			while !source.is_char_boundary(digits_end) {
				digits_end += 1;
			}
			let digits = &bytes[index + 2..digits_end];
			let value = (digits.len() == 2)
				.then(|| parse_hex(digits))
				.flatten()
				.filter(|value| *value <= 0x7F);
			match value {
				Some(value) => {
					literal.push(value as u8 as char);
					index + 4
				}
				None => {
					context.error(
						"`\\x` is followed by two hexadecimal digits, from `\\x00` to `\\x7F`: write other characters with `\\u{...}`",
						index,
						digits_end,
					);
					digits_end
				}
			}
		}
		b'u' => {
			let message = "`\\u` is followed by a Unicode scalar value of 1 to 6 hexadecimal digits between braces, like `\\u{1F600}`";
			if bytes.get(index + 2) != Some(&b'{') || index + 2 >= end {
				context.error(message, index, index + 2);
				return index + 2;
			}
			let digits_start = index + 3;
			let mut cursor = digits_start;
			while cursor < end && bytes[cursor].is_ascii_hexdigit() {
				cursor += 1;
			}
			let closed = cursor < end && bytes[cursor] == b'}';
			let digits = &bytes[digits_start..cursor];
			let character = (closed && (1..=6).contains(&digits.len()))
				.then(|| parse_hex(digits))
				.flatten()
				.and_then(char::from_u32);
			let after = if closed { cursor + 1 } else { cursor };
			match character {
				Some(character) => literal.push(character),
				None => context.error(message, index, after),
			}
			after
		}
		_ => {
			let length = source[index + 1..]
				.chars()
				.next()
				.map_or(1, |character| character.len_utf8());
			let sequence = &source[index..index + 1 + length];
			context.error(
				format!(
					"Unknown escape sequence `{sequence}`: the escapes are `\\\\`, `\\\"`, `\\n`, `\\r`, `\\t`, `\\0`, `\\xNN` and `\\u{{...}}`"
				),
				index,
				index + 1 + length,
			);
			index + 1 + length
		}
	}
}

fn parse_hex(digits: &[u8]) -> Option<u32> {
	let mut value: u32 = 0;
	for &digit in digits {
		value = value.checked_mul(16)? + (digit as char).to_digit(16)?;
	}
	Some(value)
}
