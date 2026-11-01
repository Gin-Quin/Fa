use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::is_stop_token::can_start_expression;
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_test::is_test_declaration;
use crate::priority::Priority;
use crate::tokens::TokenKind;

const MEMBER_STOP_TOKENS: [TokenKind; 2] = [TokenKind::BracesClose, TokenKind::Comma];

/// Parse the members of an object literal, a type or a namespace, from `{` to `}`:
/// `{ a = 12, b: String = "Hello", #x, foo, ...other }`
pub(crate) fn parse_members(context: &mut Context<'_>) -> usize {
	let start = context.token().start;
	let (items, end) = parse_member_items(context);
	context.insert(Node::Members { items }, start, end)
}

/// Parse the members between `{` and `}`, and return them with the end of the `}`.
pub(crate) fn parse_member_items(context: &mut Context<'_>) -> (Vec<usize>, usize) {
	parse_delimited(context, TokenKind::BracesClose, |context| {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::new(true, true),
			&MEMBER_STOP_TOKENS,
		)
	})
}

/// Parse one member: a field (`name`, `name = value`, `name: Type = value`), a nested key set
/// (`name { a, b }`), a spread (`...other`), or a declaration (`function`, `type`, `mutable`...).
pub(crate) fn parse_member_expression(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	match context.kind() {
		TokenKind::Identifier if context.slice() == "test" && is_test_declaration(context) => {
			parse_declaration(context, stop_at)
		}
		TokenKind::Identifier => parse_member_identifier(context, stop_at),
		TokenKind::Function
		| TokenKind::Async
		| TokenKind::Type
		| TokenKind::Namespace
		| TokenKind::Let
		| TokenKind::Mutable
		| TokenKind::Use
		| TokenKind::Static
		| TokenKind::Reactive
		| TokenKind::Derived
		| TokenKind::Watch
		| TokenKind::AtTest
		| TokenKind::AtIf
		| TokenKind::AtFor
		| TokenKind::TripleDot => parse_declaration(context, stop_at),
		TokenKind::Readonly => {
			let start = context.token().start;
			context.go_to_next_token();
			let right = if context.kind() == TokenKind::Identifier {
				parse_member_identifier(context, stop_at)
			} else {
				context.error_at_token(
					"Expected a field after `readonly`, like `readonly id: Integer`",
				);
				context.insert_missing()
			};
			let end = context.span(right).end;
			context.insert(Node::Readonly { right }, start, end)
		}
		kind => {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected a member, like `name = value`, `name: Type` or `...other`, found {found}"
			));
			let token = context.token();
			if can_start_expression(kind) {
				// parse the expression to skip it as a whole
				parse_expression(
					context,
					Priority::None,
					ExpressionContext::expression(),
					stop_at,
				);
			} else if !kind.is_closing() && kind != TokenKind::End {
				context.go_to_next_token();
			}
			let end = context.last_end().max(token.start);
			context.insert(Node::Invalid, token.start, end)
		}
	}
}

fn parse_declaration(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(true, false),
		stop_at,
	)
}

fn parse_member_identifier(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let token = context.token();
	let name_text = context.slice();
	let name = context.name();
	let name_index = context.insert(Node::Identifier { name }, token.start, token.end);
	context.go_to_next_token();

	match context.kind() {
		TokenKind::Colon => {
			context.go_to_next_token();
			let type_expression = parse_expression(
				context,
				Priority::Assignment,
				ExpressionContext::expression(),
				&[TokenKind::Equal, TokenKind::Comma, TokenKind::BracesClose],
			);
			let expression = if context.kind() == TokenKind::Equal {
				Some(parse_member_value(context, stop_at))
			} else {
				None
			};
			let end = context.span(expression.unwrap_or(type_expression)).end;
			context.insert(
				Node::Assignment {
					name: name_index,
					type_expression: Some(type_expression),
					expression,
				},
				token.start,
				end,
			)
		}
		TokenKind::Equal => {
			let expression = parse_member_value(context, stop_at);
			let end = context.span(expression).end;
			context.insert(
				Node::Assignment {
					name: name_index,
					type_expression: None,
					expression: Some(expression),
				},
				token.start,
				end,
			)
		}
		TokenKind::Comma | TokenKind::Stop | TokenKind::BracesClose | TokenKind::End => name_index,
		// a nested key set, like `nested { bar }` in `{ foo, nested { bar } }`
		TokenKind::BracesOpen => {
			let (members, end) = parse_member_items(context);
			context.insert(
				Node::MemberIndex {
					target: name_index,
					members,
				},
				token.start,
				end,
			)
		}
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration => {
			context.error_at_token(format!(
				"Methods are declared with the `function` keyword: `function {name_text} = (self) => ...`"
			));
			context.skip_until(&[TokenKind::Comma]);
			name_index
		}
		_ => {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected `=`, `:`, `,` or a new line after the member `{name_text}`, found {found}"
			));
			name_index
		}
	}
}

/// Parse `= value` in a member.
fn parse_member_value(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	context.go_to_next_token(); // skip `=`
	parse_expression(
		context,
		Priority::Comma,
		ExpressionContext::expression(),
		stop_at,
	)
}
