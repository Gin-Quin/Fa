use crate::{nodes::Node, typed_syntax_tree::TypedSyntaxTree, types::Type};

pub fn analyze(tree: &mut TypedSyntaxTree) -> &TypedSyntaxTree {
	println!("Analyzing...");
	let root = tree.root;
	// analyze_node(tree, root);
	tree
}

pub fn analyze_node(tree: &mut TypedSyntaxTree, node_index: usize) {
	let node = tree.at_mutable(node_index);
	match &mut *node {
		Node::Module { statements } => {
			let statements = statements.clone();
			for statement in statements {
				analyze_node(tree, statement);
			}
		}
		Node::Let {
			right,
			resolved_type,
		} => {
			let right = *right;
			let resolved_type_ptr = resolved_type as *mut Option<Type>;
			let right_node = tree.at(right);

			match right_node {
				Node::Identifier(name) => {
					println!("Todo: add some diagnostics. Missing type or value.")
				}
				Node::Assignment {
					name,
					type_expression,
					expression,
				} => {
					if let Some(expression_to_validate) = expression {
						if let Some(type_expression) = type_expression {
							// let type_to_satisfy =
							unsafe {
								*resolved_type_ptr = None;
							}
							println!("Todo: add some diagnostics. Missing type or value.");
						} else {
						}
						println!("Todo: add some diagnostics. Missing type or value.");
					}
				}
				_ => {
					println!("Todo: add some diagnostics")
				}
			};
		}
		_ => println!("Missing analysis for node type {:?}", node),
	}
}
