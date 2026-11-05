//! Index proofs for fields (`design/decisions.md`, "Index proofs for
//! fields"): `nodes[node.next]` needs no bounds check when every value that
//! can reach that index is an index of `nodes`, and `nodes` never shrinks.
//!
//! The array `X` is the only array of its type in the program: one `let` or
//! `mutable` of the file, every other value of the type being a parameter
//! that receives `X` at every call (directly, or from another such
//! parameter); `X` is never assigned, and is only indexed, measured, pushed
//! to, reserved or iterated (nothing removes an item). Its initial length
//! `k` is the length of its literal (0 otherwise).
//!
//! An index expression `e` is proven, `P(e)`, when it is an integer known
//! to be in `0..k` (its interval), `X.length - 1` with `k >= 1`, or a read of
//! a place every store of which is proven: a field `T.f` (every object
//! literal and assignment of the field), the items of an array type, the
//! values of a map type, a local (its value and assignments), a parameter
//! (every call's argument), the result of a function (every returned value),
//! through `try`, `??`, groups and `none`. A greatest fixpoint: every claim
//! holds until one of its stores is not proven. Values stay indices once
//! proven, since `X` never shrinks.
//!
//! A program lowered as one module from one file only (`whole_program`):
//! the stores of other files or units are not seen.

use fa_analyzer::analysis::{Argument, Callee, NodeFlags};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, TypeKind};
use rustc_hash::{FxHashMap, FxHashSet};

use super::Lowering;
use crate::proofs::{Goal, ProofFile, Prover};

/// The array of a proof, and what it proves.
#[derive(Default)]
pub(crate) struct IndexProofs {
	/// The array local, and its parameters.
	arrays: FxHashSet<SymbolId>,
	/// The index nodes proven.
	proven: FxHashSet<NodeId>,
	/// The integer nodes proven never negative (`Goal::NonNegative`).
	non_negative: FxHashSet<NodeId>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether the index of this `array[index]` node is proven.
	pub(crate) fn proven_index(&self, node: NodeId) -> bool {
		if self.index_proofs.borrow().is_none() {
			let proofs = self.compute_index_proofs();
			*self.index_proofs.borrow_mut() = Some(proofs);
		}
		self.index_proofs
			.borrow()
			.as_ref()
			.is_some_and(|proofs| proofs.proven.contains(&node))
	}

	/// Whether an integer node is proven never negative by the facts of
	/// the places it reads (fields, locals, parameters, results).
	pub(crate) fn proven_non_negative(&self, node: NodeId) -> bool {
		if self.index_proofs.borrow().is_none() {
			let proofs = self.compute_index_proofs();
			*self.index_proofs.borrow_mut() = Some(proofs);
		}
		self.index_proofs
			.borrow()
			.as_ref()
			.is_some_and(|proofs| proofs.non_negative.contains(&node))
	}

	fn compute_index_proofs(&self) -> IndexProofs {
		let mut result = IndexProofs::default();
		if self.tasks.asynchronous {
			return result;
		}
		// a build per unit: the places every unit stores to, resolved by the
		// driver (`facts::PROOF_CLAIMS`); the index proofs stay whole-program
		if !self.whole_program {
			if let Some(failed) = self
				.program_facts
				.as_ref()
				.and_then(|facts| facts.set(crate::facts::PROOF_CLAIMS))
			{
				let file = ProofFile::new(self.tree, self.analysis, self.types())
					.with_module(self.session.module(self.file));
				result.non_negative = self.compute_non_negative(&file, Some(failed));
			}
			return result;
		}
		if !self.other_files.is_empty() {
			return result;
		}
		let count = self.tree.nodes.len();
		// `Json.parse` builds objects, arrays and maps out of any integers
		let parses = (0..count).any(|at| {
			matches!(self.analysis.call(NodeId::new(at)).map(|info| info.callee),
				Some(Callee::Builtin(id)) if fa_analyzer::builtins::get(id).path == "Json.parse")
		});
		if parses {
			return result;
		}
		let file = ProofFile::new(self.tree, self.analysis, self.types());
		result.non_negative = self.compute_non_negative(&file, None);
		// the candidate arrays: a let or mutable of an array of objects
		let types = self.types();
		let mut by_type: FxHashMap<TypeId, Vec<SymbolId>> = FxHashMap::default();
		for (index, symbol) in self.analysis.symbols.symbols().iter().enumerate() {
			if !matches!(symbol.kind, SymbolKind::Let | SymbolKind::Mutable) {
				continue;
			}
			if let TypeKind::Collection(CollKind::Array, item, _) = types.kind(symbol.ty)
				&& matches!(types.kind(item), TypeKind::Object(_))
			{
				by_type
					.entry(symbol.ty)
					.or_default()
					.push(SymbolId(index as u32));
			}
		}
		for (array_type, symbols) in by_type {
			let [array] = symbols[..] else {
				continue;
			};
			let Some(arrays) = self.array_family(array, array_type, &file) else {
				continue;
			};
			let declaration = self.analysis.symbols.symbol(array).decl_node;
			let initial = match self.view(declaration) {
				View::Declaration {
					value: Some(value), ..
				} => match self.view(value) {
					View::Array { items } => items.len() as u64,
					_ => 0,
				},
				_ => continue,
			};
			let mut prover = Prover::new(
				&file,
				Goal::Index {
					arrays: &arrays,
					initial,
				},
			);
			// the indices of the array, then the fixpoint of their claims
			let indices: Vec<(NodeId, NodeId)> = (0..count)
				.filter_map(|at| match self.view(NodeId::new(at)) {
					View::Index { target, index } if prover.is_array(target) => {
						Some((NodeId::new(at), index))
					}
					_ => None,
				})
				.collect();
			let roots: Vec<NodeId> = indices.iter().map(|&(_, index)| index).collect();
			prover.fixpoint(&roots);
			for (node, index) in indices {
				if prover.proves(index) {
					result.proven.insert(node);
				}
			}
			result.arrays.extend(arrays);
		}
		result
	}

	/// The never-negative proofs of the indices that may be negative by
	/// their types and intervals: the index of every `x[index]` and every
	/// argument of a built-in `get`.
	fn compute_non_negative(
		&self,
		file: &ProofFile<'_>,
		program: Option<&std::collections::BTreeSet<String>>,
	) -> FxHashSet<NodeId> {
		let mut roots = Vec::new();
		for at in 0..self.tree.nodes.len() {
			let node = NodeId::new(at);
			match self.view(node) {
				View::Index { index, .. } => roots.push(index),
				View::Call { .. } => {
					let Some(info) = self.analysis.call(node) else {
						continue;
					};
					let Callee::Builtin(id) = info.callee else {
						continue;
					};
					if !fa_analyzer::builtins::get(id).path.ends_with("#get") {
						continue;
					}
					for argument in &info.arguments {
						if let Argument::Given(value) = argument {
							roots.push(*value);
						}
					}
				}
				_ => {}
			}
		}
		let types = self.types();
		roots.retain(|&root| {
			types.as_int(self.analysis.type_of(root)).is_some()
				&& self
					.analysis
					.interval(root)
					.is_none_or(|interval| interval.lo < ethnum::I256::ZERO)
		});
		if roots.is_empty() {
			return FxHashSet::default();
		}
		let goal = Goal::NonNegative {
			checked: self.overflow_checks(),
		};
		let mut prover = match program {
			Some(failed) => Prover::new(file, goal).with_program(failed),
			None => {
				let mut prover = Prover::new(file, goal);
				prover.fixpoint(&roots);
				prover
			}
		};
		roots
			.into_iter()
			.filter(|&root| prover.proves(root))
			.collect()
	}

	/// The array and the parameters that receive it, when it is the only
	/// array of its type and never shrinks.
	fn array_family(
		&self,
		array: SymbolId,
		array_type: TypeId,
		file: &ProofFile<'_>,
	) -> Option<FxHashSet<SymbolId>> {
		let parents = &file.parents;
		let declaration = self.analysis.symbols.symbol(array).decl_node;
		let initializer = match self.view(declaration) {
			View::Declaration { value, .. } => value,
			_ => None,
		};
		let mut family: FxHashSet<SymbolId> = FxHashSet::default();
		family.insert(array);
		let mut names = Vec::new();
		for at in 0..self.tree.nodes.len() {
			let node = NodeId::new(at);
			if self.analysis.type_of(node) != array_type
				|| self.analysis.has_flags(node, NodeFlags::TYPE_EXPR)
				|| Some(node) == initializer
				|| node == declaration
			{
				continue;
			}
			match self.view(node) {
				View::Name(_) => match self.analysis.resolution(node) {
					Resolution::Local(symbol) => {
						let data = self.analysis.symbols.symbol(symbol);
						if symbol != array && !matches!(data.kind, SymbolKind::Parameter { .. }) {
							return None;
						}
						if matches!(data.kind, SymbolKind::Parameter { .. }) {
							family.insert(symbol);
						}
						names.push(node);
					}
					_ => return None,
				},
				// a parameter's own node
				_ if self
					.analysis
					.symbols
					.symbol_of_decl(node)
					.is_some_and(|symbol| {
						matches!(
							self.analysis.symbols.symbol(symbol).kind,
							SymbolKind::Parameter { .. }
						)
					}) => {}
				_ => return None,
			}
		}
		// every use: indexed, measured, pushed to, reserved, iterated, or given
		// to a parameter of the family
		for &name in &names {
			let parent = parents.get(name.index()).copied().unwrap_or(usize::MAX);
			if parent == usize::MAX {
				return None;
			}
			let fine = match self.view(NodeId::new(parent)) {
				// the name of a parameter, in its declaration
				View::Assignment {
					target,
					value: None,
					..
				} => target == name,
				View::Index { target, .. } => target == name,
				// `mutates nodes`: the callee's own uses are checked
				View::Lambda { signature, .. } => {
					signature.mutates.iter().any(|effect| effect == name)
						|| signature.invalidates.iter().any(|effect| effect == name)
				}
				View::For { iterable, .. } => iterable == name,
				View::Access { target, members } if target == name && members.len() == 1 => {
					match self.analysis.resolution(members.first().expect("a member")) {
						Resolution::Builtin(id) => matches!(
							fa_analyzer::builtins::get(id).path,
							"Array#length" | "Array#push" | "Array#reserve" | "Array#get"
						),
						_ => false,
					}
				}
				View::Call { .. } => {
					let info = self.analysis.call(NodeId::new(parent))?;
					let Callee::Function(function) = info.callee else {
						return None;
					};
					let fn_info = self.analysis.function(function)?;
					info.arguments
						.iter()
						.enumerate()
						.all(|(position, argument)| {
							!matches!(argument, Argument::Given(value) if *value == name)
								|| fn_info.params.get(position).is_some_and(|param| {
									self.analysis.symbols.symbol(param.symbol).ty == array_type
								})
						})
				}
				_ => false,
			};
			if !fine {
				return None;
			}
		}
		// every call of a function with a parameter of the family passes it a
		// member of the family; and the function is only called directly
		for &symbol in family.iter().filter(|&&symbol| symbol != array) {
			let function = file.function_of_param(symbol)?;
			let SymbolKind::Parameter { index } = self.analysis.symbols.symbol(symbol).kind else {
				return None;
			};
			for at in 0..self.tree.nodes.len() {
				let node = NodeId::new(at);
				match self.analysis.resolution(node) {
					Resolution::Local(callee)
						if self.analysis.symbols.symbol(callee).decl_node == function =>
					{
						let parent = parents.get(at).copied().unwrap_or(usize::MAX);
						let direct = parent != usize::MAX
							&& matches!(self.view(NodeId::new(parent)), View::Call { callee, .. } if callee == node);
						if !direct {
							return None;
						}
					}
					_ => {}
				}
				let Some(info) = self.analysis.call(node) else {
					continue;
				};
				if info.callee != Callee::Function(function) {
					continue;
				}
				let given = match info.arguments.get(index as usize) {
					Some(Argument::Given(value)) => *value,
					_ => return None,
				};
				let member = matches!(self.view(given), View::Name(_))
					&& matches!(self.analysis.resolution(given),
						Resolution::Local(passed) if family.contains(&passed));
				if !member {
					return None;
				}
			}
		}
		Some(family)
	}
}
