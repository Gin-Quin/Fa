use crate::{
	context::Context,
	nodes::Node,
	parsing::{
		is_stop_token::is_stop_token, parse_arrow_function::parse_arrow_function,
		parse_expression::parse_expression,
		parse_function_call_parameters::parse_function_call_parameters,
		parse_index_expression::parse_index_expression,
	},
	priority::Priority,
	source::SourceSpan,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub enum RightExpressionResult {
	Stop,
	Continue,
	Yield(usize),
}

use RightExpressionResult::*;

/// Parse the right side of an expression.
pub(crate) fn parse_expression_right<const STOP_COUNT: usize>(
	context: &mut Context,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
	left: usize,
) -> RightExpressionResult {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token_kind = context.token.kind;
	let token_end = context.token.end;

	if is_stop_token(&stop_at, token_kind) {
		return Stop;
	}

	if token_kind == TokenKind::FatArrow {
		return parse_arrow_function(context, left, priority, stop_at);
	}

	macro_rules! Stop {
		() => {
			return Stop
		};
	}

	macro_rules! Operation {
		($node_type:ident, $priority:expr) => {
			if priority >= $priority {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, $priority, stop_at);
				let span = SourceSpan::new(tree.span(left).start, tree.span(right).end);
				(Node::$node_type { left, right }, span)
			}
		};
	}

	macro_rules! List {
		($node_type:ident, $elements:ident, $priority:expr) => {
			if priority >= $priority {
				Stop!()
			} else {
				let left_span = tree.span(left);
				let left_node = unsafe { tree.nodes.get_unchecked_mut(left).clone() };

				match left_node {
					Node::$node_type { mut $elements } => {
						tree.remove(left);
						context.go_to_next_token();
						let right = parse_expression(context, $priority, stop_at);
						$elements.push(right);
						let span = span_from_operands(tree, &$elements)
							.unwrap_or(SourceSpan::new(left_span.start, left_span.end));
						(Node::$node_type { $elements }, span)
					}
					_ => {
						context.go_to_next_token();
						let right = parse_expression(context, $priority, stop_at);
						let span = SourceSpan::new(left_span.start, tree.span(right).end);
						(
							Node::$node_type {
								$elements: vec![left, right],
							},
							span,
						)
					}
				}
			}
		};
	}

	let (node, span): (Node, SourceSpan) = match token_kind {
		TokenKind::Stop => Stop!(),

		// Operators
		TokenKind::Plus => List!(Add, operands, Priority::Additive),
		TokenKind::MinusWithoutSpaceAfter => List!(Subtract, operands, Priority::Additive),
		TokenKind::Star => List!(Multiply, operands, Priority::Multiplicative),
		TokenKind::Slash => List!(Divide, operands, Priority::Multiplicative),
		TokenKind::DoubleSlash => List!(IntegerDivide, operands, Priority::Multiplicative),
		TokenKind::Modulo => List!(Modulo, operands, Priority::Multiplicative),
		TokenKind::DoubleStar => List!(Power, operands, Priority::Exponentiation),
		TokenKind::LessThan => List!(LessThan, operands, Priority::Comparison),
		TokenKind::LessThanOrEqual => List!(LessThanOrEqual, operands, Priority::Comparison),
		TokenKind::GreaterThan => List!(GreaterThan, operands, Priority::Comparison),
		TokenKind::GreaterThanOrEqual => List!(GreaterThanOrEqual, operands, Priority::Comparison),
		TokenKind::DoubleEqual => List!(Equal, operands, Priority::Equality),
		TokenKind::NotEqual => List!(NotEqual, operands, Priority::Equality),
		TokenKind::Is => Operation!(Is, Priority::Equality),
		TokenKind::And => List!(And, operands, Priority::And),
		TokenKind::Or => List!(Or, operands, Priority::Or),
		TokenKind::Union => List!(Union, operands, Priority::Union),
		TokenKind::Intersection => List!(Intersection, operands, Priority::Union),
		TokenKind::Pipe => List!(Pipe, operands, Priority::Pipe),
		TokenKind::Compose => List!(Compose, operands, Priority::Pipe),
		TokenKind::Insert => Operation!(Insert, Priority::Transfer),
		TokenKind::Extract => Operation!(Extract, Priority::Transfer),
		TokenKind::Arrow => Operation!(Relation, Priority::Transfer),
		TokenKind::Dot => List!(Access, operands, Priority::Access),
		TokenKind::QuestionMarkDot => List!(OptionalAccess, operands, Priority::Access),
		TokenKind::Percent => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				context.go_to_next_token();
				(
					Node::Percentage { value: left },
					SourceSpan::new(tree.span(left).start, token_end),
				)
			}
		}
		TokenKind::QuestionMark => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				match context.lookup_next_token_kind() {
					TokenKind::ParenthesisOpen => {
						context.go_to_next_token();
						if context.token.kind != TokenKind::ParenthesisOpen {
							panic!("Expected `(` after `?`");
						}
						let parameters = parse_function_call_parameters(context);
						let end = context.last_token.end;
						(
							Node::OptionalFunctionCall {
								function: left,
								parameters,
							},
							SourceSpan::new(tree.span(left).start, end),
						)
					}
					TokenKind::BracketsOpen => {
						context.go_to_next_token();
						if context.token.kind != TokenKind::BracketsOpen {
							panic!("Expected `[` after `?`");
						}
						let index = parse_index_expression(context);
						let end = context.last_token.end;
						(
							Node::OptionalIndex {
								target: left,
								index,
							},
							SourceSpan::new(tree.span(left).start, end),
						)
					}
					_ => {
						context.go_to_next_token();
						(
							Node::Optional { value: left },
							SourceSpan::new(tree.span(left).start, token_end),
						)
					}
				}
			}
		}
		TokenKind::ExclamationMark => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				context.go_to_next_token();
				(
					Node::Assert { value: left },
					SourceSpan::new(tree.span(left).start, token_end),
				)
			}
		}

		TokenKind::Comma => List!(Tuple, items, Priority::Comma),

		// -- function call
		TokenKind::ParenthesisOpen => {
			let parameters = parse_function_call_parameters(context);
			let end = context.last_token.end;
			(
				Node::FunctionCall {
					function: left,
					parameters,
				},
				SourceSpan::new(tree.span(left).start, end),
			)
		}

		// -- type assignment
		TokenKind::Colon => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, Priority::TypeAssignment, stop_at);

				(
					Node::Assignment {
						name: left,
						type_expression: Some(right),
						expression: None,
					},
					SourceSpan::new(tree.span(left).start, tree.span(right).end),
				)
			}
		}

		// -- assignment
		TokenKind::Equal => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, Priority::Assignment, stop_at);
				let mut left_node = unsafe { tree.nodes.get_unchecked_mut(left) };

				match &mut left_node {
					Node::Assignment { expression, .. } => {
						if expression.is_some() {
							panic!("Chaining equal assignments, like `a = b = c`, is forbidden");
						}
						// reusing a type assignment, like `a: int`
						*expression = Some(right);
						let end = tree.span(right).end;
						let start = tree.span(left).start;
						tree.spans[left] = SourceSpan::new(start, end);
						return Continue;
					}
					// Node::ParenthesisClose => {
					// 	Node::Assignment {
					// 		name: Err(left),
					// 		type_expression: Some(right),
					// 		expression: None,
					// 	}
					// }
					_ => (
						Node::Assignment {
							name: left,
							type_expression: None,
							expression: Some(right),
						},
						SourceSpan::new(tree.span(left).start, tree.span(right).end),
					),
				}
			}
		} // for assignment expressions; should not be used

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token_kind);
		}
	};

	Yield(tree.insert(node, span))
}

fn span_from_operands(tree: &TypedSyntaxTree, operands: &[usize]) -> Option<SourceSpan> {
	let first = *operands.first()?;
	let last = *operands.last()?;
	Some(SourceSpan::new(tree.span(first).start, tree.span(last).end))
}
