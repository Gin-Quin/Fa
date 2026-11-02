use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_declaration::{
	parse_declaration_value, parse_optional_generic_parameters, parse_optional_type_annotation,
};
use crate::parsing::parse_expression::ExpressionContext;
use crate::tokens::TokenKind;

/// Parse an export, starting at `export`:
/// `export = value`, `export: Type = value`, `export mutable: Type = value`,
/// `export function(Item) = ...`, `export async function = ...`, `export type(T) = ...`,
/// `export namespace = { ... }`.
pub(crate) fn parse_export(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	let (node, expression) = match context.kind() {
		TokenKind::Function | TokenKind::Async => {
			let is_async = context.kind() == TokenKind::Async;
			if is_async {
				context.go_to_next_token();
				if context.kind() == TokenKind::Function {
					context.go_to_next_token();
				} else {
					context.error_at_token(
						"Expected `function` after `export async`, like `export async function = (): Integer => ...`",
					);
				}
			} else {
				context.go_to_next_token();
			}
			let parameters = parse_optional_generic_parameters(context);
			let type_expression = parse_optional_type_annotation(context);
			let expression = parse_export_value(context, stop_at);
			let node = Node::ExportFunction {
				is_async,
				parameters,
				type_expression,
				expression,
			};
			(node, expression)
		}
		TokenKind::Type => {
			context.go_to_next_token();
			let parameters = parse_optional_generic_parameters(context);
			let expression = parse_export_value(context, stop_at);
			let node = Node::ExportType {
				parameters,
				expression,
			};
			(node, expression)
		}
		TokenKind::Namespace => {
			context.go_to_next_token();
			let expression = parse_export_value(context, stop_at);
			(Node::ExportNamespace { expression }, expression)
		}
		TokenKind::Mutable | TokenKind::Colon | TokenKind::Equal => {
			let is_mutable = context.kind() == TokenKind::Mutable;
			if is_mutable {
				context.go_to_next_token();
			}
			let type_expression = parse_optional_type_annotation(context);
			let expression = parse_export_value(context, stop_at);
			let node = Node::ExportValue {
				is_mutable,
				type_expression,
				expression,
			};
			(node, expression)
		}
		// a reactive value belongs to its module (advanced brief §11.2, `REA005`)
		TokenKind::Reactive | TokenKind::Derived => {
			let keyword = context.slice().to_string();
			context.error_at_token_with_code(
				"REA005",
				format!(
					"A `{keyword}` value can't be exported: export a function that reads it, or a plain value"
				),
			);
			context.skip_to_statement_end();
			let end = context.last_end();
			return context.insert(Node::Invalid, start, end);
		}
		_ => {
			let message = match context.kind() {
				TokenKind::Let
				| TokenKind::Static
				| TokenKind::Use => {
					"A file exports one anonymous value, named after the file: write `export = value`. `static` values can't be exported".to_string()
				}
				TokenKind::Identifier => format!(
					"An export has no name: the file's name is the name of its export. Write `export = {}` or `export function = ...`",
					context.slice()
				),
				_ => format!(
					"Expected `=`, `mutable`, `function`, `type` or `namespace` after `export`, found {}",
					context.describe_token()
				),
			};
			context.error_at_token(message);
			context.skip_to_statement_end();
			let end = context.last_end();
			return context.insert(Node::Invalid, start, end);
		}
	};

	let end = context.span(expression).end;
	context.insert(node, start, end)
}

fn parse_export_value(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let is_namespace = context.last_token().kind == TokenKind::Namespace;
	parse_declaration_value(
		context,
		"export",
		ExpressionContext::new(is_namespace, false),
		stop_at,
	)
}
