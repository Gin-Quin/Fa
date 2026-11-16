use crate::context::{Context, MAX_DEPTH};
use crate::nodes::{Node, PatternField};
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Return true if the current token starts a destructuring pattern.
pub(crate) fn is_pattern_start(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::BracesOpen
			| TokenKind::BracketsOpen
			| TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
	)
}

/// An item of an object or array pattern, before it is sorted into fields and rest.
enum PatternItem {
	Field(PatternField),
	Item(usize),
	// `...name`, with the index of its `Identifier` node
	Rest(usize),
	Invalid,
}

/// Parse a name or a destructuring pattern:
/// `name`, `{ a, b as c, d = 4, nested { e }, ...rest }`, `[first, second, ...others]` or
/// `(number, name)`.
pub(crate) fn parse_pattern(context: &mut Context<'_>) -> usize {
	if context.depth >= MAX_DEPTH {
		return context.skip_too_deep_code();
	}
	context.depth += 1;
	let pattern = parse_pattern_inner(context);
	context.depth -= 1;
	pattern
}

fn parse_pattern_inner(context: &mut Context<'_>) -> usize {
	let start = context.token().start;

	match context.kind() {
		TokenKind::Identifier => {
			let name = context.name();
			let end = context.token().end;
			context.go_to_next_token();
			context.insert(Node::Identifier { name }, start, end)
		}
		TokenKind::BracesOpen => {
			let (items, end) = parse_delimited(context, TokenKind::BracesClose, parse_object_item);
			let (fields, rest) = sort_items(context, items, "object", |item| match item {
				PatternItem::Field(field) => Some(field),
				_ => None,
			});
			context.insert(Node::ObjectPattern { fields, rest }, start, end)
		}
		TokenKind::BracketsOpen => {
			let (items, end) = parse_delimited(context, TokenKind::BracketsClose, parse_array_item);
			let (items, rest) = sort_items(context, items, "array", |item| match item {
				PatternItem::Item(index) => Some(index),
				_ => None,
			});
			context.insert(Node::ArrayPattern { items, rest }, start, end)
		}
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration => {
			let (items, end) =
				parse_delimited(context, TokenKind::ParenthesisClose, parse_array_item);
			let (items, rest) = sort_items(context, items, "tuple", |item| match item {
				PatternItem::Item(index) => Some(index),
				_ => None,
			});
			if let Some(rest) = rest {
				let span = context.span(rest);
				context.error_with_code("E-DST-6", 
					"A tuple pattern has exactly one name per element: `...rest` is only allowed in object and array patterns",
					span.start,
					span.end,
				);
			}
			context.insert(Node::TuplePattern { items }, start, end)
		}
		_ => {
			let found = context.describe_token();
			let message = format!(
				"Expected a name or a destructuring pattern, like `{{ a, b }}` or `[first, second]`, found {found}"
			);
			context.error_expected_name(message);
			context.insert_missing()
		}
	}
}

/// Separate the items of a pattern from its rest, which must be last and unique.
fn sort_items<'source, T>(
	context: &mut Context<'source>,
	items: Vec<PatternItem>,
	what: &str,
	mut extract: impl FnMut(PatternItem) -> Option<T>,
) -> (Vec<T>, Option<usize>) {
	let mut values = Vec::with_capacity(items.len());
	let mut rest = None;
	for item in items {
		match item {
			PatternItem::Rest(index) => {
				if rest.is_some() {
					let span = context.span(index);
					context.error(
						format!("An {what} pattern has at most one `...rest`"),
						span.start,
						span.end,
					);
				}
				rest = Some(index);
			}
			PatternItem::Invalid => {}
			item => {
				if let Some(rest) = rest {
					let span = context.span(rest);
					context.error_with_code(
						"E-DST-9",
						"`...rest` must be the last item of a pattern",
						span.start,
						span.end,
					);
				}
				if let Some(value) = extract(item) {
					values.push(value);
				}
			}
		}
	}
	(values, rest)
}

/// Parse `...name` in a pattern, starting at `...`.
fn parse_rest<'source>(context: &mut Context<'source>) -> PatternItem {
	context.go_to_next_token();
	if context.kind() == TokenKind::Identifier {
		let token = context.token();
		let name = context.name();
		context.go_to_next_token();
		PatternItem::Rest(context.insert(Node::Identifier { name }, token.start, token.end))
	} else {
		context.error_expected_name("Expected a name after `...`, like `...rest`");
		PatternItem::Invalid
	}
}

fn parse_object_item<'source>(context: &mut Context<'source>) -> PatternItem {
	if context.kind() == TokenKind::TripleDot {
		return parse_rest(context);
	}

	if context.kind() != TokenKind::Identifier {
		let found = context.describe_token();
		context.error_expected_name(format!(
			"Expected a field name in the pattern, found {found}"
		));
		return PatternItem::Invalid;
	}

	let name_text = context.slice();
	let name = context.name();
	context.go_to_next_token();

	let mut field = PatternField {
		name,
		alias: None,
		nested: None,
		default: None,
	};

	if is_pattern_start(context.kind()) {
		field.nested = Some(parse_pattern(context));
	} else if context.kind() == TokenKind::As {
		context.go_to_next_token();
		if context.kind() == TokenKind::Identifier {
			field.alias = Some(context.name());
			context.go_to_next_token();
		} else {
			context.error_expected_name(format!(
				"Expected a name after `as`, like `{name_text} as other`"
			));
		}
	} else if context.kind() == TokenKind::Colon {
		context.error_at_token_with_code(
			"E-DST-8",
			format!(
				"`:` is always followed by a type: to rename a field, write `{name_text} as other`"
			),
		);
		context.skip_until(&[TokenKind::Comma]);
	}

	if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		field.default = Some(parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			&[TokenKind::Comma, TokenKind::BracesClose],
		));
	}

	PatternItem::Field(field)
}

fn parse_array_item<'source>(context: &mut Context<'source>) -> PatternItem {
	if context.kind() == TokenKind::TripleDot {
		return parse_rest(context);
	}
	let before = context.token_index;
	let item = parse_pattern(context);
	if context.token_index == before {
		return PatternItem::Invalid;
	}
	// `[a = 0]`: items are read by position, a default would hide a missing item
	if context.kind() == TokenKind::Equal {
		context.error_at_token_with_code(
			"E-DST-6",
			"The items of an array or tuple pattern have no default value: check the length first, or use an object pattern",
		);
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			&[
				TokenKind::Comma,
				TokenKind::BracketsClose,
				TokenKind::ParenthesisClose,
			],
		);
	}
	PatternItem::Item(item)
}
