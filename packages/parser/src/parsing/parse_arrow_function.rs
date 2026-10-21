use crate::{
	context::Context,
	nodes::{FunctionBody, Node},
	parsing::{
		parse_block_body::parse_block_body_with_hoisted,
		parse_expression::{ExpressionContext, parse_expression},
		parse_expression_right::RightExpressionResult,
	},
	priority::Priority,
	source::SourceSpan,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

struct ArrowSignature {
	parameters: Vec<usize>,
	return_type_expression: Option<usize>,
	errors: Option<usize>,
	requirements: Vec<usize>,
}

pub fn parse_arrow_function<const STOP_COUNT: usize>(
	context: &mut Context,
	left: usize,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
) -> RightExpressionResult {
	if priority >= Priority::TypeAssignment {
		return RightExpressionResult::Stop;
	}

	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let signature = resolve_arrow_signature(tree, left);
	let start = tree.span(left).start;

	context.go_to_next_token();

	let (body, end) = if context.token().kind == TokenKind::BracesOpen {
		let (statements, hoisted_symbols) = parse_block_body_with_hoisted(context, "function");
		let end = context.last_token().end;
		(
			FunctionBody::Block {
				statements,
				hoisted_symbols,
			},
			end,
		)
	} else {
		let expression = parse_expression(
			context,
			Priority::None,
			ExpressionContext::new(false, false),
			stop_at,
		);
		let end = tree.span(expression).end;
		(FunctionBody::Expression(expression), end)
	};

	let node = Node::ArrowFunction {
		parameters: signature.parameters,
		return_type_expression: signature.return_type_expression,
		errors: signature.errors,
		requirements: signature.requirements,
		body,
	};

	let span = SourceSpan::new(start, end);
	RightExpressionResult::Yield(tree.insert(node, span))
}

/// Resolve the signature placed before `=>`:
/// `(parameters): ReturnType throws Errors requires name: Type`
fn resolve_arrow_signature(tree: &TypedSyntaxTree, left: usize) -> ArrowSignature {
	match tree.at(left) {
		Node::Requires {
			signature,
			requirements,
		} => {
			let mut resolved = resolve_arrow_signature(tree, *signature);
			if !resolved.requirements.is_empty() {
				panic!("Requirements must be declared with a single `requires` keyword");
			}
			resolved.requirements = requirements.clone();
			resolved
		}
		Node::Throws { signature, errors } => {
			let mut resolved = resolve_arrow_signature(tree, *signature);
			if resolved.errors.is_some() {
				panic!("Errors must be declared with a single `throws` keyword");
			}
			if !resolved.requirements.is_empty() {
				panic!("`throws` must be written before `requires`");
			}
			resolved.errors = Some(*errors);
			resolved
		}
		Node::Assignment {
			name,
			type_expression,
			expression,
		} => {
			if expression.is_some() {
				panic!("Unexpected expression before arrow function body");
			}
			let Some(return_type_expression) = type_expression else {
				panic!("Expected return type expression before arrow function");
			};
			let mut resolved = resolve_arrow_signature(tree, *name);
			if resolved.return_type_expression.is_some()
				|| resolved.errors.is_some()
				|| !resolved.requirements.is_empty()
			{
				panic!("The return type must be written right after the parameters");
			}
			resolved.return_type_expression = Some(*return_type_expression);
			resolved
		}
		Node::Group { expression } => ArrowSignature {
			parameters: match tree.at(*expression) {
				Node::Tuple { items } => items.clone(),
				_ => vec![*expression],
			},
			return_type_expression: None,
			errors: None,
			requirements: Vec::new(),
		},
		Node::EmptyGroup => ArrowSignature {
			parameters: Vec::new(),
			return_type_expression: None,
			errors: None,
			requirements: Vec::new(),
		},
		_ => panic!("Function parameters must be wrapped in parentheses, like `(value) => value`"),
	}
}
