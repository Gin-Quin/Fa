use slab::Slab;
use crate::tokens::Token;
use crate::tokenize::tokenize;
use crate::nodes::Node;

pub struct Context<'input> {
	pub input: &'input str,
	pub tokens: Vec<Token>,
	pub tokens_lengths: Vec<u8>,
	pub nodes: Slab<Node<'input>>,
	pub index: usize,
}

impl<'input> Context<'input> {
	pub fn slice(&self, index: usize) -> &'input str {
		&self.input
			[
				index..index +
					(unsafe { *self.tokens_lengths.get_unchecked(index) as usize })
			]
	}
}

pub fn parse(input: &str) -> Vec<Token> {
	let (tokens, tokens_lengths) = tokenize(input.as_bytes());
	tokens
}

#[test]
fn test_parse() {
	let input = "if x == 12";
	let tokens = parse(input);
	println!("{:?}", tokens);
}

#[inline]
pub fn expression_left(context: &mut Context) -> usize {
	let token = context.tokens.get(context.index).unwrap();

	let node = match token {
		Token::Identifier => Node::Identifier(context.slice(context.index)),
		Token::Integer =>
			Node::Integer(context.slice(context.index).parse::<i32>().unwrap()),
		Token::True => Node::Boolean(true),
		Token::False => Node::Boolean(false),
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let left = context.nodes.insert(node);
	context.index += 1;
	expression_right(context, left)
}

pub fn expression_right(context: &mut Context, left: usize) -> usize {
	let token = context.tokens.get(context.index);

	match token {
		None => left,
		Some(Token::Stop) => left,
		Some(Token::Plus) => {
			context.index += 1;
			let right = expression_left(context);
			let node = context.nodes.insert(Node::Add { left, right });
			context.index += 1;
			node
		}
		_ => panic!("Unexpected token: {:?}", token),
	}
}
