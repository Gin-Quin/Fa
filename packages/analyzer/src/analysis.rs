//! The per-file results of the analysis: side tables indexed by node.
//!
//! Types, resolutions, flags, symbols and diagnostics are filled by the walk
//! (`check/`): dense tables indexed by node, then sparse tables for the
//! minority node classes (arithmetic, sinks with a conversion, functions).
//! The backend-facing part is `types`, `resolutions`, `flags`, `arith`,
//! `coercions` and `symbols` (design §2.6), read through the accessors.

use bitflags::bitflags;
use rustc_hash::FxHashMap;

use crate::diagnostics::Diagnostic;
use crate::ids::{FileId, ListId, NodeId, SymbolId, TypeId};
use crate::symbols::{Resolution, Symbol, SymbolTable};
use crate::types::{ComputeType, Interval};

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

	// ── sparse tables (design §2.6): only the nodes of a minority class ──
	/// Every integer arithmetic node (`+ - * // %% **`, unary `-`, compound
	/// assignment) and every range-checked sink: its interval and compute type.
	pub arith: FxHashMap<NodeId, ArithInfo>,
	/// The implicit conversion of a value at its sink, by value node: the
	/// target type of a union injection, `Integer` ↔ `Int64`, `Float32` →
	/// `Float`, `Character` → `String`, and of a `SINK_CHECK` range check.
	pub coercions: FxHashMap<NodeId, TypeId>,
	/// Integer nodes whose flow fact is narrower than their type (hover, proofs).
	pub intervals: FxHashMap<NodeId, Interval>,
	/// The type arguments of a generic call, typed literal or generic function value (M2).
	pub instantiations: FxHashMap<NodeId, ListId>,
	/// Every function declaration and lambda node.
	pub functions: FxHashMap<NodeId, FnInfo>,
}

/// The arithmetic facts of an integer node, for the backends (design §3.6).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ArithInfo {
	/// The values the node may take, from the stable loop round. Kept by
	/// value: the fact store of the walk doesn't outlive it.
	pub interval: Interval,
	/// The type the tree is computed in (values brief §6.7).
	pub compute: ComputeType,
	/// The root of its arithmetic tree: its parent is not an arithmetic node
	/// of the same tree.
	pub root: bool,
}

/// What the backends need of a function or a lambda node.
#[derive(Clone, Debug, PartialEq)]
pub struct FnInfo {
	/// The function type: declared, or inferred for a single-expression body.
	pub ty: TypeId,
	/// The symbol of a named function (`None` for a lambda).
	pub symbol: Option<SymbolId>,
	/// The errors the body may throw: the declared ones, or the inferred ones
	/// for an entry function, a test or a lambda (`NEVER` for none).
	pub throws: TypeId,
	/// The local symbols of enclosing functions it captures (M2: empty until then).
	pub captures: Vec<SymbolId>,
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
			arith: FxHashMap::default(),
			coercions: FxHashMap::default(),
			intervals: FxHashMap::default(),
			instantiations: FxHashMap::default(),
			functions: FxHashMap::default(),
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

	/// Whether a node has all these flags.
	pub fn has_flags(&self, node: NodeId, flags: NodeFlags) -> bool {
		self.flags_of(node).contains(flags)
	}

	/// The symbols and scopes of the file.
	pub fn symbols(&self) -> &SymbolTable {
		&self.symbols
	}

	/// The local symbol a name refers to (`None` for the prelude, other
	/// modules, fields…).
	pub fn symbol_of(&self, node: NodeId) -> Option<&Symbol> {
		match self.resolution(node) {
			Resolution::Local(symbol) => Some(self.symbols.symbol(symbol)),
			_ => None,
		}
	}

	/// The symbol a declaration node declares (`let`, `function`, `type`…).
	pub fn declared_by(&self, node: NodeId) -> Option<&Symbol> {
		self.symbols
			.symbol_of_decl(node)
			.map(|symbol| self.symbols.symbol(symbol))
	}

	/// The arithmetic facts of an integer arithmetic node or range-checked sink.
	pub fn arith(&self, node: NodeId) -> Option<&ArithInfo> {
		self.arith.get(&node)
	}

	/// The implicit conversion of a value at its sink: the type it is converted to.
	pub fn coercion(&self, node: NodeId) -> Option<TypeId> {
		self.coercions.get(&node).copied()
	}

	/// The interval of an integer node, when narrower than its type.
	pub fn interval(&self, node: NodeId) -> Option<Interval> {
		self.intervals.get(&node).copied()
	}

	/// The type arguments of a generic call or literal (M2).
	pub fn instantiation(&self, node: NodeId) -> Option<ListId> {
		self.instantiations.get(&node).copied()
	}

	/// The information of a function declaration or lambda node.
	pub fn function(&self, node: NodeId) -> Option<&FnInfo> {
		self.functions.get(&node)
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
		assert!(analysis.arith(NodeId(1)).is_none());
		assert!(analysis.coercion(NodeId(1)).is_none());
		assert!(analysis.function(NodeId(1)).is_none());
		assert!(analysis.symbol_of(NodeId(1)).is_none());
	}

	#[test]
	fn accessors_read_the_side_tables_of_a_walk() {
		let (session, file) = crate::analyze_source("let answer = 42\nconsole.log(answer)\n");
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let read = (0..tree.nodes.len())
			.rev()
			.find(|&node| tree.text(node) == "answer")
			.map(NodeId::new)
			.unwrap();
		let symbol = analysis.symbol_of(read).unwrap();
		assert_eq!(session.types().syms.resolve(symbol.name), "answer");
		assert!(analysis.declared_by(NodeId::new(tree.root)).is_none());
		let declared = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| analysis.declared_by(node).is_some())
			.unwrap();
		assert_eq!(analysis.declared_by(declared).unwrap().ty, TypeId::INTEGER);
		assert_eq!(analysis.symbols().symbols().len(), 1);
		assert!(!analysis.has_flags(read, NodeFlags::TYPE_EXPR));
	}
}
