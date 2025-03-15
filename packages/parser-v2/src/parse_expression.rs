use crate::{
	context::Context,
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
	nodes::Node,
};

/// Parse an expression from the given input.
#[inline]
pub fn parse_expression(context: &mut Context, priority: Priority) -> usize {
	parse_expression_left(context, priority, TokenKind::None)
}

enum RightExpressionResult {
	Stop,
	Continue,
	Yield(usize),
}

use RightExpressionResult::*;

/// Parse the left side of an expression.
fn parse_expression_left(
	context: &mut Context,
	priority: Priority,
	stop_at: TokenKind
) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;
	let mut increment_at_the_end = true;

	context.debug("PARSE LEFT");

	macro_rules! Prefix {
		($node_type:ident, $priority:expr) => {
            {
					context.go_to_next_token();
               increment_at_the_end = false;
               let right = parse_expression_left(context, $priority, stop_at);
               Node::$node_type { right }
            }
		};
	}

	let node: Node = match token.kind {
		TokenKind::Identifier => Node::Identifier(context.slice()),
		TokenKind::Integer => Node::Integer(context.slice().parse::<i32>().unwrap()),
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::Minus => Prefix!(Negate, Priority::Prefix),
		TokenKind::Not => Prefix!(Not, Priority::Not),
		TokenKind::ParenthesisOpen => {
			// group expression or tuple (no function calls, see `parse_expression_right`)
			context.go_to_next_token();

			// check if the next token is a parenthesis close
			if context.token.kind == TokenKind::ParenthesisClose {
				Node::EmptyGroup
			} else {
				Node::Group {
					expression: parse_expression_left(
						context,
						Priority::None,
						TokenKind::ParenthesisClose
					),
				}
			}
		}
		_ => {
			return tree.nodes.insert(Node::DanglingToken {
				token: token.clone(),
			});
		}
	};

	let mut left = tree.nodes.insert(node);

	if increment_at_the_end {
		context.go_to_next_token();
	}

	while !context.done() {
		match parse_expression_right(context, priority, stop_at, left) {
			Stop => {
				break;
			}
			Continue => {}
			Yield(right) => {
				left = right;
			}
		}
	}

	left
}

/// Parse the right side of an expression.
fn parse_expression_right(
	context: &mut Context,
	priority: Priority,
	stop_at: TokenKind,
	left: usize
) -> RightExpressionResult {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;

	context.debug("PARSE RIGHT");

	if token.kind == stop_at {
		return Stop;
	}

	macro_rules! Stop {
		() => { return Stop };
	}

	macro_rules! Operation {
		($node_type:ident, $priority:expr) => {
            if priority >= $priority { Stop!() }
            else {
                context.go_to_next_token();
                let right = parse_expression_left(context, $priority, stop_at);
                Node::$node_type {
                    left,
                    right,
                }
            }
		};
	}

	macro_rules! List {
		($node_type:ident, $elements:ident, $priority:expr) => {
			if priority >= $priority { Stop!() }
			else {
				let left_node = unsafe { tree.nodes.get_unchecked_mut(left).clone() };

				match left_node {
					Node::$node_type { mut $elements } => {
						tree.nodes.remove(left);
						context.go_to_next_token();
						let right = parse_expression_left(context, $priority, stop_at);
						$elements.push(right);
						Node::$node_type { $elements }
					}
					_ => {
						context.go_to_next_token();
						let right = parse_expression_left(context, $priority, stop_at);
						Node::$node_type {
							$elements: vec![left, right],
						}
					}
				}
			}
		};
	}

	let node: Node = match token.kind {
		TokenKind::Stop => Stop!(),

		// Operators
		TokenKind::Plus => List!(Add, operands, Priority::Additive),
		TokenKind::Minus => List!(Subtract, operands, Priority::Additive),
		TokenKind::Star => List!(Multiply, operands, Priority::Multiplicative),
		TokenKind::Slash => List!(Divide, operands, Priority::Multiplicative),
		TokenKind::DoubleSlash =>
			List!(IntegerDivide, operands, Priority::Multiplicative),
		TokenKind::Modulo => List!(Modulo, operands, Priority::Multiplicative),
		TokenKind::DoubleStar => List!(Power, operands, Priority::Exponentiation),
		TokenKind::LessThan => List!(LessThan, operands, Priority::Comparison),
		TokenKind::LessThanOrEqual =>
			List!(LessThanOrEqual, operands, Priority::Comparison),
		TokenKind::GreaterThan => List!(GreaterThan, operands, Priority::Comparison),
		TokenKind::GreaterThanOrEqual =>
			List!(GreaterThanOrEqual, operands, Priority::Comparison),
		TokenKind::DoubleEqual => List!(Equal, operands, Priority::Equality),
		TokenKind::NotEqual => List!(NotEqual, operands, Priority::Equality),
		TokenKind::Is => Operation!(Is, Priority::Equality),
		TokenKind::And => List!(And, operands, Priority::And),
		TokenKind::Or => List!(Or, operands, Priority::Or),
		TokenKind::Union => List!(Union, operands, Priority::Union),
		TokenKind::Pipe => List!(Pipe, operands, Priority::Pipe),
		TokenKind::Insert => Operation!(Insert, Priority::Transfer),
		TokenKind::Extract => Operation!(Extract, Priority::Transfer),

		TokenKind::Comma => List!(Tuple, items, Priority::Comma),

		// -- function call
		TokenKind::ParenthesisOpen => {
			if context.lookup_next_token_kind() == TokenKind::ParenthesisClose {
				Node::FunctionCall { function: left, parameters: None }
			} else {
				context.go_to_next_token();
				let parameters = parse_expression_left(
					context,
					Priority::None,
					TokenKind::ParenthesisClose
				);
				Node::FunctionCall { function: left, parameters: Some(parameters) }
			}
		}

		// -- type assignment
		TokenKind::Colon => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			} else {
				let left_node: Node = unsafe { tree.nodes.get_unchecked(left).clone() };
				context.go_to_next_token();
				let right = parse_expression_left(
					context,
					Priority::TypeAssignment,
					stop_at
				);

				match left_node {
					Node::Identifier(name) => {
						Node::Assignment {
							name: Ok(name),
							type_expression: Some(right),
							expression: None,
						}
					}
					// Node::ParenthesisClose => {
					// 	Node::Assignment {
					// 		name: Err(left),
					// 		type_expression: Some(right),
					// 		expression: None,
					// 	}
					// }
					_ => {
						Node::Assignment {
							name: Err(left),
							type_expression: Some(right),
							expression: None,
						}
					}
				}
			}
		}

		// -- assignment
		TokenKind::Equal => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression_left(context, Priority::Assignment, stop_at);
				let mut left_node = unsafe { tree.nodes.get_unchecked_mut(left) };

				match &mut left_node {
					Node::Identifier(name) => {
						Node::Assignment {
							name: Ok(name),
							type_expression: None,
							expression: Some(right),
						}
					}
					Node::Assignment { expression, .. } => {
						if expression.is_some() {
							// chaining equal assignments, like `a = b = c`, is forbidden
							Node::Assignment {
								name: Err(left),
								type_expression: None,
								expression: Some(right),
							}
						} else {
							// reusing a type assignment, like `a: int`
							*expression = Some(right);
							// context.go_to_next_token();
							return Continue;
						}
					}
					// Node::ParenthesisClose => {
					// 	Node::Assignment {
					// 		name: Err(left),
					// 		type_expression: Some(right),
					// 		expression: None,
					// 	}
					// }
					_ => {
						Node::Assignment {
							name: Err(left),
							type_expression: None,
							expression: Some(right),
						}
					}
				}
			}
		} // for assignment expressions; should not be used

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	Yield(tree.nodes.insert(node))
}
