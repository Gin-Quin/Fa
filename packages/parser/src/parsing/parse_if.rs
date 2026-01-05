use crate::{
	context::Context,
	nodes::{IfElseBody, Node},
	parsing::{parse_block_body::parse_block_body, parse_expression::parse_expression},
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub(crate) fn parse_if(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind == TokenKind::BracesOpen {
		panic!("Expected expression after `if`");
	}

	let condition = parse_expression(context, Priority::None, [TokenKind::BracesOpen]);

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after if condition");
	}

	let then_body = parse_block_body(context, "if");
	let mut else_body: Option<IfElseBody> = None;

	if context.token.kind == TokenKind::Else {
		context.go_to_next_token();
		if context.token.kind == TokenKind::If {
			let else_if = parse_if(context);
			let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
			else_body = Some(IfElseBody::If(tree.insert(else_if)));
		} else if context.token.kind == TokenKind::BracesOpen {
			let body = parse_block_body(context, "else");
			else_body = Some(IfElseBody::Block(body));
		} else {
			panic!("Expected `if` or `{{` after `else`");
		}
	}

	Node::If {
		condition,
		then_body,
		else_body,
	}
}
