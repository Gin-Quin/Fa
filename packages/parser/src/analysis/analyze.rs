use crate::{nodes::Node, typed_syntax_tree::TypedSyntaxTree};

pub fn analyze(tree: &mut TypedSyntaxTree) -> &TypedSyntaxTree {
	println!("Analyzing...");
	tree
}

pub fn analyze_node(tree: &mut TypedSyntaxTree, node_index: usize) {
	let node = tree.at_mutable(node_index);
	match &mut *node {
		Node::Module { statements, .. } => {
			let statements = statements.clone();
			for statement in statements {
				analyze_node(tree, statement);
			}
		}
		// Node::Let {
		// 	resolved_type,
		// 	type_expression,
		// 	expression,
		// 	..
		// } => {
		// 	let expression = *expression;
		// 	let resolved_type_ptr = resolved_type as *mut Option<Type>;

		// 	if let Some(expression) = expression {
		// 		if let Some(type_expression) = type_expression {
		// 			// let type_to_satisfy =
		// 			unsafe {
		// 				*resolved_type_ptr = None;
		// 			}
		// 			println!("Todo: add some diagnostics. Missing type or value.");
		// 		} else {
		// 		}
		// 		println!("Todo: add some diagnostics. Missing type or value.");
		// 	}
		// }
		_ => println!("Missing analysis for node type {:?}", node),
	}
}
