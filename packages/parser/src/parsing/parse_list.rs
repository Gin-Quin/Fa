use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// `[a, b, c]`
pub(crate) fn parse_list(context: &mut Context<'_>) -> usize {
	let start = context.token().start;
	let (items, end) = parse_delimited(context, TokenKind::BracketsClose, |context| {
		parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			&[TokenKind::BracketsClose, TokenKind::Comma],
		)
	});
	context.insert(Node::List { items }, start, end)
}
