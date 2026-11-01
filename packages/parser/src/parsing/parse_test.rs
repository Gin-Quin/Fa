use crate::context::Context;
use crate::nodes::{Node, TestModifier};
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_string::parse_string;
use crate::tokens::TokenKind;

/// Return true if the `test` identifier at the current token starts a test declaration:
/// `test "name" { }`, `test skip "name" { }` or `test only "name" { }`.
pub(crate) fn is_test_declaration(context: &Context<'_>) -> bool {
	let next = context.next_token();
	match next.kind {
		TokenKind::String => true,
		TokenKind::Identifier => {
			let word = context.text(next.start, next.end);
			(word == "skip" || word == "only")
				&& context
					.tokens
					.get(context.token_index + 2)
					.map(|token| token.kind)
					== Some(TokenKind::String)
		}
		_ => false,
	}
}

/// Parse a test declaration: `test [skip | only] "name" { body }` or `@test "name" { body }`
pub(crate) fn parse_test(context: &mut Context<'_>, is_compile_time: bool) -> usize {
	let start = context.token().start;
	context.go_to_next_token(); // skip `test` or `@test`

	let mut modifier = None;
	if context.kind() == TokenKind::Identifier {
		modifier = match context.slice() {
			"skip" => Some(TestModifier::Skip),
			"only" => Some(TestModifier::Only),
			other => {
				context.error_at_token(format!(
					"Unknown test modifier `{other}`: write `test skip \"...\"` or `test only \"...\"`"
				));
				None
			}
		};
		context.go_to_next_token();
	}

	let name = if context.kind() == TokenKind::String {
		parse_string(context)
	} else {
		context
			.error_at_token("Expected the name of the test, like `test \"adds numbers\" { ... }`");
		context.insert_missing()
	};

	let body = parse_block(context, "test");
	let end = context.last_end();

	context.insert(
		Node::Test {
			name,
			modifier,
			body,
			is_compile_time,
		},
		start,
		end,
	)
}
