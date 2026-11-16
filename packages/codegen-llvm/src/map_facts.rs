//! Resolve map-use templates before any constructor chooses its layout.
//!
//! A unit retains global types and application tuples, not its analysis. A
//! parameter is identified by declaration and position: two mergers can give
//! the same exported binder different ids. Substitution always uses types,
//! never printed names (notably, substitution re-normalizes unions).

use std::collections::{BTreeSet, VecDeque};

use fa_analyzer::analysis::{Analysis, Callee, GenericMethod};
use fa_analyzer::ids::{DeclKey, FileId, NodeId, ParamId, TypeId};
use fa_analyzer::session::Session;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{View, view};
use fa_analyzer::types::{Bindings, TypeFlags, TypeKind, TypeStore};

use crate::facts::{self, DELETED_WHILE_ITERATED_MAPS, ProgramFacts, REFERENCED_MAPS};

type Binder = (DeclKey, u16);

#[derive(Default)]
pub(crate) struct MapFacts {
	referenced: BTreeSet<TypeId>,
	deleted: BTreeSet<TypeId>,
	applications: Vec<(Vec<Binder>, Vec<TypeId>)>,
	function_types: BTreeSet<TypeId>,
}

impl MapFacts {
	/// Called while this unit's local type identities are still available.
	pub(crate) fn collect(&mut self, session: &Session, files: &[FileId]) {
		let types = session.types();
		let mut merger = types.merger(None);
		self.referenced.extend(
			facts::referenced_map_types(session, files)
				.into_iter()
				.map(|ty| merger.ty(ty)),
		);
		self.deleted.extend(
			facts::deleted_while_iterated_map_types(session, files)
				.into_iter()
				.map(|ty| merger.ty(ty)),
		);
		for &file in files {
			let tree = session.tree(file);
			let analysis = session.analysis(file);
			for index in 0..tree.nodes.len() {
				let ty = analysis.type_of(NodeId::new(index));
				if let TypeKind::Function(sig) = types.kind(ty)
					&& !types.list_items(types.sig(sig).generics).is_empty() {
					self.function_types.insert(merger.ty(ty));
				}
			}
			for (&node, &arguments) in &analysis.instantiations {
				let mut callee = match view(tree, node) {
					View::Call { callee, .. } => callee,
					_ => node,
				};
				while let View::Group(inner) = view(tree, callee) {
					callee = inner;
				}
				let given = types.list_items(arguments);
				let params = match analysis.call(node).map(|info| info.callee) {
					Some(Callee::Function(decl)) => {
						analysis.function(decl).map(|info| info.generics.clone())
					}
					_ => None,
				}
				.unwrap_or_else(|| function_params(session, analysis, tree, callee));
				// Method-owner parameters are recorded separately below. The
				// call's instantiation list contains the method's own arguments.
				if params.len() <= given.len() {
					self.application(
						types,
						&params,
						&given[given.len() - params.len()..],
						&mut merger,
					);
				}
			}
			for owner in analysis.method_instantiations.values() {
				let params = owner_params(session, owner.declaration);
				self.application(
					types,
					&params,
					types.list_items(owner.arguments),
					&mut merger,
				);
			}
			for methods in analysis.instantiated_methods.values() {
				for method in methods {
					self.method(session, method, &mut merger);
				}
			}
		}
	}

	fn method(
		&mut self,
		session: &Session,
		method: &GenericMethod,
		merger: &mut fa_analyzer::types::merge::Merger<'_>,
	) {
		let module = session.types().decl_key_data(method.function).module;
		if let Some(owner) = session.interface(module).and_then(|interface| {
			let node = interface.node_of(method.function)?;
			interface.function_owners.get(&node).copied()
		}) {
			let params = owner_params(session, owner);
			self.application(
				session.types(),
				&params,
				session.types().list_items(method.arguments),
				merger,
			);
		}
		for method in method.methods.iter() {
			self.method(session, method, merger);
		}
	}

	fn application(
		&mut self,
		types: &TypeStore,
		params: &[ParamId],
		arguments: &[TypeId],
		merger: &mut fa_analyzer::types::merge::Merger<'_>,
	) {
		if params.is_empty() || params.len() != arguments.len() {
			return;
		}
		let binders = params
			.iter()
			.map(|&param| {
				let info = types.param(param);
				(merger.decl_key(info.owner), info.index)
			})
			.collect();
		let arguments = arguments.iter().map(|&ty| merger.ty(ty)).collect();
		self.applications.push((binders, arguments));
	}

	/// Contributions from different units share the pipeline's global store.
	pub(crate) fn resolve(&self, types: &TypeStore) -> ProgramFacts {
		let mut scratch = TypeStore::view(types.global());
		let mut applications = self.applications.clone();
		// An indirect demand applies to every producer of the same generic
		// function class, exactly as the generated dispatch tables do.
		for &left in &self.function_types {
			for &right in &self.function_types {
				if left != right && same_function_class(&mut scratch, left, right) {
					let own = function_generics(&scratch, left);
					let others = function_generics(&scratch, right);
					let binders = own.iter().map(|&param| { let info = scratch.param(param); (info.owner, info.index) }).collect();
					let arguments = others.iter().map(|&param| scratch.intern(TypeKind::Param(param))).collect();
					applications.push((binders, arguments));
				}
			}
		}
		// Symbolic recursive calls are not seeds: an unused generic function
		// must not change any concrete map's policy.
		let mut active = BTreeSet::new();
		loop {
			let before = active.len();
			for (binders, arguments) in &applications {
				if arguments.iter().all(|&ty| parameters(&scratch, ty).iter().all(|&param| active.contains(&scratch.param(param).owner))) {
					active.extend(binders.iter().map(|binder| binder.0));
				}
			}
			if active.len() == before { break; }
		}
		let mut result = ProgramFacts::default();
		for (kind, templates) in [
			(REFERENCED_MAPS, &self.referenced),
			(DELETED_WHILE_ITERATED_MAPS, &self.deleted),
		] {
			let mut found = BTreeSet::new();
			for &template in templates {
				let mut pending = VecDeque::from([(template, 0)]);
				let mut seen = BTreeSet::new();
				while let Some((ty, depth)) = pending.pop_front() {
					if !seen.insert(ty) {
						continue;
					}
					let params = parameters(&scratch, ty);
					if params.is_empty() {
						if let Some(name) = facts::stable_type_name(&scratch, ty) {
							found.insert(name);
						}
						continue;
					}
					for (binders, arguments) in &applications {
						if !binders.iter().all(|binder| active.contains(&binder.0)) { continue; }
						let mut bindings = Bindings::default();
						for &param in &params {
							let info = scratch.param(param);
							if let Some(index) = binders
								.iter()
								.position(|binder| *binder == (info.owner, info.index))
							{
								bindings.params.push(param);
								bindings.types.push(arguments[index]);
							}
						}
						if bindings.params.is_empty() {
							continue;
						}
						let next = scratch.substitute(ty, &bindings);
						if next == ty {
							continue;
						}
						if depth >= 64 || seen.len() >= 2048 {
							// Polymorphic recursion must not silently leave later
							// instances dense. Widen only this affected family.
							found.insert(family(&scratch, next));
						} else {
							pending.push_back((next, depth + 1));
						}
					}
				}
			}
			result.extend(kind, found);
		}
		result
	}
}

fn function_params(
	session: &Session,
	analysis: &Analysis,
	tree: &fa_parser::TypedSyntaxTree,
	mut node: NodeId,
) -> Vec<ParamId> {
	loop {
		match view(tree, node) {
			View::Group(inner) => node = inner,
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				if let Some(last) = members.last() {
					node = last;
				} else {
					break;
				}
			}
			_ => break,
		}
	}
	let types = session.types();
	if let Resolution::External(key) = analysis.resolution(node)
		&& let Some((module, keyed)) = session.keyed(key)
		&& let Some(function) = session
			.interface(module)
			.and_then(|interface| interface.generic_functions.get(&keyed.node))
	{
		return type_params(types, types.list_items(function.generics));
	}
	if let TypeKind::Function(sig) = types.kind(analysis.type_of(node)) {
		return type_params(types, types.list_items(types.sig(sig).generics));
	}
	Vec::new()
}

fn owner_params(session: &Session, owner: DeclKey) -> Vec<ParamId> {
	let types = session.types();
	let module = types.decl_key_data(owner).module;
	session
		.interface(module)
		.and_then(|interface| interface.type_parameters.get(&interface.node_of(owner)?))
		.map(|&list| type_params(types, types.list_items(list)))
		.unwrap_or_default()
}

fn type_params(types: &TypeStore, list: &[TypeId]) -> Vec<ParamId> {
	list.iter()
		.filter_map(|&ty| match types.kind(ty) {
			TypeKind::Param(param) => Some(param),
			_ => None,
		})
		.collect()
}

fn parameters(types: &TypeStore, ty: TypeId) -> Vec<ParamId> {
	let mut found = Vec::new();
	let mut bound = BTreeSet::new();
	let mut pending = vec![ty];
	let mut seen = BTreeSet::new();
	while let Some(ty) = pending.pop() {
		if !seen.insert(ty) || !types.flags(ty).contains(TypeFlags::HAS_PARAM) {
			continue;
		}
		match types.kind(ty) {
			TypeKind::Param(param) => {
				if !found.contains(&param) {
					found.push(param);
				}
			}
			TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
				pending.extend([item, key])
			}
			TypeKind::Tuple(list) | TypeKind::Union(list) | TypeKind::Fields(list) => {
				pending.extend(types.list_items(list))
			}
			TypeKind::Ref(item, _)
			| TypeKind::Pointer(item)
			| TypeKind::Iterator(item, _)
			| TypeKind::Sync(_, item)
			| TypeKind::CArray(item, _) => pending.push(item),
			TypeKind::Object(_) => pending.extend(types.fields_of(ty).iter().map(|field| field.ty)),
			TypeKind::Rec(rec) => pending.extend(types.list_items(types.rec_info(rec).args)),
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				pending.extend(types.list_items(types.nominal(nominal).args))
			}
			TypeKind::Function(sig) => {
				let sig = types.sig(sig);
				bound.extend(type_params(types, types.list_items(sig.generics)));
				pending.extend(types.params(sig.params).iter().map(|param| param.ty));
				pending.extend([sig.ret, sig.throws]);
			}
			_ => {}
		}
	}
	found.retain(|param| !bound.contains(param));
	found
}

fn family(types: &TypeStore, ty: TypeId) -> String {
	let TypeKind::Collection(_, item, key) = types.kind(ty) else {
		unreachable!("map template");
	};
	let component = |ty| {
		if parameters(types, ty).is_empty() {
			facts::stable_type_name(types, ty).unwrap_or_else(|| "*".into())
		} else {
			"*".into()
		}
	};
	format!(
		"\u{1f}map-family:{}\u{1e}{}",
		component(key),
		component(item)
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn application_tuples_preserve_correlated_keys_and_values() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function retain = <Key, Item>(entries: Map<Key, Item>, key: Key, value: Item) mutates entries => {
 use slot = entries.getOrInsert(copy key, copy value)
 entries[key] = copy value
 slot = copy value
}
mutable a: Map<String, Integer> = []
retain(a, "key", 7)
mutable b: Map<Int8, String> = []
retain(b, Int8(1), "text")
"#,
		);
		assert!(
			session.analysis(file).diagnostics.is_empty(),
			"{:?}",
			session.analysis(file).diagnostics
		);
		let facts = facts::map_facts(&session, &[file]);
		let found = facts.set(REFERENCED_MAPS).unwrap();
		assert!(found.contains("Map<String, Integer>"), "{found:?}");
		assert!(found.contains("Map<Int8, String>"), "{found:?}");
		assert!(!found.contains("Map<String, String>"), "{found:?}");
		assert!(!found.contains("Map<Int8, Integer>"), "{found:?}");
	}

	#[test]
	fn unused_generic_restrictions_leave_other_maps_optimized() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function unused = <Item>(entries: Map<String, Item>, value: Item) mutates entries => {
 use slot = entries.getOrInsert("key", copy value)
 entries["other"] = copy value
 slot = copy value
}
mutable a: Map<String, Integer> = []
a["key"] = 7
"#,
		);
		let facts = facts::map_facts(&session, &[file]);
		assert!(facts.set(REFERENCED_MAPS).unwrap().is_empty(), "{facts:?}");
	}

	#[test]
	fn family_matching_invalidates_cached_absence() {
		let mut facts = ProgramFacts::default();
		let log = facts::FactLog::default();
		assert!(
			!facts
				.consulted(REFERENCED_MAPS, Some(&log))
				.contains("Map<String, Integer>")
		);
		assert!(facts::FactLog::still_holds(&log.to_text(), Some(&facts)));
		facts.extend(
			REFERENCED_MAPS,
			BTreeSet::from(["\u{1f}map-family:String\u{1e}*".into()]),
		);
		assert!(!facts::FactLog::still_holds(&log.to_text(), Some(&facts)));
		assert!(!facts.contains(REFERENCED_MAPS, "Map<Int8, Integer>"));
	}
}

fn function_generics(types: &TypeStore, ty: TypeId) -> Vec<ParamId> {
	let TypeKind::Function(sig) = types.kind(ty) else { return Vec::new(); };
	type_params(types, types.list_items(types.sig(sig).generics))
}

/// The alpha-equivalence used by native generic function dispatch tables.
pub(crate) fn same_function_class(types: &mut TypeStore, left: TypeId, right: TypeId) -> bool {
	let own = function_generics(types, left);
	let theirs = function_generics(types, right);
	if own.len() != theirs.len() { return false; }
	if left == right { return true; }
	let arguments: Vec<_> = theirs.iter().map(|&param| types.intern(TypeKind::Param(param))).collect();
	let renamed = types.substitute(left, &Bindings { params: own, types: arguments.clone(), ..Default::default() });
	let opened = types.substitute(right, &Bindings { params: theirs, types: arguments, ..Default::default() });
	renamed == opened
}
