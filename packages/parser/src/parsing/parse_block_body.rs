use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;

/// Parse statements up to `closing` (or the end of the file when `closing` is `End`). The
/// closing token is not consumed. A statement ends at a new line: anything else after it is
/// reported, then skipped up to the next line.
pub(crate) fn parse_statements(context: &mut Context<'_>, closing: TokenKind) -> Vec<usize> {
	let mut statements = Vec::new();
	let stop_at = [closing];

	loop {
		context.skip_stops();
		let kind = context.kind();
		if kind == closing || kind == TokenKind::End {
			return statements;
		}
		if kind.is_closing() {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Unexpected {found}: there is no opening bracket to close"
			));
			context.go_to_next_token();
			continue;
		}

		let before = context.token_index;
		statements.push(parse_expression(
			context,
			Priority::None,
			ExpressionContext::new(true, false),
			&stop_at,
		));

		let kind = context.kind();
		if kind != TokenKind::Stop && kind != closing && kind != TokenKind::End {
			let found = context.describe_token();
			context.error_at_token(format!(
				"Unexpected {found} after the end of the statement: a statement ends at the end of its line. Add an operator, or start a new line"
			));
			context.skip_to_statement_end();
		}

		if context.token_index == before {
			context.go_to_next_token();
		}
	}
}

/// Parse a block, from its `{` to its `}`, into a `Block` node with its own scope of hoisted
/// declarations. `label` names the construct that owns the block in diagnostics, like "if".
/// A missing `{` is reported, and gives an empty block.
pub(crate) fn parse_block(context: &mut Context<'_>, label: &str) -> usize {
	let open = context.token();
	if open.kind != TokenKind::BracesOpen {
		let found = context.describe_token();
		context.error_at_token(format!(
			"Expected `{{` to start the {label} block, found {found}"
		));
		let position = context.last_end();
		return context.insert(
			Node::Block {
				statements: Vec::new(),
				hoisted_symbols: Vec::new(),
			},
			position,
			position,
		);
	}
	context.go_to_next_token();
	context.enter_hoisted_scope();
	let statements = parse_statements(context, TokenKind::BracesClose);

	let end = if context.kind() == TokenKind::BracesClose {
		let end = context.token().end;
		context.go_to_next_token();
		end
	} else {
		context.error(
			format!("This `{{` of the {label} block is never closed: add a `}}`"),
			open.start,
			open.end,
		);
		context.last_end()
	};

	let hoisted_symbols = context.exit_hoisted_scope();
	context.insert(
		Node::Block {
			statements,
			hoisted_symbols,
		},
		open.start,
		end,
	)
}

/// Parse a keyword followed by a block, like `loop { ... }`, `do { ... }` or `watch { ... }`.
pub(crate) fn parse_keyword_block(
	context: &mut Context<'_>,
	keyword: &str,
	make: impl FnOnce(usize) -> Node,
) -> usize {
	let start = context.token().start;
	context.go_to_next_token();
	let body = parse_block(context, keyword);
	let end = context.last_end();
	context.insert(make(body), start, end)
}
