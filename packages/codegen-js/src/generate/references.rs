//! References use existing objects or evaluated container/key pairs. No cells.
use std::collections::HashSet;

use fa_analyzer::analysis::{Argument, Callee, FnInfo, NodeFlags, ReferenceLayout, RequirementArg};
use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, PathRoot, PathStep, TypeKind};

use crate::generate::objects::{LocalChoice, Place};
use crate::generate::{Dest, Generator, Kind};
use crate::js::{self, Expr};

#[derive(Clone, Debug)]
pub(crate) enum Reference {
	Object,
	Place(Parts),
}

#[derive(Clone, Debug)]
enum Key {
	Whole(String, Option<(String, String)>),
	Field(String),
	Index(Expr),
	Entry(Expr),
	Set(Expr),
	/// The innermost container can be an object, array or map.
	Dynamic(Expr),
	Value(Expr),
	Choice {
		selector: Expr,
		locals: Vec<String>,
		key: Expr,
	},
}

#[derive(Clone, Debug)]
pub(crate) struct Parts {
	container: Option<Expr>,
	key: Key,
	optional: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Access {
	Bracket,
	Map,
	Set,
	Dynamic,
}

impl Generator<'_> {
	fn static_reference_key(
		&mut self,
		node: NodeId,
		key: fa_analyzer::ids::ConstId,
		ty: TypeId,
	) -> Expr {
		use fa_analyzer::types::ConstValue;
		match self.types.const_value(key) {
			ConstValue::Bool(value) => Expr::primary(value.to_string()),
			ConstValue::Int(value) => {
				let suffix = if self.repr(self.without_none(ty)) == crate::generate::Repr::BigInt {
					"n"
				} else {
					""
				};
				Expr::primary(format!("{value}{suffix}"))
			}
			ConstValue::Float(bits) => js::float_literal(f64::from_bits(bits)),
			ConstValue::String(name) => {
				Expr::primary(js::string_literal(self.types.syms.resolve(name)))
			}
			ConstValue::BigInt(name) => {
				Expr::primary(format!("{}n", self.types.syms.resolve(name)))
			}
			ConstValue::EnumMember(nominal, index) => self.enum_value(nominal, index),
			ConstValue::Binding(_) => self.unsupported(node, "a nonconstant reference key"),
		}
	}

	pub(crate) fn returns_reference(&self) -> bool {
		self.current_function()
			.and_then(|node| self.analysis.function(node))
			.and_then(|info| info.sig)
			.is_some_and(|sig| self.types.sig(sig).reference.is_some())
	}

	pub(crate) fn call_returns_reference(&self, node: NodeId) -> bool {
		if self.has(node, NodeFlags::GENERIC_APPLICATION) {
			return false;
		}
		let View::Call { callee, .. } = self.view(node) else {
			return false;
		};
		matches!(self.types.kind(self.type_of(callee)), TypeKind::Function(sig) if self.types.sig(sig).reference.is_some())
	}

	pub(crate) fn reference_parameter(&self, call: NodeId, value: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return false;
		};
		let Some(info) = self.analysis.call(call) else {
			return false;
		};
		let params = self.types.params(self.types.sig(sig).params);
		let skip = info.arguments.len().saturating_sub(params.len());
		params
			.iter()
			.zip(info.arguments.iter().skip(skip))
			.any(|(param, argument)| {
				param.reference.is_some()
					&& matches!(argument, Argument::Given(node) if *node == value)
			})
	}

	pub(crate) fn bind_reference_parameter(
		&mut self,
		info: &FnInfo,
		position: usize,
		name: &str,
	) -> Option<String> {
		let param = self
			.types
			.params(self.types.sig(info.sig?).params)
			.get(position)?;
		param.reference?;
		let symbol = info.params[position].symbol;
		if self.is_object_reference(param.ty) {
			self.references.insert(symbol, Reference::Object);
			return None;
		}
		let key = format!("{name}$key");
		self.bind_reference_parts(
			symbol,
			Parts {
				container: Some(Expr::name(name.to_string(), true)),
				key: Key::Dynamic(Expr::name(key.clone(), true)),
				optional: self.may_be_none(param.ty),
			},
		);
		Some(key)
	}

	pub(crate) fn is_object_reference(&self, ty: TypeId) -> bool {
		let ty = self.without_none(ty);
		match self.types.kind(ty) {
			TypeKind::Param(param) => self.reference_parameter_object(param),
			TypeKind::Object(_)
			| TypeKind::Collection(..)
			| TypeKind::Tuple(_)
			| TypeKind::Range
			| TypeKind::CArray(..) => true,
			TypeKind::Union(list) => self
				.types
				.list_items(list)
				.iter()
				.all(|&item| self.is_object_reference(item)),
			TypeKind::Rec(_) => self.is_object_reference(self.types.unfold(ty)),
			_ => false,
		}
	}

	pub(crate) fn without_none(&self, ty: TypeId) -> TypeId {
		if let TypeKind::Union(list) = self.types.kind(ty) {
			let others: Vec<_> = self
				.types
				.list_items(list)
				.iter()
				.copied()
				.filter(|&item| item != TypeId::NONE)
				.collect();
			if let [only] = others.as_slice() {
				return *only;
			}
		}
		ty
	}

	pub(super) fn reference_layout_suffix(layout: ReferenceLayout) -> &'static str {
		match layout {
			ReferenceLayout::Owner => "$key",
			ReferenceLayout::Field(_) => "$field",
			ReferenceLayout::Index(_) => "$index",
			ReferenceLayout::Entry { .. } => "$entry",
			ReferenceLayout::General => "",
		}
	}

	pub(super) fn reference_call_layout(&self, node: NodeId) -> ReferenceLayout {
		let Some(call) = self.analysis.call(node) else {
			return ReferenceLayout::General;
		};
		if self.is_object_reference(self.type_of(node)) {
			return ReferenceLayout::General;
		}
		if let View::Call { callee, .. } = self.view(node)
			&& let TypeKind::Function(sig) = self.types.kind(self.type_of(callee))
			&& let Some(path) = self.types.sig(sig).reference
		{
			let index = match self.types.path_data(path).0.root {
				PathRoot::Param(index) => Some(index as usize),
				PathRoot::SelfParam => Some(0),
				_ => None,
			};
			if index.is_some_and(|index| {
				!matches!(
					call.arguments.get(index),
					Some(Argument::Given(_) | Argument::Receiver(_))
				)
			}) {
				return ReferenceLayout::General;
			}
		}

		match call.callee {
			Callee::Function(declaration) => self
				.analysis
				.reference_layouts
				.get(&declaration)
				.copied()
				.unwrap_or(ReferenceLayout::General),
			Callee::Value => {
				let View::Call { callee, .. } = self.view(node) else {
					return ReferenceLayout::General;
				};
				let named = match self.view(self.strip_groups(callee)) {
					View::Access { members, .. } => members.last().unwrap_or(callee),
					_ => callee,
				};
				let Some((module, declaration)) =
					self.session.external_node(self.analysis.resolution(named))
				else {
					return ReferenceLayout::General;
				};
				self.session
					.interface(module)
					.and_then(|interface| interface.reference_layouts.get(&declaration))
					.copied()
					.unwrap_or(ReferenceLayout::General)
			}
			_ => ReferenceLayout::General,
		}
	}

	pub(crate) fn read_reference_call(&mut self, node: NodeId) -> Expr {
		let View::Call { callee, .. } = self.view(node) else {
			return self.unsupported(node, "this reference call");
		};
		let Some(parts) = self.call_reference_parts(node, callee) else {
			return self.unsupported(node, "this reference call");
		};
		self.read_parts(&parts)
	}

	pub(super) fn reference_call_owner(&mut self, node: NodeId) -> Option<(Expr, Option<bool>)> {
		let View::Call { callee, .. } = self.view(node) else {
			return None;
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return None;
		};
		let sig = self.types.sig(sig);
		let (data, steps) = self.types.path_data(sig.reference?);
		let steps = steps.to_vec();
		let argument_type = |index: usize| match self.analysis.call(node)?.arguments.get(index)? {
			Argument::Given(value) => Some(self.type_of(*value)),
			Argument::Receiver(callee) => Some(self.receiver_type(*callee)),
			_ => None,
		};
		let (mut value, mut ty) = match data.root {
			PathRoot::Param(index) => (
				self.call_arguments
					.get(&node)?
					.get(index as usize)?
					.clone()?,
				argument_type(index as usize)?,
			),
			PathRoot::SelfParam => (
				self.call_arguments.get(&node)?.first()?.clone()?,
				argument_type(0)?,
			),
			PathRoot::ModuleValue(key) => {
				if let Some((module, keyed)) = self.session.keyed(key) {
					if module == self.session.module_id(self.file) {
						let symbol = self.analysis.symbols.symbol_of_decl(keyed.node)?;
						(
							self.name_of(symbol),
							self.analysis.symbols.symbol(symbol).ty,
						)
					} else {
						let resolution = Resolution::External(key);
						let value = self.external(resolution)?;
						let interface = self.session.interface(module)?;
						let ty = interface
							.export
							.iter()
							.chain(interface.members.values())
							.find(|export| export.decl == keyed.node)
							.map(|export| export.ty)
							.unwrap_or(TypeId::UNKNOWN);
						(self.import(&value.path, &value.export), ty)
					}
				} else {
					return None;
				}
			}
			_ => return None,
		};
		for step in steps {
			match step {
				PathStep::Field(name) | PathStep::Private(name) => {
					value.push('.');
					value.push_str(&Self::property(self.types.syms.resolve(name)));
					ty = self
						.types
						.fields_of(ty)
						.iter()
						.find(|field| {
							self.types.syms.resolve(field.name) == self.types.syms.resolve(name)
						})
						.map(|field| field.ty)
						.unwrap_or(TypeId::UNKNOWN);
				}
				_ => return None,
			}
		}
		if self.is_value_pool(ty) {
			return Some((Expr::primary(format!("{value}.$slots")), Some(false)));
		}
		let map = match self.types.kind(ty) {
			TypeKind::Collection(
				CollKind::Map
				| CollKind::GrowingMap
				| CollKind::Pool
				| CollKind::GrowingPool
				| CollKind::Columnar
				| CollKind::GrowingColumnar,
				..,
			) => Some(true),
			TypeKind::Collection(..) => Some(false),
			_ => None,
		};
		Some((Expr::primary(value), map))
	}

	pub(super) fn is_reference_owner_argument(&self, call: NodeId, value: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return false;
		};
		let Some(path) = self.types.sig(sig).reference else {
			return false;
		};
		let index = match self.types.path_data(path).0.root {
			PathRoot::Param(index) => index as usize,
			PathRoot::SelfParam => 0,
			_ => return false,
		};
		matches!(self.analysis.call(call).and_then(|info| info.arguments.get(index)), Some(Argument::Given(node)) if *node == value)
	}

	pub(super) fn reference_pair_slot(
		&self,
		call: NodeId,
		position: usize,
		supplied: usize,
	) -> bool {
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return false;
		};
		let sig = self.types.sig(sig);
		let skip = self
			.analysis
			.call(call)
			.map(|info| info.arguments.len().saturating_sub(supplied))
			.unwrap_or(0);
		let Some(param) = self.types.params(sig.params).get(position + skip) else {
			return false;
		};
		if param.reference.is_none() {
			return false;
		}
		!self.reference_bound_object(call, sig, param.ty)
	}

	fn read_parts(&mut self, parts: &Parts) -> Expr {
		let optional = if parts.optional { "?." } else { "" };
		match &parts.key {
			Key::Whole(text, _) => Expr::primary(text.clone()),
			Key::Choice {
				selector,
				locals,
				key,
			} => {
				let mut value = self.read_parts(&Parts {
					container: parts.container.clone(),
					key: Key::Dynamic(key.clone()),
					optional: parts.optional,
				});
				for (index, local) in locals.iter().enumerate().rev() {
					value = Expr::new(
						format!(
							"{} === {} ? {local} : {}",
							selector.text,
							index + 1,
							value.text
						),
						js::Prec::Conditional,
					);
				}
				Expr::primary(format!("({})", value.text))
			}
			Key::Value(value) | Key::Set(value) => value.clone(),
			Key::Field(field) => Expr::primary(format!(
				"{}{}.{}",
				parts
					.container
					.as_ref()
					.expect("field container")
					.primary_text(),
				if parts.optional { "?" } else { "" },
				field
			)),
			Key::Index(key) => Expr::primary(format!(
				"{}{optional}[{}]",
				parts
					.container
					.as_ref()
					.expect("index container")
					.primary_text(),
				key.text
			)),
			Key::Entry(key) => Expr::primary(format!(
				"{}{}.get({})",
				parts
					.container
					.as_ref()
					.expect("entry container")
					.primary_text(),
				if parts.optional { "?" } else { "" },
				key.text
			)),
			Key::Dynamic(key) => {
				let read = self.helper("faReferenceRead");
				js::call(
					read,
					&[
						parts
							.container
							.as_ref()
							.expect("reference container")
							.text
							.clone(),
						key.text.clone(),
					],
				)
			}
		}
	}

	fn bind_reference_parts(&mut self, symbol: SymbolId, parts: Parts) {
		let value = self.read_parts(&parts);
		self.alias(symbol, value.text);
		if let Key::Entry(key) = &parts.key {
			self.map_aliases.insert(
				symbol,
				(
					parts
						.container
						.as_ref()
						.expect("entry container")
						.text
						.clone(),
					key.text.clone(),
				),
			);
		}
		self.references.insert(symbol, Reference::Place(parts));
	}

	pub(super) fn reference_write(&mut self, symbol: SymbolId) -> Option<(String, String)> {
		let Reference::Place(parts) = self.references.get(&symbol)?.clone() else {
			return None;
		};
		if let Key::Dynamic(key) = parts.key {
			let helper = self.helper("faReferenceSet");
			Some((
				helper.to_string(),
				format!("{}, {}", parts.container?.text, key.text),
			))
		} else {
			None
		}
	}

	pub(super) fn reference_pair(&mut self, value: NodeId) -> Option<(Expr, Expr)> {
		let parts = self.reference_parts(value, true)?;
		self.parts_pair(&parts)
	}

	fn parts_pair(&mut self, parts: &Parts) -> Option<(Expr, Expr)> {
		match &parts.key {
			Key::Whole(_, Some((container, key))) => {
				Some((Expr::primary(container.clone()), Expr::primary(key.clone())))
			}
			Key::Field(field) => Some((
				parts.container.clone()?,
				Expr::primary(js::string_literal(field)),
			)),
			Key::Index(key) | Key::Entry(key) | Key::Dynamic(key) | Key::Set(key) => {
				Some((parts.container.clone()?, key.clone()))
			}
			Key::Value(value) => Some((value.clone(), Expr::primary("null"))),
			Key::Whole(_, None) | Key::Choice { .. } => None,
		}
	}

	/// A physical argument pair, evaluated in the argument's written order.
	pub(crate) fn reference_argument(&mut self, call: NodeId, value: NodeId) -> Expr {
		if self.reference_argument_object(call, value) {
			return self.expr(value);
		}
		let Some((container, key)) = self.reference_pair(value) else {
			return self.unsupported(value, "this reference argument");
		};
		self.reference_argument_keys.insert(value, key);
		container
	}

	pub(super) fn reference_call_generics(
		&self,
		call: NodeId,
		sig: fa_analyzer::types::FnSig,
	) -> Vec<fa_analyzer::ids::ParamId> {
		if let Some(info) = self.analysis.call(call)
			&& let Callee::Function(declaration) = info.callee
		{
			return self.function_generics(declaration);
		}
		let mut generics: Vec<_> = self
			.types
			.list_items(sig.generics)
			.iter()
			.filter_map(|&ty| match self.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect();
		if let View::Call { callee, .. } = self.view(call) {
			let named = match self.view(self.strip_groups(callee)) {
				View::Access { members, .. } => members.last().unwrap_or(callee),
				_ => callee,
			};
			if let Resolution::External(key) = self.analysis.resolution(named)
				&& let Some((module, declaration)) =
					self.session.external_node(Resolution::External(key))
			{
				let mut owner = self.external_owner_parameters(key);
				if let Some(summary) = self
					.session
					.interface(module)
					.and_then(|interface| interface.generic_functions.get(&declaration))
				{
					generics = self
						.types
						.list_items(summary.generics)
						.iter()
						.filter_map(|&ty| match self.types.kind(ty) {
							TypeKind::Param(param) => Some(param),
							_ => None,
						})
						.collect();
				}
				owner.extend(generics);
				generics = owner;
			}
		}
		generics
	}

	fn reference_bound_object(
		&self,
		call: NodeId,
		sig: fa_analyzer::types::FnSig,
		ty: TypeId,
	) -> bool {
		if let TypeKind::Param(param) = self.types.kind(self.without_none(ty)) {
			let generics = self.reference_call_generics(call, sig);
			let (bindings, _) = self.generic_call_bindings(call, &generics);
			if let Some(index) = generics
				.iter()
				.position(|&candidate| self.same_reference_parameter(candidate, param))
				&& let Some(&bound) = bindings.get(index)
			{
				return self.is_object_reference(bound);
			}
		}
		self.is_object_reference(ty)
	}

	pub(super) fn reference_argument_object(&self, call: NodeId, value: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(call) else {
			return self.is_object_reference(self.type_of(value));
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return self.is_object_reference(self.type_of(value));
		};
		let sig = self.types.sig(sig);
		let params = self.types.params(sig.params);
		let Some(info) = self.analysis.call(call) else {
			return false;
		};
		let Some((param, _)) = params
			.iter()
			.zip(
				info.arguments
					.iter()
					.skip(info.arguments.len().saturating_sub(params.len())),
			)
			.find(|(_, argument)| matches!(argument, Argument::Given(node) if *node == value))
		else {
			return false;
		};
		self.reference_bound_object(call, sig, param.ty)
	}

	pub(crate) fn reference_return(&mut self, value: NodeId) {
		if self.is_object_reference(
			self.current_function()
				.and_then(|node| self.analysis.function(node))
				.and_then(|info| info.sig)
				.map(|sig| self.types.sig(sig).ret)
				.unwrap_or(self.type_of(value)),
		) {
			let value = self.expr(value);
			self.out.line(format!("return {};", value.text));
			return;
		}
		if matches!(self.view(self.strip_reference(value)), View::None) {
			self.emit_reference_pair_return(Expr::primary("undefined"), Expr::primary("undefined"));
			return;
		}
		let Some((container, key)) = self.reference_pair(value) else {
			self.unsupported(value, "this reference result");
			return;
		};
		self.emit_reference_pair_return(container, key);
	}

	fn emit_reference_pair_return(&mut self, container: Expr, key: Expr) {
		match self.fcx.reference_layout {
			ReferenceLayout::Owner => self.out.line(format!("return {};", key.text)),
			ReferenceLayout::Field(_)
			| ReferenceLayout::Index(_)
			| ReferenceLayout::Entry { .. } => self.out.line(format!("return {};", container.text)),
			ReferenceLayout::General => {
				let (result_container, result_key, label) = self
					.fcx
					.reference_result
					.clone()
					.expect("general reference result destination");
				self.out
					.line(format!("{result_container} = {};", container.text));
				self.out.line(format!("{result_key} = {};", key.text));
				self.out.line(format!("break {label};"));
			}
		}
	}

	fn primitive_access(&self, value: NodeId) -> Option<Access> {
		let value = self.strip_reference(value);
		match self.view(value) {
			View::None => None,
			View::Call { .. }
				if self.analysis.call(value).is_some_and(
					|call| matches!(call.callee, Callee::Builtin(id) if builtins::get(id).path == "Set#find"),
				) =>
			{
				Some(Access::Set)
			}
			View::Access { .. } => Some(Access::Bracket),
			View::Index { target, .. } => Some(
				if matches!(
					self.types.kind(self.type_of(target)),
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
				) {
					Access::Map
				} else {
					Access::Bracket
				},
			),
			View::Name(_) => {
				if let Resolution::Local(symbol) = self.analysis.resolution(value) {
					Some(match self.references.get(&symbol) {
						Some(Reference::Place(Parts {
							key: Key::Set(_), ..
						})) => Access::Set,
						Some(Reference::Place(Parts {
							key: Key::Entry(_), ..
						})) => Access::Map,
						Some(Reference::Place(Parts {
							key: Key::Index(_) | Key::Field(_),
							..
						})) => Access::Bracket,
						_ => Access::Dynamic,
					})
				} else {
					Some(Access::Dynamic)
				}
			}
			View::Call { .. } => {
				if self.call_returns_reference(value) {
					Some(match self.reference_call_layout(value) {
						ReferenceLayout::Field(_) | ReferenceLayout::Index(_) => Access::Bracket,
						ReferenceLayout::Entry { .. } => Access::Map,
						_ => Access::Dynamic,
					})
				} else if let Some(Callee::Builtin(id)) =
					self.analysis.call(value).map(|call| call.callee)
				{
					Some(if builtins::get(id).path.starts_with("Map#") {
						Access::Map
					} else {
						Access::Bracket
					})
				} else {
					Some(Access::Dynamic)
				}
			}
			_ => Some(Access::Dynamic),
		}
	}

	fn reseatable_access(&self, symbol: SymbolId, initial: NodeId) -> Access {
		let mut access = self.primitive_access(initial);
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if self.analysis.resolution(node) != Resolution::Local(symbol) {
				continue;
			}
			if let View::Reuse { value, .. } = self.view(node)
				&& let Some(next) = self.primitive_access(value)
			{
				if access.is_some_and(|old| old != next) {
					return Access::Dynamic;
				}
				access = Some(next);
			}
		}
		access.unwrap_or(Access::Dynamic)
	}

	/// Plan scalar alternatives without introducing boxes for local variables.
	fn scalar_reference_targets(&mut self, symbol: SymbolId) -> Vec<String> {
		let mut pending = vec![symbol];
		let mut visited = HashSet::new();
		let mut locals = Vec::new();
		while let Some(current) = pending.pop() {
			if !visited.insert(current) {
				continue;
			}
			for index in 0..self.tree.nodes.len() {
				let node = NodeId::new(index);
				let value = match self.view(node) {
					View::Use { value, .. }
						if self.analysis.symbols.symbol_of_decl(node) == Some(current) =>
					{
						value
					}
					View::Reuse { value, .. }
						if self.analysis.resolution(node) == Resolution::Local(current) =>
					{
						value
					}
					_ => continue,
				};
				let value = self.strip_reference(value);
				if !matches!(self.view(value), View::Name(_)) {
					continue;
				}
				let Resolution::Local(source) = self.analysis.resolution(value) else {
					continue;
				};
				let info = self.analysis.symbols.symbol(source);
				if matches!(info.kind, SymbolKind::Use | SymbolKind::MutableUse) {
					pending.push(source);
				} else if !self.is_object_reference(info.ty)
					&& !self.map_aliases.contains_key(&source)
				{
					match self.references.get(&source) {
						Some(Reference::Place(Parts {
							key: Key::Whole(local, None),
							..
						})) => locals.push(local.clone()),
						Some(_) => {}
						None => locals.push(self.name_of(source)),
					}
				}
			}
		}
		locals.sort();
		locals.dedup();
		locals
	}

	fn choice_values(&mut self, parts: &Parts, locals: &[String]) -> Option<(Expr, Expr, Expr)> {
		let absent = || Expr::primary("undefined");
		match &parts.key {
			Key::Whole(local, None) => {
				let position = locals.iter().position(|candidate| candidate == local)?;
				Some((
					absent(),
					absent(),
					Expr::primary((position + 1).to_string()),
				))
			}
			Key::Choice {
				selector,
				locals: source,
				key,
			} => {
				let mut selected = "0".to_string();
				for (index, local) in source.iter().enumerate().rev() {
					let position = locals.iter().position(|candidate| candidate == local)?;
					selected = format!(
						"{} === {} ? {} : {selected}",
						selector.text,
						index + 1,
						position + 1
					);
				}
				Some((
					parts.container.clone()?,
					key.clone(),
					Expr::new(selected, js::Prec::Conditional),
				))
			}
			_ => self
				.parts_pair(parts)
				.map(|(container, key)| (container, key, Expr::primary("0"))),
		}
	}

	fn bind_local_choice(&mut self, symbol: SymbolId, parts: Parts, locals: Vec<String>) -> bool {
		let Some((container, key, selector)) = self.choice_values(&parts, &locals) else {
			return false;
		};
		let name = self.declare(symbol);
		let names = [
			format!("{name}$container"),
			format!("{name}$key"),
			format!("{name}$choice"),
		];
		for (name, value) in names.iter().zip([container, key, selector]) {
			let keyword = if self.is_module_value(symbol) || self.guard_binding(name) {
				""
			} else {
				"let "
			};
			self.out.line(format!("{keyword}{name} = {};", value.text));
		}
		self.bind_reference_parts(
			symbol,
			Parts {
				container: Some(Expr::name(names[0].clone(), false)),
				key: Key::Choice {
					selector: Expr::name(names[2].clone(), false),
					locals,
					key: Expr::name(names[1].clone(), false),
				},
				optional: parts.optional,
			},
		);
		true
	}

	/// Snapshot the selected location before evaluating an assignment's value.
	pub(super) fn local_choice_place(&mut self, symbol: SymbolId, ty: TypeId) -> Option<Place> {
		let Reference::Place(parts) = self.references.get(&symbol)?.clone() else {
			return None;
		};
		let Key::Choice {
			selector,
			locals,
			key,
		} = parts.key
		else {
			return None;
		};
		let selector = self.keep_value_part(selector);
		let container = self.keep_value_part(parts.container?);
		let key = self.keep_value_part(key);
		let text = self
			.read_parts(&Parts {
				container: Some(container.clone()),
				key: Key::Choice {
					selector: selector.clone(),
					locals: locals.clone(),
					key: key.clone(),
				},
				optional: parts.optional,
			})
			.text;
		let helper = self.helper("faReferenceSet");
		Some(Place {
			text,
			ty,
			map: None,
			reference: None,
			choice: Some(LocalChoice {
				selector: selector.text,
				locals,
				helper: helper.to_string(),
				container: container.text,
				key: key.text,
			}),
		})
	}

	pub(crate) fn use_declaration(&mut self, node: NodeId, value: NodeId, mutable: bool) {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(node) else {
			return;
		};
		let ty = self.analysis.symbols.symbol(symbol).ty;
		let byte_slice = self.without_none(ty) == TypeId::BYTES
			&& matches!(self.view(self.strip_reference(value)), View::Index { index, .. } if self.type_of(index) == TypeId::RANGE);
		if self.is_object_reference(ty) || byte_slice {
			let value = match self.view(self.strip_reference(value)) {
				View::Index { target, index } => self
					.native_slice(self.strip_reference(value), target, index, true)
					.unwrap_or_else(|| self.expr(value)),
				_ => self.expr(value),
			};
			let name = self.declare(symbol);
			let keyword = if self.is_module_value(symbol) || self.guard_binding(&name) {
				""
			} else if mutable {
				"let "
			} else {
				"const "
			};
			self.out.line(format!("{keyword}{name} = {};", value.text));
			self.references.insert(symbol, Reference::Object);
			return;
		}
		let Some(mut parts) = self.reference_parts(value, true) else {
			return;
		};
		if mutable {
			let locals = self.scalar_reference_targets(symbol);
			if !locals.is_empty() && self.bind_local_choice(symbol, parts.clone(), locals) {
				return;
			}
		}
		if let Key::Value(value) = &parts.key {
			let name = self.declare(symbol);
			let keyword = if self.is_module_value(symbol) || self.guard_binding(&name) {
				""
			} else if mutable {
				"let "
			} else {
				"const "
			};
			self.out.line(format!("{keyword}{name} = {};", value.text));
			// A string slice is an immutable JS value, but a mutable use
			// still needs its local slot when `reuse` selects another slice.
			if mutable {
				self.references.insert(symbol, Reference::Object);
			}
			return;
		}
		// The location survives the statement's temporary-resource cleanup.
		if self
			.guard_frames
			.last()
			.is_some_and(|frame| !frame.is_empty())
			&& let Some((container, key)) = self.parts_pair(&parts)
		{
			let container_name = self.temp();
			let key_name = self.temp();
			self.guard_binding(&container_name);
			self.guard_binding(&key_name);
			self.out
				.line(format!("{container_name} = {};", container.text));
			self.out.line(format!("{key_name} = {};", key.text));
			parts.container = Some(Expr::name(container_name, true));
			parts.key = match parts.key {
				Key::Entry(_) => Key::Entry(Expr::name(key_name, true)),
				Key::Set(_) => Key::Set(Expr::name(key_name, true)),
				Key::Dynamic(_) => Key::Dynamic(Expr::name(key_name, true)),
				_ => Key::Index(Expr::name(key_name, true)),
			};
		}
		if (self.is_module_value(symbol) || mutable)
			&& let Some((container, key)) = self.parts_pair(&parts)
		{
			let name = self.declare(symbol);
			let (container_name, key_name) = (format!("{name}$container"), format!("{name}$key"));
			let keyword = if self.is_module_value(symbol) || {
				let guarded = self.guard_binding(&container_name);
				if guarded {
					self.guard_binding(&key_name);
				}
				guarded
			} {
				""
			} else {
				"let "
			};
			self.out
				.line(format!("{keyword}{container_name} = {};", container.text));
			self.out
				.line(format!("{keyword}{key_name} = {};", key.text));
			let access = if mutable {
				self.reseatable_access(symbol, value)
			} else {
				match parts.key {
					Key::Entry(_) => Access::Map,
					Key::Dynamic(_) => Access::Dynamic,
					_ => Access::Bracket,
				}
			};
			let key = match access {
				Access::Map => Key::Entry(Expr::name(key_name, false)),
				Access::Set => Key::Set(Expr::name(key_name, false)),
				Access::Dynamic => Key::Dynamic(Expr::name(key_name, false)),
				Access::Bracket => Key::Index(Expr::name(key_name, false)),
			};
			self.bind_reference_parts(
				symbol,
				Parts {
					container: Some(Expr::name(container_name, false)),
					key,
					optional: parts.optional,
				},
			);
			return;
		}
		self.bind_reference_parts(symbol, parts);
	}

	pub(crate) fn prepare_module_references(&mut self) {
		let mut symbols: Vec<_> = self.module_values.iter().copied().collect();
		symbols.sort();
		for symbol in symbols {
			if !matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Use | SymbolKind::MutableUse
			) {
				continue;
			}
			let ty = self.analysis.symbols.symbol(symbol).ty;
			if self.is_object_reference(ty) {
				self.references.insert(symbol, Reference::Object);
				continue;
			}
			let declaration = self.analysis.symbols.symbol(symbol).decl_node;
			let View::Use { value, .. } = self.view(declaration) else {
				continue;
			};
			if let View::Index { target, .. } = self.view(self.strip_reference(value))
				&& self.kind(self.type_of(target)) == Kind::Text
			{
				continue;
			}
			let locals = self.scalar_reference_targets(symbol);
			if self.analysis.symbols.symbol(symbol).kind == SymbolKind::MutableUse
				&& !locals.is_empty()
			{
				let name = self.declare(symbol);
				let container = format!("{name}$container");
				let key = format!("{name}$key");
				let selector = format!("{name}$choice");
				self.module_slots
					.extend([container.clone(), key.clone(), selector.clone()]);
				self.bind_reference_parts(
					symbol,
					Parts {
						container: Some(Expr::name(container, false)),
						key: Key::Choice {
							selector: Expr::name(selector, false),
							locals,
							key: Expr::name(key, false),
						},
						optional: self.may_be_none(ty),
					},
				);
				continue;
			}
			if let View::Name(_) = self.view(self.strip_reference(value))
				&& let Resolution::Local(source) =
					self.analysis.resolution(self.strip_reference(value))
				&& !matches!(
					self.analysis.symbols.symbol(source).kind,
					SymbolKind::Use | SymbolKind::MutableUse
				) {
				let text = self.name_of(source);
				self.bind_reference_parts(
					symbol,
					Parts {
						container: None,
						key: Key::Whole(text, None),
						optional: false,
					},
				);
				continue;
			}
			let name = self.declare(symbol);
			let (container, key) = (format!("{name}$container"), format!("{name}$key"));
			self.module_slots.extend([container.clone(), key.clone()]);
			self.bind_reference_parts(
				symbol,
				Parts {
					container: Some(Expr::name(container, false)),
					key: Key::Dynamic(Expr::name(key, false)),
					optional: self.may_be_none(ty),
				},
			);
		}
	}

	pub(crate) fn reuse_statement(&mut self, node: NodeId, value: NodeId) {
		let Resolution::Local(symbol) = self.analysis.resolution(node) else {
			self.unsupported(node, "this reuse");
			return;
		};
		match self.references.get(&symbol).cloned() {
			Some(Reference::Object) => {
				let value = match self.view(self.strip_reference(value)) {
					View::Index { target, index } => self
						.native_slice(self.strip_reference(value), target, index, true)
						.unwrap_or_else(|| self.expr(value)),
					_ => self.expr(value),
				};
				let name = self.declare(symbol);
				self.out.line(format!("{name} = {};", value.text));
			}
			Some(Reference::Place(Parts {
				container: Some(container),
				key: Key::Choice {
					selector,
					locals,
					key,
				},
				..
			})) => {
				let Some(parts) = self.reference_parts(value, true) else {
					return;
				};
				let Some((new_container, new_key, new_selector)) =
					self.choice_values(&parts, &locals)
				else {
					self.unsupported(node, "this reference selection");
					return;
				};
				let new_container = self.keep_value_part(new_container);
				let new_key = self.keep_value_part(new_key);
				let new_selector = self.keep_value_part(new_selector);
				self.out
					.line(format!("{} = {};", container.text, new_container.text));
				self.out.line(format!("{} = {};", key.text, new_key.text));
				self.out
					.line(format!("{} = {};", selector.text, new_selector.text));
			}
			Some(Reference::Place(parts)) => {
				let absent = matches!(self.view(self.strip_reference(value)), View::None);
				let pair = if absent {
					Some((Expr::primary("undefined"), Expr::primary("undefined")))
				} else {
					self.reference_pair(value)
				};
				let (Some(container), Some((new_container, new_key))) = (parts.container, pair)
				else {
					self.unsupported(node, "this reuse of a local scalar");
					return;
				};
				let key = match parts.key {
					Key::Index(key) | Key::Entry(key) | Key::Dynamic(key) | Key::Set(key) => key,
					_ => {
						self.unsupported(node, "this reuse");
						return;
					}
				};
				let new_container = self.keep_value_part(new_container);
				let new_key = self.keep_value_part(new_key);
				self.out
					.line(format!("{} = {};", container.text, new_container.text));
				self.out.line(format!("{} = {};", key.text, new_key.text));
			}
			None => {
				self.unsupported(node, "this reuse");
			}
		}
	}

	/// The container and the key of the primitive place a reference
	/// designates, evaluated once (`keep_names`: a local name stays itself,
	/// for a `use`; a `mutable use` keeps every part, since its locals are
	/// assigned).
	fn reference_parts(&mut self, value: NodeId, keep_names: bool) -> Option<Parts> {
		let node = self.strip_reference(value);
		let simple = |container, key| Parts {
			container,
			key,
			optional: false,
		};
		match self.view(node) {
			View::None => Some(Parts {
				container: Some(Expr::primary("undefined")),
				key: Key::Dynamic(Expr::primary("undefined")),
				optional: true,
			}),
			View::Name(_) => {
				if let Resolution::Local(source) = self.analysis.resolution(node) {
					if let Some(Reference::Place(mut parts)) = self.references.get(&source).cloned()
					{
						if let Some(container) = parts.container {
							parts.container = Some(self.keep_value_part(container));
						}
						parts.key = match parts.key {
							Key::Index(key) => Key::Index(self.keep_value_part(key)),
							Key::Entry(key) => Key::Entry(self.keep_value_part(key)),
							Key::Dynamic(key) => Key::Dynamic(self.keep_value_part(key)),
							Key::Set(key) => Key::Set(self.keep_value_part(key)),
							Key::Choice {
								selector,
								locals,
								key,
							} => Key::Choice {
								selector: self.keep_value_part(selector),
								locals,
								key: self.keep_value_part(key),
							},
							other => other,
						};
						return Some(parts);
					}

					let text = self.name_of(source);
					let map = self.map_aliases.get(&source).cloned();
					return Some(simple(None, Key::Whole(text, map)));
				}
				let place = self.place(node, true)?;
				Some(simple(None, Key::Whole(place.text, place.map)))
			}
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				let (&last, prefix) = members.split_last()?;
				let property = match self.analysis.resolution(last) {
					Resolution::Field(shape, index) => {
						let field = self.types.shape_fields(shape)[index as usize];
						Self::property(self.types.syms.resolve(field.name))
					}
					Resolution::Builtin(builtin) => builtins::get(builtin)
						.path
						.strip_prefix("Range#")?
						.to_string(),
					_ => {
						self.unsupported(node, "a reference to this member");
						return None;
					}
				};
				let container = self.access_chain(node, target, prefix);
				let path = prefix.iter().all(|&member| {
					matches!(self.analysis.resolution(member), Resolution::Field(..))
				}) && self.is_plain_local(target);
				let container = if path && keep_names {
					container
				} else {
					self.keep_value_part(container)
				};
				Some(simple(Some(container), Key::Field(property)))
			}
			View::Index { target, index } => {
				let target_ty = self.type_of(target);
				if self.type_of(index) == TypeId::RANGE || self.kind(target_ty) == Kind::Text {
					let value = self
						.native_slice(node, target, index, true)
						.unwrap_or_else(|| self.expr(node));
					return Some(simple(None, Key::Value(value)));
				}
				let values = self.in_order(&[target, index], |g, part| g.expr(part));
				let keep_container = !(keep_names && self.is_plain_local(target));
				let container = if keep_container {
					self.keep_value_part(values[0].clone())
				} else {
					values[0].clone()
				};
				let key = self.keep_value_part(values[1].clone());
				if let TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, key_ty) =
					self.types.kind(target_ty)
				{
					if self.has(node, NodeFlags::INDEX_CHECK) {
						// `KeyNotFound` now, when the key is absent
						let check = self.map_index(node, container.clone(), key.clone(), key_ty);
						self.out.line(format!("{};", check.text));
					}
					return Some(simple(Some(container), Key::Entry(key)));
				}
				let key = if self.index_needs_normalizing(node, index) {
					let helper = self.helper("faNormalizeIndex");
					let temp = self.temp();
					self.out.line(format!(
						"const {temp} = {helper}({}.length, {});",
						container.primary_text(),
						key.text
					));
					Expr::name(temp, true)
				} else {
					key
				};
				Some(simple(Some(container), Key::Index(key)))
			}
			View::Call { callee, .. } => self.call_reference_parts(node, callee),
			_ => {
				self.unsupported(node, "a reference to this value");
				None
			}
		}
	}

	/// The parts of a reference returned by `map.get(key)`,
	/// `map.getOrInsert(key, value)`, `array.get(index)` or
	/// `array.find(predicate)` (a primitive value: the place, `none` when
	/// absent).
	fn call_reference_parts(&mut self, node: NodeId, callee: NodeId) -> Option<Parts> {
		if self.call_returns_reference(node) {
			let View::Call { arguments, .. } = self.view(node) else {
				return None;
			};
			let layout = self.reference_call_layout(node);
			let result = self.call(node, callee, arguments);
			let name = self.temp();
			self.out.line(format!("const {name} = {};", result.text));
			let result = Expr::name(name, true);
			let optional = self.may_be_none(self.type_of(node));
			return match layout {
				ReferenceLayout::General => {
					let key_value = if let Some(key) = self.reference_call_keys.remove(&node) {
						key
					} else {
						let key = self.helper("faReferenceKey").to_string();
						let key_name = self.temp();
						self.out.line(format!("const {key_name} = {key};"));
						Expr::name(key_name, true)
					};
					Some(Parts {
						container: Some(result),
						key: Key::Dynamic(key_value),
						optional,
					})
				}
				ReferenceLayout::Index(index) => Some(Parts {
					container: Some(result),
					key: Key::Index(Expr::primary(index.to_string())),
					optional,
				}),
				ReferenceLayout::Entry { key, ty } => {
					let key = self.static_reference_key(node, key, ty);
					Some(Parts {
						container: Some(result),
						key: Key::Entry(key),
						optional,
					})
				}
				ReferenceLayout::Field(name) => Some(Parts {
					container: Some(result),
					key: Key::Field(Self::property(self.types.syms.resolve(name))),
					optional,
				}),
				ReferenceLayout::Owner => {
					let (owner, map) = self.reference_call_owner(node)?;
					let container = if optional {
						let name = self.temp();
						self.out.line(format!(
							"const {name} = {} === undefined ? undefined : {};",
							result.text, owner.text
						));
						Expr::name(name, true)
					} else {
						owner
					};
					Some(Parts {
						container: Some(container),
						key: match map {
							Some(true) => Key::Entry(result),
							Some(false) => Key::Index(result),
							None => Key::Dynamic(result),
						},
						optional,
					})
				}
			};
		}
		let path = match self.analysis.call(node).map(|info| info.callee) {
			Some(Callee::Builtin(builtin)) => builtins::get(builtin).path,
			_ => "",
		};
		let arguments: Vec<NodeId> = self
			.analysis
			.call(node)
			.map(|info| {
				info.arguments
					.iter()
					.filter_map(|argument| match argument {
						Argument::Given(value) => Some(*value),
						_ => None,
					})
					.collect()
			})
			.unwrap_or_default();
		if path == "Set#find" {
			let receiver = self.receiver(callee);
			let receiver = self.keep_value_part(receiver);
			let predicate = self.expr(*arguments.first()?);
			let helper = self.helper("faFind");
			let key = self.temp();
			self.out.line(format!(
				"const {key} = {helper}({}, {});",
				receiver.text, predicate.text
			));
			let container = self.temp();
			self.out.line(format!(
				"const {container} = {key} === undefined ? undefined : {};",
				receiver.text
			));
			return Some(Parts {
				container: Some(Expr::name(container, true)),
				key: Key::Set(Expr::name(key, true)),
				optional: true,
			});
		}

		let value_pool = matches!(
			path,
			"Pool#find" | "Pool#add" | "Columnar#find" | "Columnar#add"
		) && self.is_value_pool(self.receiver_type(callee));
		if !value_pool
			&& !matches!(
				path,
				"Map#get"
					| "Map#getOrInsert"
					| "Array#get" | "Array#find"
					| "OrderedColumnar#get"
					| "OrderedColumnar#find"
					| "OrderedColumnar#add"
			) {
			self.unsupported(node, "a reference to a primitive value returned by a call");
			return None;
		}
		let receiver = self.receiver(callee);
		let receiver = self.keep_value_part(receiver);
		let values = self.in_order(&arguments, |g, argument| g.expr(argument));
		// an element of a pool of values: the pool and its slot
		if matches!(path, "Pool#find" | "Columnar#find") {
			let (slot, container) = (self.temp(), self.temp());
			let pool = receiver.primary_text();
			self.out.line(format!(
				"const {slot} = {pool}.slotOf({});",
				values.first()?.text
			));
			self.out.line(format!(
				"const {container} = {slot} !== undefined ? {pool}.$slots : undefined;"
			));
			return Some(Parts {
				container: Some(Expr::name(container, true)),
				key: Key::Index(Expr::name(slot, true)),
				optional: true,
			});
		}
		if matches!(path, "Pool#add" | "Columnar#add") {
			let slot = self.temp();
			self.out.line(format!(
				"const {slot} = {}.add({});",
				receiver.primary_text(),
				values.first()?.text
			));
			return Some(Parts {
				container: Some(Expr::primary(format!("{}.$slots", receiver.primary_text()))),
				key: Key::Index(Expr::name(slot, true)),
				optional: false,
			});
		}
		if path == "OrderedColumnar#add" {
			let slot = self.temp();
			self.out.line(format!(
				"const {slot} = {}.length;",
				receiver.primary_text()
			));
			let item = arguments.first().copied()?;
			let value = self.copy_stored(item, values.first()?.clone());
			self.out
				.line(format!("{}.push({});", receiver.primary_text(), value.text));
			return Some(Parts {
				container: Some(receiver),
				key: Key::Index(Expr::name(slot, true)),
				optional: false,
			});
		}

		if matches!(path, "Array#find" | "OrderedColumnar#find") {
			// the item the predicate finds first: its index, `none` without one
			let (position, container) = (self.temp(), self.temp());
			let array = receiver.primary_text();
			self.out.line(format!(
				"const {position} = {array}.findIndex({});",
				values.first()?.text
			));
			self.out.line(format!(
				"const {container} = {position} >= 0 ? {array} : undefined;"
			));
			return Some(Parts {
				container: Some(Expr::name(container, true)),
				key: Key::Index(Expr::name(position, true)),
				optional: true,
			});
		}
		let key = self.keep_value_part(values.first()?.clone());
		match path {
			"Map#getOrInsert" => {
				let helper = self.helper("faMapGetOrInsert");
				self.out.line(format!(
					"{helper}({}, {}, {});",
					receiver.text,
					key.text,
					values.get(1)?.text
				));
				Some(Parts {
					container: Some(receiver),
					key: Key::Entry(key),
					optional: false,
				})
			}
			"Map#get" => {
				let temp = self.temp();
				self.out.line(format!(
					"const {temp} = {0}.has({1}) ? {0} : undefined;",
					receiver.primary_text(),
					key.text
				));
				Some(Parts {
					container: Some(Expr::name(temp, true)),
					key: Key::Entry(key),
					optional: true,
				})
			}
			_ => {
				let (position, container) = (self.temp(), self.temp());
				let array = receiver.primary_text();
				self.out.line(format!(
					"const {position} = {0} < 0 ? {0} + {array}.length : {0};",
					key.text
				));
				self.out.line(format!(
					"const {container} = {position} >= 0 && {position} < {array}.length ? {array} : undefined;"
				));
				Some(Parts {
					container: Some(Expr::name(container, true)),
					key: Key::Index(Expr::name(position, true)),
					optional: true,
				})
			}
		}
	}

	pub(crate) fn reference_slot(&self, node: NodeId) -> Option<String> {
		let Resolution::Local(symbol) = self.analysis.resolution(self.strip_reference(node)) else {
			return None;
		};
		if let Some(Reference::Place(Parts {
			key: Key::Index(key) | Key::Entry(key) | Key::Dynamic(key),
			..
		})) = self.references.get(&symbol)
		{
			return Some(key.text.clone());
		}
		self.map_aliases.get(&symbol).map(|(_, key)| key.clone())
	}

	/// The node of a place without its parentheses and `try`.
	fn strip_reference(&self, n: NodeId) -> NodeId {
		let mut current = n;
		while let View::Group(inner) | View::Try(inner) = self.view(current) {
			current = inner;
		}
		current
	}

	/// Whether a node is a local name read as is (re-reading it reads the
	/// same Fa place).
	fn is_plain_local(&self, n: NodeId) -> bool {
		let n = self.strip_groups(n);
		matches!(self.view(n), View::Name(_))
			&& match self.analysis.resolution(n) {
				Resolution::Local(symbol) => {
					!self.aliases.contains_key(&symbol)
						&& self.analysis.symbols.symbol(symbol).kind != SymbolKind::MutableUse
				}
				_ => false,
			}
	}

	/// A part of a place evaluated once: stable, or kept in a temporary.
	pub(super) fn keep_value_part(&mut self, value: Expr) -> Expr {
		if value.stable {
			return value;
		}
		let temp = self.temp();
		self.out.line(format!("const {temp} = {};", value.text));
		Expr::name(temp, true)
	}

	// ── in-place writes (D60) ────────────────────────────────────────────

	/// Whether a whole write into a place must keep the object it holds:
	/// a reference to an object or a collection, or a place rooted at a
	/// value a reference designates (not a value of a map, which `set`
	/// replaces, nor a union or an optional place).
	pub(crate) fn writes_in_place(&mut self, target: NodeId) -> bool {
		if let View::Name(_) = self.view(target)
			&& let Resolution::Local(symbol) = self.analysis.resolution(target)
			&& matches!(self.references.get(&symbol), Some(Reference::Object))
		{
			return true;
		}
		let storage_ty = match self.view(target) {
			View::Name(_) => match self.analysis.resolution(target) {
				Resolution::Local(symbol)
					if !matches!(self.references.get(&symbol), Some(Reference::Object)) =>
				{
					self.analysis.symbols.symbol(symbol).ty
				}
				_ => self.type_of(target),
			},
			View::Access { members, .. } => {
				match members.last().map(|node| self.analysis.resolution(node)) {
					Some(Resolution::Field(shape, index)) => {
						self.types.shape_fields(shape)[index as usize].ty
					}
					_ => self.type_of(target),
				}
			}
			_ => self.type_of(target),
		};
		let ty = self.type_of(target);
		if self.may_be_none(storage_ty) || !self.is_object_reference(ty) {
			return false;
		}
		match self.view(target) {
			View::Name(_) => match self.analysis.resolution(target) {
				Resolution::Local(symbol) => {
					matches!(self.references.get(&symbol), Some(Reference::Object))
						|| self.referenced_roots().contains(&symbol)
				}
				_ => false,
			},
			View::Index { target: map, .. }
				if matches!(
					self.types.kind(self.type_of(map)),
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
				) =>
			{
				false
			}
			View::Access { .. } | View::Index { .. } => {
				self.place_root(target).is_some_and(|root| {
					matches!(self.references.get(&root), Some(Reference::Object))
						|| self.referenced_roots().contains(&root)
						|| matches!(
							self.analysis.symbols.symbol(root).kind,
							SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
						) || self.is_module_value(root)
				})
			}
			_ => false,
		}
	}

	pub(crate) fn map_value_may_be_borrowed(&mut self, target: NodeId) -> bool {
		self.is_object_reference(self.type_of(target))
			&& self.place_root(target).is_some_and(|root| {
				self.referenced_roots().contains(&root)
					|| matches!(
						self.analysis.symbols.symbol(root).kind,
						SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
					) || self.is_module_value(root)
			})
	}

	/// `place = value` in place: `faAssign(place, value)` (the value is
	/// copied by `faAssign` where it keeps parts of it).
	pub(crate) fn assign_in_place(&mut self, target: NodeId, value: NodeId) {
		let lifted = self.needs_statements(value);
		let Some(place) = self.place(target, !lifted) else {
			self.unsupported(target, "this assignment target");
			return;
		};
		// Resource owners keep their referenced outer identity, but a whole
		// write destroys the previous contents and transfers the new ones.
		let resource_array = matches!(
			self.types.kind(self.type_of(target)),
			TypeKind::Collection(
				CollKind::Array | CollKind::InlineArray | CollKind::GrowingArray,
				..
			)
		);
		let resource_drop = if resource_array
			|| matches!(self.types.kind(self.type_of(target)), TypeKind::Object(_))
		{
			self.drop_function(self.type_of(target))
		} else {
			None
		};
		let resource_target = resource_drop.as_ref().map(|_| {
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", place.text));
			temp
		});
		let value = if lifted {
			let temp = self.temp();
			self.out.line(format!("let {temp};"));
			self.into(value, Dest::Assign(temp.clone()));
			Expr::name(temp, true)
		} else {
			self.expr(value)
		};
		if let (Some(drop), Some(target)) = (resource_drop, resource_target) {
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", value.text));
			self.out.line(format!("{drop}({target});"));
			if resource_array {
				self.out.line(format!("{target}.length = 0;"));
				self.out
					.line(format!("for (const item of {temp}) {target}.push(item);"));
			} else {
				self.out.line(format!("Object.assign({target}, {temp});"));
			}
			return;
		}
		let helper = self.helper("faAssign");
		self.out
			.line(format!("{helper}({}, {});", place.text, value.text));
	}

	/// The roots of the places designated by the references of the file
	/// (`use`, `mutable use`, `reuse`, `provide`, `for` variables over
	/// collections of objects).
	fn referenced_roots(&mut self) -> &HashSet<SymbolId> {
		if self.referenced_roots.is_none() {
			let mut roots = HashSet::new();
			for index in 0..self.tree.nodes.len() {
				let node = NodeId::new(index);
				let value = match self.view(node) {
					View::Use { value, .. } | View::Reuse { value, .. } => value,
					View::Provide { value, .. } => value,
					_ => continue,
				};
				let value = self.strip_reference(value);
				let root = match self.view(value) {
					View::Call { callee, .. } => match self.view(callee) {
						View::Access { target, .. } => self.place_root(target),
						_ => None,
					},
					_ => self.place_root(value),
				};
				roots.extend(root);
			}
			self.referenced_roots = Some(roots);
		}
		self.referenced_roots.as_ref().expect("computed")
	}

	// ── requirements ─────────────────────────────────────────────────────

	/// `provide Key = value`: a temporary evaluated once, or nothing for a
	/// primitive place (each call reads it).
	pub(crate) fn provide_statement(&mut self, n: NodeId, ty: NodeId, value: NodeId) {
		let place = matches!(
			self.view(self.strip_groups(value)),
			View::Name(_) | View::Access { .. } | View::Index { .. }
		);
		if place && !self.is_composite(self.type_of(value)) {
			self.provided.insert(n, None);
			return;
		}
		let value = self.expr(value);
		let type_name = self.text(ty);
		let mut wanted: String = type_name
			.chars()
			.take_while(|c| c.is_alphanumeric())
			.collect();
		if let Some(first) = wanted.get(0..1) {
			wanted = first.to_lowercase() + &wanted[1..];
		}
		let name = self.scopes.declare(&format!("{wanted}$p"));
		self.out.line(format!("const {name} = {};", value.text));
		self.provided.insert(n, Some(name));
	}

	/// The hidden arguments of a call for the callee's requirements.
	pub(crate) fn requirement_arguments(&mut self, call: NodeId) -> Vec<String> {
		let Some(arguments) = self.analysis.requirement_args.get(&call).cloned() else {
			return Vec::new();
		};
		arguments
			.into_iter()
			.map(|argument| match argument {
				RequirementArg::Provided(provide) => match self.provided.get(&provide).cloned() {
					Some(Some(name)) => name,
					_ => match self.view(provide) {
						View::Provide { value, .. } => self.expr(value).text,
						_ => "undefined".to_string(),
					},
				},
				RequirementArg::Own(key) => self
					.requirement_scopes
					.iter()
					.rev()
					.flatten()
					.find(|(own, _)| *own == key)
					.map_or_else(|| "undefined".to_string(), |(_, name)| name.clone()),
				RequirementArg::Missing => "undefined".to_string(),
			})
			.collect()
	}

	/// Declares the hidden parameters of a function's requirements (after
	/// its regular parameters): their names, in the order of
	/// `FnSig::requires`. The requirement symbols of the body take them.
	pub(crate) fn declare_requirements(&mut self, info: &FnInfo) -> Vec<(TypeId, String)> {
		let Some(sig) = info.sig else {
			return Vec::new();
		};
		let requires = self.types.sig(sig).requires;
		let requirements = self.types.requirements(requires).to_vec();
		let mut declared = Vec::new();
		for (index, requirement) in requirements.iter().enumerate() {
			let symbol = self
				.analysis
				.symbols
				.symbols()
				.iter()
				.position(|symbol| {
					symbol.scope == info.scope
						&& symbol.kind
							== SymbolKind::Requirement {
								index: index as u16,
							}
				})
				.map(SymbolId::new);
			let name = match symbol {
				Some(symbol) => self.declare(symbol),
				None => {
					let wanted = self.types.syms.resolve(requirement.name).to_string();
					self.scopes.declare(&wanted)
				}
			};
			declared.push((requirement.ty, name));
		}
		declared
	}

	// ── escaping closures ────────────────────────────────────────────────

	/// Before an escaping closure: copies of the captures that may change
	/// after its creation, which the closure reads instead. Returns the
	/// aliases to restore after the closure is generated.
	pub(crate) fn copy_captures(&mut self, info: &FnInfo) -> Vec<(SymbolId, Option<String>)> {
		if !self.analysis.escapes.contains(&info.lambda) {
			return Vec::new();
		}
		let captures = info.captures.clone();
		self.capture_values(&captures, false, Some(info.lambda))
	}

	/// Copies of the local values `symbols` that may change later, which a
	/// closure being generated reads instead (see
	/// [`Generator::copy_captures`]). A `reactive` value is always copied,
	/// even a module-level one: it is captured by the closures that don't
	/// track it (the cleanup of a `watch` block). Returns the aliases to
	/// restore.
	pub(crate) fn copy_captured(
		&mut self,
		symbols: &[SymbolId],
	) -> Vec<(SymbolId, Option<String>)> {
		self.copy_values(symbols, false)
	}

	/// [`Generator::copy_captured`]; `module_values`: the module-level ones
	/// too (a detached `start do` copies the top-level values it uses, on
	/// every target: `design/decisions.md`).
	pub(crate) fn copy_values(
		&mut self,
		symbols: &[SymbolId],
		module_values: bool,
	) -> Vec<(SymbolId, Option<String>)> {
		self.capture_values(symbols, module_values, None)
	}

	fn capture_values(
		&mut self,
		symbols: &[SymbolId],
		module_values: bool,
		owner: Option<NodeId>,
	) -> Vec<(SymbolId, Option<String>)> {
		let mut saved = Vec::new();
		for &symbol in symbols {
			let kind = self.analysis.symbols.symbol(symbol).kind;
			if kind == SymbolKind::Derived {
				continue;
			}
			let ty = self.analysis.symbols.symbol(symbol).ty;
			// An immutable parameter (or a let sharing it) can designate a
			// caller's mutable object. An owned ordinary capture is a value
			// snapshot regardless of the binding's own mutability.
			let ordinary_composite = self.is_composite(ty)
				&& self.types.ownership(ty) == fa_analyzer::types::Ownership::Ordinary;
			let moved =
				owner.is_some_and(|owner| self.analysis.capture_moves.contains(&(owner, symbol)));
			let changes = moved
				|| self.is_mutable_place(symbol)
				|| self.aliases.contains_key(&symbol)
				|| kind == SymbolKind::Use
				|| ordinary_composite;
			let reactive = kind == SymbolKind::Reactive;
			if !changes || (self.is_module_value(symbol) && !reactive && !module_values) {
				continue;
			}
			let current = Expr::name(self.name_of(symbol), true);
			let value = if !moved && self.is_composite(ty) {
				self.copy_value(current, ty)
			} else {
				current
			};
			let wanted = self
				.types
				.syms
				.resolve(self.analysis.symbols.symbol(symbol).name)
				.to_string();
			let name = self.scopes.declare(&format!("{wanted}$copy"));
			// a task may change its copy
			let keyword = if module_values { "let" } else { "const" };
			self.out.line(format!("{keyword} {name} = {};", value.text));
			saved.push((symbol, self.aliases.insert(symbol, name)));
		}
		saved
	}

	/// Restores the aliases [`Generator::copy_captures`] replaced.
	pub(crate) fn restore_captures(&mut self, saved: Vec<(SymbolId, Option<String>)>) {
		for (symbol, alias) in saved {
			match alias {
				Some(alias) => self.aliases.insert(symbol, alias),
				None => self.aliases.remove(&symbol),
			};
		}
	}

	/// Whether the read of a local at a node is its last use, as a copy
	/// (`NodeFlags::MOVE`): the value is moved, not copied (D15).
	pub(crate) fn is_move(&self, node: NodeId) -> bool {
		fa_analyzer::lower::copies::is_move(self.tree, self.analysis, node)
	}
}
