use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_pattern::parse_pattern;
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse a `while` loop: `while <condition> { <body> }` or `while let <name> = <optional> { <body> }`
pub(crate) fn parse_while(context: &mut Context<'_>) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	let pattern = if context.kind() == TokenKind::Let {
		context.go_to_next_token();
		let pattern = parse_pattern(context);
		if !matches!(context.at(pattern), Node::Identifier { .. } | Node::Invalid) {
			let span = context.span(pattern);
			context.error_with_code(
				"E-LOOP-9",
				"`while let` binds a name, not a pattern: write `while let item = stack.pop() { ... }` and destructure inside the loop",
				span.start,
				span.end,
			);
		}
		if context.kind() == TokenKind::Equal {
			context.go_to_next_token();
		} else {
			context.error_at_token(
				"Expected `=` after the name, like `while let item = stack.pop() { ... }`",
			);
		}
		Some(pattern)
	} else {
		None
	};

	let expression = if context.kind() == TokenKind::BracesOpen {
		context
			.error_at_token("Expected a condition after `while`, like `while count > 0 { ... }`");
		context.insert_missing()
	} else {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[TokenKind::BracesOpen],
		)
	};

	let body = parse_block(context, "while");
	let end = context.last_end();

	let node = match pattern {
		Some(pattern) => Node::WhileLet {
			pattern,
			expression,
			body,
		},
		None => Node::While { expression, body },
	};
	context.insert(node, start, end)
}
