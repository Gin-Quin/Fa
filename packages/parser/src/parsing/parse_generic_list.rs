//! Generic lists, written between `<` and `>`: the arguments of a generic application, like
//! `Map<String, Array<Integer>>`, and the generic parameters of a declaration, like
//! `type Pair<Left, Right> = …`. The tokenizer tells them apart from the comparisons (see
//! `Lexer::brackets`): a `<` not followed by a space is `GenericOpen`, and the `>` that closes
//! it is `GenericClose`.

use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse a generic list from its `<` to its `>`, and return the items with the end of the `>`.
/// `closing` is `GenericClose`, or `GreaterThan` for a list whose `<` is followed by a space,
/// like `Array< Integer>` (reported by the caller, and parsed as a list to recover). Items are
/// separated by commas or new lines. A list that is never closed is reported where it opens:
/// it is often a comparison written without a space after `<`.
pub(crate) fn parse_generic_list<'source>(
	context: &mut Context<'source>,
	closing: TokenKind,
	mut parse_item: impl FnMut(&mut Context<'source>, &[TokenKind]) -> usize,
) -> (Vec<usize>, usize) {
	let open = context.token();
	context.go_to_next_token();
	let stop_at = [TokenKind::Comma, closing];
	let mut items = Vec::new();

	loop {
		while matches!(context.kind(), TokenKind::Comma | TokenKind::Stop) {
			context.go_to_next_token();
		}

		let kind = context.kind();
		if kind == closing {
			let end = context.token().end;
			context.go_to_next_token();
			if items.is_empty() {
				context.error(
					"A generic list can't be empty: remove the `<>`",
					open.start,
					end,
				);
			}
			return (items, end);
		}
		if kind == TokenKind::End || kind.is_closing() {
			context.error(
				"This `<` opens a generic list that is never closed: close it with `>`, or put a space after `<` for a comparison, like `a < b`",
				open.start,
				open.end,
			);
			return (items, context.last_end());
		}

		let before = context.token_index;
		let item = parse_item(context, &stop_at);
		if closing == TokenKind::GenericClose
			&& let Node::Comparison { operators, .. } = context.at(item)
		{
			// rule 3: `Storage<size < 256>` reads, but a comparison holding `>` can't
			let symbol = operators.first().map_or("<", |operator| operator.symbol());
			let span = context.span(item);
			context.error(
				format!(
					"A comparison inside a generic list goes in parentheses, like `Storage<(size {symbol} 256)>`"
				),
				span.start,
				span.end,
			);
		}
		items.push(item);

		let kind = context.kind();
		if !matches!(kind, TokenKind::Comma | TokenKind::Stop | TokenKind::End)
			&& kind != closing
			&& !kind.is_closing()
		{
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected `,` or `>`, found {found}: separate the items of a generic list with commas or new lines"
			));
			context.skip_until(&stop_at);
		}

		if context.token_index == before
			&& !matches!(
				context.kind(),
				TokenKind::Comma | TokenKind::Stop | TokenKind::End
			) && context.kind() != closing
			&& !context.kind().is_closing()
		{
			context.go_to_next_token();
		}
	}
}

/// Parse the arguments of a generic application, from `<` to `>`: types and compile-time
/// values, positional or named, like `<String, Integer>` or `<min = 0, max = 9>`.
pub(crate) fn parse_generic_arguments(
	context: &mut Context<'_>,
	closing: TokenKind,
) -> (Vec<usize>, usize) {
	parse_generic_list(context, closing, |context, stop_at| {
		parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			stop_at,
		)
	})
}

/// Whether the `<` at the current token, followed by a space, was meant to open a generic list,
/// like `Array< Integer>`: it touches a capitalized name (types are capitalized), and a `>`
/// follows on the same line, in the same brackets. A comparison like `a < b` has spaces on both
/// sides.
pub(crate) fn is_spaced_generic_open(context: &Context<'_>, left: usize) -> bool {
	let token = context.token();
	if token.kind != TokenKind::LessThan || context.span(left).end != token.start {
		return false;
	}
	let name = match context.at(left) {
		Node::Identifier { name } => *name,
		Node::Access { operands } => match operands.last().map(|last| context.at(*last)) {
			Some(Node::Identifier { name }) => *name,
			_ => return false,
		},
		_ => return false,
	};
	if !context
		.name_text(name)
		.starts_with(|character: char| character.is_ascii_uppercase())
	{
		return false;
	}

	let mut depth: usize = 0;
	for token in &context.tokens[context.token_index + 1..] {
		match token.kind {
			TokenKind::GreaterThan if depth == 0 => return true,
			TokenKind::Stop | TokenKind::End if depth == 0 => return false,
			TokenKind::End => return false,
			TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
			| TokenKind::BracesOpen
			| TokenKind::BracketsOpen
			| TokenKind::GenericOpen => depth += 1,
			kind if kind.is_closing() => {
				if depth == 0 {
					return false;
				}
				depth -= 1;
			}
			_ => {}
		}
	}
	false
}

/// Report a `<` followed by a space where a generic list is expected, like `Array< Integer>`.
pub(crate) fn report_spaced_generic_open(context: &mut Context<'_>) {
	context.error_at_token(
		"Remove the space after `<`: a generic list starts with `<` directly followed by its first item, like `Array<Integer>` (a `<` followed by a space is a comparison)",
	);
}
