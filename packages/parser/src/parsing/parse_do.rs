use crate::{
	context::Context, nodes::Node, parsing::parse_block_body::parse_block_body, tokens::TokenKind,
};

pub(crate) fn parse_do(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after `do`");
	}

	let body = parse_block_body(context, "do");

	Node::Do { body }
}
