//! Copy decisions (advanced brief §6.2, §6.5, decision D15): when a value
//! stored somewhere must be copied, so that two places never share an
//! object or a collection.
//!
//! A value read from a place is copied when it is stored into a mutable
//! destination (a `mutable` binding, an assignment, a field, an item), or
//! when the place itself may change later (a `mutable` root, a `for`
//! variable over a collection). Fresh values (literals, call results) are
//! never copied, and neither is the last use of a local (`NodeFlags::MOVE`).
//! A conditional value (`if`, `when`, `catch`, `??`, `do`) is read from a
//! place when one of its branches is.
//!
//! Which types own memory is the backend's business (JavaScript: objects,
//! collections, tuples, unions holding them; native code: strings too):
//! [`copies`] takes the answer for the value's type.

use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, NodeFlags};
use crate::ids::{NodeId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};

/// The local symbol at the root of a place expression (`a`, `a.b`,
/// `a[i].b`), if the node reads a place.
pub fn place_root(tree: &TypedSyntaxTree, analysis: &Analysis, n: NodeId) -> Option<SymbolId> {
	match syntax::view(tree, n) {
		View::Group(inner) => place_root(tree, analysis, inner),
		View::Name(_) => match analysis.resolution(n) {
			Resolution::Local(symbol) => Some(symbol),
			_ => None,
		},
		View::Access { target, .. } | View::Index { target, .. } => {
			place_root(tree, analysis, target)
		}
		_ => None,
	}
}

/// The root of the place a value may be read from: [`place_root`], or for
/// a conditional value, the place of one of its branches, a mutable one
/// first.
pub fn value_place_root(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	n: NodeId,
) -> Option<SymbolId> {
	let branches: Vec<NodeId> = match syntax::view(tree, n) {
		View::Group(inner) | View::Do(inner) => vec![inner],
		View::If {
			then_body,
			else_body,
			..
		} => std::iter::once(then_body).chain(else_body).collect(),
		View::When { arms, .. } => arms.iter().map(|arm| arm.value).collect(),
		View::Catch {
			expression,
			handler,
			..
		} => vec![expression, handler],
		View::Fallback { operands } => operands.iter().collect(),
		View::Block { statements } => statements.iter().last().into_iter().collect(),
		// `items.get(0)`, `scores.values()`: elements of the receiver's place
		View::Call { .. } => match element_view(tree, analysis, n) {
			Some((receiver, _)) => vec![receiver],
			None => return None,
		},
		_ => return place_root(tree, analysis, n),
	};
	let roots: Vec<SymbolId> = branches
		.into_iter()
		.filter_map(|branch| value_place_root(tree, analysis, branch))
		.collect();
	roots
		.iter()
		.copied()
		.find(|&root| is_mutable_place(analysis, root))
		.or(roots.first().copied())
}

/// A built-in call whose result holds elements of its receiver as they are
/// stored, not copies, where a collection holds its elements by reference
/// (JavaScript): one element (`items.get(0)`, `scores.get(name)`,
/// `find`), or a new collection of them (`filter`, `values`, `keys`, the
/// non-in-place `sort` and `reverse`, set operations). Returns the node the
/// receiver is read from (its place root is the receiver's) and whether the
/// result is a collection of elements.
pub fn element_view(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	n: NodeId,
) -> Option<(NodeId, bool)> {
	let View::Call { callee, .. } = syntax::view(tree, n) else {
		return None;
	};
	let crate::analysis::Callee::Builtin(id) = analysis.call(n)?.callee else {
		return None;
	};
	let View::Access { target, .. } = syntax::view(tree, callee) else {
		return None;
	};
	let path = crate::builtins::get(id).path;
	let (owner, method) = path.split_once('#')?;
	if !matches!(
		owner,
		"Array"
			| "GrowingArray"
			| "Map" | "GrowingMap"
			| "Set" | "GrowingSet"
			| "Pool" | "GrowingPool"
			| "Columnar"
			| "OrderedColumnar"
			| "Iterator"
	) {
		return None;
	}
	match method {
		"get" | "find" => Some((target, false)),
		"filter" | "values" | "keys" | "sort" | "reverse" | "union" | "intersection"
		| "difference" | "firstValues" | "toArray" => Some((target, true)),
		_ => None,
	}
}

/// Whether a symbol may be assigned or reseated after its declaration
/// (`mutable`, `mutable use`, `reactive`).
pub fn is_reassignable(analysis: &Analysis, symbol: SymbolId) -> bool {
	matches!(
		analysis.symbols.symbol(symbol).kind,
		SymbolKind::Mutable | SymbolKind::MutableUse | SymbolKind::Reactive
	)
}

/// Whether the place rooted at a symbol may change later: a reassignable
/// symbol, or a `for` variable designating an item of a collection.
pub fn is_mutable_place(analysis: &Analysis, root: SymbolId) -> bool {
	is_reassignable(analysis, root)
		|| matches!(
			analysis.symbols.symbol(root).kind,
			SymbolKind::LoopVariable { copy: false }
		)
}

/// Whether the read at a node is the last use of a local, in a copy
/// position (`NodeFlags::MOVE`, parentheses skipped).
pub fn is_move(tree: &TypedSyntaxTree, analysis: &Analysis, n: NodeId) -> bool {
	let mut n = n;
	while let View::Group(inner) | View::Try(inner) = syntax::view(tree, n) {
		n = inner;
	}
	analysis.has_flags(n, NodeFlags::MOVE)
}

/// Whether a value stored into a destination (`destination_mutable`: a
/// `mutable` binding, an assignment, a field or an item) must be copied;
/// `owns_memory` says whether values of its type own memory on the target.
pub fn copies(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	value: NodeId,
	destination_mutable: bool,
	owns_memory: bool,
) -> bool {
	if !owns_memory || is_move(tree, analysis, value) {
		return false;
	}
	let Some(root) = value_place_root(tree, analysis, value) else {
		return false;
	};
	destination_mutable || is_mutable_place(analysis, root)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Whether the value of the last declaration of a source is copied.
	fn copied(source: &str) -> bool {
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let (value, mutable) = (0..tree.nodes.len())
			.rev()
			.map(NodeId::new)
			.find_map(|n| match syntax::view(tree, n) {
				View::Declaration {
					value: Some(value),
					kind,
					..
				} => Some((value, kind == syntax::BindingKind::Mutable)),
				_ => None,
			})
			.expect("a declaration");
		copies(tree, analysis, value, mutable, true)
	}

	#[test]
	fn places_are_copied_fresh_values_and_moves_are_not() {
		// read again after the copy: a copy
		assert!(copied(
			"let a = [1, 2]\nmutable b = a\nb.push(3)\nconsole.log(a, b)\n"
		));
		// a fresh value
		assert!(!copied("mutable b = [1, 2]\nb.push(3)\nconsole.log(b)\n"));
		// two immutable sides
		assert!(!copied("let a = [1, 2]\nlet b = a\nconsole.log(a, b)\n"));
		// a conditional value with a branch read from a mutable place
		assert!(copied(
			"mutable a = [1]\nlet c = true\nlet b = if c { a } else { [2] }\na.push(2)\nconsole.log(a, b)\n"
		));
	}

	#[test]
	fn the_last_use_of_a_local_is_moved() {
		let source = "function f = () => {\n\tmutable a = [1, 2]\n\ta.push(3)\n\tmutable b = a\n\tb.push(4)\n\tconsole.log(b)\n}\nf()\n";
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let moved = (0..tree.nodes.len())
			.map(NodeId::new)
			.any(|n| is_move(tree, analysis, n));
		assert!(moved, "the read of `a` in `mutable b = a` is a move");
	}
}
