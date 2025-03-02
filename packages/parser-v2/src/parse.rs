use crate::tokens::{ Token, TokenKind };
use crate::tokenize::tokenize;
use crate::nodes::*;

fn next(index: &mut usize, offset: &mut usize, tokens: &[Token]) {
	increment(index, offset, tokens);
	skip_spaces(index, offset, tokens);
}

fn increment(index: &mut usize, offset: &mut usize, tokens: &[Token]) {
	if *index < tokens.len() {
		let token_length = unsafe { tokens.get_unchecked(*index).length };
		*offset += token_length as usize;
		*index += 1;
	}
}

fn skip_spaces(index: &mut usize, offset: &mut usize, tokens: &[Token]) {
	while *index < tokens.len() {
		let token = unsafe { tokens.get_unchecked(*index) };
		match token.kind {
			TokenKind::Space => {
				*offset += token.length as usize;
				*index += 1;
			}
			_ => {
				break;
			}
		}
	}
}

pub fn parse<'input>(input: &'input str) -> SemanticTree<'input> {
	let tokens = tokenize(input.as_bytes());

	let mut index = 0;
	let mut offset = 0;

	let mut nodes: SemanticTree<'input> = SemanticTree::with_capacity(tokens.len());

	expression_left(input, &mut index, &mut offset, &tokens, &mut nodes);

	nodes
}

#[test]
fn test_parse() {
	let input = "X + 4 + 12";
	let semantic_tree = parse(input);
	println!("{}", semantic_tree_to_string(&semantic_tree));
}

#[inline]
fn expression_left<'input>(
	input: &'input str,
	index: &mut usize,
	offset: &mut usize,
	tokens: &[Token],
	nodes: &mut SemanticTree<'input>
) -> usize {
	let token: &Token = tokens.get(*index).unwrap();

	let node: Node<'input> = match token.kind {
		TokenKind::Identifier =>
			Node::Identifier(&input[*offset..*offset + (token.length as usize)]),
		TokenKind::Integer => {
			Node::Integer(
				input[*offset..*offset + (token.length as usize)].parse::<i32>().unwrap()
			)
		}
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let left = nodes.insert(node);
	next(index, offset, tokens);
	expression_right(input, index, offset, tokens, nodes, left)
}

fn expression_right<'input>(
	input: &'input str,
	index: &mut usize,
	offset: &mut usize,
	tokens: &[Token],
	nodes: &mut SemanticTree<'input>,
	left: usize
) -> usize {
	let maybe_token = tokens.get(*index);

	match maybe_token {
		None => left,
		Some(token) =>
			match token.kind {
				TokenKind::Stop => left,
				TokenKind::Plus => {
					next(index, offset, tokens);
					let right = expression_left(input, index, offset, tokens, nodes);
					let node = nodes.insert(Node::Add { left, right });
					next(index, offset, tokens);
					node
				}
				_ => panic!("Unexpected token: {:?}", maybe_token),
			}
	}
}
