use crate::{
	context::Context,
	nodes::{ArrowFunctionBody, Node},
	parsing::{
		parse_arrow_block_body::parse_arrow_block_body, parse_expression::parse_expression,
		parse_expression_right::RightExpressionResult,
	},
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub fn parse_arrow_function<const STOP_COUNT: usize>(
	context: &mut Context,
	left: usize,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
) -> RightExpressionResult {
	if priority >= Priority::TypeAssignment {
		return RightExpressionResult::Stop;
	}

	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let (parameters, parenthesized_parameters, return_type_expression) =
		resolve_arrow_signature(tree, left);

	context.go_to_next_token();

	let body = if context.token.kind == TokenKind::BracesOpen {
		ArrowFunctionBody::Block(parse_arrow_block_body(context))
	} else {
		let expression = parse_expression(context, Priority::None, stop_at);
		ArrowFunctionBody::Expression(expression)
	};

	let node = Node::ArrowFunction {
		parameters,
		parenthesized_parameters,
		return_type_expression,
		body,
	};

	RightExpressionResult::Yield(tree.insert(node))
}

fn resolve_arrow_signature(
	tree: &TypedSyntaxTree,
	left: usize,
) -> (Option<usize>, bool, Option<usize>) {
	match unsafe { tree.nodes.get_unchecked(left) } {
		Node::Group { expression, .. } => (Some(*expression), true, None),
		Node::EmptyGroup => (None, true, None),
		Node::Assignment {
			name,
			type_expression,
			expression,
			..
		} => {
			if expression.is_some() {
				panic!("Unexpected expression before arrow function body");
			}
			let return_type_expression = match type_expression {
				Some(value) => Some(*value),
				None => panic!("Expected return type expression before arrow function"),
			};
			let parameter_node = unsafe { tree.nodes.get_unchecked(*name) };
			match parameter_node {
				Node::Group { expression, .. } => (Some(*expression), true, return_type_expression),
				Node::EmptyGroup => (None, true, return_type_expression),
				_ => {
					panic!("Arrow function return types require parameters wrapped in parentheses");
				}
			}
		}
		_ => (Some(left), false, None),
	}
}
