//! The per-file results of the analysis: side tables indexed by node.
//!
//! Types, resolutions, flags, symbols and diagnostics are filled by the walk
//! (`check/`); M1a adds arithmetic info and coercions, and freezes the
//! backend-facing part (design §2.6).

use bitflags::bitflags;

use crate::diagnostics::Diagnostic;
use crate::ids::{FileId, NodeId, TypeId};
use crate::symbols::{Resolution, SymbolTable};

bitflags! {
	/// Per-node facts for the backends (design §2.6).
	#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
	pub struct NodeFlags: u32 {
		/// The node may exceed the native compute type (values brief §6.7).
		const OVERFLOW_NATIVE = 1 << 0;
		/// A number-represented node may exceed ±(2^53 − 1).
		const OVERFLOW_JS = 1 << 1;
		/// The root value must be range-checked against its sink type.
		const SINK_CHECK = 1 << 2;
		/// The divisor is not proven non-zero: `DivisionByZero` path.
		const DIVISOR_CHECK = 1 << 3;
		/// The index or slice is not proven: `IndexError` path.
		const INDEX_CHECK = 1 << 4;
		/// The conversion is not proven to fit: `ArithmeticError` path.
		const CONVERSION_CHECK = 1 << 5;
		/// The exponent is not proven non-negative.
		const EXPONENT_CHECK = 1 << 6;
		/// The shift amount is not proven in `[0, width)`.
		const SHIFT_CHECK = 1 << 7;
		/// `INT_MIN // -1` and `INT_MIN %% -1` guards are needed (values brief D-49).
		const MIN_DIV_GUARD = 1 << 8;
		/// A payload access without re-testing the tag.
		const NARROWED = 1 << 9;
		/// Unreachable code.
		const UNREACHABLE = 1 << 10;
		/// A type expression: its type is the type it denotes.
		const TYPE_EXPR = 1 << 11;
		/// A constant folded by the analyzer.
		const CONST_FOLDED = 1 << 12;
		/// A place (assignment target, `use`, effect argument).
		const PLACE = 1 << 13;
		/// A move (M2).
		const MOVE = 1 << 14;
		/// The expression can fail with an error.
		const CAN_FAIL = 1 << 15;
	}
}

impl NodeFlags {
	/// The flags a `-- no-check` annotation forbids: every runtime check.
	pub const CHECKS: NodeFlags = NodeFlags::OVERFLOW_NATIVE
		.union(NodeFlags::OVERFLOW_JS)
		.union(NodeFlags::SINK_CHECK)
		.union(NodeFlags::DIVISOR_CHECK)
		.union(NodeFlags::INDEX_CHECK)
		.union(NodeFlags::CONVERSION_CHECK)
		.union(NodeFlags::EXPONENT_CHECK)
		.union(NodeFlags::SHIFT_CHECK);
}

/// The analysis of one file.
pub struct Analysis {
	/// The analyzed file.
	pub file: FileId,
	/// The type of every node (`UNSET` for nodes without one).
	pub types: Vec<TypeId>,
	/// What every name or member node refers to (`Unresolved` for other nodes).
	pub resolutions: Vec<Resolution>,
	/// The flags of every node.
	pub flags: Vec<NodeFlags>,
	/// The symbols and the scope arena of the file.
	pub symbols: SymbolTable,
	/// Parse diagnostics first, then the analysis' own, sorted by span.
	pub diagnostics: Vec<Diagnostic>,
}

impl Analysis {
	/// An empty analysis of a file with `nodes` nodes.
	pub fn new(file: FileId, nodes: usize) -> Self {
		Self {
			file,
			types: vec![TypeId::UNSET; nodes],
			resolutions: vec![Resolution::Unresolved; nodes],
			flags: vec![NodeFlags::empty(); nodes],
			symbols: SymbolTable::new(),
			diagnostics: Vec::new(),
		}
	}

	/// The type of a node.
	pub fn type_of(&self, node: NodeId) -> TypeId {
		self.types
			.get(node.index())
			.copied()
			.unwrap_or(TypeId::UNSET)
	}

	/// What a name or member node refers to.
	pub fn resolution(&self, node: NodeId) -> Resolution {
		self.resolutions
			.get(node.index())
			.copied()
			.unwrap_or_default()
	}

	/// The flags of a node.
	pub fn flags_of(&self, node: NodeId) -> NodeFlags {
		self.flags.get(node.index()).copied().unwrap_or_default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flags_by_name() {
		assert_eq!(
			NodeFlags::from_name("SINK_CHECK"),
			Some(NodeFlags::SINK_CHECK)
		);
		assert_eq!(
			NodeFlags::from_name("OVERFLOW_NATIVE"),
			Some(NodeFlags::OVERFLOW_NATIVE)
		);
		assert_eq!(NodeFlags::from_name("NOPE"), None);
		assert!(NodeFlags::CHECKS.contains(NodeFlags::DIVISOR_CHECK));
		assert!(!NodeFlags::CHECKS.contains(NodeFlags::NARROWED));
	}

	#[test]
	fn out_of_range_nodes_have_no_type() {
		let analysis = Analysis::new(FileId(0), 2);
		assert_eq!(analysis.type_of(NodeId(5)), TypeId::UNSET);
		assert_eq!(analysis.flags_of(NodeId(1)), NodeFlags::empty());
	}
}
