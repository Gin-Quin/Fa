//! Calls: argument binding (types brief §4.2.2, §4.2.3), conversions, generic
//! inference (M2), effect application (design §3.5, M2).
//!
//! A direct call of a `function` declaration binds positional arguments,
//! then named ones, fills the variadic parameter and leaves the omitted
//! parameters to their defaults. A call through any other function value
//! takes positional arguments only (`FN027`). The result is recorded in
//! `Analysis::calls` ([`CallInfo`]), in parameter order, for the backends.

use std::sync::Arc;

use crate::analysis::{Argument, CallInfo, Callee, NodeFlags};
use crate::builtins::{self, BuiltinKind};
use crate::check::Walker;
use crate::diagnostics::Code;
use crate::diagnostics::codes::types::{FN021, FN022, FN023, FN024, FN025, FN026, FN027, FN029};
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, ParamSig, TypeKind};

/// An argument as written.
#[derive(Copy, Clone, Debug)]
enum ArgSyntax {
	/// `value`
	Positional(NodeId),
	/// `...collection`: the spread node and its value.
	Spread(NodeId, NodeId),
	/// `name = value`: the argument node, the name node and the value.
	Named(NodeId, NodeId, NodeId),
}

/// A parameter of the callee, as seen by the binding.
#[derive(Copy, Clone, Debug)]
struct BindParam {
	/// Its type as passed (`Array(T)` for a variadic parameter).
	sig: ParamSig,
	/// Its name and symbol, for a direct call.
	named: Option<(Sym, SymbolId)>,
	/// Whether it has a default value.
	has_default: bool,
}

/// The result of binding the arguments of a call to the callee's parameters.
#[derive(Debug, Default)]
struct Binding {
	/// What fills each parameter.
	arguments: Vec<Argument>,
	/// The expected type of each argument value (`None`: checked alone).
	expected: Vec<(NodeId, Option<TypeId>)>,
	/// Spread arguments in the variadic part: the spread node, its value and the item type.
	spreads: Vec<(NodeId, NodeId, TypeId)>,
	/// Named arguments: the name node and the parameter it fills.
	names: Vec<(NodeId, NodeId, SymbolId)>,
	/// Type diagnostics.
	errors: Vec<(Code, NodeId, String)>,
}

impl Walker<'_> {
	/// `callee(arguments)`: a conversion `Int8(x)`, a direct call of a
	/// `function`, or a call of a function value.
	pub(super) fn check_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		if let Some((builtin, target)) = self.conversion_target(callee) {
			return self.check_conversion_call(n, callee, builtin, target, arguments);
		}
		if self.is_type_name(callee) {
			// `String(x)`, `Point(…)`, type functions: later milestones
			return self.unsupported(n, "calling a type");
		}
		let callee_ty = self.check(callee, None);
		let syntaxes = self.argument_syntax(arguments);
		let sig = match self.s.types.kind(callee_ty) {
			TypeKind::Function(sig) => self.s.types.sig(sig),
			_ => {
				for argument in &syntaxes {
					let value = match *argument {
						ArgSyntax::Positional(value)
						| ArgSyntax::Spread(_, value)
						| ArgSyntax::Named(_, _, value) => value,
					};
					// an unknown callee: the arguments are checked leniently (no `FN061`)
					let unknown = self.s.types.is_recovering(callee_ty);
					self.check(value, unknown.then_some(TypeId::UNKNOWN));
				}
				return self.memo(n, |w| {
					if !w.s.types.is_recovering(callee_ty) {
						let message = format!(
							"a `{}` is not a function: it can't be called",
							w.display(callee_ty)
						);
						let diagnostic = w.diagnostic(&E_TYPE_1, callee, message);
						w.report(diagnostic);
					}
					TypeId::UNKNOWN
				});
			}
		};
		let (kind, params) = self.callee_parameters(callee, sig.params);
		let binding = self.bind_arguments(n, &syntaxes, &params, kind);

		// the arguments: every visit (they carry flow)
		for &(value, expected) in &binding.expected {
			let actual = self.check(value, expected);
			if let Some(expected) = expected {
				self.sink(value, actual, expected);
			}
		}
		for &(spread, value, item) in &binding.spreads {
			let actual = self.check(value, None);
			self.set_type(spread, actual);
			self.sink_spread(value, actual, item);
		}

		self.memo(n, |w| {
			// `console.log(…)`: every value must be printable (strings.rs)
			if let Callee::Builtin(builtin) = kind
				&& builtins::console::prints(builtins::get(builtin).path)
			{
				for argument in &binding.arguments {
					let values: &[NodeId] = match argument {
						Argument::Given(value) => std::slice::from_ref(value),
						Argument::Variadic(values) => values,
						_ => &[],
					};
					for &value in values {
						let ty = w.type_of(value);
						w.check_printable(value, ty);
					}
				}
			}
			for (code, node, message) in binding.errors {
				let diagnostic = w.diagnostic(code, node, message);
				w.report(diagnostic);
			}
			for (argument, name, param) in binding.names {
				let ty = w.a.symbols.symbol(param).ty;
				w.set_resolution(name, Resolution::Local(param));
				w.set_type(name, ty);
				let value_ty = match w.view(argument) {
					View::Assignment {
						value: Some(value), ..
					} => w.type_of(value),
					_ => TypeId::UNKNOWN,
				};
				w.set_type(argument, value_ty);
			}
			w.a.calls.insert(
				n,
				CallInfo {
					callee: kind,
					arguments: binding.arguments,
				},
			);
			sig.ret
		})
	}

	/// The arguments as written.
	fn argument_syntax(&self, arguments: NodeList<'_>) -> Vec<ArgSyntax> {
		arguments
			.iter()
			.map(|argument| match self.view(argument) {
				View::Assignment {
					target,
					annotation: None,
					value: Some(value),
				} if matches!(self.view(target), View::Name(_)) => ArgSyntax::Named(argument, target, value),
				View::Spread(value) => ArgSyntax::Spread(argument, value),
				_ => ArgSyntax::Positional(argument),
			})
			.collect()
	}

	/// What a callee is, and its parameters as the binding sees them: names
	/// and defaults for a direct call of a declaration.
	fn callee_parameters(
		&self,
		callee: NodeId,
		params: crate::ids::ParamListId,
	) -> (Callee, Vec<BindParam>) {
		let sigs = self.s.types.params(params);
		let plain = |kind| -> (Callee, Vec<BindParam>) {
			let params = sigs
				.iter()
				.map(|&sig| BindParam {
					sig,
					named: None,
					has_default: false,
				})
				.collect();
			(kind, params)
		};
		let resolution = match self.view(callee) {
			View::Name(_) => self.a.resolution(callee),
			View::Access { members, .. } => members
				.last()
				.map_or(Resolution::Unresolved, |member| self.a.resolution(member)),
			_ => Resolution::Unresolved,
		};
		match resolution {
			Resolution::Builtin(builtin) => {
				let (kind, mut params) = plain(Callee::Builtin(builtin));
				// the defaults of a built-in function may be omitted (`padStart(3)`)
				if let Some(signature) = builtins::get(builtin).signature()
					&& signature.params.len() == params.len()
				{
					for (param, written) in params.iter_mut().zip(signature.params) {
						param.has_default = written.default;
					}
				}
				(kind, params)
			}
			Resolution::Local(symbol) => {
				let Some(declaration) = self.direct_function(symbol) else {
					return plain(Callee::Value);
				};
				let Some(info) = self.a.functions.get(&declaration) else {
					return plain(Callee::Value);
				};
				if info.params.len() != sigs.len() {
					return plain(Callee::Value);
				}
				let params = info
					.params
					.iter()
					.zip(sigs)
					.map(|(param, &sig)| BindParam {
						sig,
						named: Some((self.a.symbols.symbol(param.symbol).name, param.symbol)),
						has_default: param.default.is_some(),
					})
					.collect();
				(Callee::Function(declaration), params)
			}
			_ => plain(Callee::Value),
		}
	}

	/// Binds the arguments to the parameters (types brief §4.2.2): positional
	/// arguments first, in order, the remaining ones into the variadic
	/// parameter; then named ones. Pure: the diagnostics are returned.
	fn bind_arguments(
		&self,
		call: NodeId,
		arguments: &[ArgSyntax],
		params: &[BindParam],
		callee: Callee,
	) -> Binding {
		let mut binding = Binding {
			arguments: vec![Argument::Missing; params.len()],
			..Binding::default()
		};
		let variadic = params.iter().position(|param| param.sig.variadic);
		let fixed = variadic.unwrap_or(params.len());
		if let Some(index) = variadic {
			binding.arguments[index] = Argument::Variadic(Vec::new());
		}
		let mut position = 0;
		let mut named_seen = false;
		let mut extra_reported = false;
		for argument in arguments {
			match *argument {
				ArgSyntax::Named(node, name_node, value) => {
					named_seen = true;
					let text = self.tree.text(name_node.index());
					let found = params.iter().enumerate().find(|(_, param)| {
						param
							.named
							.is_some_and(|(name, _)| self.s.types.syms.resolve(name) == text)
					});
					let by_value = matches!(callee, Callee::Value | Callee::Builtin(_));
					let error = match found {
						_ if by_value && binding.errors.iter().any(|error| error.0.id == FN027.id) => {
							None
						}
						_ if by_value => Some((
							&FN027,
							"named arguments only exist for direct calls of a `function`: pass the arguments by position".to_string(),
						)),
						None => Some((&FN023, format!("this function has no parameter `{text}`"))),
						Some((_, param)) if param.sig.variadic => Some((
							&FN026,
							format!("the variadic parameter `{text}` can't be named: pass its values by position"),
						)),
						Some((index, _)) if binding.arguments[index] != Argument::Missing => {
							Some((&FN022, format!("`{text}` is already given")))
						}
						Some(_) => None,
					};
					match (error, found) {
						(None, Some((index, param))) if !by_value => {
							binding.arguments[index] = Argument::Given(value);
							binding.expected.push((value, Some(param.sig.ty)));
							if let Some((_, symbol)) = param.named {
								binding.names.push((node, name_node, symbol));
							}
						}
						(error, _) => {
							binding.expected.push((value, None));
							if let Some((code, message)) = error {
								binding.errors.push((code, node, message));
							}
						}
					}
				}
				ArgSyntax::Positional(value) | ArgSyntax::Spread(_, value) if named_seen => {
					binding.expected.push((value, None));
					binding.errors.push((
						&FN021,
						value,
						"a positional argument can't follow a named argument".to_string(),
					));
				}
				ArgSyntax::Positional(value) => {
					if position < fixed {
						binding.arguments[position] = Argument::Given(value);
						binding
							.expected
							.push((value, Some(params[position].sig.ty)));
						position += 1;
					} else if let Some(index) = variadic {
						let item = self.array_item(params[index].sig.ty);
						if let Argument::Variadic(items) = &mut binding.arguments[index] {
							items.push(value);
						}
						binding.expected.push((value, Some(item)));
					} else {
						binding.expected.push((value, None));
						if !extra_reported {
							extra_reported = true;
							binding.errors.push((
								&FN025,
								value,
								format!(
									"this function takes {} argument{}",
									params.len(),
									if params.len() == 1 { "" } else { "s" }
								),
							));
						}
					}
				}
				ArgSyntax::Spread(spread, value) => match variadic {
					Some(index) if position >= fixed => {
						let item = self.array_item(params[index].sig.ty);
						if let Argument::Variadic(items) = &mut binding.arguments[index] {
							items.push(spread);
						}
						binding.spreads.push((spread, value, item));
					}
					_ => {
						binding.expected.push((value, None));
						binding.errors.push((
							&FN029,
							spread,
							"a collection can only be spread into a variadic parameter".to_string(),
						));
					}
				},
			}
		}
		let mut missing = Vec::new();
		for (index, param) in params.iter().enumerate() {
			if binding.arguments[index] != Argument::Missing {
				continue;
			}
			if param.has_default {
				binding.arguments[index] = Argument::Default;
			} else {
				missing.push(match param.named {
					Some((name, _)) => format!("`{}`", self.s.types.syms.resolve(name)),
					None => format!("#{}", index + 1),
				});
			}
		}
		// a missing argument is often the consequence of another binding error
		if !missing.is_empty() && binding.errors.is_empty() {
			let plural = if missing.len() == 1 { "" } else { "s" };
			binding.errors.push((
				&FN024,
				call,
				format!("missing argument{plural}: {}", missing.join(", ")),
			));
		}
		binding
	}

	/// A spread argument: a collection whose items satisfy the variadic parameter.
	fn sink_spread(&mut self, value: NodeId, actual: TypeId, item: TypeId) {
		if self.s.types.is_recovering(actual) {
			return;
		}
		match self.s.types.kind(actual) {
			TypeKind::Collection(kind, actual_item, _) if kind != CollKind::Map => {
				self.sink(value, actual_item, item);
			}
			_ => {
				let message = format!(
					"a `{}` can't be spread: spread a collection of `{}`",
					self.display(actual),
					self.display(item)
				);
				let diagnostic = self.diagnostic(&E_TYPE_1, value, message);
				self.report_flow(diagnostic);
			}
		}
	}

	/// Whether a callee is a type name: a local `type`, or a type of the prelude.
	fn is_type_name(&mut self, callee: NodeId) -> bool {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, callee) else {
			return false;
		};
		let text = tree.name(name);
		let sym = self.s.types.syms.intern(text);
		match self.a.symbols.lookup(self.body.scope, sym) {
			Some(symbol) => self.a.symbols.symbol(symbol).kind == SymbolKind::Type,
			None => {
				super::type_expr::is_later_prelude_name(text)
					|| builtins::lookup(text).is_some_and(|builtin| {
						matches!(builtins::get(builtin).kind, BuiltinKind::Type(_))
					})
			}
		}
	}

	/// The number type a callee converts to: a built-in number type name not
	/// hidden by a local (`Int8`, `Integer`, `Float`…).
	fn conversion_target(&mut self, callee: NodeId) -> Option<(crate::ids::BuiltinId, TypeId)> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, callee) else {
			return None;
		};
		let text = tree.name(name);
		let sym = self.s.types.syms.intern(text);
		if let Some(symbol) = self.a.symbols.lookup(self.body.scope, sym)
			&& self.a.symbols.symbol(symbol).kind != SymbolKind::Type
		{
			return None;
		}
		let builtin = builtins::lookup(text)?;
		let BuiltinKind::Type(ty) = builtins::get(builtin).kind else {
			return None;
		};
		matches!(
			self.s.types.kind(ty),
			TypeKind::Int(_) | TypeKind::Float(_) | TypeKind::BigInt | TypeKind::Percentage
		)
		.then_some((builtin, ty))
	}

	/// `Int8(x)`, `Float(n)`: one value converted to a number type. The rules
	/// (which conversions exist, ranges, `CONVERSION_CHECK`) are in
	/// `numbers.rs` ([`Walker::check_conversion`]).
	fn check_conversion_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		builtin: crate::ids::BuiltinId,
		target: TypeId,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.memo(callee, |w| {
			w.set_resolution(callee, Resolution::Builtin(builtin));
			target
		});
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let syntaxes = self.argument_syntax(arguments);
		let value = match syntaxes.as_slice() {
			[ArgSyntax::Positional(value)] => Some(*value),
			_ => None,
		};
		match value {
			Some(value) => {
				let actual = self.check(value, None);
				self.check_conversion(n, value, actual, target);
			}
			None => {
				for argument in &syntaxes {
					let (ArgSyntax::Positional(value)
					| ArgSyntax::Spread(_, value)
					| ArgSyntax::Named(_, _, value)) = *argument;
					self.check(value, None);
				}
			}
		}
		self.memo(n, |w| {
			if value.is_none() {
				let name = w.display(target);
				let code = if arguments.is_empty() { &FN024 } else { &FN025 };
				let diagnostic = w.diagnostic(
					code,
					n,
					format!("a conversion takes one value, ex: `{name}(value)`"),
				);
				w.report(diagnostic);
			}
			w.a.calls.insert(
				n,
				CallInfo {
					callee: Callee::Conversion(target),
					arguments: vec![value.map_or(Argument::Missing, Argument::Given)],
				},
			);
			target
		})
	}
}
