use crate::{
	context::Context,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
};

pub fn parse_statement(context: &mut Context) -> usize {
	let node: usize = parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(true, false),
		[],
	);

	context.go_to_next_token();
	node
}
