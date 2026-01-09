use crate::{
	context::Context, parsing::parse_expression::parse_expression, priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_arrow_block_body(context: &mut Context) -> Vec<usize> {
	context.go_to_next_token();
	let mut body: Vec<usize> = Vec::new();
	if context.token().kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after arrow function body");
			}
			body.push(parse_expression(
				context,
				Priority::None,
				true,
				[TokenKind::BracesClose],
			));
			if context.token().kind == TokenKind::BracesClose {
				break;
			}
			context.go_to_next_token();
		}
	}
	if context.token().kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after arrow function body");
	}
	context.go_to_next_token();
	body
}
