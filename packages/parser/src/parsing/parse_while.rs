use crate::{
	context::Context,
	nodes::Node,
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
	},
	priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_while(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token().kind == TokenKind::BracesOpen {
		panic!("Expected expression after `while`");
	}

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

	Node::While {
		expression,
		body,
		hoisted_symbols,
	}
}
