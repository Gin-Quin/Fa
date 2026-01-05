use crate::context::Context;
use crate::parsing::parse_statement::parse_statement;
use crate::typed_syntax_tree::TypedSyntaxTree;

pub fn parse_single_statement(input: &'static str) -> TypedSyntaxTree {
	let mut tree = TypedSyntaxTree::new(input);
	let mut context = Context::new(input, &mut tree);
	let root = parse_statement(&mut context);
	tree.root = root;
	tree
}
