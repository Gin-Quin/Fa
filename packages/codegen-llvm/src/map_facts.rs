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
	producer_types: BTreeSet<TypeId>,
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
			let parents = facts::parents_of(tree);
			let declared = facts::declaration_lambdas(tree);
			for index in 0..tree.nodes.len() {
				let node = NodeId::new(index);
				let ty = analysis.type_of(node);
				if let TypeKind::Function(sig) = types.kind(ty)
					&& !types.list_items(types.sig(sig).generics).is_empty()
					&& !analysis.has_flags(node, fa_analyzer::analysis::NodeFlags::TYPE_EXPR)
					&& !declared.contains(&node)
					&& matches!(
						view(tree, node),
						View::Name(_) | View::Access { .. } | View::Lambda { .. }
					) && used_as_value(tree, node, &parents)
				{
					self.producer_types.insert(merger.ty(ty));
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
				if (matches!(
					analysis.call(node).map(|info| info.callee),
					Some(Callee::Value)
				) || analysis
					.has_flags(node, fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION))
					&& !is_declaration(session, analysis, tree, callee)
				{
					let ty = analysis.type_of(callee);
					if let TypeKind::Function(sig) = types.kind(ty)
						&& !types.list_items(types.sig(sig).generics).is_empty()
					{
						self.function_types.insert(merger.ty(ty));
					}
				}
				let mut params = match analysis.call(node).map(|info| info.callee) {
					Some(Callee::Function(decl)) => {
						analysis.function(decl).map(|info| info.generics.clone())
					}
					_ => None,
				}
				.unwrap_or_else(|| function_params(session, analysis, tree, callee));
				let mut prefix_params = Vec::new();
				let mut prefix_args = Vec::new();
				if let Some(owner) = method_owner(analysis, tree, callee) {
					prefix_params = owner_params(session, owner.declaration);
					prefix_args = types.list_items(owner.arguments).to_vec();
					params.retain(|param| !prefix_params.contains(param));
				}
				if params.len() <= given.len() {
					prefix_args.extend_from_slice(&given[given.len() - params.len()..]);
					prefix_params.extend(params);
					self.application(types, &prefix_params, &prefix_args, &mut merger);
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
		for &left in &self.producer_types {
			for &right in &self.function_types {
				if left != right && same_function_class(&mut scratch, left, right) {
					let own = function_generics(&scratch, left);
					let others = function_generics(&scratch, right);
					let binders = own
						.iter()
						.map(|&param| {
							let info = scratch.param(param);
							(info.owner, info.index)
						})
						.collect();
					let arguments = others
						.iter()
						.map(|&param| scratch.intern(TypeKind::Param(param)))
						.collect();
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
				if arguments.iter().all(|&ty| {
					parameters(&scratch, ty)
						.iter()
						.all(|&param| active.contains(&scratch.param(param).owner))
				}) {
					active.extend(binders.iter().map(|binder| binder.0));
				}
			}
			if active.len() == before {
				break;
			}
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
						if let Some(name) = facts::map_key(&scratch, ty) {
							found.insert(name);
						}
						continue;
					}
					let needed: Vec<_> = params
						.iter()
						.map(|&param| {
							let info = scratch.param(param);
							(info.owner, info.index)
						})
						.collect();
					let coverage = |binders: &[Binder]| {
						needed
							.iter()
							.filter(|binder| binders.contains(binder))
							.count()
					};
					let most = applications
						.iter()
						.filter(|(binders, _)| {
							binders.iter().all(|binder| active.contains(&binder.0))
						})
						.map(|(binders, _)| coverage(binders))
						.max()
						.unwrap_or(0);
					for (binders, arguments) in &applications {
						if coverage(binders) < most {
							continue;
						}
						if !binders.iter().all(|binder| active.contains(&binder.0)) {
							continue;
						}
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

/// Declaration lambdas are excluded separately; operands of calls/accesses
/// are handled by their containing expression rather than as producers.
fn used_as_value(tree: &fa_parser::TypedSyntaxTree, node: NodeId, parents: &[usize]) -> bool {
	let mut at = node;
	while let Some(&parent) = parents
		.get(at.index())
		.filter(|&&parent| parent != usize::MAX)
	{
		match view(tree, NodeId::new(parent)) {
			View::Group(_) => at = NodeId::new(parent),
			View::Call { callee, .. } if callee == at => return false,
			View::Access { .. } | View::OptionalAccess { .. } => return false,
			_ => return true,
		}
	}
	true
}

fn function_node(
	analysis: &Analysis,
	tree: &fa_parser::TypedSyntaxTree,
	mut node: NodeId,
) -> NodeId {
	loop {
		match view(tree, node) {
			View::Group(inner) => node = inner,
			View::Call { callee, .. }
				if analysis
					.has_flags(node, fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION) =>
			{
				node = callee
			}
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				let Some(last) = members.last() else {
					return node;
				};
				node = last;
			}
			_ => break,
		}
	}
	node
}

fn is_declaration(
	session: &Session,
	analysis: &Analysis,
	tree: &fa_parser::TypedSyntaxTree,
	node: NodeId,
) -> bool {
	let node = function_node(analysis, tree, node);
	match analysis.resolution(node) {
		Resolution::Local(symbol) => matches!(
			analysis.symbols.symbol(symbol).kind,
			fa_analyzer::symbols::SymbolKind::Function
		),
		Resolution::External(key) => session.keyed(key).is_some_and(|(module, decl)| {
			session
				.interface(module)
				.is_some_and(|interface| interface.generic_functions.contains_key(&decl.node))
		}),
		_ => false,
	}
}

fn method_owner<'a>(
	analysis: &'a Analysis,
	tree: &fa_parser::TypedSyntaxTree,
	mut callee: NodeId,
) -> Option<&'a fa_analyzer::analysis::WrittenType> {
	loop {
		match view(tree, callee) {
			View::Group(inner) => callee = inner,
			View::Call { callee: inner, .. }
				if analysis.has_flags(
					callee,
					fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION,
				) =>
			{
				callee = inner
			}
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				return analysis.method_instantiations.get(&members.last()?);
			}
			_ => return None,
		}
	}
}

fn function_params(
	session: &Session,
	analysis: &Analysis,
	tree: &fa_parser::TypedSyntaxTree,
	node: NodeId,
) -> Vec<ParamId> {
	let node = function_node(analysis, tree, node);
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
			facts::map_component(types, ty).unwrap_or_else(|| "*".into())
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

fn function_generics(types: &TypeStore, ty: TypeId) -> Vec<ParamId> {
	let TypeKind::Function(sig) = types.kind(ty) else {
		return Vec::new();
	};
	type_params(types, types.list_items(types.sig(sig).generics))
}

/// The alpha-equivalence used by native generic function dispatch tables.
pub(crate) fn same_function_class(types: &mut TypeStore, left: TypeId, right: TypeId) -> bool {
	let own = function_generics(types, left);
	let theirs = function_generics(types, right);
	if own.len() != theirs.len() {
		return false;
	}
	if left == right {
		return true;
	}
	let arguments: Vec<_> = theirs
		.iter()
		.map(|&param| types.intern(TypeKind::Param(param)))
		.collect();
	let renamed = types.substitute(
		left,
		&Bindings {
			params: own,
			types: arguments.clone(),
			..Default::default()
		},
	);
	let opened = types.substitute(
		right,
		&Bindings {
			params: theirs,
			types: arguments,
			..Default::default()
		},
	);
	renamed == opened
}

#[cfg(test)]
mod tests {
	use fa_analyzer::types::CollKind;

	use super::*;

	#[test]
	fn resolved_map_facts_keep_scalar_views_and_concrete_applications_consistent() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function retain = <Key, Item>(entries: Map<Key, Item>, key: Key, value: Item) mutates entries => {
 use slot = entries.getOrInsert(copy key, copy value)
 entries[key] = copy value
 slot = copy value
}
mutable entries: Map<String, Integer> = []
retain(entries, "key", 7)
"#,
		);
		assert!(session.analysis(file).diagnostics.is_empty());
		let mut complete = ProgramFacts::of_unit(&session, &[file]);
		complete.resolve_maps(facts::map_facts(&session, &[file]));
		let mut types = TypeStore::new();
		let integer = types.collection(CollKind::Map, TypeId::INTEGER, TypeId::STRING);
		let int64 = types.collection(CollKind::Map, TypeId::INT64, TypeId::STRING);
		let unrelated = types.collection(CollKind::Map, TypeId::STRING, TypeId::STRING);
		for ty in [integer, int64] {
			assert!(facts::map_excluded(&types, ty, |key| complete.contains(REFERENCED_MAPS, key)));
		}
		assert!(!facts::map_excluded(&types, unrelated, |key| complete
			.contains(REFERENCED_MAPS, key)));
	}

	#[test]
	fn application_tuples_preserve_correlated_keys_and_values() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function retain = <Key, Item>(entries: Map<Key, Item>, key: Key, value: Item) mutates entries => {
 use slot = entries.getOrInsert(copy key, copy value)
 entries[key] = copy value
 slot = copy value
}
mutable a: Map<String, Int64> = []
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
		assert!(found.contains("Map<String, Int64>"), "{found:?}");
		assert!(found.contains("Map<Int8, String>"), "{found:?}");
		assert!(!found.contains("Map<String, String>"), "{found:?}");
		assert!(!found.contains("Map<Int8, Int64>"), "{found:?}");
	}

	#[test]
	fn unused_generic_restrictions_leave_other_maps_optimized() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function unused = <Item>(entries: Map<String, Item>, value: Item) mutates entries => {
 use slot = entries.getOrInsert("key", copy value)
 entries["other"] = copy value
 slot = copy value
 unused(entries, value)
}
function used = <Item>(entries: Map<String, Item>, value: Item) mutates entries => { entries["key"] = copy value }
mutable a: Map<String, Int64> = []
used(a, 7)
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
				.contains("Map<String, Int64>")
		);
		assert!(facts::FactLog::still_holds(&log.to_text(), Some(&facts)));
		facts.extend(
			REFERENCED_MAPS,
			BTreeSet::from(["\u{1f}map-family:String\u{1e}*".into()]),
		);
		assert!(!facts::FactLog::still_holds(&log.to_text(), Some(&facts)));
		assert!(!facts.contains(REFERENCED_MAPS, "Map<Int8, Int64>"));
	}

	#[test]
	fn generic_deletion_restricts_only_its_instantiated_map() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function prune = <Key, Item>(entries: Map<Key, Item>) mutates entries => entries.deleteWhere((_key, _value) => true)
mutable a: Map<Int8, Int16> = []
prune(a)
mutable b: Map<Int16, Int8> = []
b[Int16(1)] = Int8(7)
"#,
		);
		assert!(
			session.analysis(file).diagnostics.is_empty(),
			"{:?}",
			session.analysis(file).diagnostics
		);
		let facts = facts::map_facts(&session, &[file]);
		assert!(
			facts.contains(DELETED_WHILE_ITERATED_MAPS, "Map<Int8, Int16>"),
			"{facts:?}"
		);
		assert!(
			!facts.contains(DELETED_WHILE_ITERATED_MAPS, "Map<Int16, Int8>"),
			"{facts:?}"
		);
	}

	#[test]
	fn indirect_demands_reach_only_actual_function_value_producers() {
		let (session, file) = fa_analyzer::session::analyze_source(
			r#"
function pause = () => {}
function read = <Item>(entries: Map<String, Item>): Item? => {
 use slot = entries.get("key")
 pause()
 if slot is none { none } else { copy slot }
}
function invoke = (action: <Value>(entries: Map<String, Value>) => Value?, entries: Map<String, Int64>): Integer => action(entries) ?? 0
let entries: Map<String, Int64> = [("key", 9)]
let result = invoke(read, entries)
"#,
		);
		assert!(
			session.analysis(file).diagnostics.is_empty(),
			"{:?}",
			session.analysis(file).diagnostics
		);
		let facts = facts::map_facts(&session, &[file]);
		assert!(
			facts.contains(REFERENCED_MAPS, "Map<String, Int64>"),
			"{facts:?}"
		);
	}

	#[test]
	fn method_owner_and_own_arguments_stay_correlated() {
		let (session, file) = fa_analyzer::session::analyze_source(include_str!(
			"../../analyzer/tests/cases/references/review-links-generic-map-method.fa"
		));
		assert!(
			session.analysis(file).diagnostics.is_empty(),
			"{:?}",
			session.analysis(file).diagnostics
		);
		let facts = facts::map_facts(&session, &[file]);
		assert!(
			facts.contains(REFERENCED_MAPS, "Map<String, Int64>"),
			"{facts:?}"
		);
		assert!(
			facts.contains(REFERENCED_MAPS, "Map<Int8, String>"),
			"{facts:?}"
		);
		assert!(
			!facts.contains(REFERENCED_MAPS, "Map<String, String>"),
			"{facts:?}"
		);
		assert!(
			!facts.contains(REFERENCED_MAPS, "Map<Int8, Int64>"),
			"{facts:?}"
		);
	}
}
