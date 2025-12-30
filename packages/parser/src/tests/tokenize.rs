use crate::tokenize::tokenize;
use crate::tokens::TokenKind;

#[cfg(test)]
// Helper function to check if tokenization results match expected tokens
fn assert_tokens(input: &str, expected_kinds: Vec<TokenKind>) {
	let tokens = tokenize(input.as_bytes());
	assert_eq!(
		tokens.len(),
		expected_kinds.len(),
		"\nError tokenizing input: '{}'\nExpected \n    {:?}\ngot \n    {:?}",
		input,
		expected_kinds,
		tokens.iter().map(|t| t.kind).collect::<Vec<_>>()
	);

	for (i, (token, expected_kind)) in tokens.iter().zip(expected_kinds.iter()).enumerate() {
		assert_eq!(
			token.kind, *expected_kind,
			"\nError tokenizing input: '{}'\nToken at position {} doesn't match. Expected {:?}, got {:?}",
			input, i, expected_kind, token.kind
		);
	}
}

#[test]
fn space_and_stop() {
	assert_tokens(" ", vec![]);
	assert_tokens(" \t ", vec![]);
	assert_tokens("  \n ", vec![]);
	assert_tokens("  \n \t \n ", vec![]);
	assert_tokens(
		"  \n identifier \t \n ",
		vec![TokenKind::Identifier, TokenKind::Stop],
	);
	assert_tokens(
		"  \n identifier \t \n .zabu",
		vec![TokenKind::Identifier, TokenKind::Dot, TokenKind::Identifier],
	);
	assert_tokens(
		"  \n identifier \t \n + zabu",
		vec![
			TokenKind::Identifier,
			TokenKind::Plus,
			TokenKind::Identifier,
		],
	);
	assert_tokens(
		"  \n identifier \t \n + \n\nzabu",
		vec![
			TokenKind::Identifier,
			TokenKind::Plus,
			TokenKind::Identifier,
		],
	);
}

#[test]
fn comments() {
	// Test single line comments
	assert_tokens("-- inline comment", vec![TokenKind::InlineComment]);
	assert_tokens(
		"-- this is an inline comment",
		vec![TokenKind::InlineComment],
	);
	assert_tokens("--empty", vec![TokenKind::InlineComment]);

	// Test single line comments after other tokens
	assert_tokens(
		"identifier -- single line comment",
		vec![TokenKind::Identifier, TokenKind::InlineComment],
	);
	assert_tokens(
		"123 -- number comment",
		vec![TokenKind::Integer, TokenKind::InlineComment],
	);

	// Test multi-line comments
	assert_tokens(
		"--- This is a multi-line comment---",
		vec![TokenKind::BlockComment],
	);
	assert_tokens(
		"--- This is a\n multi-line \ncomment---",
		vec![TokenKind::BlockComment],
	);
	assert_tokens(
		"--- This is a\n multi-line \ncomment---\n",
		vec![TokenKind::BlockComment],
	);
	assert_tokens(
		"--- This is a\n multi-line \ncomment---  \n  ",
		vec![TokenKind::BlockComment],
	);
	assert_tokens(
		"--- This is a\n multi-line \ncomment---  \n  \n",
		vec![TokenKind::BlockComment, TokenKind::Stop],
	);
	assert_tokens(
		"identifier --- comment after token ---",
		vec![TokenKind::Identifier, TokenKind::BlockComment],
	);
	assert_tokens(
		"--- comment before token --- identifier",
		vec![TokenKind::BlockComment, TokenKind::Identifier],
	);
}

#[test]
fn parenthesis() {
	assert_tokens(
		"()",
		vec![TokenKind::ParenthesisOpen, TokenKind::ParenthesisClose],
	);
	assert_tokens(
		"()=>",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
		],
	);
}

#[test]
fn primitives() {
	// Test integers
	assert_tokens("123", vec![TokenKind::Integer]);
	assert_tokens("0", vec![TokenKind::Integer]);

	// Test booleans
	assert_tokens("true", vec![TokenKind::True]);
	assert_tokens("false", vec![TokenKind::False]);

	// Test null
	assert_tokens("null", vec![TokenKind::Null]);

	// Test identifiers
	assert_tokens("variable", vec![TokenKind::Identifier]);
	assert_tokens("_underscore", vec![TokenKind::Identifier]);
	assert_tokens("camelCase", vec![TokenKind::Identifier]);
	assert_tokens("PascalCase", vec![TokenKind::Identifier]);
	assert_tokens("snake_case", vec![TokenKind::Identifier]);
}

#[test]
fn keywords() {
	// Test all keywords
	assert_tokens("let", vec![TokenKind::Let]);
	assert_tokens("function", vec![TokenKind::Function]);
	assert_tokens("mutable", vec![TokenKind::Mutable]);
	assert_tokens("static", vec![TokenKind::Static]);
	assert_tokens("type", vec![TokenKind::Type]);
	assert_tokens("union", vec![TokenKind::UnionKeyword]);
	assert_tokens("enum", vec![TokenKind::Enum]);
	assert_tokens("fields", vec![TokenKind::Fields]);
	assert_tokens("reactive", vec![TokenKind::Reactive]);
	assert_tokens("derived", vec![TokenKind::Derived]);
	assert_tokens("namespace", vec![TokenKind::Namespace]);
	assert_tokens("return", vec![TokenKind::Return]);
	assert_tokens("if", vec![TokenKind::If]);
	assert_tokens("else", vec![TokenKind::Else]);
	assert_tokens("for", vec![TokenKind::For]);
	assert_tokens("@for", vec![TokenKind::AtFor]);
	assert_tokens("while", vec![TokenKind::While]);
	assert_tokens("loop", vec![TokenKind::Loop]);
	assert_tokens("when", vec![TokenKind::When]);
	assert_tokens("use", vec![TokenKind::Use]);
	assert_tokens("async", vec![TokenKind::Async]);
	assert_tokens("await", vec![TokenKind::Await]);
	assert_tokens("yield", vec![TokenKind::Yield]);
	assert_tokens("exit", vec![TokenKind::Exit]);
	assert_tokens("continue", vec![TokenKind::Continue]);
	assert_tokens("break", vec![TokenKind::Break]);

	// Test keywords in context
	assert_tokens("if true", vec![TokenKind::If, TokenKind::True]);
	assert_tokens(
		"return value",
		vec![TokenKind::Return, TokenKind::Identifier],
	);
}

#[test]
fn operators_with_common_characters() {
	// Test operators with common characters
	assert_tokens("=", vec![TokenKind::Equal]);
	assert_tokens("==", vec![TokenKind::DoubleEqual]);
	assert_tokens("=>", vec![TokenKind::FatArrow]);

	assert_tokens("<", vec![TokenKind::LessThan]);
	assert_tokens("<=", vec![TokenKind::LessThanOrEqual]);

	assert_tokens(">", vec![TokenKind::GreaterThan]);
	assert_tokens(">=", vec![TokenKind::GreaterThanOrEqual]);

	assert_tokens("*", vec![TokenKind::Star]);
	assert_tokens("**", vec![TokenKind::DoubleStar]);

	assert_tokens("/", vec![TokenKind::Slash]);
	assert_tokens("//", vec![TokenKind::DoubleSlash]);
	assert_tokens("||>", vec![TokenKind::Compose]);
	assert_tokens("%", vec![TokenKind::Percent]);

	assert_tokens(".", vec![TokenKind::Dot]);
	assert_tokens("..", vec![TokenKind::DoubleDot]);
	assert_tokens("...", vec![TokenKind::TripleDot]);
}

#[test]
fn word_operators() {
	// Test operators that are full words
	assert_tokens("and", vec![TokenKind::And]);
	assert_tokens("or", vec![TokenKind::Or]);
	assert_tokens("not", vec![TokenKind::Not]);
	assert_tokens("is", vec![TokenKind::Is]);

	// Test in context
	assert_tokens(
		"x and y",
		vec![TokenKind::Identifier, TokenKind::And, TokenKind::Identifier],
	);
	assert_tokens(
		"x or y",
		vec![TokenKind::Identifier, TokenKind::Or, TokenKind::Identifier],
	);
	assert_tokens("not x", vec![TokenKind::Not, TokenKind::Identifier]);
	assert_tokens(
		"x is y",
		vec![TokenKind::Identifier, TokenKind::Is, TokenKind::Identifier],
	);
}

#[test]
fn chainable_operators() {
	// Test chainable operators
	assert_tokens(",", vec![TokenKind::Comma]);
	assert_tokens(":", vec![TokenKind::Colon]);

	// Test in context
	assert_tokens(
		"x, y",
		vec![
			TokenKind::Identifier,
			TokenKind::Comma,
			TokenKind::Identifier,
		],
	);
	assert_tokens(
		"x: y",
		vec![
			TokenKind::Identifier,
			TokenKind::Colon,
			TokenKind::Identifier,
		],
	);
}

#[test]
fn parenthesis_and_parameters() {
	// Test regular parenthesis
	assert_tokens(
		"()",
		vec![TokenKind::ParenthesisOpen, TokenKind::ParenthesisClose],
	);
	assert_tokens(
		"(x)",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose,
		],
	);

	// Test parameters with fat arrow
	assert_tokens(
		"()=>",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
		],
	);
	assert_tokens(
		"(x)=>",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
		],
	);
	assert_tokens(
		"(x, y)=>",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::Comma,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
		],
	);
}

#[test]
fn other_groups() {
	// Test braces
	assert_tokens("{}", vec![TokenKind::BracesOpen, TokenKind::BracesClose]);
	assert_tokens(
		"{x}",
		vec![
			TokenKind::BracesOpen,
			TokenKind::Identifier,
			TokenKind::BracesClose,
		],
	);

	// Test brackets
	assert_tokens(
		"[]",
		vec![TokenKind::BracketsOpen, TokenKind::BracketsClose],
	);
	assert_tokens(
		"[x]",
		vec![
			TokenKind::BracketsOpen,
			TokenKind::Identifier,
			TokenKind::BracketsClose,
		],
	);
}

#[test]
fn complex_expressions() {
	// Test a more complex expression
	assert_tokens(
		"if (x == 10) { return true }",
		vec![
			TokenKind::If,
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::DoubleEqual,
			TokenKind::Integer,
			TokenKind::ParenthesisClose,
			TokenKind::BracesOpen,
			TokenKind::Return,
			TokenKind::True,
			TokenKind::BracesClose,
		],
	);

	// Test a function definition with parameters
	assert_tokens(
		"(x, y)=> { return x + y }",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::Comma,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
			TokenKind::BracesOpen,
			TokenKind::Return,
			TokenKind::Identifier,
			TokenKind::Plus,
			TokenKind::Identifier,
			TokenKind::BracesClose,
		],
	);
}

#[test]
fn numbers() {
	// Test integers
	assert_tokens("123", vec![TokenKind::Integer]);
	assert_tokens("0", vec![TokenKind::Integer]);
	assert_tokens("-123", vec![TokenKind::NegativeInteger]);
	assert_tokens("-0", vec![TokenKind::NegativeInteger]);
	assert_tokens("123n", vec![TokenKind::BigInteger]);
	assert_tokens("-123n", vec![TokenKind::NegativeBigInteger]);

	// Test decimal numbers
	assert_tokens("123.456", vec![TokenKind::Number]);
	assert_tokens("0.123", vec![TokenKind::Number]);
	assert_tokens("-123.456", vec![TokenKind::Number]);
	assert_tokens("-0.123", vec![TokenKind::Number]);

	// Test binary notation (0b prefix)
	assert_tokens("0b101", vec![TokenKind::BinaryInteger]);
	assert_tokens("-0b101", vec![TokenKind::NegativeBinaryInteger]);

	// Test octal notation (0o prefix)
	assert_tokens("0o755", vec![TokenKind::OctalInteger]);
	assert_tokens("-0o755", vec![TokenKind::NegativeOctalInteger]);

	// Test hexadecimal notation (0x prefix)
	assert_tokens("0xFF", vec![TokenKind::HexadecimalInteger]);
	assert_tokens("-0xFF", vec![TokenKind::NegativeHexadecimalInteger]);
	assert_tokens("0xABC123", vec![TokenKind::HexadecimalInteger]);
	assert_tokens("-0xABC123", vec![TokenKind::NegativeHexadecimalInteger]);

	// Test numbers in context
	assert_tokens(
		"x = 123",
		vec![TokenKind::Identifier, TokenKind::Equal, TokenKind::Integer],
	);
	assert_tokens(
		"y = -123.456",
		vec![TokenKind::Identifier, TokenKind::Equal, TokenKind::Number],
	);
	assert_tokens(
		"z = 0xFF + 0b101",
		vec![
			TokenKind::Identifier,
			TokenKind::Equal,
			TokenKind::HexadecimalInteger,
			TokenKind::Plus,
			TokenKind::BinaryInteger,
		],
	);

	// Test scientific notation
	assert_tokens("1e10", vec![TokenKind::Number]);
	assert_tokens("1.5e10", vec![TokenKind::Number]);
	assert_tokens("1e+10", vec![TokenKind::Number]);
	assert_tokens("1e-10", vec![TokenKind::Number]);
	assert_tokens("1.5e-10", vec![TokenKind::Number]);
	assert_tokens("-1e10", vec![TokenKind::Number]);
	assert_tokens("-1.5e-10", vec![TokenKind::Number]);

	// Test scientific notation in context
	assert_tokens(
		"x = 1.5e-10",
		vec![TokenKind::Identifier, TokenKind::Equal, TokenKind::Number],
	);
	assert_tokens(
		"y = -1e10 * 2",
		vec![
			TokenKind::Identifier,
			TokenKind::Equal,
			TokenKind::Number,
			TokenKind::Star,
			TokenKind::Integer,
		],
	);
}
