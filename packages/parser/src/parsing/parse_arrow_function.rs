use crate::context::Context;
use crate::nodes::{EffectKind, FunctionSignature, Node};
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_expression_right::RightExpressionResult;
use crate::priority::Priority;
use crate::tokens::TokenKind;

const CLAUSE_ORDER: &str =
	"in the order `: ReturnType`, `throws`, `requires`, `mutates`, `invalidates`";

fn from_parameters(parameters: Vec<usize>) -> FunctionSignature {
	FunctionSignature {
		parameters,
		..Default::default()
	}
}

/// The rank of the last clause of the signature, to check their order:
/// `: Return throws … requires … mutates … invalidates …`
fn last_clause_rank(signature: &FunctionSignature) -> u8 {
	if !signature.invalidations.is_empty() {
		4
	} else if !signature.mutations.is_empty() {
		3
	} else if !signature.requirements.is_empty() {
		2
	} else if signature.errors.is_some() {
		1
	} else {
		0
	}
}

/// Parse a lambda or a function type, from its `=>`: the signature is `left`.
pub fn parse_arrow_function(
	context: &mut Context<'_>,
	left: usize,
	priority: Priority,
	stop_at: &[TokenKind],
) -> RightExpressionResult {
	if priority >= Priority::TypeAssignment {
		return RightExpressionResult::Stop;
	}

	let signature = resolve_arrow_signature(context, left);
	let start = context.span(left).start;

	context.go_to_next_token();

	let body = if context.kind() == TokenKind::BracesOpen {
		parse_block(context, "function")
	} else {
		parse_expression(
			context,
			Priority::None,
			ExpressionContext::expression(),
			stop_at,
		)
	};
	let end = context.last_end();

	let node = Node::ArrowFunction {
		signature: Box::new(signature),
		body,
	};

	RightExpressionResult::Yield(context.insert(node, start, end))
}

/// Resolve the signature placed before `=>`:
/// `(parameters): ReturnType throws Errors requires name: Type mutates path invalidates path`
fn resolve_arrow_signature(context: &mut Context<'_>, left: usize) -> FunctionSignature {
	let span = context.span(left);
	match context.at(left).clone() {
		Node::Effect {
			signature,
			kind,
			paths,
		} => {
			let mut resolved = resolve_arrow_signature(context, signature);
			let rank = match kind {
				EffectKind::Mutates => 3,
				EffectKind::Invalidates => 4,
			};
			if last_clause_rank(&resolved) >= rank {
				context.error(
					format!("`{}` must be written once, {CLAUSE_ORDER}", kind.keyword()),
					span.start,
					span.end,
				);
			}
			match kind {
				EffectKind::Mutates => resolved.mutations.extend(paths),
				EffectKind::Invalidates => resolved.invalidations.extend(paths),
			}
			resolved
		}
		Node::Requires {
			signature,
			requirements,
		} => {
			let mut resolved = resolve_arrow_signature(context, signature);
			if last_clause_rank(&resolved) >= 2 {
				context.error(
					format!("`requires` must be written once, {CLAUSE_ORDER}"),
					span.start,
					span.end,
				);
			}
			resolved.requirements.extend(requirements);
			resolved
		}
		Node::Throws { signature, errors } => {
			let mut resolved = resolve_arrow_signature(context, signature);
			if last_clause_rank(&resolved) >= 1 {
				context.error(
					format!("`throws` must be written once, {CLAUSE_ORDER}"),
					span.start,
					span.end,
				);
			}
			resolved.errors = Some(errors);
			resolved
		}
		Node::Assignment {
			name,
			type_expression,
			expression,
		} => {
			let mut resolved = resolve_arrow_signature(context, name);
			if expression.is_some() {
				context.error(
					"Unexpected `=` in the signature of a function: a lambda is written `(parameters) => body`",
					span.start,
					span.end,
				);
			}
			if resolved.return_type_expression.is_some() || last_clause_rank(&resolved) > 0 {
				context.error(
					format!(
						"The return type must be written right after the parameters, {CLAUSE_ORDER}"
					),
					span.start,
					span.end,
				);
			}
			resolved.return_type_expression = type_expression;
			resolved
		}
		Node::Parameters { items } => from_parameters(items),
		Node::Group { expression } => from_parameters(match context.at(expression) {
			Node::Tuple { items } => items.clone(),
			_ => vec![expression],
		}),
		Node::EmptyGroup => from_parameters(Vec::new()),
		Node::Invalid => FunctionSignature::default(),
		_ => {
			let text = context.text(span.start, span.end);
			context.error(
				format!(
					"The parameters of a function are written between parentheses: write `({text}) => ...`"
				),
				span.start,
				span.end,
			);
			from_parameters(vec![left])
		}
	}
}
