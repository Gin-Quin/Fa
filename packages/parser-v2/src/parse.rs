use slab::Slab;
use crate::tokens::Token;
use crate::tokenize::tokenize;
use crate::nodes::Node;

#[derive(Debug)]
pub struct Output<'input> {
	pub input: &'input str,
	pub tokens: Vec<Token>,
	pub tokens_lengths: Vec<u8>,
	pub nodes: Slab<Node<'input>>,
}

#[derive(Debug)]
struct Progress<'input> {
	pub index: usize,
	pub offset: usize,
	pub output: &'input mut Output<'input>,
}

impl<'input> Progress<'input> {
	fn next(&mut self) {
		self.increment();
		self.skip_spaces();
	}

	fn increment(&mut self) {
		if self.index < self.output.tokens_lengths.len() {
			let token_length = unsafe {
				self.output.tokens_lengths.get_unchecked(self.index)
			};
			self.offset += *token_length as usize;
			self.index += 1;
		}
	}

	fn skip_spaces(&mut self) {
		while self.index < self.output.tokens.len() {
			let token = unsafe { self.output.tokens.get_unchecked(self.index) };
			match token {
				Token::Space => {
					let token_length = unsafe {
						self.output.tokens_lengths.get_unchecked(self.index)
					};
					self.offset += *token_length as usize;
					self.index += 1;
				}
				_ => {
					break;
				}
			}
		}
	}
}

impl<'input> Output<'input> {
	fn slice(&self, progress: &Progress) -> &'input str {
		&self.input
			[
				progress.offset..progress.offset +
					(unsafe {
						*self.tokens_lengths.get_unchecked(progress.index) as usize
					})
			]
	}

	pub fn ast(&self) -> &Node {
		unsafe { self.nodes.get_unchecked(self.nodes.len()) }
	}
}

pub fn parse(input: &str) -> Output {
	let (tokens, tokens_lengths) = tokenize(input.as_bytes());
	let mut output = Output {
		input,
		tokens,
		tokens_lengths,
		nodes: Slab::new(),
	};
	let mut progress = Progress { index: 0, offset: 0, output: &mut output };
	expression_left(&mut progress);
	output
}

#[test]
fn test_parse() {
	let input = "X + 4";
	let output = parse(input);
	println!("{:?}", output.ast());
}

#[inline]
fn expression_left(progress: &mut Progress) -> usize {
	let token: &Token = progress.output.tokens.get(progress.index).unwrap();

	let node = match token {
		Token::Identifier => Node::Identifier(output.slice(progress)),
		Token::Integer => {
			println!("Reading integer: {:?}", progress);
			Node::Integer(output.slice(progress).parse::<i32>().unwrap())
		}
		Token::True => Node::Boolean(true),
		Token::False => Node::Boolean(false),
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let left = output.nodes.insert(node);
	progress.next(output);
	expression_right(output, progress, left)
}

fn expression_right(progress: &mut Progress, left: usize) -> usize {
	let token = output.tokens.get(progress.index);

	match token {
		None => left,
		Some(Token::Stop) => left,
		Some(Token::Plus) => {
			progress.next(output);
			let right = expression_left(output, progress);
			let node = output.nodes.insert(Node::Add { left, right });
			progress.next(output);
			node
		}
		_ => panic!("Unexpected token: {:?}", token),
	}
}
