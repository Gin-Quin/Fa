use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_block_body::parse_block;

/// `loop { ... }`
pub(crate) fn parse_loop(context: &mut Context<'_>) -> usize {
	let start = context.token().start;
	context.go_to_next_token();
	let body = parse_block(context, "loop");
	let end = context.last_end();
	context.insert(Node::Loop { body }, start, end)
}
