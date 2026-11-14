//! Declarations: `let`, `mutable`, `use`, `static`, `reactive`, `derived`, `reuse`, `type`,
//! `namespace`, their destructuring forms, and the generic parameters of types and functions.

use crate::context::Context;
use crate::nodes::{DeclarationKind, Node};
use crate::parsing::is_stop_token::{can_start_expression, is_parenthesis_open, is_stop_token};
use crate::parsing::parse_delimited::parse_delimited;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::parsing::parse_pattern::{is_pattern_start, parse_pattern};
use crate::priority::Priority;
use crate::tokens::TokenKind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariableDeclarationKind {
	Let,
	Static,
	Mutable,
	Use,
	MutableUse,
	Reactive,
	Derived,
}

impl VariableDeclarationKind {
	fn keyword(self) -> &'static str {
		match self {
			VariableDeclarationKind::Let => "let",
			VariableDeclarationKind::Static => "static",
			VariableDeclarationKind::Mutable => "mutable",
			VariableDeclarationKind::Use => "use",
			VariableDeclarationKind::MutableUse => "mutable use",
			VariableDeclarationKind::Reactive => "reactive",
			VariableDeclarationKind::Derived => "derived",
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeDeclarationKind {
	Type,
	Namespace,
}

/// Tokens that end a type annotation, like `Integer` in `let value: Integer = 12`.
pub(crate) const TYPE_ANNOTATION_STOP_TOKENS: [TokenKind; 5] = [
	TokenKind::Equal,
	TokenKind::Comma,
	TokenKind::ParenthesisClose,
	TokenKind::BracesClose,
	TokenKind::BracketsClose,
];

/// Parse `let`, `mutable`, `use`, `mutable use`, `static`, `reactive` and `derived` declarations,
/// starting at the keyword: `let name: Type = value`, or a destructuring like `let { a } = b`.
pub(crate) fn parse_variable_declaration<'source>(
	context: &mut Context<'source>,
	kind: VariableDeclarationKind,
	stop_at: &[TokenKind],
) -> usize {
	let start = context.token().start;
	let keyword = kind.keyword();
	context.go_to_next_token();

	if is_pattern_start(context.kind()) {
		return parse_destructuring_declaration(context, kind, start, stop_at);
	}

	// `mutable type`, `let function`...: report the keyword, and parse the declaration
	if matches!(
		context.kind(),
		TokenKind::Type | TokenKind::Namespace | TokenKind::Function | TokenKind::Async
	) {
		let what = match context.kind() {
			TokenKind::Type => "a type",
			TokenKind::Namespace => "a namespace",
			_ => "a function",
		};
		let declaration = context.describe_token();
		let declaration_start = context.token().start;
		context.error(
			format!(
				"`{keyword}` can't declare {what}: remove `{keyword}` and write {declaration} alone"
			),
			start,
			declaration_start,
		);
		return parse_expression(
			context,
			Priority::None,
			ExpressionContext::new(true, false),
			stop_at,
		);
	}

	if context.kind() != TokenKind::Identifier {
		let found = context.describe_token();
		let message = format!("Expected a name after `{keyword}`, found {found}");
		if context.token_is_keyword() {
			context.error_at_token_with_code("E-LEX-3", message);
		} else {
			context.error_at_token(message);
		}
		context.skip_to_statement_end();
		let end = context.last_end();
		return context.insert(Node::Invalid, start, end);
	}

	let name_text = context.slice();
	let name = context.name();
	let name_end = context.token().end;
	context.go_to_next_token();

	let type_expression = parse_optional_type_annotation(context);

	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		Some(parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		))
	} else {
		None
	};

	let requires_a_value = matches!(
		kind,
		VariableDeclarationKind::Use
			| VariableDeclarationKind::MutableUse
			| VariableDeclarationKind::Static
	);
	if expression.is_none() && (requires_a_value || type_expression.is_none()) {
		let message = if requires_a_value {
			format!("A `{keyword}` declaration needs a value: write `{keyword} {name_text} = ...`")
		} else {
			format!(
				"Expected `=` or `:` after `{keyword} {name_text}`: give it a value (`= ...`) or a type (`: Type`)"
			)
		};
		context.error_at_token(message);
	}

	// `let f = (x) => …` parses: the checker reports it (E-DECL-6)
	let end = expression
		.or(type_expression)
		.map_or(name_end, |index| context.span(index).end);
	let node = match kind {
		VariableDeclarationKind::Let => Node::Let {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Mutable => Node::Mutable {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Reactive => Node::Reactive {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Derived => Node::Derived {
			name,
			type_expression,
			expression,
		},
		VariableDeclarationKind::Use
		| VariableDeclarationKind::MutableUse
		| VariableDeclarationKind::Static => {
			let expression = match expression {
				Some(expression) => expression,
				None => context.insert_missing(),
			};
			if kind == VariableDeclarationKind::Static {
				Node::Static {
					name,
					type_expression,
					expression,
				}
			} else {
				Node::Use {
					name,
					type_expression,
					expression,
					is_mutable: kind == VariableDeclarationKind::MutableUse,
				}
			}
		}
	};

	context.insert(node, start, end)
}

/// Parse `reuse name = value`, which moves a `mutable use` reference
pub(crate) fn parse_reuse(context: &mut Context<'_>, stop_at: &[TokenKind]) -> usize {
	let start = context.token().start;
	context.go_to_next_token();

	if context.kind() != TokenKind::Identifier {
		context.error_at_token(
			"Expected the name of a `mutable use` reference after `reuse`, like `reuse best = player`",
		);
		context.skip_to_statement_end();
		let end = context.last_end();
		return context.insert(Node::Invalid, start, end);
	}

	let name_text = context.slice();
	let name = context.name();
	context.go_to_next_token();

	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		)
	} else {
		context.error_at_token(format!(
			"Expected `=` after `reuse {name_text}`, like `reuse {name_text} = other`"
		));
		context.insert_missing()
	};
	let end = context.span(expression).end;

	context.insert(Node::Reuse { name, expression }, start, end)
}

/// Parse a destructuring declaration, like `let { a, b as c } = object`
fn parse_destructuring_declaration(
	context: &mut Context<'_>,
	kind: VariableDeclarationKind,
	start: usize,
	stop_at: &[TokenKind],
) -> usize {
	let keyword = kind.keyword();
	let declaration_kind = match kind {
		VariableDeclarationKind::Let => DeclarationKind::Let,
		VariableDeclarationKind::Mutable => DeclarationKind::Mutable,
		VariableDeclarationKind::Use => DeclarationKind::Use,
		VariableDeclarationKind::MutableUse => DeclarationKind::MutableUse,
		VariableDeclarationKind::Static
		| VariableDeclarationKind::Reactive
		| VariableDeclarationKind::Derived => {
			context.error_at_token(format!(
				"A `{keyword}` value can't be destructured: declare it with a name, like `{keyword} name = ...`"
			));
			DeclarationKind::Let
		}
	};

	let pattern = parse_pattern(context);

	if context.kind() == TokenKind::Colon {
		context.error_at_token(
			"A destructuring declaration has no type annotation: the type comes from the value",
		);
		parse_optional_type_annotation(context);
	}

	let expression = if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
		parse_expression(
			context,
			Priority::PrefixKeyword,
			ExpressionContext::expression(),
			stop_at,
		)
	} else {
		context.error_at_token(format!(
			"Expected `=` after the pattern: a destructuring declaration needs a value, like `{keyword} {{ a, b }} = object`"
		));
		context.insert_missing()
	};
	let end = context.span(expression).end;

	context.insert(
		Node::Destructuring {
			kind: declaration_kind,
			pattern,
			expression,
		},
		start,
		end,
	)
}

/// Parse `type Name<Parameters> = expression` or `namespace name = expression`.
pub(crate) fn parse_type_declaration(
	context: &mut Context<'_>,
	kind: TypeDeclarationKind,
	stop_at: &[TokenKind],
) -> usize {
	let start = context.token().start;
	let keyword = match kind {
		TypeDeclarationKind::Type => "type",
		TypeDeclarationKind::Namespace => "namespace",
	};
	context.go_to_next_token();

	// the tokenizer only makes `type` and `namespace` keywords when a name follows
	let name_text = context.slice();
	let name = context.name();
	context.go_to_next_token();

	if kind == TypeDeclarationKind::Namespace
		&& (is_parenthesis_open(context.kind()) || context.kind() == TokenKind::GenericOpen)
	{
		context.error_at_token(
			"A namespace has no parameters: only types and functions can be generic",
		);
	}
	let parameters = parse_optional_generic_parameters(context);

	let expression = parse_declaration_value(
		context,
		&format!("{keyword} {name_text}"),
		ExpressionContext::new(kind == TypeDeclarationKind::Namespace, false),
		stop_at,
	);
	let end = context.span(expression).end;

	let node = match kind {
		TypeDeclarationKind::Type => Node::Type {
			name,
			parameters,
			expression,
		},
		TypeDeclarationKind::Namespace => Node::Namespace { name, expression },
	};
	context.insert(node, start, end)
}

/// Parse `= value` after a declared name (`declaration` is how the declaration starts, for
/// diagnostics). A missing `=` is reported, and the value is parsed anyway if there is one.
pub(crate) fn parse_declaration_value(
	context: &mut Context<'_>,
	declaration: &str,
	expression_context: ExpressionContext,
	stop_at: &[TokenKind],
) -> usize {
	if context.kind() == TokenKind::Equal {
		context.go_to_next_token();
	} else {
		let found = context.describe_token();
		context.error_at_token(format!("Expected `=` after `{declaration}`, found {found}"));
		if !can_start_expression(context.kind()) || is_stop_token(stop_at, context.kind()) {
			return context.insert_missing();
		}
	}

	parse_expression(
		context,
		Priority::PrefixKeyword,
		expression_context,
		stop_at,
	)
}

/// Parse generic parameters between angle brackets, starting at `<`, like `<Item>`,
/// `<Value: Integer | Float>`, `<Index = Integer>` or `<rows: Integer>`. A capitalized parameter
/// is a type parameter, a lowercase one is a compile-time value. The old form between
/// parentheses, `(Item)`, is reported and parsed anyway.
pub(crate) fn parse_generic_parameters(context: &mut Context<'_>) -> Vec<usize> {
	parse_generic_parameter_list(context, true)
}

/// `parse_generic_parameters`, reporting the old form between parentheses only with
/// `report_parentheses`.
pub(crate) fn parse_generic_parameter_list(
	context: &mut Context<'_>,
	report_parentheses: bool,
) -> Vec<usize> {
	let open = context.token();
	let closing = if open.kind == TokenKind::GenericOpen {
		TokenKind::GenericClose
	} else {
		TokenKind::ParenthesisClose
	};
	let (parameters, _) = parse_delimited(context, closing, |context| {
		if context.kind() != TokenKind::Identifier {
			let found = context.describe_token();
			context.error_at_token(format!(
					"Expected a generic parameter, like `Item`, `Value: Integer | Float` or `rows: Integer`, found {found}"
				));
		}
		let parameter = parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			&[TokenKind::Comma, closing],
		);
		let name_and_type = match context.at(parameter) {
			Node::Identifier { name } => Some((*name, false)),
			Node::Assignment {
				name,
				type_expression,
				..
			} => match context.at(*name) {
				Node::Identifier { name } => Some((*name, type_expression.is_some())),
				_ => None,
			},
			_ => None,
		};
		let span = context.span(parameter);
		match name_and_type {
			Some((name, has_type)) => {
				let name_text = context.name_text(name);
				let is_type_parameter =
					name_text.starts_with(|character: char| character.is_ascii_uppercase());
				if !is_type_parameter && !has_type {
					context.error_with_code(
							"GN001",
							format!(
								"The compile-time value parameter `{name_text}` needs a type, like `{name_text}: Integer`. Type parameters are capitalized"
							),
							span.start,
							span.end,
						);
				}
			}
			None => context.error(
				"Expected a generic parameter, like `Item`, `Value: Integer | Float` or `rows: Integer`",
				span.start,
				span.end,
			),
		}
		parameter
	});

	if closing == TokenKind::ParenthesisClose {
		if !report_parentheses {
			return parameters;
		}
		let text = context.text(open.start, context.last_end()).to_string();
		let inner = text
			.strip_prefix('(')
			.and_then(|text| text.strip_suffix(')'))
			.unwrap_or("Item");
		context.error(
			format!("Generic parameters are written between angle brackets: write `<{inner}>`"),
			open.start,
			context.last_end(),
		);
	} else if parameters.is_empty() {
		context.error(
			"Generic parameters can't be empty: remove the `<>`",
			open.start,
			context.last_end(),
		);
	}

	parameters
}

/// Parse the generic parameters written after a declared name if the current token is `<`
/// (or `(`, the old form, which is reported).
pub(crate) fn parse_optional_generic_parameters(context: &mut Context<'_>) -> Vec<usize> {
	if context.kind() == TokenKind::GenericOpen || is_parenthesis_open(context.kind()) {
		parse_generic_parameters(context)
	} else {
		Vec::new()
	}
}

/// Parse `: Type` if the current token is `:`.
pub(crate) fn parse_optional_type_annotation(context: &mut Context<'_>) -> Option<usize> {
	if context.kind() != TokenKind::Colon {
		return None;
	}
	context.go_to_next_token();
	Some(parse_expression(
		context,
		Priority::Assignment,
		ExpressionContext::expression(),
		&TYPE_ANNOTATION_STOP_TOKENS,
	))
}
