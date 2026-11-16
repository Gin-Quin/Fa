use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::nodes::{Name, Node, StringPart, WhenBranchPattern};
use crate::source::{SourceMap, SourceSpan};

/// The result of parsing: the nodes, stored by index, with the span of each node in the source.
/// The parser always returns a tree, even for broken code: the parts that could not be parsed
/// are `Node::Invalid`, and `diagnostics` explains every problem.
#[derive(Debug)]
pub struct TypedSyntaxTree {
	// the parsed text, shared with whoever gave it (a language server keeps one per version)
	pub source: Arc<str>,
	pub nodes: Vec<Node>,
	pub spans: Vec<SourceSpan>,
	pub root: usize,
	pub source_map: SourceMap,
	pub diagnostics: Vec<Diagnostic>,
	// the spans of the comments, in source order (see `documentation`)
	pub comments: Vec<SourceSpan>,
}

impl TypedSyntaxTree {
	pub fn new(source: Arc<str>) -> Self {
		let source_map = SourceMap::new(&source);
		TypedSyntaxTree {
			source,
			nodes: Vec::new(),
			spans: Vec::new(),
			root: 0,
			source_map,
			diagnostics: Vec::new(),
			comments: Vec::new(),
		}
	}

	/// Insert a node into the tree and return its index
	pub fn insert(&mut self, node: Node, span: SourceSpan) -> usize {
		self.nodes.push(node);
		self.spans.push(span);
		self.nodes.len() - 1
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

	/// The source text of a node.
	pub fn text(&self, index: usize) -> &str {
		let span = self.spans[index];
		self.source.get(span.start..span.end).unwrap_or("")
	}

	/// The text of a name.
	pub fn name(&self, name: Name) -> &str {
		self.source
			.get(name.start as usize..name.end as usize)
			.unwrap_or("")
	}

	/// The documentation of a declaration: the comments written right above it, without a blank
	/// line in between, with their `--` or `---` markers removed.
	pub fn documentation(&self, index: usize) -> Option<String> {
		let start = self.spans.get(index)?.start;
		let (line, _) = self.source_map.line_col(start)?;
		let position = self
			.comments
			.partition_point(|comment| comment.start < start);
		let mut expected_line = line;
		let mut lines: Vec<&str> = Vec::new();
		for comment in self.comments[..position].iter().rev() {
			let (end_line, _) = self.source_map.line_col(comment.end)?;
			let (start_line, column) = self.source_map.line_col(comment.start)?;
			let line_start = comment.start - column;
			let alone_on_its_line = self.source[line_start..comment.start].trim().is_empty();
			if end_line + 1 != expected_line || !alone_on_its_line {
				break;
			}
			let text = &self.source[comment.start..comment.end];
			lines.push(text.trim_start_matches('-').trim_end_matches('-').trim());
			expected_line = start_line;
		}
		if lines.is_empty() {
			return None;
		}
		lines.reverse();
		Some(lines.join("\n"))
	}

	/// True if the tokenizer or the parser reported an error.
	pub fn has_errors(&self) -> bool {
		self.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.is_error())
	}

	/// Converts a node to its string representation
	pub fn node_to_string(&self, index: usize) -> String {
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
			Node::Invalid => String::from("<invalid>"),
			Node::Placeholder => String::from("..."),

			Node::Identifier { name } => self.name(*name).to_string(),
			Node::ImplicitMember { name } => format!(".{}", self.name(*name)),
			Node::Integer(value) => value.to_string(),
			Node::Number(value) => value.to_string(),
			Node::BigInteger(value) => self.name(*value).to_string(),
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
				is_mutable,
				..
			} => declaration_with_optional_type_and_expression(
				self,
				if *is_mutable { "mutable use" } else { "use" },
				name,
				type_expression,
				&Some(*expression),
			),
			Node::Reuse { name, expression } => {
				declaration_with_expression(self, "reuse", name, expression)
			}
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
				let body_str = self.node_to_string(*body);
				format!("{keyword}{modifier_str} {name_str} {body_str}")
			}
			Node::Type {
				name,
				parameters,
				angle_brackets,
				expression,
			} => {
				if parameters.is_empty() {
					declaration_with_expression(self, "type", name, expression)
				} else {
					let expression_str = self.node_to_string(*expression);
					let parameters_str =
						self.generic_parameters_to_string(parameters, *angle_brackets);
					format!(
						"type {}{parameters_str} = {expression_str}",
						self.name(*name)
					)
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
				is_mutable,
				type_expression,
				expression,
			} => {
				let mut string = String::from(if *is_mutable {
					"export mutable"
				} else {
					"export"
				});
				if let Some(type_expression) = type_expression {
					string += ": ";
					string += &self.node_to_string(*type_expression);
				}
				string += " = ";
				string += &self.node_to_string(*expression);
				string
			}
			Node::ExportFunction {
				is_async,
				parameters,
				angle_brackets,
				type_expression,
				expression,
			} => {
				let mut string = String::from(if *is_async {
					"export async function"
				} else {
					"export function"
				});
				string += &self.generic_parameters_to_string(parameters, *angle_brackets);
				if let Some(type_expression) = type_expression {
					string += ": ";
					string += &self.node_to_string(*type_expression);
				}
				string += " = ";
				string += &self.node_to_string(*expression);
				string
			}
			Node::ExportType {
				parameters,
				angle_brackets,
				expression,
			} => {
				let expression_str = self.node_to_string(*expression);
				let parameters_str = self.generic_parameters_to_string(parameters, *angle_brackets);
				format!("export type{parameters_str} = {expression_str}")
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
			Node::Copy { right } => Prefix!("copy ", right),
			Node::Start { right } => Prefix!("start ", right),
			Node::Catch {
				expression,
				error_name,
				handler,
			} => {
				let expression_str = self.node_to_string(*expression);
				let handler_str = self.node_to_string(*handler);
				let error_name = self.name(*error_name);
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
						header += self.name(*index);
					}
					header += " in";
				}
				let iterable_str = self.node_to_string(*iterable);
				let body_str = self.node_to_string(*body);
				format!("{header} {iterable_str} {body_str}")
			}
			Node::WhileLet {
				pattern,
				expression,
				body,
				..
			} => {
				let pattern_str = self.node_to_string(*pattern);
				let expression_str = self.node_to_string(*expression);
				let body_str = self.node_to_string(*body);
				format!("while let {pattern_str} = {expression_str} {body_str}")
			}
			Node::While {
				expression, body, ..
			} => {
				let expression_str = self.node_to_string(*expression);
				let body_str = self.node_to_string(*body);
				format!("while {expression_str} {body_str}")
			}
			Node::Loop { body } => format!("loop {}", self.node_to_string(*body)),
			Node::Watch { body } => format!("watch {}", self.node_to_string(*body)),
			Node::Do { body } => format!("do {}", self.node_to_string(*body)),
			Node::Block { statements, .. } => {
				let content = ListWithoutParenthesis!("\n\t", statements);
				format!("{{\n\t{content}\n}}")
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
				let then_str = self.node_to_string(*then_body);
				let mut string = format!("{if_keyword} {condition_str} {then_str}");
				if let Some(else_body) = else_body {
					let else_str = self.node_to_string(*else_body);
					string += &format!(" {else_keyword} {else_str}");
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
						let value = self.node_to_string(branch.value);
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
			Node::Comparison {
				operands,
				operators,
			} => {
				let mut string = String::from("(");
				for (index, operand) in operands.iter().enumerate() {
					if index > 0 {
						string += " ";
						string += operators
							.get(index - 1)
							.map_or("?", |operator| operator.symbol());
						string += " ";
					}
					string += &self.node_to_string(*operand);
				}
				string += ")";
				string
			}
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
			Node::GenericApplication { target, arguments } => {
				let target_str = self.node_to_string(*target);
				let arguments_str = self.items_to_string(arguments, ", ");
				format!("{target_str}<{arguments_str}>")
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
				angle_brackets,
				type_expression,
				expression,
			} => {
				let keyword = if *is_async {
					"async function"
				} else {
					"function"
				};
				let mut string = format!("{keyword} {}", self.name(*name));
				string += &self.generic_parameters_to_string(parameters, *angle_brackets);
				if let Some(type_expression) = type_expression {
					string += ": ";
					string += &self.node_to_string(*type_expression);
				}
				string += " = ";
				string += &self.node_to_string(*expression);
				string
			}

			Node::ArrowFunction { signature, body } => {
				let crate::nodes::FunctionSignature {
					is_async,
					generics,
					parameters,
					return_type_expression,
					errors,
					requirements,
					mutations,
					invalidations,
				} = &**signature;
				let mut signature_str = format!(
					"{}{}({})",
					if *is_async { "async " } else { "" },
					self.generic_parameters_to_string(generics, true),
					self.items_to_string(parameters, ", ")
				);
				if let Some(return_type_expression) = return_type_expression {
					signature_str += ": ";
					signature_str += &self.node_to_string(*return_type_expression);
				}
				if let Some(errors) = errors {
					signature_str += " throws ";
					signature_str += &self.node_to_string(*errors);
				}
				if !requirements.is_empty() {
					signature_str += " requires ";
					signature_str += &self.items_to_string(requirements, ", ");
				}
				if !mutations.is_empty() {
					signature_str += " mutates ";
					signature_str += &self.items_to_string(mutations, ", ");
				}
				if !invalidations.is_empty() {
					signature_str += " invalidates ";
					signature_str += &self.items_to_string(invalidations, ", ");
				}
				let body_str = self.node_to_string(*body);
				format!("{signature_str} => {body_str}")
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
			Node::ObjectPattern { fields, rest } => {
				let mut fields_str = fields
					.iter()
					.map(|field| {
						let mut string = self.name(field.name).to_string();
						if let Some(alias) = field.alias {
							string += " as ";
							string += self.name(alias);
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
					.collect::<Vec<String>>();
				if let Some(rest) = rest {
					fields_str.push(format!("...{}", self.node_to_string(*rest)));
				}
				format!("{{ {} }}", fields_str.join(", "))
			}
			Node::ArrayPattern { items, rest } => {
				let mut items_str = self.items_to_string(items, ", ");
				if let Some(rest) = rest {
					if !items_str.is_empty() {
						items_str += ", ";
					}
					items_str += &format!("...{}", self.node_to_string(*rest));
				}
				format!("[{items_str}]")
			}
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
			Node::Link { right } => format!("link {}", self.node_to_string(*right)),
			Node::Readonly { right } => {
				let right_str = self.node_to_string(*right);
				format!("readonly {right_str}")
			}
		}
	}

	/// Converts a list of nodes to their string representations, joined by a separator
	/// The generic parameters of a declaration, like `<Item>` (or `(Item)`, the parenthesized
	/// spelling of the transition), or nothing.
	fn generic_parameters_to_string(&self, parameters: &[usize], angle_brackets: bool) -> String {
		if parameters.is_empty() {
			return String::new();
		}
		let items = self.items_to_string(parameters, ", ");
		if angle_brackets {
			format!("<{items}>")
		} else {
			format!("({items})")
		}
	}

	pub fn items_to_string(&self, items: &[usize], separator: &str) -> String {
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

/// The tree rendered as code, with parentheses around every operation, for tests and debugging.
impl std::fmt::Display for TypedSyntaxTree {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str(&self.node_to_string(self.root))
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

fn declaration_with_optional_type_and_expression(
	tree: &TypedSyntaxTree,
	keyword: &str,
	name: &Name,
	type_expression: &Option<usize>,
	expression: &Option<usize>,
) -> String {
	let mut string = format!("{keyword} {}", tree.name(*name));
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
	name: &Name,
	expression: &usize,
) -> String {
	let expression_str = tree.node_to_string(*expression);
	format!("{keyword} {} = {expression_str}", tree.name(*name))
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
