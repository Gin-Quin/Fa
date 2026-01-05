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
	let token = &context.token;

	if is_stop_token(&stop_at, token.kind) {
		return Stop;
	}

	if token.kind == TokenKind::FatArrow {
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
				Node::$node_type { left, right }
			}
		};
	}

	macro_rules! List {
		($node_type:ident, $elements:ident, $priority:expr) => {
			if priority >= $priority {
				Stop!()
			} else {
				let left_node = unsafe { tree.nodes.get_unchecked_mut(left).clone() };

				match left_node {
					Node::$node_type { mut $elements } => {
						tree.nodes.remove(left);
						context.go_to_next_token();
						let right = parse_expression(context, $priority, stop_at);
						$elements.push(right);
						Node::$node_type { $elements }
					}
					_ => {
						context.go_to_next_token();
						let right = parse_expression(context, $priority, stop_at);
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
				Node::Percentage { value: left }
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
						Node::OptionalFunctionCall {
							function: left,
							parameters,
						}
					}
					TokenKind::BracketsOpen => {
						context.go_to_next_token();
						if context.token.kind != TokenKind::BracketsOpen {
							panic!("Expected `[` after `?`");
						}
						let index = parse_index_expression(context);
						Node::OptionalIndex {
							target: left,
							index,
						}
					}
					_ => {
						context.go_to_next_token();
						Node::Optional { value: left }
					}
				}
			}
		}
		TokenKind::ExclamationMark => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				context.go_to_next_token();
				Node::Assert { value: left }
			}
		}

		TokenKind::Comma => List!(Tuple, items, Priority::Comma),

		// -- function call
		TokenKind::ParenthesisOpen => {
			let parameters = parse_function_call_parameters(context);
			Node::FunctionCall {
				function: left,
				parameters,
			}
		}

		// -- type assignment
		TokenKind::Colon => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, Priority::TypeAssignment, stop_at);

				Node::Assignment {
					name: left,
					type_expression: Some(right),
					expression: None,
				}
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
						return Continue;
					}
					// Node::ParenthesisClose => {
					// 	Node::Assignment {
					// 		name: Err(left),
					// 		type_expression: Some(right),
					// 		expression: None,
					// 	}
					// }
					_ => Node::Assignment {
						name: left,
						type_expression: None,
						expression: Some(right),
					},
				}
			}
		} // for assignment expressions; should not be used

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	Yield(tree.insert(node))
}
