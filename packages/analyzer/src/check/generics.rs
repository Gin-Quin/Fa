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
	GN003, GN004, GN005, GN007, GN010, GN011, GN013, GN014, GN015, GN020, GN031, TY003,
};
use crate::ids::{ListId, NodeId, ParamId, SymbolId, TypeId};
use crate::symbols::{DeclState, ScopeKind, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{Bindings, Candidates, ParamInfo, TypeFlags, TypeKind};

impl Walker<'_> {
	/// An old application is still analyzed for error recovery and the
	/// migration command, but never accepted as valid Fa.
	pub(super) fn check_generic_brackets(&mut self, n: NodeId) {
		if syntax::is_angle_application(&self.tree, n) || !self.a.type_of(n).is_unset() {
			return;
		}
		let View::Call { callee, arguments } = self.view(n) else {
			return;
		};
		let callee = self.tree.text(callee.index());
		let arguments = arguments
			.iter()
			.map(|argument| self.tree.text(argument.index()))
			.collect::<Vec<_>>()
			.join(", ");
		let diagnostic = self.diagnostic(
			&GN031,
			n,
			format!(
				"Generic arguments use angle brackets: write `{callee}<{arguments}>`. Run `fa migrate-generics --write <path>` to update existing code"
			),
		);
		self.report(diagnostic);
	}

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
			let value = text.chars().next().is_some_and(char::is_lowercase);
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
			let id = self.s.types.add_param_pending(ParamInfo {
				owner: key,
				index: index as u16,
				name: sym,
				constraint: TypeId::UNKNOWN,
				default: TypeId::UNSET,
				value,
			});
			if value {
				// `rows: Integer`: a compile-time value, a constant in the body
				self.declare_value_param(id, generic, target, param_name, constraint, default);
				params.push(id);
				continue;
			}
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
	pub(super) fn type_generics(&mut self, symbol: SymbolId) -> Option<Vec<ParamId>> {
		let decl = self.a.symbols.symbol(symbol).decl_node;
		if let Some(params) = self.generic_params.get(&decl) {
			return (!params.is_empty()).then(|| params.clone());
		}
		// a type of another file (`library.Pair`)
		if self.a.symbols.symbol(symbol).kind != SymbolKind::Type {
			return None;
		}
		let params = self.exported_generics(symbol);
		(!params.is_empty()).then_some(params)
	}

	/// The generic parameters of a type or function declaration of this
	/// file (what its export publishes), by its symbol.
	pub(super) fn declared_generics(&mut self, symbol: SymbolId) -> Vec<ParamId> {
		let decl = self.a.symbols.symbol(symbol).decl_node;
		if let Some(params) = self.generic_params.get(&decl) {
			return params.clone();
		}
		match self.direct_function(symbol) {
			Some(function) => self
				.a
				.function(function)
				.map(|info| info.generics.clone())
				.unwrap_or_default(),
			None => Vec::new(),
		}
	}

	/// The generic parameters of the declaration of another file a proxy
	/// symbol stands for.
	pub(super) fn exported_generics(&mut self, symbol: SymbolId) -> Vec<ParamId> {
		let Some(exported) = self.proxy_exported(symbol) else {
			return Vec::new();
		};
		self.s
			.types
			.list_items(exported.generics)
			.iter()
			.filter_map(|&generic| match self.s.types.kind(generic) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect()
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
		let params = self.type_generics(symbol).unwrap_or_default();
		let generic = self.symbol_type(symbol, n);
		// a type function with logic: its result is selected (type_logic.rs)
		let logic = self.type_logic_of(symbol);
		// constants for the value parameters (value_params.rs)
		let args = self.generic_arguments(&params, arguments);
		self.memo(n, |w| {
			if args.iter().any(|&arg| w.s.types.is_recovering(arg))
				|| (w.s.types.is_recovering(generic) && logic.is_none())
			{
				return TypeId::UNKNOWN;
			}
			// inside its own declaration: only with its own parameters (types brief §3.6)
			if let TypeKind::Rec(rec) = w.s.types.kind(generic)
				&& w.rec_references.get(&symbol) == Some(&generic)
			{
				let own = w.s.types.rec_info(rec).args;
				// the whole definition is the application: a type function
				// applying itself at new arguments never stops (TF001)
				let whole = match syntax::view(&w.tree, w.a.symbols.symbol(symbol).decl_node) {
					View::TypeDecl { value, .. } => w.strip_groups(value) == n,
					_ => false,
				};
				if whole && w.s.types.list_items(own) != args.as_slice() {
					let name = w.display(generic);
					let message = format!(
						"`{name}` applies itself at new arguments each time: its applications nest without end (more than 256)"
					);
					let diagnostic = w.diagnostic(&crate::diagnostics::codes::types::TF001, n, message);
					w.report(diagnostic);
					return TypeId::UNKNOWN;
				}
				if w.s.types.list_items(own) != args.as_slice() {
					let name = w.display(generic);
					let message = format!(
						"`{name}` refers to itself with other type arguments (polymorphic recursion): use its own parameters, `{name}`"
					);
					let diagnostic = w.diagnostic(&GN020, n, message);
					w.report(diagnostic);
					return TypeId::UNKNOWN;
				}
				return generic;
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
			w.check_constraints(&mut bindings, &mut problems, false);
			for (code, message) in problems {
				let diagnostic = w.diagnostic(code, n, message);
				w.report(diagnostic);
			}
			if let Some(logic) = logic {
				return w.apply_type_logic(n, symbol, logic, &bindings);
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
				self.s.types.name_type(
					instance,
					&crate::types::display::type_application(&name, &args),
				);
				// a substitution of the instance names its result too
				self.s
					.types
					.record_application(instance, &name, &bindings.types);
			}
			// a generic type of another file (`library.Pair<Integer, String>`):
			// its functions are found from the instance too
			let own = self.s.module_id(self.file);
			let origins: Vec<crate::ids::DeclKey> = self
				.s
				.type_origins(generic, |module| module != own)
				.collect();
			for origin in origins {
				self.s.record_type_origin(instance, origin);
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
		let Some(params) = self.type_generics(symbol) else {
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
			"`{name}` needs its parameter{} {}: write `{name}<{}>`",
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
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				for &arg in types.list_items(types.nominal(nominal).args) {
					self.collect_params(arg, out, depth + 1);
				}
			}
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
				let mut inner = Vec::new();
				for param in types.params(sig.params) {
					self.collect_params(param.ty, &mut inner, depth + 1);
				}
				self.collect_params(sig.ret, &mut inner, depth + 1);
				self.collect_params(sig.throws, &mut inner, depth + 1);
				// the parameters of a generic function type are bound by it
				let bound = types.list_items(sig.generics);
				for param in inner {
					let is_bound = bound
						.iter()
						.any(|&generic| types.kind(generic) == TypeKind::Param(param));
					if !is_bound && !out.contains(&param) {
						out.push(param);
					}
				}
			}
			TypeKind::Collection(_, item, key) => {
				self.collect_params(item, out, depth + 1);
				self.collect_params(key, out, depth + 1);
			}
			TypeKind::Pointer(item) | TypeKind::CArray(item, _) => {
				self.collect_params(item, out, depth + 1);
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
			// `Pick<Entity, keys>`: from the others (type_ops.rs)
			if let Some(derived) = self.a.derived_params.get(&param).copied() {
				bindings.types[index] = self.derived_binding(derived, bindings);
				continue;
			}
			let default = self.s.types.param(param).default;
			if !default.is_unset() {
				bindings.types[index] = self.s.types.substitute(default, bindings);
				continue;
			}
			let name = self.param_name(param);
			problems.push((
				&GN015,
				format!("`{name}` can't be inferred here: give it explicitly after the name of the function, ex: `f<{name}Type>(…)`, or annotate the result"),
			));
			bindings.types[index] = TypeId::UNKNOWN;
		}
	}

	/// Checks the constraints of complete bindings: a union constraint binds
	/// exactly one member (`GN010`, `GN011`), any other constraint is a
	/// supertype (`GN010`). With `values` (a call inferring the parameters
	/// from its arguments), an argument lacking a field of an object
	/// constraint is `STR001` (advanced brief §10.2).
	pub(super) fn check_constraints(
		&mut self,
		bindings: &mut Bindings,
		problems: &mut Vec<(&'static crate::diagnostics::CodeInfo, String)>,
		values: bool,
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
			// a parameter of the generic code around, whose own constraint is
			// as narrow (`double(value)` in `<Value: Integer | Float>`): bound
			// to it, each instance then binds it to one member
			if let TypeKind::Union(_) = self.s.types.kind(constraint)
				&& let TypeKind::Param(own) = self.s.types.kind(ty)
			{
				let own = self.s.types.param(own).constraint;
				if own != TypeId::UNKNOWN
					&& !own.is_unset()
					&& self.s.types.satisfies(own, constraint)
				{
					continue;
				}
			}
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
			} else if let Some(message) = self.missing_method(ty, constraint, &name) {
				problems.push((&GN013, message));
			} else if values
				&& self
					.s
					.types
					.assignable(ty, constraint, crate::types::Mode::Read)
					== Err(crate::types::Reason::MissingField)
			{
				let missing = self
					.s
					.types
					.fields_of(constraint)
					.iter()
					.find(|field| {
						!self
							.s
							.types
							.fields_of(ty)
							.iter()
							.any(|other| other.name == field.name)
					})
					.map(|field| {
						format!(
							"{}: {}",
							self.s.types.syms.resolve(field.name),
							self.display(field.ty)
						)
					})
					.unwrap_or_default();
				problems.push((
					&crate::diagnostics::codes::advanced::STR001,
					format!(
						"a `{}` has no field `{missing}`: `{name}` needs `{}`",
						self.display(ty),
						self.display(constraint)
					),
				));
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

	/// `GN013`: a method the constraint requires (`{ describe: (self) =>
	/// String }`) that the actual type doesn't declare, or declares with
	/// another signature. The message, when one is missing.
	fn missing_method(&mut self, ty: TypeId, constraint: TypeId, name: &str) -> Option<String> {
		if self
			.s
			.types
			.flags(ty)
			.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN)
		{
			return None;
		}
		let required: Vec<crate::types::FieldEntry> = self
			.s
			.types
			.fields_of(constraint)
			.iter()
			.copied()
			.filter(|field| field.flags.contains(crate::types::FieldFlags::METHOD))
			.collect();
		for method in required {
			let text = self.s.types.syms.resolve(method.name).to_string();
			let Some(function) = self.type_function(ty, &text) else {
				return Some(format!(
					"a `{}` has no method `{text}`, which `{name}` requires (`{}`)",
					self.display(ty),
					self.display(constraint)
				));
			};
			let declared = self.symbol_type(function, crate::ids::NodeId::new(0));
			let bound = self.bound_method_type(function, declared);
			if !self.s.types.is_recovering(bound) && !self.s.types.satisfies(bound, method.ty) {
				return Some(format!(
					"the method `{text}` of `{}` is a `{}`, but `{name}` requires a `{}`",
					self.display(ty),
					self.display(bound),
					self.display(method.ty)
				));
			}
		}
		None
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
			if (!self.free_params(actual).is_empty() || !self.own_generics(actual).is_empty())
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
			let mut actual = self.check(value, Some(current));
			// a generic lambda given as a callback: instantiated by the callback type
			if !self.own_generics(actual).is_empty()
				&& let Some(instance) = self.instantiate_function_value(value, actual, current)
			{
				actual = instance;
			}
			synthesized.push(value);
			let mut candidates = Candidates::default();
			self.s
				.types
				.unify(declared, actual, &bindings, &mut candidates);
			self.resolve_candidates(&mut bindings, &candidates, &mut problems);
		}
		self.finish_bindings(&mut bindings, &mut problems);
		// the methods a constraint requires are those of the type written for
		// the argument (`Cat { … }`), not of another type with the same fields
		let written = arguments
			.iter()
			.find_map(|&(value, _)| self.written_item_decl(value));
		let saved = std::mem::replace(&mut self.receiver_decl, written);
		self.check_constraints(&mut bindings, &mut problems, true);
		self.receiver_decl = saved;
		(bindings, problems, synthesized)
	}

	/// The type declaration written for a value or for the items of an
	/// array literal (`[Cat { … }]`).
	fn written_item_decl(&self, value: NodeId) -> Option<NodeId> {
		match self.view(value) {
			View::Array { items } => items.first().and_then(|item| self.written_decl(item)),
			View::Group(inner) => self.written_item_decl(inner),
			_ => self.written_decl(value),
		}
	}

	/// Whether a type still has one of the `free` parameters.
	pub(super) fn has_free(&self, ty: TypeId, free: &[ParamId]) -> bool {
		let mut found = Vec::new();
		self.collect_params(ty, &mut found, 0);
		found.iter().any(|param| free.contains(param))
	}

	/// `sum<Float>`: the explicit generic arguments of a function. Legacy
	/// parenthesized arguments are diagnosed, then checked for recovery. The instantiated
	/// function type; the call node records the arguments.
	pub(super) fn generic_function_application(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		if let Some(ty) = self.generic_value_application(n, callee, arguments) {
			return Some(ty);
		}
		// `library.firstOf<Integer>`: a generic function of another file
		let tree = Arc::clone(&self.tree);
		if let View::Access { members, .. } = syntax::view(&tree, callee) {
			if !syntax::is_angle_application(&tree, n)
				&& !arguments
					.iter()
					.all(|argument| self.is_type_argument(argument))
			{
				return None;
			}
			let ty = self.check(callee, None);
			let crate::symbols::Resolution::Local(symbol) = self.a.resolution(members.last()?)
			else {
				return None;
			};
			if self.a.symbols.symbol(symbol).kind != SymbolKind::Function {
				return None;
			}
			let params = self.exported_generics(symbol);
			if params.is_empty() {
				return None;
			}
			return Some(self.apply_function_generics(n, ty, &params, arguments));
		}
		let View::Name(_) = self.view(callee) else {
			return None;
		};
		// a value parameter makes the first list the generic one (types brief §3.11.2)
		if arguments.is_empty()
			|| (!syntax::is_angle_application(&tree, n)
				&& !(self.has_value_params(callee)
					|| arguments
						.iter()
						.all(|argument| self.is_type_argument(argument))))
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
		Some(self.apply_function_generics(n, ty, &params, arguments))
	}

	/// The instance of a generic function of type `ty` for the generic
	/// arguments written at `n`; the node records them.
	fn apply_function_generics(
		&mut self,
		n: NodeId,
		ty: TypeId,
		params: &[ParamId],
		arguments: NodeList<'_>,
	) -> TypeId {
		self.check_generic_brackets(n);
		let args = self.generic_arguments(params, arguments);
		self.set_flag(n, crate::analysis::NodeFlags::GENERIC_APPLICATION);
		self.memo(n, |w| {
			if args.len() > params.len() {
				let message = format!("this function takes {} type arguments", params.len());
				let diagnostic = w.diagnostic(&GN007, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let mut bindings = Bindings::new(params);
			for (index, &arg) in args.iter().enumerate() {
				bindings.types[index] = arg;
			}
			let mut problems = Vec::new();
			w.finish_bindings(&mut bindings, &mut problems);
			w.check_constraints(&mut bindings, &mut problems, false);
			for (code, message) in problems {
				let diagnostic = w.diagnostic(code, n, message);
				w.report(diagnostic);
			}
			w.record_instantiation(n, &bindings);
			w.s.types.substitute(ty, &bindings)
		})
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
		// a generic function type given for a generic function type is related
		// as is (`generic_function_problem`)
		if !self.own_generics(target).is_empty() {
			return None;
		}
		let mut free = self.free_params(actual);
		free.extend(self.own_generics(actual));
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

	/// The generic parameters of a generic function type (its own, bound by
	/// it), empty for any other type.
	pub(super) fn own_generics(&self, ty: TypeId) -> Vec<ParamId> {
		let TypeKind::Function(sig) = self.s.types.kind(ty) else {
			return Vec::new();
		};
		self.s
			.types
			.list_items(self.s.types.sig(sig).generics)
			.iter()
			.filter_map(|&generic| match self.s.types.kind(generic) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect()
	}

	/// A generic `function` declaration given where a generic function type is
	/// expected (`describe(identity)` for `function identity = <T>(…) => …`):
	/// its type with its own parameters bound by it, a generic function value
	/// (the name's type, for the backends).
	pub(super) fn generic_declaration_value(
		&mut self,
		n: NodeId,
		actual: TypeId,
		target: TypeId,
	) -> Option<TypeId> {
		if self.own_generics(target).is_empty() || !self.own_generics(actual).is_empty() {
			return None;
		}
		// the name, or the last member of a path (`library.transforms.identity`)
		let named = match self.view(n) {
			View::Name(_) => n,
			View::Access { members, .. } => members.last()?,
			_ => return None,
		};
		let crate::symbols::Resolution::Local(symbol) = self.a.resolution(named) else {
			return None;
		};
		let generics = match self.direct_function(symbol) {
			Some(declaration) => self.a.function(declaration)?.generics.clone(),
			// a function of another file: the parameters its export publishes
			None if self.a.symbols.symbol(symbol).kind == SymbolKind::Function => {
				self.exported_generics(symbol)
			}
			None => return None,
		};
		if generics.is_empty() {
			return None;
		}
		let types: Vec<TypeId> = generics
			.iter()
			.map(|&param| self.s.types.intern(TypeKind::Param(param)))
			.collect();
		let ty = self.generic_function_type(actual, &types);
		self.set_type(n, ty);
		Some(ty)
	}

	/// `f<Integer>`: the explicit arguments of a value of a generic function
	/// type (a parameter, a `let`, a field, `function f: Identity = …`): the
	/// instance's type. `None` when the callee isn't one, or when the list may
	/// be a call (`f(x)` with values: the parenthesized spelling).
	fn generic_value_application(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let angle = syntax::is_angle_application(&self.tree, n);
		if arguments.is_empty()
			|| (!angle && !matches!(self.view(callee), View::Name(_)))
			|| self.is_type_name(callee)
		{
			return None;
		}
		if !angle
			&& !arguments
				.iter()
				.all(|argument| self.is_type_argument(argument))
		{
			return None;
		}
		if let View::Name(_) = self.view(callee)
			&& let crate::symbols::Resolution::Local(symbol) = self.a.resolution(callee)
			&& self.direct_function(symbol).is_some()
			&& self
				.own_generics(self.a.symbols.symbol(symbol).ty)
				.is_empty()
		{
			// a generic `function` declaration: the path below
			return None;
		}
		let ty = self.check(callee, None);
		let params = self.own_generics(ty);
		if params.is_empty() {
			return None;
		}
		self.check_generic_brackets(n);
		let args = self.generic_arguments(&params, arguments);
		self.set_flag(n, crate::analysis::NodeFlags::GENERIC_APPLICATION);
		Some(self.memo(n, |w| {
			if args.len() != params.len() {
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
			w.check_constraints(&mut bindings, &mut problems, false);
			for (code, message) in problems {
				let diagnostic = w.diagnostic(code, n, message);
				w.report(diagnostic);
			}
			w.record_instantiation(n, &bindings);
			w.s.types.substitute(ty, &bindings)
		}))
	}

	/// Why a function doesn't satisfy a generic function type, when that's the
	/// reason: it isn't generic (`E-GEN-2`), or it has other parameters or a
	/// narrower constraint (`E-GEN-1`, decision "Generic functions carry their
	/// list on the function value").
	pub(super) fn generic_function_problem(
		&mut self,
		n: NodeId,
		actual: TypeId,
		target: TypeId,
	) -> Option<crate::diagnostics::Diagnostic> {
		let theirs = self.own_generics(target);
		if theirs.is_empty() || !matches!(self.s.types.kind(actual), TypeKind::Function(_)) {
			return None;
		}
		let names: Vec<String> = theirs.iter().map(|&param| self.param_name(param)).collect();
		let ours = self.own_generics(actual);
		if ours.is_empty() {
			let message = format!(
				"a `{}` isn't generic: a `{}` must accept any type for `{}`, write a generic function, ex: `<{}>(…) => …`",
				self.display(actual),
				self.display(target),
				names.join("`, `"),
				names.join(", ")
			);
			return Some(self.diagnostic(&crate::diagnostics::codes::values::E_GEN_2, n, message));
		}
		if ours.len() != theirs.len() {
			let message = format!(
				"a `{}` has {} generic parameters, its type `{}` has {}",
				self.display(actual),
				ours.len(),
				self.display(target),
				theirs.len()
			);
			return Some(self.diagnostic(&crate::diagnostics::codes::values::E_GEN_1, n, message));
		}
		for (&our, &their) in ours.iter().zip(&theirs) {
			let wanted = self.s.types.param_constraint(our);
			if wanted.is_unset() || wanted == TypeId::UNKNOWN {
				continue;
			}
			let given = self.s.types.param_constraint(their);
			let open = given.is_unset() || given == TypeId::UNKNOWN;
			if open || !self.s.types.satisfies(given, wanted) {
				let accepted = if open {
					"any type".to_string()
				} else {
					format!("a `{}`", self.display(given))
				};
				let message = format!(
					"the constraint `{}` of `{}` is narrower than the type's: a `{}` accepts {accepted} for `{}`, so the function must too",
					self.display(wanted),
					self.param_name(our),
					self.display(target),
					self.param_name(their)
				);
				return Some(self.diagnostic(
					&crate::diagnostics::codes::values::E_GEN_1,
					n,
					message,
				));
			}
		}
		None
	}

	/// Records the type arguments of a call or a typed literal for the backends.
	/// The generic parameters of the function a call calls (a declaration
	/// of this file or another one), in declaration order; empty for a value.
	pub(super) fn callee_generics(
		&mut self,
		callee: NodeId,
		kind: crate::analysis::Callee,
	) -> Vec<ParamId> {
		if let crate::analysis::Callee::Function(declaration) = kind {
			return self
				.a
				.function(declaration)
				.map(|info| info.generics.clone())
				.unwrap_or_default();
		}
		let named = match self.view(callee) {
			View::Name(_) => callee,
			View::Access { members, .. } => match members.last() {
				Some(member) => member,
				None => return Vec::new(),
			},
			_ => return Vec::new(),
		};
		let crate::symbols::Resolution::Local(symbol) = self.a.resolution(named) else {
			return Vec::new();
		};
		if self.a.symbols.symbol(symbol).kind != SymbolKind::Function {
			return Vec::new();
		}
		match self.direct_function(symbol) {
			Some(declaration) => self
				.a
				.function(declaration)
				.map(|info| info.generics.clone())
				.unwrap_or_default(),
			None => self.exported_generics(symbol),
		}
	}

	/// Inferred bindings with the declared generic parameters last, in
	/// declaration order (inference finds them in order of appearance):
	/// what the backends read by position (`<A, B>(b: B, a: A)`).
	pub(super) fn declaration_order(&self, bindings: &Bindings, declared: &[ParamId]) -> Bindings {
		if declared.is_empty() || !declared.iter().all(|param| bindings.params.contains(param)) {
			return bindings.clone();
		}
		let mut params: Vec<ParamId> = bindings
			.params
			.iter()
			.copied()
			.filter(|param| !declared.contains(param))
			.collect();
		params.extend(declared.iter().copied());
		let types = params
			.iter()
			.map(|&param| bindings.get(param).unwrap_or(TypeId::UNKNOWN))
			.collect();
		Bindings { params, types }
	}

	pub(super) fn record_instantiation(&mut self, n: NodeId, bindings: &Bindings) -> ListId {
		let list = self.s.types.list(&bindings.types);
		self.a.instantiations.insert(n, list);
		self.a
			.instantiation_parameters
			.insert(n, bindings.params.clone());
		list
	}
}
