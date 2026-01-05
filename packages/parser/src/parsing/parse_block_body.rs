use crate::{
	context::Context, parsing::parse_expression::parse_expression, priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_block_body(context: &mut Context, label: &str) -> Vec<usize> {
	context.go_to_next_token();
	let mut body: Vec<usize> = Vec::new();

	if context.token.kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after {label} body");
			}

			body.push(parse_expression(
				context,
				Priority::None,
				[TokenKind::BracesClose],
			));

			if context.token.kind == TokenKind::BracesClose {
				break;
			}

			context.go_to_next_token();
		}
	}

	if context.token.kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after {label} body");
	}

	context.go_to_next_token();
	body
}
