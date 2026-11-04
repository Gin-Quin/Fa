//! Requirements (advanced brief §9): `requires name: Type` in signatures,
//! `provide Type = value` statements, their resolution at each call.
//!
//! - The key of a requirement is its type (aliases are transparent, object
//!   types are structural). A signature's requirements are sorted by the
//!   stable hash of their key (decision D11): that order is the order of
//!   the hidden parameters the backends add after the regular ones.
//! - `provide K = value` makes `value` the provider of `K` from there to the
//!   end of its block; a place is provided as an alias, anything else as a
//!   temporary evaluated once.
//! - At a call, each requirement of the callee is the innermost visible
//!   `provide`, else the caller's own requirement of that key, else an
//!   inferred requirement of a single-expression function (a lambda looks
//!   in the code around it); else `REQ001`. `Analysis::requirement_args`
//!   records the choice.

use std::sync::Arc;

use crate::analysis::RequirementArg;
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{REQ001, REQ004, REQ007, REQ010};
use crate::ids::{NodeId, PlaceId, ScopeId, SymbolId, TypeId};
use crate::symbols::{DeclState, Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::Requirement;

/// A `provide` statement in effect.
#[derive(Copy, Clone, Debug)]
pub struct Provided {
	/// The `provide` node.
	pub node: NodeId,
	/// The key it provides.
	pub key: TypeId,
	/// The scope of its block: it is visible below it.
	pub scope: ScopeId,
	/// The place provided (an alias), `None` for a temporary.
	pub place: Option<PlaceId>,
}

/// Who provides a requirement of a callee at a call.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Provider {
	/// A `provide` statement.
	Provided(Provided),
	/// The enclosing function's own requirement: its symbol, `None` when it
	/// is inferred (a single-expression function).
	Own(Option<SymbolId>),
}

impl Walker<'_> {
	/// Declares the requirements of a signature in the function's scope, as
	/// symbols of kind `Requirement`, in the order of their keys' stable
	/// hashes (D11). Returns them in that order.
	pub(crate) fn declare_requirements(
		&mut self,
		requires: NodeList<'_>,
	) -> Vec<(Requirement, SymbolId)> {
		let tree = Arc::clone(&self.tree);
		let mut found: Vec<(Requirement, NodeId, NodeId)> = Vec::new();
		for node in requires.iter() {
			let View::Assignment {
				target,
				annotation: Some(annotation),
				value: None,
			} = syntax::view(&tree, node)
			else {
				continue;
			};
			let View::Name(name) = syntax::view(&tree, target) else {
				continue;
			};
			let ty = self.check_type_expr(annotation);
			let sym = self.s.types.syms.intern(tree.name(name));
			found.push((Requirement { name: sym, ty }, node, target));
		}
		found.sort_by_key(|(requirement, _, _)| self.s.types.stable_hash(requirement.ty));
		let mut declared = Vec::new();
		for (index, (requirement, node, target)) in found.into_iter().enumerate() {
			let symbol = match self.a.symbols.symbol_of_decl(node) {
				Some(symbol) => symbol,
				None => {
					let View::Name(name) = syntax::view(&tree, target) else {
						continue;
					};
					let symbol = self.declare(
						node,
						name,
						SymbolKind::Requirement {
							index: index as u16,
						},
					);
					let entry = self.a.symbols.symbol_mut(symbol);
					entry.ty = requirement.ty;
					entry.state = DeclState::Done;
					self.set_type(node, requirement.ty);
					symbol
				}
			};
			declared.push((requirement, symbol));
		}
		declared
	}

	/// `provide Key = value`: `value` provides `Key` until the end of the
	/// block (advanced brief §9.4).
	pub(super) fn check_provide(&mut self, n: NodeId, ty: NodeId, value: NodeId) -> TypeId {
		let key = self.check_type_expr(ty);
		let actual = self.check(value, Some(key));
		self.sink(value, actual, key);
		let place = match self.view(self.strip_groups(value)) {
			View::Name(_) | View::Access { .. } | View::Index { .. } => self.place_of(value),
			_ => None,
		};
		let provided = Provided {
			node: n,
			key,
			scope: self.body.scope,
			place,
		};
		// a `provide` walked again (a loop round) replaces its previous entry
		self.body.provides.retain(|entry| entry.node != n);
		self.body.provides.push(provided);
		self.memo(n, |_| TypeId::VOID)
	}

	/// The `provide` of `key` visible here: the latest one whose block
	/// encloses the current scope.
	fn visible_provide(&self, key: TypeId) -> Option<Provided> {
		self.body
			.provides
			.iter()
			.rev()
			.find(|entry| entry.key == key && self.scope_encloses(entry.scope, self.body.scope))
			.copied()
	}

	/// Whether `outer` is `inner` or one of its ancestors.
	fn scope_encloses(&self, outer: ScopeId, inner: ScopeId) -> bool {
		let mut current = Some(inner);
		while let Some(scope) = current {
			if scope == outer {
				return true;
			}
			current = self.a.symbols.scope(scope).parent;
		}
		false
	}

	/// Resolves the requirements of a call (advanced brief §9.3): records
	/// `Analysis::requirement_args`, reports the missing ones (`REQ001`)
	/// and returns the providers, in the callee's order.
	pub(crate) fn resolve_requirements(
		&mut self,
		n: NodeId,
		callee: &str,
		requires: &[Requirement],
	) -> Vec<Option<Provider>> {
		let mut providers = Vec::with_capacity(requires.len());
		let mut arguments = Vec::with_capacity(requires.len());
		let mut missing = Vec::new();
		for requirement in requires {
			let provider = match self.visible_provide(requirement.ty) {
				Some(provided) => Some(Provider::Provided(provided)),
				None => self.own_requirement(requirement),
			};
			// a lambda using a requirement of the code around it (REQ007)
			if let Some(cx) = self.body.functions.last()
				&& cx.symbol.is_none()
				&& !cx.test && match provider {
				Some(Provider::Provided(provided)) => {
					!self.scope_encloses(cx.scope, provided.scope)
				}
				Some(Provider::Own(_)) => true,
				None => false,
			} {
				let lambda = cx.node;
				let scope = match provider {
					Some(Provider::Provided(provided)) => Some(provided.scope),
					_ => None,
				};
				let entry = self.lambda_requirements.entry(lambda).or_default();
				if !entry.iter().any(|&(key, _, _)| key == requirement.ty) {
					entry.push((requirement.ty, n, scope));
				}
			}
			match provider {
				Some(Provider::Provided(provided)) => {
					arguments.push(RequirementArg::Provided(provided.node));
				}
				Some(Provider::Own(_)) => {
					arguments.push(RequirementArg::Own(requirement.ty));
				}
				None => {
					arguments.push(RequirementArg::Missing);
					missing.push(format!("`{}`", self.display(requirement.ty)));
				}
			}
			providers.push(provider);
		}
		if !missing.is_empty() && self.body.flow.reachable {
			let function = self.enclosing_function_text();
			let message = format!(
				"`{callee}` requires {}, which {} not provided: use `provide {} = …`, or declare `requires …` in {function}",
				missing.join(" and "),
				if missing.len() == 1 { "is" } else { "are" },
				missing[0].trim_matches('`'),
			);
			let diagnostic = self.diagnostic(&REQ001, n, message);
			self.report_flow(diagnostic);
		}
		if arguments
			.iter()
			.any(|arg| matches!(arg, crate::analysis::RequirementArg::Provided(_)))
		{
			// a `start do` around it tells it this way (tasks.rs)
			self.tasks.provided_calls += 1;
		}
		self.a.requirement_args.insert(n, arguments);
		providers
	}

	/// `REQ007` (advanced brief §9.6): a lambda stored in a function type
	/// (a field of an object, a place, a collection) is called later, where
	/// the requirements it resolved around it are not provided anymore; a
	/// function type can't declare requirements yet.
	pub(crate) fn check_stored_lambda(&mut self, value: NodeId) {
		let lambda = self.strip_groups(value);
		if !matches!(self.view(lambda), View::Lambda { .. }) {
			return;
		}
		let Some(&(key, call, _)) = self
			.lambda_requirements
			.get(&lambda)
			.and_then(|requirements| requirements.first())
		else {
			return;
		};
		if !self.reported_once.insert((lambda, REQ007.id)) {
			return;
		}
		let callee = match self.view(call) {
			View::Call { callee, .. } => self.tree.text(callee.index()).to_string(),
			_ => "a call".to_string(),
		};
		let message = format!(
			"this function is stored, but `{callee}` requires `{}`: a stored function's type must declare the requirements it needs, they are not captured. Pass the value as a parameter instead",
			self.display(key)
		);
		let diagnostic = self.diagnostic(&REQ007, lambda, message);
		self.report(diagnostic);
	}

	/// `REQ010` (advanced brief §9.6): the `Subscription` of a watch that
	/// uses a requirement provided in the function can't be its value: the
	/// provider ends with its block. `value` is the function's value.
	pub(crate) fn watch_returned(&mut self, value: NodeId) {
		let Some(function_scope) = self.body.functions.last().map(|cx| cx.scope) else {
			return;
		};
		let value = self.strip_groups(value);
		let watch = match self.view(value) {
			View::Watch(_) => value,
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(value) else {
					return;
				};
				let data = self.a.symbols.symbol(symbol);
				if !matches!(data.kind, SymbolKind::Let | SymbolKind::Mutable) {
					return;
				}
				let View::Declaration {
					value: Some(initial),
					..
				} = self.view(data.decl_node)
				else {
					return;
				};
				self.strip_groups(initial)
			}
			_ => return,
		};
		if !matches!(self.view(watch), View::Watch(_)) {
			return;
		}
		let Some(&(key, _, _)) = self
			.lambda_requirements
			.get(&watch)
			.and_then(|requirements| {
				requirements.iter().find(|(_, _, scope)| {
					scope.is_some_and(|scope| self.scope_encloses(function_scope, scope))
				})
			})
		else {
			return;
		};
		if !self.reported_once.insert((watch, REQ010.id)) {
			return;
		}
		let message = format!(
			"this watch uses `{}`, provided in this function: its `Subscription` can't leave the block that provides it. Provide it in the caller, or stop the watch here",
			self.display(key)
		);
		let diagnostic = self.diagnostic(&REQ010, watch, message);
		self.report(diagnostic);
	}

	/// The lambdas given as the fields of an object literal are stored.
	pub(crate) fn check_stored_lambdas(&mut self, items: NodeList<'_>) {
		for item in items.iter() {
			if let View::Assignment {
				value: Some(value), ..
			} = self.view(item)
			{
				self.check_stored_lambda(value);
			}
		}
	}

	/// The enclosing function's own requirement of a key: declared, or
	/// inferred for a single-expression function (a lambda passes the
	/// question to the code around it).
	fn own_requirement(&mut self, requirement: &Requirement) -> Option<Provider> {
		for position in (0..self.body.functions.len()).rev() {
			let cx = &self.body.functions[position];
			if let Some(&(_, symbol)) = cx
				.requires
				.iter()
				.find(|(declared, _)| declared.ty == requirement.ty)
			{
				return Some(Provider::Own(Some(symbol)));
			}
			if cx.symbol.is_none() && !cx.test {
				// a lambda: its requirements are those of where it is written
				continue;
			}
			if cx.infers_requires {
				let cx = &mut self.body.functions[position];
				if !cx.inferred_requires.contains(&requirement.ty) {
					cx.inferred_requires.push(requirement.ty);
				}
				return Some(Provider::Own(None));
			}
			return None;
		}
		None
	}

	/// "`main`", "this function" or "the top level", for messages.
	fn enclosing_function_text(&self) -> String {
		match self
			.body
			.functions
			.iter()
			.rev()
			.find(|cx| cx.symbol.is_some())
		{
			Some(cx) => {
				let name = self.a.symbols.symbol(cx.symbol.expect("named")).name;
				format!("`{}`", self.s.types.syms.resolve(name))
			}
			None if self.body.functions.is_empty() => "the enclosing function".to_string(),
			None => "this function".to_string(),
		}
	}

	/// `REQ004`: a callee mutates a requirement whose provider is a
	/// temporary or an immutable place. Returns the provider's place.
	pub(crate) fn mutated_requirement_place(
		&mut self,
		n: NodeId,
		callee: &str,
		provider: Option<Provider>,
	) -> Option<PlaceId> {
		match provider? {
			Provider::Own(None) => None,
			Provider::Own(Some(symbol)) => self.symbol_target(symbol),
			Provider::Provided(provided) => {
				let writable = provided.place.is_some_and(|place| {
					self.place_writable(place) == super::effects::Writable::Yes
				});
				if !writable {
					let key = self.display(provided.key);
					let message = match provided.place {
						None => format!(
							"`{callee}` mutates `{key}`, but the provided value is a temporary: declare it with `mutable value = …`, then `provide {key} = value`"
						),
						Some(_) => format!(
							"`{callee}` mutates `{key}`, but the provided value is immutable: declare it with `mutable`"
						),
					};
					let diagnostic = self.diagnostic(&REQ004, n, message);
					self.report_flow(diagnostic);
					return None;
				}
				provided.place
			}
		}
	}
}
