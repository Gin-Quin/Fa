//! The items and the length of an array indexed by a counted loop, read once
//! before it: `for y in 1..width - 1 { … destination[i] = … }` (`blur`).
//!
//! LLVM's range check elimination (`irce`, `options.rs`) takes the checks
//! `0 <= i < length` out of a loop only when `length` is loop invariant, and
//! LLVM can't move the read of a header before a loop that may exit before
//! reaching it (an earlier check fails): the header pointer isn't known to be
//! dereferenceable there. With a run-time size, a first round of `irce` then
//! `licm` happened to hoist the destination's header; with a constant size the
//! loop takes another shape and the second round doesn't apply, which left a
//! check in the loop and no vectorization. Reading the header before the loop
//! makes every check invariant at the first round, whatever the sizes.
//!
//! Safe when the loop can't change the array's storage or length: in the
//! body, the local is only indexed (read, or an element assigned) or measured
//! (`length`), and the body calls no function of the program, creates no
//! closure, task or reference: nothing else can reach the array.

use fa_analyzer::analysis::Callee;
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::values::PointerValue;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::Repr;

impl<'ctx> Lowering<'_, 'ctx> {
	/// Reads the arrays the body only indexes before the loop; the locals
	/// hoisted (to forget after the loop).
	pub(crate) fn hoist_arrays(&mut self, body: NodeId) -> Result<Vec<SymbolId>, Error> {
		let mut hoisted = Vec::new();
		if self.tasks.asynchronous {
			return Ok(hoisted);
		}
		// nothing in the body can reach an array but its own reads
		let mut names = Vec::new();
		let mut stack = vec![body.index()];
		while let Some(at) = stack.pop() {
			let node = NodeId::new(at);
			match self.view(node) {
				View::Lambda { .. }
				| View::Function { .. }
				| View::Start { .. }
				| View::Use { .. }
				| View::Reuse { .. }
				| View::Provide { .. }
				| View::Await(_) => return Ok(hoisted),
				View::Call { .. }
					if !matches!(
						self.analysis.call(node).map(|call| call.callee),
						Some(Callee::Builtin(_) | Callee::Conversion(_))
					) =>
				{
					return Ok(hoisted);
				}
				View::Name(_) => names.push(node),
				_ => {}
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		let parents = self.parents();
		let mut candidates: Vec<SymbolId> = Vec::new();
		let mut rejected: Vec<SymbolId> = Vec::new();
		for &name in &names {
			let Resolution::Local(symbol) = self.analysis.resolution(name) else {
				continue;
			};
			if rejected.contains(&symbol) {
				continue;
			}
			let parent = parents.get(name.index()).copied().unwrap_or(usize::MAX);
			let indexed = parent != usize::MAX
				&& match self.view(NodeId::new(parent)) {
					View::Index { target, .. } => target == name,
					View::Access { target, members } => {
						target == name
							&& members.len() == 1 && matches!(
							self.analysis.resolution(members.first().expect("a member")),
							Resolution::Builtin(id)
								if fa_analyzer::builtins::get(id).path == "Array#length"
						) && {
							let call = parents.get(parent).copied().unwrap_or(usize::MAX);
							call == usize::MAX
								|| !matches!(self.view(NodeId::new(call)),
									View::Call { callee, .. } if callee.index() == parent)
						}
					}
					_ => false,
				};
			if indexed {
				if !candidates.contains(&symbol) {
					candidates.push(symbol);
				}
			} else {
				rejected.push(symbol);
			}
		}
		drop(parents);
		for symbol in candidates {
			if rejected.contains(&symbol)
				|| self.hoisted_arrays.contains_key(&symbol)
				|| self.references.contains_key(&symbol)
				|| self.shared.contains_key(&symbol)
				|| self.reactive_sources.contains_key(&symbol)
				|| !matches!(
					self.analysis.symbols.symbol(symbol).kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Parameter { .. }
				) {
				continue;
			}
			let Some(&slot) = self.slots.get(&symbol) else {
				continue;
			};
			if !matches!(slot.repr, Repr::Array(_)) {
				continue;
			}
			let array = self
				.load(slot)?
				.value
				.expect("an array")
				.into_pointer_value();
			let parts = self.array_parts(array)?;
			self.hoisted_arrays.insert(symbol, array);
			self.hoisted_parts.insert(array, parts);
			hoisted.push(symbol);
		}
		Ok(hoisted)
	}

	/// Forgets the arrays read before a loop that ended.
	pub(crate) fn unhoist_arrays(&mut self, hoisted: Vec<SymbolId>) {
		for symbol in hoisted {
			if let Some(array) = self.hoisted_arrays.remove(&symbol) {
				let array: PointerValue<'ctx> = array;
				self.hoisted_parts.remove(&array);
			}
		}
	}
}
