use crate::nodes::Node;
use crate::tokens::TokenKind;
use crate::{
	context::Context,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	typed_syntax_tree::TypedSyntaxTree,
};

/// Parse a function declaration, starting at the `function` keyword:
/// `function name = <function expression>`.
pub fn parse_function_declaration(context: &mut Context, is_async: bool) -> (Node, usize) {
	context.go_to_next_token();

	if context.token().kind != TokenKind::Identifier {
		panic!("Expected identifier after `function` keyword");
	}

	let name = context.slice();
	context.go_to_next_token();

	if context.token().kind != TokenKind::Equal {
		panic!("Expected `=` after function name");
	}

	context.go_to_next_token();

	context.enter_hoisted_scope();
	let expression = parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(false, false),
		[
			TokenKind::Comma,
			TokenKind::ParenthesisClose,
			TokenKind::BracesClose,
			TokenKind::BracketsClose,
		],
	);
	let hoisted_symbols = context.exit_hoisted_scope();
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let end = tree.span(expression).end;

	if is_async && !matches!(tree.at(expression), Node::ArrowFunction { .. }) {
		panic!("An `async` function must be declared with parameters and a body");
	}

	(
		Node::Function {
			name,
			is_async,
			expression,
			hoisted_symbols,
		},
		end,
	)
}
