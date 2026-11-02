use crate::diagnostic::Diagnostic;
use crate::source::SourceSpan;
use crate::tokens::{Token, TokenKind};

/// Tokenize a source, keeping the comments. The `End` token is not included.
/// Useful for debugging, testing and syntax highlighting.
pub fn tokenize(source: &str) -> Vec<Token> {
	tokenize_with_diagnostics(source).0
}

/// Tokenize a source, keeping the comments, and return the problems found (unknown characters,
/// unclosed strings or comments...). The `End` token is not included.
pub fn tokenize_with_diagnostics(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
	let mut diagnostics = Vec::new();
	let mut tokens = tokenize_range(source, 0, source.len(), true, &mut diagnostics, None);
	if matches!(tokens.last(), Some(token) if token.kind == TokenKind::End) {
		tokens.pop();
	}
	(tokens, diagnostics)
}

/// Tokenize `source[start..end]`, with the offsets of the tokens in the whole source.
/// New lines become `Stop` tokens, except where a line continues the previous one (see
/// `TokenKind::skips_newlines_before` and `TokenKind::skips_newlines_after`). The last token is
/// always `End`. Without `keep_comments`, the spans of the comments go to `comments`.
pub(crate) fn tokenize_range(
	source: &str,
	start: usize,
	end: usize,
	keep_comments: bool,
	diagnostics: &mut Vec<Diagnostic>,
	mut comments: Option<&mut Vec<SourceSpan>>,
) -> Vec<Token> {
	let mut lexer = Lexer {
		bytes: source.as_bytes(),
		offset: start,
		end,
		diagnostics,
		string_frames: Vec::new(),
	};
	// a token every 4 bytes is a good upper estimate for code
	let mut tokens: Vec<Token> = Vec::with_capacity((end - start) / 4 + 1);
	let mut pending_stop: Option<Token> = None;
	// `Empty` at the start: new lines before the first token are ignored
	let mut previous_kind = TokenKind::Empty;

	loop {
		lexer.skip_spaces();
		if lexer.offset >= lexer.end {
			break;
		}

		let token_start = lexer.offset;
		if lexer.bytes[token_start] == b'\n' {
			lexer.offset += 1;
			if pending_stop.is_none()
				&& previous_kind != TokenKind::Empty
				&& !previous_kind.skips_newlines_after()
			{
				pending_stop = Some(Token::new(TokenKind::Stop, token_start, token_start + 1));
				// a `-` at the start of a line starts a new statement: `-1` is a constant
				previous_kind = TokenKind::Stop;
			}
			continue;
		}

		let kind = lexer.next_kind(previous_kind);
		let token = Token::new(kind, token_start, lexer.offset);

		if matches!(kind, TokenKind::InlineComment | TokenKind::BlockComment) {
			if keep_comments {
				tokens.push(token);
			} else if let Some(comments) = comments.as_deref_mut() {
				comments.push(SourceSpan::new(token.start, token.end));
			}
			continue;
		}

		if let Some(stop) = pending_stop.take()
			&& !kind.skips_newlines_before()
		{
			tokens.push(stop);
		}

		tokens.push(token);
		previous_kind = kind;
	}

	tokens.push(Token::new(TokenKind::End, end, end));
	resolve_contextual_keywords(&mut tokens);
	resolve_start_keywords(&mut tokens);
	mark_function_declaration_parentheses(&mut tokens);
	tokens
}

enum StringFrame {
	String,
	Interpolation { depth: u32 },
}

struct Lexer<'a> {
	bytes: &'a [u8],
	offset: usize,
	end: usize,
	diagnostics: &'a mut Vec<Diagnostic>,
	// reused by `scan_string`, to avoid an allocation per string
	string_frames: Vec<StringFrame>,
}

impl Lexer<'_> {
	#[inline]
	fn peek(&self, distance: usize) -> Option<u8> {
		let index = self.offset + distance;
		if index < self.end {
			Some(self.bytes[index])
		} else {
			None
		}
	}

	#[inline]
	fn skip_spaces(&mut self) {
		while self.offset < self.end && matches!(self.bytes[self.offset], b' ' | b'\t' | b'\r') {
			self.offset += 1;
		}
	}

	fn error(&mut self, message: impl Into<String>, start: usize, end: usize) {
		self.diagnostics
			.push(Diagnostic::error(message, SourceSpan::new(start, end)));
	}

	/// An error that breaks a rule with a code in the specification (`E-LEX-2`).
	fn error_with_code(
		&mut self,
		code: &'static str,
		message: impl Into<String>,
		start: usize,
		end: usize,
	) {
		self.diagnostics
			.push(Diagnostic::error(message, SourceSpan::new(start, end)).with_code(code));
	}

	/// Read the token at the current offset (never a space or a new line), move after it and
	/// return its kind.
	fn next_kind(&mut self, previous_kind: TokenKind) -> TokenKind {
		let start = self.offset;
		let byte = self.bytes[start];

		macro_rules! token {
			($kind:expr, $length:expr) => {{
				self.offset += $length;
				$kind
			}};
		}

		match byte {
			b',' => token!(TokenKind::Comma, 1),
			b':' => token!(TokenKind::Colon, 1),
			b'+' => match self.peek(1) {
				Some(b'=') => token!(TokenKind::PlusEqual, 2),
				_ => token!(TokenKind::Plus, 1),
			},
			b'-' => match self.peek(1) {
				Some(b'-') => {
					if self.peek(2) == Some(b'-') {
						self.block_comment()
					} else {
						self.inline_comment()
					}
				}
				Some(b'>') => token!(TokenKind::Arrow, 2),
				Some(b'=') => token!(TokenKind::MinusEqual, 2),
				// a minus sign written before a literal is part of the constant, unless it
				// follows an operand: `-1`, `[-1]` and `x = -1` hold a constant, `x-1` and
				// `x - 1` are subtractions
				Some(b'0'..=b'9') if !previous_kind.ends_operand() => {
					self.offset += 1;
					match self.number() {
						TokenKind::Integer => TokenKind::NegativeInteger,
						TokenKind::BinaryInteger => TokenKind::NegativeBinaryInteger,
						TokenKind::OctalInteger => TokenKind::NegativeOctalInteger,
						TokenKind::HexadecimalInteger => TokenKind::NegativeHexadecimalInteger,
						TokenKind::BigInteger => TokenKind::NegativeBigInteger,
						kind => kind,
					}
				}
				_ => token!(TokenKind::Minus, 1),
			},
			b'*' => match (self.peek(1), self.peek(2)) {
				(Some(b'*'), Some(b'=')) => token!(TokenKind::DoubleStarEqual, 3),
				(Some(b'*'), _) => token!(TokenKind::DoubleStar, 2),
				(Some(b'='), _) => token!(TokenKind::StarEqual, 2),
				_ => token!(TokenKind::Star, 1),
			},
			b'/' => match (self.peek(1), self.peek(2)) {
				(Some(b'/'), Some(b'=')) => token!(TokenKind::DoubleSlashEqual, 3),
				(Some(b'/'), _) => token!(TokenKind::DoubleSlash, 2),
				(Some(b'='), _) => token!(TokenKind::SlashEqual, 2),
				_ => token!(TokenKind::Slash, 1),
			},
			// `%` is the percentage suffix, `%%` the remainder (like `//` is the integer division)
			b'%' => match (self.peek(1), self.peek(2)) {
				(Some(b'%'), Some(b'=')) => token!(TokenKind::DoublePercentEqual, 3),
				(Some(b'%'), _) => token!(TokenKind::DoublePercent, 2),
				_ => token!(TokenKind::Percent, 1),
			},
			b'=' => match self.peek(1) {
				Some(b'=') => token!(TokenKind::DoubleEqual, 2),
				Some(b'>') => token!(TokenKind::FatArrow, 2),
				_ => token!(TokenKind::Equal, 1),
			},
			b'!' => match self.peek(1) {
				Some(b'=') => token!(TokenKind::NotEqual, 2),
				_ => {
					self.offset += 1;
					self.error_with_code(
						"E-LEX-4",
						"`!` is not an operator: use `not` to negate a boolean, or `!=` for inequality",
						start,
						self.offset,
					);
					TokenKind::Invalid
				}
			},
			b'?' => match self.peek(1) {
				Some(b'.') => token!(TokenKind::QuestionMarkDot, 2),
				Some(b'?') => token!(TokenKind::DoubleQuestionMark, 2),
				_ => token!(TokenKind::QuestionMark, 1),
			},
			b'<' => match self.peek(1) {
				Some(b'=') => token!(TokenKind::LessThanOrEqual, 2),
				_ => token!(TokenKind::LessThan, 1),
			},
			b'>' => match self.peek(1) {
				Some(b'=') => token!(TokenKind::GreaterThanOrEqual, 2),
				_ => token!(TokenKind::GreaterThan, 1),
			},
			b'.' => match self.peek(1) {
				Some(b'.') => match self.peek(2) {
					Some(b'.') => token!(TokenKind::TripleDot, 3),
					Some(b'=') => token!(TokenKind::DoubleDotEqual, 3),
					_ => token!(TokenKind::DoubleDot, 2),
				},
				// partial update, ex: `settings .= { port = 8080 }`
				Some(b'=') => token!(TokenKind::DotEqual, 2),
				_ => token!(TokenKind::Dot, 1),
			},
			b'|' => token!(TokenKind::Union, 1),
			// compile-time names, like `@foo`, `@type(x)` or `@import(source)`
			b'@' => {
				self.offset += 1;
				let word = self.word();
				match word {
					b"for" => TokenKind::AtFor,
					b"if" => TokenKind::AtIf,
					b"else" => TokenKind::AtElse,
					b"test" => TokenKind::AtTest,
					b"" => {
						self.error(
							"Expected a name after `@`, like `@readFile(path)`. Compile-time values are declared with `static name = value`",
							start,
							self.offset,
						);
						TokenKind::Invalid
					}
					_ => TokenKind::Identifier,
				}
			}
			// private names, like `#bag`
			b'#' => {
				self.offset += 1;
				if self.word().is_empty() {
					self.error(
						"Expected a name after `#`, like `#count`. Comments start with `--`",
						start,
						self.offset,
					);
					TokenKind::Invalid
				} else {
					TokenKind::Identifier
				}
			}

			b'(' => token!(TokenKind::ParenthesisOpen, 1),
			b')' => token!(TokenKind::ParenthesisClose, 1),
			b'{' => token!(TokenKind::BracesOpen, 1),
			b'}' => token!(TokenKind::BracesClose, 1),
			b'[' => token!(TokenKind::BracketsOpen, 1),
			b']' => token!(TokenKind::BracketsClose, 1),

			b'0'..=b'9' => self.number(),
			b'"' => self.string(),

			b'a'..=b'z' | b'A'..=b'Z' | b'_' => keyword_or_identifier(self.word()),

			_ => {
				// a non-ASCII character, or an ASCII character that is not valid Fa
				self.offset += 1;
				while self.offset < self.end && self.bytes[self.offset] >= 0x80 {
					self.offset += 1;
				}
				let (code, message) = if byte >= 0x80 {
					(Some("E-LEX-2"), "Names can only use ASCII letters, digits and `_`: other characters are only allowed in strings and comments".to_string())
				} else {
					let character = byte as char;
					match byte {
						b'&' | b'^' | b'~' => (
							None,
							format!(
								"`{character}` is not an operator: use `and`, `or`, `not` for booleans, and the methods of integers for bitwise operations"
							),
						),
						b';' => (
							Some("E-LEX-1"),
							"Statements are separated by new lines, not `;`".to_string(),
						),
						b'\'' | b'`' => (
							Some("E-STR-13"),
							"Strings are written between double quotes, like \"text\"".to_string(),
						),
						_ => (None, format!("Unexpected character {:?}", character)),
					}
				};
				match code {
					Some(code) => self.error_with_code(code, message, start, self.offset),
					None => self.error(message, start, self.offset),
				}
				TokenKind::Invalid
			}
		}
	}

	/// Read a word: ASCII letters, digits and underscores.
	#[inline]
	fn word(&mut self) -> &[u8] {
		let start = self.offset;
		while self.offset < self.end && is_word_byte(self.bytes[self.offset]) {
			self.offset += 1;
		}
		&self.bytes[start..self.offset]
	}

	fn inline_comment(&mut self) -> TokenKind {
		while self.offset < self.end && self.bytes[self.offset] != b'\n' {
			self.offset += 1;
		}
		TokenKind::InlineComment
	}

	/// A block comment, from `---` to the next `---`.
	fn block_comment(&mut self) -> TokenKind {
		let start = self.offset;
		self.offset += 3;
		while self.offset + 2 < self.end {
			if self.bytes[self.offset] == b'-'
				&& self.bytes[self.offset + 1] == b'-'
				&& self.bytes[self.offset + 2] == b'-'
			{
				self.offset += 3;
				return TokenKind::BlockComment;
			}
			self.offset += 1;
		}
		self.offset = self.end;
		self.error_with_code(
			"E-LEX-6",
			"This block comment is never closed: end it with `---`",
			start,
			start + 3,
		);
		TokenKind::BlockComment
	}

	/// Read a number literal. The digits are checked by the parser, which computes the value.
	fn number(&mut self) -> TokenKind {
		let start = self.offset;
		let kind = if self.bytes[start] == b'0' && matches!(self.peek(1), Some(b'b' | b'o' | b'x'))
		{
			let (kind, is_digit): (TokenKind, fn(u8) -> bool) = match self.bytes[start + 1] {
				b'b' => (TokenKind::BinaryInteger, |byte| {
					matches!(byte, b'0' | b'1' | b'_')
				}),
				b'o' => (TokenKind::OctalInteger, |byte| {
					matches!(byte, b'0'..=b'7' | b'_')
				}),
				_ => (TokenKind::HexadecimalInteger, |byte| {
					byte.is_ascii_hexdigit() || byte == b'_'
				}),
			};
			self.offset += 2;
			while self.offset < self.end && is_digit(self.bytes[self.offset]) {
				self.offset += 1;
			}
			kind
		} else {
			self.decimal_number()
		};

		let kind = if kind != TokenKind::Number && self.peek(0) == Some(b'n') {
			self.offset += 1;
			TokenKind::BigInteger
		} else {
			kind
		};

		// a number directly followed by letters, like `12px` or `0b102`
		if self.offset < self.end && is_word_byte(self.bytes[self.offset]) {
			self.word();
			let text = String::from_utf8_lossy(&self.bytes[start..self.offset]).into_owned();
			// `12.5n`: `n` on a float (E-NUM-4); `0XFF`, `1E6`, `0b2`: E-NUM-3
			let float = kind == TokenKind::Number;
			let code = if float && text.ends_with('n') {
				"E-NUM-4"
			} else {
				"E-NUM-3"
			};
			self.error_with_code(
				code,
				format!(
					"`{text}` is not a valid number: a number literal can't be followed by letters or digits of another base"
				),
				start,
				self.offset,
			);
			return TokenKind::Invalid;
		}

		kind
	}

	/// Read a decimal number, with optional `_` separators, decimals and exponent.
	fn decimal_number(&mut self) -> TokenKind {
		let mut is_float = false;
		self.skip_digits();

		// decimals: a `.` followed by a digit (`1..5` is a range, `1.max(2)` a method call)
		if self.peek(0) == Some(b'.') && matches!(self.peek(1), Some(b'0'..=b'9')) {
			is_float = true;
			self.offset += 1;
			self.skip_digits();
		}

		// exponent: `e`, an optional sign, then digits
		if self.peek(0) == Some(b'e') {
			let digits_at = if matches!(self.peek(1), Some(b'+' | b'-')) {
				2
			} else {
				1
			};
			if matches!(self.peek(digits_at), Some(b'0'..=b'9')) {
				is_float = true;
				self.offset += digits_at;
				self.skip_digits();
			}
		}

		if is_float {
			TokenKind::Number
		} else {
			TokenKind::Integer
		}
	}

	#[inline]
	fn skip_digits(&mut self) {
		while self.offset < self.end && matches!(self.bytes[self.offset], b'0'..=b'9' | b'_') {
			self.offset += 1;
		}
	}

	/// Read a string literal, including the nested strings of its interpolations, like
	/// `"Name: {user.name ?? "anonymous"}"`.
	fn string(&mut self) -> TokenKind {
		let start = self.offset;
		match scan_string(self.bytes, start + 1, self.end, &mut self.string_frames) {
			Some(end) => self.offset = end,
			None => {
				self.offset = self.end;
				self.error_with_code(
					"E-LEX-6",
					"This string is never closed: end it with `\"`",
					start,
					start + 1,
				);
			}
		}
		TokenKind::String
	}
}

#[inline]
fn is_word_byte(byte: u8) -> bool {
	byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Return the offset after the closing quote of a string whose content starts at `start`,
/// or `None` if the string is never closed.
fn scan_string(
	bytes: &[u8],
	start: usize,
	end: usize,
	frames: &mut Vec<StringFrame>,
) -> Option<usize> {
	frames.clear();
	frames.push(StringFrame::String);
	scan_frames(bytes, start, end, frames)
}

/// Return the offset after the `}` that closes an interpolation whose content starts at
/// `start` (right after its `{`), or `None` if it is never closed.
pub(crate) fn find_interpolation_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
	let mut frames = vec![StringFrame::Interpolation { depth: 0 }];
	scan_frames(bytes, start, end, &mut frames)
}

/// Scan nested strings and interpolations until the outermost frame closes.
fn scan_frames(
	bytes: &[u8],
	start: usize,
	end: usize,
	frames: &mut Vec<StringFrame>,
) -> Option<usize> {
	let mut index = start;
	while index < end {
		let byte = bytes[index];
		match frames.last_mut() {
			Some(StringFrame::String) => match byte {
				b'\\' => index += 1,
				b'"' => {
					frames.pop();
					if frames.is_empty() {
						return Some(index + 1);
					}
				}
				b'{' => {
					if index + 1 < end && bytes[index + 1] == b'{' {
						index += 1;
					} else {
						frames.push(StringFrame::Interpolation { depth: 0 });
					}
				}
				_ => {}
			},
			Some(StringFrame::Interpolation { depth }) => match byte {
				b'"' => frames.push(StringFrame::String),
				b'{' => *depth += 1,
				b'}' => {
					if *depth == 0 {
						frames.pop();
						if frames.is_empty() {
							return Some(index + 1);
						}
					} else {
						*depth -= 1;
					}
				}
				_ => {}
			},
			None => return Some(index),
		}
		index += 1;
	}
	None
}

fn keyword_or_identifier(word: &[u8]) -> TokenKind {
	match word {
		b"let" => TokenKind::Let,
		b"function" => TokenKind::Function,
		b"mutable" => TokenKind::Mutable,
		b"static" => TokenKind::Static,
		b"type" => TokenKind::Type,
		b"reactive" => TokenKind::Reactive,
		b"derived" => TokenKind::Derived,
		b"namespace" => TokenKind::Namespace,
		b"return" => TokenKind::Return,
		b"if" => TokenKind::If,
		b"else" => TokenKind::Else,
		b"for" => TokenKind::For,
		b"parallel" => TokenKind::Parallel,
		b"while" => TokenKind::While,
		b"loop" => TokenKind::Loop,
		b"do" => TokenKind::Do,
		b"watch" => TokenKind::Watch,
		b"when" => TokenKind::When,
		b"export" => TokenKind::Export,
		b"true" => TokenKind::True,
		b"false" => TokenKind::False,
		b"none" => TokenKind::None,
		b"Error" => TokenKind::Error,
		b"use" => TokenKind::Use,
		b"reuse" => TokenKind::Reuse,
		b"async" => TokenKind::Async,
		b"await" => TokenKind::Await,
		b"continue" => TokenKind::Continue,
		b"break" => TokenKind::Break,
		b"and" => TokenKind::And,
		b"or" => TokenKind::Or,
		b"not" => TokenKind::Not,
		b"is" => TokenKind::Is,
		b"in" => TokenKind::In,
		b"as" => TokenKind::As,
		b"then" => TokenKind::Then,
		b"with" => TokenKind::With,
		b"throws" => TokenKind::Throws,
		b"throw" => TokenKind::Throw,
		b"try" => TokenKind::Try,
		b"catch" => TokenKind::Catch,
		b"start" => TokenKind::Start,
		b"provide" => TokenKind::Provide,
		b"assert" => TokenKind::Assert,
		b"requires" => TokenKind::Requires,
		b"mutates" => TokenKind::Mutates,
		b"invalidates" => TokenKind::Invalidates,
		b"readonly" => TokenKind::Readonly,
		_ => TokenKind::Identifier,
	}
}

/// Mark the parentheses that open the parameters of a function: the ones whose closing
/// parenthesis is followed by `:`, `=>`, `throws`, `requires`, `mutates` or `invalidates`.
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
						TokenKind::Stop | TokenKind::InlineComment | TokenKind::BlockComment => {
							lookahead += 1;
						}
						TokenKind::Colon
						| TokenKind::FatArrow
						| TokenKind::Throws
						| TokenKind::Requires
						| TokenKind::Mutates
						| TokenKind::Invalidates => {
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

/// Contextual keywords, like `type` or `namespace`, are only keywords when they declare
/// something, like `type Foo = ...` or `export type = ...`. Otherwise, they are regular names.
fn resolve_contextual_keywords(tokens: &mut [Token]) {
	for index in 0..tokens.len() {
		let kind = tokens[index].kind;
		if !is_contextual_keyword(kind) {
			continue;
		}
		let is_exported = index > 0
			&& tokens[index - 1].kind == TokenKind::Export
			&& matches!(kind, TokenKind::Type | TokenKind::Namespace);
		let declares_a_name = tokens
			.get(index + 1)
			.is_some_and(|token| is_identifier_like(token.kind));
		if !is_exported && !declares_a_name {
			tokens[index].kind = TokenKind::Identifier;
		}
	}
}

/// `start` is only a keyword before a call or a block, like `start fetch(url)` or `start do { }`,
/// so that it can still be used as a name, like `range.start` or `Range(start = 1)`.
fn resolve_start_keywords(tokens: &mut [Token]) {
	for index in 0..tokens.len() {
		if tokens[index].kind != TokenKind::Start {
			continue;
		}
		let next_kind = tokens.get(index + 1).map(|token| token.kind);
		let follows_a_dot = index > 0
			&& matches!(
				tokens[index - 1].kind,
				TokenKind::Dot | TokenKind::QuestionMarkDot
			);
		if follows_a_dot || !matches!(next_kind, Some(TokenKind::Identifier | TokenKind::Do)) {
			tokens[index].kind = TokenKind::Identifier;
		}
	}
}

fn is_contextual_keyword(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::Type | TokenKind::Reactive | TokenKind::Derived | TokenKind::Namespace
	)
}

/// A token that can be a declared name: a name, or a contextual keyword (`start` included, which
/// is a name after a declaration keyword, like in `reactive start = 0`).
fn is_identifier_like(kind: TokenKind) -> bool {
	matches!(kind, TokenKind::Identifier | TokenKind::Start) || is_contextual_keyword(kind)
}
