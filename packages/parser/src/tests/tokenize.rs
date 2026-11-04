use super::helpers::assert_tokenizer_error;
use crate::tokenize::{tokenize, tokenize_with_diagnostics};
use crate::tokens::TokenKind;

fn assert_tokens(input: &str, expected_kinds: Vec<TokenKind>) {
	let tokens = tokenize(input);
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
	assert_tokens("  \n identifier \t \n ", vec![TokenKind::Identifier]);
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
		vec![TokenKind::BlockComment],
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
			TokenKind::ParenthesisOpenFunctionDeclaration,
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

	// Test none
	assert_tokens("none", vec![TokenKind::None]);
	assert_tokens("Error", vec![TokenKind::Error]);

	// Test identifiers
	assert_tokens("variable", vec![TokenKind::Identifier]);
	assert_tokens("_underscore", vec![TokenKind::Identifier]);
	assert_tokens("camelCase", vec![TokenKind::Identifier]);
	assert_tokens("PascalCase", vec![TokenKind::Identifier]);
	assert_tokens("snake_case", vec![TokenKind::Identifier]);
	// names are ASCII
	assert_tokens("😀", vec![TokenKind::Invalid]);
	assert_tokenizer_error("😀");

	// Test strings
	assert_tokens("\"hello\"", vec![TokenKind::String]);
	assert_tokens("\"Hello {name}\"", vec![TokenKind::String]);
}

#[test]
fn keywords() {
	// Test all keywords
	assert_tokens("let", vec![TokenKind::Let]);
	assert_tokens("function", vec![TokenKind::Function]);
	assert_tokens("mutable", vec![TokenKind::Mutable]);
	assert_tokens("static", vec![TokenKind::Static]);
	assert_tokens("type", vec![TokenKind::Identifier]);
	assert_tokens("type =", vec![TokenKind::Identifier, TokenKind::Equal]);
	assert_tokens(
		"export type =",
		vec![TokenKind::Export, TokenKind::Type, TokenKind::Equal],
	);
	assert_tokens("type foo", vec![TokenKind::Type, TokenKind::Identifier]);
	// `union`, `errors`, `enum` and `fields` are not keywords: types are declared with
	// `type X = Union { ... }`, `type X = Errors { ... }`, `type X = Enum { ... }`...
	assert_tokens("union", vec![TokenKind::Identifier]);
	assert_tokens(
		"union foo",
		vec![TokenKind::Identifier, TokenKind::Identifier],
	);
	assert_tokens("errors", vec![TokenKind::Identifier]);
	assert_tokens("enum", vec![TokenKind::Identifier]);
	assert_tokens("fields", vec![TokenKind::Identifier]);
	assert_tokens("reactive", vec![TokenKind::Identifier]);
	assert_tokens(
		"reactive foo",
		vec![TokenKind::Reactive, TokenKind::Identifier],
	);
	assert_tokens("derived", vec![TokenKind::Identifier]);
	assert_tokens(
		"derived foo",
		vec![TokenKind::Derived, TokenKind::Identifier],
	);
	assert_tokens("namespace", vec![TokenKind::Identifier]);
	assert_tokens(
		"namespace foo",
		vec![TokenKind::Namespace, TokenKind::Identifier],
	);
	assert_tokens("return", vec![TokenKind::Return]);
	assert_tokens("if", vec![TokenKind::If]);
	assert_tokens("else", vec![TokenKind::Else]);
	assert_tokens("for", vec![TokenKind::For]);
	assert_tokens("@for", vec![TokenKind::AtFor]);
	assert_tokens("while", vec![TokenKind::While]);
	assert_tokens("loop", vec![TokenKind::Loop]);
	assert_tokens("do", vec![TokenKind::Do]);
	assert_tokens("when", vec![TokenKind::When]);
	assert_tokens("use", vec![TokenKind::Use]);
	assert_tokens("reuse", vec![TokenKind::Reuse]);
	assert_tokens("async", vec![TokenKind::Async]);
	assert_tokens("await", vec![TokenKind::Await]);
	assert_tokens("yield", vec![TokenKind::Identifier]);
	assert_tokens("parallel", vec![TokenKind::Parallel]);
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
fn contextual_keyword_type() {
	assert_tokens("type Foo", vec![TokenKind::Type, TokenKind::Identifier]);
	assert_tokens("type type", vec![TokenKind::Type, TokenKind::Identifier]);
	assert_tokens("let type", vec![TokenKind::Let, TokenKind::Identifier]);
	assert_tokens(
		"let type = 12",
		vec![
			TokenKind::Let,
			TokenKind::Identifier,
			TokenKind::Equal,
			TokenKind::Integer,
		],
	);
	assert_tokens("type", vec![TokenKind::Identifier]);
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
	assert_tokens("%", vec![TokenKind::Percent]);
	assert_tokens("%%", vec![TokenKind::DoublePercent]);
	assert_tokens("%%=", vec![TokenKind::DoublePercentEqual]);
	assert_tokens("??", vec![TokenKind::DoubleQuestionMark]);
	assert_tokens("?", vec![TokenKind::QuestionMark]);
	assert_tokens("?.", vec![TokenKind::QuestionMarkDot]);
	assert_tokens("->", vec![TokenKind::Arrow]);
	assert_tokens("!=", vec![TokenKind::NotEqual]);

	assert_tokens(".", vec![TokenKind::Dot]);
	assert_tokens("..", vec![TokenKind::DoubleDot]);
	assert_tokens("..=", vec![TokenKind::DoubleDotEqual]);
	assert_tokens("...", vec![TokenKind::TripleDot]);

	// Compound assignments
	assert_tokens("+=", vec![TokenKind::PlusEqual]);
	assert_tokens("-=", vec![TokenKind::MinusEqual]);
	assert_tokens("*=", vec![TokenKind::StarEqual]);
	assert_tokens("**=", vec![TokenKind::DoubleStarEqual]);
	assert_tokens("/=", vec![TokenKind::SlashEqual]);
	assert_tokens("//=", vec![TokenKind::DoubleSlashEqual]);

	// Removed operators
	assert_tokens(">>", vec![TokenKind::GreaterThan, TokenKind::GreaterThan]);
	// the first `<` is not followed by a space: it opens a generic list
	assert_tokens("<<", vec![TokenKind::GenericOpen, TokenKind::LessThan]);
}

#[test]
fn exclamation_mark_is_not_an_operator() {
	assert_tokenizer_error("value!");
}

#[test]
fn ampersand_is_not_an_operator() {
	assert_tokenizer_error("a & b");
}

#[test]
fn word_operators() {
	// Test operators that are full words
	assert_tokens("and", vec![TokenKind::And]);
	assert_tokens("or", vec![TokenKind::Or]);
	assert_tokens("not", vec![TokenKind::Not]);
	assert_tokens("is", vec![TokenKind::Is]);
	assert_tokens("in", vec![TokenKind::In]);
	assert_tokens("as", vec![TokenKind::As]);
	assert_tokens("then", vec![TokenKind::Then]);

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
			TokenKind::ParenthesisOpenFunctionDeclaration,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
		],
	);
	assert_tokens(
		"(x)=>",
		vec![
			TokenKind::ParenthesisOpenFunctionDeclaration,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose,
			TokenKind::FatArrow,
		],
	);
	assert_tokens(
		"(x, y)=>",
		vec![
			TokenKind::ParenthesisOpenFunctionDeclaration,
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
fn compile_time_and_private_names() {
	assert_tokens("@foo", vec![TokenKind::Identifier]);
	assert_tokens("@type", vec![TokenKind::Identifier]);
	assert_tokens("#bag", vec![TokenKind::Identifier]);
	assert_tokens("@for", vec![TokenKind::AtFor]);
	assert_tokens(
		"let @foo = 12",
		vec![
			TokenKind::Let,
			TokenKind::Identifier,
			TokenKind::Equal,
			TokenKind::Integer,
		],
	);
	assert_tokens(
		"self.#bag",
		vec![TokenKind::Identifier, TokenKind::Dot, TokenKind::Identifier],
	);
}

#[test]
fn static_list_literal_is_removed() {
	assert_tokenizer_error("@[1, 2]");
}

#[test]
fn hash_requires_a_name() {
	assert_tokenizer_error("# comment");
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
			TokenKind::ParenthesisOpenFunctionDeclaration,
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

#[test]
fn contextual_keyword_at_end_of_line() {
	// the token after a contextual keyword used as a name must not be lost
	assert_tokens(
		"let x = type\nif a { b }",
		vec![
			TokenKind::Let,
			TokenKind::Identifier,
			TokenKind::Equal,
			TokenKind::Identifier,
			TokenKind::Stop,
			TokenKind::If,
			TokenKind::Identifier,
			TokenKind::BracesOpen,
			TokenKind::Identifier,
			TokenKind::BracesClose,
		],
	);
}

#[test]
fn start_keyword() {
	assert_tokens(
		"start fetch(url)",
		vec![
			TokenKind::Start,
			TokenKind::Identifier,
			TokenKind::ParenthesisOpen,
			TokenKind::Identifier,
			TokenKind::ParenthesisClose,
		],
	);
	assert_tokens(
		"start do {}",
		vec![
			TokenKind::Start,
			TokenKind::Do,
			TokenKind::BracesOpen,
			TokenKind::BracesClose,
		],
	);
	assert_tokens(
		"range.start = 2",
		vec![
			TokenKind::Identifier,
			TokenKind::Dot,
			TokenKind::Identifier,
			TokenKind::Equal,
			TokenKind::Integer,
		],
	);
	assert_tokens(
		"start\nend",
		vec![
			TokenKind::Identifier,
			TokenKind::Stop,
			TokenKind::Identifier,
		],
	);
}

/* ---------------------------- Generic lists ---------------------------- */

/// The tokens, written short: `<` and `>` for the generic brackets, `lt`, `le`, `gt` and `ge`
/// for the comparisons, `¶` for a new line, and the text of the other tokens.
fn short_tokens(input: &str) -> String {
	let (tokens, _) = tokenize_with_diagnostics(input);
	tokens
		.iter()
		.map(|token| match token.kind {
			TokenKind::GenericOpen => "<".to_string(),
			TokenKind::GenericClose => ">".to_string(),
			TokenKind::LessThan => "lt".to_string(),
			TokenKind::LessThanOrEqual => "le".to_string(),
			TokenKind::GreaterThan => "gt".to_string(),
			TokenKind::GreaterThanOrEqual => "ge".to_string(),
			TokenKind::Stop => "¶".to_string(),
			_ => input[token.start..token.end].to_string(),
		})
		.collect::<Vec<_>>()
		.join(" ")
}

fn assert_short_tokens(input: &str, expected: &str) {
	assert_eq!(short_tokens(input), expected, "\nInput: {input:?}");
}

fn tokenizer_messages(input: &str) -> Vec<String> {
	tokenize_with_diagnostics(input)
		.1
		.into_iter()
		.map(|diagnostic| diagnostic.message)
		.collect()
}

#[test]
fn generic_lists() {
	assert_short_tokens("Array<Integer>", "Array < Integer >");
	assert_short_tokens(
		"Map<String, Array<Integer>>",
		"Map < String , Array < Integer > >",
	);
	assert_short_tokens("Integer<min = 0, max = 9>", "Integer < min = 0 , max = 9 >");
	assert_short_tokens("first<Integer>(items)", "first < Integer > ( items )");
	assert_short_tokens("Pair<A, B> { left = a }", "Pair < A , B > { left = a }");
	assert_short_tokens("Set<Float>.from(values)", "Set < Float > . from ( values )");
	assert_short_tokens(
		"type Pair<Left, Right> = {}",
		"type Pair < Left , Right > = { }",
	);
	assert_short_tokens("Bag<in Self->friends>", "Bag < in Self -> friends >");
	assert_short_tokens(
		"Handler<(event: Event) => Void>",
		"Handler < ( event : Event ) => Void >",
	);
	assert_short_tokens(
		"Omit<Entity, { password }>",
		"Omit < Entity , { password } >",
	);
	// whatever comes before it, a `<` not followed by a space opens a list
	assert_short_tokens("1<2", "1 < 2");
	assert_short_tokens("a <b>", "a < b >");
	assert_short_tokens("(<T>)", "( < T > )");
	// `>>` is always two tokens
	assert_short_tokens("a >> b", "a gt gt b");
	assert_short_tokens("A<B<C<D>>>", "A < B < C < D > > >");
	assert_short_tokens("<<", "< lt");
}

#[test]
fn comparisons_next_to_generic_lists() {
	assert_short_tokens("a < b", "a lt b");
	assert_short_tokens("a\t<\tb", "a lt b");
	assert_short_tokens("a > b", "a gt b");
	assert_short_tokens("a>b", "a gt b");
	assert_short_tokens("a <= b", "a le b");
	assert_short_tokens("a<=b", "a le b");
	assert_short_tokens("x >= y", "x ge y");
	assert_short_tokens("0 <= index < length", "0 le index lt length");
	// a `<` at the end of the file has no item after it
	assert_short_tokens("a <", "a lt");
	assert_short_tokens("a<", "a lt");
}

#[test]
fn comparisons_inside_generic_lists_are_in_brackets() {
	assert_short_tokens("Storage<(size > 255)>", "Storage < ( size gt 255 ) >");
	assert_short_tokens("Storage<(size >= 256)>", "Storage < ( size ge 256 ) >");
	assert_short_tokens("Storage<(a < b)>", "Storage < ( a lt b ) >");
	assert_short_tokens("Check<[a > b]>", "Check < [ a gt b ] >");
	assert_short_tokens("Check<{ ok = a > b }>", "Check < { ok = a gt b } >");
	assert_short_tokens("Check<(a > b), c>", "Check < ( a gt b ) , c >");
	// a list inside brackets inside a list
	assert_short_tokens("A<(B<C> > d)>", "A < ( B < C > gt d ) >");
	// a `<` followed by a space is a comparison inside a list too
	assert_short_tokens("A<b < c>", "A < b lt c >");
	// out of the list, `>` is a comparison again
	assert_short_tokens("A<B> > c", "A < B > gt c");
	assert_short_tokens("A<B>>c", "A < B > gt c");
}

#[test]
fn comparison_operators_keep_priority_over_generic_brackets() {
	// `<=` is never a generic bracket
	assert_short_tokens("a<=b>", "a le b gt");
	// `=>` and `->` are read before `>`
	assert_short_tokens("A<(x) => y>", "A < ( x ) => y >");
	assert_short_tokens("A<b->c>", "A < b -> c >");
	// `>=` in a list is reported, and read as the end of the list followed by `=`
	assert_short_tokens("Box<T>= 1", "Box < T > = 1");
	let messages = tokenizer_messages("type Box<T>= Integer");
	assert_eq!(messages.len(), 1, "{messages:?}");
	assert!(messages[0].contains("write `> =` with a space, like `Box<T> = value`"));
	assert!(tokenizer_messages("Box<T> = 1").is_empty());
	assert!(tokenizer_messages("x >= y").is_empty());
	assert!(tokenizer_messages("List<(x >= y)>").is_empty());
	assert!(tokenizer_messages("[x >= y]").is_empty());
}

#[test]
fn generic_lists_and_new_lines() {
	// `>` is a closing token: a line continues before it
	assert_short_tokens("Map<String,\n\tInteger\n>", "Map < String , Integer >");
	assert_short_tokens(
		"type Pair<Left\n\tRight\n> = {}",
		"type Pair < Left ¶ Right > = { }",
	);
	assert_short_tokens("Array<Integer>\nnext", "Array < Integer > ¶ next");
	// a `<` followed by a new line is a comparison, which continues on the next line
	assert_short_tokens(
		"Map<\n\tString,\n\tInteger\n>",
		"Map lt String , Integer gt",
	);
	assert_short_tokens("a<\nb", "a lt b");
	assert_short_tokens("a <\n b", "a lt b");
	assert_short_tokens("a\n< b", "a lt b");
}

#[test]
fn minus_after_generic_lists() {
	assert_tokens(
		"Offset<-1>",
		vec![
			TokenKind::Identifier,
			TokenKind::GenericOpen,
			TokenKind::NegativeInteger,
			TokenKind::GenericClose,
		],
	);
	assert_tokens(
		"Size<T>-1",
		vec![
			TokenKind::Identifier,
			TokenKind::GenericOpen,
			TokenKind::Identifier,
			TokenKind::GenericClose,
			TokenKind::Minus,
			TokenKind::Integer,
		],
	);
}

#[test]
fn unclosed_generic_lists() {
	// a closing bracket drops the lists opened inside it
	assert_short_tokens("f(a <b) > c", "f ( a < b ) gt c");
	assert_short_tokens("[a <b] > c", "[ a < b ] gt c");
	assert_short_tokens("{ a <b } > c", "{ a < b } gt c");
	assert_short_tokens("f(A<B<C) > d", "f ( A < B < C ) gt d");
	// otherwise, the next `>` closes the list, even on another line
	assert_short_tokens("a <b\nc > d", "a < b ¶ c > d");
	// the tokenizer doesn't report them: the parser does, where they open
	assert!(tokenizer_messages("a <b").is_empty());
	assert!(tokenizer_messages("f(a <b)").is_empty());
	// a mismatched closing bracket closes the brackets up to its opening one
	assert_short_tokens("([A<b]) > c", "( [ A < b ] ) gt c");
	assert_short_tokens("(A<b] > c", "( A < b ] gt c");
}

#[test]
fn generic_lists_in_strings() {
	assert_tokens(r#""{a<b>}""#, vec![TokenKind::String]);
	assert_tokens(r#""a<b""#, vec![TokenKind::String]);
	// an interpolation is tokenized on its own, with its own brackets
	assert_short_tokens(r#"A<"x{a <b}" > c"#, r#"A < "x{a <b}" > c"#);
}

#[test]
fn garbled_brackets_never_panic() {
	let pieces = [
		"<", ">", "(", ")", "[", "]", "{", "}", " ", "\n", "a", ">=", "<=", "=", ",",
	];
	let mut state: u64 = 12345;
	for _ in 0..2000 {
		let mut source = String::new();
		for _ in 0..12 {
			state = state
				.wrapping_mul(6364136223846793005)
				.wrapping_add(1442695040888963407);
			source.push_str(pieces[(state >> 33) as usize % pieces.len()]);
		}
		let (tokens, _) = tokenize_with_diagnostics(&source);
		let opened = tokens
			.iter()
			.filter(|token| token.kind == TokenKind::GenericOpen)
			.count();
		let closed = tokens
			.iter()
			.filter(|token| token.kind == TokenKind::GenericClose)
			.count();
		assert!(closed <= opened, "{source:?}");
	}
}
