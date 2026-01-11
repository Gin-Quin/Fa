use crate::{
	context::Context,
	nodes::{FunctionBody, Node},
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
		parse_expression_right::RightExpressionResult,
	},
	priority::Priority,
	source::SourceSpan,
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
	let start = tree.span(left).start;

	context.go_to_next_token();

	if let Some(parameters) = parameters {
		let mut parameter_identifiers = Vec::new();
		collect_parameter_identifiers(tree, parameters, &mut parameter_identifiers);
		for parameter in parameter_identifiers {
			let name = match &tree.nodes[parameter] {
				Node::Identifier { name, .. } => *name,
				_ => continue,
			};
			tree.nodes[parameter] = Node::Identifier { name };
		}
	}

	let (body, end) = if context.token().kind == TokenKind::BracesOpen {
		let (statements, hoisted_symbols) = parse_block_body_with_hoisted(context, "arrow");
		let end = context.last_token().end;
		(
			FunctionBody::Block {
				statements,
				hoisted_symbols,
			},
			end,
		)
	} else {
		let expression = parse_expression(
			context,
			Priority::None,
			ExpressionContext::new(false, false),
			stop_at,
		);
		let end = tree.span(expression).end;
		(FunctionBody::Expression(expression), end)
	};

	let node = Node::ArrowFunction {
		parameters,
		parenthesized_parameters,
		return_type_expression,
		body,
	};

	let span = SourceSpan::new(start, end);
	RightExpressionResult::Yield(tree.insert(node, span))
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

fn collect_parameter_identifiers(
	tree: &TypedSyntaxTree,
	node_index: usize,
	parameters: &mut Vec<usize>,
) {
	match tree.at(node_index) {
		Node::Identifier { .. } => parameters.push(node_index),
		Node::Group { expression } => collect_parameter_identifiers(tree, *expression, parameters),
		Node::Tuple { items } => {
			for item in items {
				collect_parameter_identifiers(tree, *item, parameters);
			}
		}
		Node::Assignment { name, .. } => {
			collect_parameter_identifiers(tree, *name, parameters);
		}
		Node::EmptyGroup => {}
		_ => {}
	}
}
