//! Generic functions (types brief §3.11): erased, one JavaScript function
//! for every instance, with *hidden leading parameters* for what an
//! instance changes at runtime:
//!
//! - a compile-time value parameter (`repeatText(times: Integer(min = 0))`)
//!   is a parameter holding its value: `repeatText(2)("ab")` calls
//!   `repeatText(2, "ab")`;
//! - a method required by a constraint (`Item: { describe: (self) => String
//!   }`) is a parameter holding the function of the actual type
//!   (`Dog$describe`): `item.describe()` calls `$Item$describe(item)`.
//!
//! A call of a generic function passes them from its instantiation; inside
//! another generic function, a binding to one of its own parameters
//! forwards that function's hidden parameter.

use std::collections::HashMap;

use fa_analyzer::ids::{NodeId, ParamId, Sym, SymbolId, TypeId};
use fa_analyzer::modules::GenericRequirement;
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{FieldFlags, TypeKind};

use crate::generate::Generator;
use crate::js::{self, Expr};

/// Backend capabilities, including the representation of an erased place.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Hidden {
	Value(ParamId),
	Method(ParamId, Sym),
	ReferenceObject(ParamId),
}
impl From<GenericRequirement> for Hidden {
	fn from(value: GenericRequirement) -> Self {
		match value {
			GenericRequirement::Value(p) => Self::Value(p),
			GenericRequirement::Method(p, m) => Self::Method(p, m),
		}
	}
}

impl Generator<'_> {
	/// The hidden parameters of a generic function declaration, in order.
	pub(crate) fn hidden_parameters(&self, function: NodeId) -> Vec<Hidden> {
		let mut hidden = self.hidden_for_generics(&self.owner_parameters(function));
		let Some(info) = self.analysis.function(function) else {
			return hidden;
		};
		let declaration = info.declaration.unwrap_or(function);
		match self
			.session
			.source_file(self.file)
			.interface
			.generic_functions
			.get(&declaration)
		{
			Some(summary) => {
				hidden.extend(summary.requirements.iter().copied().map(Hidden::from));
				hidden.extend(self.reference_flags(&info.generics));
			}
			None => hidden.extend(self.hidden_for_generics(&info.generics)),
		}
		hidden
	}

	fn function_generics(&self, function: NodeId) -> Vec<ParamId> {
		let mut params = self.owner_parameters(function);
		if let Some(info) = self.analysis.function(function) {
			params.extend(info.generics.iter().copied());
		}
		params
	}

	fn owner_parameters(&self, function: NodeId) -> Vec<ParamId> {
		self.analysis
			.function_owners
			.get(&function)
			.and_then(|owner| self.analysis.type_decls.get(owner))
			.map(|info| info.generics.clone())
			.unwrap_or_default()
	}

	fn hidden_for_generics(&self, generics: &[ParamId]) -> Vec<Hidden> {
		let mut hidden: Vec<_> = fa_analyzer::modules::generic_requirements(self.types, generics)
			.into_iter()
			.map(Hidden::from)
			.collect();
		hidden.extend(self.reference_flags(generics));
		hidden
	}
	fn reference_flags(&self, generics: &[ParamId]) -> Vec<Hidden> {
		generics
			.iter()
			.copied()
			.filter(|&param| !self.types.param(param).value)
			.map(Hidden::ReferenceObject)
			.collect()
	}
	/// A union slot retains its carrier even while its current value is an object.
	pub(crate) fn reference_object_flag(&self, ty: TypeId) -> String {
		if let TypeKind::Param(param) = self.types.kind(self.without_none(ty)) {
			return self
				.hidden_names
				.get(&Hidden::ReferenceObject(param))
				.cloned()
				.unwrap_or_else(|| "false".to_string());
		}
		self.is_object_reference(ty).to_string()
	}

	/// Declares the hidden parameters of a generic function being generated
	/// (inside its JavaScript scope): their names, in order. The value
	/// parameters' names are the names their body reads.
	pub(crate) fn declare_hidden(&mut self, function: NodeId) -> Vec<String> {
		let hidden = self.hidden_parameters(function);
		let mut names = Vec::with_capacity(hidden.len());
		for entry in hidden {
			match entry {
				Hidden::ReferenceObject(param) => {
					let wanted = format!(
						"{}$referenceObject",
						self.types.syms.resolve(self.types.param(param).name)
					);
					let name = self.declare_hidden_name(&wanted);
					self.hidden_names.insert(entry, name.clone());
					names.push(name);
				}
				Hidden::Value(param) => {
					let name = match self.value_parameter_symbol(function, param) {
						Some(symbol) => self.declare(symbol),
						None => self.temp(),
					};
					self.hidden_names.insert(entry, name.clone());
					names.push(name);
				}
				Hidden::Method(param, method) => {
					let info = self.types.param(param);
					let wanted = format!(
						"{}${}",
						self.types.syms.resolve(info.name),
						self.types.syms.resolve(method)
					);
					let name = self.declare_hidden_name(&wanted);
					self.hidden_names
						.insert(Hidden::Method(param, method), name.clone());
					names.push(name);
				}
			}
		}
		names
	}

	/// The `Static` symbol a value parameter's name resolves to in the body
	/// of a function.
	fn value_parameter_symbol(&self, function: NodeId, param: ParamId) -> Option<SymbolId> {
		let name = self.types.param(param).name;
		// the declaration holds the generic parameters, not the lambda
		let declaration = self
			.analysis
			.function(function)
			.and_then(|info| info.declaration)
			.unwrap_or(function);
		let owner = self
			.analysis
			.function_owners
			.get(&function)
			.copied()
			.filter(|owner| {
				self.analysis
					.type_decls
					.get(owner)
					.is_some_and(|info| info.generics.contains(&param))
			})
			.unwrap_or(declaration);
		let span = self.tree.spans[owner.index()];
		self.analysis
			.symbols
			.symbols()
			.iter()
			.position(|symbol| {
				symbol.kind == SymbolKind::Static && symbol.name == name && {
					let declared = self.tree.spans[symbol.decl_node.index()];
					span.start <= declared.start && declared.end <= span.end
				}
			})
			.map(SymbolId::new)
	}

	/// The values of the hidden parameters of a generic function at a call
	/// (or a generic application) node.
	pub(crate) fn hidden_arguments(&mut self, function: NodeId, call: NodeId) -> Vec<String> {
		let hidden = self.hidden_parameters(function);
		if hidden.is_empty() {
			return Vec::new();
		}
		let generics = self.function_generics(function);
		self.hidden_arguments_for(call, &generics, hidden)
	}

	fn hidden_arguments_for(
		&mut self,
		call: NodeId,
		generics: &[ParamId],
		hidden: Vec<Hidden>,
	) -> Vec<String> {
		let mut bindings: Vec<TypeId> = self
			.analysis
			.instantiation(call)
			.map(|list| self.types.list_items(list).to_vec())
			.unwrap_or_default();
		let mut methods = self
			.analysis
			.instantiated_methods
			.get(&call)
			.cloned()
			.unwrap_or_default();
		if let Some(owner) = self.call_method_owner(call) {
			let prefix = self.types.list_items(owner.arguments).to_vec();
			for method in &mut methods {
				method.parameter += prefix.len() as u16;
			}
			methods.extend(owner.methods.iter().cloned());
			bindings.splice(0..0, prefix);
		}
		self.hidden_arguments_bound(call, generics, hidden, &bindings, &methods)
	}

	fn hidden_arguments_bound(
		&mut self,
		call: NodeId,
		generics: &[ParamId],
		hidden: Vec<Hidden>,
		bindings: &[TypeId],
		methods: &[fa_analyzer::analysis::GenericMethod],
	) -> Vec<String> {
		let types = self.types;
		let same_parameter = |left: ParamId, right: ParamId| {
			if left == right {
				return true;
			}
			let (left, right) = (types.param(left), types.param(right));
			left.owner == right.owner && left.index == right.index
		};
		let binding = |param: ParamId| {
			generics
				.iter()
				.position(|&generic| same_parameter(generic, param))
				.and_then(|index| bindings.get(index).copied())
		};
		let mut values = Vec::with_capacity(hidden.len());
		for entry in hidden {
			let value =
				match entry {
					Hidden::ReferenceObject(param) => {
						binding(param).map(|ty| self.reference_object_flag(ty))
					}
					Hidden::Value(param) => match binding(param) {
						Some(ty) => {
							let subject = self.analysis.key_set_params.iter().find_map(
								|(&keys, &subject)| same_parameter(keys, param).then_some(subject),
							);
							match subject {
								Some(subject) => self.key_set_value(ty, binding(subject)),
								None => self.hidden_value(ty),
							}
						}
						None => None,
					},
					Hidden::Method(param, method) => {
						let parameter = generics
							.iter()
							.position(|&generic| same_parameter(generic, param));
						let selected = methods
							.iter()
							.find(|selected| {
								Some(selected.parameter as usize) == parameter
									&& self.types.syms.resolve(selected.name)
										== self.types.syms.resolve(method)
							})
							.cloned();
						match selected {
							Some(selected) => self.selected_method_name(&selected, call),
							None => binding(param).and_then(|ty| self.method_of(ty, method)),
						}
					}
				};
			match value {
				Some(value) => values.push(value),
				None => {
					self.unsupported(call, "this instance of a generic function");
					values.push("undefined".to_string());
				}
			}
		}
		values
	}

	fn call_method_owner(&self, call: NodeId) -> Option<&fa_analyzer::analysis::WrittenType> {
		let mut callee = match self.view(call) {
			View::Call { callee, .. } => callee,
			View::Access { .. } | View::OptionalAccess { .. } => call,
			_ => return None,
		};
		loop {
			callee = self.strip_groups(callee);
			match self.view(callee) {
				View::Call { callee: inner, .. }
					if self.analysis.has_flags(
						callee,
						fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION,
					) =>
				{
					callee = inner
				}
				View::Access { members, .. } | View::OptionalAccess { members, .. } => {
					return self.analysis.method_instantiations.get(&members.last()?);
				}
				_ => return None,
			}
		}
	}

	/// Capabilities of a generic function value come from its original
	/// signature; no declaring function node is needed for a parameter/field.
	pub(crate) fn signature_hidden_arguments(
		&mut self,
		callee: NodeId,
		call: NodeId,
	) -> Vec<String> {
		let callee = self.strip_groups(callee);
		if let View::Call { callee: inner, .. } = self.view(callee)
			&& self.analysis.has_flags(
				callee,
				fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION,
			) {
			return self.signature_hidden_arguments(inner, call);
		}
		let named = match self.view(callee) {
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				members.last().unwrap_or(callee)
			}
			_ => callee,
		};
		match self.analysis.resolution(named) {
			Resolution::Local(symbol) => {
				let declaration = self.analysis.symbols.symbol(symbol).decl_node;
				if self.analysis.function(declaration).is_some() {
					return self.hidden_arguments(declaration, call);
				}
			}
			Resolution::External(key) => {
				if let Some((module, keyed)) = self.session.keyed(key)
					&& let Some(summary) = self
						.session
						.interface(module)
						.and_then(|interface| interface.generic_functions.get(&keyed.node))
						.cloned()
				{
					let mut generics = self.external_owner_parameters(key);
					let mut hidden = self.hidden_for_generics(&generics);
					generics.extend(self.types.list_items(summary.generics).iter().filter_map(
						|&ty| match self.types.kind(ty) {
							TypeKind::Param(param) => Some(param),
							_ => None,
						},
					));
					hidden.extend(summary.requirements.into_iter().map(Hidden::from));
					let own: Vec<_> = self
						.types
						.list_items(summary.generics)
						.iter()
						.filter_map(|&ty| match self.types.kind(ty) {
							TypeKind::Param(p) => Some(p),
							_ => None,
						})
						.collect();
					hidden.extend(self.reference_flags(&own));
					return self.hidden_arguments_for(call, &generics, hidden);
				}
			}
			_ => {}
		}
		let ty = match self.analysis.resolution(named) {
			Resolution::Local(symbol) => self.analysis.symbols.symbol(symbol).ty,
			Resolution::Field(shape, index) => self.types.shape_fields(shape)[index as usize].ty,
			_ => self.type_of(callee),
		};
		let ty = match self.view(callee) {
			View::Index { target, .. } => match self.types.kind(self.type_of(target)) {
				TypeKind::Collection(_, item, _) => item,
				_ => ty,
			},
			_ => ty,
		};
		let TypeKind::Function(sig) = self.types.kind(ty) else {
			return Vec::new();
		};
		let mut generics: Vec<_> = self
			.types
			.list_items(self.types.sig(sig).generics)
			.iter()
			.filter_map(|&ty| match self.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect();
		let owner_params = match self.analysis.resolution(named) {
			Resolution::External(key) => self.external_owner_parameters(key),
			Resolution::Local(symbol) => {
				self.owner_parameters(self.analysis.symbols.symbol(symbol).decl_node)
			}
			_ => Vec::new(),
		};
		generics.splice(0..0, owner_params);
		let hidden = self.hidden_for_generics(&generics);
		self.hidden_arguments_for(call, &generics, hidden)
	}

	/// The value a value parameter is bound to: a literal type, or the
	/// enclosing function's own value parameter.
	fn hidden_value(&mut self, ty: TypeId) -> Option<String> {
		if let TypeKind::Param(param) = self.types.kind(ty) {
			return self.hidden_names.get(&Hidden::Value(param)).cloned();
		}
		self.literal_of(ty).map(|literal| literal.text)
	}

	/// The JavaScript function implementing a method for a type: the
	/// function of its declaration (`Dog$describe`), or the enclosing
	/// function's hidden parameter for one of its own type parameters.
	fn method_of(&mut self, ty: TypeId, method: Sym) -> Option<String> {
		if let TypeKind::Param(param) = self.types.kind(ty) {
			return self
				.hidden_names
				.get(&Hidden::Method(param, method))
				.cloned();
		}
		None
	}

	fn selected_method_name(
		&mut self,
		method: &fa_analyzer::analysis::GenericMethod,
		call: NodeId,
	) -> Option<String> {
		let key = method.function;
		let (module, keyed) = self.session.keyed(key)?;
		let (name, asynchronous, generics) = if module == self.session.module_id(self.file) {
			let symbol = self.analysis.symbols.symbol_of_decl(keyed.node)?;
			let instances = self
				.tasks
				.instances
				.get(&keyed.node)
				.cloned()
				.unwrap_or_default();
			(
				instances.sync_name.unwrap_or_else(|| self.name_of(symbol)),
				instances.async_name,
				self.owner_parameters(keyed.node),
			)
		} else {
			let suspends = self.tasks.asynchronous
				&& self.program.is_some()
				&& self
					.session
					.interface(module)?
					.async_summaries
					.get(&keyed.node)
					.is_some_and(|summary| summary.suspends);
			let generics = self.external_owner_parameters(key);
			let external = self.external(Resolution::External(key))?;
			let (name, asynchronous) = if suspends {
				(
					self.import(&external.path, &format!("{}$sync", external.export)),
					Some(self.import(&external.path, &format!("{}$async", external.export))),
				)
			} else {
				(self.import(&external.path, &external.export), None)
			};
			(name, asynchronous, generics)
		};
		let hidden = self.hidden_for_generics(&generics);
		let bindings = self.types.list_items(method.arguments).to_vec();
		let values =
			self.hidden_arguments_bound(call, &generics, hidden, &bindings, &method.methods);
		let bind = |name: String, asynchronous| {
			if values.is_empty() {
				name
			} else if asynchronous {
				format!(
					"(($task, ...$a) => {name}($task, {}, ...$a))",
					values.join(", ")
				)
			} else {
				format!("((...$a) => {name}({}, ...$a))", values.join(", "))
			}
		};
		let name = bind(name, false);
		Some(match asynchronous {
			Some(asynchronous) => {
				let asynchronous = bind(asynchronous, true);
				let helper = self.helper("faAsyncValue");
				format!("{helper}({name}, {asynchronous})")
			}
			None => name,
		})
	}

	fn external_owner_parameters(&self, key: fa_analyzer::ids::DeclKey) -> Vec<ParamId> {
		let Some((module, keyed)) = self.session.keyed(key) else {
			return Vec::new();
		};
		let params = self
			.session
			.interface(module)
			.and_then(|interface| interface.function_owners.get(&keyed.node))
			.and_then(|&owner| {
				let owner_module = self.types.decl_key_data(owner).module;
				let owner_interface = self.session.interface(owner_module)?;
				let owner_node = owner_interface.node_of(owner)?;
				owner_interface.type_parameters.get(&owner_node).copied()
			});
		params
			.map(|list| {
				self.types
					.list_items(list)
					.iter()
					.filter_map(|&ty| match self.types.kind(ty) {
						TypeKind::Param(param) => Some(param),
						_ => None,
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// `item.describe(…)` in a generic body, where the method is required by
	/// the constraint of `item`'s type parameter: a call of the hidden
	/// parameter, the receiver first. `None` for any other call.
	pub(crate) fn constraint_method_call(&mut self, callee: NodeId) -> Option<String> {
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let member = members.last()?;
		let Resolution::Field(shape, index) = self.analysis.resolution(member) else {
			return None;
		};
		let field = self.types.shape_fields(shape)[index as usize];
		if !field.flags.contains(FieldFlags::METHOD) {
			return None;
		}
		let receiver = members
			.len()
			.checked_sub(2)
			.and_then(|index| members.get(index))
			.unwrap_or(target);
		let TypeKind::Param(param) = self.types.kind(self.type_of(receiver)) else {
			return None;
		};
		let name = self
			.hidden_names
			.get(&Hidden::Method(param, field.name))?
			.clone();
		Some(name)
	}

	/// Implicit generic-to-concrete function conversions bind capabilities
	/// once at conversion, just like an explicitly written application.
	pub(crate) fn instantiated_function_value(&mut self, node: NodeId, value: Expr) -> Expr {
		if !matches!(
			self.view(node),
			View::Name(_) | View::Access { .. } | View::Lambda { .. }
		) || self.is_callee(node)
		{
			return value;
		}
		let hidden = if self.analysis.instantiation(node).is_some() {
			self.signature_hidden_arguments(node, node)
		} else if let Some(owner) = self.call_method_owner(node).cloned() {
			// A written generic owner is already fixed even when the method
			// keeps its own generic list on the resulting function value.
			let named = match self.view(node) {
				View::Access { members, .. } => members.last().unwrap_or(node),
				_ => node,
			};
			let generics = match self.analysis.resolution(named) {
				Resolution::Local(symbol) => {
					self.owner_parameters(self.analysis.symbols.symbol(symbol).decl_node)
				}
				Resolution::External(key) => self.external_owner_parameters(key),
				_ => Vec::new(),
			};
			let hidden = self.hidden_for_generics(&generics);
			let bindings = self.types.list_items(owner.arguments).to_vec();
			self.hidden_arguments_bound(node, &generics, hidden, &bindings, &owner.methods)
		} else {
			Vec::new()
		};
		self.bind_function_hidden(value, hidden)
	}

	fn bind_function_hidden(&mut self, value: Expr, hidden: Vec<String>) -> Expr {
		if hidden.is_empty() {
			return value;
		}
		let helper = self.helper("faBindGenerics");
		let mut arguments = vec![value.text];
		arguments.extend(hidden);
		js::call(helper, &arguments)
	}

	/// A generic application used as a value (`repeatText(2)`): the
	/// function, with its hidden arguments bound when it has some.
	pub(crate) fn generic_application(&mut self, n: NodeId, callee: NodeId) -> Option<Expr> {
		self.analysis.instantiation(n)?;
		let local = match self.analysis.resolution(self.strip_groups(callee)) {
			Resolution::Local(symbol) => self
				.analysis
				.function(self.analysis.symbols.symbol(symbol).decl_node)
				.map(|info| (symbol, info.clone())),
			_ => None,
		};
		let (function, hidden) = match local {
			Some((symbol, info)) => {
				if info.generics.is_empty() {
					return None;
				}
				(
					self.name_of(symbol),
					self.hidden_arguments(info.declaration.unwrap_or(info.lambda), n),
				)
			}
			None => {
				let TypeKind::Function(_) = self.types.kind(self.type_of(n)) else {
					return None;
				};
				let hidden = self.signature_hidden_arguments(callee, n);
				(self.expr(callee).primary_text(), hidden)
			}
		};
		Some(self.bind_function_hidden(Expr::name(function, true), hidden))
	}

	/// The name of a hidden parameter (`$Item$describe`: Fa names never
	/// contain `$`).
	fn declare_hidden_name(&mut self, wanted: &str) -> String {
		let base = crate::names::identifier(&format!("${wanted}"));
		let mut name = base.clone();
		let mut suffix = 0;
		while self.hidden_names.values().any(|known| *known == name) {
			suffix += 1;
			name = format!("{base}${suffix}");
		}
		name
	}
}

/// The names of the hidden parameters in scope, by what they hold.
pub(crate) type HiddenNames = HashMap<Hidden, String>;
