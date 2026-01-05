use crate::context::Context;
use crate::parsing::parse_statement::parse_statement;
use crate::tokenize::tokenize;
use crate::tokens::Token;
use crate::typed_syntax_tree::TypedSyntaxTree;

pub fn parse_single_statement(input: &'static str) -> TypedSyntaxTree {
	let tokens: Vec<Token> = tokenize(input.as_bytes());
	let mut tree = TypedSyntaxTree::new(input);
	let mut context = Context::new(input, &mut tree, &tokens);
	let root = parse_statement(&mut context);
	tree.root = root;
	tree
}
