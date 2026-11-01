use crate::context::Context;
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse the arguments of a call, from `(` to `)`.
pub(crate) fn parse_function_call_parameters(context: &mut Context<'_>) -> Vec<usize> {
	parse_delimited(context, TokenKind::ParenthesisClose, |context| {
		parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			&[TokenKind::ParenthesisClose, TokenKind::Comma],
		)
	})
	.0
}
