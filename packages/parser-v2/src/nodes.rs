use crate::tokens::Token;
use slab::Slab;

pub type SemanticTree<'input> = Slab<Node>;

#[derive(Debug, Clone)]
pub enum Node {
	Identifier {
		token: Token,
	},
	Integer {
		value: i32,
		token: Token,
	},
	// Float(f64),
	Boolean {
		value: bool,
		token: Token,
	},
	// String(&str),
	// Array(Vec<Node>),
	// Tuple(Vec<Node>),

	/* ------------------------------- Operations ------------------------------- */
	Negate {
		right: usize,
		token: Token,
	},
	Not {
		right: usize,
		token: Token,
	},
	Add {
		left: usize,
		right: usize,
		token: Token,
	},
	Subtract {
		left: usize,
		right: usize,
		token: Token,
	},
	Multiply {
		left: usize,
		right: usize,
		token: Token,
	},
	Divide {
		left: usize,
		right: usize,
		token: Token,
	},
	IntegerDivide {
		left: usize,
		right: usize,
		token: Token,
	},
	Modulo {
		left: usize,
		right: usize,
		token: Token,
	},
	Power {
		left: usize,
		right: usize,
		token: Token,
	},
	Equal {
		left: usize,
		right: usize,
		token: Token,
	},
	NotEqual {
		left: usize,
		right: usize,
		token: Token,
	},
	LessThan {
		left: usize,
		right: usize,
		token: Token,
	},
	LessThanOrEqual {
		left: usize,
		right: usize,
		token: Token,
	},
	GreaterThan {
		left: usize,
		right: usize,
		token: Token,
	},
	GreaterThanOrEqual {
		left: usize,
		right: usize,
		token: Token,
	},
	And {
		left: usize,
		right: usize,
		token: Token,
	},
	Or {
		left: usize,
		right: usize,
		token: Token,
	},
	Is {
		left: usize,
		right: usize,
		token: Token,
	},
	FatArrow {
		left: usize,
		right: usize,
		token: Token,
	},
	Union {
		left: usize,
		right: usize,
		token: Token,
	},
	Pipe {
		left: usize,
		right: usize,
		token: Token,
	},
	Insert {
		left: usize,
		right: usize,
		token: Token,
	},
	Extract {
		left: usize,
		right: usize,
		token: Token,
	},

	/* --------------------------------- Groups --------------------------------- */
	Group {
		expression: usize,
		token: Token,
	},
}

/// Converts a node to its string representation
pub fn node_to_string<'input>(
	semantic_tree: &SemanticTree<'input>,
	index: usize
) -> String {
	let node = &semantic_tree[index];

	macro_rules! Operation {
		($operation:expr, $left:expr, $right:expr) => {
			{
				let left_str = node_to_string(semantic_tree, *$left);
				let right_str = node_to_string(semantic_tree, *$right);
				format!("({} {} {})", left_str, $operation, right_str)
			}
		};
	}

	macro_rules! Prefix {
		($operation:expr, $right:expr) => {
			{
				let right_str = node_to_string(semantic_tree, *$right);
				format!("({}{})", $operation, right_str)
			}
		};
	}
	match node {
		Node::Identifier { token } => token.to_string(),
		Node::Integer { value, .. } => value.to_string(),
		Node::Boolean { value, .. } => value.to_string(),

		Node::Add { left, right, .. } => Operation!("+", left, right),
		Node::Subtract { left, right, .. } => Operation!("-", left, right),
		Node::Multiply { left, right, .. } => Operation!("*", left, right),
		Node::Divide { left, right, .. } => Operation!("/", left, right),
		Node::IntegerDivide { left, right, .. } => Operation!("//", left, right),
		Node::Modulo { left, right, .. } => Operation!("modulo", left, right),
		Node::Power { left, right, .. } => Operation!("**", left, right),
		Node::Equal { left, right, .. } => Operation!("==", left, right),
		Node::NotEqual { left, right, .. } => Operation!("!=", left, right),
		Node::LessThan { left, right, .. } => Operation!("<", left, right),
		Node::LessThanOrEqual { left, right, .. } => Operation!("<=", left, right),
		Node::GreaterThan { left, right, .. } => Operation!(">", left, right),
		Node::GreaterThanOrEqual { left, right, .. } => Operation!(">=", left, right),
		Node::And { left, right, .. } => Operation!("and", left, right),
		Node::Or { left, right, .. } => Operation!("or", left, right),
		Node::Is { left, right, .. } => Operation!("is", left, right),
		Node::FatArrow { left, right, .. } => Operation!("=>", left, right),
		Node::Union { left, right, .. } => Operation!("|", left, right),
		Node::Pipe { left, right, .. } => Operation!("|>", left, right),
		Node::Insert { left, right, .. } => Operation!("<<", left, right),
		Node::Extract { left, right, .. } => Operation!(">>", left, right),

		Node::Not { right, .. } => Prefix!("not ", right),
		Node::Negate { right, .. } => Prefix!("-", right),
		Node::Group { expression, .. } => {
			let expression_str = node_to_string(semantic_tree, *expression);
			format!("({})", expression_str)
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
