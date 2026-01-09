use crate::{
	context::Context,
	nodes::{ExtractSymbol, ExtractSymbolKind, ExtractionKind, Node},
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
				let right = parse_expression(context, $priority, false, stop_at);
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
						let right = parse_expression(context, $priority, false, stop_at);
						$elements.push(right);
						let span = span_from_operands(tree, &$elements)
							.unwrap_or(SourceSpan::new(left_span.start, left_span.end));
						(Node::$node_type { $elements }, span)
					}
					_ => {
						context.go_to_next_token();
						let right = parse_expression(context, $priority, false, stop_at);
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
		TokenKind::Extract => {
			if priority >= Priority::Transfer {
				Stop!()
			} else {
				context.go_to_next_token();
				let mut default_kind = None;
				if matches!(context.token.kind, TokenKind::Use | TokenKind::Let) {
					let next_token = context.lookup_next_token_kind();
					if matches!(
						next_token,
						TokenKind::BracesOpen
							| TokenKind::BracketsOpen
							| TokenKind::ParenthesisOpen
							| TokenKind::Identifier
					) {
						default_kind = Some(match context.token.kind {
							TokenKind::Use => ExtractSymbolKind::Alias,
							TokenKind::Let => ExtractSymbolKind::Copy,
							_ => unreachable!(),
						});
						context.go_to_next_token();
					}
				}
				let right = parse_expression(context, Priority::Transfer, false, stop_at);
				let (extraction_kind, symbols) = resolve_extract_symbols(tree, right, default_kind);
				let span = SourceSpan::new(tree.span(left).start, tree.span(right).end);
				(
					Node::Extract {
						left,
						right,
						symbols,
						extraction_kind,
						default_kind,
					},
					span,
				)
			}
		}
		TokenKind::Arrow => Operation!(Relation, Priority::Transfer),
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
				let right = parse_expression(context, Priority::TypeAssignment, false, stop_at);

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
				let right = parse_expression(context, Priority::Assignment, false, stop_at);
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
	context.push_member_access();
	let right = parse_expression(context, Priority::Access, false, stop_at);
	context.pop_member_access();

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

fn resolve_extract_symbols(
	tree: &TypedSyntaxTree,
	right: usize,
	default_kind: Option<ExtractSymbolKind>,
) -> (ExtractionKind, Vec<ExtractSymbol>) {
	match &tree.nodes[right] {
		Node::Members { items } | Node::StaticMembers { items } => (
			ExtractionKind::Member,
			collect_extract_symbols(tree, items, default_kind),
		),
		Node::List { items } | Node::StaticList { items } => (
			ExtractionKind::Index,
			collect_extract_symbols(tree, items, default_kind),
		),
		Node::Tuple { items } => (
			ExtractionKind::TupleMember,
			collect_extract_symbols(tree, items, default_kind),
		),
		Node::Group { expression } => match &tree.nodes[*expression] {
			Node::Tuple { items } => (
				ExtractionKind::TupleMember,
				collect_extract_symbols(tree, items, default_kind),
			),
			Node::Identifier { .. } => (
				ExtractionKind::TupleMember,
				collect_extract_symbols(tree, &[*expression], default_kind),
			),
			_ => panic!("Invalid extract group expression"),
		},
		Node::Identifier { .. } => (
			ExtractionKind::TupleMember,
			collect_extract_symbols(tree, &[right], default_kind),
		),
		Node::ExtractAlias { .. } | Node::ExtractCopy { .. } => (
			ExtractionKind::TupleMember,
			collect_extract_symbols(tree, &[right], default_kind),
		),
		_ => panic!("Invalid extract target"),
	}
}

fn collect_extract_symbols(
	tree: &TypedSyntaxTree,
	items: &[usize],
	default_kind: Option<ExtractSymbolKind>,
) -> Vec<ExtractSymbol> {
	let mut symbols = Vec::new();
	for item in items {
		if let Some(symbol) = extract_symbol_from_item(tree, *item, default_kind) {
			symbols.push(symbol);
		}
	}
	symbols
}

fn extract_symbol_from_item(
	tree: &TypedSyntaxTree,
	item: usize,
	default_kind: Option<ExtractSymbolKind>,
) -> Option<ExtractSymbol> {
	let (override_kind, target) = match &tree.nodes[item] {
		Node::ExtractAlias { .. } => {
			if default_kind.is_some() {
				panic!("Unexpected `use` in extract members");
			}
			(Some(ExtractSymbolKind::Alias), item)
		}
		Node::ExtractCopy { .. } => {
			if default_kind.is_some() {
				panic!("Unexpected `let` in extract members");
			}
			(Some(ExtractSymbolKind::Copy), item)
		}
		_ => (None, item),
	};

	let symbol_kind = override_kind.or(default_kind);
	match &tree.nodes[target] {
		Node::ExtractAlias { name, expression } => symbol_kind.map(|kind| ExtractSymbol {
			name: *name,
			kind,
			default_expression: *expression,
			resolved_type: None,
		}),
		Node::ExtractCopy { name, expression } => symbol_kind.map(|kind| ExtractSymbol {
			name: *name,
			kind,
			default_expression: *expression,
			resolved_type: None,
		}),
		Node::Identifier { name, .. } => symbol_kind.map(|kind| ExtractSymbol {
			name: *name,
			kind,
			default_expression: None,
			resolved_type: None,
		}),
		Node::Assignment { name, .. } => {
			let name_index = *name;
			let name = match &tree.nodes[name_index] {
				Node::Identifier { name, .. } => *name,
				_ => panic!("Expected identifier in extract assignment"),
			};
			symbol_kind.map(|kind| ExtractSymbol {
				name,
				kind,
				default_expression: None,
				resolved_type: None,
			})
		}
		_ => panic!("Invalid extract member"),
	}
}
