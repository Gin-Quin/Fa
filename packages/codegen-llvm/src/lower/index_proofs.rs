//! The index proofs and the never-negative places of the analysis
//! (`fa_analyzer::proofs`, `design/decisions.md`, "Index proofs for fields"
//! and "Never-negative places"): `nodes[node.next]` needs no bounds check
//! when every value that can reach that index is an index of `nodes`, and
//! `nodes` never shrinks; `items[position]` doesn't count from the end when
//! every value of `position` is never negative.
//!
//! The index proofs need the whole program in one file (a program lowered
//! as one module from one file); the never-negative places also work per
//! unit, from the program's failed claims (`facts::PROOF_CLAIMS`).

use fa_analyzer::ids::NodeId;
use fa_analyzer::proofs::{IndexFacts, ProofScope, index_facts};

use super::Lowering;

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether the index of this `array[index]` node is proven.
	pub(crate) fn proven_index(&self, node: NodeId) -> bool {
		self.with_index_facts(|facts| facts.proven_index(node))
	}

	/// Whether an integer node is proven never negative by the facts of
	/// the places it reads (fields, locals, parameters, results).
	pub(crate) fn proven_non_negative(&self, node: NodeId) -> bool {
		self.with_index_facts(|facts| facts.proven_non_negative(node))
	}

	/// The interval of an integer node the lowering may rely on to drop a
	/// check (`fa_analyzer::proofs::reliable_interval`).
	pub(crate) fn reliable_interval(&self, node: NodeId) -> Option<fa_analyzer::types::Interval> {
		fa_analyzer::proofs::reliable_interval(
			self.analysis,
			self.types(),
			node,
			self.overflow_checks(),
		)
	}

	fn with_index_facts(&self, read: impl FnOnce(&IndexFacts) -> bool) -> bool {
		if self.index_proofs.borrow().is_none() {
			let facts = self.compute_index_facts();
			*self.index_proofs.borrow_mut() = Some(facts);
		}
		self.index_proofs.borrow().as_ref().is_some_and(read)
	}

	fn compute_index_facts(&self) -> IndexFacts {
		let checks = self.overflow_checks();
		if self.tasks.asynchronous {
			return IndexFacts::default();
		}
		// a build per unit: the places every unit stores to, resolved by the
		// driver (`facts::PROOF_CLAIMS`); the index proofs stay whole-program
		if !self.whole_program {
			let Some(failed) = self
				.program_facts
				.as_ref()
				.map(|facts| facts.consulted(crate::facts::PROOF_CLAIMS, self.fact_log.as_deref()))
			else {
				return IndexFacts::default();
			};
			let failed = |key: &str| failed.contains(key);
			let scope = ProofScope::Unit {
				module: self.session.module(self.file),
				failed: &failed,
			};
			return index_facts(self.tree, self.analysis, self.types(), scope, checks);
		}
		let scope = if self.other_files.is_empty() {
			ProofScope::WholeFile
		} else {
			ProofScope::Nothing
		};
		index_facts(self.tree, self.analysis, self.types(), scope, checks)
	}
}
