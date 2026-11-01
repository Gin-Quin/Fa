//! The API of the tree for the checker and the language server.

use std::sync::Arc;

use crate::nodes::{Name, Node};
use crate::parsing::parse::{parse, parse_shared};

#[test]
fn children_lists_every_child() {
	let tree = parse("let total = price * (1 + rate)");
	let mut visited = vec![false; tree.nodes.len()];
	let mut stack = vec![tree.root];
	while let Some(index) = stack.pop() {
		visited[index] = true;
		stack.extend(tree.at(index).children());
	}
	assert!(visited.iter().all(|visited| *visited));
}

#[test]
fn children_of_a_lambda_include_its_signature() {
	let tree = parse("(a: Integer): Integer throws E => a");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	assert_eq!(tree.at(statements[0]).children().count(), 4);
}

#[test]
fn kind_name_names_the_variant() {
	let tree = parse("f(x)");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	assert_eq!(tree.at(statements[0]).kind_name(), "FunctionCall");
	assert_eq!(tree.at(tree.root).kind_name(), "Module");
}

#[test]
fn names_point_into_the_source() {
	let tree = parse("let count = 1");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	let Node::Let { name, .. } = tree.at(statements[0]) else {
		panic!("expected a declaration");
	};
	assert_eq!(tree.name(*name), "count");
	assert_eq!(*name, Name::new(4, 9));
}

#[test]
fn shared_source_is_not_copied() {
	let source: Arc<str> = Arc::from("let a = 1");
	let tree = parse_shared(source.clone());
	assert!(Arc::ptr_eq(&source, &tree.source));
}

#[test]
fn tree_outlives_the_parsed_string() {
	let tree = {
		let source = String::from("let a = 1");
		parse(&source)
	};
	assert_eq!(tree.to_string(), "let a = 1;");
}

#[test]
fn blocks_are_nodes_with_hoisted_functions() {
	let tree = parse("if a {\n\tf()\n\tfunction f = () => 1\n}");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	let Node::If { then_body, .. } = tree.at(statements[0]) else {
		panic!("expected an if");
	};
	let Node::Block {
		hoisted_symbols, ..
	} = tree.at(*then_body)
	else {
		panic!("expected a block");
	};
	assert_eq!(hoisted_symbols.len(), 1);
	assert_eq!(tree.name(hoisted_symbols[0].name), "f");
}

#[test]
fn module_hoists_its_functions() {
	let tree = parse("f()\nfunction f = () => 1");
	let Node::Module {
		hoisted_symbols, ..
	} = tree.at(tree.root)
	else {
		panic!("expected a module");
	};
	assert_eq!(hoisted_symbols.len(), 1);
}

#[test]
fn documentation_is_the_comment_above_a_declaration() {
	let tree = parse("-- the answer\n-- to everything\nlet answer = 42");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	assert_eq!(
		tree.documentation(statements[0]).as_deref(),
		Some("the answer\nto everything")
	);
}

#[test]
fn blank_line_detaches_the_documentation() {
	let tree = parse("-- a note\n\nlet answer = 42");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	assert_eq!(tree.documentation(statements[0]), None);
}

#[test]
fn diagnostics_have_line_and_column() {
	let tree = parse("let a = 1\nlet = 2");
	let text = tree.diagnostics[0].display(&tree.source_map).to_string();
	assert!(text.starts_with("2:5: error:"), "{text}");
}

#[test]
fn node_is_small() {
	assert!(std::mem::size_of::<Node>() <= 64);
}
