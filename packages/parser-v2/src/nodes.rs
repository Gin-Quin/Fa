use slab::Slab;

pub type SemanticTree<'input> = Slab<Node<'input>>;

pub enum Node<'input> {
	Identifier(&'input str),
	Integer(i32),
	// Float(f64),
	Boolean(bool),
	// String(&str),
	// Array(Vec<Node>),
	// Tuple(Vec<Node>),

	/* ------------------------------- Operations ------------------------------- */
	Negate {
		right: usize,
	},
	Add {
		left: usize,
		right: usize,
	},
	Subtract {
		left: usize,
		right: usize,
	},
	Multiply {
		left: usize,
		right: usize,
	},
	// Divide {
	// 	left: &Node,
	// 	right: &Node,
	// },
	// Modulo,
	// Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
	None = 0,
	Comma,
	Assignment,
	Or,
	And,
	Equality,
	Comparison,
	Additive,
	Multiplicative,
	Exponentiation,
	Prefix,
	Postfix,
	Access,
}

/// Converts a node to its string representation
pub fn node_to_string<'input>(
	semantic_tree: &SemanticTree<'input>,
	index: usize
) -> String {
	let node = &semantic_tree[index];

	match node {
		Node::Identifier(name) => name.to_string(),
		Node::Integer(value) => value.to_string(),
		Node::Boolean(value) => value.to_string(),
		Node::Add { left, right } => {
			let left_str = node_to_string(semantic_tree, *left);
			let right_str = node_to_string(semantic_tree, *right);
			format!("({} + {})", left_str, right_str)
		}
		Node::Subtract { left, right } => {
			let left_str = node_to_string(semantic_tree, *left);
			let right_str = node_to_string(semantic_tree, *right);
			format!("({} - {})", left_str, right_str)
		}
		Node::Multiply { left, right } => {
			let left_str = node_to_string(semantic_tree, *left);
			let right_str = node_to_string(semantic_tree, *right);
			format!("({} * {})", left_str, right_str)
		}
		Node::Negate { right } => {
			let right_str = node_to_string(semantic_tree, *right);
			format!("-{}", right_str)
		}
	}
}

/// Converts a semantic tree to its string representation
pub fn semantic_tree_to_string<'input>(semantic_tree: &SemanticTree<'input>) -> String {
	let last_node_index = semantic_tree.len();

	if last_node_index == 0 {
		return String::new();
	} else {
		return node_to_string(semantic_tree, last_node_index - 1);
	}
}
