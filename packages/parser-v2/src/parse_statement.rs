use crate::tokens::TokenKind;
use crate::{context::Context, priority::Priority};

use crate::parse_expression::parse_expression;

pub fn parse_statement(context: &mut Context) -> usize {
	let token = context.token.clone();

	let node: usize = match token.kind {
		/* ------------------------------- Expression ------------------------------- */
		_ => parse_expression(context, Priority::None, TokenKind::None),
	};

	context.go_to_next_token();
	node
}
