//! Functions: declarations, lambdas, signatures, parameters, `return` (types
//! brief §4.1, §4.2, §4.6), SCC rounds (design §3.4, M2), composition and
//! partial application, `provide`.
//!
//! A function is analyzed in two phases (design §3.2): its signature
//! (parameters, declared return type), after which its symbol is in the state
//! `Body` and recursive uses see its type when the return type is declared
//! (`EFF020` otherwise); then its body. A named function's body sees no facts
//! of its surroundings (decision A11); a lambda's sees those of its creation.
//! The parameters and the top-level declarations of a block body share one
//! scope (`E-DECL-2` for a body declaration reusing a parameter name).

use std::sync::Arc;

use rustc_hash::FxHashSet;

use crate::analysis::{FnInfo, FnParam};
use crate::check::{BodyCx, FnCx, Walker};
use crate::diagnostics::SourceSpan;
use crate::diagnostics::codes::types::{
	FN011, FN012, FN013, FN020, FN028, FN040, FN041, FN061, FN062, WFN001,
};
use crate::diagnostics::codes::values::{E_DECL_9, E_DO_2, E_TYPE_1};
use crate::ids::{NodeId, SigId, SymbolId, TypeId};
use crate::symbols::{DeclState, Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, Name, NodeList, SignatureView, View};
use crate::types::{CollKind, FnSig, ParamSig, TypeKind};

/// A parameter as written.
#[derive(Copy, Clone, Debug)]
struct ParamSyntax {
	/// The parameter node.
	node: NodeId,
	/// The node of its name.
	name_node: NodeId,
	/// Its name.
	name: Name,
	/// `: Type`
	annotation: Option<NodeId>,
	/// `= default`
	default: Option<NodeId>,
	/// `...name`
	variadic: bool,
}

/// What is being analyzed: a `function` declaration or a lambda.
#[derive(Copy, Clone, Debug)]
struct FnOwner {
	/// The `function name = …` node.
	declaration: Option<NodeId>,
	/// Its symbol.
	symbol: Option<SymbolId>,
	/// Its expected type couldn't be resolved: parameters without type are
	/// `UNKNOWN`, not `FN061`.
	lenient: bool,
}

impl Walker<'_> {
	/// `function name = …` (hoisted: runs once, see `Walker::hoisted`): sets
	/// the symbol's type. The value is a lambda, or a name (an alias, which
	/// keeps the named arguments and the defaults).
	pub(super) fn check_function_decl(
		&mut self,
		n: NodeId,
		_name: Name,
		is_async: bool,
		generics: NodeList<'_>,
		annotation: Option<NodeId>,
		value: NodeId,
	) -> TypeId {
		if !generics.is_empty() {
			return self.unsupported(n, "generic functions");
		}
		if is_async {
			return self.unsupported(n, "`async` functions");
		}
		let symbol = self.a.symbols.symbol_of_decl(n);
		let mut owner = FnOwner {
			declaration: Some(n),
			symbol,
			lenient: false,
		};
		let tree = Arc::clone(&self.tree);
		let ty = match syntax::view(&tree, value) {
			View::Lambda { signature, body } => {
				// `function ascending: Comparator = (a, b) => a - b`: the type is the contract
				let expected = match annotation {
					Some(annotation) => {
						let declared = self.check_type_expr(annotation);
						match self.signature_of(declared) {
							Some(sig) => Some(sig),
							None if self.s.types.is_recovering(declared) => {
								owner.lenient = true;
								None
							}
							None => {
								let message =
									format!("`{}` is not a function type", self.display(declared));
								self.memo(annotation, |w| {
									let diagnostic = w.diagnostic(&E_TYPE_1, annotation, message);
									w.report(diagnostic);
									TypeId::UNKNOWN
								});
								None
							}
						}
					}
					None => None,
				};
				// a named function sees no facts of its surroundings (decision A11)
				let scope = self.body.scope;
				let saved = std::mem::replace(&mut self.body, BodyCx::new(scope));
				let ty = self.check_function(value, signature, body, owner, expected);
				self.body = saved;
				ty
			}
			View::Name(_) if annotation.is_none() => {
				let ty = self.check(value, None);
				if !self.s.types.is_recovering(ty) && self.signature_of(ty).is_none() {
					let message = format!(
						"`{}` is not a function: a `function` declaration needs a function",
						self.display(ty)
					);
					let diagnostic = self.diagnostic(&E_TYPE_1, value, message);
					self.report(diagnostic);
				}
				ty
			}
			_ => return self.unsupported(n, "this function expression"),
		};
		if let Some(symbol) = symbol {
			self.a.symbols.symbol_mut(symbol).ty = ty;
		}
		TypeId::VOID
	}

	/// A lambda: its parameter types, return type and effects come from the
	/// expected function type when there is one (types brief §4.6); without
	/// one, its parameters must be annotated (`FN061`) and its return type is
	/// inferred from its body, whatever its form.
	pub(super) fn check_lambda(
		&mut self,
		n: NodeId,
		signature: SignatureView<'_>,
		body: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let lenient = expected.is_some_and(|expected| self.s.types.is_recovering(expected));
		let expected = expected.and_then(|expected| self.signature_of(expected));
		// it sees the facts of its creation (values brief D-102); `break`,
		// `continue` and `return` don't cross it
		let inner = BodyCx {
			scope: self.body.scope,
			flow: self.body.flow.fork(),
			functions: Vec::new(),
			loops: Vec::new(),
			flow_buffers: std::mem::take(&mut self.body.flow_buffers),
			requirements: std::mem::take(&mut self.body.requirements),
		};
		let saved = std::mem::replace(&mut self.body, inner);
		let owner = FnOwner {
			declaration: None,
			symbol: None,
			lenient,
		};
		let ty = self.check_function(n, signature, body, owner, expected);
		let inner = std::mem::replace(&mut self.body, saved);
		self.body.flow_buffers = inner.flow_buffers;
		self.body.requirements = inner.requirements;
		ty
	}

	/// The two phases of a function or a lambda (`lambda` is the
	/// `ArrowFunction` node). Runs in the body context prepared by the caller.
	fn check_function(
		&mut self,
		lambda: NodeId,
		signature: SignatureView<'_>,
		body: NodeId,
		owner: FnOwner,
		expected: Option<FnSig>,
	) -> TypeId {
		let block_body = matches!(self.view(body), View::Block { .. });
		// the parameters share the scope of a block body's top-level declarations
		let scope_owner = if block_body { body } else { lambda };
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(scope_owner, Some(parent), ScopeKind::Function);
		self.body.scope = scope;

		// 1. the signature
		let (params, incomplete) =
			self.check_params(lambda, signature.parameters, expected, owner.lenient);
		let declared_return = signature
			.returns
			.map(|returns| self.check_type_expr(returns));
		let expected_return = expected.map(|sig| sig.ret).filter(|ret| !ret.is_unset());
		let returns = declared_return.or(expected_return);
		let throws = signature
			.throws
			.map_or(TypeId::NEVER, |throws| self.check_type_expr(throws));
		let mut effects = false;
		for path in signature
			.requires
			.iter()
			.chain(signature.mutates.iter())
			.chain(signature.invalidates.iter())
		{
			effects = true;
			self.unsupported(path, "`requires`, `mutates` and `invalidates`");
		}
		let mut param_sigs: Vec<ParamSig> = params.iter().map(|(_, sig)| *sig).collect();
		// a lambda can declare fewer parameters than its expected type
		if let Some(expected) = expected {
			let expected_params = self.s.types.params(expected.params);
			if expected_params.len() > param_sigs.len() {
				param_sigs.extend_from_slice(&expected_params[param_sigs.len()..]);
			}
		}
		if let Some(symbol) = owner.symbol {
			let frame = self.decl_stack.len().saturating_sub(1) as u32;
			let fn_ty = if incomplete {
				Some(TypeId::UNKNOWN)
			} else {
				returns.map(|ret| self.function_type(&param_sigs, ret, throws))
			};
			let symbol = self.a.symbols.symbol_mut(symbol);
			symbol.state = DeclState::Body { frame };
			if let Some(fn_ty) = fn_ty {
				symbol.ty = fn_ty;
			}
		}
		if !self.a.functions.contains_key(&lambda) {
			let info = FnInfo {
				declaration: owner.declaration,
				symbol: owner.symbol,
				lambda,
				ty: TypeId::UNKNOWN,
				sig: None,
				params: params.iter().map(|(param, _)| param.clone()).collect(),
				body,
				block_body,
				declared_return: declared_return.is_some(),
				scope,
				throws,
				captures: Vec::new(),
			};
			if let Some(declaration) = owner.declaration {
				self.a.functions.insert(declaration, info.clone());
			}
			self.a.functions.insert(lambda, info);
		}

		// 2. the body
		let mode = match returns {
			Some(returns) => returns,
			None if block_body && owner.declaration.is_some() => TypeId::VOID,
			None => TypeId::UNSET,
		};
		self.body.functions.push(FnCx {
			node: lambda,
			symbol: owner.symbol,
			returns: mode,
			infer_returns: owner.declaration.is_none() && returns.is_none(),
			returned: TypeId::NEVER,
			throws: TypeId::NEVER,
			return_states: Vec::new(),
			unsupported_at_start: self.unsupported_seen,
		});
		// with effects the analyzer doesn't know yet, the body is walked quietly
		self.quiet += u32::from(effects);
		let body_ty = if block_body {
			self.check_block_body(body, mode, owner)
		} else {
			self.check_expression_body(body, mode)
		};
		self.quiet -= u32::from(effects);
		let cx = self.body.functions.pop();
		self.body.scope = parent;

		self.memo(lambda, |w| {
			let ret = match returns {
				Some(returns) => returns,
				None if mode == TypeId::VOID => TypeId::VOID,
				None => {
					let returned = cx.map_or(TypeId::NEVER, |cx| cx.returned);
					let ty = if returned == TypeId::NEVER {
						body_ty
					} else {
						w.s.types.join(body_ty, returned)
					};
					w.s.types.widen(ty)
				}
			};
			let ty = if incomplete {
				TypeId::UNKNOWN
			} else {
				w.function_type(&param_sigs, ret, throws)
			};
			let sig = w.signature_id(ty);
			for key in [Some(lambda), owner.declaration].into_iter().flatten() {
				if let Some(info) = w.a.functions.get_mut(&key) {
					info.ty = ty;
					info.sig = sig;
				}
			}
			if let Some(symbol) = owner.symbol {
				w.a.symbols.symbol_mut(symbol).ty = ty;
			}
			ty
		})
	}

	/// The function type of a signature.
	fn function_type(&mut self, params: &[ParamSig], ret: TypeId, throws: TypeId) -> TypeId {
		let base = self.s.types.simple_sig(&[], ret, throws);
		let params = self.s.types.param_list(params);
		self.s.types.function(FnSig { params, ..base })
	}

	/// The signature id of a function type.
	fn signature_id(&self, ty: TypeId) -> Option<SigId> {
		match self.s.types.kind(ty) {
			TypeKind::Function(sig) => Some(sig),
			_ => None,
		}
	}

	/// The parameters: declared in order in the function scope, so that a
	/// default value sees the previous ones (`FN020` for a later one).
	fn check_params(
		&mut self,
		lambda: NodeId,
		parameters: NodeList<'_>,
		expected: Option<FnSig>,
		lenient: bool,
	) -> (Vec<(FnParam, ParamSig)>, bool) {
		let expected_params: Vec<ParamSig> = expected
			.map(|sig| self.s.types.params(sig.params).to_vec())
			.unwrap_or_default();
		if expected.is_some()
			&& parameters.len() > expected_params.len()
			&& self.type_of(lambda).is_unset()
		{
			let diagnostic = self.diagnostic(
				&FN062,
				lambda,
				format!(
					"this function takes at most {} parameters here",
					expected_params.len()
				),
			);
			self.report(diagnostic);
		}
		let mut syntaxes = Vec::with_capacity(parameters.len());
		let mut incomplete = false;
		let mut params = Vec::with_capacity(parameters.len());
		for (position, parameter) in parameters.iter().enumerate() {
			if let Some(name) = self.self_parameter(parameter) {
				match self.declare_self(parameter, name, position == 0) {
					Some(param) => params.push(param),
					None => incomplete = true,
				}
				continue;
			}
			match self.parameter_syntax(parameter) {
				Some(syntax) => syntaxes.push(syntax),
				None => {
					if !matches!(self.view(parameter), View::Error) {
						self.unsupported(parameter, "this kind of parameter");
					}
					// the signature is incomplete: calls are checked leniently
					incomplete = true;
				}
			}
		}
		let offset = params.len();
		for (index, param) in syntaxes.iter().enumerate() {
			let expected = expected_params.get(index).copied();
			let declared = match (param.annotation, expected) {
				(Some(annotation), _) => self.check_type_expr(annotation),
				(None, Some(expected)) if expected.variadic => self.array_item(expected.ty),
				(None, Some(expected)) => expected.ty,
				(None, None) if lenient => TypeId::UNKNOWN,
				(None, None) => TypeId::UNSET,
			};
			let mut later_use = None;
			if let Some(default) = param.default {
				let tree = Arc::clone(&self.tree);
				let later: Vec<&str> = syntaxes[index..]
					.iter()
					.map(|later| tree.name(later.name))
					.collect();
				later_use = self.find_name_use(default, &later);
				self.quiet += u32::from(later_use.is_some());
				let expected = (!declared.is_unset()).then_some(declared);
				let actual = self.check(default, expected);
				if let Some(expected) = expected {
					self.sink(default, actual, expected);
				}
				self.quiet -= u32::from(later_use.is_some());
			}
			let last = index + 1 == syntaxes.len();
			let ty = self.memo(param.node, |w| {
				if let Some((use_node, text)) = later_use {
					let diagnostic = w.diagnostic(
						&FN020,
						use_node,
						format!(
							"the default value uses `{text}`, a later parameter: it can only use the parameters before it"
						),
					);
					w.report(diagnostic);
				}
				w.declare_param(*param, index + offset, declared, last)
			});
			let Some(symbol) = self.a.symbols.symbol_of_decl(param.node) else {
				continue;
			};
			let body_ty = self.a.symbols.symbol(symbol).ty;
			params.push((
				FnParam {
					node: param.node,
					symbol,
					default: param.default,
					variadic: param.variadic,
				},
				ParamSig {
					ty: if param.variadic { body_ty } else { ty },
					variadic: param.variadic,
				},
			));
		}
		(params, incomplete)
	}

	/// The name of a `self` parameter (`self`, or `self: T`, which is `FN041`).
	fn self_parameter(&self, node: NodeId) -> Option<Name> {
		let target = match self.view(node) {
			View::Assignment { target, .. } => target,
			_ => node,
		};
		match self.view(target) {
			View::Name(name) if self.tree.name(name) == "self" => Some(name),
			_ => None,
		}
	}

	/// Declares `self`: the first parameter of a function declared in a type,
	/// without annotation (`FN040`, `FN041` otherwise). Its type is the type.
	fn declare_self(
		&mut self,
		node: NodeId,
		name: Name,
		first: bool,
	) -> Option<(FnParam, ParamSig)> {
		let owner = self
			.a
			.symbols
			.scope(self.body.scope)
			.parent
			.and_then(|parent| self.self_type(parent));
		let annotated = matches!(self.view(node), View::Assignment { .. });
		let ty = self.memo(node, |w| {
			let problem = match (owner, first, annotated) {
				(None, _, _) => Some((
					&FN040,
					"`self` is only a parameter of the functions declared in a type",
				)),
				(_, false, _) => Some((&FN040, "`self` is the first parameter of a method")),
				(_, _, true) => Some((
					&FN041,
					"`self` has no annotation: its type is the type of the method",
				)),
				_ => None,
			};
			if let Some((code, message)) = problem {
				let diagnostic = w.diagnostic(code, node, message);
				w.report(diagnostic);
			}
			let ty = owner.map_or(TypeId::UNKNOWN, |(ty, _)| ty);
			let symbol = w.declare(node, name, SymbolKind::SelfParameter);
			let symbol = w.a.symbols.symbol_mut(symbol);
			symbol.ty = ty;
			symbol.state = DeclState::Done;
			ty
		});
		let symbol = self.a.symbols.symbol_of_decl(node)?;
		(owner.is_some() && first).then_some((
			FnParam {
				node,
				symbol,
				default: None,
				variadic: false,
			},
			ParamSig {
				ty,
				variadic: false,
			},
		))
	}

	/// Declares a parameter (type part of its node): `FN061` without a type,
	/// `E-DECL-9` for `Void`, `FN028` for a variadic parameter that is not last.
	fn declare_param(
		&mut self,
		param: ParamSyntax,
		index: usize,
		declared: TypeId,
		last: bool,
	) -> TypeId {
		let text = self.tree.name(param.name).to_string();
		let mut ty = declared;
		if ty.is_unset() {
			let diagnostic = self.diagnostic(
				&FN061,
				param.node,
				format!("the type of `{text}` is not known: annotate it, ex: `{text}: Integer`"),
			);
			self.report(diagnostic);
			ty = TypeId::UNKNOWN;
		} else if ty == TypeId::VOID {
			let diagnostic = self.diagnostic(
				&E_DECL_9,
				param.annotation.unwrap_or(param.node),
				"`Void` is not a value type: a parameter can't hold it",
			);
			self.report(diagnostic);
			ty = TypeId::UNKNOWN;
		}
		if param.variadic && !last {
			let diagnostic = self.diagnostic(
				&FN028,
				param.node,
				"a variadic parameter is the last one, and a function has at most one",
			);
			self.report(diagnostic);
		}
		let symbol = self.declare(
			param.node,
			param.name,
			SymbolKind::Parameter {
				index: index as u16,
			},
		);
		let body_ty = if param.variadic {
			self.s.types.array(ty)
		} else {
			ty
		};
		self.a.symbols.symbol_mut(symbol).ty = body_ty;
		self.a.symbols.symbol_mut(symbol).state = DeclState::Done;
		if param.name_node != param.node {
			self.set_type(param.name_node, body_ty);
			self.set_resolution(param.name_node, Resolution::Local(symbol));
		}
		ty
	}

	/// The parts of a parameter: `a`, `a: T`, `a: T = d`, `...a: T`. `None` for
	/// the forms that come later (destructured, anonymous structure, `self`).
	fn parameter_syntax(&self, node: NodeId) -> Option<ParamSyntax> {
		let name_of = |target: NodeId| -> Option<(NodeId, Name, bool)> {
			match self.view(target) {
				View::Name(name) => Some((target, name, false)),
				View::Spread(inner) => match self.view(inner) {
					View::Name(name) => Some((inner, name, true)),
					_ => None,
				},
				_ => None,
			}
		};
		let (target, annotation, default) = match self.view(node) {
			View::Assignment {
				target,
				annotation,
				value,
			} => (target, annotation, value),
			View::Name(_) | View::Spread(_) => (node, None, None),
			_ => return None,
		};
		let (name_node, name, variadic) = name_of(target)?;
		if name.is_empty() || self.tree.name(name) == "self" {
			return None;
		}
		Some(ParamSyntax {
			node,
			name_node,
			name,
			annotation,
			default,
			variadic,
		})
	}

	/// The first name node below `n` whose text is one of `names` and that
	/// doesn't resolve in the current scope.
	pub(super) fn find_name_use(&mut self, n: NodeId, names: &[&str]) -> Option<(NodeId, String)> {
		if let View::Name(name) = self.view(n) {
			let text = self.tree.name(name);
			if names.contains(&text) {
				let sym = self.s.types.syms.intern(text);
				if self.a.symbols.lookup(self.body.scope, sym).is_none() {
					return Some((n, text.to_string()));
				}
			}
			return None;
		}
		for child in syntax::children(&self.tree, n) {
			if let Some(found) = self.find_name_use(child, names) {
				return Some(found);
			}
		}
		None
	}

	/// A block body. With a return type `R`, its last expression must give an
	/// `R` (`FN011` when the end is reachable without a value); without one, it
	/// returns nothing and drops its last value (`WFN001` when computing it has
	/// no effect).
	fn check_block_body(&mut self, body: NodeId, mode: TypeId, owner: FnOwner) -> TypeId {
		match mode {
			TypeId::VOID | TypeId::NEVER => {
				self.walk_block(body, None);
				if mode == TypeId::NEVER {
					if self.body.flow.reachable && self.body_is_complete() {
						self.report_never_returns(body);
					}
					return TypeId::NEVER;
				}
				if owner.declaration.is_some() {
					self.warn_dropped_value(body);
				}
				TypeId::VOID
			}
			TypeId::UNSET => self.check_block_value(body, None).0,
			returns => {
				let (ty, last) = self.check_block_value(body, Some(returns));
				match last {
					Some(last) if ty != TypeId::NEVER => self.sink(last, ty, returns),
					Some(_) => {}
					None => {
						if self.body.flow.reachable && self.body_is_complete() {
							let name = self.function_name();
							let diagnostic = self.diagnostic_at(
								&FN011,
								self.closing_brace(body),
								format!(
									"missing value at the end of {name}: it returns a `{}`",
									self.display(returns)
								),
							);
							self.report_flow(diagnostic);
						}
					}
				}
				returns
			}
		}
	}

	/// A single-expression body: its value is the function's.
	fn check_expression_body(&mut self, body: NodeId, mode: TypeId) -> TypeId {
		match mode {
			TypeId::VOID => self.check(body, None),
			TypeId::UNSET => {
				// an inferred return type: a sink without expected type (numbers.rs)
				let ty = self.check(body, None);
				self.sink_unannotated(body, ty);
				ty
			}
			TypeId::NEVER => {
				let ty = self.check(body, Some(TypeId::NEVER));
				if ty != TypeId::NEVER && !self.s.types.is_recovering(ty) && self.body_is_complete()
				{
					self.report_never_returns(body);
				}
				TypeId::NEVER
			}
			returns => {
				let ty = self.check(body, Some(returns));
				self.sink(body, ty, returns);
				returns
			}
		}
	}

	/// `FN012`: a function returning `Never` whose end is reachable.
	fn report_never_returns(&mut self, n: NodeId) {
		let name = self.function_name();
		let diagnostic = self.diagnostic(
			&FN012,
			n,
			format!("{name} returns `Never`: it must always throw, stop or loop forever"),
		);
		self.report_flow(diagnostic);
	}

	/// `WFN001`: the last value of a block body without return type is dropped.
	fn warn_dropped_value(&mut self, body: NodeId) {
		let View::Block { statements } = self.view(body) else {
			return;
		};
		let Some(last) = statements.last() else {
			return;
		};
		let ty = self.type_of(last);
		if !self.body_is_complete()
			|| !self.is_value_statement(last)
			|| !self.is_pure_expression(last)
			|| matches!(ty, TypeId::VOID | TypeId::NEVER | TypeId::UNSET)
			|| self.s.types.is_recovering(ty)
		{
			return;
		}
		let name = self.function_name();
		let diagnostic = self.diagnostic(
			&WFN001,
			last,
			format!(
				"this value is dropped: {name} has no return type. Declare one, ex: `(…): {} =>`",
				self.display(self.s.types.widen(ty))
			),
		);
		self.report_flow(diagnostic);
	}

	/// Whether computing an expression has no effect (no call, no assignment,
	/// nothing that can fail but arithmetic).
	fn is_pure_expression(&self, n: NodeId) -> bool {
		let pure = matches!(
			self.view(n),
			View::Integer(_)
				| View::Float(_)
				| View::BigInt(_)
				| View::String(_)
				| View::Template { .. }
				| View::Boolean(_)
				| View::None | View::Name(_)
				| View::ImplicitMember(_)
				| View::Group(_)
				| View::Array { .. }
				| View::Tuple { .. }
				| View::Object { .. }
				| View::Access { .. }
				| View::Arithmetic { .. }
				| View::Negate(_)
				| View::Percentage(_)
				| View::Equality { .. }
				| View::Comparison { .. }
				| View::Logic { .. }
				| View::Not(_)
				| View::Fallback { .. }
				| View::Is { .. }
				| View::Lambda { .. }
		);
		pure && (matches!(self.view(n), View::Lambda { .. })
			|| syntax::children(&self.tree, n).all(|child| self.is_pure_expression(child)))
	}

	/// Whether no unsupported construct or parse error was met in the body
	/// so far: otherwise, what depends on the whole body is not reported.
	fn body_is_complete(&self) -> bool {
		self.body
			.functions
			.last()
			.is_none_or(|cx| cx.unsupported_at_start == self.unsupported_seen)
	}

	/// Where a missing final value is reported: the closing brace of the block.
	fn closing_brace(&self, body: NodeId) -> SourceSpan {
		let span = self.span(body);
		SourceSpan::new(span.end.saturating_sub(1).max(span.start), span.end)
	}

	/// The function being analyzed, for messages: "`f`" or "this function".
	fn function_name(&self) -> String {
		self.body
			.functions
			.last()
			.and_then(|cx| cx.symbol)
			.map_or_else(
				|| "this function".to_string(),
				|symbol| {
					let name = self.a.symbols.symbol(symbol).name;
					format!("`{}`", self.s.types.syms.resolve(name))
				},
			)
	}

	/// `return`, `return value` (called by `control.rs`): the value must give
	/// the declared return type; `FN013` with a value in a `function` without
	/// return type, `FN011` without a value when one is needed, `E-DO-2`
	/// outside a function. The rest of the function is unreachable from here.
	pub(super) fn check_return_value(&mut self, n: NodeId, value: Option<NodeId>) -> TypeId {
		let Some(cx) = self.body.functions.last() else {
			if let Some(value) = value {
				self.check(value, None);
			}
			self.memo(n, |w| {
				let diagnostic = w.diagnostic(
					&E_DO_2,
					n,
					"`return` is only allowed in a function: at the top level, the program ends with its last statement",
				);
				w.report(diagnostic);
				TypeId::NEVER
			});
			self.body.flow.set_unreachable();
			return TypeId::NEVER;
		};
		let (returns, infer) = (cx.returns, cx.infer_returns);
		let name = self.function_name();
		match value {
			Some(value) if returns.is_unset() && infer => {
				let ty = self.check(value, None);
				let returned = self
					.body
					.functions
					.last()
					.map_or(TypeId::NEVER, |cx| cx.returned);
				let joined = if returned == TypeId::NEVER {
					ty
				} else {
					self.s.types.join(returned, ty)
				};
				if let Some(cx) = self.body.functions.last_mut() {
					cx.returned = joined;
				}
			}
			Some(value) if returns.is_unset() || returns == TypeId::VOID => {
				self.check(value, None);
				self.memo(n, |w| {
					let diagnostic = w.diagnostic(
						&FN013,
						value,
						format!(
							"{name} has no return type: `return` can't give a value. Declare one, ex: `(…): Integer =>`"
						),
					);
					w.report(diagnostic);
					TypeId::NEVER
				});
			}
			Some(value) if returns == TypeId::NEVER => {
				self.check(value, None);
				self.memo(n, |w| {
					let diagnostic = w.diagnostic(
						&FN012,
						n,
						format!("{name} returns `Never`: it can't return"),
					);
					w.report(diagnostic);
					TypeId::NEVER
				});
			}
			Some(value) => {
				let ty = self.check(value, Some(returns));
				self.sink(value, ty, returns);
			}
			None if returns == TypeId::NEVER => {
				self.memo(n, |w| {
					let diagnostic = w.diagnostic(
						&FN012,
						n,
						format!("{name} returns `Never`: it can't return"),
					);
					w.report(diagnostic);
					TypeId::NEVER
				});
			}
			None if !returns.is_unset() && returns != TypeId::VOID => {
				self.memo(n, |w| {
					let diagnostic = w.diagnostic(
						&FN011,
						n,
						format!(
							"`return` needs a value here: {name} returns a `{}`",
							w.display(returns)
						),
					);
					w.report(diagnostic);
					TypeId::NEVER
				});
			}
			None => {}
		}
		let state = self.body.flow.fork();
		if let Some(cx) = self.body.functions.last_mut() {
			cx.return_states.push(state);
		}
		self.body.flow.set_unreachable();
		self.memo(n, |_| TypeId::NEVER)
	}

	/// The parameters of a function (normally handled by their function).
	pub(super) fn check_parameters(&mut self, n: NodeId, _items: NodeList<'_>) -> TypeId {
		self.unsupported(n, "parameters")
	}

	/// `{ x, y }: Point` as a parameter.
	pub(super) fn check_destructured_parameter(
		&mut self,
		n: NodeId,
		_pattern: NodeId,
		_annotation: NodeId,
	) -> TypeId {
		self.unsupported(n, "destructured parameters")
	}

	/// `f then g`
	pub(super) fn check_compose(&mut self, n: NodeId, _operands: NodeList<'_>) -> TypeId {
		self.unsupported(n, "`then`")
	}

	/// `f with (y = 0)`
	pub(super) fn check_partial_application(
		&mut self,
		n: NodeId,
		_function: NodeId,
		_arguments: NodeList<'_>,
	) -> TypeId {
		self.unsupported(n, "`with`")
	}

	/// `provide Database = value` (M2 requirements).
	pub(super) fn check_provide(&mut self, n: NodeId, _ty: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "`provide`")
	}

	/// The `function` declaration with a signature that a call of `symbol`
	/// reaches: the function itself, or the target of an alias
	/// (`function f = add`), which keeps the named arguments and the defaults.
	pub(super) fn direct_function(&self, symbol: SymbolId) -> Option<NodeId> {
		let mut visited = FxHashSet::default();
		let mut current = symbol;
		loop {
			let found = self.a.symbols.symbol(current);
			if found.kind != SymbolKind::Function || !visited.insert(current) {
				return None;
			}
			if self.a.functions.contains_key(&found.decl_node) {
				return Some(found.decl_node);
			}
			let View::Function { value, .. } = self.view(found.decl_node) else {
				return None;
			};
			let Resolution::Local(target) = self.a.resolution(value) else {
				return None;
			};
			current = target;
		}
	}

	/// The item type of an `Array(T)`, or the type itself.
	pub(super) fn array_item(&self, ty: TypeId) -> TypeId {
		match self.s.types.kind(ty) {
			TypeKind::Collection(CollKind::Array, item, _) => item,
			_ => ty,
		}
	}
}
