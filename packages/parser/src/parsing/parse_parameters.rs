use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_pattern::{is_pattern_start, parse_pattern};
use crate::priority::Priority;
use crate::tokens::TokenKind;

const PARAMETER_STOP_TOKENS: [TokenKind; 2] = [TokenKind::Comma, TokenKind::ParenthesisClose];

/// Parse the parameters of a function, starting at the opening parenthesis:
/// `(self, name: Type, other: Type = default, ...rest: Integer, { x, y }: Point, { anonymous: Type })`
pub(crate) fn parse_parameters(context: &mut Context<'_>) -> usize {
	let start = context.token().start;
	let (items, end) = parse_delimited(context, TokenKind::ParenthesisClose, parse_parameter);
	context.insert(Node::Parameters { items }, start, end)
}

fn parse_parameter(context: &mut Context<'_>) -> usize {
	let start = context.token().start;

	if context.kind() == TokenKind::Mutable {
		context.error_at_token(
			"Parameters can't be `mutable`: declare the mutation in the signature instead, like `(self) mutates self => ...`",
		);
		context.go_to_next_token();
	}

	if context.kind() == TokenKind::Copy {
		context.error_expected_name("Expected a parameter name");
		context.skip_until(&PARAMETER_STOP_TOKENS);
		return context.insert(Node::Invalid, start, context.last_end());
	}

	// a destructured parameter, like `{ x, y }: Point`
	if is_pattern_start(context.kind()) && token_after_matching_close(context) == TokenKind::Colon {
		let pattern = parse_pattern(context);
		context.go_to_next_token(); // skip `:`
		let type_expression = parse_expression(
			context,
			Priority::Assignment,
			ExpressionContext::expression(),
			&PARAMETER_STOP_TOKENS,
		);
		let end = context.span(type_expression).end;
		return context.insert(
			Node::DestructuredParameter {
				pattern,
				type_expression,
			},
			start,
			end,
		);
	}

	// a named parameter, `self`, a variadic parameter, or an anonymous structure parameter
	parse_expression(
		context,
		Priority::Comma,
		ExpressionContext::expression(),
		&PARAMETER_STOP_TOKENS,
	)
}

/// Return the kind of the token that follows the group opened at the current token.
fn token_after_matching_close(context: &Context<'_>) -> TokenKind {
	let mut depth: usize = 0;
	let mut index = context.token_index;

	while let Some(token) = context.tokens.get(index) {
		match token.kind {
			TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
			| TokenKind::BracesOpen
			| TokenKind::BracketsOpen
			| TokenKind::GenericOpen => depth += 1,
			TokenKind::ParenthesisClose
			| TokenKind::BracesClose
			| TokenKind::BracketsClose
			| TokenKind::GenericClose => {
				depth = depth.saturating_sub(1);
				if depth == 0 {
					return context
						.tokens
						.get(index + 1)
						.map_or(TokenKind::End, |token| token.kind);
				}
			}
			TokenKind::End => return TokenKind::End,
			_ => {}
		}
		index += 1;
	}

	TokenKind::End
}
