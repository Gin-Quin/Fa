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

use fa_analyzer::analysis::{Argument, Callee, FnInfo, NodeFlags};
use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::TypeKind;

use crate::generate::tasks::Mode;
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
		let (View::Function { value, .. } | View::ExportFunction { value, .. }) = self.view(n)
		else {
			return;
		};
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return;
		};
		let name = self.declare(symbol);
		// a C function (foreign.rs)
		if let Some(extern_symbol) = self.analysis.externs.get(&n).cloned() {
			self.extern_declaration(n, &name, &extern_symbol);
			return;
		}
		let info = self
			.analysis
			.function(n)
			.or_else(|| self.analysis.function(value))
			.cloned();
		match (self.view(value), info) {
			(View::Lambda { .. }, Some(info)) => {
				let indent = self.out.indentation();
				// an escaping local function copies its captures (references.rs)
				let saved = self.copy_captures(&info);
				// the instances of a function that may suspend (tasks.rs)
				for (mode, instance) in self.instance_modes(n) {
					self.tasks.next_mode = Some(mode);
					let parts = self.function_parts(&info, indent, false);
					let keyword = if matches!(mode, Mode::Async { .. }) {
						"async function"
					} else {
						"function"
					};
					let name = instance.unwrap_or_else(|| name.clone());
					self.out
						.line(format!("{keyword} {name}{} {{", parts.parameters));
					self.out.append(parts.body);
					self.out.line("}");
				}
				// the integer variants (`integer_variants.rs`)
				for variant in self.integer_variants.get(&n).cloned().unwrap_or_default() {
					let suffix = self.variant_suffix(n, &variant);
					self.numeric_bindings = variant.into_iter().collect();
					for (mode, instance) in self.instance_modes(n) {
						self.tasks.next_mode = Some(mode);
						let parts = self.function_parts(&info, indent, false);
						let keyword = if matches!(mode, Mode::Async { .. }) {
							"async function"
						} else {
							"function"
						};
						let name = format!("{}{suffix}", instance.unwrap_or_else(|| name.clone()));
						self.out
							.line(format!("{keyword} {name}{} {{", parts.parameters));
						self.out.append(parts.body);
						self.out.line("}");
					}
					self.numeric_bindings.clear();
				}
				if let Some((name, code)) = self.promise_api(n) {
					self.out.line(format!("const {name} = {code};"));
				}
				// the instance receiving reactive places (D24, reactivity.rs)
				if self.reactive_instances.contains(&n) {
					let params: Vec<SymbolId> =
						info.params.iter().map(|param| param.symbol).collect();
					self.instance_params = Some(params.clone());
					let parts = self.function_parts(&info, indent, false);
					for param in &params {
						self.param_sources.remove(param);
					}
					let name = self.reactive_instance_name(symbol);
					self.out
						.line(format!("function {name}{} {{", parts.parameters));
					self.out.append(parts.body);
					self.out.line("}");
				}
				self.restore_captures(saved);
			}
			(View::Name(_), _) => {
				let alias = self.expr(value);
				self.out.line(format!("const {name} = {};", alias.text));
			}
			// `function f = g then h` (expressions.rs)
			_ if self.analysis.compositions.contains_key(&value) => {
				let composed = self.expr(value);
				self.out.line(format!("const {name} = {};", composed.text));
			}
			_ => {
				self.unsupported(n, "this function declaration");
			}
		}
	}

	/// A composition of steps: `(...a) => h(g(f(...a)))`. Each step is
	/// evaluated once, when the composition is built (lead, "`then`
	/// evaluates its steps once"): a step that may change or that computes
	/// (a mutable variable, a call, a lambda) is bound by an enclosing
	/// arrow, `((s0) => (...a) => h(s0(...a)))(labeller(prefix))`; a name
	/// that can't change is used as is. An error stops the chain: it is
	/// thrown.
	pub(crate) fn composition(&mut self, steps: &[NodeId]) -> Expr {
		let mut bound: Vec<(String, String)> = Vec::new();
		let mut functions: Vec<String> = Vec::with_capacity(steps.len());
		for &step in steps {
			let text = self.expr(step).primary_text();
			let named = self.strip_groups(step);
			let fixed = match self.view(named) {
				View::Name(_) => match self.analysis.resolution(named) {
					fa_analyzer::symbols::Resolution::Local(symbol) => {
						!self.is_mutable_place(symbol)
					}
					_ => true,
				},
				_ => false,
			};
			if fixed {
				functions.push(text);
			} else {
				let name = format!("$s{}", bound.len());
				functions.push(name.clone());
				bound.push((name, text));
			}
		}
		let mut call = String::from("...a");
		for (index, function) in functions.iter().enumerate() {
			call = if index == 0 {
				format!("{function}(...a)")
			} else {
				format!("{function}({call})")
			};
		}
		if bound.is_empty() {
			return Expr::primary(format!("((...a) => {call})"));
		}
		let names: Vec<&str> = bound.iter().map(|(name, _)| name.as_str()).collect();
		let values: Vec<&str> = bound.iter().map(|(_, value)| value.as_str()).collect();
		Expr::primary(format!(
			"(({}) => (...a) => {call})({})",
			names.join(", "),
			values.join(", ")
		))
	}

	/// A lambda: an arrow function.
	pub(crate) fn lambda(&mut self, n: NodeId) -> Expr {
		let Some(info) = self.analysis.function(n).cloned() else {
			return self.unsupported(n, "this function");
		};
		let indent = self.out.indentation();
		let saved = self.copy_captures(&info);
		let parts = self.function_parts(&info, indent, true);
		// an async lambda that may suspend: its async instance too (tasks.rs)
		let asynchronous = self.lambda_suspends(n).then(|| {
			self.tasks.next_mode = Some(Mode::Async { coop: true });
			let parts = self.function_parts(&info, indent, true);
			Self::arrow(parts, indent)
		});
		self.restore_captures(saved);
		let blocking = Self::arrow(parts, indent);
		match asynchronous {
			Some(asynchronous) => {
				let helper = self.helper("faAsyncValue");
				js::call(
					helper,
					&[blocking.text, format!("async {}", asynchronous.text)],
				)
			}
			None => blocking,
		}
	}

	/// An arrow function of generated parts.
	fn arrow(parts: FunctionParts, indent: usize) -> Expr {
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
		let instance = self.instance_params.take();
		// how it runs (tasks.rs): an async instance takes its task first
		let is_async = info.sig.is_some_and(|sig| self.types.sig(sig).is_async);
		let mode = self.tasks.next_mode.take().unwrap_or(if is_async && arrow {
			Mode::Blocking
		} else {
			Mode::Dynamic
		});
		let depth = self.tasks.asynchronous
			&& mode == Mode::Blocking
			&& is_async
			&& self
				.analysis
				.async_summaries
				.get(&info.lambda)
				.is_some_and(|summary| summary.starts);
		let (parts, temps) = self.in_function(info.lambda, |g| {
			g.fcx.mode = mode;
			// a generic function's hidden parameters first (generics.rs)
			let mut parameters = g.declare_hidden(info.lambda);
			let mut prologue = Writer::at(indent + 1);
			// the requirements: hidden parameters after the regular ones (references.rs)
			let requirements = g.declare_requirements(info);
			let required = !requirements.is_empty();
			let mut hidden: Vec<String> =
				requirements.iter().map(|(_, name)| name.clone()).collect();
			if required {
				g.requirement_scopes.push(requirements);
			}
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
					// the hidden arguments come after the variadic ones: taken
					// off the end of the rest parameter
					if !hidden.is_empty() {
						prologue.line(format!(
							"const [{}] = {name}.splice({name}.length - {});",
							hidden.join(", "),
							hidden.len()
						));
						hidden.clear();
					}
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
			parameters.extend(hidden);
			if matches!(mode, Mode::Async { .. }) {
				parameters.insert(0, "$task".to_string());
			}
			// a reactive instance: the sources of its parameters (reactivity.rs)
			if let Some(instance) = &instance {
				parameters.extend(g.declare_param_sources(instance));
			}
			let parameters = format!("({})", parameters.join(", "));
			// the parameters holding a resource: moved in, dropped at the end
			// unless they move out (resources.rs)
			let mut drops = Vec::new();
			for param in &info.params {
				if g.owns_resource(param.symbol) {
					let saved = std::mem::replace(&mut g.out, prologue);
					if let Some(drop) = g.own_resource(param.symbol) {
						drops.push(drop);
					}
					prologue = std::mem::replace(&mut g.out, saved);
				}
			}
			let mut expression = None;
			let mut body = prologue;
			if arrow && !info.block_body && body.is_empty() {
				let (lines, value) = g.capture_at(indent + 1, |g| g.expr(info.body));
				if lines.is_empty() {
					let value = if dest == Dest::Return {
						g.copy_returned(info.body, value)
					} else {
						value
					};
					expression = Some(value);
				} else {
					body.append(lines);
					g.out = std::mem::replace(&mut body, Writer::new());
					g.finish(info.body, value, &dest);
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
			if required {
				g.requirement_scopes.pop();
			}
			if !drops.is_empty() {
				if let Some(value) = expression.take() {
					body.line(format!("return {};", value.text));
				}
				let mut wrapped = Writer::at(indent + 1);
				wrapped.open("try");
				wrapped.append(indent_by_one(body));
				drops.reverse();
				wrapped.close(&format!("finally {{ {} }}", drops.join(" ")));
				body = wrapped;
			}
			FunctionParts {
				parameters,
				body,
				expression,
			}
		});
		let mut parts = parts;
		if depth {
			// a blocking call: `start` runs its tasks at once (the blocking depth)
			if let Some(expression) = parts.expression.take() {
				let mut body = Writer::at(indent + 1);
				body.line(format!("return {};", expression.text));
				parts.body = body;
			}
			let sched = self.helper("faSched");
			let mut body = Writer::at(indent + 1);
			body.line(format!("{sched}.depth += 1;"));
			body.open("try");
			let inner = std::mem::take(&mut parts.body);
			body.append(indent_by_one(inner));
			body.close(&format!("finally {{ {sched}.depth -= 1; }}"));
			parts.body = body;
		}
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
		// `pick(value, { foo })`: the value copied to the node's type
		if let Some(&value) = self.analysis.forwards.get(&n) {
			let ty = self.type_of(n);
			// `pick(entity, keys)` of a key-set parameter: its paths at run time
			if let TypeKind::Param(param) = self.types.kind(ty)
				&& let Some(derived) = self.analysis.derived_params.get(&param).copied()
				&& let Some(keys) = arguments.get(1)
			{
				let values = self.in_order(&[value, keys], |g, node| g.expr(node));
				let helper = self.helper(if derived.pick { "faPick" } else { "faOmit" });
				let paths = format!("{}.paths", values[1].primary_text());
				return js::call(helper, &[values[0].text.clone(), paths]);
			}
			let value = self.expr(value);
			return self.copy_value(value, ty);
		}
		let Some(info) = self.analysis.call(n).cloned() else {
			// `repeatText(2)`: a generic application, as a value
			if let Some(function) = self.generic_application(n, callee) {
				return function;
			}
			// `transform<Integer>`: a generic function value is a plain function
			if self
				.analysis
				.flags_of(n)
				.contains(fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION)
			{
				return self.expr(callee);
			}
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
		// a C function (foreign.rs)
		if self.analysis.extern_calls.contains_key(&n) {
			return self.foreign_call(n, callee, &info.arguments);
		}
		// a method of a worker: a message to its thread (workers.rs)
		if let Some(call) = self.worker_call(n, callee, &info, arguments) {
			return call;
		}
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
				let (receiver, drop) = if optional {
					(self.keep_value(receiver), None)
				} else {
					self.temporary_receiver(callee, receiver)
				};
				let omitted = self.sentinel_slots(declaration, 1);
				let (receiver, mut values) =
					self.arguments(n, receiver, &rest, arguments, &omitted);
				values.insert(0, receiver.text.clone());
				let hidden = self.hidden_arguments(declaration, n);
				values.splice(0..0, hidden);
				let result = self.finish_call(n, Some(declaration), function, values);
				if !optional {
					return self.after_call_drop(result, drop);
				}
				// `receiver?.method()`: `none` when the receiver is `none`
				let none = Expr::constant("undefined", Prec::Primary);
				let test = js::binary(&receiver, "===", &none, Prec::Equality);
				js::conditional(&test, &none, &result)
			}
			Callee::Value
				if info
					.arguments
					.iter()
					.any(|argument| matches!(argument, Argument::Receiver(_))) =>
			{
				// a method of a type of another file: its exported `Type$method` (modules.rs)
				let method = match self.view(callee) {
					View::Access { members, .. } | View::OptionalAccess { members, .. } => {
						members.last()
					}
					_ => None,
				};
				let Some(function) = method.and_then(|member| self.external_value(member)) else {
					return self.unsupported(n, "this method of another file");
				};
				let receiver = self.receiver(callee);
				let (receiver, drop) = self.temporary_receiver(callee, receiver);
				let rest: Vec<Argument> = info
					.arguments
					.iter()
					.filter(|argument| !matches!(argument, Argument::Receiver(_)))
					.cloned()
					.collect();
				let (receiver, mut values) = self.arguments(n, receiver, &rest, arguments, &[]);
				values.insert(0, receiver.text.clone());
				let hidden = self.signature_hidden_arguments(callee, n);
				values.splice(0..0, hidden);
				let result = self.finish_call(n, None, function.text, values);
				self.after_call_drop(result, drop)
			}
			Callee::Value if self.constraint_method_call(callee).is_some() => {
				// `item.describe()` on a type parameter: the hidden function
				let function = self.constraint_method_call(callee).expect("a method");
				let receiver = self.receiver(callee);
				let (receiver, mut values) =
					self.arguments(n, receiver, &info.arguments, arguments, &[]);
				values.insert(0, receiver.text);
				self.finish_call(n, None, function, values)
			}
			Callee::Function(_) | Callee::Value => {
				let omitted = match info.callee {
					Callee::Function(declaration) => self.sentinel_slots(declaration, 0),
					_ => Vec::new(),
				};
				// `repeatText(2)("ab")`: the application's instance, one call
				let stripped = self.strip_groups(callee);
				let application = matches!(self.view(stripped), View::Call { .. })
					&& self.analysis.call(stripped).is_none()
					&& self.analysis.instantiation(stripped).is_some();
				let function = match (application, info.callee) {
					(true, Callee::Function(declaration)) => {
						match self.analysis.symbols.symbol_of_decl(declaration) {
							Some(symbol) => Expr::name(self.name_of(symbol), true),
							None => self.expr(callee),
						}
					}
					(true, Callee::Value) => match self.view(stripped) {
						View::Call { callee: inner, .. } => self.expr(inner),
						_ => unreachable!("a generic application is a call"),
					},
					_ => self.expr(callee),
				};
				// a reactive place given: the reactive instance (reactivity.rs)
				let sources = self.reactive_call_sources(n);
				let function = match (&sources, info.callee) {
					(Some(_), Callee::Function(declaration)) => {
						match self.analysis.symbols.symbol_of_decl(declaration) {
							Some(symbol) => Expr::name(self.reactive_instance_name(symbol), true),
							None => function,
						}
					}
					_ => function,
				};
				// an integer instance of a numeric-union parameter: the
				// integer variant (`integer_variants.rs`)
				let function = match (&sources, info.callee) {
					(None, Callee::Function(declaration)) => {
						let site = if application { stripped } else { n };
						match (
							self.call_variant(site, declaration),
							self.analysis.symbols.symbol_of_decl(declaration),
						) {
							(Some(variant), Some(symbol))
								if self
									.integer_variants
									.get(&declaration)
									.is_some_and(|variants| variants.contains(&variant)) =>
							{
								let suffix = self.variant_suffix(declaration, &variant);
								Expr::name(format!("{}{suffix}", self.name_of(symbol)), true)
							}
							_ => function,
						}
					}
					// a function of another module: the variant it exports
					(None, Callee::Value) => {
						let site = if application { stripped } else { n };
						match (
							self.external_variant_suffix(site, callee),
							self.external_of_callee(callee),
						) {
							(Some(suffix), Some(external)) => Expr::name(
								self.import(
									&external.path,
									&format!("{}{suffix}", external.export),
								),
								true,
							),
							_ => function,
						}
					}
					_ => function,
				};
				let (function, mut values) =
					self.arguments(n, function, &info.arguments, arguments, &omitted);
				if let Some(sources) = sources {
					values.extend(sources);
				}
				if let Callee::Function(declaration) = info.callee {
					let site = if application { stripped } else { n };
					let hidden = self.hidden_arguments(declaration, site);
					values.splice(0..0, hidden);
				} else if info.callee == Callee::Value {
					let site = if application { stripped } else { n };
					let hidden = self.signature_hidden_arguments(callee, site);
					values.splice(0..0, hidden);
				}
				let callee_text = if matches!(self.view(self.strip_groups(callee)), View::Name(_)) {
					function.text
				} else {
					function.primary_text()
				};
				let declaration = match info.callee {
					Callee::Function(declaration) => Some(declaration),
					_ => None,
				};
				// a function of another module that may suspend: its instances
				let external = match info.callee {
					Callee::Value => self.external_suspending(callee),
					_ => None,
				};
				let protect = self.fcx.catch_depth != 0 && self.has(n, NodeFlags::CALLBACK_CALL);
				if protect {
					// Argument evaluation belongs to the callee's own errors;
					// only an error from invoking the callback bypasses catches.
					for value in &mut values {
						let spread = value.starts_with("...");
						let text = value.strip_prefix("...").unwrap_or(value).to_string();
						let saved = self.keep_value(Expr::new(text, Prec::Assign));
						*value = if spread {
							format!("...{}", saved.text)
						} else {
							saved.text
						};
					}
				}
				let call = self.finish_call_of(n, declaration, external, callee_text, values);
				if protect {
					self.passed_callback_call(call)
				} else {
					call
				}
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
	pub(crate) fn arguments(
		&mut self,
		call: NodeId,
		first: Expr,
		arguments: &[Argument],
		written: NodeList<'_>,
		omitted: &[bool],
	) -> (Expr, Vec<String>) {
		let receiver = (self.tasks.detached_call == Some(call))
			.then(|| {
				self.analysis
					.call(call)?
					.arguments
					.iter()
					.find_map(|argument| {
						let Argument::Receiver(callee) = argument else {
							return None;
						};
						match self.view(self.strip_groups(*callee)) {
							View::Access { target, .. } | View::OptionalAccess { target, .. } => {
								Some(target)
							}
							_ => None,
						}
					})
			})
			.flatten();
		// A worker's transport already snapshots its arguments at send time.
		let detached = self.tasks.detached_call == Some(call)
			&& receiver.is_none_or(|node| !self.is_worker(node));
		let first = if let Some(receiver) = receiver.filter(|_| detached) {
			let copied = self.detached_argument(receiver, first);
			self.keep_value(copied)
		} else {
			first
		};
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
		// the order in which they are written (`fa_analyzer::lower::calls`)
		let value_nodes: Vec<NodeId> = slots
			.iter()
			.filter_map(|slot| match slot {
				Slot::Value(node) => Some(*node),
				Slot::Omitted(_) => None,
			})
			.collect();
		let mut positions =
			fa_analyzer::lower::calls::written_positions(self.tree, written, &value_nodes)
				.into_iter();
		let mut position = |_: NodeId| positions.next().unwrap_or(usize::MAX);
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
		let (lines, values) =
			self.capture(|g| g.in_order(&nodes, |g, node| g.argument(node, detached)));
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
			first = self.keep(first);
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
		// hidden arguments follow: every slot is passed (references.rs)
		let requirements = self.requirement_arguments(call);
		// the sources of a reactive instance follow too (reactivity.rs)
		let every = !requirements.is_empty() || self.calls_reactive_instance(call);
		let last = if !every {
			by_slot.iter().rposition(Option::is_some)
		} else {
			by_slot.len().checked_sub(1)
		};
		let sentinel = slots
			.iter()
			.enumerate()
			.any(|(index, slot)| {
				matches!(slot, Slot::Omitted(true))
					&& last.is_some_and(|last| index < last || every)
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
			.chain(requirements)
			.collect();
		(first, values)
	}

	/// One argument value: `...collection` for a spread.
	fn argument(&mut self, node: NodeId, detached: bool) -> Expr {
		match self.view(node) {
			View::Spread(source) => {
				let value = self.expr(source);
				let value = if detached {
					self.detached_argument(source, value)
				} else {
					value
				};
				Expr::new(format!("...{}", value.at(Prec::Assign)), Prec::Assign)
			}
			_ if detached => {
				let value = self.expr(node);
				self.detached_argument(node, value)
			}
			_ => self.expr(node),
		}
	}

	/// A detached call owns its arguments, including projected values and
	/// spread elements. Ordinary copy facts preserve fresh values, last-use
	/// moves and shared handles; resource expressions perform their own moves.
	fn detached_argument(&mut self, node: NodeId, value: Expr) -> Expr {
		let mut node = node;
		while let View::Group(inner) | View::Try(inner) = self.view(node) {
			node = inner;
		}
		if self
			.types
			.flags(self.type_of(node))
			.contains(fa_analyzer::types::TypeFlags::MOVE_ONLY)
		{
			value
		} else {
			self.copy_stored(node, value)
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
		// tasks and waits (tasks.rs)
		if let Some(call) = self.task_builtin(n, callee, path, arguments) {
			return call;
		}
		// `std.time`, `std.fs`, `std.json` (system.rs)
		if let Some(call) = self.system_call(n, path, |g| g.simple_arguments(arguments)) {
			return call;
		}
		// the sockets of `std.net` (system.rs)
		if let Some(call) = self.net_call(n, callee, path) {
			return call;
		}
		// workers and worker pools (workers.rs)
		if let Some(call) = self.worker_builtin(n, callee, path, arguments) {
			return call;
		}
		match path {
			"Pointer.allocate" => self.pointer_allocate(n, arguments),
			// pointers and C strings (foreign.rs)
			_ if path.starts_with("Pointer#") || path.starts_with("CString#") => {
				self.pointer_method(n, callee, path, arguments)
			}
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
			// the room of a collection or a string: the engine's (collections.rs)
			_ if path.ends_with("#reserve") => self.reserve_call(callee, arguments),
			"String.from" => self.string_from(arguments),
			// a `watch` block's subscription (reactivity.rs)
			"Subscription#stop" => {
				let receiver = self.receiver(callee);
				let helper = self.helper("faDispose");
				js::call(helper, &[receiver.text])
			}
			// thread-safe values, on one thread (concurrency.js)
			_ if path.starts_with("AtomicInteger#")
				|| path.starts_with("AtomicBoolean#")
				|| path.starts_with("Mutex#")
				|| path.starts_with("Channel#") =>
			{
				let receiver = self.receiver(callee);
				let nodes: Vec<NodeId> = arguments
					.iter()
					.filter_map(|argument| match argument {
						Argument::Given(value) => Some(*value),
						_ => None,
					})
					.filter(|value| {
						!matches!(
							self.types.kind(self.type_of(*value)),
							fa_analyzer::types::TypeKind::Sync(
								fa_analyzer::types::SyncKind::Ordering,
								_
							)
						)
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
				let method = path.rsplit('#').next().unwrap_or(path);
				let call = Expr::primary(format!(
					"{}.{method}({})",
					receiver.primary_text(),
					values.join(", ")
				));
				if path == "Mutex#tryLock" {
					// a guard used as a temporary (reactivity.rs)
					return self.guard_temporary(n, call);
				}
				call
			}
			"Integer.parse" => {
				let values = self.simple_arguments(arguments);
				let helper = self.helper("faParseInteger");
				js::call(helper, &values)
			}
			"Bytes.from" => {
				let values = self.builtin_arguments(n, arguments);
				js::call("Uint8Array.from", &values)
			}
			"Bytes.filled" => {
				let values = self.builtin_arguments(n, arguments);
				let array = format!("new Uint8Array({})", values[0]);
				Expr::primary(match values.get(1).filter(|value| value.as_str() != "0") {
					Some(value) => format!("{array}.fill({value})"),
					None => array,
				})
			}
			"String.fromUtf8" | "String.fromUtf8Lossy" => {
				let values = self.simple_arguments(arguments);
				let helper = self.helper(if path == "String.fromUtf8" {
					"faFromUtf8"
				} else {
					"faUtf8LossyDecoder"
				});
				let function = if path == "String.fromUtf8" {
					helper.to_string()
				} else {
					format!("{helper}.decode")
				};
				js::call(&function, &values)
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
			// a growing collection is its standard counterpart on JavaScript
			"GrowingArray.from" => self.collection_from(n, "Array.from", arguments),
			"GrowingSet.from" => self.collection_from(n, "Set.from", arguments),
			"GrowingMap.from" => self.collection_from(n, "Map.from", arguments),
			"Pool.from" | "GrowingPool.from" | "Columnar.from" | "GrowingColumnar.from" => {
				self.collection_from(n, "Pool.from", arguments)
			}
			"OrderedColumnar.from" => self.collection_from(n, "Array.from", arguments),
			// iterators: JavaScript's iterator helpers (types brief §5.0.6)
			_ if path.starts_with("Iterator#")
				|| path.ends_with("#values")
				|| path == "Map#keys" =>
			{
				self.array_method(n, callee, path, arguments)
			}
			// relation pools (relations.rs)
			_ if path.starts_with("RelationPool#") => {
				self.relation_pool_call(n, callee, path, arguments)
			}
			// pools and columnars (collections.rs)
			_ if path.starts_with("Pool#")
				|| path.starts_with("Columnar#")
				|| path.starts_with("OrderedColumnar#") =>
			{
				self.pool_call(n, callee, path, arguments)
			}
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
			"Fields#toStringArray" => {
				let receiver = self.receiver(callee);
				let ty = self.receiver_type(callee);
				self.fields_to_string_array(ty, receiver)
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
			"Error#toString" => {
				let receiver = self.receiver(callee);
				Expr::primary(format!("{}.toString()", receiver.primary_text()))
			}
			_ if path.starts_with("String#") || path.starts_with("Bytes#") => {
				let View::Access { .. } = self.view(callee) else {
					return self.unsupported(n, &format!("`{path}`"));
				};
				let receiver = self.receiver(callee);
				let View::Call {
					arguments: written, ..
				} = self.view(n)
				else {
					unreachable!("a builtin call")
				};
				let (receiver, values) = self.arguments(n, receiver, arguments, written, &[]);
				let values = values
					.into_iter()
					.enumerate()
					.map(|(index, text)| {
						let pure = match arguments.get(index) {
							Some(Argument::Given(node)) => {
								matches!(self.view(self.strip_groups(*node)), View::String(_))
							}
							_ => false,
						};
						Some(Expr::primary(text).with_purity(pure, pure))
					})
					.collect();
				self.string_method(n, path, receiver, values)
			}
			_ => self.unsupported(n, &format!("`{path}`")),
		}
	}

	fn builtin_arguments(&mut self, n: NodeId, arguments: &[Argument]) -> Vec<String> {
		let View::Call {
			arguments: written, ..
		} = self.view(n)
		else {
			unreachable!("a builtin call")
		};
		self.arguments(
			n,
			Expr::constant("undefined", Prec::Primary),
			arguments,
			written,
			&[],
		)
		.1
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
	pub(crate) fn array_method(
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
			"Array#join" => method("join", &texts),
			_ if path.ends_with("#values") => method("values", &[]),
			"Map#keys" => method("keys", &[]),
			"Iterator#filter" | "Iterator#map" | "Iterator#find" | "Iterator#some"
			| "Iterator#every" => {
				let name = path.trim_start_matches("Iterator#");
				let function = callback(self, 0, 1);
				method(name, &[function])
			}
			"Iterator#reduce" => {
				let function = callback(self, 0, 2);
				let initial = texts.get(1).cloned().unwrap_or_default();
				method("reduce", &[function, initial])
			}
			// `firstValues(count = 1)`
			"Iterator#firstValues" => {
				let count = texts.first().cloned().unwrap_or_else(|| "1".to_string());
				method("take", &[count])
			}
			"Iterator#toArray" => method("toArray", &[]),
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
			"Array#sort" | "Array#sortInPlace"
				if values.is_empty() && self.natural_sort(callee).is_some() =>
			{
				// the natural order of numbers or strings: a direct sort (`faSortIntegers`)
				let name = self.natural_sort(callee).unwrap_or("faArraySortInPlace");
				let name = self.helper(name);
				let array = if path == "Array#sort" {
					format!("{target}.slice()")
				} else {
					receiver.text.clone()
				};
				js::call(name, &[array])
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

	/// The helper sorting the items of an array receiver in their natural
	/// order without comparator calls: number-represented integers, floats
	/// and strings (the others go through `faCompare`).
	fn natural_sort(&self, callee: NodeId) -> Option<&'static str> {
		let item = match self.types.kind(self.receiver_type(callee)) {
			fa_analyzer::types::TypeKind::Collection(_, item, _) => item,
			_ => return None,
		};
		match self.kind(item) {
			crate::generate::Kind::Int(storage) if !storage.is_js_bigint() => {
				Some("faSortIntegers")
			}
			crate::generate::Kind::Float(_) => Some("faSortFloats"),
			crate::generate::Kind::Text => Some("faSortStrings"),
			_ => None,
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
	pub(crate) fn keep_value(&mut self, value: Expr) -> Expr {
		if value.stable {
			return value;
		}
		let temp = self.temp();
		self.out.line(format!("const {temp} = {};", value.text));
		Expr::name(temp, true)
	}

	/// A value computed before statements that come after it: kept in a
	/// temporary when those statements could change it.
	pub(crate) fn keep_before(&mut self, value: Expr, lines: &Writer) -> Expr {
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

/// The lines of a writer, one level deeper.
pub(super) fn indent_by_one(lines: Writer) -> Writer {
	let text = lines.finish();
	let mut out = Writer::at(0);
	for line in text.lines() {
		if line.is_empty() {
			out.blank();
		} else {
			out.line(format!("\t{line}"));
		}
	}
	out
}
