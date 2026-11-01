use crate::context::Context;
use crate::nodes::IntegerLiteral;

/// Compute the value of the integer literal at the current token (decimal, `0b`, `0o` or `0x`,
/// with an optional leading `-`), reporting misplaced underscores and literals beyond 128 bits.
pub(crate) fn parse_integer(context: &mut Context<'_>) -> IntegerLiteral {
	let token = context.token();
	let text = context.slice();
	let (is_negative, unsigned) = match text.strip_prefix('-') {
		Some(rest) => (true, rest),
		None => (false, text),
	};
	let (radix, digits) = split_radix(unsigned);

	if !check_digits(context, digits, radix) {
		return IntegerLiteral {
			magnitude: 0,
			is_negative,
		};
	}

	let mut magnitude: u128 = 0;
	for character in digits.chars().filter(|character| *character != '_') {
		let digit = character.to_digit(radix).unwrap_or(0) as u128;
		match magnitude
			.checked_mul(radix as u128)
			.and_then(|value| value.checked_add(digit))
		{
			Some(value) => magnitude = value,
			None => {
				context.error(
					format!(
						"`{text}` is too large for any integer type (the largest is `UInt128`): write `{text}n` for a `BigInt`"
					),
					token.start,
					token.end,
				);
				return IntegerLiteral {
					magnitude: 0,
					is_negative,
				};
			}
		}
	}

	IntegerLiteral {
		magnitude,
		is_negative,
	}
}

/// Check the digits of the big integer literal at the current token, like `12n` or `0xFFn` (the
/// node keeps the literal as written).
pub(crate) fn check_big_integer(context: &mut Context<'_>) {
	let text = context.slice();
	let unsigned = text.strip_prefix('-').unwrap_or(text);
	let unsigned = unsigned.strip_suffix('n').unwrap_or(unsigned);
	let (radix, digits) = split_radix(unsigned);
	check_digits(context, digits, radix);
}

/// Split an unsigned integer literal into its radix and its digits, after `0b`, `0o` or `0x`.
fn split_radix(unsigned: &str) -> (u32, &str) {
	match unsigned.get(..2) {
		Some("0b") => (2, &unsigned[2..]),
		Some("0o") => (8, &unsigned[2..]),
		Some("0x") => (16, &unsigned[2..]),
		_ => (10, unsigned),
	}
}

/// Compute the value of the float literal at the current token, like `1.5`, `2.5e-3` or `1e6`.
pub(crate) fn parse_float(context: &mut Context<'_>) -> f64 {
	let text = context.slice();
	let mantissa_end = text.find('e').unwrap_or(text.len());
	let (mantissa, exponent) = text.split_at(mantissa_end);
	let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
	let (integer_part, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
	let exponent = exponent
		.trim_start_matches('e')
		.trim_start_matches(['+', '-']);

	let mut is_valid = check_digits(context, integer_part, 10);
	if !fraction.is_empty() {
		is_valid &= check_digits(context, fraction, 10);
	}
	if !exponent.is_empty() {
		is_valid &= check_digits(context, exponent, 10);
	}
	if !is_valid {
		return 0.0;
	}

	text.replace('_', "").parse::<f64>().unwrap_or(0.0)
}

/// Check that `digits` (a part of the current token) is made of digits of the radix, with
/// underscores only between two digits. Report the problem and return false otherwise.
fn check_digits(context: &mut Context<'_>, digits: &str, radix: u32) -> bool {
	let token = context.token();
	let text = context.slice();
	let base = match radix {
		2 => "binary",
		8 => "octal",
		16 => "hexadecimal",
		_ => "decimal",
	};

	if digits.is_empty() {
		context.error(
			format!("`{text}` has no digits: write at least one {base} digit"),
			token.start,
			token.end,
		);
		return false;
	}

	let bytes = digits.as_bytes();
	for (index, byte) in bytes.iter().enumerate() {
		if *byte == b'_' {
			let is_between_digits = index > 0
				&& index + 1 < bytes.len()
				&& bytes[index - 1] != b'_'
				&& bytes[index + 1] != b'_';
			if !is_between_digits {
				context.error(
					format!("`{text}`: an underscore must be between two digits, like `1_000`"),
					token.start,
					token.end,
				);
				return false;
			}
		} else if !(*byte as char).is_digit(radix) {
			context.error(
				format!(
					"`{text}` is not a valid number: `{}` is not a {base} digit",
					*byte as char
				),
				token.start,
				token.end,
			);
			return false;
		}
	}

	true
}
