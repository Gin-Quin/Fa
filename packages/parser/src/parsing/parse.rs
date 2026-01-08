use crate::analysis::analyze::analyze;
use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_statement::parse_statement;
use crate::source::SourceSpan;
use crate::typed_syntax_tree::TypedSyntaxTree;

pub fn parse(input: &'static str) -> TypedSyntaxTree {
	let mut tree = TypedSyntaxTree::new(input);
	let mut context = Context::new(input, &mut tree);

	let mut statements: Vec<usize> = vec![];

	println!("{tree:#?}");
	println!("{:#?}", unsafe { &*context.tree });

	while !context.done() {
		let statement = parse_statement(&mut context);
		statements.push(statement);
	}

	let span = if statements.is_empty() {
		SourceSpan::new(0, 0)
	} else {
		let first = statements[0];
		let last = statements[statements.len() - 1];
		SourceSpan::new(tree.span(first).start, tree.span(last).end)
	};
	tree.root = tree.insert(Node::Module { statements }, span);

	analyze(&mut tree);
	tree
}
