use slab::Slab;
use crate::tokens::Token;
use crate::tokenize::tokenize;
use crate::nodes::Node;

fn next(index: &mut usize, offset: &mut usize, tokens: &[Token], tokens_lengths: &[u8]) {
	increment(index, offset, tokens_lengths);
	skip_spaces(index, offset, tokens, tokens_lengths);
}

fn increment(index: &mut usize, offset: &mut usize, tokens_lengths: &[u8]) {
	if *index < tokens_lengths.len() {
		let token_length = unsafe { tokens_lengths.get_unchecked(*index) };
		*offset += *token_length as usize;
		*index += 1;
	}
}

fn skip_spaces(
	index: &mut usize,
	offset: &mut usize,
	tokens: &[Token],
	tokens_lengths: &[u8]
) {
	while *index < tokens.len() {
		let token = unsafe { tokens.get_unchecked(*index) };
		match token {
			Token::Space => {
				let token_length = unsafe { tokens_lengths.get_unchecked(*index) };
				*offset += *token_length as usize;
				*index += 1;
			}
			_ => {
				break;
			}
		}
	}
}

fn get_slice<'input>(
	input: &'input str,
	index: usize,
	offset: usize,
	tokens_lengths: &[u8]
) -> &'input str {
	&input[offset..offset + (unsafe { *tokens_lengths.get_unchecked(index) as usize })]
}

pub fn ast<'input>(nodes: &'input Slab<Node>) -> &'input Node<'input> {
	unsafe { nodes.get_unchecked(nodes.len() - 1) }
}

pub fn parse<'input>(input: &'input str) -> Slab<Node<'input>> {
	let (tokens, tokens_lengths) = tokenize(input.as_bytes());

	let mut index = 0;
	let mut offset = 0;

	let mut nodes: Slab<Node<'input>> = Slab::with_capacity(tokens.len());

	expression_left(input, &mut index, &mut offset, &tokens, &tokens_lengths, &mut nodes);

	nodes
}

#[test]
fn test_parse() {
	let input = "X + 4";
	let nodes = parse(input);
	println!("{:?}", ast(&nodes));
}

#[inline]
fn expression_left<'input>(
	input: &'input str,
	index: &mut usize,
	offset: &mut usize,
	tokens: &[Token],
	tokens_lengths: &[u8],
	nodes: &mut Slab<Node<'input>>
) -> usize {
	let token: &Token = tokens.get(*index).unwrap();

	let node: Node<'input> = match token {
		Token::Identifier =>
			Node::Identifier(get_slice(input, *index, *offset, tokens_lengths)),
		Token::Integer => {
			Node::Integer(
				get_slice(input, *index, *offset, tokens_lengths).parse::<i32>().unwrap()
			)
		}
		Token::True => Node::Boolean(true),
		Token::False => Node::Boolean(false),
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let left = nodes.insert(node);
	next(index, offset, tokens, tokens_lengths);
	expression_right(input, index, offset, tokens, tokens_lengths, nodes, left)
}

fn expression_right<'input>(
	input: &'input str,
	index: &mut usize,
	offset: &mut usize,
	tokens: &[Token],
	tokens_lengths: &[u8],
	nodes: &mut Slab<Node<'input>>,
	left: usize
) -> usize {
	let token = tokens.get(*index);

	match token {
		None => left,
		Some(Token::Stop) => left,
		Some(Token::Plus) => {
			next(index, offset, tokens, tokens_lengths);
			let right = expression_left(
				input,
				index,
				offset,
				tokens,
				tokens_lengths,
				nodes
			);
			let node = nodes.insert(Node::Add { left, right });
			next(index, offset, tokens, tokens_lengths);
			node
		}
		_ => panic!("Unexpected token: {:?}", token),
	}
}
