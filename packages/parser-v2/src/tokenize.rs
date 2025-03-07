use crate::tokens::{ Token, TokenKind };

// Parse an U8 iterator and yield a token
pub fn tokenize(input: &[u8]) -> Vec<Token> {
	let mut tokens: Vec<Token> = Vec::new();
	let mut groups: Vec<usize> = Vec::with_capacity(8);

	let mut offset = 0;

	while offset < input.len() {
		let token = match_token(&input[offset..]);
		offset += token.length as usize;

		match token.kind {
			TokenKind::Stop => {
				if let Some(last_token) = tokens.last_mut() {
					if last_token.kind == TokenKind::Space {
						last_token.kind = TokenKind::Stop;
						last_token.length += token.length;
						continue;
					}
				}
			}
			kind if kind >= TokenKind::Plus && kind <= TokenKind::Pipe => {
				if let Some(last_token) = tokens.last_mut() {
					if last_token.kind == TokenKind::Stop {
						last_token.kind = TokenKind::Space;
					}
				}
			}
			TokenKind::ParenthesisOpen => {
				groups.push(tokens.len());
			}
			TokenKind::ParenthesisClose => {
				let start_offset = groups.pop().unwrap();

				if offset < input.len() {
					let next_token = match_token(&input[offset..]);
					if next_token.kind == TokenKind::FatArrow {
						tokens[start_offset].kind = TokenKind::ParametersStart;
						tokens.push(Token { kind: TokenKind::ParametersEnd, length: 1 });
						continue;
					}
				}
			}
			_ => {}
		}

		tokens.push(token);
	}

	tokens
}

fn match_token(input: &[u8]) -> Token {
	match input[0] {
		b' ' => Token { kind: TokenKind::Space, length: get_space_length(input) },
		b'\t' => Token { kind: TokenKind::Space, length: get_space_length(input) },
		b'\n' => Token { kind: TokenKind::Stop, length: get_stop_length(input) },

		// -- Punctuation --
		b',' => Token { kind: TokenKind::Comma, length: 1 },
		b':' => Token { kind: TokenKind::Colon, length: 1 },
		b'+' => Token { kind: TokenKind::Plus, length: 1 },
		b'-' => Token { kind: TokenKind::Minus, length: 1 },
		b'*' =>
			match input.get(1) {
				Some(b'*') => Token { kind: TokenKind::DoubleStar, length: 2 },
				_ => Token { kind: TokenKind::Star, length: 1 },
			}
		b'/' =>
			match input.get(1) {
				Some(b'/') => Token { kind: TokenKind::DoubleSlash, length: 2 },
				_ => Token { kind: TokenKind::Slash, length: 1 },
			}
		// b'%' => Token { kind: TokenKind::Percent, length: 1 },
		b'=' =>
			match input.get(1) {
				Some(b'=') => Token { kind: TokenKind::DoubleEqual, length: 2 },
				Some(b'>') => Token { kind: TokenKind::FatArrow, length: 2 },
				_ => Token { kind: TokenKind::Equal, length: 1 },
			}
		b'!' =>
			match input.get(1) {
				Some(b'=') => Token { kind: TokenKind::NotEqual, length: 2 },
				_ => Token { kind: TokenKind::ExclamationMark, length: 1 },
			}
		b'?' =>
			match input.get(1) {
				Some(b'.') => Token { kind: TokenKind::QuestionMarkDot, length: 2 },
				Some(b'(') =>
					Token { kind: TokenKind::QuestionMarkParenthesisOpen, length: 2 },
				Some(b'[') =>
					Token { kind: TokenKind::QuestionMarkBracketsOpen, length: 2 },
				_ => Token { kind: TokenKind::QuestionMark, length: 1 },
			}
		b'<' =>
			match input.get(1) {
				Some(b'=') => Token { kind: TokenKind::LessThanOrEqual, length: 2 },
				Some(b'<') => Token { kind: TokenKind::Insert, length: 2 },
				_ => Token { kind: TokenKind::LessThan, length: 1 },
			}
		b'>' =>
			match input.get(1) {
				Some(b'=') => Token { kind: TokenKind::GreaterThanOrEqual, length: 2 },
				Some(b'>') => Token { kind: TokenKind::Extract, length: 2 },
				_ => Token { kind: TokenKind::GreaterThan, length: 1 },
			}
		b'.' =>
			match input.get(1) {
				Some(b'.') =>
					match input.get(2) {
						Some(b'.') => Token { kind: TokenKind::TripleDot, length: 3 },
						_ => Token { kind: TokenKind::DoubleDot, length: 2 },
					}
				_ => Token { kind: TokenKind::Dot, length: 1 },
			}
		b'|' =>
			match input.get(1) {
				Some(b'>') => Token { kind: TokenKind::Pipe, length: 2 },
				_ => Token { kind: TokenKind::Union, length: 1 },
			}

		// -- Groups --
		b'(' => Token { kind: TokenKind::ParenthesisOpen, length: 1 },
		b')' => Token { kind: TokenKind::ParenthesisClose, length: 1 },
		b'{' => Token { kind: TokenKind::BracesOpen, length: 1 },
		b'}' => Token { kind: TokenKind::BracesClose, length: 1 },
		b'[' => Token { kind: TokenKind::BracketsOpen, length: 1 },
		b']' => Token { kind: TokenKind::BracketsClose, length: 1 },

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

			Token { kind: TokenKind::Integer, length: word_length }
		}

		// -- Strings --
		// b'"' => Token { kind: TokenKind::String, length: get_string_length }&input[index..])),

		// -- Keywords & literals --
		_ => {
			let word = get_word(input);
			let word_length = word.len() as u8;
			if word_length == 0 {
				panic!("Unknown character: '{}'", input[0] as char);
			}

			match word {
				b"return" => Token { kind: TokenKind::Return, length: word_length },
				b"if" => Token { kind: TokenKind::If, length: word_length },
				b"else" => Token { kind: TokenKind::Else, length: word_length },
				b"for" => Token { kind: TokenKind::For, length: word_length },
				b"while" => Token { kind: TokenKind::While, length: word_length },
				b"when" => Token { kind: TokenKind::When, length: word_length },
				b"true" => Token { kind: TokenKind::True, length: word_length },
				b"false" => Token { kind: TokenKind::False, length: word_length },
				b"null" => Token { kind: TokenKind::Null, length: word_length },
				b"use" => Token { kind: TokenKind::Use, length: word_length },
				b"async" => Token { kind: TokenKind::Async, length: word_length },
				b"await" => Token { kind: TokenKind::Await, length: word_length },
				b"yield" => Token { kind: TokenKind::Yield, length: word_length },
				b"exit" => Token { kind: TokenKind::Exit, length: word_length },
				b"continue" => Token { kind: TokenKind::Continue, length: word_length },
				b"and" => Token { kind: TokenKind::And, length: word_length },
				b"or" => Token { kind: TokenKind::Or, length: word_length },
				b"not" => Token { kind: TokenKind::Not, length: word_length },
				b"is" => Token { kind: TokenKind::Is, length: word_length },
				b"modulo" => Token { kind: TokenKind::Modulo, length: word_length },
				_ => Token { kind: TokenKind::Identifier, length: word_length },
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
