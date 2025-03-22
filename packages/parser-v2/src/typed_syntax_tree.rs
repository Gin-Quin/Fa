use crate::nodes::Node;

#[derive(Debug)]
pub struct TypedSyntaxTree {
	pub input: &'static str,
	pub nodes: Vec<Node>,
	pub root: usize,
}

impl TypedSyntaxTree {
	pub fn new(input: &'static str) -> Self {
		TypedSyntaxTree {
			input,
			nodes: Vec::new(),
			root: 0,
		}
	}

	/// Insert a node into the tree and return its index
	pub fn insert(&mut self, node: Node) -> usize {
		self.nodes.push(node);
		self.nodes.len() - 1
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
				.filter_map(|e| {
					let node_str = self.node_to_string(*e);
					if node_str.is_empty() { None } else { Some(node_str) }
				})
				.collect::<Vec<String>>()
				.join($operation))
			};
		}

		macro_rules! ListWithoutParenthesis {
			($operation:expr, $operands:expr) => {
				$operands
				.iter()
				.filter_map(|e| {
					let node_str = self.node_to_string(*e);
					if node_str.is_empty() { None } else { Some(node_str) }
				})
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

		macro_rules! PrefixWithoutParenthesis {
			($operation:expr, $right:expr) => {
			{
				let right_str = self.node_to_string(*$right);
				format!("{}{}", $operation, right_str)
			}
			};
		}

		match node {
			Node::Module { statements, .. } =>
				format!("{};\n", ListWithoutParenthesis!(";\n", statements)),
			Node::DanglingToken { token, .. } => format!("Dangling {:#?}", token),

			Node::Identifier(value) => value.to_string(),
			Node::Integer(value) => value.to_string(),
			Node::Number(value) => value.to_string(),
			Node::Boolean(value) => value.to_string(),

			Node::Not { right, .. } => Prefix!("not ", right),
			Node::Negate { right, .. } => Prefix!("-", right),

			Node::Let { right, .. } => PrefixWithoutParenthesis!("let ", right),
			Node::Return { expression, .. } => if let Some(expression) = expression {
				PrefixWithoutParenthesis!("return ", expression)
			} else {
				String::from("return")
			}

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
				let parameters_str = self.parameters_to_string(parameters);
				format!("{}({})", function_str, parameters_str)
			}

			Node::Assignment { name, type_expression, expression, .. } => {
				let mut string: String = String::new();
				string += &self.node_to_string(*name);

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

			Node::Function { name, parameters, return_type_expression, body, .. } => {
				let mut string: String = String::from("function ");
				string += *name;
				string += "(";
				string += &self.parameters_to_string(parameters);
				string += "): ";
				string += &self.node_to_string(*return_type_expression);
				string += " {\n\t";
				string += &ListWithoutParenthesis!("\n\t", body);
				string += "\n}";
				string
			}

			Node::ShortFunction { name, parameters, expression, .. } => {
				let mut string: String = String::from("function ");
				string += name;
				string += "(";
				string += &self.parameters_to_string(parameters);
				string += ")";
				string += " => ";
				string += &self.node_to_string(*expression);
				string
			}
		}
	}

	/// Converts a semantic tree to its string representation
	pub fn to_string(self: &TypedSyntaxTree) -> String {
		self.node_to_string(self.root)
	}

	pub fn parameters_to_string(
		self: &TypedSyntaxTree,
		parameters: &Option<usize>
	) -> String {
		if let Some(parameters) = parameters {
			let node: &Node = &self.nodes[*parameters];
			match node {
				Node::Tuple { items, .. } =>
					items
						.iter()
						.filter_map(|e| {
							let node_str = self.node_to_string(*e);
							if node_str.is_empty() {
								None
							} else {
								Some(node_str)
							}
						})
						.collect::<Vec<String>>()
						.join(", "),
				_ => self.node_to_string(*parameters),
			}
		} else {
			String::new()
		}
	}
}
