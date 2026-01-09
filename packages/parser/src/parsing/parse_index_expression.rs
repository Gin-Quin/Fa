use crate::{
	context::Context, parsing::parse_expression::parse_expression, priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_index_expression(context: &mut Context) -> usize {
	context.go_to_next_token();
	let index_expression =
		parse_expression(context, Priority::None, false, [TokenKind::BracketsClose]);
	if context.token.kind != TokenKind::BracketsClose {
		panic!("Missing closing `]`");
	}
	context.go_to_next_token();
	index_expression
}
