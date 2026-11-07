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
//! lowering of every unit, instance wave and entry (`SplitOptions::facts`).
//! A unit's lowering records the facts it consults ([`FactLog`]: each kind,
//! key and answer), and its cached object is reused only while every one of
//! them still has the same answer (`units.rs`): an edit that changes a fact
//! rebuilds only the units that read it, and a cached object never keeps a
//! layout the program changed. Deterministic: sets of strings, merged in any
//! order to the same result.
//!
//! A fact kind is a [`FactKind`]: a name, what a unit contributes, and how
//! two contributions merge. Its keys are stable names: the same in every
//! unit's session ([`stable_type_name`] for types).

use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;

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
	/// Turns the merged set into the program's facts, once every unit's is
	/// merged (the driver, `ProgramFacts::finish`).
	pub finish: fn(BTreeSet<String>) -> BTreeSet<String>,
}

/// A merged set that is the program's facts as is.
pub fn unchanged(merged: BTreeSet<String>) -> BTreeSet<String> {
	merged
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

/// The claims of the never-negative places (`proofs.rs`): each unit's part
/// of them (`proofs::unit_claims`), merged, then resolved by the driver into
/// the places that fail (`proofs::resolve_claims`).
pub const PROOF_CLAIMS: &str = "proofs.claims";

/// The relation sides of the program's object types (relations across
/// files): `<stable type name>\u{1f}<field>`. A side's layout (the address
/// of its target, or a set of links) differs from a field of the same type
/// (a copy), so every unit, and every instance emitted by a module that
/// doesn't read the relation's file, lays the type out the same way.
pub const RELATION_SIDES: &str = "relations.sides";

/// The key of a relation side in [`RELATION_SIDES`].
pub fn relation_side_key(types: &TypeStore, object: TypeId, field: &str) -> Option<String> {
	stable_type_name(types, object).map(|name| format!("{name}\u{1f}{field}"))
}

/// See [`RELATION_SIDES`]: the sides of the relations the unit's files
/// declare or import.
fn relation_sides(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let types = session.types();
	let mut found = BTreeSet::new();
	for &file in files {
		for relation in &session.analysis(file).relations {
			let sides = [
				(relation.holder_type, Some(relation.direct)),
				(relation.target_type, relation.reverse),
			];
			for (object, field) in sides {
				let Some(field) = field else {
					continue;
				};
				if let Some(key) = relation_side_key(types, object, types.syms.resolve(field)) {
					found.insert(key);
				}
			}
		}
	}
	found
}

/// The fields that may be written after the literal that built their
/// object, and the types whose whole values may be replaced
/// (`proofs::written_fields`): every other field is never written after
/// construction, and a method reads it once (`lower/frozen.rs`).
pub const WRITTEN_FIELDS: &str = "fields.written";

/// Closed root-region claims, resolved only with every module's contribution.
/// Values include type, builder, child-array and unobserved-capacity facts
/// (`fa_analyzer::regions`).
pub const ROOT_REGIONS: &str = "regions.roots";

/// Every fact kind, in a fixed order.
pub const KINDS: &[FactKind] = &[
	FactKind {
		name: ROOT_REGIONS,
		collect: fa_analyzer::regions::unit_claims,
		merge: union,
		finish: fa_analyzer::regions::resolve_claims,
	},
	FactKind {
		name: REFERENCED_MAPS,
		collect: referenced_maps,
		merge: union,
		finish: unchanged,
	},
	FactKind {
		name: DELETED_WHILE_ITERATED_MAPS,
		collect: deleted_while_iterated_maps,
		merge: union,
		finish: unchanged,
	},
	FactKind {
		name: RELATION_SIDES,
		collect: relation_sides,
		merge: union,
		finish: unchanged,
	},
	FactKind {
		name: PROOF_CLAIMS,
		collect: proof_claims,
		merge: union,
		finish: fa_analyzer::proofs::resolve_claims_owned,
	},
	FactKind {
		name: WRITTEN_FIELDS,
		collect: written_fields,
		merge: union,
		finish: unchanged,
	},
];

/// See [`WRITTEN_FIELDS`].
fn written_fields(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let types = session.types();
	let proof_files: Vec<fa_analyzer::proofs::ProofFile<'_>> = files
		.iter()
		.map(|&file| {
			fa_analyzer::proofs::ProofFile::new(session.tree(file), session.analysis(file), types)
		})
		.collect();
	fa_analyzer::proofs::written_fields(&proof_files)
}

/// See [`PROOF_CLAIMS`].
fn proof_claims(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let types = session.types();
	let proof_files: Vec<fa_analyzer::proofs::ProofFile<'_>> = files
		.iter()
		.map(|&file| {
			fa_analyzer::proofs::ProofFile::new(session.tree(file), session.analysis(file), types)
				.with_module(session.module(file))
		})
		.collect();
	fa_analyzer::proofs::unit_claims(&proof_files)
}

/// The facts a lowering consulted: `(kind, key)` → the answer it got. A
/// cached object depends on these only (`units.rs`, `cached_unit`).
#[derive(Debug, Default)]
pub struct FactLog {
	consulted: std::sync::Mutex<BTreeMap<(&'static str, String), bool>>,
}

impl FactLog {
	/// Records an answer.
	pub fn record(&self, kind: &'static str, key: &str, answer: bool) {
		self.consulted
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.insert((kind, key.to_string()), answer);
	}

	/// The facts consulted, one per line (`kind`, `key`, `0` or `1`, tab
	/// separated), sorted: the `.facts` file next to a cached object.
	pub fn to_text(&self) -> String {
		let consulted = self
			.consulted
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		let mut text = String::new();
		for ((kind, key), answer) in consulted.iter() {
			text.push_str(&format!("{kind}\t{key}\t{}\n", u8::from(*answer)));
		}
		text
	}

	/// Whether the facts a text of [`FactLog::to_text`] recorded still have
	/// the same answers (no facts: every answer is "absent").
	pub fn still_holds(text: &str, facts: Option<&ProgramFacts>) -> bool {
		text.lines().all(|line| {
			let mut parts = line.splitn(3, '\t');
			let (Some(kind), Some(key), Some(answer)) = (parts.next(), parts.next(), parts.next())
			else {
				return false;
			};
			let now = facts.is_some_and(|facts| facts.contains(kind, key));
			(answer == "1") == now
		})
	}
}

/// A set of facts read through a lowering's [`FactLog`].
#[derive(Clone, Copy, Debug)]
pub struct Consulted<'f> {
	set: &'f BTreeSet<String>,
	kind: &'static str,
	log: Option<&'f FactLog>,
}

impl Consulted<'_> {
	/// Whether the set holds a key (recorded).
	pub fn contains(&self, key: &str) -> bool {
		let answer = self.set.contains(key);
		if let Some(log) = self.log {
			log.record(self.kind, key, answer);
		}
		answer
	}
}

/// The empty set of a kind no unit contributed to.
static EMPTY: BTreeSet<String> = BTreeSet::new();

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

	/// Turns every merged set into the program's facts (after the last
	/// unit's merge).
	pub fn finish(&mut self) {
		for kind in KINDS {
			let merged = self.facts.remove(kind.name).unwrap_or_default();
			self.facts.insert(kind.name, (kind.finish)(merged));
		}
	}

	/// The set of a kind.
	pub fn set(&self, kind: &str) -> Option<&BTreeSet<String>> {
		self.facts.get(kind)
	}

	/// The set of a kind, read through a log of the facts consulted.
	pub fn consulted<'f>(&'f self, kind: &'static str, log: Option<&'f FactLog>) -> Consulted<'f> {
		Consulted {
			set: self.facts.get(kind).unwrap_or(&EMPTY),
			kind,
			log,
		}
	}

	/// Whether the set of a kind holds a key.
	pub fn contains(&self, kind: &str, key: &str) -> bool {
		self.facts.get(kind).is_some_and(|set| set.contains(key))
	}
}

pub use fa_analyzer::stable_names::stable_type_name;

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
		let parents = parents_of(tree);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			match view(tree, node) {
				// a reference that no change of a map can outlive is fine
				View::Use { .. } if short_reference(tree, analysis, types, node, &parents) => {}
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

/// The parent of every node of a tree (`usize::MAX` for the root).
pub(crate) fn parents_of(tree: &fa_parser::TypedSyntaxTree) -> Vec<usize> {
	let mut parents = vec![usize::MAX; tree.nodes.len()];
	for (index, node) in tree.nodes.iter().enumerate() {
		node.for_each_child(|child| {
			if child < parents.len() {
				parents[child] = index;
			}
		});
	}
	parents
}

/// Whether a `use` reference dies before any map can change: in the rest of
/// its block, nothing calls a function of the program (which may change a
/// map), creates a closure, or calls a built-in method of a map other than a
/// read (an insertion moves dense entries, a deletion the entries of a probe
/// array). The reference's own writes (`count += 1`) change no map's layout.
pub(crate) fn short_reference(
	tree: &fa_parser::TypedSyntaxTree,
	analysis: &fa_analyzer::analysis::Analysis,
	types: &TypeStore,
	node: NodeId,
	parents: &[usize],
) -> bool {
	// the statement holding the reference, in its block
	let mut statement = node.index();
	let block = loop {
		let parent = parents.get(statement).copied().unwrap_or(usize::MAX);
		if parent == usize::MAX {
			return false;
		}
		if matches!(
			view(tree, NodeId::new(parent)),
			View::Block { .. } | View::Module { .. }
		) {
			break parent;
		}
		statement = parent;
	};
	let statements: Vec<NodeId> = match view(tree, NodeId::new(block)) {
		View::Block { statements } | View::Module { statements } => statements.iter().collect(),
		_ => return false,
	};
	let Some(position) = statements.iter().position(|&at| at.index() == statement) else {
		return false;
	};
	// the module's top level: functions may run while the reference lives
	if matches!(view(tree, NodeId::new(block)), View::Module { .. }) {
		return false;
	}
	let reads = [
		"Map#get",
		"Map#has",
		"Map#size",
		"Map#keys",
		"Map#values",
		"Map#includes",
	];
	let mut stack: Vec<usize> = std::iter::once(node.index())
		.chain(statements[position + 1..].iter().map(|at| at.index()))
		.collect();
	while let Some(at) = stack.pop() {
		let current = NodeId::new(at);
		match view(tree, current) {
			View::Lambda { .. } | View::Function { .. } | View::Start { .. } => return false,
			View::Call { callee, .. } => match analysis.call(current).map(|info| info.callee) {
				Some(Callee::Builtin(id)) => {
					let path = fa_analyzer::builtins::get(id).path;
					// `getOrInsert` of the reference's own value is the one
					// insertion allowed: it happens before the reference exists
					let own = parents.get(at) == Some(&node.index());
					if path.starts_with("Map#") && !reads.contains(&path) && !own {
						let _ = callee;
						return false;
					}
				}
				Some(Callee::Conversion(_) | Callee::Member(_)) => {}
				_ => return false,
			},
			View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
				// an item of a map stored, whatever names the map (`m[k]`,
				// `self.m[k]`, `tables[0][k]`), also in a destructuring
				let mut places = vec![target.index()];
				while let Some(place) = places.pop() {
					if let View::Index { target: map, .. } = view(tree, NodeId::new(place))
						&& matches!(
							types.kind(analysis.type_of(map)),
							TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
						) {
						return false;
					}
					tree.nodes[place].for_each_child(|child| places.push(child));
				}
			}
			_ => {}
		}
		tree.nodes[at].for_each_child(|child| stack.push(child));
	}
	true
}

#[cfg(test)]
mod tests {
	use super::stable_type_name;

	/// The stable names of the values of a program, by name.
	fn names(source: &str, wanted: &[&str]) -> Vec<Option<String>> {
		let (session, file) = fa_analyzer::session::analyze_source(source);
		let analysis = session.analysis(file);
		let types = session.types();
		wanted
			.iter()
			.map(|name| {
				let symbol = analysis
					.symbols
					.symbols()
					.iter()
					.find(|symbol| types.syms.resolve(symbol.name) == *name)
					.unwrap_or_else(|| panic!("no `{name}`"));
				stable_type_name(types, symbol.ty)
			})
			.collect()
	}

	#[test]
	fn types_equal_up_to_aliases_share_a_name() {
		let found = names(
			"type Cell = { row: Int32, column: Int32 }\ntype Other = { column: Int32, row: Int32 }\nlet a = Map<Int32, Cell>.from([])\nlet b = Map<Int32, Other>.from([])\nlet c = Map<Int32, Int16>.from([])\n",
			&["a", "b", "c"],
		);
		assert_eq!(found[0], found[1], "{found:?}");
		assert_ne!(found[0], found[2], "{found:?}");
		assert_eq!(
			found[0].as_deref(),
			Some("Map<Int32, {column: Int32, row: Int32}>"),
			"{found:?}"
		);
	}

	/// The program's writes of fields (`proofs::written_fields`).
	fn written(source: &str) -> std::collections::BTreeSet<String> {
		let (session, file) = fa_analyzer::session::analyze_source(source);
		super::ProgramFacts::of_unit(&session, &[file])
			.set(super::WRITTEN_FIELDS)
			.cloned()
			.unwrap_or_default()
	}

	#[test]
	fn fields_written_after_construction_are_found() {
		let base = "type Cursor = { text: String, position: Integer }\nmutable cursor = Cursor { text = \"ab\", position = 0 }\ncursor.position += 1\n";
		let found = written(base);
		assert!(
			found.contains("field:{position: Integer, text: String}.position"),
			"{found:?}"
		);
		assert!(
			!found
				.iter()
				.any(|key| key.ends_with(".text") || key.starts_with("whole:")),
			"{found:?}"
		);
		// a store to the field, an append, a whole value assigned, an item replaced
		for (change, key) in [
			(
				"cursor.text = \"\"\n",
				"field:{position: Integer, text: String}.text",
			),
			(
				"cursor.text += \"c\"\n",
				"field:{position: Integer, text: String}.text",
			),
			// a built-in method changing the field (its receiver)
			(
				"cursor.text.reserve(10)\n",
				"field:{position: Integer, text: String}.text",
			),
			(
				"cursor = Cursor { text = \"\", position = 0 }\n",
				"whole:{position: Integer, text: String}",
			),
			(
				"mutable all = [cursor]\nall[0] = Cursor { text = \"\", position = 0 }\n",
				"whole:{position: Integer, text: String}",
			),
			(
				"mutable all = [cursor]\nall.reverseInPlace()\n",
				"whole:{position: Integer, text: String}",
			),
		] {
			let found = written(&format!("{base}{change}"));
			assert!(found.contains(key), "{change}: {found:?}");
		}
		// a push moves no item
		let found = written(&format!("{base}mutable all = [cursor]\nall.push(cursor)\n"));
		assert!(
			!found.iter().any(|key| key.starts_with("whole:")),
			"{found:?}"
		);
	}

	#[test]
	fn function_types_have_names() {
		let found = names(
			"function f = (a: Int32, b: String): Int32 => a\nfunction g = (x: Int32, y: String): Int32 => x\nfunction h = (a: Int32): String => \"\"\n",
			&["f", "g", "h"],
		);
		assert!(found[0].is_some(), "{found:?}");
		assert_eq!(found[0], found[1], "{found:?}");
		assert_ne!(found[0], found[2], "{found:?}");
	}

	#[test]
	fn generic_functions_equal_up_to_renaming_share_a_name() {
		let found = names(
			"function first = <Item>(items: Array<Item>): Item? => items.get(0)\nfunction head = <T>(values: Array<T>): T? => values.get(0)\n",
			&["first", "head"],
		);
		assert!(
			found[0].as_deref().is_some_and(|name| name.contains("$0")),
			"{found:?}"
		);
		assert_eq!(found[0], found[1], "{found:?}");
	}
}
