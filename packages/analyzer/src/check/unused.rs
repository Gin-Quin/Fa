//! Unused values (declaring-values.md, "Unused values"): a local value, a
//! parameter or the error of a `catch` that is never used is `W-DECL-2`,
//! unless its name starts with `_`.
//!
//! A use is any name resolving to the symbol other than its declaration:
//! a read, a write, a capture. Module-level values (other files may use
//! them), loop variables, pattern bindings and `self` are not reported.

use crate::check::Walker;
use crate::diagnostics::codes::values::W_DECL_2;
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};

impl Walker<'_> {
	/// `W-DECL-2` for the locals, parameters and catch bindings never used.
	pub(super) fn report_unused(&mut self) {
		let count = self.a.symbols.symbols().len();
		let mut used = vec![false; count];
		for index in 0..self.tree.nodes.len() {
			let node = crate::ids::NodeId::new(index);
			if let Resolution::Local(symbol) = self.a.resolution(node)
				&& symbol.index() < count
				&& !used[symbol.index()]
				&& let View::Name(_) = syntax::view(&self.tree, node)
				&& self.span(node) != self.a.symbols.symbol(symbol).name_span
			{
				used[symbol.index()] = true;
			}
		}
		let mut unused = Vec::new();
		for (index, symbol) in self.a.symbols.symbols().iter().enumerate() {
			if used[index] {
				continue;
			}
			let kind = match symbol.kind {
				SymbolKind::Let | SymbolKind::Mutable => "value",
				SymbolKind::Use | SymbolKind::MutableUse => "reference",
				SymbolKind::Parameter { .. } => "parameter",
				SymbolKind::CatchBinding => "error",
				_ => continue,
			};
			// a symbol standing for a declaration of another file (the
			// parameters of its functions, for named arguments): no span
			if symbol.name_span == crate::diagnostics::SourceSpan::default() {
				continue;
			}
			let name = self.s.types.syms.resolve(symbol.name);
			if name.is_empty() || name.starts_with('_') {
				continue;
			}
			unused.push((symbol.decl_node, symbol.scope, kind, name.to_string()));
		}
		for (node, scope, kind, name) in unused {
			// module-level values: other files may use them
			if kind == "value" && !self.inside_function(scope) {
				continue;
			}
			let message = format!(
				"the {kind} `{name}` is never used: remove it, or start its name with `_` to say that it is unused on purpose"
			);
			let diagnostic = self.diagnostic(&W_DECL_2, node, message);
			self.report(diagnostic);
		}
	}

	/// Whether a scope is inside a function (or a test).
	fn inside_function(&self, scope: crate::ids::ScopeId) -> bool {
		let mut current = Some(scope);
		while let Some(scope) = current {
			let data = self.a.symbols.scope(scope);
			if data.kind == crate::symbols::ScopeKind::Function {
				return true;
			}
			current = data.parent;
		}
		false
	}
}
