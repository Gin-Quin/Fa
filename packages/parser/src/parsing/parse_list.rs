use crate::{
	context::Context,
	nodes::Node,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub(crate) fn parse_list(context: &mut Context) -> Node {
	let tree: &TypedSyntaxTree = unsafe { &*context.tree };
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
			if let Node::DanglingToken { token } = tree.at(expression) {
				panic!("Unexpected token {:?} in list", token.kind);
			}
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

	Node::List { items }
}
