use crate::{
	context::Context,
	nodes::{IfElseBody, Node},
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
	},
	priority::Priority,
	source::SourceSpan,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub(crate) fn parse_if(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token().kind == TokenKind::BracesOpen {
		panic!("Expected expression after `if`");
	}

	let condition = parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(false, false),
		[TokenKind::BracesOpen],
	);

	if context.token().kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after if condition");
	}

	let (then_body, hoisted_then_symbols) = parse_block_body_with_hoisted(context, "if");
	let mut else_body: Option<IfElseBody> = None;
	let mut hoisted_else_symbols = Vec::new();

	if context.token().kind == TokenKind::Else {
		context.go_to_next_token();
		if context.token().kind == TokenKind::If {
			let start = context.token().start;
			let else_if = parse_if(context);
			let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
			let end = context.last_token().end;
			let span = SourceSpan::new(start, end);
			else_body = Some(IfElseBody::If(tree.insert(else_if, span)));
		} else if context.token().kind == TokenKind::BracesOpen {
			let (body, hoisted_symbols) = parse_block_body_with_hoisted(context, "else");
			else_body = Some(IfElseBody::Block(body));
			hoisted_else_symbols = hoisted_symbols;
		} else {
			panic!("Expected `if` or `{{` after `else`");
		}
	}

	Node::If {
		condition,
		then_body,
		else_body,
		hoisted_then_symbols,
		hoisted_else_symbols,
	}
}
