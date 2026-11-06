//! Callbacks that throw, in user functions (decision "Callbacks that throw,
//! in user functions"; defining-functions.md, "Functions as parameters").
//!
//! A function-typed parameter that the function only calls (never stores,
//! returns, passes on, nor reads in a closure) accepts a callback that
//! throws, though its type doesn't: the call then throws the callback's
//! errors, like a built-in (`calls.rs`). Inside the function such a call
//! needs no `try`, and a `catch` around such calls only is `EF018`.
//!
//! "Only called" is read from the syntax, by name: every occurrence of the
//! parameter's name in the body and the default values is the callee of a
//! call (`action()`), outside of any nested function. A local of the same
//! name only makes the answer `false`, never wrongly `true`.

use fa_parser::TypedSyntaxTree;

use crate::check::Walker;
use crate::ids::{NodeId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};
use crate::types::TypeKind;

impl Walker<'_> {
	/// Bit `i`: the function-typed parameter `i` of the function declared by
	/// `declaration` is only called by it (at most 32 parameters).
	pub(crate) fn only_called(&mut self, declaration: NodeId) -> u32 {
		if let Some(&bits) = self.only_called.get(&declaration) {
			return bits;
		}
		let Some(info) = self.a.functions.get(&declaration) else {
			return 0;
		};
		let mut bits = 0;
		for (index, param) in info.params.iter().enumerate().take(32) {
			let symbol = self.a.symbols.symbol(param.symbol);
			if param.variadic || !matches!(self.s.types.kind(symbol.ty), TypeKind::Function(_)) {
				continue;
			}
			let name = self.s.types.syms.resolve(symbol.name);
			let mut roots = std::iter::once(info.body)
				.chain(info.params.iter().filter_map(|param| param.default));
			if roots.all(|root| only_calls(&self.tree, root, name, false, false)) {
				bits |= 1 << index;
			}
		}
		self.only_called.insert(declaration, bits);
		bits
	}

	/// Whether `parameter` (a parameter symbol) is only called by its
	/// function (`declaration`).
	pub(crate) fn is_only_called(&mut self, declaration: NodeId, parameter: SymbolId) -> bool {
		let Some(index) = self.a.functions.get(&declaration).and_then(|info| {
			info.params
				.iter()
				.position(|param| param.symbol == parameter)
		}) else {
			return false;
		};
		index < 32 && self.only_called(declaration) & (1 << index) != 0
	}

	/// Whether `n` holds a call of a parameter that the enclosing function
	/// only calls: its errors are the caller's (`EF018` when a `catch`
	/// covers only such calls).
	pub(crate) fn calls_passed_callback(&mut self, n: NodeId) -> bool {
		let Some(function) = self.body.functions.last().and_then(|cx| cx.symbol) else {
			return false;
		};
		let declaration = self.a.symbols.symbol(function).decl_node;
		let mut found = false;
		let mut stack = vec![n];
		while let Some(node) = stack.pop() {
			match syntax::view(&self.tree, node) {
				View::Lambda { .. } | View::Function { .. } => continue,
				View::Call { callee, .. } => {
					let callee = self.strip_groups(callee);
					if let (View::Name(_), Resolution::Local(symbol)) =
						(syntax::view(&self.tree, callee), self.a.resolution(callee))
						&& matches!(
							self.a.symbols.symbol(symbol).kind,
							SymbolKind::Parameter { .. }
						) && self.is_only_called(declaration, symbol)
					{
						found = true;
						break;
					}
				}
				_ => {}
			}
			stack.extend(syntax::children(&self.tree, node));
		}
		found
	}
}

/// Whether every occurrence of `name` below `n` is the callee of a call
/// (`callee`: `n` is the callee of a call; `nested`: inside a nested
/// function, where no occurrence is allowed).
fn only_calls(tree: &TypedSyntaxTree, n: NodeId, name: &str, callee: bool, nested: bool) -> bool {
	match syntax::view(tree, n) {
		View::Name(found) => tree.name(found) != name || (callee && !nested),
		View::Group(inner) => only_calls(tree, inner, name, callee, nested),
		View::Call { callee, arguments } => {
			only_calls(tree, callee, name, true, nested)
				&& arguments
					.iter()
					.all(|argument| only_calls(tree, argument, name, false, nested))
		}
		View::Lambda { .. } | View::Function { .. } => {
			syntax::children(tree, n).all(|child| only_calls(tree, child, name, false, true))
		}
		_ => syntax::children(tree, n).all(|child| only_calls(tree, child, name, false, nested)),
	}
}
