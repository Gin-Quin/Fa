use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_declaration::{
	parse_declaration_value, parse_generic_parameter_list, parse_optional_type_annotation,
};
use crate::parsing::is_stop_token::is_parenthesis_open;
use crate::tokens::Token;
use crate::parsing::parse_expression::ExpressionContext;
use crate::tokens::TokenKind;

/// Parse a function declaration, starting at the `function` keyword:
/// `function name = <function expression>`, `function name = <Item>(…) => …` or
/// `function name: FunctionType = ...`. `start` is the start of the declaration (the `async`
/// keyword of an asynchronous function).
pub fn parse_function_declaration(
	context: &mut Context<'_>,
	is_async: bool,
	start: usize,
) -> usize {
	context.go_to_next_token(); // skip `function`

	if context.kind() != TokenKind::Identifier {
		let found = context.describe_token();
		context.error_at_token(format!(
			"Expected the name of the function after `function`, found {found}"
		));
		context.skip_to_statement_end();
		let end = context.last_end();
		return context.insert(Node::Invalid, start, end);
	}

	let name_text = context.slice();
	let name = context.name();
	context.go_to_next_token();

	let misplaced = parse_misplaced_generic_parameters(context, &format!("function {name_text}"));

	let type_expression = parse_optional_type_annotation(context);

	let expression = parse_declaration_value(
		context,
		&format!("function {name_text}"),
		ExpressionContext::expression(),
		&[
			TokenKind::Comma,
			TokenKind::ParenthesisClose,
			TokenKind::BracesClose,
			TokenKind::BracketsClose,
		],
	);
	let end = context.span(expression).end;
	attach_generic_parameters(context, expression, misplaced);

	if is_async
		&& !matches!(
			context.at(expression),
			Node::ArrowFunction { .. } | Node::Invalid
		) {
		let span = context.span(expression);
		context.error(
			"An `async` function is declared with its parameters and body, like `async function load = (url: String) => ...`",
			span.start,
			span.end,
		);
	}

	context.insert(
		Node::Function {
			name,
			is_async,
			type_expression,
			expression,
		},
		start,
		end,
	)
}

/// Generic parameters written after the name of a function (`function first<Item> = …`), or
/// after `export function`: reported, since they start the signature of the function. They are
/// returned, to be attached to the lambda (see `attach_generic_parameters`).
pub(crate) fn parse_misplaced_generic_parameters(
	context: &mut Context<'_>,
	declaration: &str,
) -> Vec<usize> {
	let open: Token = context.token();
	if open.kind != TokenKind::GenericOpen && !is_parenthesis_open(open.kind) {
		return Vec::new();
	}
	let parameters = parse_generic_parameter_list(context, false);
	let end = context.last_end();
	let text = context.text(open.start, end);
	let inner = text
		.get(1..text.len().saturating_sub(1))
		.unwrap_or("Item")
		.to_string();
	let message = format!(
		"The generic parameters of a function start its signature: write `{declaration} = <{inner}>(…) => …`"
	);
	context.error(message, open.start, end);
	parameters
}

/// Give the generic parameters written at the wrong place to the lambda of the declaration.
pub(crate) fn attach_generic_parameters(
	context: &mut Context<'_>,
	expression: usize,
	parameters: Vec<usize>,
) {
	if parameters.is_empty() {
		return;
	}
	if let Node::ArrowFunction { signature, .. } = context.tree.at_mutable(expression)
		&& signature.generic_parameters.is_empty()
	{
		signature.generic_parameters = parameters;
	}
}
