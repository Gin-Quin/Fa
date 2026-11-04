use crate::context::Context;
use crate::nodes::{ComparisonOperator, CompoundOperator, EffectKind, Name, Node};
use crate::parsing::is_stop_token::{can_start_expression, is_parenthesis_open, is_stop_token};
use crate::parsing::parse_arrow_function::parse_arrow_function;
use crate::parsing::parse_block_body::parse_block;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression, parse_range_end};
use crate::parsing::parse_function_call_parameters::parse_function_call_parameters;
use crate::parsing::parse_generic_list::{
	is_spaced_generic_open, parse_generic_arguments, report_spaced_generic_open,
};
use crate::parsing::parse_index_expression::parse_index_expression;
use crate::parsing::parse_members::parse_member_items;
use crate::priority::Priority;
use crate::tokens::TokenKind;
use crate::typed_syntax_tree::TypedSyntaxTree;

pub enum RightExpressionResult {
	// the token doesn't continue the expression
	Stop,
	// the expression was extended in place
	Continue,
	// a new node, that contains the previous expression
	Yield(usize),
}

use RightExpressionResult::*;

/// The tokens that end a requirement or an effect path in a signature.
const SIGNATURE_CLAUSE_STOP_TOKENS: [TokenKind; 10] = [
	TokenKind::Comma,
	TokenKind::FatArrow,
	TokenKind::Throws,
	TokenKind::Requires,
	TokenKind::Mutates,
	TokenKind::Invalidates,
	TokenKind::Equal,
	TokenKind::ParenthesisClose,
	TokenKind::BracesClose,
	TokenKind::BracketsClose,
];

/// Parse the right side of an expression: the operator at the current token (if it binds
/// tighter than `priority`) and its right operand.
pub(crate) fn parse_expression_right(
	context: &mut Context<'_>,
	priority: Priority,
	stop_at: &[TokenKind],
	left: usize,
) -> RightExpressionResult {
	let token = context.token();
	let token_kind = token.kind;

	if is_stop_token(stop_at, token_kind) {
		return Stop;
	}

	macro_rules! Stop {
		() => {
			return Stop
		};
	}

	// a left-associative operator whose chains are stored flat, like `a + b + c`
	macro_rules! List {
		($variant:ident, $field:ident, $operator_priority:expr) => {{
			if priority >= $operator_priority {
				Stop!()
			}
			context.go_to_next_token();
			let right = parse_operand(context, $operator_priority, stop_at);
			let right_end = context.span(right).end;
			if let Node::$variant { $field } = context.tree.at_mutable(left) {
				$field.push(right);
				context.tree.spans[left].end = right_end;
				return Continue;
			}
			let start = context.span(left).start;
			context.insert(
				Node::$variant {
					$field: vec![left, right],
				},
				start,
				right_end,
			)
		}};
	}

	// a binary operator, left-associative unless the right operand is parsed with a lower
	// priority than the operator's
	macro_rules! Binary {
		($variant:ident, $operator_priority:expr, $right_priority:expr) => {{
			if priority >= $operator_priority {
				Stop!()
			}
			context.go_to_next_token();
			let right = parse_operand(context, $right_priority, stop_at);
			let start = context.span(left).start;
			let end = context.span(right).end;
			context.insert(
				Node::$variant {
					operands: vec![left, right],
				},
				start,
				end,
			)
		}};
	}

	let node = match token_kind {
		TokenKind::FatArrow => return parse_arrow_function(context, left, priority, stop_at),

		TokenKind::Plus => List!(Add, operands, Priority::Additive),
		TokenKind::Minus => List!(Subtract, operands, Priority::Additive),
		TokenKind::Star => List!(Multiply, operands, Priority::Multiplicative),
		TokenKind::Slash => List!(Divide, operands, Priority::Multiplicative),
		TokenKind::DoubleSlash => List!(IntegerDivide, operands, Priority::Multiplicative),
		TokenKind::DoublePercent => List!(Remainder, operands, Priority::Multiplicative),
		TokenKind::DoubleQuestionMark => {
			let next = context.lookup_next_token_kind();
			if can_start_expression(next) && !is_stop_token(stop_at, next) {
				List!(Fallback, operands, Priority::Fallback)
			} else {
				// a type made optional twice, like `Integer??` (which is `Integer?`)
				if priority >= Priority::Postfix {
					Stop!()
				}
				context.go_to_next_token();
				let start = context.span(left).start;
				let inner = context.insert(Node::Optional { value: left }, start, token.end - 1);
				context.insert(Node::Optional { value: inner }, start, token.end)
			}
		}
		// right-associative: `2 ** 3 ** 2` is `2 ** (3 ** 2)`
		TokenKind::DoubleStar => Binary!(Power, Priority::Exponentiation, Priority::Multiplicative),
		TokenKind::DoubleEqual | TokenKind::NotEqual => {
			if priority < Priority::Equality
				&& matches!(context.at(left), Node::Equal { .. } | Node::NotEqual { .. })
			{
				let start = context.span(left).start;
				context.error_with_code("E-CMP-3", 
					format!(
						"{} can't be chained: compare two values at a time, like `a == b and b == c`, or add parentheses",
						token_kind.describe()
					),
					start,
					token.end,
				);
			}
			if token_kind == TokenKind::DoubleEqual {
				Binary!(Equal, Priority::Equality, Priority::Equality)
			} else {
				Binary!(NotEqual, Priority::Equality, Priority::Equality)
			}
		}
		// a generic application, like `Array<Integer>`, `first<Integer>(items)` or
		// `Pair<String, Integer> { … }`
		TokenKind::GenericOpen => parse_generic_application(context, left, TokenKind::GenericClose),
		// `Array< Integer>`: reported, and read as a generic application
		TokenKind::LessThan if is_spaced_generic_open(context, left) => {
			report_spaced_generic_open(context);
			parse_generic_application(context, left, TokenKind::GreaterThan)
		}
		TokenKind::LessThan
		| TokenKind::LessThanOrEqual
		| TokenKind::GreaterThan
		| TokenKind::GreaterThanOrEqual => {
			if priority >= Priority::Comparison {
				Stop!()
			}
			let operator = match token_kind {
				TokenKind::LessThan => ComparisonOperator::LessThan,
				TokenKind::LessThanOrEqual => ComparisonOperator::LessThanOrEqual,
				TokenKind::GreaterThan => ComparisonOperator::GreaterThan,
				_ => ComparisonOperator::GreaterThanOrEqual,
			};
			context.go_to_next_token();
			let right = parse_operand(context, Priority::Comparison, stop_at);
			let right_end = context.span(right).end;
			// a chained comparison, like `0 <= index < length`
			if let Node::Comparison {
				operands,
				operators,
			} = context.tree.at_mutable(left)
			{
				operands.push(right);
				operators.push(operator);
				context.tree.spans[left].end = right_end;
				return Continue;
			}
			let start = context.span(left).start;
			context.insert(
				Node::Comparison {
					operands: vec![left, right],
					operators: vec![operator],
				},
				start,
				right_end,
			)
		}
		TokenKind::Is => {
			if priority >= Priority::Equality {
				Stop!()
			}
			context.go_to_next_token();
			let negated = context.kind() == TokenKind::Not;
			if negated {
				context.go_to_next_token();
			}
			let is_in = context.kind() == TokenKind::In;
			if is_in {
				context.go_to_next_token();
			}
			let right = parse_operand(context, Priority::Equality, stop_at);
			let start = context.span(left).start;
			let end = context.span(right).end;
			let node = if is_in {
				Node::IsIn {
					left,
					right,
					negated,
				}
			} else {
				Node::Is {
					left,
					right,
					negated,
				}
			};
			context.insert(node, start, end)
		}
		TokenKind::In => {
			if priority >= Priority::Comparison {
				Stop!()
			}
			context.go_to_next_token();
			let right = parse_operand(context, Priority::Comparison, stop_at);
			let start = context.span(left).start;
			let end = context.span(right).end;
			context.insert(Node::In { left, right }, start, end)
		}
		TokenKind::And => List!(And, operands, Priority::And),
		TokenKind::Or => List!(Or, operands, Priority::Or),
		TokenKind::Union => List!(Union, operands, Priority::Union),
		TokenKind::Then => List!(Compose, operands, Priority::Compose),
		TokenKind::Comma => List!(Tuple, items, Priority::Comma),
		TokenKind::With => {
			if priority >= Priority::Access {
				Stop!()
			}
			context.go_to_next_token();
			let arguments = if is_parenthesis_open(context.kind()) {
				parse_function_call_parameters(context)
			} else {
				context.error_at_token(
					"Expected the bound parameters after `with`, like `with (y = 0)`",
				);
				Vec::new()
			};
			let start = context.span(left).start;
			let end = context.last_end();
			context.insert(
				Node::PartialApplication {
					function: left,
					arguments,
				},
				start,
				end,
			)
		}
		TokenKind::Catch => {
			if priority >= Priority::Catch {
				Stop!()
			}
			parse_catch(context, left, stop_at)
		}
		TokenKind::Throws => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			}
			context.go_to_next_token();
			let errors = parse_operand(context, Priority::TypeAssignment, stop_at);
			let start = context.span(left).start;
			let end = context.span(errors).end;
			context.insert(
				Node::Throws {
					signature: left,
					errors,
				},
				start,
				end,
			)
		}
		TokenKind::Requires => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			}
			let requirements = parse_signature_clause_items(context, |context, item| {
				let is_requirement = matches!(
					context.at(item),
					Node::Assignment {
						type_expression: Some(_),
						expression: None,
						..
					}
				);
				if !is_requirement {
					let span = context.span(item);
					context.error(
						"Expected a requirement like `name: Type` after `requires`, like `requires database: Database`",
						span.start,
						span.end,
					);
				}
			});
			let start = context.span(left).start;
			let end = context.last_end();
			context.insert(
				Node::Requires {
					signature: left,
					requirements,
				},
				start,
				end,
			)
		}
		TokenKind::Mutates | TokenKind::Invalidates => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			}
			let kind = if token_kind == TokenKind::Mutates {
				EffectKind::Mutates
			} else {
				EffectKind::Invalidates
			};
			let paths = parse_signature_clause_items(context, |context, path| {
				if !is_effect_path(&context.tree, path) {
					let span = context.span(path);
					context.error(
						format!(
							"Expected a parameter, a field or an index after `{}`, like `self`, `self.balance`, `self[index]`, `self[index..]` or `players[..].score`",
							kind.keyword()
						),
						span.start,
						span.end,
					);
				}
			});
			let start = context.span(left).start;
			let end = context.last_end();
			context.insert(
				Node::Effect {
					signature: left,
					kind,
					paths,
				},
				start,
				end,
			)
		}
		TokenKind::DoubleDot | TokenKind::DoubleDotEqual => {
			if priority >= Priority::Range {
				Stop!()
			}
			let inclusive = token_kind == TokenKind::DoubleDotEqual;
			context.go_to_next_token();
			let end = parse_range_end(context, inclusive, stop_at);
			let start = context.span(left).start;
			let end_position = end.map_or(token.end, |end| context.span(end).end);
			context.insert(
				Node::Range {
					start: Some(left),
					end,
					inclusive,
				},
				start,
				end_position,
			)
		}
		TokenKind::Arrow => {
			if priority >= Priority::Relation {
				Stop!()
			}
			context.go_to_next_token();
			let right = parse_operand(context, Priority::Relation, stop_at);
			let start = context.span(left).start;
			let end = context.span(right).end;
			context.insert(Node::Relation { left, right }, start, end)
		}
		TokenKind::Dot => return parse_access(context, priority, left, false),
		TokenKind::QuestionMarkDot => return parse_access(context, priority, left, true),
		TokenKind::Percent | TokenKind::QuestionMark => {
			if priority >= Priority::Postfix {
				Stop!()
			}
			if token_kind == TokenKind::Percent
				&& !matches!(
					context.at(left),
					Node::Integer(_)
						| Node::Number(_) | Node::BigInteger(_)
						| Node::Group { .. }
						| Node::Invalid
				) {
				let span = context.span(left);
				let text = context.text(span.start, span.end);
				context.error_with_code(
					"E-NUM-5",
					format!(
						"`%` only applies to a number literal or to an expression in parentheses: write `({text})%`"
					),
					span.start,
					token.end,
				);
			}
			context.go_to_next_token();
			let start = context.span(left).start;
			let node = if token_kind == TokenKind::Percent {
				Node::Percentage { value: left }
			} else {
				Node::Optional { value: left }
			};
			context.insert(node, start, token.end)
		}

		// a call (the parenthesis can be marked as a function declaration when the call is
		// followed by `=>`, like in the return type `(): Array(Integer) => ...`)
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration => {
			let parameters = parse_function_call_parameters(context);
			let start = context.span(left).start;
			let end = context.last_end();
			context.insert(
				Node::FunctionCall {
					function: left,
					parameters,
				},
				start,
				end,
			)
		}
		// an object of a named type, like `Point { x = 1 }`
		TokenKind::BracesOpen => {
			let (members, end) = parse_member_items(context);
			let start = context.span(left).start;
			context.insert(
				Node::MemberIndex {
					target: left,
					members,
				},
				start,
				end,
			)
		}
		TokenKind::BracketsOpen => {
			let index = parse_index_expression(context);
			let start = context.span(left).start;
			let end = context.last_end();
			context.insert(
				Node::Index {
					target: left,
					index,
				},
				start,
				end,
			)
		}

		// a type annotation, or the return type of a function
		TokenKind::Colon => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			}
			context.go_to_next_token();
			// After parameters, like `(a: Integer): Integer`, the type is a return type and
			// stops at `=>`. Otherwise, it's a type annotation that can be a function type.
			let is_return_type = matches!(
				context.at(left),
				Node::Parameters { .. } | Node::Group { .. } | Node::EmptyGroup
			);
			let type_priority = if is_return_type {
				Priority::TypeAssignment
			} else {
				Priority::Assignment
			};
			let right = parse_operand(context, type_priority, stop_at);
			let start = context.span(left).start;
			let end = context.span(right).end;
			context.insert(
				Node::Assignment {
					name: left,
					type_expression: Some(right),
					expression: None,
				},
				start,
				end,
			)
		}

		TokenKind::PlusEqual
		| TokenKind::MinusEqual
		| TokenKind::StarEqual
		| TokenKind::DoubleStarEqual
		| TokenKind::SlashEqual
		| TokenKind::DoubleSlashEqual
		| TokenKind::DoublePercentEqual
		| TokenKind::DotEqual => {
			if priority >= Priority::Assignment {
				Stop!()
			}
			if matches!(
				context.at(left),
				Node::CompoundAssignment { .. }
					| Node::Assignment {
						expression: Some(_),
						..
					}
			) {
				context.error_at_token_with_code(
					"E-DECL-8",
					"Assignments can't be chained, like `a += b += c`: write one assignment per line",
				);
			}
			let operator = match token_kind {
				TokenKind::PlusEqual => CompoundOperator::Add,
				TokenKind::MinusEqual => CompoundOperator::Subtract,
				TokenKind::StarEqual => CompoundOperator::Multiply,
				TokenKind::DoubleStarEqual => CompoundOperator::Power,
				TokenKind::SlashEqual => CompoundOperator::Divide,
				TokenKind::DoubleSlashEqual => CompoundOperator::IntegerDivide,
				TokenKind::DoublePercentEqual => CompoundOperator::Remainder,
				_ => CompoundOperator::Update,
			};
			context.go_to_next_token();
			let expression = parse_operand(context, Priority::Assignment, stop_at);
			if operator == CompoundOperator::Update
				&& !matches!(context.at(expression), Node::Members { .. } | Node::Invalid)
			{
				let span = context.span(expression);
				context.error_with_code(
					"E-OBJ-13",
					"`.=` expects an object literal, like `settings .= { port = 8080 }`",
					span.start,
					span.end,
				);
			}
			let start = context.span(left).start;
			let end = context.span(expression).end;
			context.insert(
				Node::CompoundAssignment {
					operator,
					target: left,
					expression,
				},
				start,
				end,
			)
		}

		TokenKind::Equal => {
			if priority >= Priority::Assignment {
				Stop!()
			}
			context.go_to_next_token();
			let right = parse_operand(context, Priority::Assignment, stop_at);
			let right_end = context.span(right).end;

			if let Node::Assignment { expression, .. } = context.tree.at_mutable(left) {
				if expression.is_none() {
					// the value of a typed name, like `a: Integer = 1`
					*expression = Some(right);
					context.tree.spans[left].end = right_end;
				} else {
					let span = context.span(right);
					context.error_with_code(
						"E-DECL-8",
						"Assignments can't be chained, like `a = b = c`: write one assignment per line",
						span.start,
						span.end,
					);
				}
				return Continue;
			}

			let start = context.span(left).start;
			context.insert(
				Node::Assignment {
					name: left,
					type_expression: None,
					expression: Some(right),
				},
				start,
				right_end,
			)
		}

		// not an operator: the caller decides if the token is expected
		_ => Stop!(),
	};

	Yield(node)
}

/// `target<arguments>`, from the `<`: `closing` is `GenericClose`, or `GreaterThan` after a `<`
/// followed by a space (see `is_spaced_generic_open`).
fn parse_generic_application(context: &mut Context<'_>, left: usize, closing: TokenKind) -> usize {
	let (arguments, end) = parse_generic_arguments(context, closing);
	let start = context.span(left).start;
	context.insert(
		Node::GenericApplication {
			target: left,
			arguments,
		},
		start,
		end,
	)
}

/// Parse the right operand of an operator.
fn parse_operand(context: &mut Context<'_>, priority: Priority, stop_at: &[TokenKind]) -> usize {
	parse_expression(context, priority, ExpressionContext::expression(), stop_at)
}

/// `expression catch error => handler`, the handler being a block or an expression that
/// extends as far as possible, like a lambda body.
fn parse_catch(context: &mut Context<'_>, left: usize, stop_at: &[TokenKind]) -> usize {
	let catch_token = context.token();
	context.go_to_next_token();

	if matches!(context.at(left), Node::Try { .. }) {
		let span = context.span(left);
		context.error_with_code("E-ERR-6", 
			"`try` and `catch` can't be used on the same expression: `try` passes the error to the caller, `catch` handles it. Remove one of them",
			span.start,
			catch_token.end,
		);
	}

	let error_name = if context.kind() == TokenKind::Identifier {
		let name = context.name();
		context.go_to_next_token();
		name
	} else {
		context.error_at_token(
			"Expected the name of the error after `catch`, like `catch error => ...`",
		);
		Name::new(context.last_end(), context.last_end())
	};
	let error_text = context.name_text(error_name);

	if context.kind() == TokenKind::FatArrow {
		context.go_to_next_token();
	} else {
		context.error_at_token(format!(
			"Expected `=>` after `catch {error_text}`, like `catch {error_text} => fallback`"
		));
	}

	let handler = if context.kind() == TokenKind::BracesOpen {
		parse_block(context, "catch")
	} else {
		parse_operand(context, Priority::None, stop_at)
	};

	let start = context.span(left).start;
	let end = context.last_end();
	context.insert(
		Node::Catch {
			expression: left,
			error_name,
			handler,
		},
		start,
		end,
	)
}

/// Parse the comma-separated items after `requires`, `mutates` or `invalidates`, and check each
/// one with `check`.
fn parse_signature_clause_items<'source>(
	context: &mut Context<'source>,
	mut check: impl FnMut(&mut Context<'source>, usize),
) -> Vec<usize> {
	let mut items = Vec::new();
	loop {
		context.go_to_next_token(); // skip the keyword or the comma
		let item = parse_expression(
			context,
			Priority::Comma,
			ExpressionContext::expression(),
			&SIGNATURE_CLAUSE_STOP_TOKENS,
		);
		check(context, item);
		items.push(item);
		if context.kind() != TokenKind::Comma {
			return items;
		}
	}
}

/// `a.b` or `a?.b`: the name after the dot is added to the access, so `a.b.c` is one access.
fn parse_access(
	context: &mut Context<'_>,
	priority: Priority,
	left: usize,
	optional: bool,
) -> RightExpressionResult {
	if priority >= Priority::Access {
		return Stop;
	}

	context.go_to_next_token();
	let token = context.token();
	// any word is a valid field name after a dot, even a keyword, like `range.start`
	let is_name = token.kind == TokenKind::Identifier
		|| context
			.slice()
			.bytes()
			.next()
			.is_some_and(|byte| byte.is_ascii_alphabetic());

	let right = if is_name && token.kind != TokenKind::End {
		let name = context.name();
		context.go_to_next_token();
		context.insert(Node::Identifier { name }, token.start, token.end)
	} else {
		let (code, message) = match context.at(left) {
			Node::Integer(_) => {
				let span = context.span(left);
				let message = format!(
					"`{}.` is not a number: write `{}.0`",
					context.text(span.start, span.end),
					context.text(span.start, span.end)
				);
				(Some("E-NUM-1"), message)
			}
			_ if token.kind == TokenKind::Integer => {
				let span = context.span(left);
				let message = format!(
					"Tuples are indexed with brackets: write `{}[{}]`",
					context.text(span.start, span.end),
					context.slice()
				);
				(Some("E-TUP-4"), message)
			}
			_ => {
				let message = format!(
					"Expected a field name after `{}`, found {}",
					if optional { "?." } else { "." },
					context.describe_token()
				);
				(None, message)
			}
		};
		match code {
			Some(code) => context.error_at_token_with_code(code, message),
			None => context.error_at_token(message),
		}
		context.insert_missing()
	};
	let right_end = context.span(right).end;

	match context.tree.at_mutable(left) {
		Node::Access { operands } if !optional => {
			operands.push(right);
			context.tree.spans[left].end = right_end;
			return Continue;
		}
		Node::OptionalAccess { operands } if optional => {
			operands.push(right);
			context.tree.spans[left].end = right_end;
			return Continue;
		}
		_ => {}
	}

	let start = context.span(left).start;
	let operands = vec![left, right];
	let node = if optional {
		Node::OptionalAccess { operands }
	} else {
		Node::Access { operands }
	};
	Yield(context.insert(node, start, right_end))
}

/// Return true if the token can start the end bound of a range, like `b` in `a..b`.
pub(crate) fn can_start_range_bound(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::Identifier
			| TokenKind::Integer
			| TokenKind::NegativeInteger
			| TokenKind::BinaryInteger
			| TokenKind::NegativeBinaryInteger
			| TokenKind::OctalInteger
			| TokenKind::NegativeOctalInteger
			| TokenKind::HexadecimalInteger
			| TokenKind::NegativeHexadecimalInteger
			| TokenKind::BigInteger
			| TokenKind::NegativeBigInteger
			| TokenKind::Number
			| TokenKind::String
			| TokenKind::Minus
			| TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
	)
}

/// Return true if the node is a valid path for `mutates` / `invalidates`: a parameter (`self`),
/// a field (`self.balance`), an index (`self[index]`, `self[index..]`), every element
/// (`players[..].score`), or a path through them (`self.#nodes[..].next`).
pub(crate) fn is_effect_path(tree: &TypedSyntaxTree, node: usize) -> bool {
	match tree.at(node) {
		Node::Identifier { .. } | Node::Invalid => true,
		Node::Access { operands } => operands.split_first().is_some_and(|(first, rest)| {
			is_effect_path(tree, *first)
				&& rest
					.iter()
					.all(|operand| matches!(tree.at(*operand), Node::Identifier { .. }))
		}),
		Node::Index { target, index } => {
			is_effect_path(tree, *target) && is_effect_index(tree, *index)
		}
		_ => false,
	}
}

// ex: `index`, `0`, `self.length - 1`, `index..`, `..`
fn is_effect_index(tree: &TypedSyntaxTree, node: usize) -> bool {
	match tree.at(node) {
		Node::Integer(_) => true,
		Node::Identifier { .. } | Node::Access { .. } | Node::Index { .. } => {
			is_effect_path(tree, node)
		}
		Node::Add { operands } | Node::Subtract { operands } => operands
			.iter()
			.all(|operand| is_effect_index(tree, *operand)),
		Node::Range { start, end, .. } => {
			start.is_none_or(|start| is_effect_index(tree, start))
				&& end.is_none_or(|end| is_effect_index(tree, end))
		}
		_ => false,
	}
}
