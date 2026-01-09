use crate::{
	context::Context, parsing::parse_expression::parse_expression, priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_function_call_parameters(context: &mut Context) -> Vec<usize> {
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
				false,
				[TokenKind::ParenthesisClose, TokenKind::Comma],
			);
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
