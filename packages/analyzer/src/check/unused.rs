//! Unused values (declaring-values.md, "Unused values"): a local value, a
//! parameter or the error of a `catch` that is never used is `W-DECL-2`,
//! unless its name starts with `_`.
//!
//! A use is any name resolving to the symbol other than its declaration:
//! a read, a write, a capture. Module-level values (other files may use
//! them), loop variables, pattern bindings and `self` are not reported.

use crate::check::Walker;
use crate::diagnostics::codes::values::W_DECL_2;
use crate::symbols::SymbolKind;

impl Walker<'_> {
	/// `W-DECL-2` for the locals, parameters and catch bindings never used.
	pub(super) fn report_unused(&mut self) {
		let mut unused = Vec::new();
		for (index, symbol) in self.a.symbols.symbols().iter().enumerate() {
			if self.used_symbols.get(index).copied().unwrap_or(false)
				|| self
					.a
					.has_flags(symbol.decl_node, crate::analysis::NodeFlags::TYPE_EXPR)
			{
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
