use crate::{
	context::Context,
	nodes::Node,
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
		parse_function_call_parameters::parse_function_call_parameters,
		parse_pattern::parse_pattern,
	},
	priority::Priority,
	tokens::TokenKind,
};

/// Parse a `for` loop: `for [parallel[(options)]] [<pattern>[, <index>] in] <iterable> { <body> }`
pub(crate) fn parse_for(context: &mut Context, is_compile_time: bool) -> Node {
	context.go_to_next_token();

	if context.token().kind == TokenKind::BracesOpen && !has_binding(context) {
		panic!("Expected expression after `for`");
	}

	let parallel = parse_parallel_modifier(context);

	let mut pattern = None;
	let mut index = None;

	if has_binding(context) {
		pattern = Some(parse_pattern(context));

		if context.token().kind == TokenKind::Comma {
			context.go_to_next_token();
			if context.token().kind != TokenKind::Identifier {
				panic!("Expected the name of the index after `,` in for loop");
			}
			index = Some(context.slice());
			context.go_to_next_token();
		}

		if context.token().kind != TokenKind::In {
			panic!("Expected `in` after the values of the for loop");
		}
		context.go_to_next_token();
	}

	let iterable = parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(false, false),
		[TokenKind::BracesOpen],
	);

	if context.token().kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after for expression");
	}

	let (body, hoisted_symbols) = parse_block_body_with_hoisted(context, "for");

	Node::For {
		pattern,
		index,
		iterable,
		parallel,
		body,
		is_compile_time,
		hoisted_symbols,
	}
}

/// Parse the optional `parallel` or `parallel(options)` modifier of a for loop.
fn parse_parallel_modifier(context: &mut Context) -> Option<Vec<usize>> {
	if context.token().kind != TokenKind::Identifier || context.slice() != "parallel" {
		return None;
	}

	// `parallel` used as a regular name, like in `for parallel in values`
	if matches!(
		context.lookup_next_token_kind(),
		TokenKind::In | TokenKind::Comma | TokenKind::BracesOpen
	) {
		return None;
	}

	context.go_to_next_token();

	if matches!(
		context.token().kind,
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration
	) {
		Some(parse_function_call_parameters(context))
	} else {
		Some(Vec::new())
	}
}

/// Return true if the loop declares its values, i.e. if `in` appears before the body.
fn has_binding(context: &Context) -> bool {
	let mut depth = 0;
	let mut index = context.token_index;
	let mut previous_kind = TokenKind::Empty;

	while let Some(token) = context.tokens.get(index) {
		match token.kind {
			TokenKind::In if depth == 0 => return true,
			// a `{` at the start or after a comma begins a pattern, otherwise it begins the body
			TokenKind::BracesOpen
				if depth == 0
					&& index != context.token_index
					&& previous_kind != TokenKind::Comma =>
			{
				return false;
			}
			TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
			| TokenKind::BracesOpen
			| TokenKind::BracketsOpen => depth += 1,
			TokenKind::ParenthesisClose | TokenKind::BracesClose | TokenKind::BracketsClose => {
				depth -= 1
			}
			TokenKind::End => return false,
			_ => {}
		}
		previous_kind = token.kind;
		index += 1;
	}

	false
}
