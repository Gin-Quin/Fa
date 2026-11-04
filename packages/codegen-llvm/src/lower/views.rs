//! Parts of a split text as views of its bytes (`design/decisions.md`,
//! "Minimal representations": text is bytes, characters are views).
//!
//! `let words = text.split(" ")` copies every part into a string of its own.
//! When the parts can't outlive the text's bytes nor see them change, a part
//! is a view instead: a pointer into the text and a length, with the
//! `KIND_MASK` kind of the runtime (never freed; a copy allocates, as for the
//! variable of a `for` over characters). The proof is syntactic and
//! conservative, on top of the analyzer's facts:
//!
//! - the call is `<source>.split(…)`, maybe followed by `filter`s (which move
//!   the parts of their temporary receiver), and is the value of a `let`
//!   declaration of a name `parts`: an immutable array, so no part is moved
//!   out, replaced or appended to; every use of `parts` that takes a value
//!   copies it (a copy of a view allocates), as `parts` is never moved
//!   ([`Lowering::holds_views`] keeps `movable_local` from moving it);
//! - `source` is a `let` or `mutable` of this file, not designated by a
//!   reference (`use`, `reuse`, `provide`), whose scope encloses the
//!   declaration of `parts`, so that it lives at least as long;
//! - no write to `source` (an assignment, a compound assignment, a mutated
//!   argument, a method with an effect on its receiver) is in the scope of
//!   `parts` after its declaration, nor in another function, lambda, task,
//!   test or watcher than the one declaring `source` (those may run at any
//!   time).
//!
//! The runtime still copies the parts of an inline source (its bytes are the
//! value, maybe a temporary copy of it): `fa_string_split_views`.

use fa_analyzer::analysis::{Callee, NodeFlags};
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{BindingKind, View};
use rustc_hash::FxHashMap;

use super::Lowering;

/// The split calls of a file whose parts are views, with the local holding them.
pub(crate) type SplitViews = FxHashMap<NodeId, SymbolId>;

impl<'ctx> Lowering<'_, 'ctx> {
	/// The path of the built-in a call calls, if it calls one.
	fn builtin_path(&self, call: NodeId) -> Option<&'static str> {
		match self.analysis.call(call)?.callee {
			Callee::Builtin(id) => Some(fa_analyzer::builtins::get(id).path),
			_ => None,
		}
	}

	/// The split calls of the file being lowered whose parts are views.
	fn split_views(&self) -> std::cell::Ref<'_, SplitViews> {
		if !self.split_views.borrow().contains_key(&self.file) {
			let mut views = SplitViews::default();
			for index in 0..self.tree.nodes.len() {
				let call = NodeId::new(index);
				if let Some(holder) = self.split_view_holder(call) {
					views.insert(call, holder);
				}
			}
			self.split_views.borrow_mut().insert(self.file, views);
		}
		std::cell::Ref::map(self.split_views.borrow(), |views| &views[&self.file])
	}

	/// Whether the parts of this `split` call are views of its receiver.
	pub(crate) fn splits_into_views(&self, call: NodeId) -> bool {
		self.split_views().contains_key(&call)
	}

	/// Whether a local holds the parts of a split as views (never moved).
	pub(crate) fn holds_views(&self, symbol: SymbolId) -> bool {
		self.split_views().values().any(|&holder| holder == symbol)
	}

	/// The `let` holding the parts of this call, when they can be views.
	fn split_view_holder(&self, call: NodeId) -> Option<SymbolId> {
		if self.builtin_path(call) != Some("String#split") {
			return None;
		}
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		if members.len() != 1 {
			return None;
		}
		let mut source_node = target;
		while let View::Group(inner) = self.view(source_node) {
			source_node = inner;
		}
		if !matches!(self.view(source_node), View::Name(_)) {
			return None;
		}
		let Resolution::Local(source) = self.analysis.resolution(source_node) else {
			return None;
		};
		let source_data = self.analysis.symbols.symbol(source);
		if !matches!(source_data.kind, SymbolKind::Let | SymbolKind::Mutable)
			|| self.referenced_locals().contains(&source)
		{
			return None;
		}
		let parents = self.parents();
		// another file may change an exported value
		let source_parent = parents
			.get(source_data.decl_node.index())
			.copied()
			.unwrap_or(usize::MAX);
		if source_parent != usize::MAX
			&& matches!(
				self.view(NodeId::new(source_parent)),
				View::ExportValue { .. }
			) {
			return None;
		}
		// up to the declaration, through groups and filters of the temporary
		let mut current = call.index();
		let declaration = loop {
			let parent = *parents.get(current)?;
			if parent == usize::MAX {
				return None;
			}
			match self.view(NodeId::new(parent)) {
				View::Group(_) => current = parent,
				View::Access { target, members }
					if target.index() == current && members.len() == 1 =>
				{
					let call = *parents.get(parent)?;
					if call == usize::MAX
						|| self.builtin_path(NodeId::new(call)) != Some("Array#filter")
						|| !matches!(self.view(NodeId::new(call)), View::Call { callee, .. } if callee.index() == parent)
					{
						return None;
					}
					current = call;
				}
				View::Declaration {
					kind: BindingKind::Let,
					value: Some(value),
					..
				} if value.index() == current => break NodeId::new(parent),
				_ => return None,
			}
		};
		let holder = self.analysis.symbols.symbol_of_decl(declaration)?;
		if self.analysis.symbols.symbol(holder).kind != SymbolKind::Let {
			return None;
		}
		// the scope of the holder after its declaration: the rest of its block
		let block = {
			let mut at = declaration.index();
			loop {
				let parent = *parents.get(at)?;
				if parent == usize::MAX {
					return None;
				}
				if matches!(
					self.view(NodeId::new(parent)),
					View::Block { .. } | View::Module { .. }
				) {
					break parent;
				}
				at = parent;
			}
		};
		let region = self.tree.spans[declaration.index()].end..self.tree.spans[block].end;
		// the source is declared around the holder's block (its scope encloses it)
		let declared = self.tree.spans[source_data.decl_node.index()];
		if declared.start >= self.tree.spans[declaration.index()].start {
			return None;
		}
		let home = self.body_of(source_data.decl_node.index(), &parents);
		for read in self.uses_of(source) {
			if !self.writes_local(read, &parents) {
				continue;
			}
			let start = self.tree.spans[read.index()].start;
			if region.contains(&start) || self.body_of(read.index(), &parents) != home {
				return None;
			}
		}
		Some(holder)
	}

	/// The closest function, lambda, task, test or watcher around a node
	/// (`usize::MAX` for the top level of the file).
	fn body_of(&self, node: usize, parents: &[usize]) -> usize {
		let mut at = node;
		while let Some(&parent) = parents.get(at) {
			if parent == usize::MAX {
				break;
			}
			if matches!(
				self.view(NodeId::new(parent)),
				View::Function { .. }
					| View::Lambda { .. }
					| View::Start { .. }
					| View::Test { .. }
					| View::Watch { .. }
					| View::ExportFunction { .. }
			) {
				return parent;
			}
			at = parent;
		}
		usize::MAX
	}

	/// Whether this read of a local may change it: a place (an assignment
	/// target, a mutated argument), the receiver of a method with an effect
	/// or of a method that isn't a string's, or an argument of a function
	/// that isn't a built-in.
	fn writes_local(&self, read: NodeId, parents: &[usize]) -> bool {
		let mut current = read.index();
		loop {
			if self
				.analysis
				.has_flags(NodeId::new(current), NodeFlags::PLACE)
			{
				return true;
			}
			let Some(&parent) = parents.get(current) else {
				return true;
			};
			if parent == usize::MAX {
				return false;
			}
			match self.view(NodeId::new(parent)) {
				View::Group(_) => current = parent,
				View::Access { target, .. } | View::Index { target, .. }
					if target.index() == current =>
				{
					// a method call on the local
					let call = parents.get(parent).copied().unwrap_or(usize::MAX);
					if call != usize::MAX
						&& matches!(self.view(NodeId::new(call)), View::Call { callee, .. } if callee.index() == parent)
					{
						return match self.builtin_path(NodeId::new(call)) {
							Some(path) => {
								!path.starts_with("String#")
									|| !fa_analyzer::builtins::effects::effects_of(path).is_empty()
							}
							None => true,
						};
					}
					current = parent;
				}
				View::Assignment { target, .. }
				| View::CompoundAssignment { target, .. }
				| View::UpdateFields { target, .. } => return target.index() == current,
				View::Call { callee, .. } => {
					return callee.index() == current
						|| !matches!(
							self.analysis
								.call(NodeId::new(parent))
								.map(|info| info.callee),
							Some(Callee::Builtin(_) | Callee::Conversion(_))
						);
				}
				_ => return false,
			}
		}
	}
}
