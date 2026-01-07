use crate::{nodes::Node, typed_syntax_tree::TypedSyntaxTree, types::Type};

/// Takes a "type node" as an argument (like "Number" or { foo: Number }) and resolves it to a Type.
pub fn resolve_type_node(tree: &TypedSyntaxTree, node_index: usize) -> Type {
	match tree.at(node_index) {
		/* -------------------------------- Literals -------------------------------- */
		Node::Integer(value) => Type::IntegerLiteral(value.clone()),
		Node::StringLiteral(value) => Type::StringLiteral(value.clone()),
		Node::StringTemplate { .. } => {
			// send a diagnosic instead of a panic later
			panic!("StringTemplate cannot (yet) be used as a type node");
		}
		Node::Boolean(value) => {
			if *value {
				Type::True
			} else {
				Type::False
			}
		}
		Node::Null => Type::Null,
		// Node::Percentage { value } => {
		// 	let inner_type = infer_node_type(tree, *value);
		// 	match inner_type {
		// 		Type::Number(inner_number_type) => Type::Percentage(inner_number_type),
		// 		_ => Type::Errored(TypeError::PercentageMustBeNumber),
		// 	}
		// }

		/* ------------------------------- Identifiers ------------------------------- */
		// Node::Identifier(name) => Type::Identifier(name.clone()),

		/* -------------------------------- Composed -------------------------------- */
		// Node::List { items } => Type::Array {
		// 	items: Box::new(merge_inferred_type_list(tree, items)),
		// },
		// Node::Members { .. } => Type::Object {
		// 	properties: HashMap::new(),
		// },
		_ => Type::Any,
	}
}
