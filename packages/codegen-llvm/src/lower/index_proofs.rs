//! The index proofs and the never-negative places of the analysis
//! (`fa_analyzer::proofs`, `design/decisions.md`, "Index proofs for fields"
//! and "Never-negative places"): `nodes[node.next]` needs no bounds check
//! when every value that can reach that index is an index of `nodes`, and
//! `nodes` never shrinks; `items[position]` doesn't count from the end when
//! every value of `position` is never negative.
//!
//! Both work per unit, from the program's failed claims
//! (`facts::PROOF_CLAIMS`). Index proofs require an array private to its
//! file and places whose values no other file may supply.

use fa_analyzer::ids::{NodeId, SymbolId};
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

	/// Exact length of a closed, never-resized array family.
	pub(crate) fn fixed_array_length(&self, symbol: SymbolId) -> Option<u64> {
		self.with_index_facts(|facts| facts.fixed_array_length(symbol))
	}

	fn with_index_facts<R: Default>(&self, read: impl FnOnce(&IndexFacts) -> R) -> R {
		if self.index_proofs.borrow().is_none() {
			let facts = self.compute_index_facts();
			*self.index_proofs.borrow_mut() = Some(facts);
		}
		self.index_proofs
			.borrow()
			.as_ref()
			.map(read)
			.unwrap_or_default()
	}

	fn compute_index_facts(&self) -> IndexFacts {
		// The current analysis may be one key's snapshot. Its stores do
		// not summarize the other keys, and its node indices are the same.
		if !self.session.analysis(self.file).key_loop_bodies.is_empty() {
			return IndexFacts::default();
		}
		let checks = self.overflow_checks();
		// asynchronous code too: the claims are flow-insensitive (every
		// store of a place, wherever and whenever it runs: a task, a callee
		// running while a function waits at `await`, a parallel body, a
		// worker), and an array proven never shrinks anywhere; only the
		// analysis's intervals are flow facts, and it drops those of the
		// values others may write at each `await`
		// a build per unit: the places every unit stores to, resolved by the
		// driver (`facts::PROOF_CLAIMS`)
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
		if self.other_files.is_empty() {
			return index_facts(
				self.tree,
				self.analysis,
				self.types(),
				ProofScope::WholeFile,
				checks,
			);
		}
		// A multi-file program lowered as one module uses the same claims
		// as a package built per unit. Resolve them once for the module;
		// the index facts themselves are cached separately in each FileState.
		let claims = self.program_proof_claims.get_or_init(|| {
			let files: Vec<_> = std::iter::once(self.file)
				.chain(self.other_files.keys().copied())
				.map(|file| {
					fa_analyzer::proofs::ProofFile::new(
						self.session.tree(file),
						self.session.analysis(file),
						self.types(),
					)
					.with_module(self.session.module(file))
				})
				.collect();
			fa_analyzer::proofs::resolve_claims(&fa_analyzer::proofs::unit_claims_of_goal(
				&files, checks,
			))
		});
		let failed = |key: &str| claims.contains(key);
		let scope = ProofScope::Unit {
			module: self.session.module(self.file),
			failed: &failed,
		};
		index_facts(self.tree, self.analysis, self.types(), scope, checks)
	}
}
