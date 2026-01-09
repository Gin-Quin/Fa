use crate::{context::Context, parsing::parse_expression::parse_expression, priority::Priority};

pub fn parse_statement(context: &mut Context) -> usize {
	let node: usize = parse_expression(context, Priority::None, true, []);

	context.go_to_next_token();
	node
}
