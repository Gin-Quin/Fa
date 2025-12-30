use crate::{
	context::Context,
	nodes::{IfElseBody, Node, StringPart, WhenBranch, WhenBranchPattern, WhenBranchValue},
	parse_arrow_function::parse_arrow_function,
	parse_function_declaration::parse_function_declaration,
	priority::Priority,
	tokenize::tokenize,
	tokens::TokenKind,
	typed_syntax_tree::TypedSyntaxTree,
};

pub enum RightExpressionResult {
	Stop,
	Continue,
	Yield(usize),
}

use RightExpressionResult::*;

/// Parse the left side of an expression.
pub fn parse_expression<const STOP_COUNT: usize>(
	context: &mut Context,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;
	let mut increment_at_the_end = true;

	context.debug("PARSE LEFT");

	macro_rules! Prefix {
		($node_type:ident, $priority:expr) => {{
			context.go_to_next_token();
			increment_at_the_end = false;
			let right = parse_expression(context, $priority, stop_at);
			Node::$node_type { right }
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
				Node::$node_type { expression: None }
			} else {
				let expression = parse_expression(context, $priority, stop_at);
				Node::$node_type {
					expression: Some(expression),
				}
			}
		}};
	}

	let node: Node = match token.kind {
		TokenKind::Identifier => Node::Identifier(context.slice()),
		TokenKind::Integer => Node::Integer(i64::from_str_radix(context.slice(), 10).unwrap()),
		TokenKind::NegativeInteger => {
			Node::Integer(i64::from_str_radix(context.slice(), 10).unwrap())
		}
		TokenKind::BinaryInteger => {
			Node::Integer(i64::from_str_radix(context.slice_at(2), 2).unwrap())
		}
		TokenKind::NegativeBinaryInteger => {
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 2).unwrap())
		}
		TokenKind::OctalInteger => {
			Node::Integer(i64::from_str_radix(context.slice_at(2), 8).unwrap())
		}
		TokenKind::NegativeOctalInteger => {
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 8).unwrap())
		}
		TokenKind::HexadecimalInteger => {
			Node::Integer(i64::from_str_radix(context.slice_at(2), 16).unwrap())
		}
		TokenKind::NegativeHexadecimalInteger => {
			Node::Integer(-i64::from_str_radix(context.slice_at(3), 16).unwrap())
		}
		TokenKind::BigInteger => Node::BigInteger(context.slice()),
		TokenKind::NegativeBigInteger => Node::BigInteger(context.slice()),
		TokenKind::Number => Node::Number(context.slice().parse::<f64>().unwrap()),
		TokenKind::String => {
			increment_at_the_end = false;
			parse_string(context)
		}
		TokenKind::NoneValue => Node::NoneValue,
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::MinusWithoutSpaceAfter => Prefix!(Negate, Priority::Prefix),
		TokenKind::TripleDot => Prefix!(Spread, Priority::Prefix),
		TokenKind::Not => Prefix!(Not, Priority::Not),
		TokenKind::Let => Prefix!(Let, Priority::PrefixKeyword),
		TokenKind::Mutable => Prefix!(Mutable, Priority::PrefixKeyword),
		TokenKind::Type => Prefix!(Type, Priority::PrefixKeyword),
		TokenKind::UnionKeyword => Prefix!(UnionDeclaration, Priority::PrefixKeyword),
		TokenKind::Enum => Prefix!(Enum, Priority::PrefixKeyword),
		TokenKind::Fields => Prefix!(Fields, Priority::PrefixKeyword),
		TokenKind::Reactive => Prefix!(Reactive, Priority::PrefixKeyword),
		TokenKind::Derived => Prefix!(Derived, Priority::PrefixKeyword),
		TokenKind::Namespace => Prefix!(Namespace, Priority::PrefixKeyword),
		TokenKind::Return => PrefixWithOptionalExpression!(Return, Priority::PrefixKeyword),
		TokenKind::Break => PrefixWithOptionalExpression!(Break, Priority::PrefixKeyword),
		TokenKind::Continue => Node::Continue,
		TokenKind::Static => Prefix!(Static, Priority::PrefixKeyword),
		TokenKind::For => {
			increment_at_the_end = false;
			parse_for(context, false)
		}
		TokenKind::AtFor => {
			increment_at_the_end = false;
			parse_for(context, true)
		}
		TokenKind::If => {
			increment_at_the_end = false;
			parse_if(context)
		}
		TokenKind::When => {
			increment_at_the_end = false;
			parse_when(context)
		}
		TokenKind::While => {
			increment_at_the_end = false;
			parse_while(context)
		}
		TokenKind::Loop => {
			increment_at_the_end = false;
			parse_loop(context)
		}
		TokenKind::BracesOpen => {
			increment_at_the_end = false;
			parse_members(context)
		}
		TokenKind::ParenthesisOpen => {
			// group expression or tuple (no function calls, see `parse_expression_right`)
			context.go_to_next_token();

			// check if the next token is a parenthesis close
			if context.token.kind == TokenKind::ParenthesisClose {
				Node::EmptyGroup
			} else {
				Node::Group {
					expression: parse_expression(
						context,
						Priority::None,
						[TokenKind::ParenthesisClose],
					),
				}
			}
		}

		TokenKind::Function => {
			let (node, should_increment) = parse_function_declaration(context);
			increment_at_the_end = should_increment;
			node
		}

		_ => {
			return tree.insert(Node::DanglingToken {
				token: token.clone(),
			});
		}
	};

	let mut left = tree.insert(node);

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

fn parse_members(context: &mut Context) -> Node {
	context.go_to_next_token();
	let mut items: Vec<usize> = Vec::new();

	if context.token.kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after members");
			}

			while context.token.kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token.kind == TokenKind::BracesClose {
					break;
				}
			}

			if context.token.kind == TokenKind::BracesClose {
				break;
			}

			let expression = parse_expression(
				context,
				Priority::None,
				[TokenKind::BracesClose, TokenKind::Comma],
			);
			items.push(expression);

			if context.token.kind == TokenKind::BracesClose {
				break;
			}

			if context.token.kind == TokenKind::Comma || context.token.kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}

	if context.token.kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after members");
	}

	context.go_to_next_token();

	Node::Members { items }
}

fn parse_function_call_parameters(context: &mut Context) -> Vec<usize> {
	context.go_to_next_token();
	let mut parameters: Vec<usize> = Vec::new();
	if context.token.kind != TokenKind::ParenthesisClose {
		loop {
			if context.done() {
				panic!("Missing closing `)`");
			}

			while context.token.kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token.kind == TokenKind::ParenthesisClose {
					break;
				}
			}

			if context.token.kind == TokenKind::ParenthesisClose {
				break;
			}

			let parameter = parse_expression(
				context,
				Priority::None,
				[TokenKind::ParenthesisClose, TokenKind::Comma],
			);
			parameters.push(parameter);

			if context.token.kind == TokenKind::ParenthesisClose {
				break;
			}

			if context.token.kind == TokenKind::Comma || context.token.kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}
	if context.token.kind != TokenKind::ParenthesisClose {
		panic!("Missing closing `)`");
	}
	context.go_to_next_token();
	parameters
}

fn parse_index_expression(context: &mut Context) -> usize {
	context.go_to_next_token();
	let index_expression = parse_expression(context, Priority::None, [TokenKind::BracketsClose]);
	if context.token.kind != TokenKind::BracketsClose {
		panic!("Missing closing `]`");
	}
	context.go_to_next_token();
	index_expression
}

fn parse_string(context: &mut Context) -> Node {
	let raw = context.slice();
	if raw.len() < 2 {
		panic!("Invalid string literal");
	}

	let content = &raw[1..raw.len() - 1];
	let mut parts: Vec<StringPart> = Vec::new();
	let mut literal = String::new();
	let mut has_expression = false;
	let bytes = content.as_bytes();
	let mut index = 0;

	while index < bytes.len() {
		match bytes[index] {
			b'\\' => {
				let (escaped, new_index) = parse_escape_sequence(bytes, index);
				literal.push(escaped);
				index = new_index;
			}
			b'{' => {
				if bytes.get(index + 1) == Some(&b'{') {
					literal.push('{');
					index += 2;
					continue;
				}

				has_expression = true;
				if !literal.is_empty() {
					parts.push(StringPart::Literal(literal));
					literal = String::new();
				}

				let expression_start = index + 1;
				let mut expression_end = expression_start;
				while expression_end < bytes.len() && bytes[expression_end] != b'}' {
					expression_end += 1;
				}

				if expression_end >= bytes.len() {
					panic!("Missing closing `}}` in template string");
				}

				let expression_content = &content[expression_start..expression_end];
				if expression_content.trim().is_empty() {
					panic!("Empty expression in template string");
				}

				let expression = parse_template_expression(context, expression_content);
				parts.push(StringPart::Expression(expression));

				index = expression_end + 1;
			}
			b'}' => {
				if bytes.get(index + 1) == Some(&b'}') {
					literal.push('}');
					index += 2;
				} else {
					literal.push('}');
					index += 1;
				}
			}
			_ => {
				literal.push(bytes[index] as char);
				index += 1;
			}
		}
	}

	context.go_to_next_token();

	if has_expression {
		if !literal.is_empty() {
			parts.push(StringPart::Literal(literal));
		}
		Node::StringTemplate { parts }
	} else {
		Node::StringLiteral(literal)
	}
}

fn parse_escape_sequence(bytes: &[u8], index: usize) -> (char, usize) {
	let Some(&next) = bytes.get(index + 1) else {
		panic!("Unexpected end of escape sequence");
	};

	match next {
		b'\\' => ('\\', index + 2),
		b'"' => ('"', index + 2),
		b'n' => ('\n', index + 2),
		b'r' => ('\r', index + 2),
		b't' => ('\t', index + 2),
		b'0' => ('\0', index + 2),
		b'{' => ('{', index + 2),
		b'}' => ('}', index + 2),
		b'x' => {
			let Some(&first) = bytes.get(index + 2) else {
				panic!("Invalid \\x escape sequence");
			};
			let Some(&second) = bytes.get(index + 3) else {
				panic!("Invalid \\x escape sequence");
			};
			let value = (hex_value(first) << 4) | hex_value(second);
			(
				char::from_u32(value).expect("Invalid \\x escape sequence"),
				index + 4,
			)
		}
		b'u' => {
			if bytes.get(index + 2) != Some(&b'{') {
				panic!("Invalid \\u escape sequence");
			}
			let mut cursor = index + 3;
			let mut value: u32 = 0;
			let mut digits = 0;
			while cursor < bytes.len() && bytes[cursor] != b'}' {
				value = (value << 4) | hex_value(bytes[cursor]);
				cursor += 1;
				digits += 1;
			}
			if cursor >= bytes.len() || bytes[cursor] != b'}' || digits == 0 || digits > 6 {
				panic!("Invalid \\u escape sequence");
			}
			let Some(character) = char::from_u32(value) else {
				panic!("Invalid \\u escape sequence");
			};
			(character, cursor + 1)
		}
		_ => {
			panic!("Unknown escape sequence");
		}
	}
}

fn hex_value(byte: u8) -> u32 {
	match byte {
		b'0'..=b'9' => (byte - b'0') as u32,
		b'a'..=b'f' => (byte - b'a' + 10) as u32,
		b'A'..=b'F' => (byte - b'A' + 10) as u32,
		_ => panic!("Invalid hex digit in escape sequence"),
	}
}

fn parse_template_expression(context: &mut Context, input: &'static str) -> usize {
	let tokens = tokenize(input.as_bytes());
	if tokens.is_empty() {
		panic!("Empty expression in template string");
	}

	let mut sub_context = Context::new(input, unsafe { &mut *context.tree }, &tokens);
	let expression = parse_expression(&mut sub_context, Priority::None, []);

	if !sub_context.done() {
		sub_context.go_to_next_token();
	}

	while !sub_context.done() {
		if sub_context.token.kind == TokenKind::Stop {
			sub_context.go_to_next_token();
		} else {
			panic!("Unexpected token in template string expression");
		}
	}

	expression
}

fn parse_for(context: &mut Context, is_compile_time: bool) -> Node {
	context.go_to_next_token();

	if context.token.kind == TokenKind::BracesOpen {
		panic!("Expected expression after `for`");
	}

	let expression = parse_expression(context, Priority::None, [TokenKind::BracesOpen]);

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after for expression");
	}

	let body = parse_block_body(context, "for");

	Node::For {
		expression,
		body,
		is_compile_time,
	}
}

fn parse_if(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind == TokenKind::BracesOpen {
		panic!("Expected expression after `if`");
	}

	let condition = parse_expression(context, Priority::None, [TokenKind::BracesOpen]);

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after if condition");
	}

	let then_body = parse_block_body(context, "if");
	let mut else_body: Option<IfElseBody> = None;

	if context.token.kind == TokenKind::Else {
		context.go_to_next_token();
		if context.token.kind == TokenKind::If {
			let else_if = parse_if(context);
			let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
			else_body = Some(IfElseBody::If(tree.insert(else_if)));
		} else if context.token.kind == TokenKind::BracesOpen {
			let body = parse_block_body(context, "else");
			else_body = Some(IfElseBody::Block(body));
		} else {
			panic!("Expected `if` or `{{` after `else`");
		}
	}

	Node::If {
		condition,
		then_body,
		else_body,
	}
}

fn parse_when(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind == TokenKind::BracesOpen {
		panic!("Expected expression after `when`");
	}

	let expression = parse_expression(context, Priority::None, [TokenKind::Is]);

	if context.token.kind != TokenKind::Is {
		panic!("Expected `is` after when expression");
	}

	context.go_to_next_token();

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after `is` in when expression");
	}

	let branches = parse_when_branches(context);

	Node::When {
		expression,
		branches,
	}
}

fn parse_while(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind == TokenKind::BracesOpen {
		panic!("Expected expression after `while`");
	}

	let expression = parse_expression(context, Priority::None, [TokenKind::BracesOpen]);

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after while expression");
	}

	let body = parse_block_body(context, "while");

	Node::While { expression, body }
}

fn parse_loop(context: &mut Context) -> Node {
	context.go_to_next_token();

	if context.token.kind != TokenKind::BracesOpen {
		panic!("Expected `{{` after `loop`");
	}

	let body = parse_block_body(context, "loop");

	Node::Loop { body }
}

fn parse_block_body(context: &mut Context, label: &str) -> Vec<usize> {
	context.go_to_next_token();
	let mut body: Vec<usize> = Vec::new();

	if context.token.kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after {label} body");
			}

			body.push(parse_expression(
				context,
				Priority::None,
				[TokenKind::BracesClose],
			));

			if context.token.kind == TokenKind::BracesClose {
				break;
			}

			context.go_to_next_token();
		}
	}

	if context.token.kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after {label} body");
	}

	context.go_to_next_token();
	body
}

fn parse_when_branches(context: &mut Context) -> Vec<WhenBranch> {
	context.go_to_next_token();
	let mut branches: Vec<WhenBranch> = Vec::new();

	if context.token.kind != TokenKind::BracesClose {
		loop {
			if context.done() {
				panic!("Missing closing `}}` after when branches");
			}

			while context.token.kind == TokenKind::Stop {
				context.go_to_next_token();
				if context.token.kind == TokenKind::BracesClose {
					break;
				}
			}

			if context.token.kind == TokenKind::BracesClose {
				break;
			}

			let pattern = if context.token.kind == TokenKind::Else {
				context.go_to_next_token();
				WhenBranchPattern::Else
			} else {
				let expression = parse_expression(context, Priority::None, [TokenKind::FatArrow]);
				WhenBranchPattern::Expression(expression)
			};

			if context.token.kind != TokenKind::FatArrow {
				panic!("Expected `=>` after when branch expression");
			}

			context.go_to_next_token();

			let value = if context.token.kind == TokenKind::BracesOpen {
				let body = parse_block_body(context, "when");
				WhenBranchValue::Block(body)
			} else {
				let expression = parse_expression(
					context,
					Priority::None,
					[TokenKind::Stop, TokenKind::BracesClose],
				);
				WhenBranchValue::Expression(expression)
			};

			branches.push(WhenBranch { pattern, value });

			if context.token.kind == TokenKind::BracesClose {
				break;
			}

			if context.token.kind == TokenKind::Stop {
				context.go_to_next_token();
			}
		}
	}

	if context.token.kind != TokenKind::BracesClose {
		panic!("Missing closing `}}` after when branches");
	}

	context.go_to_next_token();
	branches
}

fn is_stop_token<const STOP_COUNT: usize>(
	stop_at: &[TokenKind; STOP_COUNT],
	kind: TokenKind,
) -> bool {
	stop_at.iter().any(|stop| *stop == kind)
}

/// Parse the right side of an expression.
fn parse_expression_right<const STOP_COUNT: usize>(
	context: &mut Context,
	priority: Priority,
	stop_at: [TokenKind; STOP_COUNT],
	left: usize,
) -> RightExpressionResult {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;

	if is_stop_token(&stop_at, token.kind) {
		return Stop;
	}

	if token.kind == TokenKind::FatArrow {
		return parse_arrow_function(context, left, priority, stop_at);
	}

	macro_rules! Stop {
		() => {
			return Stop
		};
	}

	macro_rules! Operation {
		($node_type:ident, $priority:expr) => {
			if priority >= $priority {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, $priority, stop_at);
				Node::$node_type { left, right }
			}
		};
	}

	macro_rules! List {
		($node_type:ident, $elements:ident, $priority:expr) => {
			if priority >= $priority {
				Stop!()
			} else {
				let left_node = unsafe { tree.nodes.get_unchecked_mut(left).clone() };

				match left_node {
					Node::$node_type { mut $elements } => {
						tree.nodes.remove(left);
						context.go_to_next_token();
						let right = parse_expression(context, $priority, stop_at);
						$elements.push(right);
						Node::$node_type { $elements }
					}
					_ => {
						context.go_to_next_token();
						let right = parse_expression(context, $priority, stop_at);
						Node::$node_type {
							$elements: vec![left, right],
						}
					}
				}
			}
		};
	}

	let node: Node = match token.kind {
		TokenKind::Stop => Stop!(),

		// Operators
		TokenKind::Plus => List!(Add, operands, Priority::Additive),
		TokenKind::MinusWithoutSpaceAfter => List!(Subtract, operands, Priority::Additive),
		TokenKind::Star => List!(Multiply, operands, Priority::Multiplicative),
		TokenKind::Slash => List!(Divide, operands, Priority::Multiplicative),
		TokenKind::DoubleSlash => List!(IntegerDivide, operands, Priority::Multiplicative),
		TokenKind::Modulo => List!(Modulo, operands, Priority::Multiplicative),
		TokenKind::DoubleStar => List!(Power, operands, Priority::Exponentiation),
		TokenKind::LessThan => List!(LessThan, operands, Priority::Comparison),
		TokenKind::LessThanOrEqual => List!(LessThanOrEqual, operands, Priority::Comparison),
		TokenKind::GreaterThan => List!(GreaterThan, operands, Priority::Comparison),
		TokenKind::GreaterThanOrEqual => List!(GreaterThanOrEqual, operands, Priority::Comparison),
		TokenKind::DoubleEqual => List!(Equal, operands, Priority::Equality),
		TokenKind::NotEqual => List!(NotEqual, operands, Priority::Equality),
		TokenKind::Is => Operation!(Is, Priority::Equality),
		TokenKind::And => List!(And, operands, Priority::And),
		TokenKind::Or => List!(Or, operands, Priority::Or),
		TokenKind::Union => List!(Union, operands, Priority::Union),
		TokenKind::Intersection => List!(Intersection, operands, Priority::Union),
		TokenKind::Pipe => List!(Pipe, operands, Priority::Pipe),
		TokenKind::Compose => List!(Compose, operands, Priority::Pipe),
		TokenKind::Insert => Operation!(Insert, Priority::Transfer),
		TokenKind::Extract => Operation!(Extract, Priority::Transfer),
		TokenKind::Arrow => Operation!(Relation, Priority::Transfer),
		TokenKind::Dot => List!(Access, operands, Priority::Access),
		TokenKind::QuestionMarkDot => List!(OptionalAccess, operands, Priority::Access),
		TokenKind::Percent => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				context.go_to_next_token();
				Node::Percentage { value: left }
			}
		}
		TokenKind::QuestionMark => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				match context.lookup_next_token_kind() {
					TokenKind::ParenthesisOpen => {
						context.go_to_next_token();
						if context.token.kind != TokenKind::ParenthesisOpen {
							panic!("Expected `(` after `?`");
						}
						let parameters = parse_function_call_parameters(context);
						Node::OptionalFunctionCall {
							function: left,
							parameters,
						}
					}
					TokenKind::BracketsOpen => {
						context.go_to_next_token();
						if context.token.kind != TokenKind::BracketsOpen {
							panic!("Expected `[` after `?`");
						}
						let index = parse_index_expression(context);
						Node::OptionalIndex {
							target: left,
							index,
						}
					}
					_ => {
						context.go_to_next_token();
						Node::Optional { value: left }
					}
				}
			}
		}
		TokenKind::ExclamationMark => {
			if priority >= Priority::Postfix {
				Stop!()
			} else {
				context.go_to_next_token();
				Node::Assert { value: left }
			}
		}

		TokenKind::Comma => List!(Tuple, items, Priority::Comma),

		// -- function call
		TokenKind::ParenthesisOpen => {
			let parameters = parse_function_call_parameters(context);
			Node::FunctionCall {
				function: left,
				parameters,
			}
		}

		// -- type assignment
		TokenKind::Colon => {
			if priority >= Priority::TypeAssignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, Priority::TypeAssignment, stop_at);

				Node::Assignment {
					name: left,
					type_expression: Some(right),
					expression: None,
				}
			}
		}

		// -- assignment
		TokenKind::Equal => {
			if priority >= Priority::Assignment {
				Stop!()
			} else {
				context.go_to_next_token();
				let right = parse_expression(context, Priority::Assignment, stop_at);
				let mut left_node = unsafe { tree.nodes.get_unchecked_mut(left) };

				match &mut left_node {
					Node::Assignment { expression, .. } => {
						if expression.is_some() {
							panic!("Chaining equal assignments, like `a = b = c`, is forbidden");
						}
						// reusing a type assignment, like `a: int`
						*expression = Some(right);
						return Continue;
					}
					// Node::ParenthesisClose => {
					// 	Node::Assignment {
					// 		name: Err(left),
					// 		type_expression: Some(right),
					// 		expression: None,
					// 	}
					// }
					_ => Node::Assignment {
						name: left,
						type_expression: None,
						expression: Some(right),
					},
				}
			}
		} // for assignment expressions; should not be used

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	Yield(tree.insert(node))
}
