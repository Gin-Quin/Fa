//! Symbols and scopes of a file (design §2.3).
//!
//! Scopes form an arena kept in the [`Analysis`](crate::analysis::Analysis), so
//! that completions and on-demand analysis can jump to any scope by id. Lookup
//! is a linear scan of each scope (local scopes hold few symbols), from the
//! innermost scope outwards; the latest declaration of a name wins.

use rustc_hash::FxHashMap;

use crate::diagnostics::SourceSpan;
use crate::ids::{
	BuiltinId, DeclKey, ModuleId, NodeId, NominalId, ScopeId, ShapeId, SpellingId, SummaryId, Sym,
	SymbolId, TypeId,
};

/// What a symbol declares.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SymbolKind {
	/// `let`
	Let,
	/// `mutable`
	Mutable,
	/// `use`
	Use,
	/// `mutable use`
	MutableUse,
	/// `static`
	Static,
	/// A function parameter.
	Parameter { index: u16 },
	/// `self`
	SelfParameter,
	/// The variable of a `for` loop (`copy`: `for copy x in …`).
	LoopVariable { copy: bool },
	/// The error of a `catch`.
	CatchBinding,
	/// A name bound by a pattern.
	PatternBinding,
	/// `function`
	Function,
	/// `type`
	Type,
	/// A type parameter.
	TypeParameter,
	/// `namespace`
	Namespace,
	/// `test`
	Test,
	/// `reactive`
	Reactive,
	/// `derived`
	Derived,
	/// A name alias.
	NameAlias,
	/// An implicit name (`arguments`, values brief D4).
	Implicit,
}

impl SymbolKind {
	/// Whether the symbol can be assigned as a whole (`x = …`, `x += …`).
	pub fn is_assignable(self) -> bool {
		matches!(self, SymbolKind::Mutable | SymbolKind::Reactive)
	}
}

/// Where the analysis of a declaration stands (design §3.2).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DeclState {
	/// An ordinary local, declared when reached.
	Sequential,
	/// Hoisted, not analyzed yet: analyzed on demand at its first use.
	Pending,
	/// Its signature (or type) is being resolved: reaching it again is a cycle.
	Signature { frame: u32 },
	/// Its body is being analyzed: the signature is available (recursion).
	Body { frame: u32 },
	/// Analyzed.
	Done,
}

/// A declared name.
#[derive(Clone, Debug)]
pub struct Symbol {
	/// The name.
	pub name: Sym,
	/// What it declares.
	pub kind: SymbolKind,
	/// The declaration node.
	pub decl_node: NodeId,
	/// The span of the name in the declaration.
	pub name_span: SourceSpan,
	/// The scope it is declared in.
	pub scope: ScopeId,
	/// The declared or inferred type; `UNSET` while pending.
	pub ty: TypeId,
	/// The type as written, for messages and hover.
	pub spelling: Option<SpellingId>,
	/// Where its analysis stands.
	pub state: DeclState,
	/// Nominal identity: module-level declarations, types, functions.
	pub key: Option<DeclKey>,
	/// The summary of a function.
	pub summary: Option<SummaryId>,
}

impl Symbol {
	/// A sequential symbol with no type yet.
	pub fn new(name: Sym, kind: SymbolKind, decl_node: NodeId, name_span: SourceSpan) -> Self {
		Self {
			name,
			kind,
			decl_node,
			name_span,
			scope: ScopeId(0),
			ty: TypeId::UNSET,
			spelling: None,
			state: DeclState::Sequential,
			key: None,
			summary: None,
		}
	}
}

/// What opens a scope.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ScopeKind {
	/// The file.
	Module,
	/// A block.
	Block,
	/// The parameters of a function.
	Function,
	/// A namespace or a type's members.
	Namespace,
}

/// A scope: its symbols in declaration order.
#[derive(Clone, Debug)]
pub struct Scope {
	/// The enclosing scope.
	pub parent: Option<ScopeId>,
	/// The node that opens it.
	pub owner: NodeId,
	/// What opens it.
	pub kind: ScopeKind,
	/// Its symbols, in declaration order.
	pub symbols: Vec<SymbolId>,
}

/// What a name or member refers to.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Resolution {
	/// Not a reference, or not found.
	#[default]
	Unresolved,
	/// A symbol of this file.
	Local(SymbolId),
	/// A declaration of another module.
	Module(ModuleId, Sym),
	/// A built-in of the prelude.
	Builtin(BuiltinId),
	/// A field of an object shape.
	Field(ShapeId, u16),
	/// A method of a type.
	Method(DeclKey, Sym),
	/// A member of a named union, an `Errors` type or an enumeration.
	Member(NominalId, u16),
}

/// The symbols and scopes of a file.
#[derive(Clone, Debug, Default)]
pub struct SymbolTable {
	symbols: Vec<Symbol>,
	scopes: Vec<Scope>,
	by_decl: FxHashMap<NodeId, SymbolId>,
	by_owner: FxHashMap<NodeId, ScopeId>,
}

impl SymbolTable {
	/// An empty table.
	pub fn new() -> Self {
		Self::default()
	}

	/// The scope opened by `owner`, created on first use: a block walked again
	/// (loop rounds, on-demand analysis) gets the same scope.
	pub fn scope_for(
		&mut self,
		owner: NodeId,
		parent: Option<ScopeId>,
		kind: ScopeKind,
	) -> ScopeId {
		if let Some(&scope) = self.by_owner.get(&owner) {
			return scope;
		}
		let scope = ScopeId::new(self.scopes.len());
		self.scopes.push(Scope {
			parent,
			owner,
			kind,
			symbols: Vec::new(),
		});
		self.by_owner.insert(owner, scope);
		scope
	}

	/// Declares a symbol in `scope`. Returns it, and the symbol of the same
	/// name already declared in that scope, if any (`E-DECL-2` for the caller).
	/// The new symbol shadows the old one either way.
	pub fn declare(&mut self, scope: ScopeId, mut symbol: Symbol) -> (SymbolId, Option<SymbolId>) {
		let previous = self.lookup_in(scope, symbol.name);
		let id = SymbolId::new(self.symbols.len());
		symbol.scope = scope;
		self.by_decl.entry(symbol.decl_node).or_insert(id);
		self.symbols.push(symbol);
		self.scopes[scope.index()].symbols.push(id);
		(id, previous)
	}

	/// The symbol named `name` in `scope` only.
	pub fn lookup_in(&self, scope: ScopeId, name: Sym) -> Option<SymbolId> {
		self.scopes[scope.index()]
			.symbols
			.iter()
			.rev()
			.copied()
			.find(|&id| self.symbols[id.index()].name == name)
	}

	/// The symbol named `name` in `scope` or an enclosing scope.
	pub fn lookup(&self, scope: ScopeId, name: Sym) -> Option<SymbolId> {
		let mut current = Some(scope);
		while let Some(scope) = current {
			if let Some(id) = self.lookup_in(scope, name) {
				return Some(id);
			}
			current = self.scopes[scope.index()].parent;
		}
		None
	}

	/// The (first) symbol declared by a declaration node.
	pub fn symbol_of_decl(&self, node: NodeId) -> Option<SymbolId> {
		self.by_decl.get(&node).copied()
	}

	/// A symbol.
	pub fn symbol(&self, id: SymbolId) -> &Symbol {
		&self.symbols[id.index()]
	}

	/// A symbol, to update its type or state.
	pub fn symbol_mut(&mut self, id: SymbolId) -> &mut Symbol {
		&mut self.symbols[id.index()]
	}

	/// Every symbol.
	pub fn symbols(&self) -> &[Symbol] {
		&self.symbols
	}

	/// A scope.
	pub fn scope(&self, id: ScopeId) -> &Scope {
		&self.scopes[id.index()]
	}

	/// Every scope.
	pub fn scopes(&self) -> &[Scope] {
		&self.scopes
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn symbol(name: u32, node: u32) -> Symbol {
		Symbol::new(
			Sym(name),
			SymbolKind::Let,
			NodeId(node),
			SourceSpan::default(),
		)
	}

	#[test]
	fn lookup_walks_outwards_and_latest_wins() {
		let mut table = SymbolTable::new();
		let module = table.scope_for(NodeId(0), None, ScopeKind::Module);
		let block = table.scope_for(NodeId(1), Some(module), ScopeKind::Block);
		assert_eq!(
			table.scope_for(NodeId(1), Some(module), ScopeKind::Block),
			block
		);
		let (outer, previous) = table.declare(module, symbol(1, 10));
		assert_eq!(previous, None);
		assert_eq!(table.lookup(block, Sym(1)), Some(outer));
		let (inner, previous) = table.declare(block, symbol(1, 11));
		assert_eq!(
			previous, None,
			"shadowing an outer scope is not a redeclaration"
		);
		assert_eq!(table.lookup(block, Sym(1)), Some(inner));
		assert_eq!(table.lookup(module, Sym(1)), Some(outer));
		let (again, previous) = table.declare(block, symbol(1, 12));
		assert_eq!(previous, Some(inner));
		assert_eq!(table.lookup(block, Sym(1)), Some(again));
		assert_eq!(table.lookup(block, Sym(2)), None);
		assert_eq!(table.symbol_of_decl(NodeId(11)), Some(inner));
		assert_eq!(table.symbol(inner).scope, block);
	}
}
