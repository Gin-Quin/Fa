use crate::source::SourceMap;
use crate::tokens::{
	FIRST_CHAINABLE_TOKEN, FIRST_CLOSING_TOKEN, FIRST_OPENING_TOKEN, Token, TokenKind,
};

pub struct Tokenizer<'a> {
	input: &'a [u8],
	offset: usize,
	pending_stop: Option<Token>,
	buffered: Option<Token>,
	force_export_keyword: bool,
	line_starts: Vec<usize>,
}

impl<'a> Tokenizer<'a> {
	pub fn new(input: &'a [u8]) -> Self {
		let mut tokenizer = Self {
			input,
			offset: 0,
			pending_stop: None,
			buffered: None,
			force_export_keyword: false,
			line_starts: vec![0],
		};
		tokenizer.skip_spaces_and_newlines();
		tokenizer
	}

	pub fn next_token(&mut self) -> Token {
		let mut token = self.next_token_internal();

		if self.force_export_keyword {
			self.force_export_keyword = false;
			if is_export_keyword_target(token.kind) {
				return token;
			}
		}

		if token.kind == TokenKind::Export {
			let next_kind = self.peek_next_token_kind();
			if is_export_keyword_target(next_kind) {
				self.force_export_keyword = true;
			}
		}

		if is_contextual_keyword(token.kind) {
			let next_kind = self.peek_next_token_kind();
			if !is_identifier_like(next_kind) {
				token.kind = TokenKind::Identifier;
			}
		}

		token
	}

	fn next_token_internal(&mut self) -> Token {
		if let Some(buffered) = self.buffered.take() {
			return buffered;
		}

		loop {
			let raw = match self.next_raw_token() {
				Some(token) => token,
				None => {
					if let Some(pending_stop) = self.pending_stop.take() {
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
				if self.pending_stop.is_none() {
					self.pending_stop = Some(raw);
				}
				continue;
			}

			if let Some(pending_stop) = self.pending_stop.take() {
				if is_chainable_or_closing(raw.kind) {
					return raw;
				}
				self.buffered = Some(raw);
				return pending_stop;
			}

			return raw;
		}
	}

	fn peek_next_token_kind(&mut self) -> TokenKind {
		let token = self.next_token_internal();
		self.buffered = Some(token.clone());
		token.kind
	}

	fn next_raw_token(&mut self) -> Option<Token> {
		if self.offset >= self.input.len() {
			return None;
		}

		let (kind, length) = match_token(
			&self.input[self.offset..],
			&mut self.line_starts,
			self.offset,
		);
		let start = self.offset;
		let end = start + length;
		self.offset += length;

		if (kind as isize) < FIRST_OPENING_TOKEN {
			self.skip_spaces();
		} else {
			self.skip_spaces_and_newlines();
		}

		Some(Token { kind, start, end })
	}

	pub fn source_map(&self) -> SourceMap {
		SourceMap::from_line_starts(self.line_starts.clone())
	}

	fn skip_spaces(&mut self) {
		while self.offset < self.input.len()
			&& (self.input[self.offset] == b' ' || self.input[self.offset] == b'\t')
		{
			self.offset += 1;
		}
	}

	fn skip_spaces_and_newlines(&mut self) {
		while self.offset < self.input.len()
			&& (self.input[self.offset] == b' '
				|| self.input[self.offset] == b'\t'
				|| self.input[self.offset] == b'\n')
		{
			if self.input[self.offset] == b'\n' {
				self.line_starts.push(self.offset + 1);
			}
			self.offset += 1;
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

/// Parse an U8 iterator and yield a vector of tokens
/// Actually not used by the compiler (tokens are streamed instead)
/// Useful for debugging or testing
pub fn tokenize(input: &[u8]) -> Vec<Token> {
	let mut tokenizer = Tokenizer::new(input);
	let mut tokens: Vec<Token> = Vec::new();

	loop {
		let token = tokenizer.next_token();
		if token.kind == TokenKind::End {
			break;
		}
		tokens.push(token);
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
