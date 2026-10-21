use crate::{
	context::Context,
	nodes::{EffectKind, FunctionBody, Node},
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
	mutations: Vec<usize>,
	invalidations: Vec<usize>,
}

impl ArrowSignature {
	fn from_parameters(parameters: Vec<usize>) -> Self {
		ArrowSignature {
			parameters,
			return_type_expression: None,
			errors: None,
			requirements: Vec::new(),
			mutations: Vec::new(),
			invalidations: Vec::new(),
		}
	}

	/// The rank of the last clause of the signature, to check their order:
	/// `: Return throws … requires … mutates … invalidates …`
	fn last_clause_rank(&self) -> u8 {
		if !self.invalidations.is_empty() {
			4
		} else if !self.mutations.is_empty() {
			3
		} else if !self.requirements.is_empty() {
			2
		} else if self.errors.is_some() {
			1
		} else {
			0
		}
	}
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
		mutations: signature.mutations,
		invalidations: signature.invalidations,
		body,
	};

	let span = SourceSpan::new(start, end);
	RightExpressionResult::Yield(tree.insert(node, span))
}

/// Resolve the signature placed before `=>`:
/// `(parameters): ReturnType throws Errors requires name: Type`
fn resolve_arrow_signature(tree: &TypedSyntaxTree, left: usize) -> ArrowSignature {
	match tree.at(left) {
		Node::Effect {
			signature,
			kind,
			paths,
		} => {
			let mut resolved = resolve_arrow_signature(tree, *signature);
			let rank = match kind {
				EffectKind::Mutates => 3,
				EffectKind::Invalidates => 4,
			};
			if resolved.last_clause_rank() >= rank {
				panic!(
					"`{}` must be declared once, in the order `throws`, `requires`, `mutates`, `invalidates`",
					kind.keyword()
				);
			}
			match kind {
				EffectKind::Mutates => resolved.mutations = paths.clone(),
				EffectKind::Invalidates => resolved.invalidations = paths.clone(),
			}
			resolved
		}
		Node::Requires {
			signature,
			requirements,
		} => {
			let mut resolved = resolve_arrow_signature(tree, *signature);
			if resolved.last_clause_rank() >= 2 {
				panic!(
					"`requires` must be declared once, in the order `throws`, `requires`, `mutates`, `invalidates`"
				);
			}
			resolved.requirements = requirements.clone();
			resolved
		}
		Node::Throws { signature, errors } => {
			let mut resolved = resolve_arrow_signature(tree, *signature);
			if resolved.last_clause_rank() >= 1 {
				panic!(
					"`throws` must be declared once, in the order `throws`, `requires`, `mutates`, `invalidates`"
				);
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
			if resolved.return_type_expression.is_some() || resolved.last_clause_rank() > 0 {
				panic!("The return type must be written right after the parameters");
			}
			resolved.return_type_expression = Some(*return_type_expression);
			resolved
		}
		Node::Parameters { items } => ArrowSignature::from_parameters(items.clone()),
		Node::Group { expression } => ArrowSignature::from_parameters(match tree.at(*expression) {
			Node::Tuple { items } => items.clone(),
			_ => vec![*expression],
		}),
		Node::EmptyGroup => ArrowSignature::from_parameters(Vec::new()),
		_ => panic!("Function parameters must be wrapped in parentheses, like `(value) => value`"),
	}
}
