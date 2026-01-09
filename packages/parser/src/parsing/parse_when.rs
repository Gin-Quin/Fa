use crate::{
	context::Context,
	nodes::Node,
	parsing::{parse_expression::parse_expression, parse_when_branches::parse_when_branches},
	priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_when(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token().kind == TokenKind::BracesOpen {
		panic!("Expected expression after `when`");
	}

	let expression = parse_expression(context, Priority::None, false, [TokenKind::Is]);

	if context.token().kind != TokenKind::Is {
		panic!("Expected `is` after when expression");
	}

	context.go_to_next_token();

	if context.token().kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after `is` in when expression");
	}

	let branches = parse_when_branches(context);

	Node::When {
		expression,
		branches,
	}
}
