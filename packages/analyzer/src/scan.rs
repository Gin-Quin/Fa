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
	/// Whether the names were resolved (a resolver was given).
	pub resolved: bool,
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
		..FileScan::default()
	};
	// the names in their first order, and the access chains after them
	let mut names: Vec<&str> = Vec::new();
	let mut seen: FxHashSet<&str> = FxHashSet::default();
	// per name: the chains of members after it, and whether it is also used alone
	let mut chains: FxHashMap<&str, Vec<Vec<&str>>> = FxHashMap::default();
	let mut targets: FxHashSet<NodeId> = FxHashSet::default();
	let mut alone: Vec<(NodeId, &str)> = Vec::new();
	for index in 0..tree.nodes.len() {
		let node = NodeId::new(index);
		if syntax::is_reactive(tree, node) {
			out.has_reactivity = true;
		}
		match syntax::view(tree, node) {
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
	let alone: FxHashSet<&str> = alone
		.into_iter()
		.filter(|(node, _)| !targets.contains(node))
		.map(|(_, text)| text)
		.collect();
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
