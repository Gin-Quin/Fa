//! Function composition (`functions/function-composition`): `f then g`
//! calls `f` with the composition's arguments, then passes each result to
//! the next step. The composition takes the first step's parameters,
//! returns what the last step returns, throws the union of the steps'
//! errors and is `async` when a step is.
//!
//! A composition of compositions is flat: its steps are recorded in
//! `Analysis::compositions` (by the `then` node, or by the slice of a known
//! chain, `oneToFour[1..]`), and a `function` declared with one keeps its
//! chain, so that a slice of it is a composition of some of its steps
//! (`FN093` out of range; `FN095` on a function value without a chain). A
//! following step has exactly one parameter (`FN090`), accepts the
//! previous result (`FN091`) and can't mutate it (`FN092`: it receives a
//! temporary).

use std::sync::Arc;

use crate::check::Walker;
use crate::diagnostics::codes::types::{FN090, FN091, FN092, FN093, FN095};
use crate::ids::{NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{FnSig, PathRoot, TypeKind};

impl Walker<'_> {
	/// `f then g then …`
	pub(super) fn check_compose(&mut self, n: NodeId, operands: NodeList<'_>) -> TypeId {
		let mut steps = Vec::new();
		for operand in operands.iter() {
			self.check(operand, None);
			// the composition holds its steps: a closure copies what it captures
			self.escape(operand, n);
			match self.known_chain(operand) {
				Some(chain) => steps.extend(chain),
				None => steps.push(operand),
			}
		}
		self.a.compositions.insert(n, steps.clone());
		self.memo(n, |w| w.composition_type(n, &steps))
	}

	/// The steps of a known composition a node designates: a `then`, a slice
	/// of a chain, or a `function` declared with one.
	pub(super) fn known_chain(&self, n: NodeId) -> Option<Vec<NodeId>> {
		let n = self.strip_groups(n);
		if let Some(steps) = self.a.compositions.get(&n) {
			return Some(steps.clone());
		}
		let Resolution::Local(symbol) = self.a.resolution(n) else {
			return None;
		};
		let data = self.a.symbols.symbol(symbol);
		if data.kind != SymbolKind::Function {
			return None;
		}
		let View::Function { value, .. } = self.view(data.decl_node) else {
			return None;
		};
		self.a.compositions.get(&self.strip_groups(value)).cloned()
	}

	/// The type of a composition of these steps (checked already).
	fn composition_type(&mut self, n: NodeId, steps: &[NodeId]) -> TypeId {
		let types: Vec<TypeId> = steps.iter().map(|&step| self.type_of(step)).collect();
		if types.iter().any(|&ty| self.s.types.is_recovering(ty)) {
			return TypeId::UNKNOWN;
		}
		let mut signatures: Vec<FnSig> = Vec::with_capacity(types.len());
		for (&step, &ty) in steps.iter().zip(&types) {
			let TypeKind::Function(sig) = self.s.types.kind(ty) else {
				let message = format!(
					"`{}` is a `{}`, not a function: only functions are composed",
					self.tree.text(step.index()),
					self.display(ty)
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::values::E_TYPE_1, step, message);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			signatures.push(self.s.types.sig(sig));
		}
		let Some(first) = signatures.first().copied() else {
			return TypeId::UNKNOWN;
		};
		let mut result = first.ret;
		let mut throws = vec![first.throws];
		let mut is_async = first.is_async;
		for (index, sig) in signatures.iter().enumerate().skip(1) {
			let step = steps[index];
			let name = self.tree.text(step.index()).to_string();
			let params = self.s.types.params(sig.params).to_vec();
			let [param] = params.as_slice() else {
				let message = format!(
					"`{name}` takes {} parameters: a following step of a composition takes exactly one, the result of the step before it",
					params.len()
				);
				let diagnostic = self.diagnostic(&FN090, n, message);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if self
				.s
				.types
				.assignable(result, param.ty, crate::types::Mode::Copy)
				.is_err()
			{
				let message = format!(
					"`{name}` expects a `{}`, but receives a `{}`",
					self.display(param.ty),
					self.display(result)
				);
				let diagnostic = self.diagnostic(&FN091, n, message);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let (mutates, invalidates) = self.s.types.effects(sig.effects);
			let changes_parameter = mutates
				.iter()
				.chain(invalidates)
				.any(|&path| matches!(self.s.types.path_data(path).0.root, PathRoot::Param(0)));
			if changes_parameter {
				let message = format!(
					"`{name}` changes its parameter, but in a composition it receives a temporary value: call it on a place instead"
				);
				let diagnostic = self.diagnostic(&FN092, n, message);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			result = sig.ret;
			throws.push(sig.throws);
			is_async |= sig.is_async;
		}
		let throws = self.s.types.union(&throws);
		self.s.types.function(FnSig {
			ret: result,
			throws,
			is_async,
			..first
		})
	}

	/// `chain[range]`, `chain[index]` on a known composition: some of its
	/// steps (`FN093` out of range); `None` when the target is no function.
	/// `FN095` on a function value without a chain.
	pub(super) fn check_composition_slice(
		&mut self,
		n: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Option<TypeId> {
		let target_ty = self.check(target, None);
		if !matches!(self.s.types.kind(target_ty), TypeKind::Function(_)) {
			return None;
		}
		let tree = Arc::clone(&self.tree);
		let Some(chain) = self.known_chain(target) else {
			return Some(self.memo(n, |w| {
				let message = format!(
					"`{}` is a plain `{}`: it has no composition chain to slice, only a function declared with `then` has one",
					tree.text(target.index()),
					w.display(target_ty)
				);
				let diagnostic = w.diagnostic(&FN095, n, message);
				w.report(diagnostic);
				TypeId::UNKNOWN
			}));
		};
		let count = chain.len() as i128;
		let constant = |w: &mut Self, node: Option<NodeId>| -> Option<i128> {
			let node = node?;
			w.check(node, Some(TypeId::INTEGER));
			w.constant_integer(node).map(|value| value.as_i128())
		};
		let (start, end) = match syntax::view(&tree, index) {
			View::Range {
				start,
				end,
				inclusive,
			} => {
				let first = match start {
					Some(_) => constant(self, start),
					None => Some(0),
				};
				let last = match end {
					Some(_) => constant(self, end).map(|end| if inclusive { end + 1 } else { end }),
					None => Some(count),
				};
				(first, last)
			}
			_ => {
				let at = constant(self, Some(index));
				(at, at.map(|at| at + 1))
			}
		};
		let steps = match (start, end) {
			(Some(start), Some(end)) if 0 <= start && start < end && end <= count => {
				chain[start as usize..end as usize].to_vec()
			}
			_ => {
				return Some(self.memo(n, |w| {
					let message = format!(
						"this slice is out of the {count} steps of `{}`",
						tree.text(target.index())
					);
					let diagnostic = w.diagnostic(&FN093, n, message);
					w.report(diagnostic);
					TypeId::UNKNOWN
				}));
			}
		};
		self.a.compositions.insert(n, steps.clone());
		Some(self.memo(n, |w| w.composition_type(n, &steps)))
	}
}
