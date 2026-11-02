//! How a backend reads or writes `items[index]` (values brief §15, D-27):
//! from the `INDEX_CHECK` flag and the facts of the index.
//!
//! - [`IndexAccess::Checked`]: the analyzer didn't prove the index: check it
//!   against `-length..length` (`IndexError`), then count a negative one
//!   from the end.
//! - [`IndexAccess::FromEnd`]: proven in bounds, but it may be negative: add
//!   the length to a negative index (JavaScript: `items.at(index)` for a
//!   read), no check.
//! - [`IndexAccess::Direct`]: proven in `0..length`: the plain access.

use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, NodeFlags};
use crate::ids::NodeId;
use crate::syntax::{self, View};
use crate::types::{Interval, TypeStore};

/// How an index is used (see the module documentation).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IndexAccess {
	/// Not proven: checked, then normalized.
	Checked,
	/// Proven, maybe negative: normalized without check.
	FromEnd,
	/// Proven non-negative and in bounds.
	Direct,
}

impl IndexAccess {
	/// Whether the index must be normalized (checked or counted from the end)
	/// before a write `items[index] = value`.
	pub fn normalizes(self) -> bool {
		self != IndexAccess::Direct
	}
}

/// How the `Index` node `n` (an item of an array, not a slice nor a map
/// value) reads or writes its item; `index` is its index node.
pub fn index_access(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	types: &TypeStore,
	n: NodeId,
	index: NodeId,
) -> IndexAccess {
	if analysis.has_flags(n, NodeFlags::INDEX_CHECK) {
		return IndexAccess::Checked;
	}
	match interval(tree, analysis, types, index) {
		Some(interval) if interval.lo >= ethnum::I256::ZERO => IndexAccess::Direct,
		_ => IndexAccess::FromEnd,
	}
}

/// The values an integer node may take: its arithmetic interval, its flow
/// fact, or its type's range (parentheses skipped).
pub fn interval(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	types: &TypeStore,
	n: NodeId,
) -> Option<Interval> {
	let mut n = n;
	while let View::Group(inner) = syntax::view(tree, n) {
		n = inner;
	}
	if let Some(info) = analysis.arith(n) {
		return Some(info.interval);
	}
	if let Some(interval) = analysis.interval(n) {
		return Some(interval);
	}
	types.interval_of(analysis.type_of(n))
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The access of the last `Index` node of a source.
	fn access_of(source: &str) -> IndexAccess {
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let (n, index) = (0..tree.nodes.len())
			.rev()
			.map(NodeId::new)
			.find_map(|n| match syntax::view(tree, n) {
				View::Index { index, .. } => Some((n, index)),
				_ => None,
			})
			.expect("an index");
		index_access(tree, analysis, session.types(), n, index)
	}

	#[test]
	fn indices_are_checked_counted_from_the_end_or_direct() {
		assert_eq!(
			access_of("let items = [1, 2, 3]\nconsole.log(items[0])\n"),
			IndexAccess::Direct
		);
		assert_eq!(
			access_of("let items = [1, 2, 3]\nconsole.log(items[-1])\n"),
			IndexAccess::FromEnd
		);
		assert_eq!(
			access_of(
				"function at = (items: Array(Integer), i: Integer) => try items[i]\nconsole.log(at([1], 0) catch error => 0)\n"
			),
			IndexAccess::Checked
		);
		assert!(IndexAccess::Checked.normalizes() && !IndexAccess::Direct.normalizes());
	}
}
