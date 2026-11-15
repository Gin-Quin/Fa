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
use crate::ids::{NodeId, PlaceId, SigId, SymbolId, TypeId};
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
	/// A destructured parameter `({ x, y }: Point)`: its pattern. The
	/// parameter is then an anonymous symbol named by the pattern's text.
	pattern: Option<NodeId>,
}

/// A default value, checked in the function's context: the value, its
/// expected type, and whether it is checked quietly (`FN020`).
type DeferredDefault = (NodeId, Option<TypeId>, bool);

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
	/// `async` (`tasks.rs`).
	is_async: bool,
}

impl Walker<'_> {
	/// `function name = …` (hoisted: runs once, see `Walker::hoisted`): sets
	/// the symbol's type. The value is a lambda, or a name (an alias, which
	/// keeps the named arguments and the defaults).
	pub(super) fn check_function_decl(
		&mut self,
		n: NodeId,
		name: Name,
		is_async: bool,
		generics: NodeList<'_>,
		annotation: Option<NodeId>,
		value: NodeId,
	) -> TypeId {
		// `Type(parameter)` in the signature: hidden generic parameters (structural.rs)
		let implicit = self.implicit_generics(n, value);
		if generics.is_empty() && implicit.is_empty() {
			return self.function_decl(n, is_async, annotation, value);
		}
		// `function first(Item) = …`: the parameters are rigid in the body
		let text = self.decl_text(n, name);
		let saved = self.body.scope;
		let mut params = if generics.is_empty() {
			Vec::new()
		} else {
			self.declare_generics(n, &text, generics)
		};
		params.extend(implicit);
		let rigid = self.rigid_params.len();
		self.rigid_params.extend(params.iter().copied());
		let derived = std::mem::take(&mut self.pending_derived);
		let ty = self.function_decl(n, is_async, annotation, value);
		// `Pick<Entity, keys>`: bound at each call from the others (type_ops.rs)
		params.extend(std::mem::replace(&mut self.pending_derived, derived));
		self.pending_implicit.clear();
		self.rigid_params.truncate(rigid);
		self.body.scope = saved;
		let lambda = match self.view(value) {
			View::Lambda { .. } => Some(value),
			_ => None,
		};
		for key in [Some(n), lambda].into_iter().flatten() {
			if let Some(info) = self.a.functions.get_mut(&key) {
				info.generics = params.clone();
			}
		}
		ty
	}

	/// A `function` declaration once its generic parameters are declared.
	fn function_decl(
		&mut self,
		n: NodeId,
		is_async: bool,
		annotation: Option<NodeId>,
		value: NodeId,
	) -> TypeId {
		let symbol = self.a.symbols.symbol_of_decl(n);
		let mut owner = FnOwner {
			declaration: Some(n),
			symbol,
			lenient: false,
			is_async,
		};
		let tree = Arc::clone(&self.tree);
		// `function length: (text: CString) => c.SizeT = @extern("strlen")` (foreign.rs)
		if self.is_extern_value(value) {
			return self.check_extern_decl(n, annotation, value);
		}
		let ty = match syntax::view(&tree, value) {
			// `function f: Identity = <T>(value: T): T => value`: a generic
			// function whose type must satisfy the annotation (phase 2)
			View::Lambda { signature, body }
				if !signature.generics.is_empty() && !syntax::is_lifted_lambda(&tree, n, value) =>
			{
				let declared = annotation.map(|annotation| self.check_type_expr(annotation));
				let saved_scope = self.body.scope;
				let (params, generics) = self.declare_lambda_generics(value, signature.generics);
				let expected = declared
					.and_then(|declared| self.expected_for_generic(declared, &generics))
					.and_then(|expected| self.signature_of(expected));
				let rigid = self.rigid_params.len();
				self.rigid_params.extend(params.iter().copied());
				let scope = self.body.scope;
				let inside_loop = self.body.inside_loop || !self.body.loops.is_empty();
				let saved = std::mem::replace(&mut self.body, BodyCx::new(scope));
				self.body.inside_loop = inside_loop;
				let ty = self.check_function(value, signature, body, owner, expected);
				self.body = saved;
				self.rigid_params.truncate(rigid);
				self.body.scope = saved_scope;
				for key in [n, value] {
					if let Some(info) = self.a.functions.get_mut(&key) {
						info.generics = params.clone();
					}
				}
				let ty = self.generic_function_type(ty, &generics);
				if let Some(declared) = declared
					&& !self.s.types.is_recovering(declared)
					&& !self.s.types.is_recovering(ty)
					&& self
						.s
						.types
						.assignable(ty, declared, crate::types::Mode::Read)
						.is_err()
				{
					let diagnostic = match self.generic_function_problem(value, ty, declared) {
						Some(diagnostic) => diagnostic,
						None => {
							let message = format!(
								"this function, a `{}`, doesn't satisfy its type `{}`",
								self.display(ty),
								self.display(declared)
							);
							self.diagnostic(&E_TYPE_1, value, message)
						}
					};
					self.report(diagnostic);
				}
				self.set_type(value, ty);
				// the function is called as its type says (the parameters of a
				// generic type are the function's, in order)
				match declared {
					Some(declared)
						if !self.s.types.is_recovering(declared)
							&& self.own_generics(declared).len() == generics.len() =>
					{
						declared
					}
					_ => ty,
				}
			}
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
				let inside_loop = self.body.inside_loop || !self.body.loops.is_empty();
				let saved = std::mem::replace(&mut self.body, BodyCx::new(scope));
				self.body.inside_loop = inside_loop;
				let ty = self.check_function(value, signature, body, owner, expected);
				self.body = saved;
				ty
			}
			// `function f = g then h`, `function f = chain[1..]` (compose.rs)
			View::Compose { .. } | View::Index { .. } if annotation.is_none() => {
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

	/// The generic parameters of a generic lambda or function, declared in a
	/// scope of its own (the caller restores the scope), with their `Param`
	/// types.
	fn declare_lambda_generics(
		&mut self,
		lambda: NodeId,
		generics: NodeList<'_>,
	) -> (Vec<crate::ids::ParamId>, Vec<TypeId>) {
		let params = self.declare_generics(lambda, "this function", generics);
		let types = params
			.iter()
			.map(|&param| self.s.types.intern(TypeKind::Param(param)))
			.collect();
		(params, types)
	}

	/// An expected generic function type read with a generic lambda's own
	/// parameters (`<T>(value: T) => T` for `<U>(value) => value`), so that the
	/// lambda takes its parameter types from it. `None` when it isn't one with
	/// as many parameters.
	fn expected_for_generic(&mut self, expected: TypeId, own: &[TypeId]) -> Option<TypeId> {
		let TypeKind::Function(sig) = self.s.types.kind(expected) else {
			return None;
		};
		let theirs = self
			.s
			.types
			.list_items(self.s.types.sig(sig).generics)
			.to_vec();
		if theirs.is_empty() || theirs.len() != own.len() {
			return None;
		}
		let params: Vec<crate::ids::ParamId> = theirs
			.iter()
			.filter_map(|&ty| match self.s.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect();
		let mut bindings = crate::types::Bindings::new(&params);
		bindings.types = own.to_vec();
		Some(self.s.types.substitute(expected, &bindings))
	}

	/// The generic function type of a function whose type `ty` is written with
	/// its own parameters `generics`.
	pub(super) fn generic_function_type(&mut self, ty: TypeId, generics: &[TypeId]) -> TypeId {
		let TypeKind::Function(sig) = self.s.types.kind(ty) else {
			return ty;
		};
		let sig = self.s.types.sig(sig);
		let generics = self.s.types.list(generics);
		self.s.types.function(FnSig { generics, ..sig })
	}

	/// A generic lambda, `<T>(value: T) => value` (phase 2 of "Generic
	/// functions carry their list on the function value"): its parameters are
	/// rigid in its body, and its type is a generic function type. An expected
	/// generic function type gives the types of its parameters; an expected
	/// function type that isn't generic instantiates it (`sink`).
	pub(super) fn check_generic_lambda(
		&mut self,
		n: NodeId,
		signature: SignatureView<'_>,
		body: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let saved = self.body.scope;
		let (params, generics) = self.declare_lambda_generics(n, signature.generics);
		let expected = expected.and_then(|expected| self.expected_for_generic(expected, &generics));
		let rigid = self.rigid_params.len();
		self.rigid_params.extend(params.iter().copied());
		let ty = self.check_lambda(n, signature, body, expected);
		self.rigid_params.truncate(rigid);
		self.body.scope = saved;
		if let Some(info) = self.a.functions.get_mut(&n) {
			info.generics = params;
		}
		let ty = self.generic_function_type(ty, &generics);
		self.set_type(n, ty);
		ty
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
		// it sees the facts of its creation (values brief D-102) that stay
		// valid at each of its calls: the ones of the places it (or a sibling
		// lambda of the call) writes go, as at a loop head (decision "After
		// the analyzer review"); `break`, `continue` and `return` don't cross it
		// (only the arguments of the call see the writes of their siblings)
		let entry_writes = std::mem::take(&mut self.lambda_entry_writes);
		let mut dropped: Vec<(PlaceId, bool, NodeId)> = entry_writes.clone();
		let mut rounds = 0u32;
		let ty = loop {
			rounds += 1;
			let inner = BodyCx {
				scope: self.body.scope,
				flow: self.body.flow.fork(),
				functions: Vec::new(),
				loops: Vec::new(),
				flow_buffers: std::mem::take(&mut self.body.flow_buffers),
				requirements: std::mem::take(&mut self.body.requirements),
				handlers: Vec::new(),
				inside_loop: self.body.inside_loop || !self.body.loops.is_empty(),
				provides: self.body.provides.clone(),
			};
			let saved = std::mem::replace(&mut self.body, inner);
			let entry = self.body.flow.clone();
			for &(place, invalidates, at) in &dropped {
				self.drop_entry_facts(place, invalidates, at);
			}
			self.push_flow_buffer();
			// `async` written, or expected (tasks.rs)
			let is_async = self.lambda_is_async(n, signature.is_async, false, expected);
			let owner = FnOwner {
				declaration: None,
				symbol: None,
				lenient,
				is_async,
			};
			let ty = self.check_function(n, signature, body, owner, expected);
			// the places it writes whose facts it saw: walked again without them
			let written: Vec<(PlaceId, bool, NodeId)> = self
				.closure_effects
				.get(&n)
				.map(|entries| {
					entries
						.iter()
						.map(|entry| (entry.place, entry.invalidates, entry.node))
						.collect()
				})
				.unwrap_or_default();
			let mut again = false;
			if rounds < 3 {
				for (place, invalidates, at) in written {
					if dropped
						.iter()
						.any(|&(other, both, _)| other == place && both == invalidates)
					{
						continue;
					}
					let saved_flow = std::mem::replace(&mut self.body.flow, entry.clone());
					self.drop_entry_facts(place, invalidates, at);
					let changed = self.body.flow != entry;
					self.body.flow = saved_flow;
					if changed {
						dropped.push((place, invalidates, at));
						again = true;
					}
				}
			}
			self.pop_flow_buffer(!again);
			let inner = std::mem::replace(&mut self.body, saved);
			self.body.flow_buffers = inner.flow_buffers;
			self.body.requirements = inner.requirements;
			if !again {
				break ty;
			}
		};
		// its writes of captured places happen where it is created: the facts
		// of the written places go (effects.rs)
		if let Some(entries) = self.closure_effects.remove(&n) {
			for entry in entries {
				self.drop_place_facts(entry.place);
				self.record_effect(entry.place, entry.invalidates, entry.node, entry.call);
				if let Some(writes) = &mut self.lambda_call_writes {
					writes.push((entry.place, entry.invalidates, entry.node));
				}
			}
		}
		self.lambda_entry_writes = entry_writes;
		// Captures are borrowed by a temporary callback, copied by an
		// escaping closure. A Task cannot satisfy that capture-copy.
		let captured: Vec<SymbolId> = self
			.a
			.functions
			.get(&n)
			.map(|info| info.captures.clone())
			.unwrap_or_default();
		for symbol in captured {
			self.note_use(n, symbol);
		}
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
		// a callback of a parallel view: the rules of a parallel body
		// (concurrency.rs), its own locals and parameters being the iteration's
		let parallel_callback = self.parallel_callbacks.contains(&lambda);
		if parallel_callback {
			self.parallel.push(super::concurrency::ParallelFrame {
				node: lambda,
				scope,
				pattern: None,
				range: false,
				function_depth: self.body.functions.len() + 1,
				indexed: Vec::new(),
				through_variable: false,
				callback: true,
			});
		}

		// 1. the signature
		let (params, incomplete, defaults) =
			self.check_params(lambda, signature.parameters, expected, owner.lenient);
		// the parameters named by `Type(…)` are declared: the body's lambdas are not
		self.pending_implicit.clear();
		let declared_return = signature
			.returns
			.map(|returns| self.check_type_expr(returns));
		// errors are thrown, never returned (errors.md, `E-ERR-7`)
		if let (Some(returns), Some(ty)) = (signature.returns, declared_return)
			&& let Some(error) = self.error_member(ty)
			&& self.reported_once.insert((returns, "E-ERR-7"))
		{
			let error = self.s.types.display(error);
			let shown = self.s.types.display(ty);
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::values::E_ERR_7,
				returns,
				format!(
					"errors are never part of the return type: remove `{error}` from `{shown}`, and declare `throws {error}`"
				),
			);
			self.report(diagnostic);
		}
		// a return type with parameters still being inferred is inferred from the body
		let expected_return = expected
			.map(|sig| sig.ret)
			.filter(|&ret| !ret.is_unset() && self.free_params(ret).is_empty());
		let returns = declared_return.or(expected_return);
		let annotation_throws = owner.declaration.and(expected).map(|sig| sig.throws);
		let throws_contract = signature
			.throws
			.map(|throws| self.check_type_expr(throws))
			.or(annotation_throws)
			.or_else(|| (block_body && owner.declaration.is_some()).then_some(TypeId::NEVER));
		let throws = throws_contract.unwrap_or(TypeId::NEVER);
		// `requires name: Type` (requirements.rs)
		let requires = self.declare_requirements(signature.requires);
		let mut declared_requires: Vec<crate::types::Requirement> = requires
			.iter()
			.map(|(requirement, _)| *requirement)
			.collect();
		if !declared_requires.is_empty() && self.is_worker_method(signature.parameters) {
			// its callers are on another thread: nothing can provide them
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::advanced::REQ006,
				lambda,
				"a worker method can't declare `requires`: it runs on the worker's thread, where nothing provides them. Pass the values as parameters, or keep them in the worker's state",
			);
			self.report(diagnostic);
			declared_requires.clear();
		}
		// `mutates` and `invalidates` (effects.rs)
		let written_effects = !signature.mutates.is_empty() || !signature.invalidates.is_empty();
		let annotation_effects = owner.declaration.and(expected).map(|sig| sig.effects);
		let has_effect_contract = written_effects || annotation_effects.is_some();
		let declared_effects = if written_effects {
			self.declared_effects(lambda, signature.mutates, signature.invalidates)
		} else {
			annotation_effects.unwrap_or(crate::ids::EffectSetId::EMPTY)
		};
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
				returns.map(|ret| {
					self.function_type(
						&param_sigs,
						ret,
						throws,
						declared_effects,
						&declared_requires,
						owner.is_async,
					)
				})
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
				generics: Vec::new(),
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
		// a named function with a block body or written effects declares them all
		// the entry function of a program infers its effects (advanced brief D51)
		let entry = owner.declaration.is_some_and(|declaration| {
			matches!(self.view(declaration), View::ExportFunction { .. })
				&& (self.s.role(self.file) == crate::modules::FileRole::Program
					|| self.s.is_entry(self.file))
		});
		let coverage = (has_effect_contract || (!entry && block_body && owner.symbol.is_some()))
			.then_some(declared_effects);
		self.body.functions.push(FnCx {
			infer_returns: owner.declaration.is_none() && returns.is_none(),
			unsupported_at_start: self.unsupported_seen,
			declared: has_effect_contract.then_some(declared_effects),
			extra_effect_contract: annotation_effects
				.filter(|&expected| written_effects && expected != declared_effects),
			throws_contract,
			coverage,
			signature_node: signature
				.mutates
				.first()
				.or(signature.invalidates.first())
				.or(owner.declaration.filter(|_| annotation_effects.is_some())),
			requires: requires.clone(),
			infers_requires: !block_body && owner.symbol.is_some(),
			is_async: owner.is_async,
			..FnCx::new(lambda, owner.symbol, mode, scope)
		});
		// `: T in owner`: a parameter or a module-level value (references.rs)
		let ref_owner = signature
			.returns
			.and_then(|returns| self.reference_return_owner(returns));
		if let Some(cx) = self.body.functions.last_mut() {
			cx.ref_owner = ref_owner;
		}
		// a default value runs inside the function: its errors and effects
		// are the function's (defining-functions.md)
		for (default, expected, quiet) in defaults {
			self.quiet += u32::from(quiet);
			let actual = self.check(default, expected);
			self.note_copy_site(default);
			if let Some(expected) = expected {
				self.sink(default, actual, expected);
			}
			self.quiet -= u32::from(quiet);
		}
		let body_ty = if block_body {
			self.check_block_body(body, mode, owner)
		} else {
			self.check_expression_body(body, mode)
		};
		let cx = self.body.functions.pop();
		if parallel_callback {
			self.parallel.pop();
		}
		self.body.scope = parent;
		if let Some(cx) = &cx {
			self.finish_captures(cx, lambda, owner.declaration);
			// its async summary (tasks.rs)
			self.finish_function_async(cx, lambda, owner.declaration, super::tasks::Mode::Created);
		}
		// a closure given as the value of a function escapes
		let value_node = match self.view(body) {
			View::Block { statements } => statements.last(),
			_ => Some(body),
		};
		if let Some(value_node) = value_node {
			if matches!(
				self.view(self.strip_groups(value_node)),
				View::Lambda { .. }
			) {
				self.escape(value_node, value_node);
			}
			// the value of the function is a copy, maybe a move (moves.rs)
			if !matches!(mode, TypeId::VOID | TypeId::NEVER) {
				self.note_copy_site(value_node);
				if !block_body || !matches!(self.view(value_node), View::Return(_)) {
					self.check_returned_reference(
						value_node,
						cx.as_ref().and_then(|cx| cx.ref_owner),
					);
				}
			}
			// a task it returns can't use its locals (tasks.rs)
			if let Some(cx) = &cx {
				let saved = std::mem::take(&mut self.body.functions);
				self.body.functions.push(cx.clone());
				self.task_returned(self.strip_groups(value_node));
				// a watch it returns can't outlive its providers (requirements.rs)
				self.watch_returned(value_node);
				self.body.functions = saved;
			}
		}
		// its effects: declared, or inferred from the body (effects.rs)
		let effects = match &cx {
			Some(cx) => self.finish_effects(cx, owner.declaration),
			None => declared_effects,
		};
		// a lambda's type doesn't carry the effects it infers: they happen
		// where it is created (the callee it is given to runs it)
		let effects = if owner.declaration.is_none() && !written_effects {
			crate::ids::EffectSetId::EMPTY
		} else {
			effects
		};
		// declared requirements, and the inferred ones of a single expression
		let mut all_requires = declared_requires.clone();
		if let Some(cx) = &cx {
			for &key in &cx.inferred_requires {
				if !all_requires.iter().any(|requirement| requirement.ty == key) {
					let name = self
						.s
						.types
						.syms
						.intern(&self.s.types.display(key).to_lowercase());
					all_requires.push(crate::types::Requirement { name, ty: key });
				}
			}
		}
		all_requires.sort_by_key(|requirement| self.s.types.stable_hash(requirement.ty));
		// a single-expression body without `throws` infers its errors (errors.rs)
		let thrown = cx.as_ref().map_or(TypeId::NEVER, |cx| cx.throws);
		let throws = throws_contract.unwrap_or(thrown);

		// Error sets depend on flow (including sibling callbacks' writes),
		// so the check belongs to the retained flow round, not the type memo.
		if cx
			.as_ref()
			.is_some_and(|cx| cx.unsupported_at_start == self.unsupported_seen)
		{
			let at = signature.throws.or(owner.declaration).unwrap_or(lambda);
			if let Some(declared) = throws_contract {
				self.check_declared_throws(at, declared, thrown);
			}
			if let Some(declared) =
				annotation_throws.filter(|declared| Some(*declared) != throws_contract)
			{
				self.check_declared_throws(owner.declaration.unwrap_or(at), declared, thrown);
			}
		}
		let mut ty = self.memo(lambda, |w| {
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
			w.check_stored_iterator_type(signature.returns.unwrap_or(lambda), ret, "returned");
			if incomplete {
				TypeId::UNKNOWN
			} else {
				w.function_type(
					&param_sigs,
					ret,
					throws,
					effects,
					&all_requires,
					owner.is_async,
				)
			}
		});
		if let TypeKind::Function(sig) = self.s.types.kind(ty) {
			let mut sig = self.s.types.sig(sig);
			if sig.throws != throws {
				sig.throws = throws;
				ty = self.s.types.function(sig);
				self.set_type(lambda, ty);
			}
		}
		let sig = self.signature_id(ty);
		if self.is_worker_method(signature.parameters) {
			self.check_worker_signature(lambda, ty);
		}
		for key in [Some(lambda), owner.declaration].into_iter().flatten() {
			if let Some(info) = self.a.functions.get_mut(&key) {
				info.ty = ty;
				info.sig = sig;
				info.throws = throws;
			}
		}
		if let Some(symbol) = owner.symbol {
			self.a.symbols.symbol_mut(symbol).ty = ty;
		}
		ty
	}

	/// The function type of a signature.
	fn function_type(
		&mut self,
		params: &[ParamSig],
		ret: TypeId,
		throws: TypeId,
		effects: crate::ids::EffectSetId,
		requires: &[crate::types::Requirement],
		is_async: bool,
	) -> TypeId {
		let base = self.s.types.simple_sig(&[], ret, throws);
		let params = self.s.types.param_list(params);
		let requires = self.s.types.req_list(requires);
		self.s.types.function(FnSig {
			params,
			effects,
			requires,
			is_async,
			..base
		})
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
	) -> (Vec<(FnParam, ParamSig)>, bool, Vec<DeferredDefault>) {
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
		// the default values, checked in the function's context
		let mut defaults = Vec::new();
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
				let expected = (!declared.is_unset()).then_some(declared);
				defaults.push((default, expected, later_use.is_some()));
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
			if let Some(pattern) = param.pattern {
				let mut names = super::destructuring::PatternNames::default();
				self.destructure(pattern, body_ty, SymbolKind::Let, None, &mut names);
			}
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
		(params, incomplete, defaults)
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
		if let Some((_, declaration)) = owner
			&& let Some(origin) = self.node_key(declaration)
		{
			let mut written = crate::analysis::WrittenType::named(origin);
			let params = self
				.generic_params
				.get(&declaration)
				.cloned()
				.unwrap_or_default();
			let types: Vec<_> = params
				.into_iter()
				.map(|param| self.s.types.intern(TypeKind::Param(param)))
				.collect();
			written.arguments = self.s.types.list(&types);
			self.written_type_keys.insert(symbol, written);
		}
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
		// named by `Type(parameter)`: the type of its argument (structural.rs)
		if !param.variadic && !self.s.types.is_recovering(ty) {
			let sym = self.s.types.syms.intern(&text);
			ty = self.implicit_parameter_type(sym, ty);
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
		if !param.variadic {
			// its methods are those of its written type (structural.rs)
			self.record_written_type(symbol, param.annotation);
		}
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
			// `({ x, y }: Point)` (destructuring.rs)
			View::DestructuredParameter {
				pattern,
				annotation,
			} => {
				let span = self.tree.spans[pattern.index()];
				return Some(ParamSyntax {
					node,
					name_node: node,
					name: Name::new(span.start, span.end),
					annotation: Some(annotation),
					default: None,
					variadic: false,
					pattern: Some(pattern),
				});
			}
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
			pattern: None,
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
					// an unreachable end gives no value (W-DO-2 reports the dead code)
					Some(last) if ty != TypeId::NEVER && self.body.flow.reachable => {
						self.sink(last, ty, returns)
					}
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
	pub(super) fn is_pure_expression(&self, n: NodeId) -> bool {
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
				self.note_copy_site(value);
				let owner = self.body.functions.last().and_then(|cx| cx.ref_owner);
				self.check_returned_reference(value, owner);
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

	/// `f with (y = 0)`
	pub(super) fn check_partial_application(
		&mut self,
		n: NodeId,
		_function: NodeId,
		_arguments: NodeList<'_>,
	) -> TypeId {
		self.unsupported(n, "`with`")
	}

	/// The `function` declaration with a signature that a call of `symbol`
	/// reaches: the function itself, or the target of an alias
	/// (`function f = add`), which keeps the named arguments and the defaults.
	/// At the entry of a lambda that writes `place`: the facts of the place
	/// go, and the references the write invalidates may be invalid (the
	/// lambda may run again after its write).
	fn drop_entry_facts(&mut self, place: PlaceId, invalidates: bool, n: NodeId) {
		self.drop_place_facts(place);
		if self.body.flow.refs.is_empty() {
			return;
		}
		let before = self.body.flow.refs.clone();
		self.invalidate_references(place, invalidates, n);
		let after = self.body.flow.refs.clone();
		for (symbol, state) in after {
			if before
				.iter()
				.any(|&(other, old)| other == symbol && old == state)
			{
				continue;
			}
			let mut current = self.facts.refs.get(state).clone();
			if let Some((by, _)) = current.invalidated {
				current.invalidated = Some((by, true));
				let next = self.facts.refs.state(current);
				self.body.flow.set_ref_state(symbol, next);
			}
		}
	}

	/// An `Errors` type of a type or of its union members.
	fn error_member(&self, ty: TypeId) -> Option<TypeId> {
		match self.s.types.kind(ty) {
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _)
				if self.s.types.nominal(nominal).kind == crate::types::NominalKind::Errors =>
			{
				Some(ty)
			}
			TypeKind::Union(list) => self
				.s
				.types
				.list_items(list)
				.iter()
				.find_map(|&member| self.error_member(member)),
			_ => None,
		}
	}

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
