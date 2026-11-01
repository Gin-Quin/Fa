use crate::context::Context;
use crate::tokens::TokenKind;

/// Parse items between brackets, separated by commas or new lines, starting at the opening
/// bracket: `(a, b)`, `[a, b]`, `{ a, b }`... Return the items and the end of the closing bracket.
/// A missing closing bracket or separator is reported, and the parsing goes on as if it was
/// there, so that one mistake gives one diagnostic.
pub(crate) fn parse_delimited<'source, T>(
	context: &mut Context<'source>,
	closing: TokenKind,
	mut parse_item: impl FnMut(&mut Context<'source>) -> T,
) -> (Vec<T>, usize) {
	let open = context.token();
	let opening = open.kind.describe();
	let closing_text = closing.describe();
	context.go_to_next_token();
	let mut items = Vec::new();

	loop {
		while matches!(context.kind(), TokenKind::Comma | TokenKind::Stop) {
			context.go_to_next_token();
		}

		let kind = context.kind();
		if kind == closing {
			let end = context.token().end;
			context.go_to_next_token();
			return (items, end);
		}
		if kind == TokenKind::End {
			context.error(
				format!("This {opening} is never closed: add a {closing_text}"),
				open.start,
				open.end,
			);
			return (items, context.last_end());
		}
		if kind.is_closing() {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected {closing_text} to close the {opening}, found {found}"
			));
			return (items, context.last_end());
		}

		let before = context.token_index;
		items.push(parse_item(context));

		let kind = context.kind();
		if !matches!(kind, TokenKind::Comma | TokenKind::Stop | TokenKind::End)
			&& !kind.is_closing()
		{
			let found = context.describe_token();
			context.error_at_token(format!(
				"Expected `,` or {closing_text}, found {found}: separate the items with commas or new lines"
			));
			context.skip_until(&[TokenKind::Comma]);
		}

		if context.token_index == before
			&& !matches!(
				context.kind(),
				TokenKind::Comma | TokenKind::Stop | TokenKind::End
			) && !context.kind().is_closing()
		{
			context.go_to_next_token();
		}
	}
}
