use crate::nodes::*;
use crate::tokenize::tokenize;
use crate::tokens::{ Token, TokenKind };

#[derive(Debug, Clone, Copy)]
struct Position {
	token: Token,
	index: usize,
	offset: usize,
}

struct Context<'input> {
	tree: *mut SemanticTree<'input>,
	input: *const str,
	tokens: *const [Token],
}

fn start_position(tokens: &[Token]) -> Option<Position> {
	let mut index = 0;
	let mut offset = 0;

	while index < tokens.len() {
		let token = unsafe { tokens.get_unchecked(index).clone() };
		if token.kind != TokenKind::Space {
			return Some(Position {
				token,
				index,
				offset,
			});
		}
		index += 1;
		offset += token.length as usize;
	}

	None
}

/// Return the current token and the position of the next token.
fn next_position(
	tokens: &[Token],
	Position { token, index, offset }: &Position
) -> Option<Position> {
	let mut next_index = index + 1;
	let mut next_offset = offset + (token.length as usize);

	// first, skip all spaces
	while next_index < tokens.len() {
		let next_token = unsafe { tokens.get_unchecked(next_index).clone() };
		if next_token.kind != TokenKind::Space {
			return Some(Position {
				token: next_token,
				index: next_index,
				offset: next_offset,
			});
		}
		next_index += 1;
		next_offset += token.length as usize;
	}

	None
}

pub fn parse_expression<'input>(input: &'input str) -> SemanticTree<'input> {
	let tokens = tokenize(input.as_bytes());

	// let length = tokens.len();

	let mut tree = SemanticTree::new();

	let context = Context { tree: &mut tree, input, tokens: &*tokens };

	if let Some(position) = start_position(&tokens) {
		expression_left(&context, &position, Priority::None);
	}

	tree
}

#[test]
fn test_parse_expression() {
	let input = "a * b + c";
	let semantic_tree = parse_expression(input);
	println!("{}", semantic_tree_to_string(&semantic_tree));
}

#[inline]
fn expression_left<'input>(
	context: &Context<'input>,
	position: &Position,
	priority: Priority
) -> usize {
	let tree: &mut SemanticTree<'input> = unsafe { &mut *context.tree };
	let input: &str = unsafe { &*context.input };
	let tokens: &[Token] = unsafe { &*context.tokens };
	let Position { token, index: _, offset } = *position;

	println!(
		"PARSE LEFT: <{:?}> '{}'",
		token.kind,
		&input[position.offset..position.offset + (token.length as usize)]
	);

	let node: Node<'input> = match token.kind {
		TokenKind::Identifier =>
			Node::Identifier(&input[offset..offset + (token.length as usize)]),
		TokenKind::Integer =>
			Node::Integer(
				input[offset..offset + (token.length as usize)].parse::<i32>().unwrap()
			),
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::Minus =>
			Node::Negate {
				right: expression_left(
					context,
					&next_position(tokens, position).unwrap(),
					priority
				),
			},
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let mut left = tree.insert(node);

	let mut new_position = next_position(tokens, &position);
	while let Some(position) = new_position {
		let new_left = expression_right(context, &position, priority, left);
		if new_left == left {
			break;
		}
		left = new_left;
		new_position = next_position(tokens, &position);
	}

	left
}

fn expression_right<'input>(
	context: &Context<'input>,
	position: &Position,
	priority: Priority,
	left: usize
) -> usize {
	let input: &str = unsafe { &*context.input };
	let tree: &mut SemanticTree<'input> = unsafe { &mut *context.tree };
	let tokens: &[Token] = unsafe { &*context.tokens };
	let Position { token, index: _, offset: _ } = position;

	println!(
		"PARSE RIGHT: <{:?}> '{}'",
		token.kind,
		&input[position.offset..position.offset + (token.length as usize)]
	);

	let node: Node<'input> = match token.kind {
		TokenKind::Stop => {
			return left;
		}
		TokenKind::Plus => {
			if priority > Priority::Additive {
				return left;
			} else {
				Node::Add {
					left,
					right: expression_left(
						context,
						&next_position(tokens, &position).unwrap(),
						Priority::Additive
					),
				}
			}
		}
		TokenKind::Minus => {
			if priority > Priority::Additive {
				return left;
			} else {
				Node::Subtract {
					left,
					right: expression_left(
						context,
						&next_position(tokens, &position).unwrap(),
						Priority::Additive
					),
				}
			}
		}
		TokenKind::Star => {
			if priority > Priority::Multiplicative {
				return left;
			} else {
				Node::Multiply {
					left,
					right: expression_left(
						context,
						&next_position(tokens, &position).unwrap(),
						Priority::Multiplicative
					),
				}
			}
		}
		_ => {
			panic!(
				"Unexpected token <{:?}> '{}'",
				token.kind,
				&input[position.offset..position.offset + (token.length as usize)]
			);
			return left;
		}
	};

	let new_left: usize = tree.insert(node);
	println!("new_left: {}", node_to_string(tree, new_left));

	new_left
}
