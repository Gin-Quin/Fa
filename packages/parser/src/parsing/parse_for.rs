use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_function_call_parameters::parse_function_call_parameters;
use crate::parsing::parse_pattern::parse_pattern;
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse a `for` loop: `for [parallel[(options)]] [<pattern>[, <index>] in] <iterable> { <body> }`
pub(crate) fn parse_for(context: &mut Context<'_>, is_compile_time: bool) -> usize {
	let start = context.token().start;
	let keyword = if is_compile_time { "@for" } else { "for" };
	context.go_to_next_token();

	let parallel = parse_parallel_modifier(context);

	let mut pattern = None;
	let mut index = None;

	if has_binding(context) {
		pattern = Some(parse_pattern(context));

		if context.kind() == TokenKind::Comma {
			context.go_to_next_token();
			if context.kind() == TokenKind::Identifier {
				index = Some(context.name());
				context.go_to_next_token();
			} else {
				context.error_at_token(format!(
					"Expected the name of the index after `,`, like `{keyword} item, index in items`"
				));
			}
		}

		if context.kind() == TokenKind::In {
			context.go_to_next_token();
		} else {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected `in` after the values of the loop, like `{keyword} item in items`, found {found}"
			));
			context.skip_until(&[TokenKind::In, TokenKind::BracesOpen]);
			if context.kind() == TokenKind::In {
				context.go_to_next_token();
			}
		}
	}

	let iterable = if context.kind() == TokenKind::BracesOpen {
		context.error_at_token(format!(
			"Expected what to iterate after `{keyword}`, like `{keyword} item in items {{ ... }}`"
		));
		context.insert_missing()
	} else {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[TokenKind::BracesOpen],
		)
	};

	let body = parse_block(context, keyword);
	let end = context.last_end();

	context.insert(
		Node::For {
			pattern,
			index,
			iterable,
			parallel,
			body,
			is_compile_time,
		},
		start,
		end,
	)
}

/// Parse the optional `parallel` or `parallel(options)` modifier of a for loop.
fn parse_parallel_modifier(context: &mut Context<'_>) -> Option<Box<[usize]>> {
	if context.kind() != TokenKind::Parallel {
		return None;
	}
	context.go_to_next_token();

	if matches!(
		context.kind(),
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration
	) {
		Some(parse_function_call_parameters(context).into_boxed_slice())
	} else {
		Some(Box::default())
	}
}

/// Return true if the loop declares its values, i.e. if `in` appears before the body.
fn has_binding(context: &Context<'_>) -> bool {
	let mut depth: usize = 0;
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
				if depth == 0 {
					return false;
				}
				depth -= 1;
			}
			TokenKind::End | TokenKind::Stop if depth == 0 => return false,
			TokenKind::End => return false,
			_ => {}
		}
		previous_kind = token.kind;
		index += 1;
	}

	false
}
