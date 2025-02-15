use crate::recorder::Recorder;
use crate::tokens::Token;

struct Parser<'a, Input> where Input: Iterator<Item = u8> {
	input: &'a Input,
	tokens: Recorder<Token>,
}

impl<'a, Input> Parser<'a, Input> where Input: Iterator<Item = u8> {
	fn new(input: &'a Input) -> Self {
		Parser { input, tokens: Recorder::new() }
	}

	fn parse(&self) -> Result<(), String> {
		Ok(())
	}
}

fn main() {
	let input = "Hello";
	let parser = Parser::new(&input.bytes());
}
