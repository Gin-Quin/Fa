use crate::nodes::Node;
use crate::parsing::parse_single_statement::parse_single_statement;
use crate::source::SourceSpan;

#[test]
fn group_span_includes_parentheses() {
	let tree = parse_single_statement("(a)");
	let span = tree.span(tree.root);
	assert_eq!(span, SourceSpan::new(0, 3));
	match tree.at(tree.root) {
		Node::Group { .. } | Node::EmptyGroup => {}
		_ => panic!("Expected group node"),
	}
}

#[test]
fn members_span_includes_braces() {
	let tree = parse_single_statement("{ a }");
	let span = tree.span(tree.root);
	assert_eq!(span, SourceSpan::new(0, 5));
	match tree.at(tree.root) {
		Node::Members { .. } | Node::StaticMembers { .. } => {}
		_ => panic!("Expected members node"),
	}
}
