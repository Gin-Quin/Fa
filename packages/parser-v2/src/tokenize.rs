use crate::tokens::{ Token, TokenKind };

// Parse an U8 iterator and yield a token
pub fn tokenize(input: &[u8]) -> Vec<Token> {
	let mut tokens: Vec<Token> = Vec::new();
	let mut groups: Vec<usize> = Vec::with_capacity(8);

	let mut offset = 0;

	skip_spaces(&input, &mut offset);

	while offset < input.len() {
		let (kind, length) = match_token(&input[offset..]);
		offset += length as usize;
		let start = offset;
		let end = start + length;
		skip_spaces(&input, &mut offset);

		match kind {
			TokenKind::Stop => {
				if let Some(previous) = tokens.last_mut() {
					if previous.kind == TokenKind::Stop {
						continue;
					}
				}
			}
			kind if kind >= TokenKind::Plus && kind <= TokenKind::Pipe => {
				if let Some(last_token) = tokens.last_mut() {
					if last_token.kind == TokenKind::Stop {
						tokens.pop();
					}
				}
			}
			TokenKind::ParenthesisOpen => {
				groups.push(tokens.len());
			}
			TokenKind::ParenthesisClose => {
				let last_group_index = groups.pop().unwrap();

				if offset < input.len() {
					tokens.push(Token { kind, start, end });

					let (next_kind, next_length) = match_token(&input[offset..]);
					let next_start = offset;
					let next_end = next_start + next_length;
					skip_spaces(&input, &mut offset);

					if next_kind == TokenKind::FatArrow {
						tokens[last_group_index].kind = TokenKind::ParametersStart;
					}

					tokens.push(Token {
						kind: next_kind,
						start: next_start,
						end: next_end,
					});
					continue;
				}
			}
			_ => {}
		}

		tokens.push(Token { kind, start, end });
	}

	tokens
}

fn skip_spaces(input: &[u8], offset: &mut usize) {
	while *offset < input.len() && (input[*offset] == b' ' || input[*offset] == b'\t') {
		*offset += 1;
	}
}

fn match_token(input: &[u8]) -> (TokenKind, usize) {
	match input[0] {
		b'\n' => (TokenKind::Stop, 1),

		// -- Punctuation --
		b',' => (TokenKind::Comma, 1),
		b':' => (TokenKind::Colon, 1),
		b'+' => (TokenKind::Plus, 1),
		b'-' => (TokenKind::Minus, 1),
		b'*' =>
			match input.get(1) {
				Some(b'*') => (TokenKind::DoubleStar, 2),
				_ => (TokenKind::Star, 1),
			}
		b'/' =>
			match input.get(1) {
				Some(b'/') => (TokenKind::DoubleSlash, 2),
				_ => (TokenKind::Slash, 1),
			}
		// b'%' => (TokenKind::Percent, 1),
		b'=' =>
			match input.get(1) {
				Some(b'=') => (TokenKind::DoubleEqual, 2),
				Some(b'>') => (TokenKind::FatArrow, 2),
				_ => (TokenKind::Equal, 1),
			}
		b'!' =>
			match input.get(1) {
				Some(b'=') => (TokenKind::NotEqual, 2),
				_ => (TokenKind::ExclamationMark, 1),
			}
		b'?' =>
			match input.get(1) {
				Some(b'.') => (TokenKind::QuestionMarkDot, 2),
				Some(b'(') => (TokenKind::QuestionMarkParenthesisOpen, 2),
				Some(b'[') => (TokenKind::QuestionMarkBracketsOpen, 2),
				_ => (TokenKind::QuestionMark, 1),
			}
		b'<' =>
			match input.get(1) {
				Some(b'=') => (TokenKind::LessThanOrEqual, 2),
				Some(b'<') => (TokenKind::Insert, 2),
				_ => (TokenKind::LessThan, 1),
			}
		b'>' =>
			match input.get(1) {
				Some(b'=') => (TokenKind::GreaterThanOrEqual, 2),
				Some(b'>') => (TokenKind::Extract, 2),
				_ => (TokenKind::GreaterThan, 1),
			}
		b'.' =>
			match input.get(1) {
				Some(b'.') =>
					match input.get(2) {
						Some(b'.') => (TokenKind::TripleDot, 3),
						_ => (TokenKind::DoubleDot, 2),
					}
				_ => (TokenKind::Dot, 1),
			}
		b'|' =>
			match input.get(1) {
				Some(b'>') => (TokenKind::Pipe, 2),
				_ => (TokenKind::Union, 1),
			}

		// -- Groups --
		b'(' => (TokenKind::ParenthesisOpen, 1),
		b')' => (TokenKind::ParenthesisClose, 1),
		b'{' => (TokenKind::BracesOpen, 1),
		b'}' => (TokenKind::BracesClose, 1),
		b'[' => (TokenKind::BracketsOpen, 1),
		b']' => (TokenKind::BracketsClose, 1),

		// -- Numbers --
		b'0'..=b'9' => {
			let mut word_length = 1;

			if input.len() > 1 {
				for &byte in &input[1..] {
					if byte >= b'0' && byte <= b'9' {
						word_length += 1;
					} else {
						break;
					}
				}
			}

			(TokenKind::Integer, word_length)
		}

		// -- Strings --
		// b'"' => (TokenKind::String, g)_string_length }&input[index..])),

		// -- Keywords & literals --
		_ => {
			let word = get_word(input);
			let word_length = word.len();
			if word_length == 0 {
				panic!("Unknown character: '{}'", input[0] as char);
			}

			match word {
				b"return" => (TokenKind::Return, word_length),
				b"if" => (TokenKind::If, word_length),
				b"else" => (TokenKind::Else, word_length),
				b"for" => (TokenKind::For, word_length),
				b"while" => (TokenKind::While, word_length),
				b"when" => (TokenKind::When, word_length),
				b"true" => (TokenKind::True, word_length),
				b"false" => (TokenKind::False, word_length),
				b"null" => (TokenKind::Null, word_length),
				b"use" => (TokenKind::Use, word_length),
				b"async" => (TokenKind::Async, word_length),
				b"await" => (TokenKind::Await, word_length),
				b"yield" => (TokenKind::Yield, word_length),
				b"exit" => (TokenKind::Exit, word_length),
				b"continue" => (TokenKind::Continue, word_length),
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

#[inline]
fn get_space_length(input: &[u8]) -> u8 {
	let mut word_length = 1;
	if input.len() > 1 {
		for &byte in &input[1..] {
			if byte == b' ' || byte == b'\t' {
				word_length += 1;
			} else {
				break;
			}
		}
	}
	word_length
}

#[inline]
fn get_stop_length(input: &[u8]) -> u8 {
	let mut word_length = 1;
	if input.len() > 1 {
		for &byte in &input[1..] {
			if byte == b' ' || byte == b'\t' || byte == b'\n' {
				word_length += 1;
			} else {
				break;
			}
		}
	}
	word_length
}

/**
 * Return all bytes from the current index to the next punctuation character or space.
 * A word separator is:
 * - Any ASCII character that is not a letter, number or underscore.
 * - Any UTF-8 character has a word_length of 2 bytes or more.
 */
#[inline]
fn get_word(input: &[u8]) -> &[u8] {
	let mut word_length = 0;

	for &byte in &input[0..] {
		if
			byte < 128 &&
			(byte < b'0' || byte > b'9') &&
			(byte < b'a' || byte > b'z') &&
			(byte < b'A' || byte > b'Z') &&
			byte != b'_'
		{
			break;
		}
		word_length += 1;
	}

	if word_length > 255 {
		// Word is too long; Fa only accepts word lengths of size u8
		panic!(
			"Word in Fa cannot be longer than 254 bytes. It's very likely that there is a syntax error."
		);
	}

	&input[0..word_length]
}
