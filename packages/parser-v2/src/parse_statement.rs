use crate::{
	context::Context,
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
	nodes::Node,
};

use crate::parse_expression::parse_expression;

pub fn parse_statement(context: &mut Context) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = context.token.clone();

	let node: usize = match token.kind {
		/* ------------------------------- Expression ------------------------------- */
		_ => parse_expression(context, Priority::None),
	};

	context.go_to_next_token();
	node
}
