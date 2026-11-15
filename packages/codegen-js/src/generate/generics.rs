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
pub(crate) use fa_analyzer::modules::GenericRequirement as Hidden;
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{FieldFlags, TypeKind};

use crate::Target;
use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// The original binders survive an expression's implicit instantiation.
	fn value_generics(&self, node: NodeId) -> Vec<ParamId> {
		let node = self.strip_groups(node);
		if let Some(info) = self.analysis.function(node) {
			return info.generics.clone();
		}
		let named = match self.view(node) {
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				members.last().unwrap_or(node)
			}
			_ => node,
		};
		let original = match self.analysis.resolution(named) {
			Resolution::Local(symbol) => {
				let symbol = self.analysis.symbols.symbol(symbol);
				if let Some(info) = self.analysis.function(symbol.decl_node)
					&& !info.generics.is_empty()
				{
					return info.generics.clone();
				}
				symbol.ty
			}
			Resolution::Field(shape, index) => self.types.shape_fields(shape)[index as usize].ty,
			_ => self.type_of(node),
		};
		let original = match self.view(node) {
			View::Index { target, .. } => match self.types.kind(self.type_of(target)) {
				TypeKind::Collection(_, item, _) => item,
				_ => original,
			},
			_ => original,
		};
		let TypeKind::Function(sig) = self.types.kind(original) else {
			return Vec::new();
		};
		self.types
			.list_items(self.types.sig(sig).generics)
			.iter()
			.filter_map(|&ty| match self.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect()
	}

	/// A generic value used at a concrete function type (a callback, field,
	/// return or alias) binds its capabilities at the same conversion point.
	pub(crate) fn bind_generic_value(&mut self, node: NodeId, value: Expr) -> Expr {
		if self.analysis.instantiation(node).is_none()
			|| !matches!(self.types.kind(self.type_of(node)), TypeKind::Function(_))
		{
			return value;
		}
		match self.view(node) {
			View::Call { .. } => return value,
			View::Group(inner) if self.analysis.instantiation(inner).is_some() => return value,
			_ => {}
		}
		let hidden = self.external_hidden_arguments(node, node);
		if hidden.is_empty() {
			return value;
		}
		Expr::primary(format!(
			"{}.bind(undefined, {})",
			value.primary_text(),
			hidden.join(", ")
		))
	}

	/// The hidden parameters of a generic function declaration, in order.
	pub(crate) fn hidden_parameters(&self, function: NodeId) -> Vec<Hidden> {
		let Some(info) = self.analysis.function(function) else {
			return Vec::new();
		};
		let decl = info.declaration.unwrap_or(function);
		if let Some(summary) = self
			.session
			.source_file(self.file)
			.interface
			.generic_functions
			.get(&decl)
		{
			return self.runtime_requirements(summary.requirements.clone());
		}
		self.runtime_requirements(fa_analyzer::modules::generic_requirements(
			self.types,
			&info.generics,
		))
	}

	fn runtime_requirements(&self, mut requirements: Vec<Hidden>) -> Vec<Hidden> {
		if self.options.target != Target::Bun {
			requirements.retain(|entry| !matches!(entry, Hidden::Layout(_)));
		}
		requirements
	}

	/// Declares the hidden parameters of a generic function being generated
	/// (inside its JavaScript scope): their names, in order. The value
	/// parameters' names are the names their body reads.
	pub(crate) fn declare_hidden(&mut self, function: NodeId) -> Vec<String> {
		let hidden = self.hidden_parameters(function);
		let mut names = Vec::with_capacity(hidden.len());
		for entry in hidden {
			match entry {
				Hidden::Layout(param) => {
					let name = self.declare_hidden_name(&format!(
						"{}$layout",
						self.types.syms.resolve(self.types.param(param).name)
					));
					self.hidden_names.insert(entry, name.clone());
					names.push(name);
				}
				Hidden::Value(param) => {
					let info = self.types.param(param);
					let name = match self.value_parameter_symbol(function, info.name) {
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
	fn value_parameter_symbol(&self, function: NodeId, name: Sym) -> Option<SymbolId> {
		// the declaration holds the generic parameters, not the lambda
		let declaration = self
			.analysis
			.function(function)
			.and_then(|info| info.declaration)
			.unwrap_or(function);
		let span = self.tree.spans[declaration.index()];
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
		let generics = self
			.analysis
			.function(function)
			.map(|info| info.generics.clone())
			.unwrap_or_default();
		self.hidden_arguments_for(call, &generics, hidden)
	}

	/// The same ABI for an imported function: only its interface is needed.
	pub(crate) fn external_hidden_arguments(
		&mut self,
		callee: NodeId,
		call: NodeId,
	) -> Vec<String> {
		let callee = self.strip_groups(callee);
		let named = match self.view(callee) {
			View::Name(_) => callee,
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				match members.last() {
					Some(member) => member,
					None => return Vec::new(),
				}
			}
			View::Call { callee: inner, .. }
				if self.analysis.has_flags(
					callee,
					fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION,
				) =>
			{
				return self.external_hidden_arguments(inner, call);
			}
			_ => callee,
		};
		let summary = self
			.session
			.external_node(self.analysis.resolution(named))
			.and_then(|(module, decl)| self.session.interface(module)?.generic_functions.get(&decl))
			.cloned();
		let Some(summary) = summary else {
			let generics = self.value_generics(callee);
			let requirements = self.runtime_requirements(
				fa_analyzer::modules::generic_requirements(self.types, &generics),
			);
			return self.hidden_arguments_for(call, &generics, requirements);
		};
		let generics: Vec<_> = self
			.types
			.list_items(summary.generics)
			.iter()
			.filter_map(|&ty| match self.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect();
		let requirements = self.runtime_requirements(summary.requirements);
		self.hidden_arguments_for(call, &generics, requirements)
	}

	fn hidden_arguments_for(
		&mut self,
		call: NodeId,
		generics: &[ParamId],
		hidden: Vec<Hidden>,
	) -> Vec<String> {
		let bindings: Vec<TypeId> = self
			.analysis
			.instantiation(call)
			.map(|list| self.types.list_items(list).to_vec())
			.unwrap_or_default();
		let parameters = self
			.analysis
			.instantiation_parameters
			.get(&call)
			.map_or(generics, |params| params.as_slice());
		let types = self.types;
		let binding = |param: ParamId| {
			parameters
				.iter()
				.position(|&generic| {
					generic == param || {
						let a = types.param(generic);
						let b = types.param(param);
						a.owner == b.owner && a.index == b.index
					}
				})
				// A declaration annotated by an independently bound generic
				// function type has alpha-renamed parameters in that signature.
				.or_else(|| {
					(parameters.len() == generics.len())
						.then(|| generics.iter().position(|&generic| generic == param))
						.flatten()
				})
				.and_then(|index| bindings.get(index).copied())
		};
		let mut values = Vec::with_capacity(hidden.len());
		for entry in hidden {
			let value = match entry {
				Hidden::Layout(param) => binding(param).and_then(|ty| self.layout_descriptor(ty)),
				Hidden::Value(param) => match binding(param) {
					// a key set: the index of its application (fields.rs)
					Some(ty) if self.analysis.key_set_params.contains_key(&param) => {
						let subject = binding(self.analysis.key_set_params[&param]);
						self.key_set_index(param, ty, subject)
					}
					Some(ty) => self.hidden_value(ty),
					None => None,
				},
				Hidden::Method(param, method) => match binding(param) {
					Some(ty) => self.method_of(ty, method),
					None => None,
				},
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
		let decl = self
			.analysis
			.type_decls
			.values()
			.find(|decl| decl.ty == ty)?
			.clone();
		let function = decl
			.functions
			.iter()
			.copied()
			.find(|&function| self.analysis.symbols.symbol(function).name == method)?;
		Some(self.name_of(function))
	}

	/// `item.describe(…)` in a generic body, where the method is required by
	/// the constraint of `item`'s type parameter: a call of the hidden
	/// parameter, the receiver first. `None` for any other call.
	pub(crate) fn constraint_method_call(&mut self, callee: NodeId) -> Option<(String, NodeId)> {
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let [member] = members.iter().collect::<Vec<_>>()[..] else {
			return None;
		};
		let Resolution::Field(shape, index) = self.analysis.resolution(member) else {
			return None;
		};
		let field = self.types.shape_fields(shape)[index as usize];
		if !field.flags.contains(FieldFlags::METHOD) {
			return None;
		}
		let TypeKind::Param(param) = self.types.kind(self.type_of(target)) else {
			return None;
		};
		let name = self
			.hidden_names
			.get(&Hidden::Method(param, field.name))?
			.clone();
		Some((name, target))
	}

	/// A generic application used as a value (`repeatText(2)`): the
	/// function, with its hidden arguments bound when it has some.
	pub(crate) fn generic_application(&mut self, n: NodeId, callee: NodeId) -> Option<Expr> {
		self.analysis.instantiation(n)?;
		let (function, hidden, arity) = match self.analysis.resolution(self.strip_groups(callee)) {
			Resolution::Local(symbol)
				if self
					.analysis
					.function(self.analysis.symbols.symbol(symbol).decl_node)
					.is_some() =>
			{
				let declaration = self.analysis.symbols.symbol(symbol).decl_node;
				let info = self.analysis.function(declaration)?.clone();
				if info.generics.is_empty() {
					return None;
				}
				(
					self.name_of(symbol),
					self.hidden_arguments(declaration, n),
					info.params.len(),
				)
			}
			_ => {
				let TypeKind::Function(sig) = self.types.kind(self.type_of(n)) else {
					return None;
				};
				let arity = self.types.params(self.types.sig(sig).params).len();
				let hidden = self.external_hidden_arguments(callee, n);
				let function = self.expr(callee).primary_text();
				(function, hidden, arity)
			}
		};
		if hidden.is_empty() {
			return Some(Expr::name(function, true));
		}
		let parameters: Vec<String> = (0..arity).map(|i| format!("$p{i}")).collect();
		let mut arguments = hidden;
		arguments.extend(parameters.iter().cloned());
		let call = js::call(&function, &arguments);
		Some(Expr::new(
			format!("({}) => {}", parameters.join(", "), call.text),
			Prec::Assign,
		))
	}

	/// The name of a hidden parameter (`$Item$describe`: Fa names never
	/// contain `$`).
	fn declare_hidden_name(&mut self, wanted: &str) -> String {
		self.scopes.declare_global(&format!("${wanted}"))
	}
}

/// The names of the hidden parameters in scope, by what they hold.
pub(crate) type HiddenNames = HashMap<Hidden, String>;
