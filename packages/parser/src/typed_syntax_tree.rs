use crate::nodes::{
	ArrowFunctionBody, IfElseBody, Node, StringPart, WhenBranchPattern, WhenBranchValue,
};

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

	pub fn at(&self, index: usize) -> &Node {
		&self.nodes[index]
	}

	pub fn at_mutable(&mut self, index: usize) -> &mut Node {
		&mut self.nodes[index]
	}

	/// Converts a node to its string representation
	pub fn node_to_string(self: &TypedSyntaxTree, index: usize) -> String {
		let node = &self.nodes[index];

		macro_rules! Operation {
			($operation:expr, $left:expr, $right:expr) => {{
				let left_str = self.node_to_string(*$left);
				let right_str = self.node_to_string(*$right);
				format!("({} {} {})", left_str, $operation, right_str)
			}};
		}

		macro_rules! List {
			($operation:expr, $operands:expr) => {
				format!(
					"({})",
					$operands
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
						.join($operation)
				)
			};
		}

		macro_rules! ListWithoutParenthesis {
			($operation:expr, $operands:expr) => {
				$operands
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
					.join($operation)
			};
		}

		macro_rules! Prefix {
			($operation:expr, $right:expr) => {{
				let right_str = self.node_to_string(*$right);
				format!("({}{})", $operation, right_str)
			}};
		}

		macro_rules! PrefixWithoutParenthesis {
			($operation:expr, $right:expr) => {{
				let right_str = self.node_to_string(*$right);
				format!("{}{}", $operation, right_str)
			}};
		}

		match node {
			Node::Module { statements, .. } => {
				let content = ListWithoutParenthesis!(";\n", statements);
				if content.is_empty() {
					String::new()
				} else {
					format!("{content};")
				}
			}
			Node::DanglingToken { token, .. } => format!("Dangling {token:#?}"),

			Node::Identifier(value) => value.to_string(),
			Node::Integer(value) => value.to_string(),
			Node::Number(value) => value.to_string(),
			Node::BigInteger(value) => value.to_string(),
			Node::StringLiteral(value) => format!("\"{}\"", escape_string_literal(value, true)),
			Node::StringTemplate { parts } => {
				let mut result = String::from("\"");
				for part in parts {
					match part {
						StringPart::Literal(value) => {
							result.push_str(&escape_string_literal(value, true));
						}
						StringPart::Expression(expression) => {
							let expression_str = self.node_to_string(*expression);
							result.push('{');
							result.push_str(&expression_str);
							result.push('}');
						}
					}
				}
				result.push('"');
				result
			}
			Node::Null => String::from("null"),
			Node::Boolean(value) => value.to_string(),
			Node::True => String::from("true"),
			Node::False => String::from("false"),

			Node::Not { right, .. } => Prefix!("not ", right),
			Node::Negate { right, .. } => Prefix!("-", right),
			Node::Spread { right, .. } => Prefix!("...", right),

			Node::Let { right, .. } => PrefixWithoutParenthesis!("let ", right),
			Node::Mutable { right, .. } => PrefixWithoutParenthesis!("mutable ", right),
			Node::Static { right, .. } => PrefixWithoutParenthesis!("static ", right),
			Node::Type { right, .. } => PrefixWithoutParenthesis!("type ", right),
			Node::UnionDeclaration { right, .. } => PrefixWithoutParenthesis!("union ", right),
			Node::Enum { right, .. } => PrefixWithoutParenthesis!("enum ", right),
			Node::Fields { right, .. } => PrefixWithoutParenthesis!("fields ", right),
			Node::Reactive { right, .. } => PrefixWithoutParenthesis!("reactive ", right),
			Node::Derived { right, .. } => PrefixWithoutParenthesis!("derived ", right),
			Node::Namespace { right, .. } => PrefixWithoutParenthesis!("namespace ", right),
			Node::Return { expression, .. } => {
				if let Some(expression) = expression {
					PrefixWithoutParenthesis!("return ", expression)
				} else {
					String::from("return")
				}
			}
			Node::Break { expression, .. } => {
				if let Some(expression) = expression {
					PrefixWithoutParenthesis!("break ", expression)
				} else {
					String::from("break")
				}
			}
			Node::Continue => String::from("continue"),
			Node::For {
				expression,
				body,
				is_compile_time,
			} => {
				let expression_str = self.node_to_string(*expression);
				let body_str = ListWithoutParenthesis!("\n\t", body);
				let keyword = if *is_compile_time { "@for" } else { "for" };
				format!("{keyword} {expression_str} {{\n\t{body_str}\n}}")
			}
			Node::While { expression, body } => {
				let expression_str = self.node_to_string(*expression);
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("while {expression_str} {{\n\t{body_str}\n}}")
			}
			Node::Loop { body } => {
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("loop {{\n\t{body_str}\n}}")
			}
			Node::If {
				condition,
				then_body,
				else_body,
			} => {
				let condition_str = self.node_to_string(*condition);
				let then_str = ListWithoutParenthesis!("\n\t", then_body);
				let mut string = format!("if {condition_str} {{\n\t{then_str}\n}}");
				if let Some(else_body) = else_body {
					match else_body {
						IfElseBody::If(index) => {
							let else_if_str = self.node_to_string(*index);
							string += &format!(" else {else_if_str}");
						}
						IfElseBody::Block(body) => {
							let else_str = ListWithoutParenthesis!("\n\t", body);
							string += &format!(" else {{\n\t{else_str}\n}}");
						}
					}
				}
				string
			}
			Node::When {
				expression,
				branches,
			} => {
				let expression_str = self.node_to_string(*expression);
				let content = branches
					.iter()
					.map(|branch| {
						let pattern = match branch.pattern {
							WhenBranchPattern::Expression(index) => self.node_to_string(index),
							WhenBranchPattern::Else => String::from("else"),
						};
						let value = match &branch.value {
							WhenBranchValue::Expression(index) => self.node_to_string(*index),
							WhenBranchValue::Block(body) => {
								format!("{{\n\t{}\n}}", ListWithoutParenthesis!("\n\t", body))
							}
						};
						let line = format!("{pattern} => {value}");
						line.replace('\n', "\n\t")
					})
					.map(|line| format!("\t{line}"))
					.collect::<Vec<String>>()
					.join("\n");
				format!("when {expression_str} is {{\n{content}\n}}")
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
			Node::Intersection { operands, .. } => List!(" & ", operands),
			Node::Pipe { operands, .. } => List!(" |> ", operands),
			Node::Compose { operands, .. } => List!(" ||> ", operands),
			Node::Insert { left, right, .. } => Operation!("<<", left, right),
			Node::Extract { left, right, .. } => Operation!(">>", left, right),
			Node::Relation { left, right, .. } => Operation!("->", left, right),
			Node::Access { operands, .. } => ListWithoutParenthesis!(".", operands),
			Node::OptionalAccess { operands, .. } => ListWithoutParenthesis!("?.", operands),
			Node::Percentage { value, .. } => {
				let value_str = self.node_to_string(*value);
				format!("{value_str}%")
			}
			Node::Optional { value, .. } => {
				let value_str = self.node_to_string(*value);
				format!("{value_str}?")
			}
			Node::Assert { value, .. } => {
				let value_str = self.node_to_string(*value);
				format!("{value_str}!")
			}
			Node::List { items, .. } => {
				let content = items
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
					.join(", ");
				format!("[{content}]")
			}
			Node::StaticList { items, .. } => {
				let content = items
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
					.join(", ");
				format!("@[{content}]")
			}
			Node::Tuple { items, .. } => List!(", ", items),
			Node::Members { items, .. } => {
				if items.is_empty() {
					String::from("{}")
				} else {
					let content = items
						.iter()
						.filter_map(|e| {
							let node_str = self.node_to_string(*e);
							if node_str.is_empty() {
								None
							} else {
								let indented = node_str.replace('\n', "\n\t");
								Some(format!("\t{indented}"))
							}
						})
						.collect::<Vec<String>>()
						.join("\n");
					format!("{{\n{content}\n}}")
				}
			}
			Node::StaticMembers { items, .. } => {
				if items.is_empty() {
					String::from("@{}")
				} else {
					let content = items
						.iter()
						.filter_map(|e| {
							let node_str = self.node_to_string(*e);
							if node_str.is_empty() {
								None
							} else {
								let indented = node_str.replace('\n', "\n\t");
								Some(format!("\t{indented}"))
							}
						})
						.collect::<Vec<String>>()
						.join("\n");
					format!("@{{\n{content}\n}}")
				}
			}

			Node::Group { expression, .. } => {
				let expression_str = self.node_to_string(*expression);
				format!("({expression_str})")
			}
			Node::EmptyGroup => String::from("()"),

			Node::FunctionCall {
				function,
				parameters,
				..
			} => {
				let function_str = self.node_to_string(*function);
				let parameters_str = parameters
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
					.join(", ");
				format!("{function_str}({parameters_str})")
			}
			Node::OptionalFunctionCall {
				function,
				parameters,
				..
			} => {
				let function_str = self.node_to_string(*function);
				let parameters_str = parameters
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
					.join(", ");
				format!("{function_str}?({parameters_str})")
			}
			Node::OptionalIndex { target, index, .. } => {
				let target_str = self.node_to_string(*target);
				let index_str = self.node_to_string(*index);
				format!("{target_str}?[{index_str}]")
			}

			Node::Assignment {
				name,
				type_expression,
				expression,
				..
			} => {
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

			Node::Function { name, value, .. } => {
				let mut string: String = String::from("function ");
				string += *name;
				string += " = ";
				string += &self.node_to_string(*value);
				string
			}

			Node::ArrowFunction {
				parameters,
				parenthesized_parameters,
				return_type_expression,
				body,
				..
			} => {
				let mut string = String::new();
				if *parenthesized_parameters || parameters.is_none() {
					string += "(";
					string += &self.parameters_to_string(parameters);
					string += ")";
				} else if let Some(parameters) = parameters {
					string += &self.node_to_string(*parameters);
				} else {
					string += "()";
				}

				if let Some(return_type_expression) = return_type_expression {
					string += ": ";
					string += &self.node_to_string(*return_type_expression);
				}

				string += " => ";
				match body {
					ArrowFunctionBody::Expression(expression) => {
						string += &self.node_to_string(*expression);
					}
					ArrowFunctionBody::Block(statements) => {
						string += "{\n\t";
						string += &ListWithoutParenthesis!("\n\t", statements);
						string += "\n}";
					}
				}
				string
			}
		}
	}

	/// Converts a semantic tree to its string representation
	pub fn to_string(self: &TypedSyntaxTree) -> String {
		self.node_to_string(self.root)
	}

	pub fn parameters_to_string(self: &TypedSyntaxTree, parameters: &Option<usize>) -> String {
		if let Some(parameters) = parameters {
			let node: &Node = &self.nodes[*parameters];
			match node {
				Node::Tuple { items, .. } => items
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

fn escape_string_literal(value: &str, escape_braces: bool) -> String {
	let mut escaped = String::new();
	for character in value.chars() {
		match character {
			'\\' => escaped.push_str("\\\\"),
			'"' => escaped.push_str("\\\""),
			'\n' => escaped.push_str("\\n"),
			'\r' => escaped.push_str("\\r"),
			'\t' => escaped.push_str("\\t"),
			'\0' => escaped.push_str("\\0"),
			'{' if escape_braces => escaped.push_str("{{"),
			'}' if escape_braces => escaped.push_str("}}"),
			_ => escaped.push(character),
		}
	}
	escaped
}
