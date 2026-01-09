use crate::{
	context::Context, parsing::parse_expression::parse_expression, priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_template_expression(context: &mut Context, input: &'static str) -> usize {
	let mut sub_context = Context::new(input, unsafe { &mut *context.tree });
	if sub_context.token.kind == TokenKind::End {
		panic!("Empty expression in template string");
	}
	let expression = parse_expression(&mut sub_context, Priority::None, false, []);

	if !sub_context.done() {
		sub_context.go_to_next_token();
	}

	while !sub_context.done() {
		if sub_context.token.kind == TokenKind::Stop {
			sub_context.go_to_next_token();
		} else {
			panic!("Unexpected token in template string expression");
		}
	}

	expression
}
