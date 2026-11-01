use std::fs;

use super::helpers::assert_no_diagnostics;
use crate::nodes::Node;
use crate::parsing::parse::parse;

/// Parse a file of `src/tests/fixtures`, and check that it has no diagnostic.
fn parse_fixture(name: &str) -> crate::TypedSyntaxTree {
	let path = format!("{}/src/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
	let source = fs::read_to_string(&path).expect("fixture should be readable");
	let tree = parse(&source);
	assert_no_diagnostics(&tree, name);
	tree
}

#[test]
fn parse_long_fixture() {
	let tree = parse_fixture("long-file.fa");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("the root should be a module");
	};
	assert_eq!(statements.len(), 17);
}
