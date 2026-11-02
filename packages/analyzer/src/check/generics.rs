//! Generics (types brief §3.11): generic parameters of functions and types,
//! applications `Point(Float)`, inference at calls and typed literals, the
//! instantiations recorded for the backends (`Analysis::instantiations`).
//!
//! A generic declaration is checked once: in its body, a parameter is a
//! *rigid* `TypeKind::Param` whose values have what its constraint gives. At
//! a call, the *free* parameters of the callee's signature (those that are
//! not rigid in the code being walked) are inferred: explicit arguments, then
//! the expected type, then the arguments (lambdas last), then defaults.

use std::sync::Arc;

use crate::check::Walker;
use crate::diagnostics::codes::types::{
	GN003, GN004, GN005, GN007, GN010, GN011, GN014, GN015, TY003,
};
use crate::ids::{ListId, NodeId, ParamId, SymbolId, TypeId};
use crate::symbols::{DeclState, ScopeKind, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{Bindings, Candidates, ParamInfo, TypeFlags, TypeKind};

impl Walker<'_> {
	/// Declares the generic parameters of a declaration (`(Item, Value: C = D)`)
	/// in a new scope opened by `owner`, which becomes the current scope (the
	/// caller restores it). Returns the parameters, in order.
	pub(super) fn declare_generics(
		&mut self,
		owner: NodeId,
		name: &str,
		generics: NodeList<'_>,
	) -> Vec<ParamId> {
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(owner, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		if let Some(params) = self.generic_params.get(&owner) {
			return params.clone();
		}
		let key = self.nominal_key(&format!("{name}#generics"));
		let tree = Arc::clone(&self.tree);
		let mut params = Vec::with_capacity(generics.len());
		let mut defaulted = false;
		for (index, generic) in generics.iter().enumerate() {
			let (target, constraint, default) = match syntax::view(&tree, generic) {
				View::Name(_) => (generic, None, None),
				View::Assignment {
					target,
					annotation,
					value,
				} => (target, annotation, value),
				_ => {
					self.unsupported(generic, "this generic parameter");
					continue;
				}
			};
			let View::Name(param_name) = syntax::view(&tree, target) else {
				self.unsupported(generic, "this generic parameter");
				continue;
			};
			let text = tree.name(param_name).to_string();
			if text.chars().next().is_some_and(char::is_lowercase) {
				self.unsupported(generic, "compile-time value parameters");
				continue;
			}
			let sym = self.s.types.syms.intern(&text);
			if self.a.symbols.lookup_in(scope, sym).is_some() {
				let diagnostic = self.diagnostic(
					&GN004,
					target,
					format!("`{text}` is already a parameter of `{name}`"),
				);
				self.report(diagnostic);
				continue;
			}
			let id = self.s.types.add_param(ParamInfo {
				owner: key,
				index: index as u16,
				name: sym,
				constraint: TypeId::UNKNOWN,
				default: TypeId::UNSET,
			});
			let ty = self.s.types.intern(TypeKind::Param(id));
			let symbol = self.declare(generic, param_name, SymbolKind::TypeParameter);
			let symbol = self.a.symbols.symbol_mut(symbol);
			symbol.ty = ty;
			symbol.state = DeclState::Done;
			self.set_type(target, ty);
			// the constraint and the default may name the earlier parameters
			self.constraint_depth += 1;
			let constraint_ty = constraint.map_or(TypeId::UNKNOWN, |c| self.check_type_expr(c));
			self.constraint_depth -= 1;
			let default_ty = default.map_or(TypeId::UNSET, |d| self.check_type_expr(d));
			if !default_ty.is_unset()
				&& constraint_ty != TypeId::UNKNOWN
				&& !self.s.types.is_recovering(default_ty)
				&& !self.s.types.satisfies(default_ty, constraint_ty)
			{
				let message = format!(
					"the default `{}` doesn't satisfy the constraint `{}`",
					self.display(default_ty),
					self.display(constraint_ty)
				);
				let diagnostic = self.diagnostic(&GN003, generic, message);
				self.report(diagnostic);
			}
			if defaulted && default_ty.is_unset() {
				let diagnostic = self.diagnostic(
					&GN005,
					generic,
					format!("`{text}` comes after a parameter with a default: give it one too"),
				);
				self.report(diagnostic);
			}
			defaulted |= !default_ty.is_unset();
			self.s.types.set_param_bounds(id, constraint_ty, default_ty);
			params.push(id);
		}
		self.generic_params.insert(owner, params.clone());
		params
	}

	/// The generic parameters of a declared type, by its symbol.
	pub(super) fn type_generics(&self, symbol: SymbolId) -> Option<&Vec<ParamId>> {
		let decl = self.a.symbols.symbol(symbol).decl_node;
		self.generic_params
			.get(&decl)
			.filter(|params| !params.is_empty())
	}

	/// `Point(Float)`, `Result(Integer, String)` in a type: the instance of a
	/// generic type (`GN007` for a wrong number of arguments, `GN010` for a
	/// constraint). Type part.
	pub(super) fn generic_type_application(
		&mut self,
		n: NodeId,
		symbol: SymbolId,
		arguments: NodeList<'_>,
	) -> TypeId {
		let params = self.type_generics(symbol).cloned().unwrap_or_default();
		let generic = self.symbol_type(symbol, n);
		let args: Vec<TypeId> = arguments
			.iter()
			.map(|argument| self.check_type_expr(argument))
			.collect();
		self.memo(n, |w| {
			if args.iter().any(|&arg| w.s.types.is_recovering(arg))
				|| w.s.types.is_recovering(generic)
			{
				return TypeId::UNKNOWN;
			}
			let required = params
				.iter()
				.filter(|&&param| w.s.types.param(param).default.is_unset())
				.count();
			if args.len() < required || args.len() > params.len() {
				let message = format!(
					"`{}` takes {} type argument{}, not {}",
					w.display(generic),
					params.len(),
					if params.len() == 1 { "" } else { "s" },
					args.len()
				);
				let diagnostic = w.diagnostic(&GN007, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let mut bindings = Bindings::new(&params);
			for (index, &param) in params.iter().enumerate() {
				let ty = match args.get(index) {
					Some(&arg) => arg,
					None => {
						let default = w.s.types.param(param).default;
						w.s.types.substitute(default, &bindings)
					}
				};
				bindings.types[index] = ty;
			}
			let mut problems = Vec::new();
			w.check_constraints(&mut bindings, &mut problems);
			for (code, message) in problems {
				let diagnostic = w.diagnostic(code, n, message);
				w.report(diagnostic);
			}
			w.instantiate(generic, &bindings)
		})
	}

	/// The instance of a generic type for complete bindings, registered so
	/// that its functions are found (`decls_by_type`) and substituted.
	pub(super) fn instantiate(&mut self, generic: TypeId, bindings: &Bindings) -> TypeId {
		let instance = self.s.types.substitute(generic, bindings);
		if instance != generic {
			if let Some(name) = self.s.types.type_name(generic).map(str::to_string) {
				let args: Vec<String> = bindings.types.iter().map(|&ty| self.display(ty)).collect();
				self.s
					.types
					.name_type(instance, &format!("{name}({})", args.join(", ")));
			}
			let decls = self.declarations_of(generic);
			let entry = self.decls_by_type.entry(instance).or_default();
			for decl in decls {
				if !entry.contains(&decl) {
					entry.push(decl);
				}
			}
			self.instance_bindings
				.entry(instance)
				.or_insert_with(|| bindings.clone());
		}
		instance
	}

	/// `TY003`: a generic type used without its parameters.
	pub(super) fn missing_type_arguments(&mut self, n: NodeId, symbol: SymbolId) -> bool {
		let Some(params) = self.type_generics(symbol).cloned() else {
			return false;
		};
		let required: Vec<String> = params
			.iter()
			.filter(|&&param| self.s.types.param(param).default.is_unset())
			.map(|&param| {
				self.s
					.types
					.syms
					.resolve(self.s.types.param(param).name)
					.to_string()
			})
			.collect();
		if required.is_empty() {
			return false;
		}
		let name = self
			.s
			.types
			.syms
			.resolve(self.a.symbols.symbol(symbol).name)
			.to_string();
		let message = format!(
			"`{name}` needs its parameter{} {}: write `{name}({})`",
			if required.len() == 1 { "" } else { "s" },
			required
				.iter()
				.map(|r| format!("`{r}`"))
				.collect::<Vec<_>>()
				.join(", "),
			required.join(", ")
		);
		let diagnostic = self.diagnostic(&TY003, n, message);
		self.report(diagnostic);
		true
	}

	/// The parameters of a type that are free here: not the rigid parameters
	/// of the generic declarations being walked.
	pub(super) fn free_params(&self, ty: TypeId) -> Vec<ParamId> {
		let mut found = Vec::new();
		self.collect_params(ty, &mut found, 0);
		found.retain(|param| !self.rigid_params.contains(param));
		found
	}

	fn collect_params(&self, ty: TypeId, out: &mut Vec<ParamId>, depth: u32) {
		let types = &self.s.types;
		if depth > 32 || !types.flags(ty).contains(TypeFlags::HAS_PARAM) {
			if let TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) = types.kind(ty)
			{
				for &arg in types.list_items(types.nominal(nominal).args) {
					self.collect_params(arg, out, depth + 1);
				}
			}
			return;
		}
		match types.kind(ty) {
			TypeKind::Param(param) => {
				if !out.contains(&param) {
					out.push(param);
				}
			}
			TypeKind::Union(list) | TypeKind::Tuple(list) => {
				for &member in types.list_items(list) {
					self.collect_params(member, out, depth + 1);
				}
			}
			TypeKind::Object(shape) => {
				for field in types.shape_fields(shape) {
					self.collect_params(field.ty, out, depth + 1);
				}
			}
			TypeKind::Function(sig) => {
				let sig = types.sig(sig);
				for param in types.params(sig.params) {
					self.collect_params(param.ty, out, depth + 1);
				}
				self.collect_params(sig.ret, out, depth + 1);
				self.collect_params(sig.throws, out, depth + 1);
			}
			TypeKind::Collection(_, item, key) => {
				self.collect_params(item, out, depth + 1);
				self.collect_params(key, out, depth + 1);
			}
			_ => {}
		}
	}

	/// Turns the candidates of unification into bindings: every candidate of
	/// a parameter must be the same type once literals are widened (`GN014`).
	pub(super) fn resolve_candidates(
		&mut self,
		bindings: &mut Bindings,
		candidates: &Candidates,
		problems: &mut Vec<(&'static crate::diagnostics::CodeInfo, String)>,
	) {
		for (index, found) in candidates.found.iter().enumerate() {
			if !bindings.types[index].is_unset() || found.is_empty() {
				continue;
			}
			let mut chosen = TypeId::UNSET;
			for &candidate in found {
				// a parameter still being inferred elsewhere gives nothing
				if let TypeKind::Param(param) = self.s.types.kind(candidate)
					&& !self.rigid_params.contains(&param)
				{
					continue;
				}
				if self.s.types.is_recovering(candidate) {
					chosen = TypeId::UNKNOWN;
					break;
				}
				let candidate = self.join_member(candidate);
				if chosen.is_unset() {
					chosen = candidate;
				} else if chosen != candidate {
					if self.s.types.satisfies(candidate, chosen) {
						continue;
					}
					if self.s.types.satisfies(chosen, candidate) {
						chosen = candidate;
						continue;
					}
					let name = self.param_name(bindings.params[index]);
					problems.push((
						&GN014,
						format!(
							"`{name}` is `{}` here and `{}` there: give it explicitly, or make the values agree",
							self.display(chosen),
							self.display(candidate)
						),
					));
					chosen = TypeId::UNKNOWN;
					break;
				}
			}
			bindings.types[index] = chosen;
		}
	}

	/// The defaults of the parameters still unbound, then `GN015` for the
	/// others (bound to `UNKNOWN`).
	pub(super) fn finish_bindings(
		&mut self,
		bindings: &mut Bindings,
		problems: &mut Vec<(&'static crate::diagnostics::CodeInfo, String)>,
	) {
		for index in 0..bindings.params.len() {
			if !bindings.types[index].is_unset() {
				continue;
			}
			let param = bindings.params[index];
			let default = self.s.types.param(param).default;
			if !default.is_unset() {
				bindings.types[index] = self.s.types.substitute(default, bindings);
				continue;
			}
			let name = self.param_name(param);
			problems.push((
				&GN015,
				format!("`{name}` can't be inferred here: give it explicitly, ex: `(…)({name}Type)`, or annotate the result"),
			));
			bindings.types[index] = TypeId::UNKNOWN;
		}
	}

	/// Checks the constraints of complete bindings: a union constraint binds
	/// exactly one member (`GN010`, `GN011`), any other constraint is a
	/// supertype (`GN010`).
	pub(super) fn check_constraints(
		&mut self,
		bindings: &mut Bindings,
		problems: &mut Vec<(&'static crate::diagnostics::CodeInfo, String)>,
	) {
		for index in 0..bindings.params.len() {
			let ty = bindings.types[index];
			let param = bindings.params[index];
			let constraint = self.s.types.param(param).constraint;
			if constraint == TypeId::UNKNOWN
				|| constraint.is_unset()
				|| self.s.types.is_recovering(ty)
				|| ty.is_unset()
			{
				continue;
			}
			let constraint = self.s.types.substitute(constraint, bindings);
			let name = self.param_name(param);
			if let TypeKind::Union(list) = self.s.types.kind(constraint) {
				let members = self.s.types.list_items(list).to_vec();
				let fitting: Vec<TypeId> = members
					.into_iter()
					.filter(|&member| self.s.types.satisfies(ty, member))
					.collect();
				match fitting.as_slice() {
					[member] => bindings.types[index] = *member,
					[] => problems.push((
						&GN010,
						format!(
							"`{}` doesn't satisfy the constraint `{}` of `{name}`",
							self.display(ty),
							self.display(constraint)
						),
					)),
					_ => problems.push((
						&GN011,
						format!(
							"`{}` satisfies several members of the constraint `{}` of `{name}`: give it explicitly",
							self.display(ty),
							self.display(constraint)
						),
					)),
				}
			} else if !self.s.types.satisfies(ty, constraint) {
				problems.push((
					&GN010,
					format!(
						"`{}` doesn't satisfy the constraint `{}` of `{name}`",
						self.display(ty),
						self.display(constraint)
					),
				));
			}
		}
	}

	/// The members an operation on a value of type `ty` is checked against
	/// when `ty` is a type parameter with a union constraint (D19: once per
	/// member, the result being the same for every member or the member
	/// itself, then `ty`). `None` for any other type. For the arithmetic and
	/// comparison rules (E2).
	pub fn constraint_members(&self, ty: TypeId) -> Option<Vec<TypeId>> {
		let TypeKind::Param(param) = self.s.types.kind(ty) else {
			return None;
		};
		let constraint = self.s.types.param(param).constraint;
		if constraint == TypeId::UNKNOWN || constraint.is_unset() {
			return None;
		}
		Some(self.s.types.union_members(&constraint).to_vec())
	}

	/// The name of a type parameter, for messages.
	pub(super) fn param_name(&self, param: ParamId) -> String {
		self.s
			.types
			.syms
			.resolve(self.s.types.param(param).name)
			.to_string()
	}

	/// Infers the free parameters of a call (types brief §3.11.4): the
	/// expected type first (fixed), then the arguments whose parameter type
	/// still has free parameters, checked alone (*synthesized*, returned so
	/// that the caller doesn't check them again), lambdas last, then
	/// defaults and constraints. `arguments` are the value nodes and their
	/// declared parameter types.
	pub(super) fn infer_call(
		&mut self,
		arguments: &[(NodeId, Option<TypeId>)],
		sig: crate::types::FnSig,
		free: &[ParamId],
		expected: Option<TypeId>,
	) -> (
		Bindings,
		Vec<(&'static crate::diagnostics::CodeInfo, String)>,
		Vec<NodeId>,
	) {
		let mut bindings = Bindings::new(free);
		let mut problems = Vec::new();
		if let Some(expected) = expected
			&& !self.s.types.is_recovering(expected)
		{
			let mut candidates = Candidates::default();
			self.s
				.types
				.unify(sig.ret, expected, &bindings, &mut candidates);
			let mut ignored = Vec::new();
			self.resolve_candidates(&mut bindings, &candidates, &mut ignored);
			// a conflict with the expected type is the argument's problem
			for index in 0..bindings.types.len() {
				if bindings.types[index] == TypeId::UNKNOWN {
					bindings.types[index] = TypeId::UNSET;
				}
			}
		}
		let mut candidates = Candidates::default();
		let mut synthesized = Vec::new();
		let mut lambdas = Vec::new();
		for &(value, declared) in arguments {
			let Some(declared) = declared else {
				continue;
			};
			let current = self.s.types.substitute(declared, &bindings);
			if !self.has_free(current, free) {
				continue;
			}
			if matches!(self.view(value), View::Lambda { .. }) {
				lambdas.push((value, declared));
				continue;
			}
			let mut actual = self.check(value, None);
			synthesized.push(value);
			// a generic function given as a callback: instantiated by the callback type
			if !self.free_params(actual).is_empty()
				&& let Some(instance) = self.instantiate_function_value(value, actual, current)
			{
				actual = instance;
			}
			self.s
				.types
				.unify(declared, actual, &bindings, &mut candidates);
		}
		self.resolve_candidates(&mut bindings, &candidates, &mut problems);
		// lambdas once their parameter types are known (`map` infers `Result`)
		for (value, declared) in lambdas {
			let current = self.s.types.substitute(declared, &bindings);
			let actual = self.check(value, Some(current));
			synthesized.push(value);
			let mut candidates = Candidates::default();
			self.s
				.types
				.unify(declared, actual, &bindings, &mut candidates);
			self.resolve_candidates(&mut bindings, &candidates, &mut problems);
		}
		self.finish_bindings(&mut bindings, &mut problems);
		self.check_constraints(&mut bindings, &mut problems);
		(bindings, problems, synthesized)
	}

	/// Whether a type still has one of the `free` parameters.
	pub(super) fn has_free(&self, ty: TypeId, free: &[ParamId]) -> bool {
		let mut found = Vec::new();
		self.collect_params(ty, &mut found, 0);
		found.iter().any(|param| free.contains(param))
	}

	/// `sum(Float)`: the generic arguments of a generic function, when every
	/// argument is a type (types brief §3.11.2, D18). The instantiated
	/// function type; the call node records the arguments.
	pub(super) fn generic_function_application(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let View::Name(_) = self.view(callee) else {
			return None;
		};
		if arguments.is_empty()
			|| !arguments
				.iter()
				.all(|argument| self.is_type_argument(argument))
		{
			return None;
		}
		let ty = self.check(callee, None);
		let crate::symbols::Resolution::Local(symbol) = self.a.resolution(callee) else {
			return None;
		};
		let decl = self.direct_function(symbol)?;
		let params = self.a.function(decl)?.generics.clone();
		if params.is_empty() {
			return None;
		}
		let args: Vec<TypeId> = arguments
			.iter()
			.map(|argument| self.check_type_expr(argument))
			.collect();
		Some(self.memo(n, |w| {
			if args.len() > params.len() {
				let message = format!("this function takes {} type arguments", params.len());
				let diagnostic = w.diagnostic(&GN007, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let mut bindings = Bindings::new(&params);
			for (index, &arg) in args.iter().enumerate() {
				bindings.types[index] = arg;
			}
			let mut problems = Vec::new();
			w.finish_bindings(&mut bindings, &mut problems);
			w.check_constraints(&mut bindings, &mut problems);
			for (code, message) in problems {
				let diagnostic = w.diagnostic(code, n, message);
				w.report(diagnostic);
			}
			w.record_instantiation(n, &bindings);
			w.s.types.substitute(ty, &bindings)
		}))
	}

	/// Whether an argument is written as a type (D18): a type name, a type
	/// application, an object type, a union, an optional, a function type.
	/// Literals, object literals, lambdas and value names are values.
	pub(super) fn is_type_argument(&mut self, n: NodeId) -> bool {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Name(name) => {
				let text = tree.name(name);
				let sym = self.s.types.syms.intern(text);
				match self.a.symbols.lookup(self.body.scope, sym) {
					Some(symbol) => matches!(
						self.a.symbols.symbol(symbol).kind,
						SymbolKind::Type | SymbolKind::TypeParameter
					),
					None => crate::builtins::lookup(text).is_some_and(|builtin| {
						crate::builtins::denoted_type(&mut self.s.types, builtin).is_some()
					}),
				}
			}
			View::Call { callee, .. } => self.is_type_argument(callee),
			View::Union { .. } | View::Optional(_) | View::ErrorType => true,
			View::Object { items } => {
				!items.is_empty()
					&& items.iter().all(|item| {
						matches!(
							syntax::view(&tree, item),
							View::Assignment {
								annotation: Some(_),
								..
							} | View::Readonly(_)
						)
					})
			}
			View::Group(inner) => self.is_type_argument(inner),
			_ => false,
		}
	}

	/// A generic function used as a value where a function type is expected
	/// (`names.map(identity)`): instantiated by unifying the two types. The
	/// value node records the arguments. `None` when it is not one.
	pub(super) fn instantiate_function_value(
		&mut self,
		n: NodeId,
		actual: TypeId,
		target: TypeId,
	) -> Option<TypeId> {
		if !matches!(self.s.types.kind(actual), TypeKind::Function(_))
			|| !matches!(self.s.types.kind(target), TypeKind::Function(_))
		{
			return None;
		}
		let free = self.free_params(actual);
		if free.is_empty() {
			return None;
		}
		let bindings = Bindings::new(&free);
		let mut candidates = Candidates::default();
		self.s
			.types
			.unify(actual, target, &bindings, &mut candidates);
		let mut bindings = bindings;
		let mut problems = Vec::new();
		self.resolve_candidates(&mut bindings, &candidates, &mut problems);
		self.finish_bindings(&mut bindings, &mut problems);
		if !problems.is_empty() {
			return Some(TypeId::UNKNOWN);
		}
		self.record_instantiation(n, &bindings);
		Some(self.s.types.substitute(actual, &bindings))
	}

	/// Records the type arguments of a call or a typed literal for the backends.
	pub(super) fn record_instantiation(&mut self, n: NodeId, bindings: &Bindings) -> ListId {
		let list = self.s.types.list(&bindings.types);
		self.a.instantiations.insert(n, list);
		list
	}
}
