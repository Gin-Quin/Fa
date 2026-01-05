use crate::{
	context::Context,
	nodes::Node,
	parsing::{
		is_stop_token::is_stop_token,
		parse_expression_right::{RightExpressionResult, parse_expression_right},
		parse_for::parse_for,
		parse_function_declaration::parse_function_declaration,
		parse_if::parse_if,
		parse_list::parse_list,
		parse_loop::parse_loop,
		parse_members::parse_members,
		parse_string::parse_string,
		parse_when::parse_when,
		parse_while::parse_while,
	},
	priority::Priority,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

use RightExpressionResult::{Continue, Stop, Yield};

/// Parse the left side of an expression.
pub fn parse_expression<const STOP_COUNT: usize>(
	context: &mut Context,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;
	let mut increment_at_the_end = true;

	context.debug("PARSE LEFT");

	macro_rules! Prefix {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			let right = parse_expression(context, $priority, stop_at);
			Node::$node_type { right }
		}};
	}

	macro_rules! PrefixWithResolvedType {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			let right = parse_expression(context, $priority, stop_at);
			Node::$node_type {
				right,
				resolved_type: None,
			}
		}};
	}

	macro_rules! PrefixWithOptionalExpression {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			if context.token.kind == TokenKind::Stop
				|| context.token.kind == TokenKind::End
				|| is_stop_token(&stop_at, context.token.kind)
			{
				Node::$node_type { expression: None }
			} else {
				let expression = parse_expression(context, $priority, stop_at);
				Node::$node_type {
					expression: Some(expression),
				}
			}
		}};
	}

	let node: Node = match token.kind {
		TokenKind::Identifier => Node::Identifier(context.slice()),
		TokenKind::Integer => Node::Integer(i64::from_str_radix(context.slice(), 10).unwrap()),
		TokenKind::NegativeInteger => {
			Node::Integer(i64::from_str_radix(context.slice(), 10).unwrap())
		}
		TokenKind::BinaryInteger => {
			Node::Integer(i64::from_str_radix(context.slice_at(2), 2).unwrap())
		}
		TokenKind::NegativeBinaryInteger => {
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 2).unwrap())
		}
		TokenKind::OctalInteger => {
			Node::Integer(i64::from_str_radix(context.slice_at(2), 8).unwrap())
		}
		TokenKind::NegativeOctalInteger => {
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 8).unwrap())
		}
		TokenKind::HexadecimalInteger => {
			Node::Integer(i64::from_str_radix(context.slice_at(2), 16).unwrap())
		}
		TokenKind::NegativeHexadecimalInteger => {
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 16).unwrap())
		}
		TokenKind::BigInteger => Node::BigInteger(context.slice()),
		TokenKind::NegativeBigInteger => Node::BigInteger(context.slice()),
		TokenKind::Number => Node::Number(context.slice().parse::<f64>().unwrap()),
		TokenKind::String => {
			increment_at_the_end = false;
			parse_string(context)
		}
		TokenKind::Null => Node::Null,
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::MinusWithoutSpaceAfter => Prefix!(Negate, Priority::Prefix),
		TokenKind::TripleDot => Prefix!(Spread, Priority::Prefix),
		TokenKind::Not => Prefix!(Not, Priority::Not),
		TokenKind::Let => PrefixWithResolvedType!(Let, Priority::PrefixKeyword),
		TokenKind::Mutable => PrefixWithResolvedType!(Mutable, Priority::PrefixKeyword),
		TokenKind::Type => PrefixWithResolvedType!(Type, Priority::PrefixKeyword),
		TokenKind::UnionKeyword => Prefix!(UnionDeclaration, Priority::PrefixKeyword),
		TokenKind::Enum => Prefix!(Enum, Priority::PrefixKeyword),
		TokenKind::Fields => Prefix!(Fields, Priority::PrefixKeyword),
		TokenKind::Reactive => PrefixWithResolvedType!(Reactive, Priority::PrefixKeyword),
		TokenKind::Derived => PrefixWithResolvedType!(Derived, Priority::PrefixKeyword),
		TokenKind::Namespace => Prefix!(Namespace, Priority::PrefixKeyword),
		TokenKind::Return => PrefixWithOptionalExpression!(Return, Priority::PrefixKeyword),
		TokenKind::Break => PrefixWithOptionalExpression!(Break, Priority::PrefixKeyword),
		TokenKind::Continue => Node::Continue,
		TokenKind::Static => Prefix!(Static, Priority::PrefixKeyword),
		TokenKind::For => {
			increment_at_the_end = false;
			parse_for(context, false)
		}
		TokenKind::AtFor => {
			increment_at_the_end = false;
			parse_for(context, true)
		}
		TokenKind::If => {
			increment_at_the_end = false;
			parse_if(context)
		}
		TokenKind::When => {
			increment_at_the_end = false;
			parse_when(context)
		}
		TokenKind::While => {
			increment_at_the_end = false;
			parse_while(context)
		}
		TokenKind::Loop => {
			increment_at_the_end = false;
			parse_loop(context)
		}
		TokenKind::BracesOpen => {
			increment_at_the_end = false;
			parse_members(context, false)
		}
		TokenKind::AtBracesOpen => {
			increment_at_the_end = false;
			parse_members(context, true)
		}
		TokenKind::BracketsOpen => {
			increment_at_the_end = false;
			parse_list(context, false)
		}
		TokenKind::AtBracketsOpen => {
			increment_at_the_end = false;
			parse_list(context, true)
		}
		TokenKind::ParenthesisOpen => {
			// group expression or tuple (no function calls, see `parse_expression_right`)
			context.go_to_next_token();

			// check if the next token is a parenthesis close
			if context.token.kind == TokenKind::ParenthesisClose {
				Node::EmptyGroup
			} else {
				Node::Group {
					expression: parse_expression(
						context,
						Priority::None,
						[TokenKind::ParenthesisClose],
					),
				}
			}
		}

		TokenKind::Function => {
			let (node, should_increment) = parse_function_declaration(context);
			increment_at_the_end = should_increment;
			node
		}

		_ => {
			return tree.insert(Node::DanglingToken {
				token: token.clone(),
			});
		}
	};

	let mut left = tree.insert(node);

	if increment_at_the_end {
		context.go_to_next_token();
	}

	while !context.done() {
		let result = parse_expression_right(context, priority, stop_at, left);
		match result {
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
