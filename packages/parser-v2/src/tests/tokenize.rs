use crate::tokenize::tokenize;
use crate::tokens::TokenKind;

#[cfg(test)]
// Helper function to check if tokenization results match expected tokens
fn assert_tokens(input: &str, expected_kinds: Vec<TokenKind>) {
	let tokens = tokenize(input.as_bytes());
	assert_eq!(
		tokens.len(),
		expected_kinds.len(),
		"Expected {} tokens, got {}",
		expected_kinds.len(),
		tokens.len()
	);

	for (i, (token, expected_kind)) in tokens
		.iter()
		.zip(expected_kinds.iter())
		.enumerate() {
		assert_eq!(
			token.kind,
			*expected_kind,
			"Token at position {} doesn't match. Expected {:?}, got {:?}",
			i,
			expected_kind,
			token.kind
		);
	}
}

#[test]
fn parenthesis() {
	assert_tokens("()", vec![TokenKind::ParenthesisOpen, TokenKind::ParenthesisClose]);
	assert_tokens(
		"()=>",
		vec![TokenKind::ParametersStart, TokenKind::ParametersEnd, TokenKind::FatArrow]
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
	assert_tokens("return", vec![TokenKind::Return]);
	assert_tokens("if", vec![TokenKind::If]);
	assert_tokens("else", vec![TokenKind::Else]);
	assert_tokens("for", vec![TokenKind::For]);
	assert_tokens("while", vec![TokenKind::While]);
	assert_tokens("when", vec![TokenKind::When]);
	assert_tokens("use", vec![TokenKind::Use]);
	assert_tokens("async", vec![TokenKind::Async]);
	assert_tokens("await", vec![TokenKind::Await]);
	assert_tokens("yield", vec![TokenKind::Yield]);
	assert_tokens("exit", vec![TokenKind::Exit]);
	assert_tokens("continue", vec![TokenKind::Continue]);

	// Test keywords in context
	assert_tokens("if true", vec![TokenKind::If, TokenKind::Space, TokenKind::True]);
	assert_tokens(
		"return value",
		vec![TokenKind::Return, TokenKind::Space, TokenKind::Identifier]
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
		vec![
			TokenKind::Identifier,
			TokenKind::Space,
			TokenKind::And,
			TokenKind::Space,
			TokenKind::Identifier
		]
	);
	assert_tokens(
		"x or y",
		vec![
			TokenKind::Identifier,
			TokenKind::Space,
			TokenKind::Or,
			TokenKind::Space,
			TokenKind::Identifier
		]
	);
	assert_tokens("not x", vec![TokenKind::Not, TokenKind::Space, TokenKind::Identifier]);
	assert_tokens(
		"x is y",
		vec![
			TokenKind::Identifier,
			TokenKind::Space,
			TokenKind::Is,
			TokenKind::Space,
			TokenKind::Identifier
		]
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
			TokenKind::Space,
			TokenKind::Identifier
		]
	);
	assert_tokens(
		"x: y",
		vec![
			TokenKind::Identifier,
			TokenKind::Colon,
			TokenKind::Space,
			TokenKind::Identifier
		]
	);
}

#[test]
fn parenthesis_and_parameters() {
	// Test regular parenthesis
	assert_tokens("()", vec![TokenKind::ParenthesisOpen, TokenKind::ParenthesisClose]);
	assert_tokens(
		"(x)",
		vec![
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose
		]
	);

	// Test parameters with fat arrow
	assert_tokens(
		"()=>",
		vec![TokenKind::ParametersStart, TokenKind::ParametersEnd, TokenKind::FatArrow]
	);
	assert_tokens(
		"(x)=>",
		vec![
			TokenKind::ParametersStart,
			TokenKind::Identifier,
			TokenKind::ParametersEnd,
			TokenKind::FatArrow
		]
	);
	assert_tokens(
		"(x, y)=>",
		vec![
			TokenKind::ParametersStart,
			TokenKind::Identifier,
			TokenKind::Comma,
			TokenKind::Space,
			TokenKind::Identifier,
			TokenKind::ParametersEnd,
			TokenKind::FatArrow
		]
	);
}

#[test]
fn other_groups() {
	// Test braces
	assert_tokens("{}", vec![TokenKind::BracesOpen, TokenKind::BracesClose]);
	assert_tokens(
		"{x}",
		vec![TokenKind::BracesOpen, TokenKind::Identifier, TokenKind::BracesClose]
	);

	// Test brackets
	assert_tokens("[]", vec![TokenKind::BracketsOpen, TokenKind::BracketsClose]);
	assert_tokens(
		"[x]",
		vec![TokenKind::BracketsOpen, TokenKind::Identifier, TokenKind::BracketsClose]
	);
}

#[test]
fn complex_expressions() {
	// Test a more complex expression
	assert_tokens(
		"if (x == 10) { return true }",
		vec![
			TokenKind::If,
			TokenKind::Space,
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::Space,
			TokenKind::DoubleEqual,
			TokenKind::Space,
			TokenKind::Integer,
			TokenKind::ParenthesisClose,
			TokenKind::Space,
			TokenKind::BracesOpen,
			TokenKind::Space,
			TokenKind::Return,
			TokenKind::Space,
			TokenKind::True,
			TokenKind::Space,
			TokenKind::BracesClose
		]
	);

	// Test a function definition with parameters
	assert_tokens(
		"(x, y)=> { return x + y }",
		vec![
			TokenKind::ParametersStart,
			TokenKind::Identifier,
			TokenKind::Comma,
			TokenKind::Space,
			TokenKind::Identifier,
			TokenKind::ParametersEnd,
			TokenKind::FatArrow,
			TokenKind::Space,
			TokenKind::BracesOpen,
			TokenKind::Space,
			TokenKind::Return,
			TokenKind::Space,
			TokenKind::Identifier,
			TokenKind::Space,
			TokenKind::Plus,
			TokenKind::Space,
			TokenKind::Identifier,
			TokenKind::Space,
			TokenKind::BracesClose
		]
	);
}
