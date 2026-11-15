//! Explicit binary-transport capabilities. Stable generic layouts compose
//! precompiled element operations; a union that substitution can normalize
//! keeps its whole type as a capability supplied at the concrete call site.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::check::Walker;
use crate::ids::{DeclKey, NodeId, ParamId, TypeId};
use crate::provenance::OriginKind;
use crate::types::{Bindings, TypeKind};

impl Walker<'_> {
	pub(super) fn use_wire_type(&mut self, site: NodeId, ty: TypeId) {
		if self.quiet > 0 || self.s.types.is_recovering(ty) {
			return;
		}
		let types = self.a.wire_uses.entry(site).or_default();
		if !types.contains(&ty) {
			types.push(ty);
		}
	}

	pub(super) fn finish_wire_requirements(&mut self) {
		let mut requirements: FxHashMap<DeclKey, Vec<TypeId>> = FxHashMap::default();
		let uses: Vec<_> = self
			.a
			.wire_uses
			.iter()
			.map(|(&site, types)| (site, types.clone()))
			.collect();
		for (site, types) in uses {
			let Some(owner) = self.wire_owner(site) else {
				continue;
			};
			for ty in types {
				for needed in minimal_codec_needs(&self.s.types, ty) {
					insert_requirement(&mut requirements, owner, needed);
				}
			}
		}
		let mut calls: Vec<_> = self
			.a
			.calls
			.keys()
			.filter_map(|&site| {
				let value = self.a.value_origins.get(&site)?;
				let OriginKind::Call(call) = &value.kind else {
					return None;
				};
				Some((site, call.clone(), self.wire_owner(site), false))
			})
			.collect();
		for (&site, origin) in &self.a.value_origins {
			if !self.a.calls.contains_key(&site)
				&& self.callable_materialization(site)
				&& matches!(origin.kind, OriginKind::Function { .. })
			{
				calls.push((
					site,
					crate::provenance::Invocation {
						callee: origin.clone(),
						bindings: Vec::new(),
						arguments: Vec::new(),
					},
					self.wire_owner(site),
					true,
				));
			}
		}
		calls.sort_unstable_by_key(|(site, ..)| *site);
		let mut applications = Vec::new();
		for (site, call, owner, materialized) in &calls {
			for callee in self.resolve_value_origin(call.callee.clone(), &mut FxHashSet::default())
			{
				let OriginKind::Function { declaration, .. } = callee.kind else {
					continue;
				};
				let Some(summary) = self.callable_summary(declaration) else {
					continue;
				};
				applications.push(Application {
					site: *site,
					owner: *owner,
					declaration,
					materialized: *materialized && self.a.instantiation(*site).is_none(),
					bindings: self.invocation_bindings(&callee, call, &summary),
				});
			}
		}
		let expanding = expanding_parameters(&self.s.types, &applications);
		for needed in requirements.values_mut() {
			needed.retain(|&ty| !self.reject_growing_wire(ty, &expanding));
		}
		loop {
			let mut changed = false;
			for application in &applications {
				let needed = self.wire_requirements_of(application.declaration, &requirements);
				for requirement in needed {
					let subject = self.s.types.substitute(requirement, &application.bindings);
					if unbound_materialization(&self.s.types, application, subject) {
						continue;
					}
					if let Some(owner) = application.owner {
						for needed in minimal_codec_needs(&self.s.types, subject) {
							if !self.reject_growing_wire(needed, &expanding) {
								changed |= insert_requirement(&mut requirements, owner, needed);
							}
						}
					}
				}
			}
			if !changed {
				break;
			}
		}
		for types in requirements.values_mut() {
			types.sort_by_cached_key(|&ty| {
				(
					crate::stable_names::stable_type_name(&self.s.types, ty),
					self.s.types.stable_hash(ty),
				)
			});
		}
		// Record arguments only after the summaries settle, so a temporary
		// empty summary does not look like an incompatible indirect callee.
		self.a.wire_arguments.clear();
		self.a.wire_calls.clear();
		self.a.wire_ambiguous_calls.clear();
		for application in &applications {
			let needed = self.wire_requirements_of(application.declaration, &requirements);
			let arguments: Vec<_> = needed
				.into_iter()
				.map(|requirement| {
					(
						requirement,
						self.s.types.substitute(requirement, &application.bindings),
					)
				})
				.collect();
			let subjects: Vec<_> = arguments.iter().map(|&(_, subject)| subject).collect();
			if subjects
				.iter()
				.any(|&subject| unbound_materialization(&self.s.types, application, subject))
			{
				// A raw forall value can hide a different wire ABI from another
				// value with the same signature. Until that value carries a
				// static adapter, reject its materialization in JS explicitly.
				self.a.wire_ambiguous_calls.insert(application.site);
				continue;
			}
			if let Some(known) = self.a.wire_calls.get(&application.site) {
				if known != &subjects {
					self.a.wire_ambiguous_calls.insert(application.site);
				}
			} else {
				self.a.wire_calls.insert(application.site, subjects);
			}
			self.a
				.wire_arguments
				.entry(application.site)
				.or_default()
				.extend(arguments);
		}
		self.s
			.source_file_mut(self.file)
			.interface
			.wire_requirements = requirements.clone();
		self.a.wire_calls.retain(|_, subjects| !subjects.is_empty());
		self.a
			.wire_arguments
			.retain(|_, arguments| !arguments.is_empty());
		self.a.wire_requirements = requirements;
	}

	fn wire_requirements_of(
		&self,
		declaration: DeclKey,
		local: &FxHashMap<DeclKey, Vec<TypeId>>,
	) -> Vec<TypeId> {
		let module = self.s.types.decl_key_data(declaration).module;
		if module == self.s.module_id(self.file) {
			// The current sparse finish is authoritative even when empty.
			// Falling back to this file's previous interface would retain a
			// removed need forever during an incremental reanalysis.
			return local.get(&declaration).cloned().unwrap_or_default();
		}
		self.s
			.interface(module)
			.and_then(|interface| interface.wire_requirements.get(&declaration))
			.cloned()
			.unwrap_or_default()
	}

	fn reject_growing_wire(
		&mut self,
		ty: TypeId,
		expanding: &FxHashMap<Parameter, NodeId>,
	) -> bool {
		if !matches!(self.s.types.kind(ty), TypeKind::Union(_)) {
			return false;
		}
		let at = binding_parameters(&self.s.types, ty)
			.into_iter()
			.filter_map(|(parameter, _)| expanding.get(&parameter).copied())
			.min();
		let Some(at) = at else { return false };
		if self
			.reported_once
			.insert((at, crate::diagnostics::codes::types::GN020.id))
		{
			let message = format!(
				"the binary codec of `{}` grows through this recursive generic call; its concrete union needs infinitely many statically compiled operations: use type arguments that do not grow",
				self.display(ty)
			);
			let diagnostic = self.diagnostic(&crate::diagnostics::codes::types::GN020, at, message);
			self.report(diagnostic);
		}
		true
	}

	fn wire_owner(&self, mut site: NodeId) -> Option<DeclKey> {
		loop {
			if let Some(info) = self.a.function(site) {
				return self.a.callable_keys.get(&info.lambda).copied();
			}
			site = self.parent(site)?;
		}
	}
}

struct Application {
	site: NodeId,
	owner: Option<DeclKey>,
	declaration: DeclKey,
	materialized: bool,
	bindings: Bindings,
}

fn unbound_materialization(
	types: &crate::types::TypeStore,
	application: &Application,
	subject: TypeId,
) -> bool {
	application.materialized && !binding_parameters(types, subject).is_empty()
}

type Parameter = (DeclKey, u16);

fn parameter_key(types: &crate::types::TypeStore, parameter: ParamId) -> Parameter {
	let info = types.param(parameter);
	(info.owner, info.index)
}

/// Edges through union flattening have zero growth; an Array/object/etc.
/// adds structure. A positive edge inside an SCC proves type expansion,
/// without an arbitrary iteration or instance-count limit.
fn expanding_parameters(
	types: &crate::types::TypeStore,
	applications: &[Application],
) -> FxHashMap<Parameter, NodeId> {
	let mut ids = FxHashMap::default();
	let mut parameters = Vec::new();
	let mut links = Vec::new();
	let mut id = |parameter: Parameter| {
		*ids.entry(parameter).or_insert_with(|| {
			parameters.push(parameter);
			parameters.len() - 1
		})
	};
	for application in applications {
		for (&parameter, &actual) in application
			.bindings
			.params
			.iter()
			.zip(&application.bindings.types)
		{
			let from = id(parameter_key(types, parameter));
			for (parameter, grows) in binding_parameters(types, actual) {
				links.push((from, id(parameter), grows, application.site));
			}
		}
	}
	let mut edges = vec![Vec::new(); parameters.len()];
	let mut reverse = vec![Vec::new(); parameters.len()];
	for &(from, to, _, _) in &links {
		edges[from].push(to);
		reverse[to].push(from);
	}
	let components = crate::graph::components(&edges);
	let mut owner = vec![0; parameters.len()];
	for (index, members) in components.iter().enumerate() {
		for &member in members {
			owner[member] = index;
		}
	}
	let mut pending = Vec::new();
	for &(from, to, grows, site) in &links {
		if grows && owner[from] == owner[to] {
			pending.extend(components[owner[from]].iter().map(|&member| (member, site)));
		}
	}
	let mut blocked: FxHashMap<usize, NodeId> = FxHashMap::default();
	while let Some((node, site)) = pending.pop() {
		if blocked.get(&node).is_some_and(|known| *known <= site) {
			continue;
		}
		blocked.insert(node, site);
		pending.extend(reverse[node].iter().map(|&parent| (parent, site)));
	}
	blocked
		.into_iter()
		.map(|(index, site)| (parameters[index], site))
		.collect()
}

/// Parameters in a binding, not those mentioned only in their constraints.
fn binding_parameters(types: &crate::types::TypeStore, root: TypeId) -> Vec<(Parameter, bool)> {
	let mut pending = vec![(root, false)];
	let mut seen = FxHashSet::default();
	let mut result = Vec::new();
	while let Some((ty, grows)) = pending.pop() {
		if ty.is_unset() || !seen.insert((ty, grows)) {
			continue;
		}
		match types.kind(ty) {
			TypeKind::Param(parameter) => result.push((parameter_key(types, parameter), grows)),
			TypeKind::Union(list) => {
				pending.extend(types.list_items(list).iter().map(|&ty| (ty, grows)))
			}
			TypeKind::Tuple(list) | TypeKind::Fields(list) => {
				pending.extend(types.list_items(list).iter().map(|&ty| (ty, true)))
			}
			TypeKind::Object(_) => pending.extend(
				types
					.fields_of(ty)
					.iter()
					.filter(|field| !field.flags.contains(crate::types::FieldFlags::METHOD))
					.map(|field| (field.ty, true)),
			),
			TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
				pending.extend([(item, true), (key, true)])
			}
			TypeKind::Sync(_, item)
			| TypeKind::Pointer(item)
			| TypeKind::CArray(item, _)
			| TypeKind::Ref(item, _)
			| TypeKind::Iterator(item, _) => pending.push((item, true)),
			TypeKind::Rec(_) => pending.extend(types.rec_target(ty).map(|ty| (ty, true))),
			TypeKind::NamedUnion(nominal, _)
			| TypeKind::Member(nominal, _)
			| TypeKind::CStruct(nominal) => {
				let nominal = types.nominal(nominal);
				pending.extend(types.list_items(nominal.args).iter().map(|&ty| (ty, true)));
			}
			TypeKind::Function(sig) => {
				let sig = types.sig(sig);
				pending.extend(
					types
						.params(sig.params)
						.iter()
						.map(|parameter| (parameter.ty, true)),
				);
				pending.extend([(sig.ret, true), (sig.throws, true)]);
			}
			_ => {}
		}
	}
	result
}

fn insert_requirement(
	requirements: &mut FxHashMap<DeclKey, Vec<TypeId>>,
	owner: DeclKey,
	ty: TypeId,
) -> bool {
	let requirements = requirements.entry(owner).or_default();
	if requirements.contains(&ty) {
		return false;
	}
	requirements.push(ty);
	true
}

fn minimal_codec_needs(types: &crate::types::TypeStore, ty: TypeId) -> Vec<TypeId> {
	let mut pending = vec![ty];
	let mut seen = FxHashSet::default();
	let mut result = Vec::new();
	while let Some(ty) = pending.pop() {
		if !seen.insert(ty) || ty.is_unset() || types.is_resource(ty) {
			continue;
		}
		match types.kind(ty) {
			TypeKind::Param(_) | TypeKind::SymInt(_) => result.push(ty),
			TypeKind::Union(_) if !crate::modules::reachable_params(types, [ty]).is_empty() => {
				result.push(ty)
			}
			TypeKind::Object(_) => pending.extend(
				types
					.fields_of(ty)
					.iter()
					.filter(|field| !field.flags.contains(crate::types::FieldFlags::METHOD))
					.map(|field| field.ty),
			),
			TypeKind::Collection(_, item, key) => pending.extend([item, key]),
			TypeKind::Tuple(list) | TypeKind::Union(list) => pending.extend(types.list_items(list)),
			TypeKind::Rec(_) => pending.extend(types.rec_target(ty)),
			TypeKind::Member(nominal, index) => pending.push(types.payload(nominal, index)),
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				pending.extend(
					(0..count)
						.filter(|&index| types.mask_has(mask, index))
						.map(|index| types.payload(nominal, index)),
				);
			}
			_ => {}
		}
	}
	result
}

#[cfg(test)]
mod tests {
	#[test]
	fn codec_parameter_growth_uses_substitution_cycles_not_a_count_limit() {
		use super::{Application, expanding_parameters, minimal_codec_needs};
		use crate::ids::{NodeId, TypeId};
		use crate::types::store::GlobalStore;
		use crate::types::{Bindings, ParamInfo, TypeKind, TypeStore};

		let global = GlobalStore::new();
		let mut types = TypeStore::view(&global);
		let module = types.module("library/wire");
		let declaration = types.decl_key(module, &["grow"], 0);
		let name = types.syms.intern("Item");
		let parameter = types.add_param(ParamInfo {
			owner: declaration,
			index: 0,
			name,
			constraint: TypeId::UNKNOWN,
			default: TypeId::UNSET,
			value: false,
		});
		let item = types.intern(TypeKind::Param(parameter));
		let array = types.array(item);
		let optional = types.optional(item);
		let application = |actual| Application {
			site: NodeId::new(7),
			owner: Some(declaration),
			declaration,
			materialized: false,
			bindings: Bindings {
				params: vec![parameter],
				types: vec![actual],
				..Default::default()
			},
		};
		assert_eq!(
			expanding_parameters(&types, &[application(array)]).get(&(declaration, 0)),
			Some(&NodeId::new(7))
		);
		assert!(expanding_parameters(&types, &[application(optional)]).is_empty());
		assert!(expanding_parameters(&types, &[application(item)]).is_empty());
		// Array factories compose a leaf capability, while a normalized union
		// must arrive as an exact operation for the caller's concrete type.
		assert_eq!(minimal_codec_needs(&types, array), [item]);
		assert_eq!(minimal_codec_needs(&types, optional), [optional]);
	}

	#[test]
	fn changing_only_wire_requirements_reanalyzes_transitive_importers() {
		let plain = "export function = <Item>(_value: Item): Integer => 0\n";
		let transport = "use { Channel } = std.concurrency\nexport function = <Item>(_value: Item): Integer => { mutable _queue = Channel<Item | none> {}\n0 }\n";
		let files = vec![
			("library/source.fa".to_string(), plain.to_string()),
			(
				"library/forward.fa".to_string(),
				"export function = <Element>(value: Element): Integer => source(value)\n"
					.to_string(),
			),
			(
				"application/main.fa".to_string(),
				"export function = () => console.log(library.forward(7))\n".to_string(),
			),
		];
		let mut session = crate::Session::new();
		session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
		session.analyze_package(Some("application/main"));
		let source = session.loaded_module("library/source").unwrap();
		let before = session.source_file(source).interface.hash(session.types());
		assert!(
			session
				.source_file(source)
				.interface
				.wire_requirements
				.is_empty()
		);
		let changed = session.update("library/source", transport.into());
		let modules: Vec<_> = changed.iter().map(|&file| session.module(file)).collect();
		assert!(
			modules.contains(&"library/forward") && modules.contains(&"application/main"),
			"{modules:?}"
		);
		let source = session.loaded_module("library/source").unwrap();
		assert_ne!(
			session.source_file(source).interface.hash(session.types()),
			before
		);
		for module in ["library/source", "library/forward", "application/main"] {
			let file = session.loaded_module(module).unwrap();
			let errors: Vec<_> = session
				.diagnostics(file)
				.iter()
				.filter(|diagnostic| diagnostic.severity == crate::Severity::Error)
				.collect();
			assert!(errors.is_empty(), "{module}: {errors:?}");
			let analysis = session.analysis(file);
			if module != "application/main" {
				assert_eq!(analysis.wire_requirements.len(), 1, "{module}");
				assert!(
					analysis
						.wire_requirements
						.values()
						.all(|types| types.len() == 1)
				);
			} else {
				assert_eq!(analysis.wire_calls.len(), 1);
				let subject = analysis.wire_calls.values().next().unwrap()[0];
				assert!(
					!session
						.types()
						.flags(subject)
						.contains(crate::types::TypeFlags::HAS_PARAM)
				);
			}
		}
		let changed = session.update("library/source", plain.into());
		let modules: Vec<_> = changed.iter().map(|&file| session.module(file)).collect();

		assert!(
			modules.contains(&"library/forward") && modules.contains(&"application/main"),
			"{modules:?}"
		);
		for module in ["library/source", "library/forward", "application/main"] {
			let file = session.loaded_module(module).unwrap();
			assert!(
				session.analysis(file).wire_requirements.is_empty(),
				"{module}"
			);
			assert!(session.analysis(file).wire_calls.is_empty(), "{module}");
		}
	}

	#[test]
	fn generic_wire_needs_follow_calls_without_becoming_universal_type_info() {
		let source = "use { Mutex } = std.concurrency\nfunction holder = <Item>(value: Item): Mutex<Item | none> => Mutex<Item | none> { value = value }\nfunction forward = <Element>(value: Element): Mutex<Element | none> => holder(value)\nfunction identity = <Item>(value: Item): Item => value\nlet cell = forward(7)\nconsole.log(identity(1))\n";
		let (session, file) = crate::analyze_source(source);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|error| error.severity == crate::Severity::Error)
			.collect();
		assert!(errors.is_empty(), "{errors:?}");
		let analysis = session.analysis(file);
		assert_eq!(analysis.wire_requirements.len(), 2);
		assert_eq!(
			analysis
				.wire_arguments
				.keys()
				.filter(|site| analysis.calls.contains_key(site))
				.count(),
			2
		);
		assert!(
			analysis
				.wire_requirements
				.values()
				.all(|types| types.len() == 1)
		);
		let interface = session
			.interface(
				session
					.types()
					.decl_key_data(*analysis.wire_requirements.keys().next().unwrap())
					.module,
			)
			.unwrap();
		assert_eq!(interface.wire_requirements, analysis.wire_requirements);
	}
}
