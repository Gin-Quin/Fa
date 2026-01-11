use crate::nodes::{FunctionBody, Node};
use crate::tokens::TokenKind;
use crate::{
	context::Context,
	parsing::parse_expression::{ExpressionContext, parse_expression},
	priority::Priority,
	source::SourceSpan,
	typed_syntax_tree::TypedSyntaxTree,
};

pub fn parse_function_declaration(context: &mut Context) -> (Node, usize, bool) {
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
	let value = parse_expression(
		context,
		Priority::None,
		ExpressionContext::new(false, false),
		[],
	);
	let hoisted_symbols = context.exit_hoisted_scope();
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let end = tree.span(value).end;

	let (parameters, return_type_expression, body) = match tree.nodes[value].clone() {
		Node::ArrowFunction {
			parameters,
			parenthesized_parameters,
			return_type_expression,
			body,
			..
		} => {
			let parameters = if parenthesized_parameters {
				match parameters {
					Some(parameters) => {
						if matches!(tree.nodes[parameters], Node::Identifier { .. }) {
							let span = tree.span(parameters);
							Some(tree.insert(
								Node::Group {
									expression: parameters,
								},
								span,
							))
						} else {
							Some(parameters)
						}
					}
					None => {
						let span = tree.span(value);
						Some(tree.insert(Node::EmptyGroup, SourceSpan::new(span.start, span.start)))
					}
				}
			} else {
				parameters
			};
			(parameters, return_type_expression, body)
		}
		_ => (None, None, FunctionBody::Expression(value)),
	};

	(
		Node::Function {
			name,
			parameters,
			return_type_expression,
			body,
			hoisted_symbols,
		},
		end,
		false,
	)
}
