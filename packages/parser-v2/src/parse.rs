use slab::Slab;
use crate::tokens::Token;
use crate::tokenize::tokenize;
use crate::nodes::Node;

pub struct Context<'input> {
	pub input: &'input str,
	pub tokens: Vec<Token>,
	pub tokens_lengths: Vec<u8>,
	pub nodes: Slab<Node<'input>>,
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

pub fn expression(context: &mut Context, index: usize) -> usize {
	let token = context.tokens.get(index).unwrap();

	let node = match token {
		Token::Plus => {
			let left = expression(context, index + 1);
			let right = expression(context, index + 2);
			Node::Add { left, right }
		}
		Token::Identifier => Node::Identifier(context.slice(index)),
		Token::Integer => Node::Integer(context.slice(index).parse::<i32>().unwrap()),
		Token::True => Node::Boolean(true),
		Token::False => Node::Boolean(false),
		// Add other token matches here
		_ => todo!("Handle other token types"),
	};

	context.nodes.insert(node)
}
