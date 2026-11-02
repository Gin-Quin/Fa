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
	/// A requirement of a function (`requires name: Type`), by its position
	/// in the signature's requirements (advanced brief §9).
	Requirement { index: u16 },
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
	/// The names a file sees through the module system (siblings, roots,
	/// `library`): the parent of its module scope, holding proxies of the
	/// other files' declarations (`check/modules.rs`).
	Imports,
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
	/// By name, the latest symbol of each name, once the scope holds more
	/// than [`INDEXED_SCOPE`] symbols (a module scope, a large block): lookups
	/// stay O(1) (design §2.3).
	index: Option<FxHashMap<Sym, SymbolId>>,
}

/// The number of symbols above which a scope gets a hash index.
const INDEXED_SCOPE: usize = 16;

/// What a name or member refers to.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Resolution {
	/// Not a reference, or not found.
	#[default]
	Unresolved,
	/// A symbol of this file.
	Local(SymbolId),
	/// A module-level declaration (or a namespace member, a function of a
	/// type, a parameter) of another file, by its declaration key (its
	/// module, its enclosing names, its name: decision A10), so that a new
	/// version of the other file never leaves it stale.
	/// [`crate::Session::external`] finds the current file and the symbol,
	/// [`crate::Session::external_node`] the declaration node.
	External(DeclKey),
	/// A folder namespace (its path, as a module path: `library/reports`).
	Folder(ModuleId),
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
			index: None,
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
		let name = symbol.name;
		self.symbols.push(symbol);
		let scope = &mut self.scopes[scope.index()];
		scope.symbols.push(id);
		match &mut scope.index {
			Some(index) => {
				index.insert(name, id);
			}
			None if scope.symbols.len() > INDEXED_SCOPE => {
				let mut index = FxHashMap::default();
				for &symbol in &scope.symbols {
					index.insert(self.symbols[symbol.index()].name, symbol);
				}
				scope.index = Some(index);
			}
			None => {}
		}
		(id, previous)
	}

	/// Sets the parent of a scope (the imports scope above a module scope).
	pub fn set_parent(&mut self, scope: ScopeId, parent: Option<ScopeId>) {
		self.scopes[scope.index()].parent = parent;
	}

	/// The scope opened by a node, if it was created.
	pub fn scope_of_owner(&self, owner: NodeId) -> Option<ScopeId> {
		self.by_owner.get(&owner).copied()
	}

	/// The symbol named `name` in `scope` only.
	pub fn lookup_in(&self, scope: ScopeId, name: Sym) -> Option<SymbolId> {
		let scope = &self.scopes[scope.index()];
		if let Some(index) = &scope.index {
			return index.get(&name).copied();
		}
		scope
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
	fn large_scopes_are_indexed() {
		let mut table = SymbolTable::new();
		let module = table.scope_for(NodeId(0), None, ScopeKind::Module);
		for name in 0..100 {
			let (_, previous) = table.declare(module, symbol(name, name + 10));
			assert_eq!(previous, None);
		}
		assert!(table.scope(module).index.is_some());
		let (again, previous) = table.declare(module, symbol(7, 500));
		assert!(previous.is_some());
		assert_eq!(table.lookup(module, Sym(7)), Some(again), "the latest wins");
		assert_eq!(
			table
				.lookup(module, Sym(99))
				.map(|s| table.symbol(s).decl_node),
			Some(NodeId(109))
		);
		assert_eq!(table.lookup(module, Sym(1000)), None);
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
