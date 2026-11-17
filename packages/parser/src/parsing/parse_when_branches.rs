use crate::context::Context;
use crate::nodes::{WhenBranch, WhenBranchPattern};
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_pattern::parse_pattern;
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse the branches of a `when`, from `{` to `}`. Branches are separated by new lines or
/// commas: `{ 1 => "one", else => "many" }`.
pub(crate) fn parse_when_branches(context: &mut Context<'_>) -> Vec<WhenBranch> {
	parse_delimited(context, TokenKind::BracesClose, parse_when_branch).0
}

fn parse_when_branch(context: &mut Context<'_>) -> WhenBranch {
	let pattern = if context.kind() == TokenKind::Else {
		context.go_to_next_token();
		WhenBranchPattern::Else
	} else {
		WhenBranchPattern::Expression(parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[
				TokenKind::FatArrow,
				TokenKind::As,
				TokenKind::Comma,
				TokenKind::BracesClose,
			],
		))
	};

	let binding = if context.kind() == TokenKind::As {
		if matches!(pattern, WhenBranchPattern::Else) {
			context.error_at_token_with_code(
				"E-WHEN-6",
				"The `else` branch can't be bound with `as`: bind the subject before the `when`",
			);
		}
		context.go_to_next_token();
		Some(parse_pattern(context))
	} else {
		None
	};

	if context.kind() == TokenKind::If {
		skip_guard(context);
	}

	if context.kind() == TokenKind::FatArrow {
		context.go_to_next_token();
	} else {
		let found = context.describe_token();
		context.error_at_token(format!(
			"Expected `=>` after the pattern of the branch, like `Shape.Circle => ...`, found {found}"
		));
	}

	let value = if context.kind() == TokenKind::BracesOpen {
		parse_block(context, "when branch")
	} else {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[TokenKind::Stop, TokenKind::Comma, TokenKind::BracesClose],
		)
	};

	WhenBranch {
		pattern,
		binding,
		value,
	}
}

/// `E-WHEN-4`: a guard after a pattern (`Integer as n if n > 0 =>`). It is skipped, up to the
/// `=>` of the branch.
fn skip_guard(context: &mut Context<'_>) {
	let start = context.token().start;
	context.skip_until(&[TokenKind::FatArrow, TokenKind::Comma]);
	let end = context.last_end().max(start);
	context.error_with_code(
		"E-WHEN-4",
		"A `when` branch has no guard: remove this `if ...`, and test the condition with an `if` inside the branch",
		start,
		end,
	);
}
