use crate::{
	context::Context,
	nodes::Node,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_list(context: &mut Context, is_static: bool) -> Node {
	context.go_to_next_token();
	let mut items: Vec<usize> = Vec::new();

	if context.token().kind != TokenKind::BracketsClose {
		loop {
			if context.done() {
				panic!("Missing closing `]` after list");
			}

			while context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token().kind == TokenKind::BracketsClose {
					break;
				}
			}

			if context.token().kind == TokenKind::BracketsClose {
				break;
			}

			let expression = parse_expression(
				context,
				Priority::None,
				ExpressionContext::new(false, false),
				[TokenKind::BracketsClose, TokenKind::Comma],
			);
			items.push(expression);

			if context.token().kind == TokenKind::BracketsClose {
				break;
			}

			if context.token().kind == TokenKind::Comma || context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}

	if context.token().kind != TokenKind::BracketsClose {
		panic!("Missing closing `]` after list");
	}

	context.go_to_next_token();

	if is_static {
		Node::StaticList { items }
	} else {
		Node::List { items }
	}
}
