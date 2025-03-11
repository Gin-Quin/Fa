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

		macro_rules! List {
			($operation:expr, $operands:expr) => {
				format!("({})", $operands
				.iter()
				.map(|e| self.node_to_string(*e))
				.collect::<Vec<String>>()
				.join($operation))
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

			Node::Add { operands, .. } => List!(" + ", operands),
			Node::Subtract { operands, .. } => List!(" - ", operands),
			Node::Multiply { operands, .. } => List!(" * ", operands),
			Node::Divide { operands, .. } => List!(" / ", operands),
			Node::Modulo { operands, .. } => List!(" modulo ", operands),
			Node::Power { operands, .. } => List!(" ** ", operands),
			Node::Equal { operands, .. } => List!(" == ", operands),
			Node::NotEqual { operands, .. } => List!(" != ", operands),
			Node::LessThan { operands, .. } => List!(" < ", operands),
			Node::LessThanOrEqual { operands, .. } => List!(" <= ", operands),
			Node::GreaterThan { operands, .. } => List!(" > ", operands),
			Node::GreaterThanOrEqual { operands, .. } => List!(" >= ", operands),
			Node::And { operands, .. } => List!(" and ", operands),
			Node::Or { operands, .. } => List!(" or ", operands),
			Node::Is { left, right, .. } => Operation!("is", left, right),
			Node::Union { operands, .. } => List!(" | ", operands),
			Node::Pipe { operands, .. } => List!(" |> ", operands),
			Node::Insert { left, right, .. } => Operation!("<<", left, right),
			Node::Extract { left, right, .. } => Operation!(">>", left, right),

			Node::Not { right, .. } => Prefix!("not ", right),
			Node::Negate { right, .. } => Prefix!("-", right),
			Node::Group { expression, .. } => {
				let expression_str = self.node_to_string(*expression);
				format!("({})", expression_str)
			}
			Node::Tuple { items, .. } => List!(", ", items),
			Node::UnexpectedTokenError { token, .. } =>
				format!("Error: Unexpected token {:#?}", token),
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
