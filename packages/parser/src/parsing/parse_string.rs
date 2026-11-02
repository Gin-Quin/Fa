use crate::context::Context;
use crate::nodes::{Node, StringPart};
use crate::parsing::parse_escape_sequence::parse_escape_sequence;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokenize::find_interpolation_end;

/// Parse the string token at the current position: a `StringLiteral`, or a `StringTemplate`
/// when it interpolates expressions, like `"Hello {name}"`.
pub(crate) fn parse_string(context: &mut Context<'_>) -> usize {
	let token = context.token();
	context.go_to_next_token();

	let source = context.source;
	let bytes = source.as_bytes();
	let end = token.end;
	let mut parts: Vec<StringPart> = Vec::new();
	let mut literal = String::new();
	let mut index = token.start + 1;
	let mut segment_start = index;

	while index < end {
		match bytes[index] {
			// the closing quote (the tokenizer already reported a missing one)
			b'"' => break,
			b'\\' => {
				push_segment(&mut literal, &source[segment_start..index]);
				index = parse_escape_sequence(context, index, end, &mut literal);
				segment_start = index;
			}
			b'{' if index + 1 < end && bytes[index + 1] == b'{' => {
				push_segment(&mut literal, &source[segment_start..index]);
				literal.push('{');
				index += 2;
				segment_start = index;
			}
			b'}' if index + 1 < end && bytes[index + 1] == b'}' => {
				push_segment(&mut literal, &source[segment_start..index]);
				literal.push('}');
				index += 2;
				segment_start = index;
			}
			b'}' => {
				context.error_with_code(
					"E-STR-4",
					"A `}` in a string must be doubled: write `}}` for a literal brace",
					index,
					index + 1,
				);
				index += 1;
			}
			b'{' => {
				push_segment(&mut literal, &source[segment_start..index]);
				if !literal.is_empty() {
					parts.push(StringPart::Literal(std::mem::take(&mut literal)));
				}
				let expression_start = index + 1;
				let Some(after_close) = find_interpolation_end(bytes, expression_start, end) else {
					// the tokenizer reported the string as never closed
					segment_start = end;
					index = end;
					break;
				};
				let expression_end = after_close - 1;
				if source[expression_start..expression_end].trim().is_empty() {
					context.error_with_code("E-STR-5", 
						"Empty `{}` in a string: write an expression between the braces, or `{{` and `}}` for literal braces",
						index,
						after_close,
					);
				} else {
					let expression = parse_interpolation(context, expression_start, expression_end);
					parts.push(StringPart::Expression(expression));
				}
				index = after_close;
				segment_start = index;
			}
			_ => index += 1,
		}
	}
	push_segment(&mut literal, &source[segment_start.min(index)..index]);

	let node = if parts.is_empty() {
		Node::StringLiteral(literal)
	} else {
		if !literal.is_empty() {
			parts.push(StringPart::Literal(literal));
		}
		Node::StringTemplate { parts }
	};
	context.insert(node, token.start, token.end)
}

/// Add a part of a string written as is, with its line breaks normalized to `\n`.
fn push_segment(literal: &mut String, segment: &str) {
	if segment.contains('\r') {
		literal.push_str(&segment.replace("\r\n", "\n"));
	} else {
		literal.push_str(segment);
	}
}

/// Parse the expression interpolated in `source[start..end]`.
fn parse_interpolation(context: &mut Context<'_>, start: usize, end: usize) -> usize {
	context.with_sub_tokens(start, end, |context| {
		context.skip_stops();
		let expression = parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			&[],
		);
		context.skip_stops();
		if !context.done() {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Unexpected {found} in the interpolated expression: a string interpolates one expression"
			));
		}
		expression
	})
}
