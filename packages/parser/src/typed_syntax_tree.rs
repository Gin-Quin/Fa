use crate::nodes::{
	FunctionBody, IfElseBody, Node, StringPart, WhenBranchPattern, WhenBranchValue,
};
use crate::source::{SourceMap, SourceSpan};

#[derive(Debug)]
pub struct TypedSyntaxTree {
	pub input: &'static str,
	pub nodes: Vec<Node>,
	pub spans: Vec<SourceSpan>,
	pub root: usize,
	pub source_map: SourceMap,
}

impl TypedSyntaxTree {
	pub fn new(input: &'static str) -> Self {
		TypedSyntaxTree {
			input,
			nodes: Vec::new(),
			spans: Vec::new(),
			root: 0,
			source_map: SourceMap::new(input),
		}
	}

	/// Insert a node into the tree and return its index
	pub fn insert(&mut self, node: Node, span: SourceSpan) -> usize {
		self.nodes.push(node);
		self.spans.push(span);
		self.nodes.len() - 1
	}

	pub fn remove(&mut self, index: usize) -> Node {
		self.spans.remove(index);
		self.nodes.remove(index)
	}

	pub fn at(&self, index: usize) -> &Node {
		&self.nodes[index]
	}

	pub fn at_mutable(&mut self, index: usize) -> &mut Node {
		&mut self.nodes[index]
	}

	pub fn span(&self, index: usize) -> SourceSpan {
		self.spans[index]
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

			Node::Identifier { name, .. } => name.to_string(),
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
			Node::None => String::from("none"),
			Node::Error => String::from("Error"),
			Node::Boolean(value) => value.to_string(),

			Node::Not { right, .. } => Prefix!("not ", right),
			Node::Negate { right, .. } => Prefix!("-", right),
			Node::Spread { right, .. } => PrefixWithoutParenthesis!("...", right),

			Node::Let {
				name,
				type_expression,
				expression,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				"let",
				name,
				type_expression,
				expression,
			),
			Node::Mutable {
				name,
				type_expression,
				expression,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				"mutable",
				name,
				type_expression,
				expression,
			),
			Node::Use {
				name,
				type_expression,
				expression,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				"use",
				name,
				type_expression,
				&Some(*expression),
			),
			Node::Static {
				name,
				type_expression,
				expression,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				"static",
				name,
				type_expression,
				&Some(*expression),
			),
			Node::Test {
				name,
				modifier,
				body,
				is_compile_time,
				..
			} => {
				let keyword = if *is_compile_time { "@test" } else { "test" };
				let modifier_str = match modifier {
					Some(crate::nodes::TestModifier::Skip) => " skip",
					Some(crate::nodes::TestModifier::Only) => " only",
					None => "",
				};
				let name_str = self.node_to_string(*name);
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("{keyword}{modifier_str} {name_str} {{\n\t{body_str}\n}}")
			}
			Node::Type {
				name,
				parameters,
				expression,
				..
			} => {
				if parameters.is_empty() {
					declaration_with_expression(self, "type", name, expression)
				} else {
					let expression_str = self.node_to_string(*expression);
					let parameters_str = self.items_to_string(parameters, ", ");
					format!("type {name}({parameters_str}) = {expression_str}")
				}
			}
			Node::Reactive {
				name,
				type_expression,
				expression,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				"reactive",
				name,
				type_expression,
				expression,
			),
			Node::Derived {
				name,
				type_expression,
				expression,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				"derived",
				name,
				type_expression,
				expression,
			),
			Node::Namespace { name, expression } => {
				declaration_with_expression(self, "namespace", name, expression)
			}
			Node::ExportValue {
				type_expression,
				expression,
				..
			} => {
				let mut string = String::from("export");
				if let Some(type_expression) = type_expression {
					string += ": ";
					string += &self.node_to_string(*type_expression);
				}
				string += " = ";
				string += &self.node_to_string(*expression);
				string
			}
			Node::ExportFunction {
				type_expression,
				expression,
				..
			} => {
				let mut string = String::from("export function");
				if let Some(type_expression) = type_expression {
					string += ": ";
					string += &self.node_to_string(*type_expression);
				}
				string += " = ";
				string += &self.node_to_string(*expression);
				string
			}
			Node::ExportType { expression, .. } => {
				let expression_str = self.node_to_string(*expression);
				format!("export type = {expression_str}")
			}
			Node::ExportNamespace { expression, .. } => {
				let expression_str = self.node_to_string(*expression);
				format!("export namespace = {expression_str}")
			}
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
			Node::Try { right } => PrefixWithoutParenthesis!("try ", right),
			Node::Throw { right } => PrefixWithoutParenthesis!("throw ", right),
			Node::Assert { right } => PrefixWithoutParenthesis!("assert ", right),
			Node::Await { right } => Prefix!("await ", right),
			Node::Start { right } => Prefix!("start ", right),
			Node::Catch {
				expression,
				error_name,
				handler,
			} => {
				let expression_str = self.node_to_string(*expression);
				let handler_str = function_body_to_string(self, handler);
				format!("({expression_str} catch {error_name} => {handler_str})")
			}
			Node::Provide {
				type_expression,
				expression,
			} => {
				let type_str = self.node_to_string(*type_expression);
				let expression_str = self.node_to_string(*expression);
				format!("provide {type_str} = {expression_str}")
			}
			Node::For {
				pattern,
				index,
				iterable,
				parallel,
				body,
				is_compile_time,
				..
			} => {
				let mut header = String::from(if *is_compile_time { "@for" } else { "for" });
				if let Some(arguments) = parallel {
					header += " parallel";
					if !arguments.is_empty() {
						header += &format!("({})", self.items_to_string(arguments, ", "));
					}
				}
				if let Some(pattern) = pattern {
					header += " ";
					header += &self.node_to_string(*pattern);
					if let Some(index) = index {
						header += ", ";
						header += index;
					}
					header += " in";
				}
				let iterable_str = self.node_to_string(*iterable);
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("{header} {iterable_str} {{\n\t{body_str}\n}}")
			}
			Node::WhileLet {
				pattern,
				expression,
				body,
				..
			} => {
				let pattern_str = self.node_to_string(*pattern);
				let expression_str = self.node_to_string(*expression);
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("while let {pattern_str} = {expression_str} {{\n\t{body_str}\n}}")
			}
			Node::While {
				expression, body, ..
			} => {
				let expression_str = self.node_to_string(*expression);
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("while {expression_str} {{\n\t{body_str}\n}}")
			}
			Node::Loop { body, .. } => {
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("loop {{\n\t{body_str}\n}}")
			}
			Node::Watch { body, .. } => {
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("watch {{\n\t{body_str}\n}}")
			}
			Node::Do { body, .. } => {
				let body_str = ListWithoutParenthesis!("\n\t", body);
				format!("do {{\n\t{body_str}\n}}")
			}
			Node::If {
				is_compile_time,
				condition,
				then_body,
				else_body,
				..
			} => {
				let (if_keyword, else_keyword) = if *is_compile_time {
					("@if", "@else")
				} else {
					("if", "else")
				};
				let condition_str = self.node_to_string(*condition);
				let then_str = ListWithoutParenthesis!("\n\t", then_body);
				let mut string = format!("{if_keyword} {condition_str} {{\n\t{then_str}\n}}");
				if let Some(else_body) = else_body {
					match else_body {
						IfElseBody::If(index) => {
							let else_if_str = self.node_to_string(*index);
							string += &format!(" {else_keyword} {else_if_str}");
						}
						IfElseBody::Block(body) => {
							let else_str = ListWithoutParenthesis!("\n\t", body);
							string += &format!(" {else_keyword} {{\n\t{else_str}\n}}");
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
						let mut pattern = match branch.pattern {
							WhenBranchPattern::Expression(index) => self.node_to_string(index),
							WhenBranchPattern::Else => String::from("else"),
						};
						if let Some(binding) = branch.binding {
							pattern += " as ";
							pattern += &self.node_to_string(binding);
						}
						let value = match &branch.value {
							WhenBranchValue::Expression(index) => self.node_to_string(*index),
							WhenBranchValue::Block { statements, .. } => {
								format!("{{\n\t{}\n}}", ListWithoutParenthesis!("\n\t", statements))
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
			Node::Remainder { operands, .. } => List!(" %% ", operands),
			Node::Fallback { operands, .. } => List!(" ?? ", operands),
			Node::Power { operands, .. } => List!(" ** ", operands),
			Node::Equal { operands, .. } => List!(" == ", operands),
			Node::NotEqual { operands, .. } => List!(" != ", operands),
			Node::LessThan { operands, .. } => List!(" < ", operands),
			Node::LessThanOrEqual { operands, .. } => List!(" <= ", operands),
			Node::GreaterThan { operands, .. } => List!(" > ", operands),
			Node::GreaterThanOrEqual { operands, .. } => List!(" >= ", operands),
			Node::And { operands, .. } => List!(" and ", operands),
			Node::Or { operands, .. } => List!(" or ", operands),
			Node::Is {
				left,
				right,
				negated,
			} => {
				let operator = if *negated { "is not" } else { "is" };
				Operation!(operator, left, right)
			}
			Node::IsIn {
				left,
				right,
				negated,
			} => {
				let operator = if *negated { "is not in" } else { "is in" };
				Operation!(operator, left, right)
			}
			Node::In { left, right } => Operation!("in", left, right),
			Node::InRelation { right } => PrefixWithoutParenthesis!("in ", right),
			Node::Range {
				start,
				end,
				inclusive,
			} => {
				let start_str = start.map(|start| self.node_to_string(start));
				let end_str = end.map(|end| self.node_to_string(end));
				let operator = if *inclusive { "..=" } else { ".." };
				format!(
					"({}{operator}{})",
					start_str.unwrap_or_default(),
					end_str.unwrap_or_default()
				)
			}
			Node::Union { operands, .. } => List!(" | ", operands),
			Node::Compose { operands, .. } => List!(" then ", operands),
			Node::PartialApplication {
				function,
				arguments,
			} => {
				let function_str = self.node_to_string(*function);
				let arguments_str = self.items_to_string(arguments, ", ");
				format!("({function_str} with ({arguments_str}))")
			}
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
			Node::Tuple { items, .. } => List!(", ", items),
			Node::Members { items, .. } => members_items_to_string(self, items, ""),

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
			Node::MemberIndex {
				target, members, ..
			} => {
				let target_str = self.node_to_string(*target);
				let members_str = members_items_to_string(self, members, "");
				format!("{target_str} {members_str}")
			}
			Node::Index { target, index, .. } => {
				let target_str = self.node_to_string(*target);
				let index_str = self.node_to_string(*index);
				format!("{target_str}[{index_str}]")
			}
			Node::CompoundAssignment {
				operator,
				target,
				expression,
			} => {
				let target_str = self.node_to_string(*target);
				let expression_str = self.node_to_string(*expression);
				format!("{target_str} {} {expression_str}", operator.symbol())
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

			Node::Function {
				name,
				is_async,
				parameters,
				expression,
				..
			} => {
				let keyword = if *is_async {
					"async function"
				} else {
					"function"
				};
				let expression_str = self.node_to_string(*expression);
				if parameters.is_empty() {
					format!("{keyword} {name} = {expression_str}")
				} else {
					let parameters_str = self.items_to_string(parameters, ", ");
					format!("{keyword} {name}({parameters_str}) = {expression_str}")
				}
			}

			Node::ArrowFunction {
				parameters,
				return_type_expression,
				errors,
				requirements,
				mutations,
				invalidations,
				body,
			} => {
				let mut signature = format!("({})", self.items_to_string(parameters, ", "));
				if let Some(return_type_expression) = return_type_expression {
					signature += ": ";
					signature += &self.node_to_string(*return_type_expression);
				}
				if let Some(errors) = errors {
					signature += " throws ";
					signature += &self.node_to_string(*errors);
				}
				if !requirements.is_empty() {
					signature += " requires ";
					signature += &self.items_to_string(requirements, ", ");
				}
				if !mutations.is_empty() {
					signature += " mutates ";
					signature += &self.items_to_string(mutations, ", ");
				}
				if !invalidations.is_empty() {
					signature += " invalidates ";
					signature += &self.items_to_string(invalidations, ", ");
				}
				let body_str = function_body_to_string(self, body);
				format!("{signature} => {body_str}")
			}
			Node::Parameters { items } => format!("({})", self.items_to_string(items, ", ")),
			Node::DestructuredParameter {
				pattern,
				type_expression,
			} => {
				let pattern_str = self.node_to_string(*pattern);
				let type_str = self.node_to_string(*type_expression);
				format!("{pattern_str}: {type_str}")
			}
			Node::ObjectPattern { fields } => {
				let fields_str = fields
					.iter()
					.map(|field| {
						let mut string = field.name.to_string();
						if let Some(alias) = field.alias {
							string += " as ";
							string += alias;
						}
						if let Some(nested) = field.nested {
							string += " ";
							string += &self.node_to_string(nested);
						}
						if let Some(default) = field.default {
							string += " = ";
							string += &self.node_to_string(default);
						}
						string
					})
					.collect::<Vec<String>>()
					.join(", ");
				format!("{{ {fields_str} }}")
			}
			Node::ArrayPattern { items } => format!("[{}]", self.items_to_string(items, ", ")),
			Node::TuplePattern { items } => format!("({})", self.items_to_string(items, ", ")),
			Node::Destructuring {
				kind,
				pattern,
				expression,
			} => {
				let pattern_str = self.node_to_string(*pattern);
				let expression_str = self.node_to_string(*expression);
				format!("{} {pattern_str} = {expression_str}", kind.keyword())
			}
			Node::Throws { signature, errors } => {
				let signature_str = self.node_to_string(*signature);
				let errors_str = self.node_to_string(*errors);
				format!("{signature_str} throws {errors_str}")
			}
			Node::Requires {
				signature,
				requirements,
			} => {
				let signature_str = self.node_to_string(*signature);
				let requirements_str = self.items_to_string(requirements, ", ");
				format!("{signature_str} requires {requirements_str}")
			}
			Node::Effect {
				signature,
				kind,
				paths,
			} => {
				let signature_str = self.node_to_string(*signature);
				let paths_str = self.items_to_string(paths, ", ");
				format!("{signature_str} {} {paths_str}", kind.keyword())
			}
			Node::Readonly { right } => {
				let right_str = self.node_to_string(*right);
				format!("readonly {right_str}")
			}
		}
	}

	/// Converts a semantic tree to its string representation
	pub fn to_string(self: &TypedSyntaxTree) -> String {
		self.node_to_string(self.root)
	}

	/// Converts a list of nodes to their string representations, joined by a separator
	pub fn items_to_string(self: &TypedSyntaxTree, items: &[usize], separator: &str) -> String {
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
			.join(separator)
	}
}

fn members_items_to_string(tree: &TypedSyntaxTree, items: &[usize], prefix: &str) -> String {
	if items.is_empty() {
		format!("{prefix}{{}}")
	} else {
		let content = items
			.iter()
			.filter_map(|e| {
				let node_str = match &tree.nodes[*e] {
					Node::Spread { right, .. } => {
						let inner = tree.node_to_string(*right);
						format!("...{inner}")
					}
					_ => tree.node_to_string(*e),
				};
				if node_str.is_empty() {
					None
				} else {
					let indented = node_str.replace('\n', "\n\t");
					Some(format!("\t{indented}"))
				}
			})
			.collect::<Vec<String>>()
			.join("\n");
		format!("{prefix}{{\n{content}\n}}")
	}
}

fn function_body_to_string(tree: &TypedSyntaxTree, body: &FunctionBody) -> String {
	match body {
		FunctionBody::Expression(expression) => tree.node_to_string(*expression),
		FunctionBody::Block { statements, .. } => {
			let content = statements
				.iter()
				.filter_map(|e| {
					let node_str = tree.node_to_string(*e);
					if node_str.is_empty() {
						None
					} else {
						Some(node_str)
					}
				})
				.collect::<Vec<String>>()
				.join("\n\t");
			format!("{{\n\t{content}\n}}")
		}
	}
}

fn declaration_with_optional_type_and_expression(
	tree: &TypedSyntaxTree,
	keyword: &str,
	name: &str,
	type_expression: &Option<usize>,
	expression: &Option<usize>,
) -> String {
	let mut string = format!("{keyword} {name}");
	if let Some(type_expression) = type_expression {
		string += ": ";
		string += &tree.node_to_string(*type_expression);
	}
	if let Some(expression) = expression {
		string += " = ";
		string += &tree.node_to_string(*expression);
	}
	string
}

fn declaration_with_expression(
	tree: &TypedSyntaxTree,
	keyword: &str,
	name: &str,
	expression: &usize,
) -> String {
	let expression_str = tree.node_to_string(*expression);
	format!("{keyword} {name} = {expression_str}")
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
