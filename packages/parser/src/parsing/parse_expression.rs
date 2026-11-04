use crate::context::{Context, MAX_DEPTH};
use crate::nodes::Node;
use crate::parsing::is_stop_token::{can_start_expression, is_stop_token};
use crate::parsing::parse_block_body::parse_keyword_block;
use crate::parsing::parse_declaration::{
	TypeDeclarationKind, VariableDeclarationKind, parse_generic_function, parse_reuse,
	parse_type_declaration, parse_variable_declaration,
};
use crate::parsing::parse_export::parse_export;
use crate::parsing::parse_expression_right::{
	RightExpressionResult, can_start_range_bound, parse_expression_right,
};
use crate::parsing::parse_for::parse_for;
use crate::parsing::parse_function_declaration::parse_function_declaration;
use crate::parsing::parse_if::parse_if;
use crate::parsing::parse_list::parse_list;
use crate::parsing::parse_members::{parse_member_expression, parse_members};
use crate::parsing::parse_number::{check_big_integer, parse_float, parse_integer};
use crate::parsing::parse_parameters::parse_parameters;
use crate::parsing::parse_string::parse_string;
use crate::parsing::parse_test::{is_test_declaration, parse_test};
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
		return context.skip_too_deep_code();
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

	// each new node that contains the previous one, like each call of `f()()`, nests the tree
	// one level deeper
	let mut links = 0;
	while !context.done() {
		match parse_expression_right(context, priority, stop_at, left) {
			Stop => break,
			Continue => {}
			Yield(right) => {
				left = right;
				links += 1;
				context.depth += 1;
				if context.depth >= MAX_DEPTH {
					context.skip_too_deep_code();
					break;
				}
			}
		}
	}
	context.depth -= links;

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
			check_big_integer(context);
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
			context.error_with_code(
				"E-NUM-1",
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
		TokenKind::Loop => parse_keyword_block(context, "loop", |body| Node::Loop { body }),
		TokenKind::Do => parse_keyword_block(context, "do", |body| Node::Do { body }),
		TokenKind::Watch => parse_keyword_block(context, "watch", |body| Node::Watch { body }),
		TokenKind::BracesOpen => parse_members(context),
		TokenKind::BracketsOpen => parse_list(context),
		// the parameters of a function, followed by `:`, `=>`, `throws`, `requires`...
		TokenKind::ParenthesisOpenFunctionDeclaration => parse_parameters(context),
		// a group or a tuple (function calls are parsed in `parse_expression_right`)
		TokenKind::ParenthesisOpen => parse_group(context),
		TokenKind::Function => parse_function_declaration(context, false, start),
		// a generic function, like `<Item>(items: Array<Item>) => …`
		TokenKind::GenericOpen => parse_generic_function(context, stop_at),
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

	if context.kind() != TokenKind::ParenthesisClose {
		let found = context.describe_token();
		context.error_at_token(format!("Expected `)` to close the `(`, found {found}"));
		// skip the rest of the group, up to its `)` if it is on the same line
		context.skip_to_statement_end();
	}
	let end = if context.kind() == TokenKind::ParenthesisClose {
		let end = context.token().end;
		context.go_to_next_token();
		end
	} else {
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
