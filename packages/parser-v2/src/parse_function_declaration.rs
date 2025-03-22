use crate::{ context::Context, priority::Priority, parse_expression::parse_expression };
use crate::tokens::TokenKind;
use crate::nodes::Node;

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
		Some(parse_expression(context, Priority::None, TokenKind::ParenthesisClose))
	};

	context.go_to_next_token();

	match context.token.kind {
		TokenKind::FatArrow => {
			context.go_to_next_token();
			let expression = parse_expression(
				context,
				Priority::None,
				TokenKind::BracesClose
			);
			Node::ShortFunction {
				name,
				parameters,
				expression,
			}
		}
		TokenKind::Colon => {
			context.go_to_next_token();

			let return_type_expression = parse_expression(
				context,
				Priority::None,
				TokenKind::BracesOpen
			);

			if context.token.kind != TokenKind::BracesOpen {
				panic!("Expected `{{` after return type expression of function `{}`", name);
			}

			let mut body: Vec<usize> = vec![];

			context.go_to_next_token();

			if context.done() {
				panic!("Missing closing `}}` after function body");
			}

			if context.token.kind == TokenKind::BracesClose {
				context.go_to_next_token();
			} else {
				loop {
					if context.done() {
						panic!("Missing closing `}}` after function body");
					}
					body.push(
						parse_expression(context, Priority::None, TokenKind::BracesClose)
					);
					if context.token.kind == TokenKind::BracesClose {
						break;
					}
					context.go_to_next_token();
				}
			}

			println!("Body parsed! Next token: {:#?}", context.token);

			context.go_to_next_token();

			Node::Function {
				name,
				parameters,
				return_type_expression,
				body,
			}
		}
		_ => {
			panic!("Expected `=>` or `{{` after function parameters");
		}
	}
}
