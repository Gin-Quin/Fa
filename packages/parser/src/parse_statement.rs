use crate::{context::Context, priority::Priority};

use crate::parse_expression::parse_expression;

pub fn parse_statement(context: &mut Context) -> usize {
	let node: usize = parse_expression(context, Priority::None, []);

	context.go_to_next_token();
	node
}
