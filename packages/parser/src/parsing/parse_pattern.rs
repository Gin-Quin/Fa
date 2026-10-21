use crate::{
	context::Context,
	nodes::{Node, PatternField},
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	source::SourceSpan,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

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

/// Parse a name or a destructuring pattern:
/// `name`, `{ a, b as c, d = 4, nested { e } }`, `[first, second]` or `(number, name)`.
pub(crate) fn parse_pattern(context: &mut Context) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let start = context.token().start;

	match context.token().kind {
		TokenKind::Identifier => {
			let name = context.slice();
			let end = context.token().end;
			context.go_to_next_token();
			tree.insert(Node::Identifier { name }, SourceSpan::new(start, end))
		}
		TokenKind::BracesOpen => {
			let fields = parse_object_pattern_fields(context);
			let end = context.last_token().end;
			tree.insert(Node::ObjectPattern { fields }, SourceSpan::new(start, end))
		}
		TokenKind::BracketsOpen => {
			let items = parse_pattern_items(context, TokenKind::BracketsClose);
			let end = context.last_token().end;
			tree.insert(Node::ArrayPattern { items }, SourceSpan::new(start, end))
		}
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration => {
			let items = parse_pattern_items(context, TokenKind::ParenthesisClose);
			let end = context.last_token().end;
			tree.insert(Node::TuplePattern { items }, SourceSpan::new(start, end))
		}
		_ => panic!(
			"Expected a name or a destructuring pattern, found '{}' ({:?})",
			context.slice(),
			context.token().kind
		),
	}
}

/// Skip the separators between two items of a pattern (commas and new lines).
fn skip_separators(context: &mut Context) {
	while matches!(context.token().kind, TokenKind::Comma | TokenKind::Stop) {
		context.go_to_next_token();
	}
}

fn parse_object_pattern_fields(context: &mut Context) -> Vec<PatternField> {
	context.go_to_next_token(); // skip `{`
	let mut fields = Vec::new();

	loop {
		skip_separators(context);

		if context.token().kind == TokenKind::BracesClose {
			context.go_to_next_token();
			return fields;
		}

		if context.done() {
			panic!("Missing closing `}}` after pattern");
		}

		if context.token().kind != TokenKind::Identifier {
			panic!(
				"Expected a field name in pattern, found '{}' ({:?})",
				context.slice(),
				context.token().kind
			);
		}

		let name = context.slice();
		context.go_to_next_token();

		let mut field = PatternField {
			name,
			alias: None,
			nested: None,
			default: None,
		};

		if is_pattern_start(context.token().kind) {
			field.nested = Some(parse_pattern(context));
		} else if context.token().kind == TokenKind::As {
			context.go_to_next_token();
			if context.token().kind != TokenKind::Identifier {
				panic!("Expected a name after `as`");
			}
			field.alias = Some(context.slice());
			context.go_to_next_token();
		}

		if context.token().kind == TokenKind::Equal {
			context.go_to_next_token();
			field.default = Some(parse_expression(
				context,
				Priority::Comma,
				ExpressionContext::new(false, false),
				[TokenKind::Comma, TokenKind::BracesClose],
			));
		}

		fields.push(field);

		if !matches!(
			context.token().kind,
			TokenKind::Comma | TokenKind::Stop | TokenKind::BracesClose
		) {
			panic!(
				"Unexpected token '{}' ({:?}) in pattern",
				context.slice(),
				context.token().kind
			);
		}
	}
}

fn parse_pattern_items(context: &mut Context, closing: TokenKind) -> Vec<usize> {
	context.go_to_next_token(); // skip the opening token
	let mut items = Vec::new();

	loop {
		skip_separators(context);

		if context.token().kind == closing {
			context.go_to_next_token();
			return items;
		}

		if context.done() {
			panic!("Missing closing {closing:?} after pattern");
		}

		items.push(parse_pattern(context));

		if !matches!(context.token().kind, TokenKind::Comma | TokenKind::Stop)
			&& context.token().kind != closing
		{
			panic!(
				"Unexpected token '{}' ({:?}) in pattern",
				context.slice(),
				context.token().kind
			);
		}
	}
}
