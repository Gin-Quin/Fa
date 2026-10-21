use crate::{
	context::Context,
	nodes::{CompoundOperator, Node},
	parsing::{
		is_stop_token::is_stop_token,
		parse_arrow_function::parse_arrow_function,
		parse_expression::{ExpressionContext, parse_expression},
		parse_function_call_parameters::parse_function_call_parameters,
		parse_index_expression::parse_index_expression,
		parse_members::parse_members,
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
	let token_kind = context.token().kind;
	let token_end = context.token().end;

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
				let right = parse_expression(
					context,
					$priority,
					ExpressionContext::new(false, false),
					stop_at,
				);
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
						let right = parse_expression(
							context,
							$priority,
							ExpressionContext::new(false, false),
							stop_at,
						);
						$elements.push(right);
						let span = span_from_operands(tree, &$elements)
							.unwrap_or(SourceSpan::new(left_span.start, left_span.end));
						(Node::$node_type { $elements }, span)
					}
					_ => {
						context.go_to_next_token();
						let right = parse_expression(
							context,
							$priority,
							ExpressionContext::new(false, false),
							stop_at,
						);
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
		TokenKind::Is => {
			if priority >= Priority::Equality {
				Stop!()
			} else {
				context.go_to_next_token();
				let negated = context.token().kind == TokenKind::Not;
				if negated {
					context.go_to_next_token();
				}
				let is_in = context.token().kind == TokenKind::In;
				if is_in {
					context.go_to_next_token();
				}
				let right = parse_expression(
					context,
					Priority::Equality,
					ExpressionContext::new(false, false),
					stop_at,
				);
				let span = SourceSpan::new(tree.span(left).start, tree.span(right).end);
				if is_in {
					(
						Node::IsIn {
							left,
							right,
							negated,
						},
						span,
					)
				} else {
					(
						Node::Is {
							left,
							right,
							negated,
						},
						span,
					)
				}
			}
		}
		TokenKind::In => Operation!(In, Priority::Comparison),
		TokenKind::And => List!(And, operands, Priority::And),
		TokenKind::Or => List!(Or, operands, Priority::Or),
		TokenKind::Union => List!(Union, operands, Priority::Union),
		TokenKind::Then => List!(Compose, operands, Priority::Compose),
		TokenKind::Throws => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let errors = parse_expression(
					context,
					Priority::TypeAssignment,
					ExpressionContext::new(false, false),
					stop_at,
				);
				(
					Node::Throws {
						signature: left,
						errors,
					},
					SourceSpan::new(tree.span(left).start, tree.span(errors).end),
				)
			}
		}
		TokenKind::Requires => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				let mut requirements = Vec::new();
				loop {
					context.go_to_next_token();
					let requirement = parse_expression(
						context,
						Priority::Comma,
						ExpressionContext::new(false, false),
						[
							TokenKind::Comma,
							TokenKind::FatArrow,
							TokenKind::ParenthesisClose,
							TokenKind::BracesClose,
							TokenKind::BracketsClose,
						],
					);
					if !matches!(
						tree.at(requirement),
						Node::Assignment {
							type_expression: Some(_),
							expression: None,
							..
						}
					) {
						panic!("Expected a requirement like `name: Type` after `requires`");
					}
					requirements.push(requirement);
					if context.token().kind != TokenKind::Comma {
						break;
					}
				}
				let end = tree
					.span(*requirements.last().expect("one requirement"))
					.end;
				(
					Node::Requires {
						signature: left,
						requirements,
					},
					SourceSpan::new(tree.span(left).start, end),
				)
			}
		}
		TokenKind::DoubleDot | TokenKind::DoubleDotEqual => {
			if priority >= Priority::Range {
				Stop!()
			} else {
				let inclusive = token_kind == TokenKind::DoubleDotEqual;
				context.go_to_next_token();
				let end = if can_start_range_bound(context.token().kind)
					&& !is_stop_token(&stop_at, context.token().kind)
				{
					Some(parse_expression(
						context,
						Priority::Range,
						ExpressionContext::new(false, false),
						stop_at,
					))
				} else {
					if inclusive {
						panic!("An inclusive range `..=` must have an end");
					}
					None
				};
				let end_position = end.map(|end| tree.span(end).end).unwrap_or(token_end);
				(
					Node::Range {
						start: Some(left),
						end,
						inclusive,
					},
					SourceSpan::new(tree.span(left).start, end_position),
				)
			}
		}
		TokenKind::Arrow => Operation!(Relation, Priority::Relation),
		TokenKind::Dot => {
			return parse_access(context, priority, stop_at, left, false);
		}
		TokenKind::QuestionMarkDot => {
			return parse_access(context, priority, stop_at, left, true);
		}
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
				context.go_to_next_token();
				(
					Node::Optional { value: left },
					SourceSpan::new(tree.span(left).start, token_end),
				)
			}
		}

		TokenKind::Comma => List!(Tuple, items, Priority::Comma),

		// -- function call
		TokenKind::ParenthesisOpen => {
			let parameters = parse_function_call_parameters(context);
			let end = context.last_token().end;
			(
				Node::FunctionCall {
					function: left,
					parameters,
				},
				SourceSpan::new(tree.span(left).start, end),
			)
		}
		TokenKind::BracesOpen => {
			let members_node = parse_members(context, false, false);
			let end = context.last_token().end;
			let members = match members_node {
				Node::Members { items } => items,
				_ => unreachable!("Expected members node for member indexing"),
			};
			(
				Node::MemberIndex {
					target: left,
					members,
				},
				SourceSpan::new(tree.span(left).start, end),
			)
		}
		TokenKind::BracketsOpen => {
			let index = parse_index_expression(context);
			let end = context.last_token().end;
			(
				Node::Index {
					target: left,
					index,
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
				// After parameters, like `(a: Integer): Integer`, the type is a return type and
				// stops at `=>`. Otherwise, it's a type annotation that can be a function type.
				let is_return_type = matches!(tree.at(left), Node::Group { .. } | Node::EmptyGroup);
				let type_priority = if is_return_type {
					Priority::TypeAssignment
				} else {
					Priority::Assignment
				};
				let right = parse_expression(
					context,
					type_priority,
					ExpressionContext::new(false, false),
					stop_at,
				);

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

		// -- compound assignment
		TokenKind::PlusEqual
		| TokenKind::MinusEqual
		| TokenKind::StarEqual
		| TokenKind::DoubleStarEqual
		| TokenKind::SlashEqual
		| TokenKind::DoubleSlashEqual => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				let operator = match token_kind {
					TokenKind::PlusEqual => CompoundOperator::Add,
					TokenKind::MinusEqual => CompoundOperator::Subtract,
					TokenKind::StarEqual => CompoundOperator::Multiply,
					TokenKind::DoubleStarEqual => CompoundOperator::Power,
					TokenKind::SlashEqual => CompoundOperator::Divide,
					TokenKind::DoubleSlashEqual => CompoundOperator::IntegerDivide,
					_ => unreachable!(),
				};
				context.go_to_next_token();
				let expression = parse_expression(
					context,
					Priority::Assignment,
					ExpressionContext::new(false, false),
					stop_at,
				);
				(
					Node::CompoundAssignment {
						operator,
						target: left,
						expression,
					},
					SourceSpan::new(tree.span(left).start, tree.span(expression).end),
				)
			}
		}

		// -- assignment
		TokenKind::Equal => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(
					context,
					Priority::Assignment,
					ExpressionContext::new(false, false),
					stop_at,
				);
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

fn parse_access<const STOP_COUNT: usize>(
	context: &mut Context,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
	left: usize,
	optional: bool,
) -> RightExpressionResult {
	if priority >= Priority::Access {
		return RightExpressionResult::Stop;
	}

	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let left_span = tree.span(left);
	let left_node = unsafe { tree.nodes.get_unchecked_mut(left).clone() };

	let should_merge = if optional {
		matches!(left_node, Node::OptionalAccess { .. })
	} else {
		matches!(left_node, Node::Access { .. })
	};
	if should_merge {
		tree.remove(left);
	}

	context.go_to_next_token();
	let right = parse_expression(
		context,
		Priority::Access,
		ExpressionContext::new(false, false),
		stop_at,
	);

	let (node, span) = if optional {
		match left_node {
			Node::OptionalAccess { mut operands } => {
				operands.push(right);
				let span = span_from_operands(tree, &operands)
					.unwrap_or(SourceSpan::new(left_span.start, left_span.end));
				(Node::OptionalAccess { operands }, span)
			}
			_ => {
				let span = SourceSpan::new(left_span.start, tree.span(right).end);
				(
					Node::OptionalAccess {
						operands: vec![left, right],
					},
					span,
				)
			}
		}
	} else {
		match left_node {
			Node::Access { mut operands } => {
				operands.push(right);
				let span = span_from_operands(tree, &operands)
					.unwrap_or(SourceSpan::new(left_span.start, left_span.end));
				(Node::Access { operands }, span)
			}
			_ => {
				let span = SourceSpan::new(left_span.start, tree.span(right).end);
				(
					Node::Access {
						operands: vec![left, right],
					},
					span,
				)
			}
		}
	};

	RightExpressionResult::Yield(tree.insert(node, span))
}

/// Return true if the token can start the end bound of a range, like `b` in `a..b`.
pub(crate) fn can_start_range_bound(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::Identifier
			| TokenKind::Integer
			| TokenKind::NegativeInteger
			| TokenKind::BinaryInteger
			| TokenKind::NegativeBinaryInteger
			| TokenKind::OctalInteger
			| TokenKind::NegativeOctalInteger
			| TokenKind::HexadecimalInteger
			| TokenKind::NegativeHexadecimalInteger
			| TokenKind::BigInteger
			| TokenKind::NegativeBigInteger
			| TokenKind::Number
			| TokenKind::MinusWithoutSpaceAfter
			| TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
	)
}
