use crate::{nodes::Node, typed_syntax_tree::TypedSyntaxTree};

pub fn analyze(tree: &mut TypedSyntaxTree) -> &TypedSyntaxTree {
	println!("Analyzing...");
	let root = tree.root;
	// analyze_node(tree, root);
	tree
}

pub fn analyze_node(tree: &mut TypedSyntaxTree, node_index: usize) {
	let node = tree.at(node_index).clone();

	match node {
		Node::Module { statements } => {
			for statement in statements {
				analyze_node(tree, statement);
			}
		}
		// Node::Let { right, .. } => {
		// 	let resolved = resolve_type(tree, right);
		// 	if let Node::Let { resolved_type, .. } = tree.at_mutable(node_index) {
		// 		*resolved_type = Some(resolved);
		// 	}
		// }
		_ => println!("Missing analysis for node type {:?}", node),
	}
}
