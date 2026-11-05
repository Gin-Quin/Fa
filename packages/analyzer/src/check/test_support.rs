//! Helpers shared by the unit tests of the walk.

use crate::analyze_source;
use crate::ids::{FileId, NodeId};
use crate::session::Session;

/// The codes emitted for `source`, in order.
pub(super) fn codes(source: &str) -> Vec<&'static str> {
	let (session, file) = analyze_source(source);
	session
		.diagnostics(file)
		.iter()
		// the unused-value warning is tested by its own cases
		.filter(|diagnostic| diagnostic.code.id != "W-DECL-2")
		.map(|diagnostic| diagnostic.code.id)
		.collect()
}

/// Analyzes `source`, which must check without any diagnostic.
pub(super) fn analyzed(source: &str) -> (Session, FileId) {
	let (session, file) = analyze_source(source);
	let found: Vec<String> = session
		.diagnostics(file)
		.iter()
		// the unused-value warning is tested by its own cases
		.filter(|diagnostic| diagnostic.code.id != "W-DECL-2")
		.map(|diagnostic| format!("{} {}", diagnostic.code.id, diagnostic.message))
		.collect();
	assert!(found.is_empty(), "{source}\n{found:#?}");
	(session, file)
}

/// The type of the last symbol named `name`.
pub(super) fn symbol_type(session: &Session, file: FileId, name: &str) -> String {
	let analysis = session.analysis(file);
	let symbol = analysis
		.symbols
		.symbols()
		.iter()
		.rev()
		.find(|symbol| session.types().syms.resolve(symbol.name) == name)
		.unwrap_or_else(|| panic!("no symbol `{name}`"));
	session.display_type(symbol.ty)
}

/// The last node whose text is exactly `text`.
pub(super) fn node(session: &Session, file: FileId, text: &str) -> NodeId {
	let tree = session.tree(file);
	let found = (0..tree.nodes.len())
		.rev()
		.find(|&node| tree.text(node) == text)
		.unwrap_or_else(|| panic!("no node `{text}`"));
	NodeId::new(found)
}
