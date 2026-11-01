use crate::context::Context;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse `[index]` after an expression, starting at `[`, and return the index.
pub(crate) fn parse_index_expression(context: &mut Context<'_>) -> usize {
	let open = context.token();
	context.go_to_next_token();
	context.skip_stops();
	let index = parse_expression(
		context,
		Priority::None,
		ExpressionContext::expression(),
		&[TokenKind::BracketsClose],
	);
	if context.done() {
		context.error("This `[` is never closed: add a `]`", open.start, open.end);
	} else if context.kind() != TokenKind::BracketsClose {
		let found = context.describe_token();
		context.error_at_token(format!("Expected `]` to close the `[`, found {found}"));
		// skip the rest of the index, up to its `]` if it is on the same line
		context.skip_to_statement_end();
	}
	if context.kind() == TokenKind::BracketsClose {
		context.go_to_next_token();
	}
	index
}
