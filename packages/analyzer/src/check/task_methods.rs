//! Conditional blocking requirements of methods selected by generic calls.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{AsyncMethod, AsyncSummary, GenericMethod};
use crate::check::Walker;
use crate::ids::{DeclKey, ModuleId, NodeId, ParamId, TypeId};
use crate::types::TypeKind;

#[derive(Clone, Debug)]
pub(super) enum AsyncCallee {
	Local(NodeId),
	External(AsyncSummary),
}

#[derive(Clone, Debug, Default)]
struct MethodBindings {
	parameters: Vec<(DeclKey, u16)>,
	values: Vec<TypeId>,
	methods: Vec<GenericMethod>,
}

#[derive(Clone, Debug)]
pub(super) struct AsyncCall {
	pub target: AsyncCallee,
	bindings: MethodBindings,
}

impl Walker<'_> {
	/// Includes the generic owner before the function's own parameters, the
	/// same order as the backend's method dictionary arguments.
	pub(super) fn async_parameters(&self, module: ModuleId, node: NodeId) -> Vec<ParamId> {
		if module == self.s.module_id(self.file) {
			let Some(info) = self.a.function(node) else {
				return Vec::new();
			};
			let declaration = info.declaration.unwrap_or(node);
			let mut parameters = self
				.a
				.function_owners
				.get(&declaration)
				.and_then(|owner| self.a.type_decl(*owner))
				.map(|owner| owner.generics.clone())
				.unwrap_or_default();
			parameters.extend(&info.generics);
			return parameters;
		}
		let Some(interface) = self.s.interface(module) else {
			return Vec::new();
		};
		let owner = interface.function_owners.get(&node).and_then(|&key| {
			let module = self.s.types.decl_key_data(key).module;
			let interface = self.s.interface(module)?;
			interface
				.type_parameters
				.get(&interface.node_of(key)?)
				.copied()
		});
		let own = interface
			.generic_functions
			.get(&node)
			.map(|function| function.generics);
		owner
			.into_iter()
			.chain(own)
			.flat_map(|list| self.s.types.list_items(list))
			.filter_map(|&ty| match self.s.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect()
	}

	fn method_bindings(
		&self,
		parameters: &[ParamId],
		values: Vec<TypeId>,
		methods: Vec<GenericMethod>,
	) -> MethodBindings {
		MethodBindings {
			parameters: parameters
				.iter()
				.map(|&param| {
					let param = self.s.types.param(param);
					(param.owner, param.index)
				})
				.collect(),
			values,
			methods,
		}
	}

	pub(super) fn async_call(
		&self,
		call: NodeId,
		target: AsyncCallee,
		parameters: &[ParamId],
	) -> AsyncCall {
		let (values, methods) =
			self.a
				.generic_call_bindings(&self.tree, &self.s.types, call, parameters.len());
		AsyncCall {
			target,
			bindings: self.method_bindings(parameters, values, methods),
		}
	}

	fn method_summary(
		&self,
		module: ModuleId,
		node: NodeId,
		settled: &FxHashMap<NodeId, AsyncSummary>,
	) -> Option<AsyncSummary> {
		if module == self.s.module_id(self.file) {
			let lambda = self.a.function(node).map_or(node, |info| info.lambda);
			settled
				.get(&lambda)
				.or_else(|| self.a.async_summaries.get(&node))
				.cloned()
		} else {
			self.s
				.interface(module)?
				.async_summaries
				.get(&node)
				.cloned()
		}
	}

	/// Resolve a call's conditional requirements against its written method
	/// selections. An explicit work list and visited selections handle cycles.
	pub(super) fn async_instance(
		&self,
		call: &AsyncCall,
		settled: &FxHashMap<NodeId, AsyncSummary>,
	) -> AsyncSummary {
		let mut result = match &call.target {
			AsyncCallee::Local(key) => settled.get(key).cloned().unwrap_or_default(),
			AsyncCallee::External(summary) => summary.clone(),
		};
		let mut pending: Vec<_> = std::mem::take(&mut result.methods)
			.into_iter()
			.map(|method| (method, call.bindings.clone()))
			.collect();
		let mut visited = FxHashSet::default();
		while let Some((required, bindings)) = pending.pop() {
			let position = bindings
				.parameters
				.iter()
				.position(|&parameter| parameter == (required.owner, required.parameter));
			let selected = bindings.methods.iter().find(|method| {
				Some(method.parameter as usize) == position && method.name == required.name
			});
			if let Some(method) = selected {
				if !visited.insert(method.stable_hash(&self.s.types)) {
					continue;
				}
				if let Some((module, keyed)) = self.s.keyed(method.function)
					&& let Some(mut summary) = self.method_summary(module, keyed.node, settled)
				{
					let methods = std::mem::take(&mut summary.methods);
					result.merge_blocking(&summary, self.s.types.syms.resolve(method.name));
					let parameters = self.async_parameters(module, keyed.node);
					let bindings = self.method_bindings(
						&parameters,
						self.s.types.list_items(method.arguments).to_vec(),
						method.methods.as_ref().clone(),
					);
					pending.extend(methods.into_iter().map(|method| (method, bindings.clone())));
					continue;
				}
			}
			let parameter = position
				.and_then(|index| bindings.values.get(index))
				.and_then(|&ty| match self.s.types.kind(ty) {
					TypeKind::Param(param) => {
						Some(AsyncMethod::new(&self.s.types, param, required.name))
					}
					_ => None,
				})
				.unwrap_or(required);
			if !result.methods.contains(&parameter) {
				result.methods.push(parameter);
			}
		}
		result.methods.sort_unstable();
		result
	}
}
