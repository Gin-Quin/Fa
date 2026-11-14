use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse explicit generic parameters, from `<` to `>`, after `left`: `Array<Integer>`,
/// `Integer<min = 0, max = 200>`, `first<Float>`. A `<` that isn't closed by `>` is reported as
/// a comparison written without a space, like `count<limit`.
pub(crate) fn parse_generic_application(context: &mut Context<'_>, left: usize) -> usize {
	let open = context.token();
	context.go_to_next_token();
	let mut arguments = Vec::new();

	let end = loop {
		while context.kind() == TokenKind::Comma {
			context.go_to_next_token();
		}
		if context.kind() == TokenKind::GenericClose {
			let end = context.token().end;
			context.go_to_next_token();
			if arguments.is_empty() {
				context.error(
					"Generic parameters can't be empty: remove the `<>`, or write the parameters, like `Array<Integer>`",
					open.start,
					end,
				);
			}
			break end;
		}
		let starts_argument = !matches!(context.kind(), TokenKind::Stop | TokenKind::End)
			&& !context.kind().is_closing();
		if starts_argument {
			let before = context.token_index;
			arguments.push(parse_expression(
				context,
				Priority::Comma,
				ExpressionContext::expression(),
				&[TokenKind::Comma, TokenKind::GenericClose],
			));
			if matches!(context.kind(), TokenKind::Comma | TokenKind::GenericClose)
				&& context.token_index != before
			{
				continue;
			}
		}
		report_unclosed_list(context, left, open.start, open.end, &arguments);
		break context.last_end();
	};

	let start = context.span(left).start;
	context.insert(
		Node::GenericApplication {
			target: left,
			arguments,
		},
		start,
		end,
	)
}

fn report_unclosed_list(
	context: &mut Context<'_>,
	left: usize,
	open_start: usize,
	open_end: usize,
	arguments: &[usize],
) {
	let left_span = context.span(left);
	let left_text = context.text(left_span.start, left_span.end);
	let message = match arguments.first() {
		Some(&first) if left_span.end - left_span.start <= 40 => {
			let first_span = context.span(first);
			let first_text = context.text(first_span.start, first_span.end);
			// the operand only, not the block that follows it, like in `if count<limit { … }`
			let first_text = first_text
				.split(|character: char| character.is_whitespace() || character == '{')
				.next()
				.unwrap_or_default();
			format!(
				"`<{first_text}` opens a generic list, which is never closed by `>`: to compare, write `{left_text} < {first_text}`, with a space after `<`"
			)
		}
		_ => "This `<` opens a generic list, which is never closed by `>`: to compare, write a space after `<`, like `a < b`".to_string(),
	};
	context.error(message, open_start, open_end);
}
