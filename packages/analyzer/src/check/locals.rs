//! The locals that never escape their function (minimal representations,
//! `Analysis::non_escaping`): a backend may keep their values in the frame
//! of the function (natively, a short string in a stack buffer instead of a
//! heap block), since nothing reads them after the function returns.
//!
//! Read uses are recorded during the ordinary analysis walk. A local is a `let` or `mutable` declared
//! in a block or a function (not at the module level). Each read of it is
//! classified by what uses the value:
//!
//! - **reads in place**, which never keep the value: an operand of an
//!   operator (`+` builds a new string), an interpolation, the receiver of
//!   a member or a method, an indexed value, a condition, a `when` subject,
//!   the iterable of a `for`, a `use` alias, an argument of a user function
//!   (parameters are references: a callee that keeps one copies it) or of a
//!   built-in that doesn't store it, a value appended with `+=`, a
//!   statement whose value is dropped;
//! - **moves into another local** (`let t = s`, `t = s`): `s` escapes when
//!   that local does (solved to a fixed point);
//! - **escapes**, everything else: a returned or thrown value, the value of
//!   a block or a branch, an item of a list, tuple or object literal, a
//!   value stored by a built-in (`push`, `add`, `insert`, `getOrInsert`), a
//!   value assigned to a field, an element or a module-level value, a read
//!   inside a nested function or a `start` (a capture), a destructured value.
//!
//! The classification is conservative: a local missing from the set may
//! still stay in its frame, but a local in the set never outlives it.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::Callee;
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, View};

/// How a read of a local uses its value.
enum Use {
	/// Read in place: the value stays where it is.
	InPlace,
	/// Moved or copied into this local: escapes when it does.
	Into(SymbolId),
	/// Kept beyond the function, or in a place the pass doesn't follow.
	Escapes,
}

impl Walker<'_> {
	/// Fills `Analysis::non_escaping` for the file (after its walk).
	pub(super) fn record_non_escaping(&mut self) {
		let mut candidates: FxHashSet<SymbolId> = FxHashSet::default();
		// Captures outlive the local use that appears inside their body. An
		// operator or call there may only read its operand, while the closure
		// still keeps that operand beyond the original function's frame.
		let mut escaped: FxHashSet<SymbolId> = self
			.a
			.functions
			.values()
			.flat_map(|function| function.captures.iter())
			.chain(
				self.a
					.starts
					.values()
					.flat_map(|start| start.captures.iter()),
			)
			.copied()
			.collect();
		// `into[t]`: the locals whose values went into `t`
		let mut into: FxHashMap<SymbolId, Vec<SymbolId>> = FxHashMap::default();
		for (&symbol, reads) in &self.move_uses {
			if !self.is_frame_local(symbol) {
				continue;
			}
			for read in reads {
				let node = read.node;
				// Uses through aliases and calls of capturing functions are
				// recorded for moves too; they aren't direct reads of this name.
				if self.a.resolution(node) != Resolution::Local(symbol)
					|| !matches!(self.view(node), View::Name(_))
				{
					continue;
				}
				candidates.insert(symbol);
				if escaped.contains(&symbol) {
					continue;
				}
				match self.use_of(node) {
					Use::InPlace => {}
					Use::Into(target) if self.is_frame_local(target) => {
						into.entry(target).or_default().push(symbol);
					}
					Use::Into(_) | Use::Escapes => {
						escaped.insert(symbol);
					}
				}
			}
		}

		// what went into an escaping local escapes with it
		let mut pending: Vec<SymbolId> = escaped.iter().copied().collect();
		while let Some(symbol) = pending.pop() {
			for &source in into.get(&symbol).map_or(&[][..], Vec::as_slice) {
				if escaped.insert(source) {
					pending.push(source);
				}
			}
		}
		self.a.non_escaping = candidates
			.into_iter()
			.filter(|symbol| !escaped.contains(symbol))
			.collect();
	}

	/// A `let` or `mutable` of a block or a function: its value lives in the
	/// frame of a function (the program's for the top-level blocks).
	fn is_frame_local(&self, symbol: SymbolId) -> bool {
		let data = self.a.symbols.symbol(symbol);
		matches!(data.kind, SymbolKind::Let | SymbolKind::Mutable)
			&& !matches!(
				self.a.symbols.scope(data.scope).kind,
				ScopeKind::Module | ScopeKind::Namespace | ScopeKind::Imports
			)
	}

	/// How the read `n` of a local uses its value.
	fn use_of(&self, n: NodeId) -> Use {
		let parent_of = |node: NodeId| self.parent(node);
		let mut child = n;
		loop {
			let Some(parent) = parent_of(child) else {
				return Use::Escapes;
			};
			match syntax::view(&self.tree, parent) {
				// the value itself, one level up
				View::Group(_) | View::Try(_) => child = parent,
				View::Declaration { value, .. } if value == Some(child) => {
					return match self.a.symbols.symbol_of_decl(parent) {
						Some(target) => Use::Into(target),
						None => Use::Escapes,
					};
				}
				View::Assignment { target, value, .. } if value == Some(child) => {
					return match parent_of(parent).map(|outer| syntax::view(&self.tree, outer)) {
						// a named argument: an argument of the call
						Some(View::Call { .. }) => {
							self.argument_use(parent_of(parent).expect("a call"))
						}
						// a field of an object literal
						Some(View::Object { .. } | View::TypedObject { .. }) => Use::Escapes,
						_ => match self.a.resolution(target) {
							Resolution::Local(target) => Use::Into(target),
							_ => Use::Escapes,
						},
					};
				}
				// written (`s = …`, `s += …`), or appended (`t += s`): in place
				View::Assignment { .. } | View::CompoundAssignment { .. } => return Use::InPlace,
				View::Call { callee, .. } => {
					return if callee == child {
						Use::InPlace
					} else {
						self.argument_use(parent)
					};
				}
				View::Access { target, .. } | View::OptionalAccess { target, .. }
					if target == child =>
				{
					return Use::InPlace;
				}
				View::Index { .. }
				| View::Arithmetic { .. }
				| View::Negate(_)
				| View::Percentage(_)
				| View::Equality { .. }
				| View::Comparison { .. }
				| View::Logic { .. }
				| View::Not(_)
				| View::Is { .. }
				| View::IsIn { .. }
				| View::Template { .. }
				| View::Use { .. }
				| View::Assert(_) => return Use::InPlace,
				View::If { condition, .. } | View::While { condition, .. }
					if condition == child =>
				{
					return Use::InPlace;
				}
				View::When { subject, .. } if subject == child => return Use::InPlace,
				View::For { iterable, .. } if iterable == child => return Use::InPlace,
				// a statement: dropped, unless it is the value of its block
				View::Block { statements } => {
					let last = statements.iter().last() == Some(child);
					return if last && self.is_value_block(parent) {
						Use::Escapes
					} else {
						Use::InPlace
					};
				}
				View::Module { .. } => return Use::InPlace,
				// returned, thrown, a list or object item, a capture's value…
				_ => return Use::Escapes,
			}
		}
	}

	/// How the call `call` uses the arguments given to it: a user function
	/// reads them in place (parameters are references); a built-in that
	/// stores them keeps them, and so does a detached `start` (it moves them).
	fn argument_use(&self, call: NodeId) -> Use {
		let started = self
			.parent(call)
			.is_some_and(|parent| matches!(syntax::view(&self.tree, parent), View::Start(_)));
		if started {
			return Use::Escapes;
		}
		match self.a.calls.get(&call).map(|info| info.callee) {
			Some(Callee::Builtin(builtin)) => {
				let path = crate::builtins::get(builtin).path;
				if crate::builtins::effects::stores_arguments(path) {
					Use::Escapes
				} else {
					Use::InPlace
				}
			}
			Some(Callee::Function(_) | Callee::Value) => Use::InPlace,
			_ => Use::Escapes,
		}
	}

	/// Whether the value of a block is used: anything but a statement
	/// position (a loop body, a test, a block statement) or the body of a
	/// function returning `Void`.
	fn is_value_block(&self, block: NodeId) -> bool {
		let Some(parent) = self.parent(block) else {
			return false;
		};
		match syntax::view(&self.tree, parent) {
			View::Block { .. }
			| View::Module { .. }
			| View::For { .. }
			| View::While { .. }
			| View::WhileLet { .. }
			| View::Loop(_)
			| View::Test { .. } => false,
			_ => self.a.type_of(block) != TypeId::VOID,
		}
	}
}
