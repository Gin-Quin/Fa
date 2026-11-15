//! Portable facts about values whose representation hides owned state.
//!
//! A function signature does not describe its captures. These expressions
//! preserve the actual callable, parameters, fields and factory results across
//! annotations and unit boundaries. They contain no syntax or per-file ids.

use std::sync::Arc;

use crate::analysis::{CopyRequirement, GenericMethod, WrittenType};
use crate::ids::{DeclKey, ParamId, Sym, TypeId};
use crate::types::merge::Merger;
use crate::types::{Bindings, TypeStore};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
	Field(Sym),
	Index(u32),
	Item,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValueOrigin {
	pub ty: TypeId,
	pub kind: OriginKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OriginKind {
	/// Only the visible type is known. A function here has unknown captures.
	Opaque,
	Parameter {
		owner: DeclKey,
		index: u16,
	},
	Function {
		declaration: DeclKey,
		bindings: Vec<(ParamId, TypeId)>,
		capabilities: Arc<CallableCapabilities>,
		captures: Vec<Arc<ValueOrigin>>,
		/// Actual declaration defaults; visibility remains in SignatureNames.
		defaults: Vec<bool>,
	},
	Aggregate(Vec<(Projection, Arc<ValueOrigin>)>),
	Project {
		value: Arc<ValueOrigin>,
		step: Projection,
	},
	Call(Invocation),
	Alternatives(Vec<Arc<ValueOrigin>>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Invocation {
	pub callee: Arc<ValueOrigin>,
	/// In declaration order, including implicit binders. A callable parameter
	/// may use alpha-renamed binders; resolution maps these by position.
	pub bindings: Vec<(ParamId, TypeId)>,
	pub capabilities: Arc<CallableCapabilities>,
	pub arguments: Vec<Arc<ValueOrigin>>,
}

/// Method and written-type selections are independent of structural type
/// bindings. Their sorted portable form survives factories and annotations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CallableCapabilities {
	pub origins: Vec<(ParamId, WrittenType)>,
	pub methods: Vec<(ParamId, GenericMethod)>,
}

impl CallableCapabilities {
	pub fn from_bindings(bindings: &Bindings) -> Self {
		let mut result = Self::default();
		result.extend(bindings);
		result
	}

	pub fn extend(&mut self, bindings: &Bindings) {
		for (&param, origin) in &bindings.origins {
			if let Some((_, current)) = self.origins.iter_mut().find(|(known, _)| *known == param) {
				*current = origin.clone();
			} else {
				self.origins.push((param, origin.clone()));
			}
		}
		for (param, method) in &bindings.methods {
			if let Some((_, current)) = self
				.methods
				.iter_mut()
				.find(|(known, selected)| known == param && selected.name == method.name)
			{
				*current = method.clone();
			} else {
				self.methods.push((*param, method.clone()));
			}
		}
		self.origins.sort_unstable_by_key(|(param, _)| *param);
		self.methods
			.sort_unstable_by_key(|(param, method)| (*param, method.name));
	}

	pub fn merged(&self, merger: &mut Merger<'_>) -> Arc<Self> {
		Arc::new(Self {
			origins: self
				.origins
				.iter()
				.map(|(param, origin)| (merger.param(*param), origin.merged(merger)))
				.collect(),
			methods: self
				.methods
				.iter()
				.map(|(param, method)| (merger.param(*param), method.merged(merger)))
				.collect(),
		})
	}

	pub fn substituted(self: &Arc<Self>, types: &mut TypeStore, bindings: &Bindings) -> Arc<Self> {
		if self.origins.is_empty()
			&& self.methods.is_empty()
			&& bindings.origins.is_empty()
			&& bindings.methods.is_empty()
		{
			return Arc::clone(self);
		}
		let mut result = Self {
			origins: self
				.origins
				.iter()
				.map(|(param, origin)| (*param, origin.substituted(types, bindings)))
				.collect(),
			methods: self
				.methods
				.iter()
				.map(|(param, method)| (*param, method.substituted(types, bindings)))
				.collect(),
		};
		result.extend(bindings);
		Arc::new(result)
	}

	pub fn stable_hash(&self, types: &TypeStore) -> u128 {
		let key = |param| {
			let info = types.param(param);
			types.decl_hash(info.owner) ^ u128::from(info.index)
		};
		// Sort semantic identities, since global interning order is unrelated
		// to the order in a source session or a parallel worker.
		let mut origins: Vec<_> = self
			.origins
			.iter()
			.map(|(param, origin)| (key(*param), origin.stable_hash(types)))
			.collect();
		let mut methods: Vec<_> = self
			.methods
			.iter()
			.map(|(param, method)| (key(*param), method.stable_hash(types)))
			.collect();
		origins.sort_unstable();
		methods.sort_unstable();
		origins
			.into_iter()
			.chain(methods)
			.fold(0u128, |hash, (param, value)| {
				hash.rotate_left(11) ^ param.rotate_left(5) ^ value
			})
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallableSummary {
	pub generics: Vec<ParamId>,
	/// Sources of the captures, before an enclosing factory substitutes them.
	pub captures: Vec<Arc<ValueOrigin>>,
	pub result: Arc<ValueOrigin>,
	pub applications: Vec<Invocation>,
	pub copy_requirements: Vec<CopyRequirement>,
	/// Materialized values crossing an executor boundary, checked after
	/// substituting the caller's actual value origins.
	pub thread_transfers: Vec<Arc<ValueOrigin>>,
}

impl ValueOrigin {
	pub fn opaque(ty: TypeId) -> Arc<Self> {
		Arc::new(Self {
			ty,
			kind: OriginKind::Opaque,
		})
	}

	pub fn merged(&self, merger: &mut Merger<'_>) -> Arc<Self> {
		let projection = |step: Projection, merger: &mut Merger<'_>| match step {
			Projection::Field(name) => Projection::Field(merger.sym(name)),
			other => other,
		};
		let kind = match &self.kind {
			OriginKind::Opaque => OriginKind::Opaque,
			OriginKind::Parameter { owner, index } => OriginKind::Parameter {
				owner: merger.decl_key(*owner),
				index: *index,
			},
			OriginKind::Function {
				declaration,
				bindings,
				capabilities,
				captures,
				defaults,
			} => OriginKind::Function {
				declaration: merger.decl_key(*declaration),
				bindings: merge_bindings(bindings, merger),
				capabilities: capabilities.merged(merger),
				captures: captures.iter().map(|value| value.merged(merger)).collect(),
				defaults: defaults.clone(),
			},
			OriginKind::Aggregate(fields) => OriginKind::Aggregate(
				fields
					.iter()
					.map(|(step, value)| (projection(*step, merger), value.merged(merger)))
					.collect(),
			),
			OriginKind::Project { value, step } => OriginKind::Project {
				value: value.merged(merger),
				step: projection(*step, merger),
			},
			OriginKind::Call(call) => OriginKind::Call(call.merged(merger)),
			OriginKind::Alternatives(values) => {
				OriginKind::Alternatives(values.iter().map(|value| value.merged(merger)).collect())
			}
		};
		Arc::new(Self {
			ty: merger.ty(self.ty),
			kind,
		})
	}

	/// Substitute types and actual argument values together. Keeping these
	/// substitutions separate from FnSig prevents annotations erasing captures.
	pub fn substituted(
		self: &Arc<Self>,
		types: &mut TypeStore,
		bindings: &Bindings,
		arguments: &[(DeclKey, Vec<Arc<ValueOrigin>>)],
	) -> Arc<Self> {
		if bindings.params.is_empty()
			&& bindings.origins.is_empty()
			&& bindings.methods.is_empty()
			&& (arguments.iter().all(|(_, values)| values.is_empty())
				|| matches!(&self.kind, OriginKind::Function { captures, .. } if captures.is_empty()))
		{
			return Arc::clone(self);
		}
		if matches!(self.kind, OriginKind::Opaque) {
			let ty = types.substitute(self.ty, bindings);
			return if ty == self.ty {
				Arc::clone(self)
			} else {
				Self::opaque(ty)
			};
		}
		let sub = |value: &Arc<Self>, types: &mut TypeStore| {
			value.substituted(types, bindings, arguments)
		};
		let kind = match &self.kind {
			OriginKind::Parameter { owner, index } => {
				if let Some(value) = arguments
					.iter()
					.rev()
					.find(|(key, _)| key == owner)
					.and_then(|(_, values)| values.get(usize::from(*index)))
				{
					return Arc::clone(value);
				}
				self.kind.clone()
			}
			OriginKind::Function {
				declaration,
				bindings: own,
				capabilities,
				captures,
				defaults,
			} => {
				let mut own: Vec<_> = own
					.iter()
					.map(|&(param, ty)| (param, types.substitute(ty, bindings)))
					.collect();
				for (&param, &ty) in bindings.params.iter().zip(&bindings.types) {
					if !own.iter().any(|(known, _)| *known == param) {
						own.push((param, ty));
					}
				}
				OriginKind::Function {
					declaration: *declaration,
					bindings: own,
					capabilities: capabilities.substituted(types, bindings),
					captures: captures.iter().map(|value| sub(value, types)).collect(),
					defaults: defaults.clone(),
				}
			}
			OriginKind::Aggregate(fields) => OriginKind::Aggregate(
				fields
					.iter()
					.map(|(step, value)| (*step, sub(value, types)))
					.collect(),
			),
			OriginKind::Project { value, step } => OriginKind::Project {
				value: sub(value, types),
				step: *step,
			},
			OriginKind::Call(call) => {
				OriginKind::Call(call.substituted(types, bindings, arguments))
			}
			OriginKind::Alternatives(values) => {
				OriginKind::Alternatives(values.iter().map(|value| sub(value, types)).collect())
			}
			OriginKind::Opaque => OriginKind::Opaque,
		};
		Arc::new(Self {
			ty: types.substitute(self.ty, bindings),
			kind,
		})
	}

	pub fn stable_hash(&self, types: &TypeStore) -> u128 {
		let mut hash = types.stable_hash(self.ty);
		let mut add = |value: u128| {
			hash = hash.rotate_left(11).wrapping_mul(31) ^ value;
		};
		match &self.kind {
			OriginKind::Opaque => add(1),
			OriginKind::Parameter { owner, index } => {
				add(2);
				add(types.decl_hash(*owner));
				add(u128::from(*index));
			}
			OriginKind::Function {
				declaration,
				bindings,
				capabilities,
				captures,
				defaults,
			} => {
				add(3);
				add(types.decl_hash(*declaration));
				add(bindings_hash(bindings, types));
				add(capabilities.stable_hash(types));
				for &default in defaults {
					add(u128::from(default));
				}
				for value in captures {
					add(value.stable_hash(types));
				}
			}
			OriginKind::Aggregate(fields) => {
				add(4);
				for (step, value) in fields {
					add(projection_hash(*step, types));
					add(value.stable_hash(types));
				}
			}
			OriginKind::Project { value, step } => {
				add(5);
				add(projection_hash(*step, types));
				add(value.stable_hash(types));
			}
			OriginKind::Call(call) => {
				add(6);
				add(call.stable_hash(types));
			}
			OriginKind::Alternatives(values) => {
				add(7);
				for value in values {
					add(value.stable_hash(types));
				}
			}
		}
		hash
	}

	/// Rebind free capture sources when a factory has returned this closure.
	pub fn replacing(self: &Arc<Self>, replacements: &[(Arc<Self>, Arc<Self>)]) -> Arc<Self> {
		if replacements.is_empty() {
			return Arc::clone(self);
		}
		if let Some((_, actual)) = replacements
			.iter()
			.find(|(source, _)| source.as_ref() == self.as_ref())
		{
			return Arc::clone(actual);
		}
		let replace = |value: &Arc<Self>| value.replacing(replacements);
		let kind = match &self.kind {
			OriginKind::Function {
				declaration,
				bindings,
				capabilities,
				captures,
				defaults,
			} => OriginKind::Function {
				declaration: *declaration,
				bindings: bindings.clone(),
				capabilities: Arc::clone(capabilities),
				captures: captures.iter().map(replace).collect(),
				defaults: defaults.clone(),
			},
			OriginKind::Aggregate(fields) => OriginKind::Aggregate(
				fields
					.iter()
					.map(|(step, value)| (*step, replace(value)))
					.collect(),
			),
			OriginKind::Project { value, step } => OriginKind::Project {
				value: replace(value),
				step: *step,
			},
			OriginKind::Call(call) => OriginKind::Call(call.replacing(replacements)),
			OriginKind::Alternatives(values) => {
				OriginKind::Alternatives(values.iter().map(replace).collect())
			}
			other => other.clone(),
		};
		Arc::new(Self { ty: self.ty, kind })
	}
}

impl Invocation {
	pub fn replacing(&self, replacements: &[(Arc<ValueOrigin>, Arc<ValueOrigin>)]) -> Self {
		Self {
			callee: self.callee.replacing(replacements),
			bindings: self.bindings.clone(),
			capabilities: Arc::clone(&self.capabilities),
			arguments: self
				.arguments
				.iter()
				.map(|value| value.replacing(replacements))
				.collect(),
		}
	}
	pub fn merged(&self, merger: &mut Merger<'_>) -> Self {
		Self {
			callee: self.callee.merged(merger),
			bindings: merge_bindings(&self.bindings, merger),
			capabilities: self.capabilities.merged(merger),
			arguments: self
				.arguments
				.iter()
				.map(|value| value.merged(merger))
				.collect(),
		}
	}

	pub fn substituted(
		&self,
		types: &mut TypeStore,
		bindings: &Bindings,
		arguments: &[(DeclKey, Vec<Arc<ValueOrigin>>)],
	) -> Self {
		Self {
			callee: self.callee.substituted(types, bindings, arguments),
			capabilities: self.capabilities.substituted(types, bindings),
			bindings: self
				.bindings
				.iter()
				.map(|&(param, ty)| (param, types.substitute(ty, bindings)))
				.collect(),
			arguments: self
				.arguments
				.iter()
				.map(|value| value.substituted(types, bindings, arguments))
				.collect(),
		}
	}

	pub fn stable_hash(&self, types: &TypeStore) -> u128 {
		let mut hash = self.callee.stable_hash(types)
			^ bindings_hash(&self.bindings, types)
			^ self.capabilities.stable_hash(types);
		for argument in &self.arguments {
			hash = hash.rotate_left(13) ^ argument.stable_hash(types);
		}
		hash
	}
}

impl CallableSummary {
	pub fn stable_hash(&self, types: &TypeStore) -> u128 {
		let mut hash = self.result.stable_hash(types);
		for &param in &self.generics {
			let info = types.param(param);
			hash = hash.rotate_left(7)
				^ types.decl_hash(info.owner)
				^ u128::from(info.index)
				^ types.stable_hash(info.constraint).rotate_left(11)
				^ types.stable_hash(info.default).rotate_left(13)
				^ u128::from(info.value);
		}
		for capture in &self.captures {
			hash = hash.rotate_left(7) ^ capture.stable_hash(types);
		}
		for call in &self.applications {
			hash = hash.rotate_left(11) ^ call.stable_hash(types);
		}
		for requirement in &self.copy_requirements {
			let gate = requirement.resource_gate.map_or(0, |(ty, path)| {
				types
					.list_items(path)
					.iter()
					.fold(types.stable_hash(ty), |hash, &name| {
						hash.rotate_left(7) ^ types.stable_hash(name)
					})
			});
			hash = hash.rotate_left(13)
				^ types.stable_hash(requirement.subject)
				^ u128::from(requirement.explicit)
				^ (u128::from(requirement.task_transfer) << 1)
				^ (u128::from(requirement.task_only) << 2)
				^ gate;
		}
		for value in &self.thread_transfers {
			hash = hash.rotate_left(17) ^ value.stable_hash(types);
		}
		hash
	}

	pub fn merged(&self, merger: &mut Merger<'_>) -> Self {
		Self {
			generics: self
				.generics
				.iter()
				.map(|&param| merger.param(param))
				.collect(),
			captures: self
				.captures
				.iter()
				.map(|value| value.merged(merger))
				.collect(),
			result: self.result.merged(merger),
			applications: self
				.applications
				.iter()
				.map(|call| call.merged(merger))
				.collect(),
			copy_requirements: self
				.copy_requirements
				.iter()
				.map(|requirement| CopyRequirement {
					subject: merger.ty(requirement.subject),
					resource_gate: requirement
						.resource_gate
						.map(|(ty, path)| (merger.ty(ty), merger.list(path))),
					..*requirement
				})
				.collect(),
			thread_transfers: self
				.thread_transfers
				.iter()
				.map(|value| value.merged(merger))
				.collect(),
		}
	}
}

fn merge_bindings(
	bindings: &[(ParamId, TypeId)],
	merger: &mut Merger<'_>,
) -> Vec<(ParamId, TypeId)> {
	bindings
		.iter()
		.map(|&(param, ty)| (merger.param(param), merger.ty(ty)))
		.collect()
}

fn bindings_hash(bindings: &[(ParamId, TypeId)], types: &TypeStore) -> u128 {
	let mut hash = 0u128;
	for &(param, ty) in bindings {
		let info = types.param(param);
		hash = hash.rotate_left(7)
			^ types.decl_hash(info.owner)
			^ u128::from(info.index)
			^ types.stable_hash(ty);
	}
	hash
}

fn projection_hash(step: Projection, types: &TypeStore) -> u128 {
	match step {
		Projection::Field(name) => types.syms.resolve(name).bytes().fold(1u128, |hash, byte| {
			hash.wrapping_mul(31).wrapping_add(u128::from(byte))
		}),
		Projection::Index(index) => u128::from(index) + 2,
		Projection::Item => 0,
	}
}
