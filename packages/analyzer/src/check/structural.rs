//! Structural typing (advanced brief §10): the diagnostics of a value that
//! doesn't satisfy an object type (`STR001`, `STR003`, `STR009`), and the
//! narrowing copies the backends must make (§10.4): `let human: Human =
//! superHuman` copies `Human`'s fields only, while a parameter is a view.
//! Two declarations with the same fields are one type; the methods of a
//! value are those of the type **written** for it (§10.1): its annotation,
//! or the type of the literal it was built with.

use std::sync::Arc;

use crate::analysis::{Callee, NodeFlags, WrittenReturn, WrittenType};
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{STR001, STR003};
use crate::diagnostics::codes::types::TF020;
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{DeclKey, NodeId, ParamId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{FieldEntry, NominalKind, ParamInfo, TypeKind};

impl Walker<'_> {
	/// Records the `type` declaration written for a binding or a parameter:
	/// the annotation `Human`, the literal `Animal { … }`, or the written
	/// type of the binding it copies. Type part.
	pub(super) fn record_written_type(&mut self, symbol: SymbolId, written: Option<NodeId>) {
		if let Some(decl) = written.and_then(|node| self.written_decl(node)) {
			self.written_decls.insert(symbol, decl);
		}
		if let Some(origin) = written.and_then(|node| self.written_origin(node)) {
			self.a
				.written_types
				.insert(self.a.symbols.symbol(symbol).decl_node, origin.clone());
			self.written_type_keys.insert(symbol, origin);
		}
	}

	// ── `Type(value)` (types brief §3.11.7, advanced brief §10.4) ─────────

	/// The parameters of a function whose type is named by `Type(parameter)`
	/// in its signature: each becomes a hidden generic parameter, bound to
	/// the type of the argument at each call (`renamed(hero, …)` returns a
	/// `SuperHuman`), with the written type as constraint. Called before the
	/// signature is checked; the parameters are rigid in the body.
	pub(super) fn implicit_generics(&mut self, function: NodeId, value: NodeId) -> Vec<ParamId> {
		let tree = Arc::clone(&self.tree);
		let View::Lambda { signature, .. } = syntax::view(&tree, value) else {
			return Vec::new();
		};
		let mut names: Vec<(Sym, NodeId)> = Vec::new();
		for parameter in signature.parameters.iter() {
			if let View::Assignment { target, .. } = syntax::view(&tree, parameter)
				&& let View::Name(name) = syntax::view(&tree, target)
			{
				names.push((self.s.types.syms.intern(tree.name(name)), parameter));
			}
		}
		let mut used: Vec<Sym> = Vec::new();
		let mut roots: Vec<NodeId> = signature.returns.into_iter().collect();
		for parameter in signature.parameters.iter() {
			if let View::Assignment {
				annotation: Some(annotation),
				..
			} = syntax::view(&tree, parameter)
			{
				roots.push(annotation);
			}
		}
		while let Some(node) = roots.pop() {
			if let View::Call { callee, arguments } = syntax::view(&tree, node)
				&& matches!(syntax::view(&tree, callee), View::Name(name) if tree.name(name) == "Type")
				&& let Some(argument) = arguments.first()
				&& let View::Name(name) = syntax::view(&tree, argument)
			{
				let sym = self.s.types.syms.intern(tree.name(name));
				if names.iter().any(|&(known, _)| known == sym) && !used.contains(&sym) {
					used.push(sym);
				}
				continue;
			}
			roots.extend(syntax::children(&tree, node));
		}
		if used.is_empty() {
			return Vec::new();
		}
		let owner_name = format!("{}#implicit", function.index());
		let key = self.nominal_key(&owner_name);
		let mut params = Vec::with_capacity(used.len());
		for (index, sym) in used.into_iter().enumerate() {
			let text = format!("Type<{}>", self.s.types.syms.resolve(sym));
			let name = self.s.types.syms.intern(&text);
			let param = self.s.types.add_param_pending(ParamInfo {
				owner: key,
				index: index as u16,
				name,
				constraint: TypeId::UNKNOWN,
				default: TypeId::UNSET,
				value: false,
			});
			self.pending_implicit.push((sym, param));
			params.push(param);
		}
		params
	}

	/// Notes a write into a field of a value whose type is an implicit
	/// generic parameter (`copy.name = name` with `copy: Type<human>`): the
	/// path and the written type.
	pub(super) fn note_implicit_write(&mut self, target: NodeId, written: TypeId) {
		let tree = Arc::clone(&self.tree);
		let View::Access {
			target: root,
			members,
		} = syntax::view(&tree, self.strip_groups(target))
		else {
			return;
		};
		let TypeKind::Param(param) = self.s.types.kind(self.type_of(root)) else {
			return;
		};
		if !self.is_implicit_param(param) {
			return;
		}
		let mut path = Vec::with_capacity(members.len());
		for member in members.iter() {
			let View::Name(name) = syntax::view(&tree, member) else {
				return;
			};
			path.push(self.s.types.syms.intern(tree.name(name)));
		}
		let writes = self.implicit_writes.entry(param).or_default();
		if !writes.iter().any(|(known, _)| *known == path) {
			writes.push((path, written));
		}
	}

	/// Whether a type parameter is the hidden one of `Type(parameter)`.
	pub(super) fn is_implicit_param(&self, param: ParamId) -> bool {
		self.s
			.types
			.syms
			.resolve(self.s.types.param(param).name)
			.starts_with("Type<")
	}

	/// Notes a call of a function of this file whose parameters may be
	/// implicit generic ones: checked once every body is walked
	/// ([`Walker::finish_implicit_writes`]), so that a call checked before
	/// the callee's body (recursion) sees its writes.
	pub(super) fn note_implicit_write_call(
		&mut self,
		callee: NodeId,
		declaration: NodeId,
		arguments: &[crate::analysis::Argument],
	) {
		let implicit = self.a.function(declaration).is_some_and(|info| {
			info.params.iter().any(|param| {
				matches!(
					self.s.types.kind(self.a.symbols.symbol(param.symbol).ty),
					TypeKind::Param(_)
				)
			})
		});
		if implicit {
			let text = self.tree.text(callee.index()).to_string();
			self.implicit_write_calls
				.push((text, declaration, arguments.to_vec()));
		}
	}

	/// At each call noted by [`Walker::note_implicit_write_call`]: a field
	/// the callee writes into a copy of an argument typed `Type<parameter>`
	/// has exactly the written type in the argument (`EF004`, like `STR004`
	/// for a write through the parameter itself): the copy is returned as
	/// the argument's type.
	pub(super) fn finish_implicit_writes(&mut self) {
		let calls = std::mem::take(&mut self.implicit_write_calls);
		// A wrapper may pass its own rigid argument to a copying function.
		// Propagate those writes before validating any call of the wrapper,
		// including calls seen before its body or through a recursive cycle.
		loop {
			let mut changed = false;
			for (_, declaration, arguments) in &calls {
				let Some(info) = self.a.function(*declaration) else {
					continue;
				};
				let parameters: Vec<_> = info
					.params
					.iter()
					.map(|param| self.a.symbols.symbol(param.symbol).ty)
					.collect();
				changed |= self.propagate_implicit_write_arguments(&parameters, arguments);
			}
			if !changed {
				break;
			}
		}
		for (callee, declaration, arguments) in calls {
			let Some(info) = self.a.function(declaration) else {
				continue;
			};
			let parameters: Vec<TypeId> = info
				.params
				.iter()
				.map(|param| self.a.symbols.symbol(param.symbol).ty)
				.collect();
			self.check_implicit_write_arguments(&callee, &parameters, &arguments);
		}
	}

	/// Writes inherited by a wrapper passing its own type parameter onward.
	fn propagate_implicit_write_arguments(
		&mut self,
		parameters: &[TypeId],
		arguments: &[crate::analysis::Argument],
	) -> bool {
		let mut changed = false;
		for (index, &ty) in parameters.iter().enumerate() {
			let TypeKind::Param(source) = self.s.types.kind(ty) else {
				continue;
			};
			let Some(writes) = self.implicit_writes.get(&source).cloned() else {
				continue;
			};
			let Some(&crate::analysis::Argument::Given(argument)) = arguments.get(index) else {
				continue;
			};
			let TypeKind::Param(target) = self.s.types.kind(self.type_of(argument)) else {
				continue;
			};
			let inherited = self.implicit_writes.entry(target).or_default();
			for write in writes {
				if !inherited.contains(&write) {
					inherited.push(write);
					changed = true;
				}
			}
		}
		changed
	}

	/// The same copy-write rule for local and imported function types.
	pub(super) fn check_implicit_write_arguments(
		&mut self,
		callee: &str,
		parameters: &[TypeId],
		arguments: &[crate::analysis::Argument],
	) {
		self.propagate_implicit_write_arguments(parameters, arguments);
		for (index, &ty) in parameters.iter().enumerate() {
			let TypeKind::Param(implicit) = self.s.types.kind(ty) else {
				continue;
			};
			let Some(writes) = self.implicit_writes.get(&implicit).cloned() else {
				continue;
			};
			let Some(&crate::analysis::Argument::Given(argument)) = arguments.get(index) else {
				continue;
			};
			for (path, written) in writes {
				let mut actual = self.type_of(argument);
				for &name in &path {
					let object = self.s.types.unfold(actual);
					let Some(field) = self
						.s
						.types
						.fields_of(object)
						.iter()
						.find(|field| field.name == name)
						.copied()
					else {
						actual = TypeId::UNKNOWN;
						break;
					};
					actual = field.ty;
				}
				let unsettled = |w: &Self, ty: TypeId| {
					w.s.types.is_recovering(ty)
						|| w.s.types.flags(ty).intersects(
							crate::types::TypeFlags::HAS_PARAM
								| crate::types::TypeFlags::HAS_UNKNOWN,
						)
				};
				if actual == written || unsettled(self, actual) || unsettled(self, written) {
					continue;
				}
				let field: Vec<&str> = path
					.iter()
					.map(|&name| self.s.types.syms.resolve(name))
					.collect();
				let field = field.join(".");
				let message = format!(
					"`{callee}` writes `{field}: {}` into a copy of `{}` that it returns as its type, whose `{field}` is a `{}`: a written field has exactly the type the function writes",
					self.display(written),
					self.tree.text(argument.index()),
					self.display(actual)
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::types::EF004, argument, message);
				self.report(diagnostic);
				break;
			}
		}
	}

	/// The declared type of a parameter named by `Type(parameter)`: its
	/// hidden generic parameter, constrained by the written type.
	pub(super) fn implicit_parameter_type(&mut self, name: Sym, declared: TypeId) -> TypeId {
		let Some(&(_, param)) = self
			.pending_implicit
			.iter()
			.find(|(known, _)| *known == name)
		else {
			return declared;
		};
		self.s
			.types
			.set_param_bounds(param, declared, TypeId::UNSET);
		self.s.types.intern(TypeKind::Param(param))
	}

	/// `Type(value)` in a type: the type of a value without its facts; for
	/// a parameter, the type of its argument (an implicit generic, see
	/// [`Walker::implicit_generics`]). `TF020` on a type. Type part.
	pub(super) fn type_of_value(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let tree = Arc::clone(&self.tree);
		let argument = arguments.first().filter(|_| arguments.len() == 1);
		let ty = match argument.map(|argument| (argument, syntax::view(&tree, argument))) {
			Some((argument, View::Name(name))) => {
				let sym = self.s.types.syms.intern(tree.name(name));
				let local = self.a.symbols.lookup(self.body.scope, sym);
				let builtin_type = local.is_none()
					&& (crate::builtins::lookup(tree.name(name)).is_some_and(|builtin| {
						crate::builtins::denoted_type(&mut self.s.types, builtin).is_some()
					}) || super::type_expr::collection_kind(tree.name(name)).is_some());
				match local {
					_ if builtin_type => self.memo(n, |w| {
						let message = format!(
							"`{}` is already a type: write it without `Type<…>`",
							tree.name(name)
						);
						let diagnostic = w.diagnostic(&TF020, argument, message);
						w.report(diagnostic);
						TypeId::UNKNOWN
					}),
					Some(symbol) if self.a.symbols.symbol(symbol).kind == SymbolKind::Type => self
						.memo(n, |w| {
							let message = format!(
								"`{}` is already a type: write it without `Type<…>`",
								tree.name(name)
							);
							let diagnostic = w.diagnostic(&TF020, argument, message);
							w.report(diagnostic);
							TypeId::UNKNOWN
						}),
					_ => {
						let ty = self.check(argument, None);
						// the declared type, without the facts known here
						// (`if value is String`: still the union)
						let declared = match self.a.resolution(argument) {
							crate::symbols::Resolution::Local(symbol) => {
								let ty = self.a.symbols.symbol(symbol).ty;
								(!ty.is_unset() && !self.s.types.is_recovering(ty)).then_some(ty)
							}
							_ => None,
						};
						self.s.types.widen(declared.unwrap_or(ty))
					}
				}
			}
			Some((argument, _)) => {
				let ty = self.check(argument, None);
				self.s.types.widen(ty)
			}
			None => TypeId::UNKNOWN,
		};
		let ty = self.memo(n, |w| {
			if argument.is_none() {
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::types::TY003,
					n,
					"`Type` takes one value, ex: `Type(user)`",
				);
				w.report(diagnostic);
			}
			ty
		});
		self.memo(callee, |_| ty)
	}

	/// The `type` declaration an expression or annotation names.
	pub(super) fn written_decl(&self, n: NodeId) -> Option<NodeId> {
		match self.view(n) {
			View::Name(_) => match self.a.resolution(n) {
				Resolution::Local(symbol) => {
					let data = self.a.symbols.symbol(symbol);
					match data.kind {
						SymbolKind::Type => Some(data.decl_node),
						_ => self.written_decls.get(&symbol).copied(),
					}
				}
				_ => None,
			},
			View::TypedObject { target, .. } => self.written_decl(target),
			View::Group(inner) | View::Copy(inner) => self.written_decl(inner),
			_ => None,
		}
	}

	/// The portable identity of the type written for a value. Only a
	/// declaration of that type may supply its methods; equal fields do not
	/// make the methods of another declaration available.
	pub(super) fn written_origin(&mut self, n: NodeId) -> Option<WrittenType> {
		if let Some(origin) = self.a.written_types.get(&n).cloned() {
			return Some(origin);
		}
		if !self.written_origin_stack.insert(n) {
			return None;
		}
		let origin = self.written_origin_inner(n);
		self.written_origin_stack.remove(&n);
		if let Some(origin) = &origin {
			self.a.written_types.insert(n, origin.clone());
		}
		origin
	}

	fn written_origin_inner(&mut self, n: NodeId) -> Option<WrittenType> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Name(_) => match self.a.resolution(n) {
				Resolution::Local(symbol) => {
					let data = self.a.symbols.symbol(symbol);
					if data.kind == SymbolKind::Type
						&& !matches!(self.s.types.kind(data.ty), TypeKind::Param(_))
					{
						Some(WrittenType::named(
							self.external_value_key(symbol)
								.unwrap_or_else(|| self.symbol_key(symbol)),
						))
					} else {
						match self.external_value_key(symbol) {
							Some(key) => self.external_written_origin(key),
							None => self.written_type_keys.get(&symbol).cloned(),
						}
					}
				}
				Resolution::External(key) => self.external_written_origin(key),
				_ => None,
			},
			View::Access { members, .. } => {
				let member = members.last()?;
				match self.a.resolution(member) {
					Resolution::External(key) => self.external_written_origin(key),
					Resolution::Local(symbol)
						if self.a.symbols.symbol(symbol).kind == SymbolKind::Type =>
					{
						Some(WrittenType::named(
							self.external_value_key(symbol)
								.unwrap_or_else(|| self.symbol_key(symbol)),
						))
					}
					Resolution::Local(symbol) => self
						.external_value_key(symbol)
						.and_then(|key| self.external_written_origin(key)),
					_ => None,
				}
			}
			View::TypedObject { target, .. } => self.written_origin(target),
			View::Group(inner) | View::Copy(inner) | View::Optional(inner) => {
				self.written_origin(inner)
			}
			View::Call { callee, arguments }
				if self.a.has_flags(n, NodeFlags::TYPE_EXPR)
					&& matches!(self.view(callee), View::Name(_))
					&& matches!(
						self.a.resolution(callee),
						Resolution::Unresolved | Resolution::Builtin(_)
					) =>
			{
				let origins = arguments
					.iter()
					.map(|argument| self.written_origin(argument))
					.collect();
				self.container_origin(self.type_of(n), origins)
			}
			View::Array { items } => {
				let origins: Vec<_> = items.iter().map(|item| self.written_origin(item)).collect();
				let element =
					origins.first().cloned().flatten().filter(|origin| {
						origins.iter().all(|other| other.as_ref() == Some(origin))
					});
				self.container_origin(self.type_of(n), vec![element])
			}
			View::Tuple { items } => {
				let origins = items.iter().map(|item| self.written_origin(item)).collect();
				self.container_origin(self.type_of(n), origins)
			}
			View::Index { target, index } => {
				let owner = self.written_origin(target)?;
				if matches!(self.view(self.strip_groups(index)), View::Range { .. }) {
					Some(owner)
				} else {
					let slot = match self.s.types.kind(self.s.types.unfold(self.type_of(target))) {
						TypeKind::Collection(
							crate::types::CollKind::Map | crate::types::CollKind::GrowingMap,
							..,
						) => 1,
						TypeKind::Collection(..) | TypeKind::CArray(..) => 0,
						TypeKind::Tuple(list) => {
							let value = self.constant_integer(index)?.as_i128();
							let count = self.s.types.list_items(list).len() as i128;
							usize::try_from(if value < 0 { count + value } else { value }).ok()?
						}
						_ => return None,
					};
					owner.origins.get(slot).cloned().flatten()
				}
			}
			View::Call { callee, .. }
				if self.a.instantiation(n).is_some()
					&& self
						.a
						.flags_of(n)
						.intersects(NodeFlags::GENERIC_APPLICATION | NodeFlags::TYPE_EXPR) =>
			{
				self.written_origin(callee).map(|mut origin| {
					if let Some(arguments) = self.a.instantiation(n) {
						origin.arguments = arguments;
						let mut origins = vec![None; self.s.types.list_items(arguments).len()];
						if let Some(given) = self.a.instantiation_origins.get(&n) {
							for (index, origin) in given {
								origins[*index as usize] = Some(origin.clone());
							}
						}
						origin.origins = std::sync::Arc::new(origins);
						origin.methods = std::sync::Arc::new(
							self.a
								.instantiated_methods
								.get(&n)
								.cloned()
								.unwrap_or_default(),
						);
					}
					origin
				})
			}
			View::Call { callee, .. } => self.written_call_return(n, callee),
			_ => None,
		}
	}

	/// Built-in constructors have no user declaration, but their arguments
	/// still have written types (`Array<Greeting>`, `Pointer<Resource<T>>`).
	fn container_origin(
		&mut self,
		ty: TypeId,
		mut origins: Vec<Option<WrittenType>>,
	) -> Option<WrittenType> {
		let (name, arguments) = match self.s.types.kind(self.s.types.unfold(ty)) {
			TypeKind::Collection(kind, item, key) => {
				let arguments = if matches!(
					kind,
					crate::types::CollKind::Map | crate::types::CollKind::GrowingMap
				) {
					vec![key, item]
				} else {
					vec![item]
				};
				(kind.name(), arguments)
			}
			TypeKind::Pointer(item) => ("Pointer", vec![item]),
			TypeKind::CArray(item, _) => ("CArray", vec![item]),
			TypeKind::Iterator(item, _) => ("Iterator", vec![item]),
			TypeKind::Tuple(list) => ("Tuple", self.s.types.list_items(list).to_vec()),
			_ => return None,
		};
		if !origins.iter().any(Option::is_some) {
			return None;
		}
		origins.resize(arguments.len(), None);
		let module = self.s.types.module("prelude");
		let declaration = self.s.types.decl_key(module, &[name], 0);
		Some(WrittenType {
			declaration,
			arguments: self.s.types.list(&arguments),
			origins: Arc::new(origins),
			methods: Arc::default(),
		})
	}

	pub(super) fn written_element_origin(&mut self, iterable: NodeId) -> Option<WrittenType> {
		let owner = self.written_origin(iterable)?;
		let owner = self.resolve_written_type(owner);
		let ty = self.s.types.unfold(self.type_of(iterable));
		if let TypeKind::Collection(
			crate::types::CollKind::Map | crate::types::CollKind::GrowingMap,
			item,
			key,
		) = self.s.types.kind(ty)
		{
			let tuple = self.s.types.tuple(&[key, item]);
			self.container_origin(tuple, owner.origins.as_ref().clone())
		} else {
			owner.origins.first().cloned().flatten()
		}
	}

	fn written_builtin_return(
		&mut self,
		call: NodeId,
		callee: NodeId,
		builtin: crate::ids::BuiltinId,
	) -> Option<WrittenType> {
		let (View::Access { members, .. } | View::OptionalAccess { members, .. }) =
			self.view(self.strip_groups(callee))
		else {
			return None;
		};
		let owner = self.a.method_instantiations.get(&members.last()?)?.clone();
		let owner = self.resolve_written_type(owner);
		let path = crate::builtins::get(builtin).path;
		let (kind, name) = path.split_once('#')?;
		let item = usize::from(kind == "Map");
		let origin = |index| owner.origins.get(index).cloned().flatten();
		match (kind, name) {
			("Pointer", "read") => origin(0),
			(
				"Array" | "Set" | "Map" | "Pool" | "Columnar" | "OrderedColumnar" | "Iterator",
				"get" | "take" | "pop" | "find" | "getOrInsert",
			) => origin(item),
			(
				"Array" | "Set" | "Map" | "Pool" | "Columnar" | "OrderedColumnar" | "Iterator"
				| "Pointer",
				"values" | "keys" | "toArray" | "firstValues",
			) => self.container_origin(
				self.type_of(call),
				vec![origin(if name == "keys" { 0 } else { item })],
			),
			("Array" | "Iterator", "reverse" | "sort" | "filter") => {
				self.container_origin(self.type_of(call), owner.origins.as_ref().clone())
			}
			_ => None,
		}
	}

	pub(super) fn written_function_return(&mut self, function: NodeId) -> Option<WrittenReturn> {
		let info = self.a.function(function)?.clone();
		let ret = self.s.types.sig(info.sig?).ret;
		if let TypeKind::Param(param) = self.s.types.kind(ret) {
			return info
				.generics
				.iter()
				.position(|&generic| generic == param)
				.map(|index| WrittenReturn::Generic(index as u16));
		}
		let View::Lambda { signature, body } = self.view(info.lambda) else {
			return None;
		};
		let origin = match signature.returns {
			Some(annotation) => self.written_origin(annotation),
			None if !info.block_body => self.written_origin(body),
			None => None,
		};
		origin.map(WrittenReturn::Named)
	}

	fn written_call_return(&mut self, call: NodeId, callee: NodeId) -> Option<WrittenType> {
		let named = match self.a.call(call)?.callee {
			Callee::Builtin(builtin) => return self.written_builtin_return(call, callee, builtin),
			Callee::Function(function) => self.written_function_return(function),
			Callee::Value => {
				let last = match self.view(self.strip_groups(callee)) {
					View::Access { members, .. } => members.last()?,
					_ => self.strip_groups(callee),
				};
				match self.a.resolution(last) {
					Resolution::External(key) => {
						let module = self.s.types.decl_key_data(key).module;
						let interface = self.s.interface(module)?;
						let declaration = interface.node_of(key)?;
						interface.written_returns.get(&declaration).cloned()
					}
					Resolution::Local(symbol) => match self.external_value_key(symbol) {
						Some(key) => {
							let module = self.s.types.decl_key_data(key).module;
							let interface = self.s.interface(module)?;
							let declaration = interface.node_of(key)?;
							interface.written_returns.get(&declaration).cloned()
						}
						None => {
							self.written_function_return(self.a.symbols.symbol(symbol).decl_node)
						}
					},
					_ => None,
				}
			}
			_ => None,
		}?;
		match named {
			WrittenReturn::Named(origin) => {
				let generics = self.callee_generics(callee, self.a.call(call)?.callee);
				let arguments = self
					.a
					.instantiation(call)
					.map(|list| self.s.types.list_items(list).to_vec())
					.unwrap_or_default();
				let offset = arguments.len().saturating_sub(generics.len());
				let mut bindings = crate::types::Bindings {
					params: generics,
					types: arguments[offset..].to_vec(),
					..Default::default()
				};
				if let Some(origins) = self.a.instantiation_origins.get(&call) {
					for (index, origin) in origins {
						if let Some(&param) = (*index as usize)
							.checked_sub(offset)
							.and_then(|index| bindings.params.get(index))
						{
							bindings.origins.insert(param, origin.clone());
						}
					}
				}
				Some(self.substitute_written_type(origin, &bindings))
			}
			WrittenReturn::Generic(parameter) => self
				.a
				.instantiation_origins
				.get(&call)?
				.iter()
				.find(|(position, _)| *position == parameter)
				.map(|(_, origin)| origin.clone()),
		}
	}

	/// The annotation of a field belongs to its owner, even when another
	/// equal shape declared a field of the same data type.
	pub(super) fn written_field_origin(
		&mut self,
		owner: WrittenType,
		name: Sym,
	) -> Option<WrittenType> {
		let owner = self.resolve_written_type(owner);
		let module = self.s.types.decl_key_data(owner.declaration).module;
		if self.s.types.module_path(module) == "prelude"
			&& self.s.types.decl_name(owner.declaration) == "Pointer"
			&& self.s.types.syms.resolve(name) == "value"
		{
			return owner.origins.first().cloned().flatten();
		}
		let projection = if module == self.s.module_id(self.file) {
			let symbol = *self.modules.keyed.get(&owner.declaration)?;
			let info = self
				.a
				.type_decls
				.get(&self.a.symbols.symbol(symbol).decl_node)?;
			let field = info.fields.iter().find(|field| {
				self.s.types.syms.resolve(field.name) == self.s.types.syms.resolve(name)
			})?;
			self.a
				.written_types
				.get(&field.node)
				.cloned()
				.map(WrittenReturn::Named)
				.or_else(|| match self.s.types.kind(field.ty) {
					TypeKind::Param(param) => info
						.generics
						.iter()
						.position(|&generic| generic == param)
						.map(|index| WrittenReturn::Generic(index as u16)),
					_ => None,
				})
		} else {
			let interface = self.s.interface(module)?;
			let declaration = interface.node_of(owner.declaration)?;
			interface
				.written_fields
				.iter()
				.find(|((decl, field), _)| {
					*decl == declaration
						&& self.s.types.syms.resolve(*field) == self.s.types.syms.resolve(name)
				})
				.map(|(_, origin)| origin.clone())
		}?;
		match projection {
			WrittenReturn::Generic(index) => owner.origins.get(index as usize).cloned().flatten(),
			WrittenReturn::Named(origin) => match self.written_bindings(&owner) {
				Some(bindings) => Some(self.substitute_written_type(origin, &bindings)),
				None => Some(origin),
			},
		}
	}

	fn substitute_written_type(
		&mut self,
		mut origin: WrittenType,
		bindings: &crate::types::Bindings,
	) -> WrittenType {
		let arguments = self.s.types.list_items(origin.arguments).to_vec();
		let mut origins = Vec::with_capacity(arguments.len());
		for (index, &ty) in arguments.iter().enumerate() {
			let bound = match self.s.types.kind(ty) {
				TypeKind::Param(param) => bindings.origins.get(&param).cloned(),
				_ => None,
			};
			origins.push(bound.or_else(|| {
				origin
					.origins
					.get(index)
					.cloned()
					.flatten()
					.map(|origin| self.substitute_written_type(origin, bindings))
			}));
		}
		let types: Vec<_> = arguments
			.iter()
			.map(|&ty| self.s.types.substitute(ty, bindings))
			.collect();
		origin.arguments = self.s.types.list(&types);
		origin.origins = std::sync::Arc::new(origins);
		if let Some(mut owner) = self.written_bindings(&origin) {
			let mut problems = Vec::new();
			self.check_constraints(&mut owner, &mut problems, false);
			origin.methods = std::sync::Arc::new(
				owner
					.methods
					.iter()
					.filter_map(|(param, method)| {
						owner
							.params
							.iter()
							.position(|candidate| candidate == param)
							.map(|index| crate::analysis::GenericMethod {
								parameter: index as u16,
								..method.clone()
							})
					})
					.collect(),
			);
		}
		origin
	}

	fn external_written_origin(&mut self, key: DeclKey) -> Option<WrittenType> {
		let data = self.s.types.decl_key_data(key);
		let interface = self.s.interface(data.module)?;
		let node = interface.node_of(key)?;
		if interface.declared(node)?.kind == SymbolKind::Type {
			return Some(WrittenType::named(key));
		}
		interface.written_types.get(&node).cloned()
	}

	/// The substitution attached to a written application. Structural type
	/// ids cannot recover phantom arguments, so it comes from syntax metadata.
	pub(super) fn written_bindings(&self, origin: &WrittenType) -> Option<crate::types::Bindings> {
		let data = self.s.types.decl_key_data(origin.declaration);
		let params = if data.module == self.s.module_id(self.file) {
			let &symbol = self.modules.keyed.get(&origin.declaration)?;
			self.generic_params
				.get(&self.a.symbols.symbol(symbol).decl_node)?
				.clone()
		} else {
			let interface = self.s.interface(data.module)?;
			let declaration = interface.node_of(origin.declaration)?;
			let &list = interface.type_parameters.get(&declaration)?;
			self.s
				.types
				.list_items(list)
				.iter()
				.filter_map(|&ty| match self.s.types.kind(ty) {
					TypeKind::Param(param) => Some(param),
					_ => None,
				})
				.collect()
		};
		let arguments = self.s.types.list_items(origin.arguments);
		if arguments.len() != params.len() {
			return None;
		}
		let origins = params
			.iter()
			.enumerate()
			.filter_map(|(index, &param)| {
				origin
					.origins
					.get(index)
					.cloned()
					.flatten()
					.map(|origin| (param, origin))
			})
			.collect();
		Some(crate::types::Bindings {
			params,
			types: arguments.to_vec(),
			origins,
			..Default::default()
		})
	}

	/// A named alias inherits the explicitly named target's methods and
	/// generic arguments, including parameters absent from the data layout.
	pub(super) fn resolve_written_type(&mut self, mut origin: WrittenType) -> WrittenType {
		let mut seen = rustc_hash::FxHashSet::default();
		while seen.insert(origin.declaration) {
			let data = self.s.types.decl_key_data(origin.declaration);
			if data.module != self.s.module_id(self.file) && self.s.interface(data.module).is_some()
			{
				self.modules
					.reads
					.push(self.s.types.module_path(data.module).to_string());
			}
			let target = if data.module == self.s.module_id(self.file) {
				self.modules
					.keyed
					.get(&origin.declaration)
					.copied()
					.and_then(|symbol| {
						let decl = self.a.symbols.symbol(symbol).decl_node;
						match self.view(decl) {
							View::TypeDecl { value, .. } | View::ExportType { value, .. } => {
								Some(value)
							}
							_ => None,
						}
					})
					.and_then(|value| self.written_origin(value))
			} else {
				self.s.interface(data.module).and_then(|interface| {
					interface
						.node_of(origin.declaration)
						.and_then(|decl| interface.written_types.get(&decl))
						.cloned()
				})
			};
			let Some(mut target) = target else {
				break;
			};
			if let Some(bindings) = self.written_bindings(&origin) {
				target = self.substitute_written_type(target, &bindings);
			}
			origin = target;
		}
		origin
	}

	/// The object type of a value whose fields can be read: an object, or a
	/// member of a named union with object data.
	fn object_of(&self, ty: TypeId) -> Option<TypeId> {
		let ty = self.s.types.unfold(ty);
		match self.s.types.kind(ty) {
			TypeKind::Object(_) => Some(ty),
			TypeKind::Member(nominal, index) => {
				let payload = self.s.types.payload(nominal, index);
				matches!(self.s.types.kind(payload), TypeKind::Object(_)).then_some(payload)
			}
			_ => None,
		}
	}

	/// `STR007` (advanced brief §10.3): a callee that mutates `path` writes
	/// the fields it names (the last field of the path, or every field of
	/// `declared`, its type there, when the path ends at an object): none of
	/// them may be `readonly` in the argument. Flow part of a call.
	pub(super) fn check_readonly_written(
		&mut self,
		argument: NodeId,
		path: crate::ids::PathId,
		declared: TypeId,
		callee: &str,
	) {
		let Some((ty, last)) = self.argument_at_path(argument, path) else {
			return;
		};
		let readonly = |w: &Self, owner: TypeId, name: Sym| {
			w.s.types.fields_of(owner).iter().any(|field| {
				field.name == name && field.flags.contains(crate::types::FieldFlags::READONLY)
			})
		};
		let written: Option<Sym> = match last {
			Some((owner, name)) if readonly(self, owner, name) => Some(name),
			_ => self
				.s
				.types
				.fields_of(declared)
				.iter()
				.filter(|field| !field.flags.contains(crate::types::FieldFlags::READONLY))
				.map(|field| field.name)
				.find(|&name| readonly(self, ty, name)),
		};
		let Some(name) = written else {
			self.check_written_types(argument, ty, last, declared, callee);
			return;
		};
		let field = self.s.types.syms.resolve(name).to_string();
		let message = format!(
			"`{callee}` may write `{field}`, but `{field}` is readonly in `{}`: only the methods of its type change it",
			self.display(self.type_of(argument))
		);
		let diagnostic = self.diagnostic(
			&crate::diagnostics::codes::advanced::STR007,
			argument,
			message,
		);
		self.report_flow(diagnostic);
	}

	/// The type of an argument at the fields of a callee's effect path on
	/// one of its parameters, and the object and field the path ends with.
	/// `None` for another root, or a path through elements.
	fn argument_at_path(
		&self,
		argument: NodeId,
		path: crate::ids::PathId,
	) -> Option<(TypeId, Option<(TypeId, Sym)>)> {
		let (data, steps) = self.s.types.path_data(path);
		if !matches!(data.root, crate::types::PathRoot::Param(_)) {
			return None;
		}
		let mut ty = self.type_of(argument);
		let mut last: Option<(TypeId, Sym)> = None;
		for step in steps {
			let crate::types::PathStep::Field(name) = *step else {
				return None;
			};
			let field = self
				.s
				.types
				.fields_of(ty)
				.iter()
				.find(|field| field.name == name)
				.copied()?;
			last = Some((ty, name));
			ty = field.ty;
		}
		Some((ty, last))
	}

	/// Writing is invariant (advanced brief §10.5): a field the callee
	/// writes has exactly its type in the argument (`STR004`), a mutated
	/// collection has exactly its element type (`STR005`). `actual` is the
	/// argument's type at the path, `declared` the callee's.
	fn check_written_types(
		&mut self,
		argument: NodeId,
		actual: TypeId,
		last: Option<(TypeId, Sym)>,
		declared: TypeId,
		callee: &str,
	) {
		let unsettled = |w: &Self, ty: TypeId| {
			w.s.types.is_recovering(ty)
				|| w.s.types.flags(ty).intersects(
					crate::types::TypeFlags::HAS_PARAM | crate::types::TypeFlags::HAS_UNKNOWN,
				)
		};
		if unsettled(self, actual) || unsettled(self, declared) || actual == declared {
			return;
		}
		let argument_text = self.tree.text(argument.index()).to_string();
		if let (TypeKind::Collection(_, item, _), TypeKind::Collection(_, actual_item, _)) =
			(self.s.types.kind(declared), self.s.types.kind(actual))
		{
			if item != actual_item {
				let message = format!(
					"`{callee}` mutates `{argument_text}`, so it needs a `{}`, not a `{}`: make it generic",
					self.display(declared),
					self.display(actual)
				);
				let diagnostic = self.diagnostic(
					&crate::diagnostics::codes::advanced::STR005,
					argument,
					message,
				);
				self.report_flow(diagnostic);
			}
			return;
		}
		// the field the path ends with, or the fields of the object it designates
		let written: Vec<(Sym, TypeId, TypeId)> = match last {
			Some((_, name)) => vec![(name, declared, actual)],
			None => {
				let actual_fields = self.s.types.fields_of(actual).to_vec();
				self.s
					.types
					.fields_of(declared)
					.iter()
					.filter(|field| {
						!field.flags.intersects(
							crate::types::FieldFlags::READONLY | crate::types::FieldFlags::METHOD,
						)
					})
					.filter_map(|field| {
						let other = actual_fields
							.iter()
							.find(|other| other.name == field.name)?;
						Some((field.name, field.ty, other.ty))
					})
					.collect()
			}
		};
		for (name, written_ty, actual_ty) in written {
			if written_ty == actual_ty || unsettled(self, written_ty) || unsettled(self, actual_ty)
			{
				continue;
			}
			let field = self.s.types.syms.resolve(name).to_string();
			let message = format!(
				"`{callee}` writes `{field}: {}` into `{argument_text}`, whose `{field}` is a `{}`: a written field has exactly the type the function writes",
				self.display(written_ty),
				self.display(actual_ty)
			);
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::advanced::STR004,
				argument,
				message,
			);
			self.report_flow(diagnostic);
			return;
		}
	}

	/// `STR001`: an existing value lacks a field of the expected type;
	/// `STR003` when that field is optional (only a literal may leave an
	/// optional field out). Flow part of a sink.
	pub(super) fn report_missing_field(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		let actual_fields: Vec<FieldEntry> = self
			.object_of(actual)
			.map(|object| self.s.types.fields_of(object).to_vec())
			.unwrap_or_default();
		let missing = self
			.s
			.types
			.fields_of(target)
			.iter()
			.copied()
			.find(|field| !actual_fields.iter().any(|other| other.name == field.name));
		let Some(missing) = missing else {
			return;
		};
		let name = self.s.types.syms.resolve(missing.name).to_string();
		let optional = self.s.types.satisfies(TypeId::NONE, missing.ty);
		let (code, message) = if optional {
			(
				&STR003,
				format!(
					"a `{}` has no `{name}` field: a `{}` is expected (only a literal may leave an optional field out)",
					self.display(actual),
					self.display(target)
				),
			)
		} else {
			(
				&STR001,
				format!(
					"a `{}` has no field `{name}: {}`: a `{}` is expected",
					self.display(actual),
					self.display(missing.ty),
					self.display(target)
				),
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
	}

	/// `E-TYPE-1` (alias `STR009`): a value of a nominal type (a named union, `Errors`, an
	/// enumeration, a type with private fields) where another nominal type is
	/// expected. Flow part of a sink that failed; whether it reported.
	pub(super) fn nominal_mismatch(&mut self, n: NodeId, actual: TypeId, target: TypeId) -> bool {
		let (Some(source), Some(expected)) = (self.nominal_of(actual), self.nominal_of(target))
		else {
			return false;
		};
		if source == expected {
			return false;
		}
		let message = format!(
			"a `{}` is never a `{}`: each declaration of a named union, an enumeration or a type with private fields is its own type",
			self.display(actual),
			self.display(target)
		);
		let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
		self.report_flow(diagnostic);
		true
	}

	/// The declaration of a nominal type.
	fn nominal_of(&self, ty: TypeId) -> Option<crate::ids::DeclKey> {
		let nominal = match self.s.types.kind(self.s.types.unfold(ty)) {
			TypeKind::NamedUnion(nominal, _)
			| TypeKind::Member(nominal, _)
			| TypeKind::Enum(nominal, _)
			| TypeKind::EnumMember(nominal, _) => nominal,
			TypeKind::Object(shape) => self.s.types.shape(shape).nominal?,
			_ => return None,
		};
		let info = self.s.types.nominal(nominal);
		(info.kind != NominalKind::CStruct).then_some(info.key)
	}

	/// A value copied into a place of another object type (a binding, a
	/// field, a collection element, a return value): the copy keeps the
	/// target's fields only, element by element for a collection
	/// (`coercions[n]` = the target type). A parameter is a view and copies
	/// nothing ([`Walker::view_sink`]). Flow part of a sink that succeeded.
	pub(super) fn record_narrowing_copy(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		if self.view_sink || !self.narrows(actual, target, 0) {
			return;
		}
		self.a.coercions.insert(n, target);
	}

	/// Whether a copy from `actual` to `target` must drop or convert fields.
	fn narrows(&self, actual: TypeId, target: TypeId, depth: u32) -> bool {
		let (actual, target) = (self.s.types.unfold(actual), self.s.types.unfold(target));
		if actual == target || depth > 8 {
			return false;
		}
		match (self.s.types.kind(actual), self.s.types.kind(target)) {
			(TypeKind::Object(_) | TypeKind::Member(..), TypeKind::Object(_)) => self
				.object_of(actual)
				.is_some_and(|object| object != target),
			(
				TypeKind::Collection(kind, item, key),
				TypeKind::Collection(target_kind, target_item, target_key),
			) if kind == target_kind => {
				self.narrows(item, target_item, depth + 1)
					|| self.narrows(key, target_key, depth + 1)
			}
			(TypeKind::Tuple(items), TypeKind::Tuple(target_items)) => self
				.s
				.types
				.list_items(items)
				.iter()
				.zip(self.s.types.list_items(target_items))
				.any(|(&item, &target_item)| self.narrows(item, target_item, depth + 1)),
			_ => false,
		}
	}
}
