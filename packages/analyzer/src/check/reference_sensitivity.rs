//! Representation dependencies of generic JavaScript reference bodies.
//! Published without runtime flags, so independently generated units agree.
use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::Callee;
use crate::check::Walker;
use crate::ids::{NodeId, ParamId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;
use crate::types::{CollKind, TypeKind};

impl Walker<'_> {
	pub(super) fn record_reference_sensitivity(&mut self) {
		let functions: Vec<_> = self
			.a
			.functions
			.iter()
			.filter_map(|(&node, info)| {
				(node == info.declaration.unwrap_or(info.lambda)
					&& (!info.generics.is_empty()
						|| self
							.a
							.function_owners
							.get(&node)
							.and_then(|owner| self.a.type_decls.get(owner))
							.is_some_and(|owner| !owner.generics.is_empty())))
				.then_some((node, info.clone()))
			})
			.collect();
		let mut needed: FxHashMap<NodeId, FxHashSet<ParamId>> = FxHashMap::default();
		let mut nodes = FxHashMap::default();
		for (node, info) in &functions {
			let mut roots = Vec::new();
			let mut signatures = vec![info.ty];
			let mut seen = FxHashSet::default();
			while let Some(ty) = signatures.pop() {
				if !seen.insert(ty) {
					continue;
				}
				if let TypeKind::Function(sig) = self.s.types.kind(ty) {
					let sig = self.s.types.sig(sig);
					if sig.reference.is_some() {
						roots.push(sig.ret);
					}
					for param in self.s.types.params(sig.params) {
						if param.reference.is_some() {
							roots.push(param.ty);
						}
						signatures.push(param.ty);
					}
					signatures.push(sig.ret);
				}
			}
			let mut pending = vec![info.body];
			let mut body = Vec::new();
			while let Some(value) = pending.pop() {
				body.push(value);
				match self.view(value) {
					View::Use { value, .. } | View::Reuse { value, .. } => {
						roots.push(self.type_of(value))
					}
					View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
						if let Resolution::Local(symbol) = self.a.resolution(target)
							&& self.is_alias(symbol)
						{
							roots.push(self.type_of(target));
						}
					}
					View::Array { .. } => {
						if let TypeKind::Collection(
							CollKind::Pool
							| CollKind::GrowingPool
							| CollKind::Columnar
							| CollKind::GrowingColumnar,
							item,
							_,
						) = self.s.types.kind(self.type_of(value))
						{
							roots.push(item);
						}
					}
					View::Call { callee, .. }
						if self
							.a
							.calls
							.get(&value)
							.is_some_and(|call| matches!(call.callee, Callee::Builtin(_))) =>
					{
						if let View::Access { target, .. } = self.view(callee)
							&& let TypeKind::Collection(
								CollKind::Pool
								| CollKind::GrowingPool
								| CollKind::Columnar
								| CollKind::GrowingColumnar,
								item,
								_,
							) = self.s.types.kind(self.type_of(target))
						{
							roots.push(item);
						}
					}

					_ => {}
				}
				pending.extend(crate::syntax::children(&self.tree, value));
			}
			let mut params = FxHashSet::default();
			for root in roots {
				self.representation_parameters(root, &mut params);
			}
			needed.insert(*node, params);
			nodes.insert(*node, body);
		}
		loop {
			let previous = needed.clone();
			for (function, _) in &functions {
				for &node in &nodes[function] {
					let Some(arguments) = self.a.instantiations.get(&node) else {
						continue;
					};
					let value = match self.view(node) {
						View::Call { callee, .. } => callee,
						_ => node,
					};
					let value = match self.view(self.strip_groups(value)) {
						View::Access { members, .. } => members.last().unwrap_or(value),
						_ => value,
					};
					let sensitive = match self.a.resolution(value) {
						Resolution::Local(symbol)
							if self.a.symbols.symbol(symbol).kind == SymbolKind::Function =>
						{
							previous
								.get(&self.a.symbols.symbol(symbol).decl_node)
								.is_some_and(|params| !params.is_empty())
						}
						Resolution::Local(symbol)
							if matches!(
								self.a.symbols.symbol(symbol).kind,
								SymbolKind::Parameter { .. }
							) =>
						{
							true
						}
						Resolution::External(key) => self
							.s
							.interface(self.s.types.decl_key_data(key).module)
							.and_then(|interface| {
								interface
									.node_of(key)
									.and_then(|node| interface.reference_sensitive.get(&node))
							})
							.is_some_and(|params| !params.is_empty()),
						_ => false,
					};
					if sensitive {
						for &argument in self.s.types.list_items(*arguments) {
							self.representation_parameters(
								argument,
								needed.entry(*function).or_default(),
							);
						}
						if let Some(owner) = self.a.method_instantiations.get(&value) {
							for &argument in self.s.types.list_items(owner.arguments) {
								self.representation_parameters(
									argument,
									needed.entry(*function).or_default(),
								);
							}
						}
					}
				}
			}
			if needed == previous {
				break;
			}
		}
		for (node, info) in functions {
			let mut generics = self
				.a
				.function_owners
				.get(&node)
				.and_then(|owner| self.a.type_decls.get(owner))
				.map(|info| info.generics.clone())
				.unwrap_or_default();
			generics.extend(info.generics.iter().copied());
			generics.retain(|param| {
				needed
					.get(&node)
					.is_some_and(|params| params.contains(param))
			});
			if !generics.is_empty() {
				self.a.reference_sensitive.insert(node, generics.clone());
				self.a.reference_sensitive.insert(info.lambda, generics);
			}
		}
	}

	fn representation_parameters(&self, ty: TypeId, params: &mut FxHashSet<ParamId>) {
		match self.s.types.kind(ty) {
			TypeKind::Param(param) if !self.s.types.param(param).value => {
				params.insert(param);
			}
			TypeKind::Union(list) => {
				for &item in self.s.types.list_items(list) {
					self.representation_parameters(item, params);
				}
			}
			_ => {}
		}
	}
}
