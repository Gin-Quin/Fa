use crate::nodes::Node;
use crate::tokens::TokenKind;
use crate::{context::Context, parse_expression::parse_expression, priority::Priority};

pub fn parse_function_declaration(context: &mut Context) -> (Node, bool) {
	context.go_to_next_token();

	if context.token.kind != TokenKind::Identifier {
		panic!("Expected identifier after `function` keyword");
	}

	let name = context.slice();
	context.go_to_next_token();

	if context.token.kind != TokenKind::Equal {
		panic!("Expected `=` after function name");
	}

	context.go_to_next_token();

	let value = parse_expression(context, Priority::None, []);

	(Node::Function { name, value }, false)
}
