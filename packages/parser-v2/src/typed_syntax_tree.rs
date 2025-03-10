use slab::Slab;

use crate::nodes::Node;

#[derive(Debug)]
pub struct TypedSyntaxTree {
	pub input: &'static str,
	pub nodes: Slab<Node>,
}

impl TypedSyntaxTree {
	/// Converts a node to its string representation
	pub fn node_to_string(self: &TypedSyntaxTree, index: usize) -> String {
		let node = &self.nodes[index];

		macro_rules! Operation {
			($operation:expr, $left:expr, $right:expr) => {
			{
				let left_str = self.node_to_string(*$left);
				let right_str = self.node_to_string(*$right);
				format!("({} {} {})", left_str, $operation, right_str)
			}
			};
		}

		macro_rules! Prefix {
			($operation:expr, $right:expr) => {
			{
				let right_str = self.node_to_string(*$right);
				format!("({}{})", $operation, right_str)
			}
			};
		}
		match node {
			Node::Identifier(value) => value.to_string(),
			Node::Integer(value) => value.to_string(),
			Node::Boolean(value) => value.to_string(),

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
				let expression_str = self.node_to_string(*expression);
				format!("({})", expression_str)
			}
		}
	}

	/// Converts a semantic tree to its string representation
	pub fn to_string(self: &TypedSyntaxTree) -> String {
		let last_node_index = self.nodes.len();

		if last_node_index == 0 {
			return String::new();
		} else {
			return self.node_to_string(last_node_index - 1);
		}
	}
}
