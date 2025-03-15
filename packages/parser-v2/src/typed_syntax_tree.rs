use slab::Slab;

use crate::nodes::Node;

#[derive(Debug)]
pub struct TypedSyntaxTree {
	pub input: &'static str,
	pub nodes: Slab<Node>,
	pub root: usize,
}

impl TypedSyntaxTree {
	pub fn new(input: &'static str) -> Self {
		TypedSyntaxTree {
			input,
			nodes: Slab::new(),
			root: 0,
		}
	}

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

		macro_rules! ListWithoutParenthesis {
			($operation:expr, $operands:expr) => {
				$operands
				.iter()
				.map(|e| self.node_to_string(*e))
				.collect::<Vec<String>>()
				.join($operation)
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
			Node::Module { statements, .. } =>
				format!("{};\n", ListWithoutParenthesis!(";\n", statements)),
			Node::DanglingToken { token, .. } => format!("Dangling {:#?}", token),

			Node::Identifier(value) => value.to_string(),
			Node::Integer(value) => value.to_string(),
			Node::Boolean(value) => value.to_string(),

			Node::Not { right, .. } => Prefix!("not ", right),
			Node::Negate { right, .. } => Prefix!("-", right),

			Node::Add { operands, .. } => List!(" + ", operands),
			Node::Subtract { operands, .. } => List!(" - ", operands),
			Node::Multiply { operands, .. } => List!(" * ", operands),
			Node::Divide { operands, .. } => List!(" / ", operands),
			Node::IntegerDivide { operands, .. } => List!(" // ", operands),
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
			Node::Tuple { items, .. } => List!(", ", items),

			Node::Group { expression, .. } => {
				let expression_str = self.node_to_string(*expression);
				format!("({})", expression_str)
			}
			Node::EmptyGroup { .. } => String::from("()"),

			Node::FunctionCall { function, parameters, .. } => {
				let function_str = self.node_to_string(*function);
				let parameters_str: String = match *parameters {
					Some(parameters) => self.node_to_string(parameters),
					None => String::from("()"),
				};
				format!("{}({})", function_str, parameters_str)
			}

			Node::Assignment { name, type_expression, expression, .. } => {
				match name {
					Ok(name) => {
						let mut string: String = String::from("let ");
						string += name;
						if let Some(type_expression) = type_expression {
							string += ": ";
							string += &self.node_to_string(*type_expression);
						}
						if let Some(expression) = expression {
							string += " = ";
							string += &self.node_to_string(*expression);
						}
						string
					}
					Err(name) => format!("Expected identifier, got {:#?}", name),
				}
			}
		}
	}

	/// Converts a semantic tree to its string representation
	pub fn to_string(self: &TypedSyntaxTree) -> String {
		self.node_to_string(self.root)
	}
}
