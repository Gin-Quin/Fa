use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse `if` / `else`, or the compile-time `@if` / `@else`
pub(crate) fn parse_if(context: &mut Context<'_>, is_compile_time: bool) -> usize {
	let start = context.token().start;
	let (if_keyword, else_keyword, if_kind) = if is_compile_time {
		("@if", "@else", TokenKind::AtIf)
	} else {
		("if", "else", TokenKind::If)
	};
	context.go_to_next_token();

	let condition = if context.kind() == TokenKind::BracesOpen {
		context.error_at_token(format!(
			"Expected a condition after `{if_keyword}`, like `{if_keyword} count > 0 {{ ... }}`"
		));
		context.insert_missing()
	} else {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[TokenKind::BracesOpen],
		)
	};

	let then_body = parse_block(context, if_keyword);
	let mut else_body = None;

	// `else` on the next line: report it, and attach it anyway
	if context.kind() == TokenKind::Stop
		&& matches!(
			context.lookup_next_token_kind(),
			TokenKind::Else | TokenKind::AtElse
		) {
		context.go_to_next_token();
		context.error_at_token(format!(
			"`{}` must be on the line of the `}}` that closes the `{if_keyword}` block: write `}} {} {{`",
			context.slice(),
			context.slice()
		));
	}

	if is_compile_time && context.kind() == TokenKind::Else {
		context.error_at_token("A compile-time `@if` continues with `@else`, not `else`");
	} else if !is_compile_time && context.kind() == TokenKind::AtElse {
		context.error_at_token("An `if` continues with `else`, not `@else`");
	}

	if matches!(context.kind(), TokenKind::Else | TokenKind::AtElse) {
		context.go_to_next_token();
		if context.kind() == if_kind {
			else_body = Some(parse_if(context, is_compile_time));
		} else if context.kind() == TokenKind::BracesOpen {
			else_body = Some(parse_block(context, else_keyword));
		} else {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected `{if_keyword}` or `{{` after `{else_keyword}`, found {found}"
			));
		}
	}

	let end = context.last_end();
	context.insert(
		Node::If {
			is_compile_time,
			condition,
			then_body,
			else_body,
		},
		start,
		end,
	)
}
