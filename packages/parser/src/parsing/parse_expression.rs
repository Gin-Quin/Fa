use crate::context::{Context, MAX_DEPTH};
use crate::nodes::{DeclarationKind, Node, TestModifier};
use crate::parsing::is_stop_token::{can_start_expression, is_stop_token};
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_do::parse_do;
use crate::parsing::parse_expression_right::{
	RightExpressionResult, can_start_range_bound, parse_expression_right,
};
use crate::parsing::parse_for::parse_for;
use crate::parsing::parse_function_declaration::parse_function_declaration;
use crate::parsing::parse_if::parse_if;
use crate::parsing::parse_list::parse_list;
use crate::parsing::parse_loop::parse_loop;
use crate::parsing::parse_members::{parse_member_expression, parse_members};
use crate::parsing::parse_number::{parse_big_integer, parse_float, parse_integer};
use crate::parsing::parse_parameters::parse_parameters;
use crate::parsing::parse_pattern::{is_pattern_start, parse_pattern};
use crate::parsing::parse_string::parse_string;
use crate::parsing::parse_when::parse_when;
use crate::parsing::parse_while::parse_while;
use crate::priority::Priority;
use crate::tokens::TokenKind;

use RightExpressionResult::{Continue, Stop, Yield};

#[derive(Clone, Copy)]
pub struct ExpressionContext {
	pub is_statement_start: bool,
	pub in_members: bool,
}

impl ExpressionContext {
	pub fn new(is_statement_start: bool, in_members: bool) -> Self {
		Self {
			is_statement_start,
			in_members,
		}
	}

	pub fn expression() -> Self {
		Self::new(false, false)
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VariableDeclarationKind {
	Let,
	Static,
	Mutable,
	Use,
	MutableUse,
	Reactive,
	Derived,
}

impl VariableDeclarationKind {
	fn keyword(self) -> &'static str {
		match self {
			VariableDeclarationKind::Let => "let",
			VariableDeclarationKind::Static => "static",
			VariableDeclarationKind::Mutable => "mutable",
			VariableDeclarationKind::Use => "use",
			VariableDeclarationKind::MutableUse => "mutable use",
			VariableDeclarationKind::Reactive => "reactive",
			VariableDeclarationKind::Derived => "derived",
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TypeDeclarationKind {
	Type,
	Namespace,
}

/// Tokens that end a type annotation, like `Integer` in `let value: Integer = 12`.
pub(crate) const TYPE_ANNOTATION_STOP_TOKENS: [TokenKind; 5] = [
	TokenKind::Equal,
	TokenKind::Comma,
	TokenKind::ParenthesisClose,
	TokenKind::BracesClose,
	TokenKind::BracketsClose,
];

/// Parse an expression: a left expression, then the operators, calls and accesses that apply to
/// it, as long as they bind tighter than `priority`. Stops before the tokens of `stop_at`.
/// Always returns a node: what can't be parsed becomes `Node::Invalid`, with a diagnostic.
pub fn parse_expression<'source>(
	context: &mut Context<'source>,
	priority: Priority,
	expression_context: ExpressionContext,
	stop_at: &[TokenKind],
) -> usize {
	if context.depth >= MAX_DEPTH {
		if !context.depth_exceeded {
			context.depth_exceeded = true;
			context.error_at_token(format!(
				"This code is nested too deeply (more than {MAX_DEPTH} levels): split it into smaller functions"
			));
		}
		let start = context.token().start;
		context.skip_to_statement_end();
		let end = context.last_end();
		return context.insert(Node::Invalid, start, end);
	}

	context.depth += 1;
	let result = parse_expression_inner(context, priority, expression_context, stop_at);
	context.depth -= 1;
	result
}

fn parse_expression_inner<'source>(
	context: &mut Context<'source>,
	priority: Priority,
	expression_context: ExpressionContext,
	stop_at: &[TokenKind],
) -> usize {
	if expression_context.in_members && expression_context.is_statement_start {
		return parse_member_expression(context, stop_at);
	}

	let Some(mut left) = parse_left_expression(context, expression_context, stop_at) else {
		return report_missing_expression(context, stop_at);
	};

	if let Node::Function { name, .. } = context.at(left) {
		let name = *name;
		context.add_hoisted_symbol(name, left);
	}

	while !context.done() {
		match parse_expression_right(context, priority, stop_at, left) {
			Stop => break,
			Continue => {}
			Yield(right) => left = right,
		}
	}

	left
}

/// Report that an expression is expected at the current token, and return an `Invalid` node.
/// The token is skipped, unless it can end the enclosing construct.
fn report_missing_expression(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let token = context.token();
	let kind = token.kind;
	let is_synchronizing = matches!(kind, TokenKind::Stop | TokenKind::End | TokenKind::Comma)
		|| kind.is_closing()
		|| is_stop_token(stop_at, kind);

	let found = context.describe_token();
	let previous = context.last_token().kind;
	let message = if previous == TokenKind::Empty || previous == TokenKind::Stop {
		format!("Expected an expression, found {found}")
	} else {
		format!(
			"Expected an expression after {}, found {found}",
			previous.describe()
		)
	};
	context.error_at_token(message);

	if is_synchronizing {
		context.insert_missing()
	} else {
		context.go_to_next_token();
		context.insert(Node::Invalid, token.start, token.end)
	}
}

/// Parse the left side of an expression: a literal, a name, a prefix operator, a declaration,
/// a group... Return `None` (without moving) if the current token can't start an expression.
fn parse_left_expression<'source>(
	context: &mut Context<'source>,
	expression_context: ExpressionContext,
	stop_at: &[TokenKind],
) -> Option<usize> {
	let token = context.token();
	let start = token.start;
	let end = token.end;

	macro_rules! Single {
		($node:expr) => {{
			let node = $node;
			context.go_to_next_token();
			context.insert(node, start, end)
		}};
	}

	let index = match token.kind {
		TokenKind::Identifier
			if expression_context.is_statement_start
				&& context.slice() == "test"
				&& is_test_declaration(context) =>
		{
			parse_test(context, false)
		}
		TokenKind::Identifier => {
			let name = context.name();
			Single!(Node::Identifier { name })
		}
		TokenKind::Invalid => {
			// already reported by the tokenizer
			Single!(Node::Invalid)
		}
		TokenKind::Integer
		| TokenKind::NegativeInteger
		| TokenKind::BinaryInteger
		| TokenKind::NegativeBinaryInteger
		| TokenKind::OctalInteger
		| TokenKind::NegativeOctalInteger
		| TokenKind::HexadecimalInteger
		| TokenKind::NegativeHexadecimalInteger => {
			let value = parse_integer(context);
			Single!(Node::Integer(value))
		}
		TokenKind::BigInteger | TokenKind::NegativeBigInteger => {
			parse_big_integer(context);
			let text = context.name();
			Single!(Node::BigInteger(text))
		}
		TokenKind::Number => {
			let value = parse_float(context);
			Single!(Node::Number(value))
		}
		TokenKind::String => parse_string(context),
		TokenKind::None => Single!(Node::None),
		TokenKind::Error => Single!(Node::Error),
		TokenKind::True => Single!(Node::Boolean(true)),
		TokenKind::False => Single!(Node::Boolean(false)),
		TokenKind::Continue => Single!(Node::Continue),
		// `.5`
		TokenKind::Dot
			if context.lookup_next_token_kind() == TokenKind::Integer
				&& context.next_token().start == end =>
		{
			let digits = context.next_token();
			let text = context.text(start, digits.end);
			context.error(
				format!("`{text}` is not a number: write `0{text}`"),
				start,
				digits.end,
			);
			context.go_to_next_token();
			context.go_to_next_token();
			context.insert(Node::Invalid, start, digits.end)
		}
		// a member of the expected type, like `.red`
		TokenKind::Dot
			if context.lookup_next_token_kind() == TokenKind::Identifier
				&& context.next_token().start == end =>
		{
			context.go_to_next_token();
			let name = context.name();
			let name_end = context.token().end;
			context.go_to_next_token();
			context.insert(Node::ImplicitMember { name }, start, name_end)
		}
		TokenKind::Minus => parse_prefix(context, Priority::Prefix, stop_at, |right| {
			Node::Negate { right }
		}),
		TokenKind::TripleDot => {
			let next = context.lookup_next_token_kind();
			if !can_start_expression(next) || is_stop_token(stop_at, next) {
				Single!(Node::Placeholder)
			} else {
				parse_prefix(context, Priority::Prefix, stop_at, |right| Node::Spread {
					right,
				})
			}
		}
		// many-to-many relation target, ex: `Bag(in Self->bestFriends)`
		TokenKind::In => parse_prefix(context, Priority::Exponentiation, stop_at, |right| {
			Node::InRelation { right }
		}),
		TokenKind::DoubleDot | TokenKind::DoubleDotEqual => {
			let inclusive = token.kind == TokenKind::DoubleDotEqual;
			context.go_to_next_token();
			let range_end = parse_range_end(context, inclusive, stop_at);
			let end = range_end.map_or(end, |range_end| context.span(range_end).end);
			context.insert(
				Node::Range {
					start: None,
					end: range_end,
					inclusive,
				},
				start,
				end,
			)
		}
		TokenKind::Not => {
			parse_prefix(context, Priority::Not, stop_at, |right| Node::Not { right })
		}
		TokenKind::Try => parse_try(context, stop_at),
		TokenKind::Await => parse_prefix(context, Priority::TryAwaitStart, stop_at, |right| {
			Node::Await { right }
		}),
		TokenKind::Start => parse_prefix(context, Priority::TryAwaitStart, stop_at, |right| {
			Node::Start { right }
		}),
		TokenKind::Throw => parse_prefix(context, Priority::PrefixKeyword, stop_at, |right| {
			Node::Throw { right }
		}),
		TokenKind::Assert => parse_prefix(context, Priority::PrefixKeyword, stop_at, |right| {
			Node::Assert { right }
		}),
		TokenKind::Return | TokenKind::Break => {
			context.go_to_next_token();
			let next = context.kind();
			let expression = if can_start_expression(next) && !is_stop_token(stop_at, next) {
				Some(parse_expression(
					context,
					Priority::PrefixKeyword,
					ExpressionContext::expression(),
					stop_at,
				))
			} else {
				None
			};
			let end = expression.map_or(end, |expression| context.span(expression).end);
			let node = if token.kind == TokenKind::Return {
				Node::Return { expression }
			} else {
				Node::Break { expression }
			};
			context.insert(node, start, end)
		}
		TokenKind::Provide => parse_provide(context, stop_at),
		TokenKind::Let => {
			parse_variable_declaration(context, VariableDeclarationKind::Let, stop_at)
		}
		TokenKind::Mutable => {
			let kind = if context.lookup_next_token_kind() == TokenKind::Use {
				context.go_to_next_token();
				VariableDeclarationKind::MutableUse
			} else {
				VariableDeclarationKind::Mutable
			};
			parse_variable_declaration(context, kind, stop_at)
		}
		TokenKind::Use => {
			parse_variable_declaration(context, VariableDeclarationKind::Use, stop_at)
		}
		TokenKind::Static => {
			parse_variable_declaration(context, VariableDeclarationKind::Static, stop_at)
		}
		TokenKind::Reactive => {
			parse_variable_declaration(context, VariableDeclarationKind::Reactive, stop_at)
		}
		TokenKind::Derived => {
			parse_variable_declaration(context, VariableDeclarationKind::Derived, stop_at)
		}
		TokenKind::Reuse => parse_reuse(context, stop_at),
		TokenKind::Type => parse_type_declaration(context, TypeDeclarationKind::Type, stop_at),
		TokenKind::Namespace => {
			parse_type_declaration(context, TypeDeclarationKind::Namespace, stop_at)
		}
		TokenKind::Export => parse_export(context, stop_at),
		TokenKind::For => parse_for(context, false),
		TokenKind::AtFor => parse_for(context, true),
		TokenKind::If => parse_if(context, false),
		TokenKind::AtIf => parse_if(context, true),
		TokenKind::AtTest => parse_test(context, true),
		TokenKind::When => parse_when(context),
		TokenKind::While => parse_while(context),
		TokenKind::Loop => parse_loop(context),
		TokenKind::Do => parse_do(context),
		TokenKind::Watch => {
			context.go_to_next_token();
			let body = parse_block(context, "watch");
			let end = context.last_end();
			context.insert(Node::Watch { body }, start, end)
		}
		TokenKind::BracesOpen => parse_members(context),
		TokenKind::BracketsOpen => parse_list(context),
		// the parameters of a function, followed by `:`, `=>`, `throws`, `requires`...
		TokenKind::ParenthesisOpenFunctionDeclaration => parse_parameters(context),
		// a group or a tuple (function calls are parsed in `parse_expression_right`)
		TokenKind::ParenthesisOpen => parse_group(context),
		TokenKind::Function => parse_function_declaration(context, false, start),
		TokenKind::Async => parse_async(context, stop_at),
		_ => return None,
	};

	Some(index)
}

/// Parse a prefix operator and its operand, parsed with `priority`.
fn parse_prefix<'source>(
	context: &mut Context<'source>,
	priority: Priority,
	stop_at: &[TokenKind],
	make: impl FnOnce(usize) -> Node,
) -> usize {
	let start = context.token().start;
	context.go_to_next_token();
	let right = parse_expression(context, priority, ExpressionContext::expression(), stop_at);
	let end = context.span(right).end;
	context.insert(make(right), start, end)
}

/// Parse the end of a range after `..` or `..=`, if there is one.
pub(crate) fn parse_range_end(
	context: &mut Context<'_>,
	inclusive: bool,
	stop_at: &[TokenKind],
) -> Option<usize> {
	if can_start_range_bound(context.kind()) && !is_stop_token(stop_at, context.kind()) {
		Some(parse_expression(
			context,
			Priority::Range,
			ExpressionContext::expression(),
			stop_at,
		))
	} else {
		if inclusive {
			context.error_at_token("An inclusive range `..=` must have an end, like `1..=10`");
		}
		None
	}
}

/// `( expression )`, `(a, b)` or `()`
fn parse_group(context: &mut Context<'_>) -> usize {
	let open = context.token();
	context.go_to_next_token();
	context.skip_stops();

	if context.kind() == TokenKind::ParenthesisClose {
		let end = context.token().end;
		context.go_to_next_token();
		return context.insert(Node::EmptyGroup, open.start, end);
	}

	let expression = parse_expression(
		context,
		Priority::None,
		ExpressionContext::expression(),
		&[TokenKind::ParenthesisClose],
	);
	context.skip_stops();

	let end = if context.kind() == TokenKind::ParenthesisClose {
		let end = context.token().end;
		context.go_to_next_token();
		end
	} else {
		let found = context.describe_token();
		context.error_at_token(format!("Expected `)` to close the `(`, found {found}"));
		context.last_end()
	};
	context.insert(Node::Group { expression }, open.start, end)
}

/// `try expression`, and `try target = value` when the target of an assignment can fail.
fn parse_try(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();
	let mut right = parse_expression(
		context,
		Priority::TryAwaitStart,
		ExpressionContext::expression(),
		stop_at,
	);
	// `try scores[index] = 0`: `try` covers an assignment whose target can fail
	if matches!(
		context.kind(),
		TokenKind::Equal
			| TokenKind::PlusEqual
			| TokenKind::MinusEqual
			| TokenKind::StarEqual
			| TokenKind::DoubleStarEqual
			| TokenKind::SlashEqual
			| TokenKind::DoubleSlashEqual
			| TokenKind::DoublePercentEqual
			| TokenKind::DotEqual
	) && let Yield(assignment) = parse_expression_right(context, Priority::Comma, stop_at, right)
	{
		right = assignment;
	}
	let end = context.span(right).end;
	context.insert(Node::Try { right }, start, end)
}

/// `provide Type = value`
fn parse_provide(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();
	let type_expression = parse_expression(
		context,
		Priority::Assignment,
		ExpressionContext::expression(),
		&[TokenKind::Equal],
	);
	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		)
	} else {
		context.error_at_token(
			"Expected `=` after the provided type, like `provide Database = PostgresDatabase(url)`",
		);
		context.insert_missing()
	};
	let end = context.span(expression).end;
	context.insert(
		Node::Provide {
			type_expression,
			expression,
		},
		start,
		end,
	)
}

/// `async function name = ...`, or an asynchronous lambda or function type, like
/// `async (request: Request) => Response`.
fn parse_async(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	if context.kind() == TokenKind::Function {
		return parse_function_declaration(context, true, start);
	}

	let expression = parse_expression(
		context,
		Priority::Assignment,
		ExpressionContext::expression(),
		stop_at,
	);
	if let Node::ArrowFunction { signature, .. } = context.tree.at_mutable(expression) {
		signature.is_async = true;
		context.tree.spans[expression].start = start;
		expression
	} else {
		let span = context.span(expression);
		context.error(
			"`async` must be followed by `function` or by a function, like `async (url: String) => ...`",
			start,
			span.end,
		);
		expression
	}
}

/// Return true if the `test` identifier at the current token starts a test declaration:
/// `test "name" { }`, `test skip "name" { }` or `test only "name" { }`.
pub(crate) fn is_test_declaration(context: &Context<'_>) -> bool {
	let next = context.next_token();
	match next.kind {
		TokenKind::String => true,
		TokenKind::Identifier => {
			let word = context.text(next.start, next.end);
			(word == "skip" || word == "only")
				&& context
					.tokens
					.get(context.token_index + 2)
					.map(|token| token.kind)
					== Some(TokenKind::String)
		}
		_ => false,
	}
}

/// Parse a test declaration: `test [skip | only] "name" { body }` or `@test "name" { body }`
fn parse_test(context: &mut Context<'_>, is_compile_time: bool) -> usize {
	let start = context.token().start;
	context.go_to_next_token(); // skip `test` or `@test`

	let mut modifier = None;
	if context.kind() == TokenKind::Identifier {
		modifier = match context.slice() {
			"skip" => Some(TestModifier::Skip),
			"only" => Some(TestModifier::Only),
			other => {
				context.error_at_token(format!(
					"Unknown test modifier `{other}`: write `test skip \"...\"` or `test only \"...\"`"
				));
				None
			}
		};
		context.go_to_next_token();
	}

	let name = if context.kind() == TokenKind::String {
		parse_string(context)
	} else {
		context
			.error_at_token("Expected the name of the test, like `test \"adds numbers\" { ... }`");
		context.insert_missing()
	};

	let body = parse_block(context, "test");
	let end = context.last_end();

	context.insert(
		Node::Test {
			name,
			modifier,
			body,
			is_compile_time,
		},
		start,
		end,
	)
}

/// Parse `let`, `mutable`, `use`, `mutable use`, `static`, `reactive` and `derived` declarations,
/// starting at the keyword: `let name: Type = value`, or a destructuring like `let { a } = b`.
fn parse_variable_declaration<'source>(
	context: &mut Context<'source>,
	kind: VariableDeclarationKind,
	stop_at: &[TokenKind],
) -> usize {
	let start = context.token().start;
	let keyword = kind.keyword();
	context.go_to_next_token();

	if is_pattern_start(context.kind()) {
		return parse_destructuring_declaration(context, kind, start, stop_at);
	}

	// `mutable type`, `let function`...: report the keyword, and parse the declaration
	if matches!(
		context.kind(),
		TokenKind::Type | TokenKind::Namespace | TokenKind::Function | TokenKind::Async
	) {
		let what = match context.kind() {
			TokenKind::Type => "a type",
			TokenKind::Namespace => "a namespace",
			_ => "a function",
		};
		let declaration = context.describe_token();
		let declaration_start = context.token().start;
		context.error(
			format!(
				"`{keyword}` can't declare {what}: remove `{keyword}` and write {declaration} alone"
			),
			start,
			declaration_start,
		);
		return parse_expression(
			context,
			Priority::None,
			ExpressionContext::new(true, false),
			stop_at,
		);
	}

	if context.kind() != TokenKind::Identifier {
		let found = context.describe_token();
		context.error_at_token(format!("Expected a name after `{keyword}`, found {found}"));
		context.skip_to_statement_end();
		let end = context.last_end();
		return context.insert(Node::Invalid, start, end);
	}

	let name_text = context.slice();
	let name = context.name();
	let name_end = context.token().end;
	context.go_to_next_token();

	let type_expression = if context.kind() == TokenKind::Colon {
		context.go_to_next_token();
		Some(parse_expression(
			context,
			Priority::Assignment,
			ExpressionContext::expression(),
			&TYPE_ANNOTATION_STOP_TOKENS,
		))
	} else {
		None
	};

	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		Some(parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		))
	} else {
		None
	};

	let requires_a_value = matches!(
		kind,
		VariableDeclarationKind::Use
			| VariableDeclarationKind::MutableUse
			| VariableDeclarationKind::Static
	);
	if expression.is_none() && (requires_a_value || type_expression.is_none()) {
		let message = if requires_a_value {
			format!("A `{keyword}` declaration needs a value: write `{keyword} {name_text} = ...`")
		} else {
			format!(
				"Expected `=` or `:` after `{keyword} {name_text}`: give it a value (`= ...`) or a type (`: Type`)"
			)
		};
		context.error_at_token(message);
	}

	if kind == VariableDeclarationKind::Let
		&& let Some(expression) = expression
		&& matches!(context.at(expression), Node::ArrowFunction { .. })
	{
		let span = context.span(expression);
		context.error(
			format!(
				"A function is declared with `function`, not `let`: write `function {name_text} = {}`",
				context.text(span.start, span.end)
			),
			start,
			name_end,
		);
	}

	let end = expression
		.or(type_expression)
		.map_or(name_end, |index| context.span(index).end);
	let node = match kind {
		VariableDeclarationKind::Let => Node::Let {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Mutable => Node::Mutable {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Reactive => Node::Reactive {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Derived => Node::Derived {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Use
		| VariableDeclarationKind::MutableUse
		| VariableDeclarationKind::Static => {
			let expression = match expression {
				Some(expression) => expression,
				None => context.insert_missing(),
			};
			if kind == VariableDeclarationKind::Static {
				Node::Static {
					name,
					type_expression,
					expression,
				}
			} else {
				Node::Use {
					name,
					type_expression,
					expression,
					is_mutable: kind == VariableDeclarationKind::MutableUse,
				}
			}
		}
	};

	context.insert(node, start, end)
}

/// Parse `reuse name = value`, which moves a `mutable use` reference
fn parse_reuse(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	if context.kind() != TokenKind::Identifier {
		context.error_at_token(
			"Expected the name of a `mutable use` reference after `reuse`, like `reuse best = player`",
		);
		context.skip_to_statement_end();
		let end = context.last_end();
		return context.insert(Node::Invalid, start, end);
	}

	let name_text = context.slice();
	let name = context.name();
	context.go_to_next_token();

	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		)
	} else {
		context.error_at_token(format!(
			"Expected `=` after `reuse {name_text}`, like `reuse {name_text} = other`"
		));
		context.insert_missing()
	};
	let end = context.span(expression).end;

	context.insert(Node::Reuse { name, expression }, start, end)
}

/// Parse a destructuring declaration, like `let { a, b as c } = object`
fn parse_destructuring_declaration(
	context: &mut Context<'_>,
	kind: VariableDeclarationKind,
	start: usize,
	stop_at: &[TokenKind],
) -> usize {
	let keyword = kind.keyword();
	let declaration_kind = match kind {
		VariableDeclarationKind::Let => DeclarationKind::Let,
		VariableDeclarationKind::Mutable => DeclarationKind::Mutable,
		VariableDeclarationKind::Use => DeclarationKind::Use,
		VariableDeclarationKind::MutableUse => DeclarationKind::MutableUse,
		VariableDeclarationKind::Static
		| VariableDeclarationKind::Reactive
		| VariableDeclarationKind::Derived => {
			context.error_at_token(format!(
				"A `{keyword}` value can't be destructured: declare it with a name, like `{keyword} name = ...`"
			));
			DeclarationKind::Let
		}
	};

	let pattern = parse_pattern(context);

	if context.kind() == TokenKind::Colon {
		context.error_at_token(
			"A destructuring declaration has no type annotation: the type comes from the value",
		);
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::Assignment,
			ExpressionContext::expression(),
			&TYPE_ANNOTATION_STOP_TOKENS,
		);
	}

	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		)
	} else {
		context.error_at_token(format!(
			"Expected `=` after the pattern: a destructuring declaration needs a value, like `{keyword} {{ a, b }} = object`"
		));
		context.insert_missing()
	};
	let end = context.span(expression).end;

	context.insert(
		Node::Destructuring {
			kind: declaration_kind,
			pattern,
			expression,
		},
		start,
		end,
	)
}

/// Parse `type Name(Parameters) = expression` or `namespace name = expression`.
fn parse_type_declaration(
	context: &mut Context<'_>,
	kind: TypeDeclarationKind,
	stop_at: &[TokenKind],
) -> usize {
	let start = context.token().start;
	let keyword = match kind {
		TypeDeclarationKind::Type => "type",
		TypeDeclarationKind::Namespace => "namespace",
	};
	context.go_to_next_token();

	// the tokenizer only makes `type` and `namespace` keywords when a name follows
	let name_text = context.slice();
	let name = context.name();
	context.go_to_next_token();

	let mut parameters = Vec::new();
	if matches!(
		context.kind(),
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration
	) {
		if kind == TypeDeclarationKind::Namespace {
			context.error_at_token(
				"A namespace has no parameters: only types and functions can be generic",
			);
		}
		parameters = parse_generic_parameters(context);
	}

	let expression = parse_declaration_value(
		context,
		&format!("{keyword} {name_text}"),
		ExpressionContext::new(kind == TypeDeclarationKind::Namespace, false),
		stop_at,
	);
	let end = context.span(expression).end;

	let node = match kind {
		TypeDeclarationKind::Type => Node::Type {
			name,
			parameters,
			expression,
		},
		TypeDeclarationKind::Namespace => Node::Namespace { name, expression },
	};
	context.insert(node, start, end)
}

/// Parse `= value` after a declared name (`declaration` is how the declaration starts, for
/// diagnostics). A missing `=` is reported, and the value is parsed anyway if there is one.
pub(crate) fn parse_declaration_value(
	context: &mut Context<'_>,
	declaration: &str,
	expression_context: ExpressionContext,
	stop_at: &[TokenKind],
) -> usize {
	if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
	} else {
		let found = context.describe_token();
		context.error_at_token(format!("Expected `=` after `{declaration}`, found {found}"));
		if !can_start_expression(context.kind()) || is_stop_token(stop_at, context.kind()) {
			return context.insert_missing();
		}
	}

	parse_expression(
		context,
		Priority::PrefixKeyword,
		expression_context,
		stop_at,
	)
}

/// Parse the generic parameters written after a declared name, like
/// `(Item)`, `(Value: Integer | Float)`, `(Index = Integer)` or `(rows: Integer)`.
/// A capitalized parameter is a type parameter, a lowercase one is a compile-time value.
pub(crate) fn parse_generic_parameters(context: &mut Context<'_>) -> Vec<usize> {
	let open = context.token();
	let (parameters, _) = crate::parsing::parse_delimited::parse_delimited(
		context,
		TokenKind::ParenthesisClose,
		|context| {
			if context.kind() != TokenKind::Identifier {
				let found = context.describe_token();
				context.error_at_token(format!(
					"Expected a generic parameter, like `Item`, `Value: Integer | Float` or `rows: Integer`, found {found}"
				));
			}
			let parameter = parse_expression(
				context,
				Priority::Comma,
				ExpressionContext::expression(),
				&[TokenKind::Comma, TokenKind::ParenthesisClose],
			);
			let name_and_type = match context.at(parameter) {
				Node::Identifier { name } => Some((*name, false)),
				Node::Assignment {
					name,
					type_expression,
					..
				} => match context.at(*name) {
					Node::Identifier { name } => Some((*name, type_expression.is_some())),
					_ => None,
				},
				_ => None,
			};
			let span = context.span(parameter);
			match name_and_type {
				Some((name, has_type)) => {
					let name_text = context.name_text(name);
					let is_type_parameter =
						name_text.starts_with(|character: char| character.is_ascii_uppercase());
					if !is_type_parameter && !has_type {
						context.error(
							format!(
								"The compile-time value parameter `{name_text}` needs a type, like `{name_text}: Integer`. Type parameters are capitalized"
							),
							span.start,
							span.end,
						);
					}
				}
				None => context.error(
					"Expected a generic parameter, like `Item`, `Value: Integer | Float` or `rows: Integer`",
					span.start,
					span.end,
				),
			}
			parameter
		},
	);

	if parameters.is_empty() {
		context.error(
			"Generic parameters can't be empty: remove the `()`",
			open.start,
			context.last_end(),
		);
	}

	parameters
}

/// Parse an export, starting at `export`:
/// `export = value`, `export: Type = value`, `export mutable: Type = value`,
/// `export function(Item) = ...`, `export async function = ...`, `export type(T) = ...`,
/// `export namespace = { ... }`.
fn parse_export(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	let node = match context.kind() {
		TokenKind::Function | TokenKind::Async => {
			let is_async = context.kind() == TokenKind::Async;
			if is_async {
				context.go_to_next_token();
				if context.kind() == TokenKind::Function {
					context.go_to_next_token();
				} else {
					context.error_at_token(
						"Expected `function` after `export async`, like `export async function = (): Integer => ...`",
					);
				}
			} else {
				context.go_to_next_token();
			}
			let parameters = parse_optional_generic_parameters(context);
			let type_expression = parse_optional_type_annotation(context);
			let expression = parse_export_value(context, stop_at);
			Node::ExportFunction {
				is_async,
				parameters,
				type_expression,
				expression,
			}
		}
		TokenKind::Type => {
			context.go_to_next_token();
			let parameters = parse_optional_generic_parameters(context);
			let expression = parse_export_value(context, stop_at);
			Node::ExportType {
				parameters,
				expression,
			}
		}
		TokenKind::Namespace => {
			context.go_to_next_token();
			let expression = parse_export_value(context, stop_at);
			Node::ExportNamespace { expression }
		}
		TokenKind::Mutable | TokenKind::Colon | TokenKind::Equal => {
			let is_mutable = context.kind() == TokenKind::Mutable;
			if is_mutable {
				context.go_to_next_token();
			}
			let type_expression = parse_optional_type_annotation(context);
			let expression = parse_export_value(context, stop_at);
			Node::ExportValue {
				is_mutable,
				type_expression,
				expression,
			}
		}
		_ => {
			let message = match context.kind() {
				TokenKind::Let
				| TokenKind::Static
				| TokenKind::Reactive
				| TokenKind::Derived
				| TokenKind::Use => {
					"A file exports one anonymous value, named after the file: write `export = value`. `reactive`, `derived` and `static` values can't be exported".to_string()
				}
				TokenKind::Identifier => format!(
					"An export has no name: the file's name is the name of its export. Write `export = {}` or `export function = ...`",
					context.slice()
				),
				_ => format!(
					"Expected `=`, `mutable`, `function`, `type` or `namespace` after `export`, found {}",
					context.describe_token()
				),
			};
			context.error_at_token(message);
			context.skip_to_statement_end();
			let end = context.last_end();
			return context.insert(Node::Invalid, start, end);
		}
	};

	let end = match &node {
		Node::ExportValue { expression, .. }
		| Node::ExportFunction { expression, .. }
		| Node::ExportType { expression, .. }
		| Node::ExportNamespace { expression } => context.span(*expression).end,
		_ => context.last_end(),
	};
	context.insert(node, start, end)
}

fn parse_optional_generic_parameters(context: &mut Context<'_>) -> Vec<usize> {
	if matches!(
		context.kind(),
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration
	) {
		parse_generic_parameters(context)
	} else {
		Vec::new()
	}
}

/// Parse `: Type` if the current token is `:`.
pub(crate) fn parse_optional_type_annotation(context: &mut Context<'_>) -> Option<usize> {
	if context.kind() != TokenKind::Colon {
		return None;
	}
	context.go_to_next_token();
	Some(parse_expression(
		context,
		Priority::Assignment,
		ExpressionContext::expression(),
		&TYPE_ANNOTATION_STOP_TOKENS,
	))
}

fn parse_export_value(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let is_namespace = context.last_token().kind == TokenKind::Namespace;
	parse_declaration_value(
		context,
		"export",
		ExpressionContext::new(is_namespace, false),
		stop_at,
	)
}
