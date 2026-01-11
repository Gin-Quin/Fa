use crate::{
	context::Context,
	nodes::HoistedSymbol,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_block_body_with_hoisted(
	context: &mut Context,
	label: &str,
) -> (Vec<usize>, Vec<HoistedSymbol>) {
	context.go_to_next_token();
	context.enter_hoisted_scope();
	let mut body: Vec<usize> = Vec::new();

	if context.token().kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after {label} body");
			}

			body.push(parse_expression(
				context,
				Priority::None,
				ExpressionContext::new(true, false),
				[TokenKind::BracesClose],
			));

			if context.token().kind == TokenKind::BracesClose {
				break;
			}

			context.go_to_next_token();
		}
	}

	if context.token().kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after {label} body");
	}

	context.go_to_next_token();
	let hoisted_symbols = context.exit_hoisted_scope();
	(body, hoisted_symbols)
}
