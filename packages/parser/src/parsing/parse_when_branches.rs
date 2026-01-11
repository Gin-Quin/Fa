use crate::{
	context::Context,
	nodes::{WhenBranch, WhenBranchPattern, WhenBranchValue},
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
	},
	priority::Priority,
	tokens::TokenKind,
};

pub(crate) fn parse_when_branches(context: &mut Context) -> Vec<WhenBranch> {
	context.go_to_next_token();
	let mut branches: Vec<WhenBranch> = Vec::new();

	if context.token().kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after when branches");
			}

			while context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token().kind == TokenKind::BracesClose {
					break;
				}
			}

			if context.token().kind == TokenKind::BracesClose {
				break;
			}

			let pattern = if context.token().kind == TokenKind::Else {
				context.go_to_next_token();
				WhenBranchPattern::Else
			} else {
				let expression = parse_expression(
					context,
					Priority::None,
					ExpressionContext::new(false, false),
					[TokenKind::FatArrow],
				);
				WhenBranchPattern::Expression(expression)
			};

			if context.token().kind != TokenKind::FatArrow {
				panic!("Expected `=>` after when branch expression");
			}

			context.go_to_next_token();

			let value = if context.token().kind == TokenKind::BracesOpen {
				let (statements, hoisted_symbols) = parse_block_body_with_hoisted(context, "when");
				WhenBranchValue::Block {
					statements,
					hoisted_symbols,
				}
			} else {
				let expression = parse_expression(
					context,
					Priority::None,
					ExpressionContext::new(false, false),
					[TokenKind::Stop, TokenKind::BracesClose],
				);
				WhenBranchValue::Expression(expression)
			};

			branches.push(WhenBranch { pattern, value });

			if context.token().kind == TokenKind::BracesClose {
				break;
			}

			if context.token().kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}

	if context.token().kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after when branches");
	}

	context.go_to_next_token();
	branches
}
