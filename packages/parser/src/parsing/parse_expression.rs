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
	source::SourceSpan,
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
	let token_kind = context.token.kind;
	let start = context.token.start;
	let token_end = context.token.end;
	let mut increment_at_the_end = true;

	context.debug("PARSE LEFT");

	macro_rules! Prefix {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			let right = parse_expression(context, $priority, stop_at);
			let end = tree.span(right).end;
			(Node::$node_type { right }, SourceSpan::new(start, end))
		}};
	}

	macro_rules! PrefixWithResolvedType {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			let right = parse_expression(context, $priority, stop_at);
			let end = tree.span(right).end;
			(
				Node::$node_type {
					right,
					resolved_type: None,
				},
				SourceSpan::new(start, end),
			)
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
				(
					Node::$node_type { expression: None },
					SourceSpan::new(start, token_end),
				)
			} else {
				let expression = parse_expression(context, $priority, stop_at);
				let end = tree.span(expression).end;
				(
					Node::$node_type {
						expression: Some(expression),
					},
					SourceSpan::new(start, end),
				)
			}
		}};
	}

	let (node, span): (Node, SourceSpan) = match token_kind {
		TokenKind::Identifier => (
			Node::Identifier(context.slice()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::Integer => (
			Node::Integer(i64::from_str_radix(context.slice(), 10).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::NegativeInteger => (
			Node::Integer(i64::from_str_radix(context.slice(), 10).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::BinaryInteger => (
			Node::Integer(i64::from_str_radix(context.slice_at(2), 2).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::NegativeBinaryInteger => (
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 2).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::OctalInteger => (
			Node::Integer(i64::from_str_radix(context.slice_at(2), 8).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::NegativeOctalInteger => (
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 8).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::HexadecimalInteger => (
			Node::Integer(i64::from_str_radix(context.slice_at(2), 16).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::NegativeHexadecimalInteger => (
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 16).unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::BigInteger => (
			Node::BigInteger(context.slice()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::NegativeBigInteger => (
			Node::BigInteger(context.slice()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::Number => (
			Node::Number(context.slice().parse::<f64>().unwrap()),
			SourceSpan::new(start, token_end),
		),
		TokenKind::String => {
			increment_at_the_end = false;
			let node = parse_string(context);
			(node, SourceSpan::new(start, token_end))
		}
		TokenKind::Null => (Node::Null, SourceSpan::new(start, token_end)),
		TokenKind::True => (Node::Boolean(true), SourceSpan::new(start, token_end)),
		TokenKind::False => (Node::Boolean(false), SourceSpan::new(start, token_end)),
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
		TokenKind::Export => {
			increment_at_the_end = false;
			let node = parse_export(context, stop_at);
			let end = export_span_end(tree, &node, token_end);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Return => PrefixWithOptionalExpression!(Return, Priority::PrefixKeyword),
		TokenKind::Break => PrefixWithOptionalExpression!(Break, Priority::PrefixKeyword),
		TokenKind::Continue => (Node::Continue, SourceSpan::new(start, token_end)),
		TokenKind::Static => Prefix!(Static, Priority::PrefixKeyword),
		TokenKind::For => {
			increment_at_the_end = false;
			let node = parse_for(context, false);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::AtFor => {
			increment_at_the_end = false;
			let node = parse_for(context, true);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::If => {
			increment_at_the_end = false;
			let node = parse_if(context);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::When => {
			increment_at_the_end = false;
			let node = parse_when(context);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::While => {
			increment_at_the_end = false;
			let node = parse_while(context);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Loop => {
			increment_at_the_end = false;
			let node = parse_loop(context);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::BracesOpen => {
			increment_at_the_end = false;
			let node = parse_members(context, false);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::AtBracesOpen => {
			increment_at_the_end = false;
			let node = parse_members(context, true);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::BracketsOpen => {
			increment_at_the_end = false;
			let node = parse_list(context, false);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::AtBracketsOpen => {
			increment_at_the_end = false;
			let node = parse_list(context, true);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::ParenthesisOpen => {
			// group expression or tuple (no function calls, see `parse_expression_right`)
			context.go_to_next_token();

			// check if the next token is a parenthesis close
			if context.token.kind == TokenKind::ParenthesisClose {
				(Node::EmptyGroup, SourceSpan::new(start, context.token.end))
			} else {
				let expression =
					parse_expression(context, Priority::None, [TokenKind::ParenthesisClose]);
				let end = context.token.end;
				(Node::Group { expression }, SourceSpan::new(start, end))
			}
		}

		TokenKind::Function => {
			let (node, should_increment) = parse_function_declaration(context);
			increment_at_the_end = should_increment;
			let end = function_declaration_end(tree, &node, token_end);
			(node, SourceSpan::new(start, end))
		}

		_ => {
			return tree.insert(
				Node::DanglingToken {
					token: context.token.clone(),
				},
				SourceSpan::new(start, token_end),
			);
		}
	};

	let mut left = tree.insert(node, span);

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

fn parse_export<const STOP_COUNT: usize>(
	context: &mut Context,
	stop_at: [TokenKind; STOP_COUNT],
) -> Node {
	context.go_to_next_token();

	let node = match context.token.kind {
		TokenKind::Function => {
			context.go_to_next_token();
			let type_expression = parse_export_type_expression(context);
			let expression = parse_export_expression(context, stop_at);
			Node::ExportFunction {
				type_expression,
				expression,
				resolved_type: None,
			}
		}
		TokenKind::Type => {
			context.go_to_next_token();
			let expression = parse_export_expression(context, stop_at);
			Node::ExportType {
				expression,
				resolved_type: None,
			}
		}
		TokenKind::Namespace => {
			context.go_to_next_token();
			let expression = parse_export_expression(context, stop_at);
			Node::ExportNamespace {
				expression,
				resolved_type: None,
			}
		}
		TokenKind::UnionKeyword => {
			context.go_to_next_token();
			let expression = parse_export_expression(context, stop_at);
			Node::ExportUnion {
				expression,
				resolved_type: None,
			}
		}
		TokenKind::Enum => {
			context.go_to_next_token();
			let expression = parse_export_expression(context, stop_at);
			Node::ExportEnum {
				expression,
				resolved_type: None,
			}
		}
		TokenKind::Fields => {
			context.go_to_next_token();
			let expression = parse_export_expression(context, stop_at);
			Node::ExportFields {
				expression,
				resolved_type: None,
			}
		}
		_ => {
			let type_expression = parse_export_type_expression(context);
			let expression = parse_export_expression(context, stop_at);
			Node::ExportValue {
				type_expression,
				expression,
				resolved_type: None,
			}
		}
	};

	node
}

fn parse_export_type_expression(context: &mut Context) -> Option<usize> {
	if context.token.kind != TokenKind::Colon {
		return None;
	}

	context.go_to_next_token();
	let type_expression = parse_expression(context, Priority::TypeAssignment, [TokenKind::Equal]);

	Some(type_expression)
}

fn parse_export_expression<const STOP_COUNT: usize>(
	context: &mut Context,
	stop_at: [TokenKind; STOP_COUNT],
) -> usize {
	if context.token.kind != TokenKind::Equal {
		panic!("Expected `=` after export declaration");
	}

	context.go_to_next_token();
	parse_expression(context, Priority::PrefixKeyword, stop_at)
}

fn export_span_end(tree: &TypedSyntaxTree, node: &Node, fallback_end: usize) -> usize {
	match node {
		Node::ExportValue { expression, .. } => tree.span(*expression).end,
		Node::ExportFunction { expression, .. } => tree.span(*expression).end,
		Node::ExportType { expression, .. } => tree.span(*expression).end,
		Node::ExportNamespace { expression, .. } => tree.span(*expression).end,
		Node::ExportUnion { expression, .. } => tree.span(*expression).end,
		Node::ExportEnum { expression, .. } => tree.span(*expression).end,
		Node::ExportFields { expression, .. } => tree.span(*expression).end,
		_ => fallback_end,
	}
}

fn function_declaration_end(tree: &TypedSyntaxTree, node: &Node, fallback_end: usize) -> usize {
	match node {
		Node::Function { value, .. } => tree.span(*value).end,
		_ => fallback_end,
	}
}
