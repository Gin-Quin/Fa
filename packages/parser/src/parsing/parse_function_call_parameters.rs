use crate::{
	context::Context,
	nodes::Node,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub(crate) fn parse_function_call_parameters(context: &mut Context) -> Vec<usize> {
	let tree: &TypedSyntaxTree = unsafe { &*context.tree };
	context.go_to_next_token();
	let mut parameters: Vec<usize> = Vec::new();
	if context.token().kind != TokenKind::ParenthesisClose {
		loop {
			if context.done() {
				panic!("Missing closing `)`");
			}

			while context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token().kind == TokenKind::ParenthesisClose {
					break;
				}
			}

			if context.token().kind == TokenKind::ParenthesisClose {
				break;
			}

			let parameter = parse_expression(
				context,
				Priority::None,
				ExpressionContext::new(false, false),
				[TokenKind::ParenthesisClose, TokenKind::Comma],
			);
			if let Node::DanglingToken { token } = tree.at(parameter) {
				panic!(
					"Unexpected token {:?} in function call arguments",
					token.kind
				);
			}
			parameters.push(parameter);

			if context.token().kind == TokenKind::ParenthesisClose {
				break;
			}

			if context.token().kind == TokenKind::Comma || context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}
	if context.token().kind != TokenKind::ParenthesisClose {
		panic!("Missing closing `)`");
	}
	context.go_to_next_token();
	parameters
}
