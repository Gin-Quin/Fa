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

use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
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
			return summary.requirements.clone();
		}
		fa_analyzer::modules::generic_requirements(self.types, &info.generics)
	}

	/// Declares the hidden parameters of a generic function being generated
	/// (inside its JavaScript scope): their names, in order. The value
	/// parameters' names are the names their body reads.
	pub(crate) fn declare_hidden(&mut self, function: NodeId) -> Vec<String> {
		let hidden = self.hidden_parameters(function);
		let mut names = Vec::with_capacity(hidden.len());
		for entry in hidden {
			match entry {
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
			_ => return Vec::new(),
		};
		let Some((module, decl)) = self.session.external_node(self.analysis.resolution(named))
		else {
			return Vec::new();
		};
		let Some(summary) = self
			.session
			.interface(module)
			.and_then(|interface| interface.generic_functions.get(&decl))
			.cloned()
		else {
			return Vec::new();
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
		self.hidden_arguments_for(call, &generics, summary.requirements)
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
		let binding = |param: ParamId| {
			generics
				.iter()
				.position(|&generic| generic == param)
				.and_then(|index| bindings.get(index).copied())
		};
		let mut values = Vec::with_capacity(hidden.len());
		for entry in hidden {
			let value = match entry {
				Hidden::Value(param) => match binding(param) {
					// a key set: its paths and stable instance tag (fields.rs)
					Some(ty) if self.analysis.key_set_params.contains_key(&param) => {
						let subject = binding(self.analysis.key_set_params[&param]);
						self.key_set_value(ty, subject)
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
		if let Some(decl) = self.analysis.type_decls.values().find(|decl| decl.ty == ty) {
			let function = decl
				.functions
				.iter()
				.copied()
				.find(|&function| self.analysis.symbols.symbol(function).name == method)?;
			return Some(self.name_of(function));
		}
		let nominal = match self.types.kind(ty) {
			TypeKind::Object(shape) => self.types.shape(shape).nominal?,
			TypeKind::NamedUnion(nominal, _)
			| TypeKind::Member(nominal, _)
			| TypeKind::Enum(nominal, _)
			| TypeKind::EnumMember(nominal, _) => nominal,
			_ => return None,
		};
		let key = self.types.nominal(nominal).key;
		let module = self.types.decl_key_data(key).module;
		let interface = self.session.interface(module)?;
		let declaration = interface.keys.get(&key)?.node;
		let function = interface
			.type_functions
			.get(&declaration)?
			.iter()
			.find(|function| function.name == method)?
			.decl;
		let export = interface.export_names.get(&function)?.clone();
		let path = crate::generate::modules::js_path(self.types.module_path(module));
		Some(self.import(&path, &export))
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
		if self.analysis.instantiation(n).is_none() {
			return None;
		}
		let (function, hidden, arity) = match self.analysis.resolution(self.strip_groups(callee)) {
			Resolution::Local(symbol) => {
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
		format!("${wanted}")
	}
}

/// The names of the hidden parameters in scope, by what they hold.
pub(crate) type HiddenNames = HashMap<Hidden, String>;
