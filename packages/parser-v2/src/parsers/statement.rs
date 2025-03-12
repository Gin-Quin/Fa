use crate::{
	context::Context,
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
	nodes::Node,
};

use super::expression::parse_expression;

pub fn parse_statement(context: &mut Context) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = context.token.clone();

	let node: usize = match token.kind {
		/* --------------------- Value declaration or expression -------------------- */
		TokenKind::Identifier => {
			// value declaration or expression
			match context.lookup_next_token_kind() {
				TokenKind::Colon => {
					let identifier = context.slice();
					context.go_to_next_token();
					context.go_to_next_token();
					let type_expression = Some(
						parse_expression(context, Priority::Assignment)
					);
					let mut expression: Option<usize> = None;

					if context.token.kind == TokenKind::Equal {
						context.go_to_next_token();
						expression = Some(parse_expression(context, Priority::None));
					}

					tree.nodes.insert(Node::ValueDeclaration {
						name: identifier,
						type_expression,
						expression,
					})
				}
				TokenKind::Equal => {
					let identifier = context.slice();
					context.go_to_next_token();
					context.go_to_next_token();
					let expression = Some(parse_expression(context, Priority::None));
					tree.nodes.insert(Node::ValueDeclaration {
						name: identifier,
						type_expression: None,
						expression,
					})
				}

				_ => parse_expression(context, Priority::None),
			}
		}

		/* ------------------------------- Expression ------------------------------- */
		_ => parse_expression(context, Priority::None),
	};

	context.go_to_next_token();
	node
}
