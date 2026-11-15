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
/// separated by commas: a line break never separates two items, it ends a list left open (only
/// the list read to recover takes its items line by line). A list that is never closed is
/// reported where it opens: it is often a comparison written without a space after `<`, which
/// the message spells with `target`, the expression before the `<` of an application.
pub(crate) fn parse_generic_list<'source>(
	context: &mut Context<'source>,
	closing: TokenKind,
	target: Option<usize>,
	mut parse_item: impl FnMut(&mut Context<'source>, &[TokenKind]) -> usize,
) -> (Vec<usize>, usize) {
	let open = context.token();
	context.go_to_next_token();
	let stop_at = [TokenKind::Comma, closing];
	let recovers = closing != TokenKind::GenericClose;
	let mut items = Vec::new();

	loop {
		while context.kind() == TokenKind::Comma || (recovers && context.kind() == TokenKind::Stop)
		{
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
		if matches!(kind, TokenKind::Stop | TokenKind::End) || kind.is_closing() {
			let message = unclosed_list_message(context, target, &items);
			context.error(message, open.start, open.end);
			return (items, context.last_end());
		}

		let before = context.token_index;
		let item = parse_item(context, &stop_at);
		if !recovers && let Node::Comparison { operators, .. } = context.at(item) {
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
				"Expected `,` or `>`, found {found}: separate the items of a generic list with commas"
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

/// The message of a generic list that is never closed. After an expression, like in
/// `if count<limit { … }`, it gives the comparison to write: `count < limit`.
fn unclosed_list_message(context: &Context<'_>, target: Option<usize>, items: &[usize]) -> String {
	const MAX_OPERAND_LENGTH: usize = 40;
	let operands = target.zip(items.first()).and_then(|(target, first)| {
		let target = context.span(target);
		let first = context.span(*first);
		// the operand only, not the block that follows it, like in `if count<limit { … }`
		let first = context
			.text(first.start, first.end)
			.split(|character: char| character.is_whitespace() || character == '{')
			.next()
			.filter(|first| !first.is_empty())?;
		(target.end - target.start <= MAX_OPERAND_LENGTH)
			.then(|| (context.text(target.start, target.end), first))
	});
	match operands {
		Some((target, first)) => format!(
			"`<{first}` opens a generic list that is never closed: close it with `>`, or put a space after `<` for a comparison: write `{target} < {first}`"
		),
		None => "This `<` opens a generic list that is never closed: close it with `>`, or put a space after `<` for a comparison, like `a < b`".to_string(),
	}
}

/// Parse the arguments of the generic application of `target`, from `<` to `>`: types and
/// compile-time values, positional or named, like `<String, Integer>` or `<min = 0, max = 9>`.
pub(crate) fn parse_generic_arguments(
	context: &mut Context<'_>,
	closing: TokenKind,
	target: usize,
) -> (Vec<usize>, usize) {
	parse_generic_list(context, closing, Some(target), |context, stop_at| {
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

/// The list written between the bracket at the current token and the bracket that closes it,
/// like `Left, Right` in `(Left, Right)`, for a message that tells where to write it. `Item`
/// when the list is empty, never closed or on several lines.
pub(crate) fn written_generic_list<'source>(context: &Context<'source>) -> &'source str {
	let open = context.token();
	let mut depth: usize = 0;
	for token in &context.tokens[context.token_index + 1..] {
		match token.kind {
			TokenKind::End => break,
			TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
			| TokenKind::BracesOpen
			| TokenKind::BracketsOpen
			| TokenKind::GenericOpen => depth += 1,
			kind if kind.is_closing() => {
				if depth == 0 {
					let list = context.text(open.end, token.start).trim();
					if !list.is_empty() && !list.contains('\n') {
						return list;
					}
					break;
				}
				depth -= 1;
			}
			_ => {}
		}
	}
	"Item"
}

/// Report a `<` followed by a space where a generic list is expected, like `Array< Integer>`.
pub(crate) fn report_spaced_generic_open(context: &mut Context<'_>) {
	context.error_at_token(
		"Remove the space after `<`: a generic list starts with `<` directly followed by its first item, like `Array<Integer>` (a `<` followed by a space is a comparison)",
	);
}
