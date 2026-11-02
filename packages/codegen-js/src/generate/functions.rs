//! Functions, lambdas and calls (types brief §4.2.4).
//!
//! A `function` declaration is a JavaScript function declaration, a lambda an
//! arrow function. Defaults are JavaScript default parameters (evaluated at
//! each call that omits the argument, after the parameters before them, as
//! in Fa); named arguments are put in parameter order at compile time, an
//! omitted middle argument passing `undefined`; a variadic parameter is a
//! rest parameter.
//!
//! A parameter that may hold `none` and has a default can't be a JavaScript
//! default parameter, since `none` is `undefined` (design/decisions.md): its
//! default applies when the argument is left out (`arguments.length`) or is
//! `faOmitted`, which a call passes for it when a later argument is given.

use fa_analyzer::analysis::{Argument, Callee, FnInfo};
use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::TypeKind;

use crate::generate::{Dest, Generator};
use crate::js::{self, Expr, Prec, Writer};

/// A function generated: its parameter list and its body.
struct FunctionParts {
	/// `(a, b = 1, ...rest)`
	parameters: String,
	/// The body statements, indented one level more than the function.
	body: Writer,
	/// A body that is one expression without statements (arrow functions only).
	expression: Option<Expr>,
}

impl Generator<'_> {
	/// `function name = …`: a function declaration (or a `const` for an alias).
	pub(crate) fn function_declaration(&mut self, n: NodeId) {
		let View::Function { value, .. } = self.view(n) else {
			return;
		};
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return;
		};
		let name = self.declare(symbol);
		let info = self
			.analysis
			.function(n)
			.or_else(|| self.analysis.function(value))
			.cloned();
		match (self.view(value), info) {
			(View::Lambda { .. }, Some(info)) => {
				let indent = self.out.indentation();
				let parts = self.function_parts(&info, indent, false);
				self.out
					.line(format!("function {name}{} {{", parts.parameters));
				self.out.append(parts.body);
				self.out.line("}");
			}
			(View::Name(_), _) => {
				let alias = self.expr(value);
				self.out.line(format!("const {name} = {};", alias.text));
			}
			_ => {
				self.unsupported(n, "this function declaration");
			}
		}
	}

	/// A lambda: an arrow function.
	pub(crate) fn lambda(&mut self, n: NodeId) -> Expr {
		let Some(info) = self.analysis.function(n).cloned() else {
			return self.unsupported(n, "this function");
		};
		let indent = self.out.indentation();
		let parts = self.function_parts(&info, indent, true);
		match parts.expression {
			Some(expression) => {
				let body = if expression.text.starts_with('{') {
					format!("({})", expression.text)
				} else {
					expression.at(Prec::Assign)
				};
				Expr::new(format!("{} => {body}", parts.parameters), Prec::Assign)
			}
			None => {
				let closing = "\t".repeat(indent);
				let body = parts.body.finish();
				Expr::new(
					format!("{} => {{\n{body}{closing}}}", parts.parameters),
					Prec::Assign,
				)
			}
		}
	}

	/// The parameters and the body of a function or lambda, generated as a
	/// new JavaScript function at `indent`.
	fn function_parts(&mut self, info: &FnInfo, indent: usize, arrow: bool) -> FunctionParts {
		let ret = info
			.sig
			.map_or(TypeId::UNKNOWN, |sig| self.types.sig(sig).ret);
		let dest = if ret == TypeId::VOID {
			Dest::Discard
		} else {
			Dest::Return
		};
		let (parts, temps) = self.in_function(|g| {
			let mut parameters = Vec::new();
			let mut prologue = Writer::at(indent + 1);
			for param in &info.params {
				// a destructured parameter: a temporary, its pattern bound in the body
				if let View::DestructuredParameter { pattern, .. } = g.view(param.node) {
					let name = g.temp();
					g.declare_as(param.symbol, &name);
					let (lines, ()) = g.capture_at(indent + 1, |g| {
						g.bind_pattern(pattern, Expr::name(name.clone(), true), false, false)
					});
					prologue.append(lines);
					parameters.push(name);
					continue;
				}
				let name = g.declare(param.symbol);
				if param.variadic {
					parameters.push(format!("...{name}"));
					continue;
				}
				let Some(default) = param.default else {
					parameters.push(name);
					continue;
				};
				let (lines, value) = g.capture_at(indent + 2, |g| g.expr(default));
				if g.omitted_by_sentinel(param.symbol) {
					let omitted = g.helper("faOmitted");
					let test = if arrow {
						format!("{name} === {omitted}")
					} else {
						format!(
							"arguments.length < {} || {name} === {omitted}",
							parameters.len() + 1
						)
					};
					prologue.open(format!("if ({test})"));
					prologue.append(lines);
					prologue.line(format!("{name} = {};", value.text));
					prologue.close("");
					parameters.push(name);
				} else if lines.is_empty() && prologue.is_empty() {
					parameters.push(format!("{name} = {}", value.at(Prec::Assign)));
				} else {
					// a default that needs statements: in the body
					prologue.open(format!("if ({name} === undefined)"));
					prologue.append(lines);
					prologue.line(format!("{name} = {};", value.text));
					prologue.close("");
					parameters.push(name);
				}
			}
			let parameters = format!("({})", parameters.join(", "));
			let mut expression = None;
			let mut body = prologue;
			if arrow && !info.block_body && body.is_empty() {
				let (lines, value) = g.capture_at(indent + 1, |g| g.expr(info.body));
				if lines.is_empty() {
					expression = Some(value);
				} else {
					body.append(lines);
					g.out = std::mem::replace(&mut body, Writer::new());
					let ty = g.type_of(info.body);
					g.finish(value, &dest, ty);
					body = std::mem::take(&mut g.out);
				}
			} else {
				let saved = std::mem::replace(&mut g.out, body);
				if info.block_body {
					if let View::Block { statements } = g.view(info.body) {
						let statements: Vec<NodeId> = statements.iter().collect();
						g.statements(&statements, dest.clone());
					}
				} else {
					g.into(info.body, dest.clone());
				}
				body = std::mem::replace(&mut g.out, saved);
			}
			FunctionParts {
				parameters,
				body,
				expression,
			}
		});
		let mut parts = parts;
		if !temps.is_empty() {
			if let Some(expression) = parts.expression.take() {
				// an arrow function with temporaries gets a block body
				let mut body = Writer::at(indent + 1);
				body.line(format!("return {};", expression.text));
				parts.body = body;
			}
			let mut declaration = Writer::at(indent + 1);
			declaration.line(format!("let {};", temps.join(", ")));
			declaration.append(parts.body);
			parts.body = declaration;
		}
		parts
	}

	/// A call.
	pub(crate) fn call(&mut self, n: NodeId, callee: NodeId, arguments: NodeList<'_>) -> Expr {
		let Some(info) = self.analysis.call(n).cloned() else {
			// `x.abs()`, typed by the analyzer without a call entry
			if let View::Access { target, members } = self.view(callee)
				&& members.len() == 1
				&& arguments.is_empty()
				&& members
					.first()
					.is_some_and(|member| self.text(member) == "abs")
			{
				return self.abs(n, target);
			}
			return self.unsupported(n, "this call");
		};
		match info.callee {
			Callee::Conversion(target) => match info.arguments.first() {
				Some(Argument::Given(value)) => self.conversion(n, target, *value),
				_ => self.unsupported(n, "this conversion"),
			},
			Callee::Builtin(builtin) => self.builtin_call(n, callee, builtin, &info.arguments),
			Callee::Member(member) => match info.arguments.first() {
				Some(Argument::Given(value)) => self.member_with_data(member, *value),
				_ => self.unsupported(n, "this member"),
			},
			Callee::Function(declaration)
				if info
					.arguments
					.iter()
					.any(|argument| matches!(argument, Argument::Receiver(_))) =>
			{
				// a method: `Type$method(receiver, …)`
				let Some(symbol) = self.analysis.symbols.symbol_of_decl(declaration) else {
					return self.unsupported(n, "this method");
				};
				let function = self.name_of(symbol);
				let receiver = self.receiver(callee);
				let rest: Vec<Argument> = info
					.arguments
					.iter()
					.filter(|argument| !matches!(argument, Argument::Receiver(_)))
					.cloned()
					.collect();
				let optional = matches!(self.view(callee), View::OptionalAccess { .. });
				let receiver = if optional {
					self.keep_value(receiver)
				} else {
					receiver
				};
				let omitted = self.sentinel_slots(declaration, 1);
				let (receiver, mut values) = self.arguments(receiver, &rest, arguments, &omitted);
				values.insert(0, receiver.text.clone());
				let result = js::call(&function, &values);
				if !optional {
					return result;
				}
				// `receiver?.method()`: `none` when the receiver is `none`
				let none = Expr::constant("undefined", Prec::Primary);
				let test = js::binary(&receiver, "===", &none, Prec::Equality);
				js::conditional(&test, &none, &result)
			}
			Callee::Function(_) | Callee::Value => {
				let omitted = match info.callee {
					Callee::Function(declaration) => self.sentinel_slots(declaration, 0),
					_ => Vec::new(),
				};
				let function = self.expr(callee);
				let (function, values) =
					self.arguments(function, &info.arguments, arguments, &omitted);
				let callee_text = if matches!(self.view(self.strip_groups(callee)), View::Name(_)) {
					function.text
				} else {
					function.primary_text()
				};
				js::call(&callee_text, &values)
			}
		}
	}

	/// Whether a parameter's default can't be a JavaScript default parameter:
	/// it may hold `none` (`undefined`), so that an explicit `none` must not
	/// be taken for an omitted argument.
	pub(crate) fn omitted_by_sentinel(&self, parameter: SymbolId) -> bool {
		self.may_be_none(self.analysis.symbols.symbol(parameter).ty)
	}

	/// Whether values of a type may be `none`.
	pub(crate) fn may_be_none(&self, ty: TypeId) -> bool {
		match self.types.kind(ty) {
			TypeKind::None | TypeKind::Unknown | TypeKind::Param(_) => true,
			TypeKind::Union(list) => self
				.types
				.list_items(list)
				.iter()
				.any(|&member| self.may_be_none(member)),
			_ => false,
		}
	}

	/// Per argument of a call of a function declaration (after `skip`
	/// parameters: the receiver of a method), whether an omitted argument is
	/// passed as `faOmitted`.
	fn sentinel_slots(&self, declaration: NodeId, skip: usize) -> Vec<bool> {
		let Some(info) = self.analysis.function(declaration) else {
			return Vec::new();
		};
		info.params
			.iter()
			.skip(skip)
			.map(|param| param.default.is_some() && self.omitted_by_sentinel(param.symbol))
			.collect()
	}

	/// The arguments of a call in parameter order, evaluated in the order they
	/// are written, after `first` (the callee or the receiver). `omitted`
	/// says, per argument, whether an omitted one is passed as `faOmitted`
	/// (otherwise `undefined`) when a later one is given.
	fn arguments(
		&mut self,
		first: Expr,
		arguments: &[Argument],
		written: NodeList<'_>,
		omitted: &[bool],
	) -> (Expr, Vec<String>) {
		// the value nodes in parameter order
		let mut slots: Vec<Slot> = Vec::new();
		for (position, argument) in arguments.iter().enumerate() {
			match argument {
				Argument::Given(value) => slots.push(Slot::Value(*value)),
				Argument::Default | Argument::Missing | Argument::Receiver(_) => slots.push(
					Slot::Omitted(omitted.get(position).copied().unwrap_or(false)),
				),
				Argument::Variadic(values) => {
					for &value in values {
						slots.push(Slot::Value(value));
					}
				}
			}
		}
		// the order in which they are written
		let position = |node: NodeId| -> usize {
			let start = self.tree.spans[node.index()].start;
			written
				.iter()
				.position(|argument| {
					let span = self.tree.spans[argument.index()];
					span.start <= start && start < span.end.max(span.start + 1)
				})
				.unwrap_or(usize::MAX)
		};
		let mut order: Vec<(usize, usize)> = slots
			.iter()
			.enumerate()
			.filter_map(|(index, slot)| match slot {
				Slot::Value(node) => Some((position(*node), index)),
				Slot::Omitted(_) => None,
			})
			.collect();
		order.sort();
		let nodes: Vec<NodeId> = order
			.iter()
			.map(|&(_, index)| match slots[index] {
				Slot::Value(node) => node,
				Slot::Omitted(_) => unreachable!(),
			})
			.collect();
		let (lines, values) = self.capture(|g| g.in_order(&nodes, |g, node| g.argument(node)));
		let mut first = first;
		if !lines.is_empty() && !first.stable {
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", first.text));
			first = Expr::name(temp, true);
		}
		self.out.append(lines);
		let mut by_slot: Vec<Option<Expr>> = vec![None; slots.len()];
		let reordered = order.windows(2).any(|pair| pair[0].1 > pair[1].1);
		for (&(_, index), value) in order.iter().zip(values) {
			by_slot[index] = Some(value);
		}
		if reordered {
			// keep the written order of the values with effects: temporaries
			for &(_, index) in &order {
				let value = by_slot[index].take().expect("a value");
				if value.stable {
					by_slot[index] = Some(value);
				} else {
					let temp = self.temp();
					self.out.line(format!("const {temp} = {};", value.text));
					by_slot[index] = Some(Expr::name(temp, true));
				}
			}
		}
		let last = by_slot.iter().rposition(Option::is_some);
		let sentinel = slots
			.iter()
			.enumerate()
			.any(|(index, slot)| {
				matches!(slot, Slot::Omitted(true)) && last.is_some_and(|last| index < last)
			})
			.then(|| self.helper("faOmitted"));
		let values = by_slot
			.into_iter()
			.zip(&slots)
			.enumerate()
			.filter(|(index, _)| last.is_some_and(|last| *index <= last))
			.map(|(_, (value, slot))| match (value, slot) {
				(Some(value), _) => value.text,
				(None, Slot::Omitted(true)) => sentinel.unwrap_or("undefined").to_string(),
				(None, _) => "undefined".to_string(),
			})
			.collect();
		(first, values)
	}

	/// One argument value: `...collection` for a spread.
	fn argument(&mut self, node: NodeId) -> Expr {
		match self.view(node) {
			View::Spread(value) => {
				let value = self.expr(value);
				Expr::new(format!("...{}", value.at(Prec::Assign)), Prec::Assign)
			}
			_ => self.expr(node),
		}
	}

	/// A call of a built-in function or method.
	fn builtin_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		builtin: fa_analyzer::ids::BuiltinId,
		arguments: &[Argument],
	) -> Expr {
		let path = builtins::get(builtin).path;
		match path {
			"console.log" | "console.error" => {
				let values: Vec<NodeId> = arguments
					.iter()
					.flat_map(|argument| match argument {
						Argument::Given(value) => vec![*value],
						Argument::Variadic(values) => values.clone(),
						_ => Vec::new(),
					})
					.collect();
				let method = path.trim_start_matches("console.");
				self.console(method, &values)
			}
			"Integer.parse" => {
				let values = self.simple_arguments(arguments);
				let helper = self.helper("faParseInteger");
				js::call(helper, &values)
			}
			"String.compareLocale" => {
				let values = self.simple_arguments(arguments);
				let helper = self.helper("faStringCompareLocale");
				js::call(helper, &values)
			}
			"Array#push" => {
				let receiver = self.receiver(callee);
				let nodes: Vec<NodeId> = arguments
					.iter()
					.filter_map(|argument| match argument {
						Argument::Given(value) => Some(*value),
						_ => None,
					})
					.collect();
				let (lines, values) = self.capture(|g| {
					g.in_order(&nodes, |g, node| {
						let value = g.expr(node);
						g.copy_stored(node, value)
					})
				});
				let receiver = self.keep_before(receiver, &lines);
				self.out.append(lines);
				let values: Vec<String> = values.into_iter().map(|value| value.text).collect();
				Expr::primary(format!(
					"{}.push({})",
					receiver.primary_text(),
					values.join(", ")
				))
			}
			"Array.from" | "Set.from" | "Map.from" => self.collection_from(n, path, arguments),
			_ if path.starts_with("Set#") || path.starts_with("Map#") => {
				let receiver = self.receiver(callee);
				let nodes: Vec<NodeId> = arguments
					.iter()
					.filter_map(|argument| match argument {
						Argument::Given(value) => Some(*value),
						_ => None,
					})
					.collect();
				let (lines, values) = self.capture(|g| {
					g.in_order(&nodes, |g, node| {
						let value = g.expr(node);
						g.copy_stored(node, value)
					})
				});
				let receiver = self.keep_before(receiver, &lines);
				self.out.append(lines);
				self.collection_method(n, path, receiver, values)
			}
			_ if path.starts_with("Array#") => self.array_method(n, callee, path, arguments),
			_ if crate::generate::number_api::is_number_api(path) => {
				self.number_api_call(n, callee, path, arguments)
			}
			"Enum#toString" => {
				let receiver = self.receiver(callee);
				match self.types.kind(self.receiver_type(callee)) {
					fa_analyzer::types::TypeKind::Enum(nominal, _)
					| fa_analyzer::types::TypeKind::EnumMember(nominal, _) => {
						let names = self.enum_names(nominal);
						Expr::primary(format!("{names}[{}]", receiver.text))
					}
					_ => self.unsupported(n, "`toString`"),
				}
			}
			_ if path.starts_with("String#") => {
				let View::Access { .. } = self.view(callee) else {
					return self.unsupported(n, &format!("`{path}`"));
				};
				let receiver = self.receiver(callee);
				let nodes: Vec<Option<NodeId>> = arguments
					.iter()
					.map(|argument| match argument {
						Argument::Given(value) => Some(*value),
						_ => None,
					})
					.collect();
				let given: Vec<NodeId> = nodes.iter().flatten().copied().collect();
				let (lines, values) = self.capture(|g| g.in_order(&given, |g, node| g.expr(node)));
				let mut receiver = receiver;
				if !lines.is_empty() && !receiver.stable {
					let temp = self.temp();
					self.out.line(format!("const {temp} = {};", receiver.text));
					receiver = Expr::name(temp, true);
				}
				self.out.append(lines);
				let mut values = values.into_iter();
				let arguments = nodes
					.iter()
					.map(|node| node.map(|_| values.next().expect("a value")))
					.collect();
				self.string_method(n, path, receiver, arguments)
			}
			_ => self.unsupported(n, &format!("`{path}`")),
		}
	}

	/// The arguments of a built-in function: given ones, in order; omitted
	/// trailing ones left out.
	fn simple_arguments(&mut self, arguments: &[Argument]) -> Vec<String> {
		let nodes: Vec<NodeId> = arguments
			.iter()
			.map_while(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		self.in_order(&nodes, |g, node| g.expr(node))
			.into_iter()
			.map(|value| value.text)
			.collect()
	}

	/// `target.a.b` without its last member (the receiver of a method call).
	fn access_prefix(&mut self, callee: NodeId, target: NodeId, members: NodeList<'_>) -> Expr {
		let members: Vec<NodeId> = members.iter().collect();
		let prefix = &members[..members.len().saturating_sub(1)];
		self.access_chain(callee, target, prefix)
	}

	/// The receiver of a method call whose callee is `receiver.method` (or
	/// `receiver?.method`: the receiver before `?.`).
	pub(crate) fn receiver(&mut self, callee: NodeId) -> Expr {
		match self.view(callee) {
			View::Access { target, members } => self.access_prefix(callee, target, members),
			View::OptionalAccess { target, members } if members.len() == 1 => self.expr(target),
			_ => self.unsupported(callee, "this method call"),
		}
	}

	/// The type of the receiver of a method call.
	pub(crate) fn receiver_type(&self, callee: NodeId) -> TypeId {
		match self.view(callee) {
			View::Access { target, members } | View::OptionalAccess { target, members } => {
				match members.len() {
					0 | 1 => self.type_of(target),
					count => {
						let member = members.get(count - 2).expect("a member");
						let ty = self.type_of(member);
						match self.analysis.resolution(member) {
							// a float constant (`Float.nan`): its member node has no type
							fa_analyzer::symbols::Resolution::Builtin(builtin)
								if ty.is_unset()
									&& builtins::get(builtin).path.starts_with("Float") =>
							{
								TypeId::FLOAT
							}
							_ => ty,
						}
					}
				}
			}
			_ => TypeId::UNSET,
		}
	}

	/// A method of an array (types brief §5.1): JavaScript's own where its
	/// semantics are Fa's, a runtime helper otherwise. A function passed to
	/// `map`, `filter`… that is not a lambda is wrapped, so that it doesn't
	/// receive JavaScript's extra arguments (index, array).
	fn array_method(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Expr {
		let receiver = self.receiver(callee);
		let nodes: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		let (lines, values) = self.capture(|g| {
			g.in_order(&nodes, |g, node| {
				let value = g.expr(node);
				g.copy_stored(node, value)
			})
		});
		let receiver = self.keep_before(receiver, &lines);
		self.out.append(lines);
		let callback = |g: &Self, index: usize, arity: usize| -> String {
			let value = &values[index];
			// a lambda, or a declared function without defaults nor variadic
			// parameters: JavaScript's extra arguments (the array) are ignored
			if matches!(g.view(g.strip_groups(nodes[index])), View::Lambda { .. })
				|| g.plain_function(nodes[index])
			{
				return value.text.clone();
			}
			let parameters: Vec<String> = (0..arity).map(|i| format!("$p{i}")).collect();
			format!(
				"({}) => {}({})",
				parameters.join(", "),
				value.primary_text(),
				parameters.join(", ")
			)
		};
		let target = receiver.primary_text();
		let method = |name: &str, arguments: &[String]| {
			Expr::primary(format!("{target}.{name}({})", arguments.join(", ")))
		};
		let helper = |g: &mut Self, name: &'static str, rest: Vec<String>| {
			let name = g.helper(name);
			let mut all = vec![receiver.text.clone()];
			all.extend(rest);
			js::call(name, &all)
		};
		let texts: Vec<String> = values.iter().map(|value| value.text.clone()).collect();
		match path {
			"Array#get" => helper(self, "faArrayGet", texts),
			"Array#isEmpty" => Expr::new(format!("{target}.length === 0"), Prec::Equality),
			"Array#pop" => method("pop", &[]),
			"Array#clear" => Expr::new(format!("{target}.length = 0"), Prec::Assign),
			"Array#reverse" => Expr::primary(format!("{target}.slice().reverse()")),
			"Array#reverseInPlace" => method("reverse", &[]),
			"Array#insert" => helper(self, "faArrayInsert", texts),
			"Array#delete" => helper(self, "faArrayDelete", texts),
			"Array#take" => helper(self, "faArrayTake", texts),
			"Array#includes" => helper(self, "faArrayIncludes", texts),
			"Array#indexOf" => helper(self, "faArrayIndexOf", texts),
			"Array#map" | "Array#filter" | "Array#find" | "Array#some" | "Array#every" => {
				let name = path.trim_start_matches("Array#");
				// `(item, index)`: a function may leave out the index (`FnSig::ignorable`)
				let function = callback(self, 0, 2);
				method(name, &[function])
			}
			"Array#deleteWhere" => {
				let function = callback(self, 0, 1);
				helper(self, "faArrayDeleteWhere", vec![function])
			}
			"Array#reduce" => {
				let function = callback(self, 0, 3);
				let initial = texts.get(1).cloned().unwrap_or_default();
				method("reduce", &[function, initial])
			}
			"Array#sort" | "Array#sortInPlace" => {
				let name = if path == "Array#sort" {
					"faArraySort"
				} else {
					"faArraySortInPlace"
				};
				let rest = if values.is_empty() {
					Vec::new()
				} else {
					vec![callback(self, 0, 2)]
				};
				helper(self, name, rest)
			}
			"Array#sum" => {
				let item = match self.types.kind(self.receiver_type(callee)) {
					fa_analyzer::types::TypeKind::Collection(_, item, _) => item,
					_ => TypeId::UNKNOWN,
				};
				match self.kind(item) {
					crate::generate::Kind::Int(storage) if !storage.is_js_bigint() => {
						if self.options.overflow_checks {
							let location = self.location(n);
							helper(self, "faSumIntegers", vec![location])
						} else {
							method("reduce", &["(a, b) => a + b".to_string(), "0".to_string()])
						}
					}
					crate::generate::Kind::Int(_) | crate::generate::Kind::BigInt => {
						method("reduce", &["(a, b) => a + b".to_string(), "0n".to_string()])
					}
					_ => method("reduce", &["(a, b) => a + b".to_string(), "0".to_string()]),
				}
			}
			_ => self.unsupported(n, &format!("`{path}`")),
		}
	}

	/// Whether a node names a function declaration whose parameters are all
	/// plain (no default, no variadic): it can be given to a JavaScript
	/// method that passes extra arguments.
	fn plain_function(&self, node: NodeId) -> bool {
		let node = self.strip_groups(node);
		let Some(symbol) = self.analysis.symbol_of(node) else {
			return false;
		};
		self.analysis
			.function(symbol.decl_node)
			.is_some_and(|info| {
				info.declaration.is_some()
					&& info
						.params
						.iter()
						.all(|param| param.default.is_none() && !param.variadic)
			})
	}

	/// A value used several times: stable, or kept in a temporary.
	fn keep_value(&mut self, value: Expr) -> Expr {
		if value.stable {
			return value;
		}
		let temp = self.temp();
		self.out.line(format!("const {temp} = {};", value.text));
		Expr::name(temp, true)
	}

	/// A value computed before statements that come after it: kept in a
	/// temporary when those statements could change it.
	fn keep_before(&mut self, value: Expr, lines: &Writer) -> Expr {
		if lines.is_empty() || value.stable {
			return value;
		}
		let temp = self.temp();
		self.out.line(format!("const {temp} = {};", value.text));
		Expr::name(temp, true)
	}
}

/// A parameter slot of a call.
#[derive(Copy, Clone)]
enum Slot {
	/// Filled by this value node.
	Value(NodeId),
	/// Omitted: the default applies (`true`: passed as `faOmitted`).
	Omitted(bool),
}
