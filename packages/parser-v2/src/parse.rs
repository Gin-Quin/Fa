use crate::tokens::{ Token, TokenKind };
use crate::tokenize::tokenize;
use crate::nodes::*;

struct Position {
	/// The index of the current token.
	index: usize,
	/// The offset of the current token in the input string.
	offset: usize,
}

fn consume_token<'input>(
	position: &mut Position,
	tokens: &'input [Token]
) -> Option<(&'input Token, usize)> {
	let token = tokens.get(position.index);
	match token {
		Some(token) => {
			let offset = position.offset;
			to_next_token(position, tokens);
			skip_spaces(position, tokens);
			return Some((token, offset));
		}
		None => None,
	}
}

fn to_next_token(position: &mut Position, tokens: &[Token]) {
	if position.index < tokens.len() {
		let token_length = unsafe { tokens.get_unchecked(position.index).length };
		position.offset += token_length as usize;
		position.index += 1;
	}
}

fn skip_spaces(position: &mut Position, tokens: &[Token]) {
	while position.index < tokens.len() {
		let token = unsafe { tokens.get_unchecked(position.index) };
		match token.kind {
			TokenKind::Space => {
				position.offset += token.length as usize;
				position.index += 1;
			}
			_ => {
				break;
			}
		}
	}
}

pub fn parse<'input>(input: &'input str) -> SemanticTree<'input> {
	let tokens = tokenize(input.as_bytes());

	let mut position = Position { index: 0, offset: 0 };

	let mut nodes: SemanticTree<'input> = SemanticTree::with_capacity(tokens.len());

	expression_left(input, &mut position, &tokens, &mut nodes);

	nodes
}

#[test]
fn test_parse() {
	let input = "X + 4 + 12+-4";
	let semantic_tree = parse(input);
	println!("{}", semantic_tree_to_string(&semantic_tree));
}

#[inline]
fn expression_left<'input>(
	input: &'input str,
	position: &mut Position,
	tokens: &[Token],
	nodes: &mut SemanticTree<'input>
) -> usize {
	let (token, offset) = consume_token(position, tokens).unwrap();

	let node: Node<'input> = match token.kind {
		TokenKind::Identifier =>
			Node::Identifier(&input[offset..offset + (token.length as usize)]),
		TokenKind::Integer => {
			Node::Integer(
				input[offset..offset + (token.length as usize)].parse::<i32>().unwrap()
			)
		}
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::Minus => {
			Node::Negate { right: expression_left(input, position, tokens, nodes) }
		}
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let left = nodes.insert(node);
	expression_right(input, position, tokens, nodes, left)
}

fn expression_right<'input>(
	input: &'input str,
	position: &mut Position,
	tokens: &[Token],
	nodes: &mut SemanticTree<'input>,
	left: usize
) -> usize {
	let maybe_token = consume_token(position, tokens);

	let node: Node<'input> = match maybe_token {
		None => {
			return left;
		}
		Some((token, _offset)) =>
			match token.kind {
				TokenKind::Stop => {
					return left;
				}
				TokenKind::Plus =>
					Node::Add {
						left,
						right: expression_left(input, position, tokens, nodes),
					},
				_ => panic!("Unexpected token: {:?}", maybe_token),
			}
	};

	nodes.insert(node)
}
