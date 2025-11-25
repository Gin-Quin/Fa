use crate::nodes::Node;
use crate::tokens::TokenKind;
use crate::{context::Context, parse_expression::parse_expression, priority::Priority};

pub fn parse_function_declaration(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind != TokenKind::Identifier {
		panic!("Expected identifier after `function` keyword");
	}

	let name = context.slice();
	context.go_to_next_token();

	if context.token.kind != TokenKind::ParenthesisOpen {
		panic!("Expected `(` after function name");
	}

	context.go_to_next_token();

	let parameters: Option<usize> = if context.token.kind == TokenKind::ParenthesisClose {
		None
	} else {
		Some(parse_expression(
			context,
			Priority::None,
			TokenKind::ParenthesisClose,
		))
	};

	context.go_to_next_token();

	let mut body: Vec<usize> = vec![];
	let return_type_expression = match context.token.kind {
		TokenKind::Colon => {
			context.go_to_next_token();

			let return_type_expression =
				parse_expression(context, Priority::None, TokenKind::BracesOpen);

			if context.token.kind != TokenKind::BracesOpen {
				panic!(
					"Expected `{{` after return type expression of function `{}`",
					name
				);
			}
			context.go_to_next_token();
			Some(return_type_expression)
		}
		TokenKind::BracesOpen => {
			context.go_to_next_token();
			None
		}
		_ => {
			panic!("Expected `:` or `{{` after function parameters");
		}
	};

	if context.done() {
		panic!("Missing closing `}}` after function body");
	}

	if context.token.kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after function body");
			}
			body.push(parse_expression(
				context,
				Priority::None,
				TokenKind::BracesClose,
			));
			if context.token.kind == TokenKind::BracesClose {
				break;
			}
			context.go_to_next_token();
		}
	}

	Node::Function {
		name,
		parameters,
		return_type_expression,
		body,
	}
}
