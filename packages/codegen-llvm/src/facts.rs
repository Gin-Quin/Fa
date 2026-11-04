//! Whole-program facts for builds per unit (`design/parallel-pipeline.md`,
//! "Whole-program facts").
//!
//! A layout or a lowering may depend on what the whole program does with a
//! type (dense maps: nothing keeps a reference into the values of maps of
//! that type). A program lowered as one module computes such facts from all
//! of its files. Per unit, no session sees every body: each unit's analysis
//! computes the facts of its own files ([`ProgramFacts::of_unit`], called
//! by the driver's unit sink), keyed by stable names; the driver merges the
//! facts of every unit ([`ProgramFacts::merge`]) and gives the result to the
//! lowering of every unit, instance wave and entry (`SplitOptions::facts`),
//! and hashes it into each object's cache key, so that a cached object never
//! keeps a layout the program changed. Deterministic: sets of strings,
//! merged in any order to the same result.
//!
//! A fact kind is a [`FactKind`]: a name, what a unit contributes, and how
//! two contributions merge. Its keys are stable names: the same in every
//! unit's session ([`stable_type_name`] for types).

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::ids::{FileId, NodeId, TypeId};
use fa_analyzer::session::Session;
use fa_analyzer::syntax::{View, view};
use fa_analyzer::types::{CollKind, TypeKind, TypeStore};

/// A kind of whole-program fact.
pub struct FactKind {
	/// Its name: the key of its set in [`ProgramFacts`].
	pub name: &'static str,
	/// The keys a unit contributes, from its analyzed files.
	pub collect: fn(&Session, &[FileId]) -> BTreeSet<String>,
	/// Merges a unit's contribution into what is merged so far.
	pub merge: fn(&mut BTreeSet<String>, BTreeSet<String>),
}

/// A set merged by union: a fact that holds when one unit makes it hold
/// (a "may" fact).
pub fn union(merged: &mut BTreeSet<String>, contribution: BTreeSet<String>) {
	merged.extend(contribution);
}

/// The map types whose values something may keep a reference into
/// (`design/decisions.md`, "Dense maps"): their entries stay in stable chunks.
pub const REFERENCED_MAPS: &str = "maps.referenced";
/// The map types some `deleteWhere` iterates over: their entries stay out of
/// the probe array.
pub const DELETED_WHILE_ITERATED_MAPS: &str = "maps.deleted-while-iterated";

/// Every fact kind, in a fixed order.
pub const KINDS: &[FactKind] = &[
	FactKind {
		name: REFERENCED_MAPS,
		collect: referenced_maps,
		merge: union,
	},
	FactKind {
		name: DELETED_WHILE_ITERATED_MAPS,
		collect: deleted_while_iterated_maps,
		merge: union,
	},
];

/// The merged facts of a program (or one unit's contribution).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProgramFacts {
	facts: BTreeMap<&'static str, BTreeSet<String>>,
}

impl ProgramFacts {
	/// The facts the files of one analyzed unit contribute.
	pub fn of_unit(session: &Session, files: &[FileId]) -> Self {
		let mut facts = BTreeMap::new();
		for kind in KINDS {
			facts.insert(kind.name, (kind.collect)(session, files));
		}
		ProgramFacts { facts }
	}

	/// Merges another unit's facts, kind by kind.
	pub fn merge(&mut self, other: ProgramFacts) {
		for kind in KINDS {
			let contribution = other.facts.get(kind.name).cloned().unwrap_or_default();
			(kind.merge)(self.facts.entry(kind.name).or_default(), contribution);
		}
	}

	/// Whether the set of a kind holds a key.
	pub fn contains(&self, kind: &str, key: &str) -> bool {
		self.facts.get(kind).is_some_and(|set| set.contains(key))
	}

	/// A stable hash of every fact (object cache keys).
	pub fn hash_into(&self, hasher: &mut impl Hasher) {
		for (kind, keys) in &self.facts {
			kind.hash(hasher);
			keys.len().hash(hasher);
			for key in keys {
				key.hash(hasher);
			}
		}
	}
}

/// The name of a type that is the same in the session of every unit, if it
/// has one: a canonical spelling built from declaration identities
/// (`DeclKey`: module path, enclosing names, disambiguator, as the symbols of
/// units are named) and type arguments, independent of how a unit imported
/// or aliased the type. Anonymous structural types are spelled from their
/// parts (an object's fields, already sorted by name; a union's members,
/// sorted); a recursive reference is `^n` (n levels up). Built-in scalars are
/// spelled as Fa writes them. `None` for a type without a spelling that is
/// stable (functions, references, iterators, tasks, parameters…).
pub fn stable_type_name(types: &TypeStore, ty: TypeId) -> Option<String> {
	let mut out = String::new();
	let mut stack = Vec::new();
	write_stable(types, ty, &mut stack, &mut out).then_some(out)
}

/// A declaration's identity: `fa.<module path>.<names>[#<n>]`.
fn decl_spelling(types: &TypeStore, key: fa_analyzer::ids::DeclKey) -> String {
	let data = types.decl_key_data(key);
	let mut name = format!("fa.{}", types.module_path(data.module));
	for &sym in types.sym_list(data.names) {
		name.push('.');
		name.push_str(types.syms.resolve(sym));
	}
	if data.disambiguator != 0 {
		name.push_str(&format!("#{}", data.disambiguator));
	}
	name
}

/// Writes the spelling of a nominal type: its declaration, then its type
/// arguments.
fn write_nominal(
	types: &TypeStore,
	nominal: fa_analyzer::ids::NominalId,
	stack: &mut Vec<TypeId>,
	out: &mut String,
) -> bool {
	let info = types.nominal(nominal);
	out.push_str(&decl_spelling(types, info.key));
	let args = types.list_items(info.args).to_vec();
	if !args.is_empty() {
		out.push('<');
		for (index, arg) in args.into_iter().enumerate() {
			if index > 0 {
				out.push_str(", ");
			}
			if !write_stable(types, arg, stack, out) {
				return false;
			}
		}
		out.push('>');
	}
	true
}

fn write_stable(types: &TypeStore, ty: TypeId, stack: &mut Vec<TypeId>, out: &mut String) -> bool {
	if let Some(depth) = stack.iter().rev().position(|&outer| outer == ty) {
		out.push_str(&format!("^{depth}"));
		return true;
	}
	stack.push(ty);
	let written = match types.kind(ty) {
		TypeKind::Boolean
		| TypeKind::BoolLit(_)
		| TypeKind::Int(_)
		| TypeKind::Float(_)
		| TypeKind::FloatLit(..)
		| TypeKind::BigInt
		| TypeKind::BigIntLit(_)
		| TypeKind::Percentage
		| TypeKind::String
		| TypeKind::StringLit(_)
		| TypeKind::Character
		| TypeKind::None
		| TypeKind::Void
		| TypeKind::Never
		| TypeKind::Range => {
			out.push_str(&types.display(ty));
			true
		}
		TypeKind::Object(shape) => match types.shape(shape).nominal {
			Some(nominal) => write_nominal(types, nominal, stack, out),
			None => {
				let fields: Vec<(String, TypeId)> = types
					.shape_fields(shape)
					.iter()
					.map(|field| (types.syms.resolve(field.name).to_string(), field.ty))
					.collect();
				out.push('{');
				let mut written = true;
				for (index, (name, field)) in fields.into_iter().enumerate() {
					if index > 0 {
						out.push_str(", ");
					}
					out.push_str(&name);
					out.push_str(": ");
					written &= write_stable(types, field, stack, out);
				}
				out.push('}');
				written
			}
		},
		TypeKind::Tuple(list) => {
			let items = types.list_items(list).to_vec();
			out.push_str("Tuple<");
			let mut written = true;
			for (index, item) in items.into_iter().enumerate() {
				if index > 0 {
					out.push_str(", ");
				}
				written &= write_stable(types, item, stack, out);
			}
			out.push('>');
			written
		}
		TypeKind::Union(list) => {
			let mut members = Vec::new();
			let mut written = true;
			for member in types.list_items(list).to_vec() {
				let mut spelled = String::new();
				written &= write_stable(types, member, stack, &mut spelled);
				members.push(spelled);
			}
			members.sort();
			out.push('(');
			out.push_str(&members.join(" | "));
			out.push(')');
			written
		}
		TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
			let written = write_nominal(types, nominal, stack, out);
			// the members it holds, by index (a mask's id is the session's)
			let count = types.sym_list(types.nominal(nominal).members).len() as u16;
			let held: Vec<String> = (0..count)
				.filter(|&index| types.mask_has(mask, index))
				.map(|index| index.to_string())
				.collect();
			out.push_str(&format!("/{}", held.join(",")));
			written
		}
		TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) => {
			let written = write_nominal(types, nominal, stack, out);
			out.push_str(&format!(".{index}"));
			written
		}
		TypeKind::CStruct(nominal) => write_nominal(types, nominal, stack, out),
		TypeKind::Collection(kind, item, key) => {
			out.push_str(&format!("{kind:?}<"));
			let mut written = true;
			if matches!(kind, CollKind::Map | CollKind::GrowingMap) {
				written &= write_stable(types, key, stack, out);
				out.push_str(", ");
			}
			written &= write_stable(types, item, stack, out);
			out.push('>');
			written
		}
		_ => false,
	};
	stack.pop();
	written
}

/// Whether a type is a plain `Map`.
fn is_map(types: &TypeStore, ty: TypeId) -> bool {
	matches!(types.kind(ty), TypeKind::Collection(CollKind::Map, _, _))
}

/// The key of a map type in a fact set: its stable name, or `None` (a map
/// type without one is never dense in a build per unit).
fn map_key(types: &TypeStore, ty: TypeId) -> Option<String> {
	if !is_map(types, ty) {
		return None;
	}
	stable_type_name(types, ty)
}

/// See [`REFERENCED_MAPS`]: a node of the map type in the value of a `use`,
/// `reuse` or `provide`, in an argument of a call of a function of the
/// program other than as the whole argument, or in a lambda.
fn referenced_maps(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let types = session.types();
	let mut found = BTreeSet::new();
	for &file in files {
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let mark = |root: NodeId, skip: Option<NodeId>, found: &mut BTreeSet<String>| {
			let mut stack = vec![root.index()];
			while let Some(at) = stack.pop() {
				let node = NodeId::new(at);
				if Some(node) != skip
					&& let Some(key) = map_key(types, analysis.type_of(node))
				{
					found.insert(key);
				}
				tree.nodes[at].for_each_child(|child| stack.push(child));
			}
		};
		let declared = declaration_lambdas(tree);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			match view(tree, node) {
				View::Use { value, .. }
				| View::Reuse { value, .. }
				| View::Provide { value, .. } => {
					mark(value, None, &mut found);
				}
				View::Lambda { .. } if !declared.contains(&node) => mark(node, None, &mut found),
				View::Call { .. } => {
					let Some(info) = analysis.call(node) else {
						continue;
					};
					if matches!(
						info.callee,
						Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_)
					) {
						continue;
					}
					for argument in &info.arguments {
						if let Argument::Given(value) = argument {
							mark(*value, Some(*value), &mut found);
						}
					}
				}
				_ => {}
			}
		}
	}
	found
}

/// See [`DELETED_WHILE_ITERATED_MAPS`].
fn deleted_while_iterated_maps(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let types = session.types();
	let mut found = BTreeSet::new();
	for &file in files {
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			let View::Call { callee, .. } = view(tree, node) else {
				continue;
			};
			let Some(Callee::Builtin(id)) = analysis.call(node).map(|info| info.callee) else {
				continue;
			};
			if !fa_analyzer::builtins::get(id)
				.path
				.ends_with("#deleteWhere")
			{
				continue;
			}
			if let View::Access { target, .. } = view(tree, callee)
				&& let Some(key) = map_key(types, analysis.type_of(target))
			{
				found.insert(key);
			}
		}
	}
	found
}

/// The lambdas that are the bodies of named functions (`function f = (…) =>
/// …`, methods, exported functions): not closures.
pub(crate) fn declaration_lambdas(
	tree: &fa_parser::TypedSyntaxTree,
) -> std::collections::HashSet<NodeId> {
	let mut lambdas = std::collections::HashSet::new();
	for index in 0..tree.nodes.len() {
		match view(tree, NodeId::new(index)) {
			View::Function { value, .. } | View::ExportFunction { value, .. } => {
				lambdas.insert(value);
			}
			_ => {}
		}
	}
	lambdas
}
