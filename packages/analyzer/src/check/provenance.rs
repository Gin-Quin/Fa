//! Sparse value summaries: bodies are checked once; higher-order applications
//! substitute these portable expressions instead of revisiting their syntax.

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{Argument, Callee, FieldValue};
use crate::check::Walker;
use crate::ids::{DeclKey, NodeId, SymbolId, TypeId};
use crate::provenance::{
	CallableCapabilities, CallableSummary, Invocation, OriginKind, Projection, ValueOrigin,
};
use crate::symbols::Resolution;
use crate::syntax::View;
use crate::types::TypeKind;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Affinity {
	Safe,
	Deferred,
	Guard,
	Executor,
}

struct Builder<'a, 's> {
	w: &'a mut Walker<'s>,
	parameters: FxHashMap<SymbolId, (DeclKey, u16)>,
	active: FxHashSet<NodeId>,
}

impl Walker<'_> {
	pub(super) fn callable_materialization(&self, mut node: NodeId) -> bool {
		while let Some(parent) = self.parent(node) {
			match self.view(parent) {
				View::Access { members, .. } | View::OptionalAccess { members, .. }
					if members.last() == Some(node) =>
				{
					node = parent
				}
				View::Group(inner) if inner == node => node = parent,
				View::Call { callee, .. } if callee == node => {
					if self.a.calls.contains_key(&parent) {
						return false;
					}
					node = parent;
				}
				_ => break,
			}
		}
		true
	}
	/// A value is materialized here, rather than merely appearing in a
	/// method signature. The sparse finish can inspect its actual captures.
	pub(super) fn defer_thread_value(&mut self, at: NodeId, value: NodeId, what: &str) {
		if self.quiet == 0
			&& !self
				.thread_values
				.iter()
				.any(|(known, source, _)| *known == at && *source == value)
		{
			self.thread_values.push((at, value, what.to_string()));
		}
	}

	pub(super) fn finish_thread_value_requirements(&mut self) {
		let mut transfers: Vec<_> = self
			.thread_values
			.iter()
			.filter_map(|(at, node, what)| {
				Some((*at, self.a.value_origins.get(node)?.clone(), what.clone()))
			})
			.collect();
		for (at, node, what) in &self.thread_result_values {
			let Some(callee) = self.a.value_origins.get(node).cloned() else {
				continue;
			};
			let Some(signature) = self.signature_of(callee.ty) else {
				continue;
			};
			let arguments = self
				.s
				.types
				.params(signature.params)
				.iter()
				.map(|param| ValueOrigin::opaque(param.ty))
				.collect();
			let value = Arc::new(ValueOrigin {
				ty: signature.ret,
				kind: OriginKind::Call(Invocation {
					callee,
					arguments,
					bindings: Vec::new(),
					capabilities: Default::default(),
				}),
			});
			transfers.push((*at, value, what.clone()));
		}
		for (site, value, what) in transfers {
			match self.value_affinity(Arc::clone(&value), &mut FxHashSet::default()) {
				Affinity::Safe => {}
				status @ (Affinity::Executor | Affinity::Guard) => {
					self.report_thread_value(site, &what, status)
				}
				Affinity::Deferred => {
					if let Some(owner) = self.value_owner(site) {
						if let Some(summary) = self.a.callables.get_mut(&owner)
							&& !summary.thread_transfers.contains(&value)
						{
							summary.thread_transfers.push(value);
						}
					} else {
						self.report_thread_value(site, &what, Affinity::Deferred);
					}
				}
			}
		}
		let mut calls: Vec<_> = self
			.a
			.calls
			.keys()
			.filter_map(|site| {
				let value = self.a.value_origins.get(site)?;
				let OriginKind::Call(call) = &value.kind else {
					return None;
				};
				Some((*site, call.clone()))
			})
			.collect();
		calls.sort_unstable_by_key(|(site, _)| *site);
		for (site, call) in calls {
			self.check_callable_thread_application(site, &call, &mut FxHashSet::default());
		}
		self.s.source_file_mut(self.file).interface.callables = self.a.callables.clone();
	}

	/// The callback executes on the destination executor, then its actual
	/// returned value crosses back. Its signature alone can hide captures.
	pub(super) fn defer_thread_result(&mut self, at: NodeId, callable: NodeId, what: &str) {
		if self.quiet == 0
			&& !self
				.thread_result_values
				.iter()
				.any(|(known, value, _)| *known == at && *value == callable)
		{
			self.thread_result_values
				.push((at, callable, what.to_string()));
		}
	}

	fn value_owner(&self, mut node: NodeId) -> Option<DeclKey> {
		loop {
			if let Some(&key) = self.a.callable_keys.get(&node)
				&& !self.a.partial_applications.contains_key(&node)
			{
				return Some(key);
			}
			node = self.parent(node)?;
		}
	}

	fn report_thread_value(&mut self, site: NodeId, what: &str, affinity: Affinity) {
		let (key, message) = if affinity == Affinity::Guard {
			(
				"executor-guard-transfer",
				format!(
					"{what} captures a mutex guard: a guard can't cross threads, including inside a closure or shared handle; transfer the `Mutex` and acquire a new guard on the destination executor"
				),
			)
		} else {
			(
				"executor-task-transfer",
				format!(
					"{what} captures executor-bound state: a `Task` can't cross threads, including inside a closure or a shared handle"
				),
			)
		};
		if self.reported_once.insert((site, key)) {
			let diagnostic =
				self.diagnostic(&crate::diagnostics::codes::advanced::CONC008, site, message);
			self.report(diagnostic);
		}
	}

	fn value_affinity(
		&mut self,
		value: Arc<ValueOrigin>,
		active: &mut FxHashSet<usize>,
	) -> Affinity {
		let mut status = Affinity::Safe;
		for value in self.resolve_value_origin(value, &mut FxHashSet::default()) {
			if self.s.types.contains_executor_task(value.ty) {
				return Affinity::Executor;
			}
			if self.s.types.contains_executor_guard(value.ty) {
				return Affinity::Guard;
			}
			let own = match &value.kind {
				OriginKind::Function { captures, .. } => {
					let identity = Arc::as_ptr(&value) as usize;
					if !active.insert(identity) {
						return Affinity::Deferred;
					}
					let mut own = Affinity::Safe;
					for capture in captures {
						own = own.max(self.value_affinity(Arc::clone(capture), active));
					}
					active.remove(&identity);
					own
				}
				OriginKind::Aggregate(fields) => {
					let mut own = Affinity::Safe;
					for (_, value) in fields {
						own = own.max(self.value_affinity(Arc::clone(value), active));
					}
					own
				}
				OriginKind::Alternatives(values) => {
					let mut own = Affinity::Safe;
					for value in values {
						own = own.max(self.value_affinity(Arc::clone(value), active));
					}
					own
				}
				_ if self.type_may_hide_captures(value.ty) => Affinity::Deferred,
				_ => Affinity::Safe,
			};
			status = status.max(own);
		}
		status
	}

	pub(super) fn type_may_hide_captures(&self, ty: TypeId) -> bool {
		let mut pending = vec![ty];
		let mut seen = FxHashSet::default();
		while let Some(ty) = pending.pop() {
			if !seen.insert(ty) {
				continue;
			}
			match self.s.types.kind(ty) {
				TypeKind::Function(_) | TypeKind::Param(_) => return true,
				TypeKind::Rec(_) => pending.extend(self.s.types.rec_target(ty)),
				TypeKind::Object(shape) => pending.extend(
					self.s
						.types
						.shape_fields(shape)
						.iter()
						.map(|field| field.ty),
				),
				TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
					pending.extend([item, key])
				}
				TypeKind::Sync(_, item)
				| TypeKind::Pointer(item)
				| TypeKind::Ref(item, _)
				| TypeKind::CArray(item, _)
				| TypeKind::Iterator(item, _) => pending.push(item),
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(self.s.types.list_items(list))
				}
				TypeKind::Member(nominal, index) => {
					pending.push(self.s.types.payload(nominal, index))
				}
				TypeKind::NamedUnion(nominal, mask) => {
					for index in 0..self
						.s
						.types
						.sym_list(self.s.types.nominal(nominal).members)
						.len() as u16
					{
						if self.s.types.mask_has(mask, index) {
							pending.push(self.s.types.payload(nominal, index));
						}
					}
				}
				_ => {}
			}
		}
		false
	}

	fn check_callable_thread_application(
		&mut self,
		site: NodeId,
		call: &Invocation,
		active: &mut FxHashSet<Arc<ValueOrigin>>,
	) {
		for callee in self.resolve_value_origin(Arc::clone(&call.callee), &mut FxHashSet::default())
		{
			let OriginKind::Function {
				declaration,
				captures,
				..
			} = &callee.kind
			else {
				continue;
			};
			let Some(summary) = self.callable_summary(*declaration) else {
				continue;
			};
			if !active.insert(Arc::clone(&callee)) {
				continue;
			}
			let bindings = self.invocation_bindings(&callee, call, &summary);
			let replacements = summary
				.captures
				.iter()
				.cloned()
				.zip(captures.iter().cloned())
				.collect::<Vec<_>>();
			let arguments = vec![(*declaration, call.arguments.clone())];
			for transfer in &summary.thread_transfers {
				let transfer = transfer.replacing(&replacements).substituted(
					&mut self.s.types,
					&bindings,
					&arguments,
				);
				let status = self.value_affinity(transfer, &mut FxHashSet::default());
				if matches!(status, Affinity::Executor | Affinity::Guard)
					|| (status == Affinity::Deferred && self.value_owner(site).is_none())
				{
					self.report_thread_value(site, "a transferred value", status);
				}
			}
			for application in &summary.applications {
				let application = application.replacing(&replacements).substituted(
					&mut self.s.types,
					&bindings,
					&arguments,
				);
				self.check_callable_thread_application(site, &application, active);
			}
			active.remove(&callee);
		}
	}

	pub(super) fn callable_summary(&self, key: DeclKey) -> Option<CallableSummary> {
		self.a.callables.get(&key).cloned().or_else(|| {
			self.s
				.interface(self.s.types.decl_key_data(key).module)?
				.callables
				.get(&key)
				.cloned()
		})
	}

	/// Resolve field projections and factory results under their actual value
	/// arguments. Unbound parameter origins remain symbolic for their caller.
	pub(super) fn resolve_value_origin(
		&mut self,
		value: Arc<ValueOrigin>,
		active: &mut FxHashSet<DeclKey>,
	) -> Vec<Arc<ValueOrigin>> {
		match &value.kind {
			OriginKind::Alternatives(values) => {
				let values = values.clone();
				values
					.into_iter()
					.flat_map(|value| self.resolve_value_origin(value, active))
					.collect()
			}
			OriginKind::Project {
				value: source,
				step,
			} => {
				let step = *step;
				let mut results = Vec::new();
				for source in self.resolve_value_origin(Arc::clone(source), active) {
					if let OriginKind::Aggregate(fields) = &source.kind {
						for (field, source) in fields {
							if *field == step
								|| (step == Projection::Item
									&& matches!(field, Projection::Item | Projection::Index(_)))
								|| (*field == Projection::Item
									&& matches!(step, Projection::Index(_)))
							{
								results
									.extend(self.resolve_value_origin(Arc::clone(source), active));
							}
						}
					} else {
						results.push(Arc::new(ValueOrigin {
							ty: value.ty,
							kind: OriginKind::Project {
								value: source,
								step,
							},
						}));
					}
				}
				results
			}
			OriginKind::Call(call) => {
				let mut results = Vec::new();
				for callee in self.resolve_value_origin(Arc::clone(&call.callee), active) {
					let OriginKind::Function {
						declaration,
						captures,
						..
					} = &callee.kind
					else {
						results.push(Arc::clone(&value));
						continue;
					};
					let Some(summary) = self.callable_summary(*declaration) else {
						results.push(ValueOrigin::opaque(value.ty));
						continue;
					};
					if !active.insert(*declaration) {
						results.push(ValueOrigin::opaque(value.ty));
						continue;
					}
					let bindings = self.invocation_bindings(&callee, call, &summary);
					let replacements = summary
						.captures
						.iter()
						.cloned()
						.zip(captures.iter().cloned())
						.collect::<Vec<_>>();
					let arguments = vec![(*declaration, call.arguments.clone())];
					let result = summary.result.replacing(&replacements).substituted(
						&mut self.s.types,
						&bindings,
						&arguments,
					);
					results.extend(self.resolve_value_origin(result, active));
					active.remove(declaration);
				}
				results
			}
			_ => vec![value],
		}
	}

	pub(super) fn invocation_bindings(
		&self,
		callee: &ValueOrigin,
		call: &Invocation,
		summary: &CallableSummary,
	) -> crate::types::Bindings {
		let mut result = crate::types::Bindings::default();
		let mut add = |param, ty| {
			if let Some(index) = result.params.iter().position(|known| *known == param) {
				result.types[index] = ty;
			} else {
				result.params.push(param);
				result.types.push(ty);
			}
		};
		if let OriginKind::Function { bindings, .. } = &callee.kind {
			for &(param, ty) in bindings {
				add(param, ty);
			}
		}
		let alpha = call.bindings.len() == summary.generics.len()
			&& !call
				.bindings
				.iter()
				.any(|(param, _)| summary.generics.contains(param));
		for (index, &(param, ty)) in call.bindings.iter().enumerate() {
			add(
				if alpha {
					summary.generics[index]
				} else {
					param
				},
				ty,
			);
		}
		if let OriginKind::Function { capabilities, .. } = &callee.kind {
			result.origins.extend(capabilities.origins.iter().cloned());
			result.methods.extend(capabilities.methods.iter().cloned());
		}
		let mapped = |param| {
			if alpha {
				call.bindings
					.iter()
					.position(|(known, _)| *known == param)
					.and_then(|index| summary.generics.get(index))
					.copied()
					.unwrap_or(param)
			} else {
				param
			}
		};
		for (param, origin) in &call.capabilities.origins {
			result.origins.insert(mapped(*param), origin.clone());
		}
		for (param, method) in &call.capabilities.methods {
			let param = mapped(*param);
			if let Some((_, selected)) = result
				.methods
				.iter_mut()
				.find(|(known, selected)| *known == param && selected.name == method.name)
			{
				*selected = method.clone();
			} else {
				result.methods.push((param, method.clone()));
			}
		}
		result
	}

	pub(super) fn finish_value_origins(&mut self) {
		let mut functions: Vec<_> = self.a.functions.values().cloned().collect();
		functions.sort_unstable_by_key(|info| {
			let span = self.span(info.lambda);
			(span.start, std::cmp::Reverse(span.end))
		});
		functions.dedup_by_key(|info| info.lambda);
		let mut parameters = FxHashMap::default();
		for (ordinal, info) in functions.iter().enumerate() {
			let declaration = info.declaration.unwrap_or(info.lambda);
			let key = self.node_key(declaration).unwrap_or_else(|| {
				let mut parent = self.parent(info.lambda);
				let mut names = Vec::new();
				while let Some(node) = parent {
					if let Some(&key) = self.a.callable_keys.get(&node) {
						names.extend(self.s.types.sym_list(self.s.types.decl_key_data(key).names));
						break;
					}
					parent = self.parent(node);
				}
				names.push(self.s.types.syms.intern(&format!("<lambda{ordinal}>")));
				let names = self.s.types.intern_sym_list(&names);
				self.s
					.types
					.decl_key_of(self.s.module_id(self.file), names, 0)
			});
			self.a.callable_keys.insert(declaration, key);
			self.a.callable_keys.insert(info.lambda, key);
			for (index, parameter) in info.params.iter().enumerate() {
				if let Ok(index) = u16::try_from(index) {
					parameters.insert(parameter.symbol, (key, index));
				}
			}
		}
		let mut partials: Vec<_> = self.a.partial_applications.keys().copied().collect();
		partials.sort_unstable_by_key(|&node| {
			let span = self.span(node);
			(span.start, std::cmp::Reverse(span.end))
		});
		for (ordinal, node) in partials.iter().enumerate() {
			let name = self.s.types.syms.intern(&format!("<partial{ordinal}>"));
			let names = self.s.types.intern_sym_list(&[name]);
			let key = self
				.s
				.types
				.decl_key_of(self.s.module_id(self.file), names, 0);
			self.a.callable_keys.insert(*node, key);
		}
		let copy_sites: Vec<_> = self.copy_sites.iter().copied().collect();
		let mut calls: Vec<_> = self.a.calls.keys().copied().collect();
		calls.sort_unstable();
		let mut builder = Builder {
			w: self,
			parameters,
			active: FxHashSet::default(),
		};
		for node in partials {
			builder.origin(node);
		}
		for info in functions {
			let declaration = info.declaration.unwrap_or(info.lambda);
			let key = builder.w.a.callable_keys[&declaration];
			let mut results = vec![builder.origin(info.body)];
			for &value in &copy_sites {
				let mut node = builder.w.parent(value);
				while let Some(current) = node {
					if matches!(builder.w.view(current), View::Return(_)) {
						if builder.owner(current) == Some(key) {
							results.push(builder.origin(value));
						}
						break;
					}
					if builder.w.a.function(current).is_some() {
						break;
					}
					node = builder.w.parent(current);
				}
			}
			let result = builder.alternatives(builder.w.a.type_of(info.body), results);
			let captures = info
				.captures
				.iter()
				.map(|&symbol| builder.symbol(symbol))
				.collect();
			let mut applications = Vec::new();
			for &site in &calls {
				if builder.owner(site) != Some(key) {
					continue;
				}
				if let Some(call) = builder.invocation(site) {
					applications.push(call);
				}
			}
			builder.w.a.callables.insert(
				key,
				CallableSummary {
					generics: info.generics,
					captures,
					result,
					applications,
					copy_requirements: Vec::new(),
					thread_transfers: Vec::new(),
				},
			);
		}
		for site in calls {
			builder.origin(site);
		}
		let mut applications: Vec<_> = builder.w.copy_instantiations.keys().copied().collect();
		applications.sort_unstable();
		for site in applications {
			builder.origin(site);
		}
		let methods: Vec<_> = builder.w.a.method_instantiations.keys().copied().collect();
		for site in methods {
			builder.origin(site);
		}
		for (_, value, _) in builder.w.thread_values.clone() {
			builder.origin(value);
		}
		for (_, value, _) in builder.w.thread_result_values.clone() {
			builder.origin(value);
		}
		// Values named by an interface need their actual origin, including
		// closures stored in an exported object rather than returned directly.
		let exports: Vec<_> = builder
			.w
			.s
			.source_file(builder.w.file)
			.interface
			.members
			.values()
			.chain(&builder.w.s.source_file(builder.w.file).interface.export)
			.map(|value| (value.key, value.decl))
			.collect();
		let values: FxHashMap<_, _> = exports
			.into_iter()
			.map(|(key, node)| (key, builder.origin(node)))
			.collect();
		let callables = builder.w.a.callables.clone();
		let interface = &mut builder.w.s.source_file_mut(builder.w.file).interface;
		interface.callables = callables;
		interface.value_origins = values;
	}
}

impl Builder<'_, '_> {
	fn owner(&self, mut node: NodeId) -> Option<DeclKey> {
		loop {
			if let Some(&key) = self.w.a.callable_keys.get(&node)
				&& !self.w.a.partial_applications.contains_key(&node)
			{
				return Some(key);
			}
			node = self.w.parent(node)?;
		}
	}

	fn alternatives(&self, ty: TypeId, values: Vec<Arc<ValueOrigin>>) -> Arc<ValueOrigin> {
		let mut unique = Vec::new();
		for value in values {
			if !unique.contains(&value) {
				unique.push(value);
			}
		}
		if unique.len() == 1 {
			return unique.pop().unwrap();
		}
		Arc::new(ValueOrigin {
			ty,
			kind: OriginKind::Alternatives(unique),
		})
	}

	fn origin(&mut self, node: NodeId) -> Arc<ValueOrigin> {
		if let Some(value) = self.w.a.value_origins.get(&node) {
			return Arc::clone(value);
		}
		let ty = self.w.s.types.widen(self.w.a.type_of(node));
		if !self.active.insert(node) {
			return ValueOrigin::opaque(ty);
		}
		let mut result = if let Some(reads) = self.w.value_reads.get(&node).cloned() {
			let values = reads.iter().map(|read| self.read_origin(read)).collect();
			self.alternatives(ty, values)
		} else {
			self.origin_uncached(node, ty)
		};
		if let Some(owner) = self.w.a.method_instantiations.get(&node)
			&& let Some(bindings) = self.w.written_bindings(owner)
			&& let OriginKind::Function { .. } = &result.kind
		{
			let mut copy = (*result).clone();
			if let OriginKind::Function {
				bindings: own,
				capabilities,
				..
			} = &mut copy.kind
			{
				Arc::make_mut(capabilities).extend(&bindings);
				for (param, ty) in bindings.params.into_iter().zip(bindings.types) {
					if let Some((_, known)) = own.iter_mut().find(|(known, _)| *known == param) {
						*known = ty;
					} else {
						own.push((param, ty));
					}
				}
			}
			result = Arc::new(copy);
		}
		if let Some(bindings) = self.w.copy_instantiations.get(&node)
			&& let OriginKind::Function { bindings: own, .. } = &result.kind
		{
			let mut copy = (*result).clone();
			let mut merged = own.clone();
			for (&param, &ty) in bindings.params.iter().zip(&bindings.types) {
				if let Some((_, known)) = merged.iter_mut().find(|(known, _)| *known == param) {
					*known = ty;
				} else {
					merged.push((param, ty));
				}
			}
			if let OriginKind::Function {
				bindings,
				capabilities,
				..
			} = &mut copy.kind
			{
				if let Some(current) = self.w.copy_instantiations.get(&node) {
					Arc::make_mut(capabilities).extend(current);
				}
				*bindings = merged;
			}
			result = Arc::new(copy);
		}
		self.active.remove(&node);
		self.w.a.value_origins.insert(node, Arc::clone(&result));
		result
	}

	fn symbol(&mut self, symbol: SymbolId) -> Arc<ValueOrigin> {
		let data = self.w.a.symbols.symbol(symbol).clone();
		if let Some(&(owner, index)) = self.parameters.get(&symbol) {
			return Arc::new(ValueOrigin {
				ty: data.ty,
				kind: OriginKind::Parameter { owner, index },
			});
		}
		if let Some(function) = self.w.direct_function(symbol) {
			return self.origin(function);
		}
		match self.w.view(data.decl_node) {
			View::Declaration {
				value: Some(value), ..
			}
			| View::Function { value, .. } => self.origin(value),
			_ => ValueOrigin::opaque(data.ty),
		}
	}

	fn read_origin(&mut self, read: &super::value_state::ValueRead) -> Arc<ValueOrigin> {
		let (base, ty) = match read.root {
			crate::flow::PlaceRoot::Symbol(symbol) => {
				(self.symbol(symbol), self.w.a.symbols.symbol(symbol).ty)
			}
			crate::flow::PlaceRoot::Temporary(node) => (self.origin(node), self.w.a.type_of(node)),
			crate::flow::PlaceRoot::ModuleValue(_) => {
				return ValueOrigin::opaque(TypeId::UNKNOWN);
			}
		};
		self.updated_origin(base, ty, &read.path, &read.bindings)
	}

	fn projection_type(&self, mut ty: TypeId, step: Projection) -> TypeId {
		let mut seen = FxHashSet::default();
		while seen.insert(ty) {
			match self.w.s.types.kind(ty) {
				TypeKind::Param(param) => ty = self.w.s.types.param(param).constraint,
				TypeKind::Rec(_) => match self.w.s.types.rec_target(ty) {
					Some(target) => ty = target,
					None => break,
				},
				_ => break,
			}
		}
		match (step, self.w.s.types.kind(ty)) {
			(Projection::Field(name), _) => self
				.w
				.s
				.types
				.fields_of(ty)
				.iter()
				.find(|field| field.name == name)
				.map_or(TypeId::UNKNOWN, |field| field.ty),
			(Projection::Index(index), TypeKind::Tuple(list)) => self
				.w
				.s
				.types
				.list_items(list)
				.get(index as usize)
				.copied()
				.unwrap_or(TypeId::UNKNOWN),
			(
				Projection::Item | Projection::Index(_),
				TypeKind::Collection(_, item, _)
				| TypeKind::CArray(item, _)
				| TypeKind::Iterator(item, _),
			) => item,
			_ => TypeId::UNKNOWN,
		}
	}

	fn projected(&self, base: Arc<ValueOrigin>, step: Projection, ty: TypeId) -> Arc<ValueOrigin> {
		Arc::new(ValueOrigin {
			ty,
			kind: OriginKind::Project { value: base, step },
		})
	}

	fn updated_origin(
		&mut self,
		mut base: Arc<ValueOrigin>,
		ty: TypeId,
		path: &[Projection],
		bindings: &[(Vec<Projection>, Vec<Option<NodeId>>)],
	) -> Arc<ValueOrigin> {
		if let Some((_, sources)) = bindings.iter().find(|(path, _)| path.is_empty()) {
			let values = sources
				.iter()
				.map(|source| match source {
					Some(node) => self.origin(*node),
					None => Arc::clone(&base),
				})
				.collect();
			base = self.alternatives(ty, values);
		}
		let below = |step| {
			bindings
				.iter()
				.filter_map(|(path, sources)| {
					let (first, rest) = path.split_first()?;
					super::value_state::projection_matches(*first, step)
						.then(|| (rest.to_vec(), sources.clone()))
				})
				.collect::<Vec<_>>()
		};
		if let Some((&step, rest)) = path.split_first() {
			let next = self.projection_type(ty, step);
			let value = self.projected(base, step, next);
			return self.updated_origin(value, next, rest, &below(step));
		}
		let mut fields = Vec::new();
		for (path, _) in bindings {
			if let Some(&first) = path.first()
				&& !fields.contains(&first)
			{
				fields.push(first);
			}
		}
		if fields.is_empty() {
			return base;
		}
		// Retain the unmodified fields as projections of the copied value.
		for field in self.w.s.types.fields_of(ty) {
			let step = Projection::Field(field.name);
			if !fields.contains(&step) {
				fields.push(step);
			}
		}
		match self.w.s.types.kind(ty) {
			TypeKind::Collection(..) => {
				if !fields.contains(&Projection::Item) {
					fields.push(Projection::Item);
				}
			}
			TypeKind::Tuple(list) => {
				for index in 0..self.w.s.types.list_items(list).len() {
					let step = Projection::Index(index as u32);
					if !fields.contains(&step) {
						fields.push(step);
					}
				}
			}
			_ => {}
		}
		let mut values = Vec::new();
		for step in fields {
			let next = self.projection_type(ty, step);
			let value = self.projected(Arc::clone(&base), step, next);
			values.push((step, self.updated_origin(value, next, &[], &below(step))));
		}
		Arc::new(ValueOrigin {
			ty,
			kind: OriginKind::Aggregate(values),
		})
	}

	fn function(&mut self, node: NodeId, site: NodeId, ty: TypeId) -> Arc<ValueOrigin> {
		let Some(&declaration) = self.w.a.callable_keys.get(&node) else {
			return ValueOrigin::opaque(ty);
		};
		let ty = if let Some(info) = self.w.a.function(node) {
			let ty = info.ty;
			let generics = info.generics.clone();
			let generics: Vec<_> = generics
				.into_iter()
				.map(|param| self.w.s.types.intern(TypeKind::Param(param)))
				.collect();
			self.w.generic_function_type(ty, &generics)
		} else {
			ty
		};
		let defaults = self.w.a.function(node).map_or_else(Vec::new, |info| {
			info.params
				.iter()
				.map(|param| param.default.is_some())
				.collect()
		});
		let captures = self
			.w
			.a
			.function(node)
			.map(|info| info.captures.clone())
			.unwrap_or_default();
		let captures = captures
			.into_iter()
			.map(|symbol| {
				if let Some(reads) = self.w.capture_value_reads.get(&(site, symbol)).cloned() {
					let ty = self.w.a.symbols.symbol(symbol).ty;
					let values = reads.iter().map(|read| self.read_origin(read)).collect();
					self.alternatives(ty, values)
				} else {
					self.symbol(symbol)
				}
			})
			.collect();
		Arc::new(ValueOrigin {
			ty,
			kind: OriginKind::Function {
				declaration,
				bindings: Vec::new(),
				capabilities: Default::default(),
				captures,
				defaults,
			},
		})
	}

	/// A residual function is a normal portable callable. Its synthetic
	/// body calls the captured target with its captured and remaining
	/// arguments in the original positions; no syntax crosses the interface.
	fn partial_origin(
		&mut self,
		node: NodeId,
		ty: TypeId,
		partial: &crate::analysis::PartialApplication,
	) -> Arc<ValueOrigin> {
		let key = self.w.a.callable_keys[&node];
		let mut captures = vec![self.origin(partial.function)];
		captures.extend(partial.bound.iter().map(|&(_, value)| self.origin(value)));
		let symbolic: Vec<_> = captures
			.iter()
			.enumerate()
			.map(|(index, value)| {
				Arc::new(ValueOrigin {
					ty: value.ty,
					kind: OriginKind::Parameter {
						owner: key,
						index: (partial.remaining.len() + index) as u16,
					},
				})
			})
			.collect();
		let signature = self.w.signature_of(ty).expect("a checked partial");
		let parameters = self.w.s.types.params(signature.params).to_vec();
		let original = self
			.w
			.signature_of(partial.original_type)
			.expect("a checked target");
		let arguments = self
			.w
			.s
			.types
			.params(original.params)
			.iter()
			.enumerate()
			.map(|(index, param)| {
				if let Some(position) = partial
					.bound
					.iter()
					.position(|&(bound, _)| usize::from(bound) == index)
				{
					return Arc::clone(&symbolic[position + 1]);
				}
				let Some(position) = partial
					.remaining
					.iter()
					.position(|&remaining| usize::from(remaining) == index)
				else {
					return ValueOrigin::opaque(param.ty);
				};
				Arc::new(ValueOrigin {
					ty: parameters[position].ty,
					kind: OriginKind::Parameter {
						owner: key,
						index: position as u16,
					},
				})
			})
			.collect();
		let call = Invocation {
			capabilities: self.capabilities(node),
			callee: Arc::clone(&symbolic[0]),
			arguments,
			bindings: partial
				.generic_parameters
				.iter()
				.copied()
				.zip(
					self.w
						.s
						.types
						.list_items(partial.generic_arguments)
						.iter()
						.copied(),
				)
				.collect(),
		};
		self.w.a.callables.insert(
			key,
			CallableSummary {
				generics: self.w.own_generics(ty),
				captures: symbolic,
				result: Arc::new(ValueOrigin {
					ty: signature.ret,
					kind: OriginKind::Call(call.clone()),
				}),
				applications: vec![call],
				copy_requirements: Vec::new(),
				thread_transfers: Vec::new(),
			},
		);
		Arc::new(ValueOrigin {
			ty,
			kind: OriginKind::Function {
				declaration: key,
				bindings: Vec::new(),
				capabilities: Default::default(),
				captures,
				defaults: partial
					.remaining
					.iter()
					.map(|&index| partial.defaults[index as usize])
					.collect(),
			},
		})
	}

	fn origin_uncached(&mut self, node: NodeId, ty: TypeId) -> Arc<ValueOrigin> {
		if let Some(partial) = self.w.a.partial_applications.get(&node).cloned() {
			return self.partial_origin(node, ty, &partial);
		}
		if self.w.a.function(node).is_some() {
			return self.function(node, node, ty);
		}
		if let Some(object) = self.w.a.object(node).cloned() {
			let fields = object
				.fields
				.into_iter()
				.map(|(name, value)| {
					let origin = match value {
						FieldValue::Given(value) | FieldValue::Default(value) => self.origin(value),
						FieldValue::Spread(value) => Arc::new(ValueOrigin {
							ty: TypeId::UNKNOWN,
							kind: OriginKind::Project {
								value: self.origin(value),
								step: Projection::Field(name),
							},
						}),
						_ => ValueOrigin::opaque(TypeId::UNKNOWN),
					};
					(Projection::Field(name), origin)
				})
				.collect();
			return Arc::new(ValueOrigin {
				ty,
				kind: OriginKind::Aggregate(fields),
			});
		}
		match self.w.a.resolution(node) {
			Resolution::Local(symbol) => {
				return match self.w.direct_function(symbol) {
					Some(function) => self.function(function, node, ty),
					None => self.symbol(symbol),
				};
			}
			Resolution::External(key) => {
				let interface = self.w.s.interface(self.w.s.types.decl_key_data(key).module);
				if let Some(value) =
					interface.and_then(|interface| interface.value_origins.get(&key))
				{
					return Arc::clone(value);
				}
				if let Some(interface) = interface
					&& interface
						.keys
						.get(&key)
						.is_some_and(|decl| interface.parameters.contains_key(&decl.node))
				{
					return Arc::new(ValueOrigin {
						ty,
						kind: OriginKind::Function {
							declaration: key,
							bindings: Vec::new(),
							capabilities: Default::default(),
							captures: Vec::new(),
							defaults: interface
								.keys
								.get(&key)
								.and_then(|decl| interface.parameters.get(&decl.node))
								.map_or_else(Vec::new, |params| {
									params.iter().map(|param| param.has_default).collect()
								}),
						},
					});
				}
			}
			_ => {}
		}
		let tree = Arc::clone(&self.w.tree);
		let kind = match crate::syntax::view(&tree, node) {
			View::Group(value)
			| View::Try(value)
			| View::Copy(value)
			| View::Do(value)
			| View::Return(Some(value)) => return self.origin(value),
			View::Declaration {
				value: Some(value), ..
			}
			| View::Function { value, .. }
			| View::ExportFunction { value, .. }
			| View::ExportValue { value, .. } => return self.origin(value),
			View::Block { statements } => {
				return statements
					.last()
					.map(|value| self.origin(value))
					.unwrap_or_else(|| ValueOrigin::opaque(ty));
			}
			View::If {
				then_body,
				else_body,
				..
			} => {
				let values = std::iter::once(then_body)
					.chain(else_body)
					.map(|value| self.origin(value))
					.collect();
				return self.alternatives(ty, values);
			}
			View::When { arms, .. } => {
				let values = arms.iter().map(|arm| self.origin(arm.value)).collect();
				return self.alternatives(ty, values);
			}
			View::Catch {
				expression,
				handler,
				..
			} => {
				let values = vec![self.origin(expression), self.origin(handler)];
				return self.alternatives(ty, values);
			}
			View::Fallback { operands } => {
				let values = operands.iter().map(|value| self.origin(value)).collect();
				return self.alternatives(ty, values);
			}
			View::Array { items } => OriginKind::Aggregate(
				items
					.iter()
					.map(|value| (Projection::Item, self.origin(value)))
					.collect(),
			),
			View::Tuple { items } => OriginKind::Aggregate(
				items
					.iter()
					.enumerate()
					.map(|(index, value)| (Projection::Index(index as u32), self.origin(value)))
					.collect(),
			),
			View::Index { target, index } => {
				let step = self
					.w
					.s
					.types
					.interval_of(self.w.a.type_of(index))
					.and_then(|range| range.as_exact())
					.and_then(|value| u32::try_from(value).ok())
					.map(Projection::Index)
					.unwrap_or(Projection::Item);
				OriginKind::Project {
					value: self.origin(target),
					step,
				}
			}
			View::Access { target, members } | View::OptionalAccess { target, members } => {
				if let Some(last) = members.last()
					&& matches!(
						self.w.a.resolution(last),
						Resolution::Local(_) | Resolution::External(_)
					) {
					return self.origin(last);
				}
				let mut value = self.origin(target);
				for member in members.iter() {
					let name = self.w.s.types.syms.intern(tree.text(member.index()));
					value = Arc::new(ValueOrigin {
						ty: self.w.a.type_of(member),
						kind: OriginKind::Project {
							value,
							step: Projection::Field(name),
						},
					});
				}
				return value;
			}
			View::Call { callee, .. } => {
				if !self.w.a.calls.contains_key(&node)
					&& self.w.a.instantiations.contains_key(&node)
				{
					let value = self.origin(callee);
					let mut value = (*value).clone();
					if let OriginKind::Function {
						bindings,
						capabilities,
						..
					} = &mut value.kind
					{
						*capabilities = self.capabilities(node);
						*bindings = self.bindings(node);
					}
					return Arc::new(value);
				}
				match self.invocation(node) {
					Some(call) => OriginKind::Call(call),
					None => OriginKind::Opaque,
				}
			}
			_ => OriginKind::Opaque,
		};
		Arc::new(ValueOrigin { ty, kind })
	}

	fn capabilities(&self, node: NodeId) -> Arc<CallableCapabilities> {
		Arc::new(
			self.w
				.copy_instantiations
				.get(&node)
				.map(CallableCapabilities::from_bindings)
				.unwrap_or_default(),
		)
	}

	fn bindings(&self, node: NodeId) -> Vec<(crate::ids::ParamId, TypeId)> {
		self.w
			.copy_instantiations
			.get(&node)
			.map(|bindings| {
				bindings
					.params
					.iter()
					.copied()
					.zip(bindings.types.iter().copied())
					.collect()
			})
			.unwrap_or_default()
	}

	fn invocation(&mut self, node: NodeId) -> Option<Invocation> {
		let call = self.w.a.call(node)?.clone();
		if !matches!(call.callee, Callee::Function(_) | Callee::Value) {
			return None;
		}
		let View::Call { callee, .. } = self.w.view(node) else {
			return None;
		};
		let callee_origin = self.origin(callee);
		let arguments = call
			.arguments
			.into_iter()
			.map(|argument| match argument {
				Argument::Given(value) => self.origin(value),
				Argument::Receiver(access) => {
					let View::Access { target, members } = self.w.view(access) else {
						return ValueOrigin::opaque(TypeId::UNKNOWN);
					};
					let names: Vec<_> = members
						.iter()
						.take(members.len().saturating_sub(1))
						.map(|member| self.w.tree.text(member.index()).to_string())
						.collect();
					let mut value = self.origin(target);
					for name in names {
						let name = self.w.s.types.syms.intern(&name);
						value = Arc::new(ValueOrigin {
							ty: TypeId::UNKNOWN,
							kind: OriginKind::Project {
								value,
								step: Projection::Field(name),
							},
						});
					}
					value
				}
				Argument::Variadic(values) => {
					let fields = values
						.into_iter()
						.map(|value| (Projection::Item, self.origin(value)))
						.collect();
					Arc::new(ValueOrigin {
						ty: TypeId::UNKNOWN,
						kind: OriginKind::Aggregate(fields),
					})
				}
				Argument::Default | Argument::Missing => ValueOrigin::opaque(TypeId::UNKNOWN),
			})
			.collect();
		let mut bindings = self.bindings(callee);
		bindings.extend(self.bindings(node));
		let mut capabilities = (*self.capabilities(callee)).clone();
		if let Some(current) = self.w.copy_instantiations.get(&node) {
			capabilities.extend(current);
		}
		Some(Invocation {
			callee: callee_origin,
			capabilities: Arc::new(capabilities),
			bindings,
			arguments,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::super::test_support::analyzed;
	use crate::ids::NodeId;
	use crate::provenance::OriginKind;
	use crate::types::Bindings;

	#[test]
	fn generic_factories_preserve_written_origins_and_selected_methods() {
		let (mut session, file) = analyzed(
			"type Person = { name: String, function describe = (self): String => self.name }\ntype Greeting = { name: String, function describe = (self): String => \"Hi \" + self.name }\nfunction make = <Value: { describe: (self) => String }>(value: Value) => () => value.describe()\nlet first = make(Person { name = \"Ada\" })\nlet second = make(Greeting { name = \"Ada\" })\nconsole.log(first(), second())\n",
		);
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let calls: Vec<_> = (0..tree.nodes.len())
			.filter_map(|node| {
				let text = tree.text(node);
				if !text.starts_with("make(") {
					return None;
				}
				let origin = analysis.value_origins.get(&NodeId::new(node))?;
				let OriginKind::Call(call) = &origin.kind else {
					return None;
				};
				Some(call.clone())
			})
			.collect();
		assert_eq!(calls.len(), 2);
		assert_eq!(
			calls[0].bindings, calls[1].bindings,
			"the structural types are equal"
		);
		assert_ne!(calls[0].capabilities, calls[1].capabilities);
		assert_ne!(
			calls[0].stable_hash(session.types()),
			calls[1].stable_hash(session.types())
		);
		let OriginKind::Function { declaration, .. } = calls[0].callee.kind else {
			panic!("a known factory");
		};
		let result = analysis.callables[&declaration].result.clone();
		for (call, expected) in calls.iter().zip(["Person", "Greeting"]) {
			let bindings = Bindings {
				params: call.bindings.iter().map(|&(param, _)| param).collect(),
				types: call.bindings.iter().map(|&(_, ty)| ty).collect(),
				origins: call.capabilities.origins.iter().cloned().collect(),
				methods: call.capabilities.methods.clone(),
			};
			let specialized = result.substituted(&mut session.types, &bindings, &[]);
			let OriginKind::Function { capabilities, .. } = &specialized.kind else {
				panic!("a returned closure");
			};
			let origin = &capabilities.origins[0].1;
			let names = session
				.types()
				.sym_list(session.types().decl_key_data(origin.declaration).names);
			assert_eq!(
				session.types().syms.resolve(*names.last().unwrap()),
				expected
			);
			assert_eq!(capabilities.methods.len(), 1);
			assert_eq!(capabilities.methods[0].1, call.capabilities.methods[0].1);
		}
	}
}
