use crate::tokens::Token;

pub type TokenizeResult = (Vec<Token>, Vec<u8>);

// Parse an U8 iterator and yield a token
pub fn tokenize(input: &[u8]) -> TokenizeResult {
	let mut tokens = Vec::new();
	let mut tokens_lengths = Vec::new();

	let mut index = 0;

	while index < input.len() {
		let (token, length) = match_token(&input[index..]);
		tokens.push(token);
		tokens_lengths.push(length);
		index += length as usize;
	}

	(tokens, tokens_lengths)
}

fn match_token(input: &[u8]) -> (Token, u8) {
	match input[0] {
		b' ' => (Token::Space, get_space_length(input)),
		b'\t' => (Token::Space, get_space_length(input)),

		// -- Punctuation --
		b',' => (Token::Comma, 1),
		b':' => (Token::Colon, 1),
		b'+' => (Token::Plus, 1),
		b'-' => (Token::Minus, 1),
		b'*' =>
			match input[1] {
				b'*' => (Token::DoubleStar, 2),
				_ => (Token::Star, 1),
			}
		b'/' =>
			match input[1] {
				b'/' => (Token::DoubleSlash, 2),
				_ => (Token::Slash, 1),
			}
		b'%' => (Token::Percent, 1),
		b'=' => if input.len() == 1 {
			(Token::Equal, 1)
		} else {
			match input[1] {
				b'=' => if input.len() == 2 {
					(Token::DoubleEqual, 2)
				} else {
					match input[2] {
						b'>' => (Token::FatArrow, 3),
						_ => (Token::DoubleEqual, 2),
					}
				}
				_ => (Token::Equal, 1),
			}
		}
		b'!' => (Token::NotEqual, 1),
		b'<' =>
			match input[1] {
				b'=' => (Token::LessThanOrEqual, 2),
				_ => (Token::LessThan, 1),
			}
		b'>' =>
			match input[1] {
				b'=' => (Token::GreaterThanOrEqual, 2),
				_ => (Token::GreaterThan, 1),
			}
		b'.' =>
			match input[1] {
				b'.' =>
					match input[2] {
						b'.' => (Token::TripleDot, 3),
						_ => (Token::DoubleDot, 2),
					}
				_ => (Token::Dot, 1),
			}

		// -- Groups --
		b'(' => (Token::ParenthesisOpen, 1),
		b')' => (Token::ParenthesisClose, 1),
		b'{' => (Token::BracesOpen, 1),
		b'}' => (Token::BracesClose, 1),
		b'[' => (Token::BracketsOpen, 1),
		b']' => (Token::BracketsClose, 1),

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

			(Token::Integer, word_length)
		}

		// -- Strings --
		// b'"' => (Token::String, get_string_length(&input[index..])),

		// -- Keywords & literals --
		_ => {
			let word = get_word(input);
			let word_length = word.len() as u8;
			if word_length == 0 {
				panic!("Unknown character: '{}'", input[0] as char);
			}

			match word {
				b"return" => (Token::Return, word_length),
				b"if" => (Token::If, word_length),
				b"else" => (Token::Else, word_length),
				b"for" => (Token::For, word_length),
				b"while" => (Token::While, word_length),
				b"when" => (Token::When, word_length),
				b"true" => (Token::True, word_length),
				b"false" => (Token::False, word_length),
				b"null" => (Token::Null, word_length),
				b"use" => (Token::Use, word_length),
				b"async" => (Token::Async, word_length),
				b"await" => (Token::Await, word_length),
				b"yield" => (Token::Yield, word_length),
				b"exit" => (Token::Exit, word_length),
				b"continue" => (Token::Continue, word_length),
				b"and" => (Token::And, word_length),
				b"or" => (Token::Or, word_length),
				b"not" => (Token::Not, word_length),
				b"is" => (Token::Is, word_length),
				_ => (Token::Identifier, word_length),
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
