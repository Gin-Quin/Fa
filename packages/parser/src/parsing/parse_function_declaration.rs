use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_declaration::{
	parse_declaration_value, parse_optional_generic_parameters, parse_optional_type_annotation,
};
use crate::parsing::parse_expression::ExpressionContext;
use crate::tokens::TokenKind;

/// Parse a function declaration, starting at the `function` keyword:
/// `function name = <function expression>`, `function name(Item) = ...` or
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

	let parameters = parse_optional_generic_parameters(context);

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
			parameters,
			type_expression,
			expression,
		},
		start,
		end,
	)
}
