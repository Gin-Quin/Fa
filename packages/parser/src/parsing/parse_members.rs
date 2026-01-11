use crate::{
	context::Context,
	nodes::Node,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_members(
	context: &mut Context,
	is_static: bool,
	_is_statement_start: bool,
) -> Node {
	context.go_to_next_token();
	let mut items: Vec<usize> = Vec::new();

	if context.token().kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after members");
			}

			while context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token().kind == TokenKind::BracesClose {
					break;
				}
			}

			if context.token().kind == TokenKind::BracesClose {
				break;
			}

			let expression = parse_expression(
				context,
				Priority::None,
				ExpressionContext::new(true, true),
				[TokenKind::BracesClose, TokenKind::Comma],
			);
			items.push(expression);

			if context.token().kind == TokenKind::BracesClose {
				break;
			}

			if context.token().kind == TokenKind::Comma || context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}

	if context.token().kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after members");
	}

	context.go_to_next_token();

	if is_static {
		Node::StaticMembers { items }
	} else {
		Node::Members { items }
	}
}
