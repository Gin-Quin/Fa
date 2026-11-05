//! The scan of a file (design/parallel-pipeline.md §2.1): what the module
//! system needs to know of a file before its walk, collected in one pass
//! over its nodes right after it is parsed (on the parsing thread).
//!
//! - The names written in the file and the access chains starting at them
//!   (`library.math.add`), resolved through the module system: these are the
//!   file's imports (the walk's proxies, `check/modules.rs`), and the edges
//!   of the module graph ([`crate::graph`]). A name shadowed by a local
//!   declaration gives an edge the walk doesn't follow: it only costs
//!   parallelism.
//! - The namespace blocks, the export statement, whether the file has
//!   reactive code: what the walk used to find by scanning every node.

use rustc_hash::{FxHashMap, FxHashSet};

use fa_parser::TypedSyntaxTree;

use crate::check::modules::{export_of, namespace_block};
use crate::ids::NodeId;
use crate::modules::{ModuleEntry, ModuleResolver};
use crate::symbols::SymbolKind;
use crate::syntax::{self, View};

/// The most members of an access chain kept (folders are rarely deeper).
const MAX_CHAIN: usize = 6;

/// What the scan of a file found.
#[derive(Clone, Debug, Default)]
pub struct FileScan {
	/// The names the module system resolves, in the order first written
	/// (the `@` of a compile-time call stripped), with what they designate.
	pub imports: Vec<(Box<str>, ModuleEntry)>,
	/// The modules the file may read: the files its imports and their
	/// access chains reach, sorted.
	pub dependencies: Vec<String>,
	/// The block of each namespace declaration → the declaration.
	pub namespace_blocks: FxHashMap<NodeId, NodeId>,
	/// The export statement and the kind of what it exports.
	pub export: Option<(NodeId, SymbolKind)>,
	/// Whether the file declares `reactive` or `derived` values or `watch`es.
	pub has_reactivity: bool,
	/// Whether the file may declare relations (reverse sides, reference
	/// types): the files reading it import them (check/relations.rs).
	pub has_relations: bool,
	/// The names of `std` its `use { … } = std.module` bring, by the name
	/// they take in the file (`check/std_library.rs`).
	pub std_uses: Vec<(Box<str>, crate::ids::BuiltinId)>,
	/// The bytes of the source.
	pub bytes: usize,
	/// Whether the names were resolved (a resolver was given).
	pub resolved: bool,
}

/// The source of an `@import("…")` call (architecture/dependencies.md,
/// "Inline imports"), when it is a string literal.
pub fn import_source(tree: &TypedSyntaxTree, node: NodeId) -> Option<&str> {
	let mut node = node;
	while let View::Group(inner) = syntax::view(tree, node) {
		node = inner;
	}
	let View::Call { callee, arguments } = syntax::view(tree, node) else {
		return None;
	};
	let View::Name(name) = syntax::view(tree, callee) else {
		return None;
	};
	if tree.name(name) != "@import" || arguments.len() != 1 {
		return None;
	}
	match syntax::view(tree, arguments.iter().next()?) {
		View::String(source) => Some(source),
		_ => None,
	}
}

/// The name the module system resolves the source of an `@import` under
/// (`fa` resolves the sources, then gives them under these keys, like the
/// keys of `dependencies.fa`): no name written in a file can be it.
pub fn import_name(source: &str) -> String {
	format!("@import {source}")
}

/// The names a `use { … } = std.module` brings (`value`: `std.module`).
fn std_uses(
	tree: &TypedSyntaxTree,
	pattern: NodeId,
	value: NodeId,
	out: &mut Vec<(Box<str>, crate::ids::BuiltinId)>,
) {
	let mut value = value;
	while let View::Group(inner) | View::Try(inner) = syntax::view(tree, value) {
		value = inner;
	}
	let View::Access { target, members } = syntax::view(tree, value) else {
		return;
	};
	let mut target = target;
	while let View::Group(inner) | View::Try(inner) = syntax::view(tree, target) {
		target = inner;
	}
	let (View::Name(root), 1, Some(member)) =
		(syntax::view(tree, target), members.len(), members.first())
	else {
		return;
	};
	let View::Name(module) = syntax::view(tree, member) else {
		return;
	};
	if tree.name(root) != "std" {
		return;
	}
	let View::ObjectPattern { fields, .. } = syntax::view(tree, pattern) else {
		return;
	};
	let module = tree.name(module);
	for field in fields.iter() {
		if let Some(id) = crate::builtins::std_library::member(module, tree.name(field.name)) {
			let name = tree.name(field.alias.unwrap_or(field.name));
			out.push((name.into(), id));
		}
	}
}

/// Scans a parsed file of module `module`, resolving its names through
/// `resolver` (none: a lone file, no imports).
pub fn scan(
	tree: &TypedSyntaxTree,
	module: &str,
	resolver: Option<&dyn ModuleResolver>,
) -> FileScan {
	let mut out = FileScan {
		export: export_of(tree),
		bytes: tree.source.len(),
		..FileScan::default()
	};
	// the names in their first order, and the access chains after them
	let mut names: Vec<&str> = Vec::new();
	let mut seen: FxHashSet<&str> = FxHashSet::default();
	// per name: the chains of members after it, and whether it is also used alone
	let mut chains: FxHashMap<&str, Vec<Vec<&str>>> = FxHashMap::default();
	let mut targets: FxHashSet<NodeId> = FxHashSet::default();
	let mut alone: Vec<(NodeId, &str)> = Vec::new();
	let mut imported: Vec<String> = Vec::new();
	// a lone file has no imports: only the rest is collected
	let resolving = resolver.is_some();
	for index in 0..tree.nodes.len() {
		let node = NodeId::new(index);
		if syntax::is_reactive(tree, node) {
			out.has_reactivity = true;
		}
		if syntax::is_relation_syntax(tree, node) {
			out.has_relations = true;
		}
		match syntax::view(tree, node) {
			View::Destructuring {
				kind: syntax::BindingKind::Use,
				pattern,
				value,
			} => std_uses(tree, pattern, value, &mut out.std_uses),
			View::Name(_) | View::Access { .. } if !resolving => {}
			// `@import("source")`: its source, used whole
			View::Call { .. } if resolving && import_source(tree, node).is_some() => {
				if let Some(source) = import_source(tree, node) {
					imported.push(import_name(source));
				}
			}
			View::Name(name) => {
				let text = tree.name(name);
				let text = text.strip_prefix('@').unwrap_or(text);
				if text.is_empty() {
					continue;
				}
				if seen.insert(text) {
					names.push(text);
				}
				alone.push((node, text));
			}
			View::Access { target, members } => {
				let View::Name(name) = syntax::view(tree, target) else {
					continue;
				};
				let text = tree.name(name);
				let text = text.strip_prefix('@').unwrap_or(text);
				let chain: Vec<&str> = members
					.iter()
					.take(MAX_CHAIN)
					.map_while(|member| match syntax::view(tree, member) {
						View::Name(name) => Some(tree.name(name)),
						_ => None,
					})
					.collect();
				targets.insert(target);
				let known = chains.entry(text).or_default();
				if !known.contains(&chain) {
					known.push(chain);
				}
			}
			_ => {
				if let Some(block) = namespace_block(tree, node) {
					out.namespace_blocks.insert(block, node);
				}
			}
		}
	}
	let Some(resolver) = resolver else {
		return out;
	};
	out.resolved = true;
	// a name used alone (not as the target of an access) may be a folder used whole
	let mut alone: FxHashSet<&str> = alone
		.into_iter()
		.filter(|(node, _)| !targets.contains(node))
		.map(|(_, text)| text)
		.collect();
	imported.sort();
	imported.dedup();
	for name in &imported {
		names.push(name);
		alone.insert(name);
	}
	let mut dependencies: Vec<String> = Vec::new();
	let mut all_modules: Option<Vec<String>> = None;
	for text in names {
		let Some(entry) = resolver.lookup(module, text) else {
			continue;
		};
		match &entry {
			ModuleEntry::File(found) => dependencies.push(found.clone()),
			ModuleEntry::Folder(folder) => {
				let mut whole = alone.contains(text);
				for chain in chains.get(text).map_or(&[][..], Vec::as_slice) {
					let mut current = folder.clone();
					let mut reached = false;
					for member in chain {
						match resolver.member(&current, member) {
							Some(ModuleEntry::File(found)) => {
								dependencies.push(found);
								reached = true;
								break;
							}
							Some(ModuleEntry::Folder(next)) => current = next,
							None => break,
						}
					}
					// a folder reached at the end of the chain is used whole
					whole |= !reached;
				}
				if whole {
					let modules = all_modules.get_or_insert_with(|| resolver.modules());
					let prefix = format!("{folder}/");
					dependencies.extend(
						modules
							.iter()
							.filter(|other| other.starts_with(&prefix))
							.cloned(),
					);
				}
			}
		}
		out.imports.push((Box::from(text), entry));
	}
	dependencies.sort();
	dependencies.dedup();
	dependencies.retain(|dependency| dependency != module);
	out.dependencies = dependencies;
	out
}
