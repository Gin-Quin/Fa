use crate::{
	context::Context,
	nodes::{IdentifierReference, Node},
	parsing::{
		is_stop_token::is_stop_token,
		parse_do::parse_do,
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

enum VariableDeclarationKind {
	Let,
	Mutable,
	Use,
	Reactive,
	Derived,
}

enum TypeDeclarationKind {
	Type,
	UnionDeclaration,
	Enum,
	Fields,
	Namespace,
}

/// Parse the left side of an expression.
pub fn parse_expression<const STOP_COUNT: usize>(
	context: &mut Context,
	priority: Priority,
	is_statement_start: bool,
	stop_at: [TokenKind; STOP_COUNT],
) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token_kind = context.token.kind;
	let start = context.token.start;
	let token_end = context.token.end;
	let mut increment_at_the_end = true;
	let mut external_identifier_name: Option<&'static str> = None;

	context.debug("PARSE LEFT");

	macro_rules! Prefix {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			let right = parse_expression(context, $priority, false, stop_at);
			let end = tree.span(right).end;
			(Node::$node_type { right }, SourceSpan::new(start, end))
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
				let expression = parse_expression(context, $priority, false, stop_at);
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
		TokenKind::Identifier => {
			let name = context.slice();
			let reference = if context.is_in_member_access() {
				IdentifierReference::External
			} else if let Some(declaration) = context.resolve_symbol(name) {
				IdentifierReference::Declaration(declaration)
			} else {
				external_identifier_name = Some(name);
				IdentifierReference::External
			};
			(
				Node::Identifier { name, reference },
				SourceSpan::new(start, token_end),
			)
		}
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
		TokenKind::Let => {
			if is_statement_start {
				increment_at_the_end = false;
				let (node, end) =
					parse_variable_declaration(context, VariableDeclarationKind::Let, stop_at);
				(node, SourceSpan::new(start, end))
			} else {
				increment_at_the_end = false;
				context.go_to_next_token();
				if context.token.kind != TokenKind::Identifier {
					panic!("Expected identifier after `let`");
				}
				let name = context.slice();
				let name_end = context.token.end;
				context.go_to_next_token();
				if context.token.kind == TokenKind::Colon {
					panic!("`let` extraction does not support type annotations");
				}
				let mut expression = None;
				if context.token.kind == TokenKind::Equal {
					context.go_to_next_token();
					if context.token.kind == TokenKind::Stop
						|| context.token.kind == TokenKind::End
						|| is_stop_token(&stop_at, context.token.kind)
					{
						panic!("Expected expression after `=`");
					}
					expression = Some(parse_expression(
						context,
						Priority::PrefixKeyword,
						false,
						stop_at,
					));
				}
				let end = expression
					.map(|value| tree.span(value).end)
					.unwrap_or(name_end);
				(
					Node::ExtractCopy { name, expression },
					SourceSpan::new(start, end),
				)
			}
		}
		TokenKind::Mutable => {
			increment_at_the_end = false;
			let (node, end) =
				parse_variable_declaration(context, VariableDeclarationKind::Mutable, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Use => {
			if is_statement_start {
				increment_at_the_end = false;
				let (node, end) =
					parse_variable_declaration(context, VariableDeclarationKind::Use, stop_at);
				(node, SourceSpan::new(start, end))
			} else {
				increment_at_the_end = false;
				context.go_to_next_token();
				if context.token.kind != TokenKind::Identifier {
					panic!("Expected identifier after `use`");
				}
				let name = context.slice();
				let name_end = context.token.end;
				context.go_to_next_token();
				if context.token.kind == TokenKind::Colon {
					panic!("`use` extraction does not support type annotations");
				}
				let mut expression = None;
				if context.token.kind == TokenKind::Equal {
					context.go_to_next_token();
					if context.token.kind == TokenKind::Stop
						|| context.token.kind == TokenKind::End
						|| is_stop_token(&stop_at, context.token.kind)
					{
						panic!("Expected expression after `=`");
					}
					expression = Some(parse_expression(
						context,
						Priority::PrefixKeyword,
						false,
						stop_at,
					));
				}
				let end = expression
					.map(|value| tree.span(value).end)
					.unwrap_or(name_end);
				(
					Node::ExtractAlias { name, expression },
					SourceSpan::new(start, end),
				)
			}
		}
		TokenKind::Reactive => {
			increment_at_the_end = false;
			let (node, end) =
				parse_variable_declaration(context, VariableDeclarationKind::Reactive, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Derived => {
			increment_at_the_end = false;
			let (node, end) =
				parse_variable_declaration(context, VariableDeclarationKind::Derived, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Type => {
			increment_at_the_end = false;
			let (node, end) = parse_type_declaration(context, TypeDeclarationKind::Type, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::UnionKeyword => {
			increment_at_the_end = false;
			let (node, end) =
				parse_type_declaration(context, TypeDeclarationKind::UnionDeclaration, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Enum => {
			increment_at_the_end = false;
			let (node, end) = parse_type_declaration(context, TypeDeclarationKind::Enum, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Fields => {
			increment_at_the_end = false;
			let (node, end) = parse_type_declaration(context, TypeDeclarationKind::Fields, stop_at);
			(node, SourceSpan::new(start, end))
		}
		TokenKind::Namespace => {
			increment_at_the_end = false;
			let (node, end) =
				parse_type_declaration(context, TypeDeclarationKind::Namespace, stop_at);
			(node, SourceSpan::new(start, end))
		}
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
		TokenKind::Do => {
			increment_at_the_end = false;
			let node = parse_do(context);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::BracesOpen => {
			increment_at_the_end = false;
			let node = parse_members(context, false, is_statement_start);
			let end = context.last_token.end;
			(node, SourceSpan::new(start, end))
		}
		TokenKind::AtBracesOpen => {
			increment_at_the_end = false;
			let node = parse_members(context, true, is_statement_start);
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
				let expression = parse_expression(
					context,
					Priority::None,
					false,
					[TokenKind::ParenthesisClose],
				);
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

	if let Some(name) = external_identifier_name {
		context.add_external_reference(name, left);
		tree.external_symbols
			.entry(name.to_string())
			.or_default()
			.push(left);
	}

	match tree.nodes[left].clone() {
		Node::Let { name, .. }
		| Node::Mutable { name, .. }
		| Node::Use { name, .. }
		| Node::Reactive { name, .. }
		| Node::Derived { name, .. }
		| Node::Type { name, .. }
		| Node::UnionDeclaration { name, .. }
		| Node::Enum { name, .. }
		| Node::Fields { name, .. }
		| Node::Namespace { name, .. } => {
			context.declare_symbol(name, left);
		}
		Node::Function { name, .. } => {
			context.declare_symbol(name, left);
			if let Some(references) = context.take_external_references(name) {
				for reference in &references {
					if let Node::Identifier {
						reference: target, ..
					} = &mut tree.nodes[*reference]
					{
						*target = IdentifierReference::Declaration(left);
					}
				}
				remove_external_references(tree, name, &references);
			}
		}
		_ => {}
	}

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

	if let Node::Extract { symbols, .. } = &tree.nodes[left] {
		for symbol in symbols {
			context.declare_symbol(symbol.name, left);
		}
	}

	left
}

fn parse_variable_declaration<const STOP_COUNT: usize>(
	context: &mut Context,
	kind: VariableDeclarationKind,
	stop_at: [TokenKind; STOP_COUNT],
) -> (Node, usize) {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };

	context.go_to_next_token();
	if context.token.kind != TokenKind::Identifier {
		panic!("Expected identifier after declaration keyword");
	}

	let name = context.slice();
	context.go_to_next_token();

	let mut type_expression = None;
	let mut expression = None;

	if matches!(kind, VariableDeclarationKind::Use) && context.token.kind != TokenKind::Colon {
		if context.token.kind != TokenKind::Equal {
			panic!("Expected `=` after use declaration name");
		}
	}

	if context.token.kind == TokenKind::Colon {
		context.go_to_next_token();
		type_expression = Some(parse_expression(
			context,
			Priority::TypeAssignment,
			false,
			[TokenKind::Equal],
		));
	}

	if matches!(kind, VariableDeclarationKind::Use) && context.token.kind != TokenKind::Equal {
		panic!("Expected `=` after use declaration name");
	}

	if context.token.kind == TokenKind::Equal {
		context.go_to_next_token();
		if context.token.kind == TokenKind::Stop
			|| context.token.kind == TokenKind::End
			|| is_stop_token(&stop_at, context.token.kind)
		{
			panic!("Expected expression after `=`");
		}
		expression = Some(parse_expression(
			context,
			Priority::PrefixKeyword,
			false,
			stop_at,
		));
	}

	if matches!(kind, VariableDeclarationKind::Use) && expression.is_none() {
		panic!("Expected expression after `=` in use declaration");
	}

	let end = if let Some(expression) = expression {
		tree.span(expression).end
	} else if let Some(type_expression) = type_expression {
		tree.span(type_expression).end
	} else {
		panic!("Expected type expression or value in declaration");
	};

	let node = match kind {
		VariableDeclarationKind::Let => Node::Let {
			name,
			type_expression,
			expression,
			resolved_type: None,
		},
		VariableDeclarationKind::Mutable => Node::Mutable {
			name,
			type_expression,
			expression,
			resolved_type: None,
		},
		VariableDeclarationKind::Use => {
			let expression = expression.expect("Use declarations require a value");
			Node::Use {
				name,
				type_expression,
				expression,
				resolved_type: None,
			}
		}
		VariableDeclarationKind::Reactive => Node::Reactive {
			name,
			type_expression,
			expression,
			resolved_type: None,
		},
		VariableDeclarationKind::Derived => Node::Derived {
			name,
			type_expression,
			expression,
			resolved_type: None,
		},
	};

	(node, end)
}

fn parse_type_declaration<const STOP_COUNT: usize>(
	context: &mut Context,
	kind: TypeDeclarationKind,
	stop_at: [TokenKind; STOP_COUNT],
) -> (Node, usize) {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };

	context.go_to_next_token();
	if context.token.kind != TokenKind::Identifier {
		panic!("Expected identifier after declaration keyword");
	}

	let name = context.slice();
	context.go_to_next_token();

	if context.token.kind != TokenKind::Equal {
		panic!("Expected `=` after declaration name");
	}

	context.go_to_next_token();

	if matches!(kind, TypeDeclarationKind::Namespace) {
		context.enter_scope();
	}
	let expression = parse_expression(
		context,
		Priority::PrefixKeyword,
		matches!(kind, TypeDeclarationKind::Namespace),
		stop_at,
	);
	if matches!(kind, TypeDeclarationKind::Namespace) {
		context.exit_scope();
	}
	let end = tree.span(expression).end;

	let node = match kind {
		TypeDeclarationKind::Type => Node::Type {
			name,
			expression,
			resolved_type: None,
		},
		TypeDeclarationKind::UnionDeclaration => Node::UnionDeclaration { name, expression },
		TypeDeclarationKind::Enum => Node::Enum { name, expression },
		TypeDeclarationKind::Fields => Node::Fields { name, expression },
		TypeDeclarationKind::Namespace => Node::Namespace { name, expression },
	};

	(node, end)
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
	let type_expression =
		parse_expression(context, Priority::TypeAssignment, false, [TokenKind::Equal]);

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
	parse_expression(context, Priority::PrefixKeyword, false, stop_at)
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

fn remove_external_references(tree: &mut TypedSyntaxTree, name: &str, references: &[usize]) {
	if let Some(existing) = tree.external_symbols.get_mut(name) {
		existing.retain(|entry| !references.contains(entry));
		if existing.is_empty() {
			tree.external_symbols.remove(name);
		}
	}
}
