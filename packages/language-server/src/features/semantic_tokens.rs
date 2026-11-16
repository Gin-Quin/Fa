//! `textDocument/semanticTokens/full`: syntax highlighting from the tokenizer, refined with the
//! declarations of the syntax tree.
//!
//! Without the analyzer, a name is classified by how it is written: a declared name by its
//! declaration, a PascalCase name is a type, a name followed by `(` is a function, a name after
//! `.` is a property.

use std::collections::HashMap;

use fa_parser::tokenize::tokenize;
use fa_parser::tokens::{Token, TokenKind};
use fa_parser::{Node, TypedSyntaxTree};
use tower_lsp::lsp_types::{
	SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend,
};

use crate::document::Document;
use crate::line_index::PositionEncoding;

/// The token types, in the order of the legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TokenType {
	Namespace,
	Type,
	TypeParameter,
	Parameter,
	Variable,
	Property,
	EnumMember,
	Function,
	Method,
	Macro,
	Keyword,
	Comment,
	String,
	Number,
	Operator,
}

const TOKEN_TYPES: [SemanticTokenType; 15] = [
	SemanticTokenType::NAMESPACE,
	SemanticTokenType::TYPE,
	SemanticTokenType::TYPE_PARAMETER,
	SemanticTokenType::PARAMETER,
	SemanticTokenType::VARIABLE,
	SemanticTokenType::PROPERTY,
	SemanticTokenType::ENUM_MEMBER,
	SemanticTokenType::FUNCTION,
	SemanticTokenType::METHOD,
	SemanticTokenType::MACRO,
	SemanticTokenType::KEYWORD,
	SemanticTokenType::COMMENT,
	SemanticTokenType::STRING,
	SemanticTokenType::NUMBER,
	SemanticTokenType::OPERATOR,
];

/// The token modifiers, as bits in the order of the legend.
pub const DECLARATION: u32 = 1 << 0;
pub const READONLY: u32 = 1 << 1;

const TOKEN_MODIFIERS: [SemanticTokenModifier; 2] = [
	SemanticTokenModifier::DECLARATION,
	SemanticTokenModifier::READONLY,
];

pub fn legend() -> SemanticTokensLegend {
	SemanticTokensLegend {
		token_types: TOKEN_TYPES.to_vec(),
		token_modifiers: TOKEN_MODIFIERS.to_vec(),
	}
}

/// A classified piece of the source, in bytes, on a single line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Highlight {
	pub start: usize,
	pub end: usize,
	pub token_type: TokenType,
	pub modifiers: u32,
}

/// The semantic tokens of a document, delta-encoded for LSP.
pub fn semantic_tokens(
	tree: &TypedSyntaxTree,
	document: &Document,
	encoding: PositionEncoding,
) -> Vec<SemanticToken> {
	encode(&highlights(tree, document), document, encoding)
}

/// The classified tokens of a document, in source order, split at line breaks.
pub fn highlights(tree: &TypedSyntaxTree, document: &Document) -> Vec<Highlight> {
	let source: &str = &tree.source;
	let declarations = declarations(tree);
	let mut highlights = Vec::new();
	classify_range(source, 0, source.len(), &declarations, &mut highlights);

	// split the tokens on several lines (comments, strings): not every client supports them
	let mut split = Vec::with_capacity(highlights.len());
	let index = &document.line_index;
	for highlight in highlights {
		let first_line = index.line_of(highlight.start);
		let last_line = index.line_of(highlight.end);
		if first_line == last_line {
			split.push(highlight);
			continue;
		}
		for line in first_line..=last_line {
			let line_start = index.line_start(line).unwrap_or(0);
			let start = highlight.start.max(line_start);
			let end = highlight.end.min(index.line_end(&document.text, line));
			if start < end {
				split.push(Highlight {
					start,
					end,
					..highlight
				});
			}
		}
	}
	split
}

fn encode(
	highlights: &[Highlight],
	document: &Document,
	encoding: PositionEncoding,
) -> Vec<SemanticToken> {
	let mut tokens = Vec::with_capacity(highlights.len());
	let mut previous_line = 0;
	let mut previous_start = 0;
	for highlight in highlights {
		let start = document.position(highlight.start, encoding);
		let length = encoding.measure(
			document
				.text
				.get(highlight.start..highlight.end)
				.unwrap_or(""),
		);
		if length == 0 {
			continue;
		}
		let delta_line = start.line - previous_line;
		let delta_start = if delta_line == 0 {
			start.character - previous_start
		} else {
			start.character
		};
		tokens.push(SemanticToken {
			delta_line,
			delta_start,
			length: length as u32,
			token_type: highlight.token_type as u32,
			token_modifiers_bitset: highlight.modifiers,
		});
		previous_line = start.line;
		previous_start = start.character;
	}
	tokens
}

/// Classify the tokens of `source[start..end]`, recursing into the interpolations of strings.
fn classify_range(
	source: &str,
	start: usize,
	end: usize,
	declarations: &HashMap<usize, (TokenType, u32)>,
	out: &mut Vec<Highlight>,
) {
	let tokens: Vec<Token> = tokenize(&source[start..end])
		.into_iter()
		.map(|token| Token::new(token.kind, token.start + start, token.end + start))
		.filter(|token| token.kind != TokenKind::Stop)
		.collect();

	for (position, token) in tokens.iter().enumerate() {
		let previous = position
			.checked_sub(1)
			.and_then(|previous| tokens.get(previous))
			.map(|token| token.kind);
		let next = tokens.get(position + 1).map(|token| token.kind);
		let push = |out: &mut Vec<Highlight>, token_type: TokenType, modifiers: u32| {
			out.push(Highlight {
				start: token.start,
				end: token.end,
				token_type,
				modifiers,
			})
		};
		match token.kind {
			TokenKind::String => string(source, token.start, token.end, declarations, out),
			TokenKind::Copy if declarations.contains_key(&token.start) => {
				let (kind, modifiers) = declarations[&token.start];
				push(out, kind, modifiers);
			}
			TokenKind::Identifier => {
				let (token_type, modifiers) = classify_name(
					&source[token.start..token.end],
					token.start,
					previous,
					next,
					declarations,
				);
				push(out, token_type, modifiers);
			}
			kind => {
				if let Some(token_type) = token_type_of(kind) {
					push(out, token_type, 0);
				}
			}
		}
	}
}

fn classify_name(
	text: &str,
	start: usize,
	previous: Option<TokenKind>,
	next: Option<TokenKind>,
	declarations: &HashMap<usize, (TokenType, u32)>,
) -> (TokenType, u32) {
	if let Some(classification) = declarations.get(&start) {
		return *classification;
	}
	if text.starts_with('@') {
		return (TokenType::Macro, 0);
	}
	let is_call = matches!(
		next,
		Some(
			TokenKind::ParenthesisOpen
				| TokenKind::ParenthesisOpenFunctionDeclaration
				| TokenKind::GenericOpen
		)
	);
	let after_dot = matches!(previous, Some(TokenKind::Dot | TokenKind::QuestionMarkDot));
	let name = text.trim_start_matches('#');
	if name.starts_with(|character: char| character.is_ascii_uppercase()) {
		if after_dot {
			return (TokenType::EnumMember, 0);
		}
		return (TokenType::Type, 0);
	}
	match (after_dot, is_call) {
		(true, true) => (TokenType::Method, 0),
		(true, false) => (TokenType::Property, 0),
		(false, true) => (TokenType::Function, 0),
		(false, false) => (TokenType::Variable, 0),
	}
}

/// The class of a token that is not a name nor a string.
fn token_type_of(kind: TokenKind) -> Option<TokenType> {
	use TokenKind as K;
	Some(match kind {
		K::InlineComment | K::BlockComment => TokenType::Comment,
		K::Integer
		| K::NegativeInteger
		| K::BinaryInteger
		| K::NegativeBinaryInteger
		| K::OctalInteger
		| K::NegativeOctalInteger
		| K::HexadecimalInteger
		| K::NegativeHexadecimalInteger
		| K::BigInteger
		| K::NegativeBigInteger
		| K::Number => TokenType::Number,
		K::Error => TokenType::Type,
		K::True
		| K::False
		| K::None
		| K::Return
		| K::Continue
		| K::Break
		| K::Start
		| K::Copy
		| K::Type
		| K::Reactive
		| K::Derived
		| K::Namespace
		| K::If
		| K::Else
		| K::For
		| K::Parallel
		| K::While
		| K::Loop
		| K::Do
		| K::Watch
		| K::When
		| K::Export
		| K::Use
		| K::Reuse
		| K::Async
		| K::Await
		| K::Try
		| K::Throw
		| K::Provide
		| K::Assert
		| K::Not
		| K::Let
		| K::Function
		| K::Mutable
		| K::Static
		| K::Readonly
		| K::AtFor
		| K::AtIf
		| K::AtElse
		| K::AtTest
		| K::And
		| K::Or
		| K::Is
		| K::In
		| K::As
		| K::Then
		| K::With
		| K::Throws
		| K::Requires
		| K::Mutates
		| K::Invalidates
		| K::Catch => TokenType::Keyword,
		K::Minus
		| K::DoubleDot
		| K::DoubleDotEqual
		| K::TripleDot
		| K::QuestionMark
		| K::Percent
		| K::Plus
		| K::Star
		| K::DoubleStar
		| K::Slash
		| K::DoubleSlash
		| K::DoublePercent
		| K::DoubleQuestionMark
		| K::Equal
		| K::PlusEqual
		| K::MinusEqual
		| K::StarEqual
		| K::DoubleStarEqual
		| K::SlashEqual
		| K::DoubleSlashEqual
		| K::DoublePercentEqual
		| K::DotEqual
		| K::DoubleEqual
		| K::NotEqual
		| K::LessThan
		| K::LessThanOrEqual
		| K::GreaterThan
		| K::GreaterThanOrEqual
		| K::FatArrow
		| K::Union
		| K::Arrow
		| K::QuestionMarkDot => TokenType::Operator,
		K::Empty
		| K::Stop
		| K::End
		| K::String
		| K::Identifier
		| K::Invalid
		| K::ParenthesisClose
		| K::BracesClose
		| K::BracketsClose
		| K::Colon
		| K::Comma
		| K::ParenthesisOpen
		| K::ParenthesisOpenFunctionDeclaration
		| K::BracesOpen
		| K::BracketsOpen
		| K::GenericOpen
		| K::GenericClose
		| K::Dot => return None,
	})
}

/// A string token: its literal parts are strings, its interpolations (`{expression}`) are
/// classified like code.
fn string(
	source: &str,
	start: usize,
	end: usize,
	declarations: &HashMap<usize, (TokenType, u32)>,
	out: &mut Vec<Highlight>,
) {
	let bytes = source.as_bytes();
	let mut literal_start = start;
	let mut index = start + 1;
	while index < end {
		match bytes[index] {
			b'\\' => index += 2,
			b'{' if bytes.get(index + 1) == Some(&b'{') => index += 2,
			b'{' => {
				let content_start = index + 1;
				let content_end = interpolation_end(bytes, content_start, end);
				push_string(out, literal_start, content_start);
				if content_start < content_end {
					classify_range(source, content_start, content_end, declarations, out);
				}
				literal_start = content_end;
				index = content_end + 1;
			}
			_ => index += 1,
		}
	}
	push_string(out, literal_start, end);
}

fn push_string(out: &mut Vec<Highlight>, start: usize, end: usize) {
	if start < end {
		out.push(Highlight {
			start,
			end,
			token_type: TokenType::String,
			modifiers: 0,
		});
	}
}

/// The offset of the `}` that closes an interpolation whose content starts at `start`, or
/// `end` if it is not closed. Nested strings and braces are skipped.
fn interpolation_end(bytes: &[u8], start: usize, end: usize) -> usize {
	let mut depth = 0;
	let mut index = start;
	while index < end {
		match bytes[index] {
			b'"' => {
				// a nested string, with its own interpolations
				index += 1;
				while index < end && bytes[index] != b'"' {
					match bytes[index] {
						b'\\' => index += 1,
						b'{' if bytes.get(index + 1) == Some(&b'{') => index += 1,
						b'{' => index = interpolation_end(bytes, index + 1, end),
						_ => {}
					}
					index += 1;
				}
			}
			b'{' => depth += 1,
			b'}' if depth == 0 => return index,
			b'}' => depth -= 1,
			_ => {}
		}
		index += 1;
	}
	end
}

/// The class of the declared names, by the offset of the name.
fn declarations(tree: &TypedSyntaxTree) -> HashMap<usize, (TokenType, u32)> {
	let mut declarations = HashMap::new();
	let mut implicit_members = Vec::new();
	let mut declare = |name: fa_parser::Name, token_type: TokenType, modifiers: u32| {
		if !name.is_empty() {
			declarations.insert(name.start as usize, (token_type, DECLARATION | modifiers));
		}
	};
	let identifier = |index: usize| match tree.nodes.get(index) {
		Some(Node::Identifier { name }) => Some(*name),
		Some(Node::Assignment { name, .. }) => match tree.nodes.get(*name) {
			Some(Node::Identifier { name }) => Some(*name),
			_ => None,
		},
		_ => None,
	};
	let is_function = |index: Option<usize>| {
		matches!(
			index.and_then(|index| tree.nodes.get(index)),
			Some(Node::ArrowFunction { .. })
		)
	};

	for node in &tree.nodes {
		match node {
			Node::Function {
				name, parameters, ..
			} => {
				declare(*name, TokenType::Function, 0);
				for parameter in parameters {
					if let Some(name) = identifier(*parameter) {
						declare(name, TokenType::TypeParameter, 0);
					}
				}
			}
			Node::Type {
				name,
				parameters,
				expression,
				..
			} => {
				declare(*name, TokenType::Type, 0);
				for parameter in parameters {
					if let Some(name) = identifier(*parameter) {
						declare(name, TokenType::TypeParameter, 0);
					}
				}
				if let Some(Node::Members { items }) = tree.nodes.get(*expression) {
					for item in items {
						if let Some(Node::Assignment { .. }) = tree.nodes.get(*item)
							&& let Some(name) = identifier(*item)
						{
							declare(name, TokenType::Property, 0);
						}
					}
				}
			}
			Node::Namespace { name, .. } => declare(*name, TokenType::Namespace, 0),
			Node::Let {
				name, expression, ..
			} => {
				if is_function(*expression) {
					declare(*name, TokenType::Function, 0);
				} else {
					declare(*name, TokenType::Variable, READONLY);
				}
			}
			Node::Static { name, .. } | Node::Derived { name, .. } => {
				declare(*name, TokenType::Variable, READONLY)
			}
			Node::Mutable { name, .. }
			| Node::Reactive { name, .. }
			| Node::Use { name, .. }
			| Node::Reuse { name, .. } => declare(*name, TokenType::Variable, 0),
			Node::ArrowFunction { signature, .. } => {
				for parameter in &signature.generics {
					if let Some(name) = identifier(*parameter) {
						declare(name, TokenType::TypeParameter, 0);
					}
				}
				for parameter in &signature.parameters {
					if let Some(name) = identifier(*parameter) {
						declare(name, TokenType::Parameter, 0);
					}
				}
			}
			Node::ImplicitMember { name } if !name.is_empty() => {
				implicit_members.push(name.start as usize);
			}
			_ => {}
		}
	}
	for start in implicit_members {
		declarations.insert(start, (TokenType::EnumMember, 0));
	}
	// `copy` before a block delimiter is still a name in a condition or
	// an iterable. The syntax tree resolves that contextual ambiguity.
	for node in &tree.nodes {
		if let Node::Identifier { name } = node
			&& tree.name(*name) == "copy"
		{
			declarations
				.entry(name.start as usize)
				.or_insert((TokenType::Variable, 0));
		}
	}
	declarations
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The highlights as `text:type` (with `+d` for a declaration, `+r` for read-only).
	fn classes(source: &str) -> Vec<String> {
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		highlights(&tree, &document)
			.iter()
			.map(|highlight| {
				let mut modifiers = String::new();
				if highlight.modifiers & DECLARATION != 0 {
					modifiers.push_str("+d");
				}
				if highlight.modifiers & READONLY != 0 {
					modifiers.push_str("+r");
				}
				format!(
					"{}:{:?}{modifiers}",
					&source[highlight.start..highlight.end],
					highlight.token_type
				)
			})
			.collect()
	}

	#[test]
	fn copy_keyword_and_ordinary_copy_names_are_distinct() {
		let classified =
			classes("let copy = true\nif copy { console.log(copy) }\nlet own = copy [1]");
		let copied: Vec<&str> = classified
			.iter()
			.map(String::as_str)
			.filter(|name| name.starts_with("copy:"))
			.collect();
		assert_eq!(
			copied,
			[
				"copy:Variable+d+r",
				"copy:Variable",
				"copy:Variable",
				"copy:Keyword"
			]
		);
	}

	#[test]
	fn legend_matches_the_token_types() {
		let legend = legend();
		assert_eq!(
			legend.token_types[TokenType::Operator as usize],
			SemanticTokenType::OPERATOR
		);
		assert_eq!(
			legend.token_types[TokenType::Namespace as usize],
			SemanticTokenType::NAMESPACE
		);
		assert_eq!(
			legend.token_types[TokenType::Keyword as usize],
			SemanticTokenType::KEYWORD
		);
		assert_eq!(legend.token_modifiers.len(), 2);
	}

	#[test]
	fn declaration_keywords_and_numbers() {
		assert_eq!(
			classes("let a = 1\nmutable b = 2.5"),
			[
				"let:Keyword",
				"a:Variable+d+r",
				"=:Operator",
				"1:Number",
				"mutable:Keyword",
				"b:Variable+d",
				"=:Operator",
				"2.5:Number"
			]
		);
	}

	#[test]
	fn function_declaration_and_parameters() {
		assert_eq!(
			classes("function add = (a: Integer, b) => a + b"),
			[
				"function:Keyword",
				"add:Function+d",
				"=:Operator",
				"a:Parameter+d",
				"Integer:Type",
				"b:Parameter+d",
				"=>:Operator",
				"a:Variable",
				"+:Operator",
				"b:Variable"
			]
		);
	}

	#[test]
	fn let_with_a_lambda_is_a_function() {
		assert_eq!(classes("let f = () => 1")[1], "f:Function+d");
	}

	#[test]
	fn types_fields_and_namespaces() {
		assert_eq!(
			classes("type Point = { x: Integer }\nnamespace geometry = { }"),
			[
				"type:Keyword",
				"Point:Type+d",
				"=:Operator",
				"x:Property+d",
				"Integer:Type",
				"namespace:Keyword",
				"geometry:Namespace+d",
				"=:Operator"
			]
		);
	}

	#[test]
	fn generic_lists() {
		assert_eq!(
			classes(
				"type Pair<Left, Right> = { left: Left }\nlet n = first<Integer>(items) < limit"
			),
			[
				"type:Keyword",
				"Pair:Type+d",
				"Left:TypeParameter+d",
				"Right:TypeParameter+d",
				"=:Operator",
				"left:Property+d",
				"Left:Type",
				"let:Keyword",
				"n:Variable+d+r",
				"=:Operator",
				"first:Function",
				"Integer:Type",
				"items:Variable",
				"<:Operator",
				"limit:Variable"
			]
		);
		assert_eq!(
			classes("function first = <Item>(items: Array<Item>) => items"),
			[
				"function:Keyword",
				"first:Function+d",
				"=:Operator",
				"Item:TypeParameter+d",
				"items:Parameter+d",
				"Array:Type",
				"Item:Type",
				"=>:Operator",
				"items:Variable"
			]
		);
	}

	#[test]
	fn contextual_keyword_used_as_a_name() {
		// `type` is a keyword only when it declares something
		let classes = classes("let type = 1");
		assert_eq!(classes[1], "type:Variable+d+r");
	}

	#[test]
	fn calls_properties_and_methods() {
		assert_eq!(
			classes("console.log(user.name, user.greet())"),
			[
				"console:Variable",
				"log:Method",
				"user:Variable",
				"name:Property",
				"user:Variable",
				"greet:Method"
			]
		);
		assert_eq!(classes("print(1)"), ["print:Function", "1:Number"]);
	}

	#[test]
	fn pascal_case_names_are_types() {
		assert_eq!(
			classes("let a: Array<String> = Shape.Circle"),
			[
				"let:Keyword",
				"a:Variable+d+r",
				"Array:Type",
				"String:Type",
				"=:Operator",
				"Shape:Type",
				"Circle:EnumMember"
			]
		);
	}

	#[test]
	fn implicit_members() {
		let classes = classes("let c: Color = .red");
		assert_eq!(classes.last().unwrap(), "red:EnumMember");
	}

	#[test]
	fn keywords_and_literals() {
		assert_eq!(
			classes("if a and not b { return true } else { return none }"),
			[
				"if:Keyword",
				"a:Variable",
				"and:Keyword",
				"not:Keyword",
				"b:Variable",
				"return:Keyword",
				"true:Keyword",
				"else:Keyword",
				"return:Keyword",
				"none:Keyword"
			]
		);
	}

	#[test]
	fn operators() {
		assert_eq!(
			classes("x = a ** 2 // 3 %% 4 ?? 5"),
			[
				"x:Variable",
				"=:Operator",
				"a:Variable",
				"**:Operator",
				"2:Number",
				"//:Operator",
				"3:Number",
				"%%:Operator",
				"4:Number",
				"??:Operator",
				"5:Number"
			]
		);
	}

	#[test]
	fn numbers_of_every_base() {
		let classes = classes("let a = [0xFF, 0b101, 0o17, -3, 12n, 1.5e3]");
		for number in ["0xFF", "0b101", "0o17", "-3", "12n", "1.5e3"] {
			assert!(
				classes.contains(&format!("{number}:Number")),
				"{number}: {classes:?}"
			);
		}
	}

	#[test]
	fn comments() {
		assert_eq!(
			classes("-- a comment\nlet a = 1 -- trailing"),
			[
				"-- a comment:Comment",
				"let:Keyword",
				"a:Variable+d+r",
				"=:Operator",
				"1:Number",
				"-- trailing:Comment"
			]
		);
	}

	#[test]
	fn block_comment_is_split_by_line() {
		assert_eq!(
			classes("---\nfirst\n---"),
			["---:Comment", "first:Comment", "---:Comment"]
		);
	}

	#[test]
	fn string_interpolation() {
		assert_eq!(
			classes("\"Hello, {user.name}!\""),
			[
				"\"Hello, {:String",
				"user:Variable",
				"name:Property",
				"}!\":String"
			]
		);
	}

	#[test]
	fn nested_string_in_interpolation() {
		assert_eq!(
			classes("\"a{x ?? \"b{y}\"}c\""),
			[
				"\"a{:String",
				"x:Variable",
				"??:Operator",
				"\"b{:String",
				"y:Variable",
				"}\":String",
				"}c\":String"
			]
		);
	}

	#[test]
	fn escaped_braces_are_part_of_the_string() {
		assert_eq!(classes("\"{{ not json\""), ["\"{{ not json\":String"]);
	}

	#[test]
	fn multi_line_string_is_split_by_line() {
		assert_eq!(classes("\"one\ntwo\""), ["\"one:String", "two\":String"]);
	}

	#[test]
	fn compile_time_names() {
		let classes = classes("static v = @readFile(\"v.txt\")");
		assert!(
			classes.contains(&"@readFile:Macro".to_string()),
			"{classes:?}"
		);
	}

	#[test]
	fn invalid_characters_are_skipped() {
		assert_eq!(classes("a & b"), ["a:Variable", "b:Variable"]);
	}

	#[test]
	fn delta_encoding_in_utf16() {
		let source = "let s = \"é😀\"\n  let t = 1";
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		let tokens = semantic_tokens(&tree, &document, PositionEncoding::Utf16);
		let compact: Vec<(u32, u32, u32)> = tokens
			.iter()
			.map(|token| (token.delta_line, token.delta_start, token.length))
			.collect();
		assert_eq!(
			compact,
			[
				(0, 0, 3), // let
				(0, 4, 1), // s
				(0, 2, 1), // =
				(0, 2, 5), // "é😀": 1 + 1 + 2 + 1 UTF-16 units
				(1, 2, 3), // let
				(0, 4, 1), // t
				(0, 2, 1), // =
				(0, 2, 1), // 1
			]
		);
		let utf8 = semantic_tokens(&tree, &document, PositionEncoding::Utf8);
		assert_eq!(utf8[3].length, 8);
	}

	#[test]
	fn tokens_after_cjk_text_on_the_same_line() {
		let source = "let a = \"日本\" + b";
		let tree = fa_parser::parse(source);
		let document = Document::new(source.to_string(), 0);
		let tokens = semantic_tokens(&tree, &document, PositionEncoding::Utf16);
		// `"日本"` starts at 8 and is 4 UTF-16 units long; `+` is 1 after it
		assert_eq!(tokens[3].delta_start, 2);
		assert_eq!(tokens[3].length, 4);
		assert_eq!(tokens[4].delta_start, 5);
		assert_eq!(tokens[5].delta_start, 2);
	}

	#[test]
	fn empty_document() {
		assert!(classes("").is_empty());
	}
}
