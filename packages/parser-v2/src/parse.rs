use crate::context::Context;
use crate::nodes::*;
use crate::tokenize::tokenize;
use crate::tokens::Token;
use crate::typed_syntax_tree::TypedSyntaxTree;
use crate::parse_statement::parse_statement;

pub fn parse(input: &'static str) -> TypedSyntaxTree {
	let tokens: Vec<Token> = tokenize(input.as_bytes());
	let mut tree = TypedSyntaxTree::new(input);
	let mut context = Context::new(input, &mut tree, &*tokens);

	let mut statements: Vec<usize> = vec![];

	println!("{:#?}", tree);
	println!("{:#?}", unsafe { &*context.tree });

	while !context.done() {
		let statement = parse_statement(&mut context);
		statements.push(statement);
	}

	tree.root = tree.nodes.insert(Node::Module { statements });
	tree
}

pub fn parse_single_statement(input: &'static str) -> TypedSyntaxTree {
	let tokens: Vec<Token> = tokenize(input.as_bytes());
	let mut tree = TypedSyntaxTree::new(input);
	let mut context = Context::new(input, &mut tree, &*tokens);
	let root = parse_statement(&mut context);
	tree.root = root;
	tree
}
