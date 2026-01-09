use crate::source::SourceMap;
use crate::tokens::{
	FIRST_CHAINABLE_TOKEN, FIRST_CLOSING_TOKEN, FIRST_OPENING_TOKEN, Token, TokenKind,
};

fn next_token(
	input: &[u8],
	offset: &mut usize,
	pending_stop: &mut Option<Token>,
	buffered: &mut Option<Token>,
	force_export_keyword: &mut bool,
	line_starts: &mut Vec<usize>,
) -> Token {
	let mut token = next_token_internal(input, offset, pending_stop, buffered, line_starts);

	if *force_export_keyword {
		*force_export_keyword = false;
		if is_export_keyword_target(token.kind) {
			return token;
		}
	}

	if token.kind == TokenKind::Export {
		let next_kind = peek_next_token_kind(input, offset, pending_stop, buffered, line_starts);
		if is_export_keyword_target(next_kind) {
			*force_export_keyword = true;
		}
	}

	if is_contextual_keyword(token.kind) {
		let next_kind = peek_next_token_kind(input, offset, pending_stop, buffered, line_starts);
		if !is_identifier_like(next_kind) {
			token.kind = TokenKind::Identifier;
		}
	}

	token
}

fn next_token_internal(
	input: &[u8],
	offset: &mut usize,
	pending_stop: &mut Option<Token>,
	buffered: &mut Option<Token>,
	line_starts: &mut Vec<usize>,
) -> Token {
	if let Some(buffered) = buffered.take() {
		return buffered;
	}

	loop {
		let raw = match next_raw_token(input, offset, line_starts) {
			Some(token) => token,
			None => {
				if let Some(pending_stop) = pending_stop.take() {
					return pending_stop;
				}
				return Token {
					kind: TokenKind::End,
					start: 0,
					end: 0,
				};
			}
		};

		if raw.kind == TokenKind::Stop {
			if pending_stop.is_none() {
				*pending_stop = Some(raw);
			}
			continue;
		}

		if let Some(pending_stop) = pending_stop.take() {
			if is_chainable_or_closing(raw.kind) {
				return raw;
			}
			*buffered = Some(raw);
			return pending_stop;
		}

		return raw;
	}
}

fn peek_next_token_kind(
	input: &[u8],
	offset: &mut usize,
	pending_stop: &mut Option<Token>,
	buffered: &mut Option<Token>,
	line_starts: &mut Vec<usize>,
) -> TokenKind {
	let token = next_token_internal(input, offset, pending_stop, buffered, line_starts);
	*buffered = Some(token);
	token.kind
}

fn next_raw_token(input: &[u8], offset: &mut usize, line_starts: &mut Vec<usize>) -> Option<Token> {
	if *offset >= input.len() {
		return None;
	}

	let (kind, length) = match_token(&input[*offset..], line_starts, *offset);
	let start = *offset;
	let end = start + length;
	*offset += length;

	if (kind as isize) < FIRST_OPENING_TOKEN {
		skip_spaces(input, offset);
	} else {
		skip_spaces_and_newlines(input, offset, line_starts);
	}

	Some(Token { kind, start, end })
}

fn skip_spaces(input: &[u8], offset: &mut usize) {
	while *offset < input.len() && (input[*offset] == b' ' || input[*offset] == b'\t') {
		*offset += 1;
	}
}

fn skip_spaces_and_newlines(input: &[u8], offset: &mut usize, line_starts: &mut Vec<usize>) {
	while *offset < input.len()
		&& (input[*offset] == b' ' || input[*offset] == b'\t' || input[*offset] == b'\n')
	{
		if input[*offset] == b'\n' {
			line_starts.push(*offset + 1);
		}
		*offset += 1;
	}
}

fn mark_function_declaration_parentheses(tokens: &mut [Token]) {
	let mut open_stack: Vec<usize> = Vec::new();

	for index in 0..tokens.len() {
		match tokens[index].kind {
			TokenKind::ParenthesisOpen => open_stack.push(index),
			TokenKind::ParenthesisClose => {
				let Some(open_index) = open_stack.pop() else {
					continue;
				};
				let mut lookahead = index + 1;
				while let Some(next_token) = tokens.get(lookahead) {
					match next_token.kind {
						TokenKind::Stop => {
							lookahead += 1;
						}
						TokenKind::Colon | TokenKind::FatArrow => {
							tokens[open_index].kind = TokenKind::ParenthesisOpenFunctionDeclaration;
							break;
						}
						_ => break,
					}
				}
			}
			_ => {}
		}
	}
}

fn is_chainable_or_closing(kind: TokenKind) -> bool {
	let kind = kind as isize;
	kind >= FIRST_CHAINABLE_TOKEN || (kind >= FIRST_CLOSING_TOKEN && kind < FIRST_OPENING_TOKEN)
}

fn is_contextual_keyword(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::Fields
			| TokenKind::Type
			| TokenKind::Reactive
			| TokenKind::Derived
			| TokenKind::UnionKeyword
			| TokenKind::Namespace
			| TokenKind::Enum
	)
}

fn is_identifier_like(kind: TokenKind) -> bool {
	kind == TokenKind::Identifier || is_contextual_keyword(kind)
}

fn is_export_keyword_target(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::Type
			| TokenKind::Namespace
			| TokenKind::UnionKeyword
			| TokenKind::Fields
			| TokenKind::Enum
	)
}

pub fn tokenize_with_source_map(input: &[u8]) -> (Vec<Token>, SourceMap) {
	let mut offset = 0;
	let mut pending_stop = None;
	let mut buffered = None;
	let mut force_export_keyword = false;
	let mut line_starts = vec![0];
	skip_spaces_and_newlines(input, &mut offset, &mut line_starts);
	let mut tokens: Vec<Token> = Vec::new();

	loop {
		let token = next_token(
			input,
			&mut offset,
			&mut pending_stop,
			&mut buffered,
			&mut force_export_keyword,
			&mut line_starts,
		);
		let done = token.kind == TokenKind::End;
		tokens.push(token);
		if done {
			break;
		}
	}

	mark_function_declaration_parentheses(&mut tokens);
	let source_map = SourceMap::from_line_starts(line_starts);
	(tokens, source_map)
}

/// Parse an U8 iterator and yield a vector of tokens
/// Useful for debugging or testing
pub fn tokenize(input: &[u8]) -> Vec<Token> {
	let (mut tokens, _) = tokenize_with_source_map(input);
	if matches!(tokens.last(), Some(token) if token.kind == TokenKind::End) {
		tokens.pop();
	}
	tokens
}

/// Skip spaces and tabs until a newline is found, then increment past the newline and return
/// Match the next token in the input and return its kind and length
fn match_token(
	input: &[u8],
	line_starts: &mut Vec<usize>,
	base_offset: usize,
) -> (TokenKind, usize) {
	match input[0] {
		b'\n' => {
			line_starts.push(base_offset + 1);
			(TokenKind::Stop, 1)
		}

		// -- Punctuation --
		b',' => (TokenKind::Comma, 1),
		b':' => (TokenKind::Colon, 1),
		b'+' => (TokenKind::Plus, 1),
		b'-' => match input.get(1) {
			Some(b'-') => match input.get(2) {
				Some(b'-') => get_block_comment(input, line_starts, base_offset),
				_ => get_inline_comment(input),
			},
			Some(b'>') => (TokenKind::Arrow, 2),
			Some(b'0'..=b'9') => {
				let (kind, length) = get_number(&input[1..]);
				match kind {
					TokenKind::Integer => (TokenKind::NegativeInteger, length + 1),
					TokenKind::BinaryInteger => (TokenKind::NegativeBinaryInteger, length + 1),
					TokenKind::OctalInteger => (TokenKind::NegativeOctalInteger, length + 1),
					TokenKind::HexadecimalInteger => {
						(TokenKind::NegativeHexadecimalInteger, length + 1)
					}
					TokenKind::BigInteger => (TokenKind::NegativeBigInteger, length + 1),
					TokenKind::Number => (TokenKind::Number, length + 1),
					_ => (kind, length + 1),
				}
			}
			Some(b' ' | b'\t') => (TokenKind::MinusWithoutSpaceAfter, 1),
			_ => (TokenKind::MinusWithoutSpaceAfter, 1),
		},
		b'*' => match input.get(1) {
			Some(b'*') => (TokenKind::DoubleStar, 2),
			_ => (TokenKind::Star, 1),
		},
		b'/' => match input.get(1) {
			Some(b'/') => (TokenKind::DoubleSlash, 2),
			_ => (TokenKind::Slash, 1),
		},
		b'%' => (TokenKind::Percent, 1),
		b'=' => match input.get(1) {
			Some(b'=') => (TokenKind::DoubleEqual, 2),
			Some(b'>') => (TokenKind::FatArrow, 2),
			_ => (TokenKind::Equal, 1),
		},
		b'!' => match input.get(1) {
			Some(b'=') => (TokenKind::NotEqual, 2),
			_ => (TokenKind::ExclamationMark, 1),
		},
		b'?' => match input.get(1) {
			Some(b'.') => (TokenKind::QuestionMarkDot, 2),
			_ => (TokenKind::QuestionMark, 1),
		},
		b'<' => match input.get(1) {
			Some(b'=') => (TokenKind::LessThanOrEqual, 2),
			Some(b'<') => (TokenKind::Insert, 2),
			_ => (TokenKind::LessThan, 1),
		},
		b'>' => match input.get(1) {
			Some(b'=') => (TokenKind::GreaterThanOrEqual, 2),
			Some(b'>') => (TokenKind::Extract, 2),
			_ => (TokenKind::GreaterThan, 1),
		},
		b'.' => match input.get(1) {
			Some(b'.') => match input.get(2) {
				Some(b'.') => (TokenKind::TripleDot, 3),
				_ => (TokenKind::DoubleDot, 2),
			},
			_ => (TokenKind::Dot, 1),
		},
		b'&' => (TokenKind::Intersection, 1),
		b'|' => match input.get(1) {
			Some(b'|') => match input.get(2) {
				Some(b'>') => (TokenKind::Compose, 3),
				_ => (TokenKind::Union, 1),
			},
			Some(b'>') => (TokenKind::Pipe, 2),
			_ => (TokenKind::Union, 1),
		},
		b'@' => match input.get(1) {
			Some(b'[') => (TokenKind::AtBracketsOpen, 2),
			Some(b'{') => (TokenKind::AtBracesOpen, 2),
			_ => {
				let word = get_word(&input[1..]);
				if word == b"for" {
					(TokenKind::AtFor, word.len() + 1)
				} else {
					panic!("Unknown token after '@'");
				}
			}
		},

		// -- Groups --
		b'(' => (TokenKind::ParenthesisOpen, 1),
		b')' => (TokenKind::ParenthesisClose, 1),
		b'{' => (TokenKind::BracesOpen, 1),
		b'}' => (TokenKind::BracesClose, 1),
		b'[' => (TokenKind::BracketsOpen, 1),
		b']' => (TokenKind::BracketsClose, 1),

		// -- Numbers --
		b'0'..=b'9' => get_number(input),

		// -- Strings --
		b'"' => get_string(input, line_starts, base_offset),

		// -- Keywords & literals --
		_ => {
			let word = get_word(input);
			let word_length = word.len();
			if word_length == 0 {
				panic!("Unknown character: '{}'", input[0] as char);
			}

			match word {
				b"let" => (TokenKind::Let, word_length),
				b"function" => (TokenKind::Function, word_length),
				b"mutable" => (TokenKind::Mutable, word_length),
				b"static" => (TokenKind::Static, word_length),
				b"type" => (TokenKind::Type, word_length),
				b"union" => (TokenKind::UnionKeyword, word_length),
				b"enum" => (TokenKind::Enum, word_length),
				b"fields" => (TokenKind::Fields, word_length),
				b"reactive" => (TokenKind::Reactive, word_length),
				b"derived" => (TokenKind::Derived, word_length),
				b"namespace" => (TokenKind::Namespace, word_length),
				b"return" => (TokenKind::Return, word_length),
				b"if" => (TokenKind::If, word_length),
				b"else" => (TokenKind::Else, word_length),
				b"for" => (TokenKind::For, word_length),
				b"while" => (TokenKind::While, word_length),
				b"loop" => (TokenKind::Loop, word_length),
				b"do" => (TokenKind::Do, word_length),
				b"when" => (TokenKind::When, word_length),
				b"export" => (TokenKind::Export, word_length),
				b"true" => (TokenKind::True, word_length),
				b"false" => (TokenKind::False, word_length),
				b"null" => (TokenKind::Null, word_length),
				b"use" => (TokenKind::Use, word_length),
				b"async" => (TokenKind::Async, word_length),
				b"await" => (TokenKind::Await, word_length),
				b"yield" => (TokenKind::Yield, word_length),
				b"continue" => (TokenKind::Continue, word_length),
				b"break" => (TokenKind::Break, word_length),
				b"and" => (TokenKind::And, word_length),
				b"or" => (TokenKind::Or, word_length),
				b"not" => (TokenKind::Not, word_length),
				b"is" => (TokenKind::Is, word_length),
				b"modulo" => (TokenKind::Modulo, word_length),
				_ => (TokenKind::Identifier, word_length),
			}
		}
	}
}

/// Return all bytes from the current index to the next word separator
#[inline]
fn get_word(input: &[u8]) -> &[u8] {
	let mut word_length = 0;

	for &byte in &input[0..] {
		if byte < 128
			&& !(b'0'..=b'9').contains(&byte)
			&& !(b'a'..=b'z').contains(&byte)
			&& !(b'A'..=b'Z').contains(&byte)
			&& byte != b'_'
		{
			break;
		}
		word_length += 1;
	}

	&input[0..word_length]
}

/// Parse an inline comment and return its kind and length
#[inline]
fn get_inline_comment(input: &[u8]) -> (TokenKind, usize) {
	let mut length = 2; // Start after the opening "--"

	while length < input.len() && input[length] != b'\n' {
		length += 1;
	}

	(TokenKind::InlineComment, length)
}

/// Parse a string literal and return its kind and length
fn get_string(
	input: &[u8],
	line_starts: &mut Vec<usize>,
	base_offset: usize,
) -> (TokenKind, usize) {
	let mut length = 1; // Start after the opening quote

	while length < input.len() {
		match input[length] {
			b'\\' => {
				length += 1;
				if length < input.len() {
					length += 1;
				}
			}
			b'"' => {
				return (TokenKind::String, length + 1);
			}
			b'\n' => {
				line_starts.push(base_offset + length + 1);
				length += 1;
			}
			_ => {
				length += 1;
			}
		}
	}

	panic!("Unterminated string literal");
}

/// Parse a block comment and return its kind and length
#[inline]
fn get_block_comment(
	input: &[u8],
	line_starts: &mut Vec<usize>,
	base_offset: usize,
) -> (TokenKind, usize) {
	let mut length = 3; // Start after the opening "---"

	while length + 2 < input.len() {
		if input[length] == b'-' && input[length + 1] == b'-' && input[length + 2] == b'-' {
			// Found the closing "---", now skip all spaces and tabs until a newline is found
			length += 3;
			while length < input.len() {
				match input[length] {
					b'\n' => {
						line_starts.push(base_offset + length + 1);
						length += 1;
						return (TokenKind::BlockComment, length);
					}
					b' ' | b'\t' => {
						length += 1;
					}
					_ => return (TokenKind::BlockComment, length),
				}
			}
			return (TokenKind::BlockComment, length);
		}
		if input[length] == b'\n' {
			line_starts.push(base_offset + length + 1);
		}
		length += 1;
	}

	// If we reach here, the comment wasn't properly closed
	(TokenKind::BlockComment, input.len())
}

/// Parses a number in any supported format (decimal, binary, octal, or hexadecimal)
#[inline]
fn get_number(input: &[u8]) -> (TokenKind, usize) {
	// Handle different number formats (decimal, binary, octal, hex)
	if input.len() >= 2 && input[0] == b'0' {
		match input[1] {
			b'b' => {
				return get_integer_with_bigint_suffix(input, get_binary_number);
			}
			b'o' => {
				return get_integer_with_bigint_suffix(input, get_octal_number);
			}
			b'x' => {
				return get_integer_with_bigint_suffix(input, get_hex_number);
			}
			_ => {}
		}
	}

	get_integer_with_bigint_suffix(input, get_decimal_number)
}

fn get_integer_with_bigint_suffix(
	input: &[u8],
	parser: fn(&[u8]) -> (TokenKind, usize),
) -> (TokenKind, usize) {
	let (kind, length) = parser(input);
	if matches!(
		kind,
		TokenKind::Integer
			| TokenKind::BinaryInteger
			| TokenKind::OctalInteger
			| TokenKind::HexadecimalInteger
	) {
		if input.get(length) == Some(&b'n') {
			return (TokenKind::BigInteger, length + 1);
		}
	}
	(kind, length)
}

/// Parses a decimal number, supporting underscores, decimal points, and scientific notation
fn get_decimal_number(input: &[u8]) -> (TokenKind, usize) {
	let mut length = 0;
	let mut has_dot = false;
	let mut has_exponent = false;

	while length < input.len() {
		match input[length] {
			b'0'..=b'9' => {
				length += 1;
			}
			b'_' => {
				length += 1;
			}
			b'.' => {
				if has_dot || has_exponent {
					break;
				}
				match input.get(length + 1) {
					Some(b'0'..=b'9') => {
						has_dot = true;
						length += 1;
					}
					_ => {
						break;
					}
				}
			}
			b'e' => {
				if has_exponent {
					break;
				}
				has_exponent = true;
				length += 1;

				// Handle optional sign after exponent
				if length < input.len() && (input[length] == b'+' || input[length] == b'-') {
					length += 1;
				}
			}
			_ => {
				break;
			}
		}
	}

	(
		if has_dot || has_exponent {
			TokenKind::Number
		} else {
			TokenKind::Integer
		},
		length,
	)
}

/// Parses a binary number (0b prefix), supporting underscores
#[inline]
fn get_binary_number(input: &[u8]) -> (TokenKind, usize) {
	let mut length = 2;

	while let Some(byte) = input.get(length) {
		if *byte == b'0' || *byte == b'1' || *byte == b'_' {
			length += 1;
		} else {
			break;
		}
	}

	(TokenKind::BinaryInteger, length)
}

/// Parses an octal number (0o prefix), supporting underscores
#[inline]
fn get_octal_number(input: &[u8]) -> (TokenKind, usize) {
	let mut length = 2;

	while let Some(byte) = input.get(length) {
		if (*byte >= b'0' && *byte <= b'7') || *byte == b'_' {
			length += 1;
		} else {
			break;
		}
	}

	(TokenKind::OctalInteger, length)
}

/// Parses a hexadecimal number (0x prefix), supporting underscores
#[inline]
fn get_hex_number(input: &[u8]) -> (TokenKind, usize) {
	let mut length = 2;

	while let Some(byte) = input.get(length) {
		if (*byte >= b'0' && *byte <= b'9')
			|| (*byte >= b'a' && *byte <= b'f')
			|| (*byte >= b'A' && *byte <= b'F')
			|| *byte == b'_'
		{
			length += 1;
		} else {
			break;
		}
	}

	(TokenKind::HexadecimalInteger, length)
}
