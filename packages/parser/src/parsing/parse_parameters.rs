use crate::{
	context::Context,
	nodes::Node,
	parsing::{
		parse_expression::{ExpressionContext, parse_expression},
		parse_pattern::{is_pattern_start, parse_pattern},
	},
	priority::Priority,
	source::SourceSpan,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

/// Parse the parameters of a function, starting at the opening parenthesis:
/// `(self, name: Type, other: Type = default, ...rest: Integer, { x, y }: Point, { anonymous: Type })`
pub(crate) fn parse_parameters(context: &mut Context) -> Node {
	context.go_to_next_token(); // skip `(`
	let mut items = Vec::new();

	loop {
		while matches!(context.token().kind, TokenKind::Comma | TokenKind::Stop) {
			context.go_to_next_token();
		}

		if context.token().kind == TokenKind::ParenthesisClose {
			context.go_to_next_token();
			return Node::Parameters { items };
		}

		if context.done() {
			panic!("Missing closing `)` after function parameters");
		}

		items.push(parse_parameter(context));

		if !matches!(
			context.token().kind,
			TokenKind::Comma | TokenKind::Stop | TokenKind::ParenthesisClose
		) {
			panic!(
				"Unexpected token '{}' ({:?}) in function parameters",
				context.slice(),
				context.token().kind
			);
		}
	}
}

fn parse_parameter(context: &mut Context) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let start = context.token().start;

	if context.token().kind == TokenKind::Mutable {
		panic!(
			"Parameters are immutable references. Declare the mutation in the signature instead, like `(self) mutates self => ...`"
		);
	}

	// a destructured parameter, like `{ x, y }: Point`
	if is_pattern_start(context.token().kind)
		&& token_after_matching_close(context) == TokenKind::Colon
	{
		let pattern = parse_pattern(context);
		context.go_to_next_token(); // skip `:`
		let type_expression = parse_expression(
			context,
			Priority::Assignment,
			ExpressionContext::new(false, false),
			[TokenKind::Comma, TokenKind::ParenthesisClose],
		);
		let end = tree.span(type_expression).end;
		return tree.insert(
			Node::DestructuredParameter {
				pattern,
				type_expression,
			},
			SourceSpan::new(start, end),
		);
	}

	// a named parameter, `self`, a variadic parameter, or an anonymous structure parameter
	parse_expression(
		context,
		Priority::Comma,
		ExpressionContext::new(false, false),
		[TokenKind::Comma, TokenKind::ParenthesisClose],
	)
}

/// Return the kind of the token that follows the group opened at the current token.
fn token_after_matching_close(context: &Context) -> TokenKind {
	let mut depth = 0;
	let mut index = context.token_index;

	while let Some(token) = context.tokens.get(index) {
		match token.kind {
			TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
			| TokenKind::BracesOpen
			| TokenKind::BracketsOpen => depth += 1,
			TokenKind::ParenthesisClose | TokenKind::BracesClose | TokenKind::BracketsClose => {
				depth -= 1;
				if depth == 0 {
					return context
						.tokens
						.get(index + 1)
						.map(|token| token.kind)
						.unwrap_or(TokenKind::End);
				}
			}
			TokenKind::End => return TokenKind::End,
			_ => {}
		}
		index += 1;
	}

	TokenKind::End
}
