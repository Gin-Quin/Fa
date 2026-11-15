//! Semantic instances of compile-time code. The interpreter sees opaque
//! function keys; the analyzer keeps their exact types and method selections.
//! A returned closure therefore names the instance which actually created it,
//! even when two alternatives have the same data representation.

use std::sync::Arc;

use fa_interpreter::ir::FnKey;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::GenericMethod;
use crate::ids::{DeclKey, ModuleId, ParamId, TypeId};
use crate::provenance::CallableCapabilities;
use crate::types::{Bindings, TypeKind, TypeStore};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Context {
	/// Global ids, sorted by parameter identity, including value binders.
	pub bindings: Vec<(ParamId, TypeId)>,
	pub capabilities: Arc<CallableCapabilities>,
}

impl Context {
	pub fn as_bindings(&self) -> Bindings {
		Bindings {
			params: self.bindings.iter().map(|entry| entry.0).collect(),
			types: self.bindings.iter().map(|entry| entry.1).collect(),
			origins: self.capabilities.origins.iter().cloned().collect(),
			methods: self.capabilities.methods.clone(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.bindings.is_empty()
			&& self.capabilities.origins.is_empty()
			&& self.capabilities.methods.is_empty()
	}

	/// Adds one invocation's bindings to its enclosing concrete context.
	/// Explicit method/origin forwarding follows the argument's parameter,
	/// independently of the structural type it is instantiated with.
	pub fn applied(&self, types: &TypeStore, module: ModuleId, given: &Bindings) -> Self {
		let mut merger = types.merger(Some(module));
		let params: Vec<_> = given
			.params
			.iter()
			.map(|&param| merger.param(param))
			.collect();
		let arguments: Vec<_> = given.types.iter().map(|&ty| merger.ty(ty)).collect();
		let caps = CallableCapabilities::from_bindings(given).merged(&mut merger);
		let module = merger.module(module);
		drop(merger);
		let mut scratch = TypeStore::view(types.global());
		let mut roots = arguments.clone();
		for (_, origin) in &caps.origins {
			written_roots(&scratch, origin, &mut roots);
		}
		for (_, method) in &caps.methods {
			method_roots(&scratch, method, &mut roots);
		}
		let enclosing = self.rebased(&mut scratch, &roots, &mut FxHashSet::default());
		let caps = caps.substituted(&mut scratch, &enclosing);
		let mut result = self.clone();
		let selected = Arc::make_mut(&mut result.capabilities);
		for (&param, &argument) in params.iter().zip(&arguments) {
			let key = parameter_key(&scratch, param);
			let param = result
				.bindings
				.iter()
				.find(|entry| parameter_key(&scratch, entry.0) == key)
				.map_or(param, |entry| entry.0);
			let bound = scratch.substitute(argument, &enclosing);
			if !matches!(scratch.kind(bound), TypeKind::Param(same) if parameter_key(&scratch, same) == key)
			{
				if let Some((_, current)) =
					result.bindings.iter_mut().find(|entry| entry.0 == param)
				{
					*current = bound;
				} else {
					result.bindings.push((param, bound));
				}
			}
			if let TypeKind::Param(forwarded) = scratch.kind(argument) {
				if let Some((_, origin)) = self.capabilities.origins.iter().find(|entry| {
					parameter_key(&scratch, entry.0) == parameter_key(&scratch, forwarded)
				}) {
					selected.origins.retain(|entry| entry.0 != param);
					selected.origins.push((param, origin.clone()));
				}
				for (_, method) in self.capabilities.methods.iter().filter(|entry| {
					parameter_key(&scratch, entry.0) == parameter_key(&scratch, forwarded)
				}) {
					selected
						.methods
						.retain(|entry| entry.0 != param || entry.1.name != method.name);
					selected.methods.push((param, method.clone()));
				}
			}
		}
		for (param, origin) in &caps.origins {
			let key = parameter_key(&scratch, *param);
			let param = result
				.bindings
				.iter()
				.find(|entry| parameter_key(&scratch, entry.0) == key)
				.map_or(*param, |entry| entry.0);
			selected
				.origins
				.retain(|entry| parameter_key(&scratch, entry.0) != key);
			selected.origins.push((param, origin.clone()));
		}
		for (param, method) in &caps.methods {
			let key = parameter_key(&scratch, *param);
			let param = result
				.bindings
				.iter()
				.find(|entry| parameter_key(&scratch, entry.0) == key)
				.map_or(*param, |entry| entry.0);
			selected.methods.retain(|entry| {
				parameter_key(&scratch, entry.0) != key || entry.1.name != method.name
			});
			selected.methods.push((param, method.clone()));
		}
		let mut merger = scratch.merger(Some(module));
		for (_, bound) in &mut result.bindings {
			*bound = merger.ty(*bound);
		}
		result.capabilities = result.capabilities.merged(&mut merger);
		result.bindings.sort_unstable_by_key(|entry| entry.0);
		Arc::make_mut(&mut result.capabilities)
			.origins
			.sort_unstable_by_key(|entry| entry.0);
		Arc::make_mut(&mut result.capabilities)
			.methods
			.sort_unstable_by_key(|entry| (entry.0, entry.1.name));
		result
	}

	pub fn concrete_type(&self, types: &TypeStore, module: ModuleId, ty: TypeId) -> TypeId {
		let mut merger = types.merger(Some(module));
		let ty = merger.ty(ty);
		let module = merger.module(module);
		drop(merger);
		if self.is_empty() {
			return ty;
		}
		let mut scratch = TypeStore::view(types.global());
		let bindings = self.rebased(&mut scratch, &[ty], &mut FxHashSet::default());
		let ty = scratch.substitute(ty, &bindings);
		scratch.merger(Some(module)).ty(ty)
	}

	pub fn bound(&self, types: &TypeStore, param: ParamId) -> Option<TypeId> {
		let key = parameter_key(types, param);
		self.bindings
			.iter()
			.find(|entry| parameter_key(types, entry.0) == key)
			.map(|entry| entry.1)
	}

	fn rebased(
		&self,
		types: &mut TypeStore,
		roots: &[TypeId],
		active: &mut FxHashSet<(DeclKey, u16)>,
	) -> Bindings {
		let mut bindings = Bindings::default();
		let mut params = Vec::new();
		parameters_in(types, roots, &mut params);
		for param in params {
			let key = parameter_key(types, param);
			let Some(bound) = self.bound(types, param) else {
				continue;
			};
			if !active.insert(key) {
				continue;
			}
			let nested = self.rebased(types, &[bound], active);
			let bound = types.substitute(bound, &nested);
			active.remove(&key);
			bindings.params.push(param);
			bindings.types.push(bound);
		}
		bindings
	}

	pub fn method(&self, param: ParamId, name: crate::ids::Sym) -> Option<&GenericMethod> {
		self.capabilities
			.methods
			.iter()
			.find(|entry| entry.0 == param && entry.1.name == name)
			.map(|entry| &entry.1)
	}
}

/// Parameters are deliberately copied by each Merger. Their declaration
/// owner and position are the portable identity, not that copied index.
pub fn parameter_key(types: &TypeStore, param: ParamId) -> (DeclKey, u16) {
	let info = types.param(param);
	let owner = types.merger(None).decl_key(info.owner);
	(owner, info.index)
}

fn parameters_in(types: &TypeStore, roots: &[TypeId], found: &mut Vec<ParamId>) {
	let mut seen = FxHashSet::default();
	let mut pending = roots.to_vec();
	while let Some(ty) = pending.pop() {
		if !seen.insert(ty) {
			continue;
		}
		match types.kind(ty) {
			TypeKind::Param(param) => {
				if !found.contains(&param) {
					found.push(param);
				}
			}
			TypeKind::Object(shape) => {
				pending.extend(types.shape_fields(shape).iter().map(|field| field.ty));
				if let Some(nominal) = types.shape(shape).nominal {
					pending.extend(types.list_items(types.nominal(nominal).args));
				}
			}
			TypeKind::NamedUnion(nominal, _)
			| TypeKind::Member(nominal, _)
			| TypeKind::Enum(nominal, _)
			| TypeKind::EnumMember(nominal, _)
			| TypeKind::CStruct(nominal) => pending.extend(types.list_items(types.nominal(nominal).args)),
			TypeKind::Tuple(items) | TypeKind::Union(items) | TypeKind::Fields(items) => {
				pending.extend(types.list_items(items))
			}
			TypeKind::Pointer(item)
			| TypeKind::CArray(item, _)
			| TypeKind::Ref(item, _)
			| TypeKind::Iterator(item, _)
			| TypeKind::Sync(_, item) => pending.push(item),
			TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
				pending.extend([item, key])
			}
			TypeKind::Rec(rec) => {
				pending.extend(types.list_items(types.rec_info(rec).args));
				if let Some(target) = types.rec_target(ty) {
					pending.push(target);
				}
			}
			TypeKind::Function(sig) => {
				let sig = types.sig(sig);
				pending.extend(types.params(sig.params).iter().map(|param| param.ty));
				pending.extend([sig.ret, sig.throws]);
				pending.extend(types.list_items(sig.generics));
				pending.extend(
					types
						.requirements(sig.requires)
						.iter()
						.map(|requirement| requirement.ty),
				);
			}
			_ => {}
		}
	}
}

fn written_roots(
	types: &TypeStore,
	origin: &crate::analysis::WrittenType,
	roots: &mut Vec<TypeId>,
) {
	roots.extend(types.list_items(origin.arguments));
	for origin in origin.origins.iter().flatten() {
		written_roots(types, origin, roots);
	}
	for method in origin.methods.iter() {
		method_roots(types, method, roots);
	}
}

fn method_roots(types: &TypeStore, method: &GenericMethod, roots: &mut Vec<TypeId>) {
	roots.extend(types.list_items(method.arguments));
	for method in method.methods.iter() {
		method_roots(types, method, roots);
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Instance {
	pub source: FnKey,
	pub context: Context,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Application {
	pub arguments: Vec<TypeId>,
	pub origins: Vec<(u16, crate::analysis::WrittenType)>,
	pub methods: Vec<GenericMethod>,
}

#[derive(Debug)]
enum Entry {
	Function(Instance),
	Application(Application),
}

#[derive(Debug, Default)]
pub struct Instances {
	keys: FxHashMap<Instance, FnKey>,
	applications: FxHashMap<Application, FnKey>,
	values: Vec<Entry>,
}

impl Instances {
	pub fn function(&mut self, source: FnKey, context: Context) -> FnKey {
		if context.is_empty() {
			return source;
		}
		let instance = Instance { source, context };
		if let Some(&key) = self.keys.get(&instance) {
			return key;
		}
		let key = FnKey(u32::MAX, self.values.len() as u32);
		self.values.push(Entry::Function(instance.clone()));
		self.keys.insert(instance, key);
		key
	}

	pub fn application(&mut self, application: Application) -> FnKey {
		if let Some(&key) = self.applications.get(&application) {
			return key;
		}
		let key = FnKey(u32::MAX, self.values.len() as u32);
		self.values.push(Entry::Application(application.clone()));
		self.applications.insert(application, key);
		key
	}

	pub fn get(&self, key: FnKey) -> Option<&Instance> {
		if key.0 != u32::MAX {
			return None;
		}
		match self.values.get(key.1 as usize)? {
			Entry::Function(instance) => Some(instance),
			_ => None,
		}
	}

	pub fn get_application(&self, key: FnKey) -> Option<&Application> {
		if key.0 != u32::MAX {
			return None;
		}
		match self.values.get(key.1 as usize)? {
			Entry::Application(application) => Some(application),
			_ => None,
		}
	}
}

fn analysis_of<'a>(
	session: &'a crate::Session,
	current: Option<(crate::ids::FileId, &'a crate::analysis::Analysis)>,
	file: crate::ids::FileId,
) -> &'a crate::analysis::Analysis {
	match current {
		Some((own, analysis)) if own == file => analysis,
		_ => session.analysis(file),
	}
}

pub fn instantiate(
	session: &crate::Session,
	current: Option<(crate::ids::FileId, &crate::analysis::Analysis)>,
	function: FnKey,
	application: FnKey,
) -> Result<FnKey, fa_interpreter::Error> {
	let (instance, application) = {
		let registry = session
			.comptime
			.instances
			.lock()
			.expect("the compile-time instance registry");
		(
			registry.get(function).cloned(),
			registry.get_application(application).cloned(),
		)
	};
	let application = application
		.ok_or_else(|| fa_interpreter::Error::Stop("missing typed generic application".into()))?;
	let source = instance
		.as_ref()
		.map_or(function, |instance| instance.source);
	let context = instance.map_or_else(Context::default, |instance| instance.context);
	let file = crate::ids::FileId(source.0);
	let analysis = analysis_of(session, current, file);
	let info = analysis
		.function(crate::ids::NodeId(source.1))
		.ok_or_else(|| fa_interpreter::Error::Stop("missing generic function template".into()))?;
	let params: Vec<_> = info
		.generics
		.iter()
		.copied()
		.filter(|&param| context.bound(session.types(), param).is_none())
		.collect();
	if params.len() != application.arguments.len() {
		return Err(fa_interpreter::Error::Stop(format!(
			"generic application has {} arguments for {} remaining binders",
			application.arguments.len(),
			params.len()
		)));
	}
	let mut given = Bindings {
		params,
		types: application.arguments,
		..Default::default()
	};
	for (index, origin) in application.origins {
		if let Some(&param) = given.params.get(index as usize) {
			given.origins.insert(param, origin);
		}
	}
	for method in application.methods {
		if let Some(&param) = given.params.get(method.parameter as usize) {
			given.methods.push((param, method));
		}
	}
	let context = context.applied(session.types(), session.module_id(file), &given);
	Ok(session
		.comptime
		.instances
		.lock()
		.expect("the compile-time instance registry")
		.function(source, context))
}

pub fn bind_partial(
	session: &crate::Session,
	current: Option<(crate::ids::FileId, &crate::analysis::Analysis)>,
	function: FnKey,
	site: FnKey,
) -> Result<FnKey, fa_interpreter::Error> {
	let (instance, creation) = {
		let registry = session
			.comptime
			.instances
			.lock()
			.expect("the compile-time instance registry");
		(registry.get(function).cloned(), registry.get(site).cloned())
	};
	let source = instance
		.as_ref()
		.map_or(function, |instance| instance.source);
	let context = instance.map_or_else(Context::default, |instance| instance.context);
	let template = creation.as_ref().map_or(site, |instance| instance.source);
	let creation = creation.map_or_else(Context::default, |instance| instance.context);
	let file = crate::ids::FileId(template.0);
	let analysis = analysis_of(session, current, file);
	let Some(partial) = analysis
		.partial_applications
		.get(&crate::ids::NodeId(template.1))
	else {
		return Err(fa_interpreter::Error::Stop(
			"missing checked partial application".into(),
		));
	};
	let mut given = Bindings {
		params: partial.generic_parameters.clone(),
		types: session
			.types()
			.list_items(partial.generic_arguments)
			.to_vec(),
		..Default::default()
	};
	for (index, origin) in analysis
		.instantiation_origins
		.get(&crate::ids::NodeId(template.1))
		.into_iter()
		.flatten()
	{
		if let Some(&param) = given.params.get(*index as usize) {
			given.origins.insert(param, origin.clone());
		}
	}
	for method in analysis
		.instantiated_methods
		.get(&crate::ids::NodeId(template.1))
		.into_iter()
		.flatten()
	{
		if let Some(&param) = given.params.get(method.parameter as usize) {
			given.methods.push((param, method.clone()));
		}
	}
	let context = context
		.applied(
			session.types(),
			session.module_id(file),
			&creation.as_bindings(),
		)
		.applied(session.types(), session.module_id(file), &given);
	Ok(session
		.comptime
		.instances
		.lock()
		.expect("the compile-time instance registry")
		.function(source, context))
}
