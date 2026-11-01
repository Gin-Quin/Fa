use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_when_branches::parse_when_branches;
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// `when <expression> is { <branches> }`
pub(crate) fn parse_when(context: &mut Context<'_>) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	let expression = if matches!(context.kind(), TokenKind::BracesOpen | TokenKind::Is) {
		context.error_at_token("Expected a value after `when`, like `when shape is { ... }`");
		context.insert_missing()
	} else {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[TokenKind::Is, TokenKind::BracesOpen],
		)
	};

	if context.kind() == TokenKind::Is {
		context.go_to_next_token();
	} else {
		let found = context.describe_token();
		context.error_at_token(format!(
			"Expected `is` after the value of `when`, like `when shape is {{ ... }}`, found {found}"
		));
	}

	let branches = if context.kind() == TokenKind::BracesOpen {
		parse_when_branches(context)
	} else {
		let found = context.describe_token();
		context.error_at_token(format!("Expected `{{` after `when ... is`, found {found}"));
		Vec::new()
	};
	let end = context.last_end();

	context.insert(
		Node::When {
			expression,
			branches,
		},
		start,
		end,
	)
}
