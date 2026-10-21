use crate::{
	context::Context,
	nodes::Node,
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
		parse_pattern::parse_pattern,
	},
	priority::Priority,
	tokens::TokenKind,
};

/// Parse a `while` loop: `while <condition> { <body> }` or `while let <pattern> = <optional> { <body> }`
pub(crate) fn parse_while(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token().kind == TokenKind::BracesOpen {
		panic!("Expected expression after `while`");
	}

	let pattern = if context.token().kind == TokenKind::Let {
		context.go_to_next_token();
		let pattern = parse_pattern(context);
		if context.token().kind != TokenKind::Equal {
			panic!("Expected `=` after `while let` pattern");
		}
		context.go_to_next_token();
		Some(pattern)
	} else {
		None
	};

	let expression = parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(false, false),
		[TokenKind::BracesOpen],
	);

	if context.token().kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after while expression");
	}

	let (body, hoisted_symbols) = parse_block_body_with_hoisted(context, "while");

	match pattern {
		Some(pattern) => Node::WhileLet {
			pattern,
			expression,
			body,
			hoisted_symbols,
		},
		None => Node::While {
			expression,
			body,
			hoisted_symbols,
		},
	}
}
