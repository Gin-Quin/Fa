use slab::Slab;

pub type SemanticTree<'input> = Slab<Node<'input>>;

#[derive(Debug)]
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
	Not {
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
	Divide {
		left: usize,
		right: usize,
	},
	IntegerDivide {
		left: usize,
		right: usize,
	},
	Modulo {
		left: usize,
		right: usize,
	},
	Power {
		left: usize,
		right: usize,
	},
	Equal {
		left: usize,
		right: usize,
	},
	NotEqual {
		left: usize,
		right: usize,
	},
	LessThan {
		left: usize,
		right: usize,
	},
	LessThanOrEqual {
		left: usize,
		right: usize,
	},
	GreaterThan {
		left: usize,
		right: usize,
	},
	GreaterThanOrEqual {
		left: usize,
		right: usize,
	},
	And {
		left: usize,
		right: usize,
	},
	Or {
		left: usize,
		right: usize,
	},
	Is {
		left: usize,
		right: usize,
	},
	FatArrow {
		left: usize,
		right: usize,
	},
	Union {
		left: usize,
		right: usize,
	},
	Pipe {
		left: usize,
		right: usize,
	},
	Insert {
		left: usize,
		right: usize,
	},
	Extract {
		left: usize,
		right: usize,
	},

	/* --------------------------------- Groups --------------------------------- */
	Group {
		expression: usize,
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
		Node::Identifier(name) => name.to_string(),
		Node::Integer(value) => value.to_string(),
		Node::Boolean(value) => value.to_string(),

		Node::Add { left, right } => Operation!("+", left, right),
		Node::Subtract { left, right } => Operation!("-", left, right),
		Node::Multiply { left, right } => Operation!("*", left, right),
		Node::Divide { left, right } => Operation!("/", left, right),
		Node::IntegerDivide { left, right } => Operation!("//", left, right),
		Node::Modulo { left, right } => Operation!("modulo", left, right),
		Node::Power { left, right } => Operation!("**", left, right),
		Node::Equal { left, right } => Operation!("==", left, right),
		Node::NotEqual { left, right } => Operation!("!=", left, right),
		Node::LessThan { left, right } => Operation!("<", left, right),
		Node::LessThanOrEqual { left, right } => Operation!("<=", left, right),
		Node::GreaterThan { left, right } => Operation!(">", left, right),
		Node::GreaterThanOrEqual { left, right } => Operation!(">=", left, right),
		Node::And { left, right } => Operation!("and", left, right),
		Node::Or { left, right } => Operation!("or", left, right),
		Node::Is { left, right } => Operation!("is", left, right),
		Node::FatArrow { left, right } => Operation!("=>", left, right),
		Node::Union { left, right } => Operation!("|", left, right),
		Node::Pipe { left, right } => Operation!("|>", left, right),
		Node::Insert { left, right } => Operation!("<<", left, right),
		Node::Extract { left, right } => Operation!(">>", left, right),

		Node::Not { right } => Prefix!("not ", right),
		Node::Negate { right } => Prefix!("-", right),
		Node::Group { expression } => {
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
