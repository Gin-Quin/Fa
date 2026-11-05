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

use fa_analyzer::analysis::{Argument, Callee, FieldValue, NodeFlags};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{ArithOp, View};
use fa_analyzer::types::{CollKind, TypeKind};
use rustc_hash::{FxHashMap, FxHashSet};

use super::Lowering;

/// A place whose every stored value must be an index of the array.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum Claim {
	/// The field `name` of the objects of a type.
	Field(TypeId, fa_analyzer::ids::Sym),
	/// The items of the arrays of a type.
	Items(TypeId),
	/// The values of the maps of a type.
	MapValues(TypeId),
	/// The result of a function (its `function` node).
	Return(NodeId),
	/// A parameter of a function, by position.
	Param(NodeId, usize),
	/// A local.
	Local(SymbolId),
}

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

/// What a proof establishes of the values stored in its places.
#[derive(Copy, Clone)]
enum Goal<'p> {
	/// An index of one of these arrays (the array and its parameters),
	/// whose initial length is `initial`.
	Index {
		arrays: &'p FxHashSet<SymbolId>,
		initial: u64,
	},
	/// Never negative. With overflow checks (`checked`), sums and products
	/// of such values are too; without, an overflow wraps around, so the
	/// values stored are at most [`STEP_BOUND`] and the places only grow by
	/// steps of at most one (`position += 1`): each place holds at most
	/// `STEP_BOUND` plus the number of steps executed so far, which stays
	/// far below 2^63.
	NonNegative { checked: bool },
}

/// The largest value stored into a never-negative place without overflow
/// checks (see [`Goal::NonNegative`]).
const STEP_BOUND: u64 = 1 << 32;

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
		if !self.whole_program || !self.other_files.is_empty() || self.tasks.asynchronous {
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
		let parents = self.parents();
		result.non_negative = self.compute_non_negative(&parents);
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
			let Some(arrays) = self.array_family(array, array_type, &parents) else {
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
			let mut prover = Prover {
				lowering: self,
				parents: &parents,
				goal: Goal::Index {
					arrays: &arrays,
					initial,
				},
				failed: FxHashSet::default(),
				claims: FxHashSet::default(),
			};
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
	fn compute_non_negative(&self, parents: &[usize]) -> FxHashSet<NodeId> {
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
		let mut prover = Prover {
			lowering: self,
			parents,
			goal: Goal::NonNegative {
				checked: self.overflow_checks(),
			},
			failed: FxHashSet::default(),
			claims: FxHashSet::default(),
		};
		prover.fixpoint(&roots);
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
		parents: &[usize],
	) -> Option<FxHashSet<SymbolId>> {
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
			let function = self.function_of_param(symbol, parents)?;
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

	/// The `function` node a parameter belongs to.
	fn function_of_param(&self, symbol: SymbolId, parents: &[usize]) -> Option<NodeId> {
		let mut at = self.analysis.symbols.symbol(symbol).decl_node.index();
		loop {
			let parent = *parents.get(at)?;
			if parent == usize::MAX {
				return None;
			}
			match self.view(NodeId::new(parent)) {
				View::Function { .. } => return Some(NodeId::new(parent)),
				View::Lambda { .. } | View::Parameters { .. } => at = parent,
				_ => return None,
			}
		}
	}
}

/// The proof of the indices of one array.
struct Prover<'p, 'l, 'a, 'ctx> {
	lowering: &'l Lowering<'a, 'ctx>,
	parents: &'p [usize],
	goal: Goal<'p>,
	failed: FxHashSet<Claim>,
	claims: FxHashSet<Claim>,
}

impl Prover<'_, '_, '_, '_> {
	fn is_array(&self, node: NodeId) -> bool {
		let Goal::Index { arrays, .. } = self.goal else {
			return false;
		};
		matches!(self.lowering.view(node), View::Name(_))
			&& matches!(self.lowering.analysis.resolution(node),
				Resolution::Local(symbol) if arrays.contains(&symbol))
	}

	/// The greatest fixpoint of the claims the roots depend on: every claim
	/// holds until one of its stores is not proven.
	fn fixpoint(&mut self, roots: &[NodeId]) {
		loop {
			let before = self.failed.len();
			for &root in roots {
				self.proves(root);
			}
			let claims: Vec<Claim> = self.claims.iter().copied().collect();
			for claim in claims {
				if !self.failed.contains(&claim) && !self.holds(claim) {
					self.failed.insert(claim);
				}
			}
			if self.failed.len() == before {
				break;
			}
		}
	}

	/// Whether a constant satisfies the goal.
	fn constant(&self, value: ethnum::I256) -> bool {
		match self.goal {
			Goal::Index { initial, .. } => {
				value >= ethnum::I256::ZERO && value < ethnum::I256::from(initial)
			}
			Goal::NonNegative { checked } => {
				value >= ethnum::I256::ZERO && (checked || value <= ethnum::I256::from(STEP_BOUND))
			}
		}
	}

	/// Whether every value of an interval satisfies the goal.
	fn within(&self, lo: ethnum::I256, hi: ethnum::I256) -> bool {
		self.constant(lo) && self.constant(hi)
	}

	/// `P(e)` under the claims not failed so far (recording the claims used).
	fn proves(&mut self, node: NodeId) -> bool {
		let lowering = self.lowering;
		if let Some(interval) = lowering.analysis.interval(node)
			&& self.within(interval.lo, interval.hi)
		{
			return true;
		}
		if matches!(self.goal, Goal::NonNegative { .. })
			&& let Some(range) = lowering.types().as_int(lowering.analysis.type_of(node))
			&& self.within(range.min, range.max)
		{
			return true;
		}
		match lowering.view(node) {
			View::Integer(value) => self.constant(value),
			// with overflow checks, sums and products of non-negative values
			View::Arithmetic {
				op: ArithOp::Add | ArithOp::Multiply,
				operands,
			} if matches!(self.goal, Goal::NonNegative { checked: true }) => {
				operands.iter().all(|operand| self.proves(operand))
			}
			View::Group(inner) | View::Try(inner) => self.proves(inner),
			View::None => true,
			View::Fallback { operands } => operands.iter().all(|operand| self.proves(operand)),
			View::Name(_) => match lowering.analysis.resolution(node) {
				Resolution::Local(symbol) => {
					let data = lowering.analysis.symbols.symbol(symbol);
					match data.kind {
						SymbolKind::Let | SymbolKind::Mutable => self.claim(Claim::Local(symbol)),
						SymbolKind::Parameter { index } => {
							match lowering.function_of_param(symbol, self.parents) {
								Some(function) => {
									self.claim(Claim::Param(function, index as usize))
								}
								None => false,
							}
						}
						_ => false,
					}
				}
				_ => false,
			},
			View::Access { target, members } if members.len() == 1 => {
				let member = members.first().expect("a member");
				match self.field_name(member) {
					Some(name) => {
						let owner = lowering.analysis.type_of(target);
						self.claim(Claim::Field(owner, name))
					}
					None => false,
				}
			}
			View::Index { target, .. } => {
				let ty = lowering.analysis.type_of(target);
				matches!(
					lowering.types().kind(ty),
					TypeKind::Collection(CollKind::Array, ..)
				) && self.claim(Claim::Items(ty))
			}
			View::Call { callee, .. } => {
				let Some(info) = lowering.analysis.call(node) else {
					return false;
				};
				match info.callee {
					Callee::Function(function) => self.claim(Claim::Return(function)),
					Callee::Builtin(id) => {
						let View::Access { target, .. } = lowering.view(callee) else {
							return false;
						};
						let receiver = lowering.analysis.type_of(target);
						match fa_analyzer::builtins::get(id).path {
							"Map#get" => self.claim(Claim::MapValues(receiver)),
							"Array#pop" | "Array#get" => self.claim(Claim::Items(receiver)),
							_ => false,
						}
					}
					Callee::Conversion(_) => match info.arguments.first() {
						Some(Argument::Given(value)) => self.proves(*value),
						_ => false,
					},
					_ => false,
				}
			}
			// `X.length - 1`, with at least one item from the start
			View::Arithmetic {
				op: ArithOp::Subtract,
				operands,
			} if operands.len() == 2
				&& matches!(self.goal, Goal::Index { initial, .. } if initial >= 1) =>
			{
				let first = operands.first().expect("two operands");
				let second = operands.get(1).expect("two operands");
				let one = lowering.analysis.interval(second).is_some_and(|interval| {
					interval.lo == interval.hi && interval.lo == ethnum::I256::ONE
				});
				let length = match lowering.view(first) {
					View::Access { target, members } if members.len() == 1 => {
						self.is_array(target)
							&& matches!(lowering.analysis.resolution(members.first().expect("a member")),
								Resolution::Builtin(id) if fa_analyzer::builtins::get(id).path == "Array#length")
					}
					_ => false,
				};
				one && length
			}
			_ => false,
		}
	}

	fn claim(&mut self, claim: Claim) -> bool {
		self.claims.insert(claim);
		!self.failed.contains(&claim)
	}

	/// Whether every store of a claim's place is proven.
	fn holds(&mut self, claim: Claim) -> bool {
		let lowering = self.lowering;
		let count = lowering.tree.nodes.len();
		match claim {
			Claim::Local(symbol) => {
				if lowering.referenced_locals().contains(&symbol) {
					return false;
				}
				let declaration = lowering.analysis.symbols.symbol(symbol).decl_node;
				let initial = match lowering.view(declaration) {
					View::Declaration {
						value: Some(value), ..
					} => self.proves(value),
					_ => false,
				};
				initial
					&& (0..count).all(|at| {
						let node = NodeId::new(at);
						match lowering.view(node) {
							View::Assignment { target, value, .. }
								if self.names(target, symbol) =>
							{
								value.is_some_and(|value| self.proves(value))
							}
							View::CompoundAssignment { op, target, value }
								if self.names(target, symbol) =>
							{
								self.step(op, value)
							}
							View::UpdateFields { target, .. } => !self.names(target, symbol),
							View::Name(_) if self.names(node, symbol) => {
								!self.changed_elsewhere(node)
							}
							_ => true,
						}
					})
			}
			// every call is seen: the function is never a value
			Claim::Param(function, index) => {
				self.called_directly(function)
					&& (0..count).all(|at| {
						let node = NodeId::new(at);
						match lowering.analysis.call(node) {
							Some(info) if info.callee == Callee::Function(function) => {
								match info.arguments.get(index) {
									Some(Argument::Given(value)) => self.proves(*value),
									_ => false,
								}
							}
							_ => true,
						}
					})
			}
			Claim::Return(function) => {
				let Some(info) = lowering.analysis.function(function) else {
					return false;
				};
				let body = info.body;
				let last = match lowering.view(body) {
					View::Block { statements } => statements.last(),
					_ => Some(body),
				};
				let last_ok = match last {
					Some(last) if !matches!(lowering.view(last), View::Return(_)) => {
						self.proves(last)
					}
					_ => true,
				};
				last_ok
					&& (0..count).all(|at| match lowering.view(NodeId::new(at)) {
						View::Return(value) if self.inside(NodeId::new(at), function) => {
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					})
			}
			Claim::Field(owner, name) => (0..count).all(|at| {
				let node = NodeId::new(at);
				if let Some(object) = lowering.analysis.object(node)
					&& object.ty == owner
				{
					return object
						.fields
						.iter()
						.filter(|(field, _)| *field == name)
						.all(|(_, value)| match value {
							FieldValue::Given(value) | FieldValue::Default(value) => {
								self.proves(*value)
							}
							_ => false,
						});
				}
				match lowering.view(node) {
					// a write whose owner is unknown must be proven too
					View::Assignment { target, value, .. } => match self.field_target(target, name)
					{
						Some(target_owner)
							if target_owner == owner || target_owner == TypeId::UNKNOWN =>
						{
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					},
					View::CompoundAssignment { op, target, value } => {
						match self.field_target(target, name) {
							Some(target_owner)
								if target_owner == owner || target_owner == TypeId::UNKNOWN =>
							{
								self.step(op, value)
							}
							_ => true,
						}
					}
					View::UpdateFields { .. } => false,
					// a reference to the field, or the field given to a
					// parameter that the callee changes
					View::Access { .. } => {
						!matches!(self.field_target(node, name),
							Some(target_owner) if target_owner == owner || target_owner == TypeId::UNKNOWN)
							|| !self.changed_elsewhere(node)
					}
					_ => true,
				}
			}),
			Claim::Items(array_type) => (0..count).all(|at| {
				let node = NodeId::new(at);
				match lowering.view(node) {
					View::Array { items } if lowering.analysis.type_of(node) == array_type => {
						items.iter().all(|item| self.proves(item))
					}
					View::Assignment { target, value, .. } => match lowering.view(target) {
						View::Index { target: array, .. }
							if lowering.analysis.type_of(array) == array_type =>
						{
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					},
					View::CompoundAssignment { op, target, value } => match lowering.view(target) {
						View::Index { target: array, .. }
							if lowering.analysis.type_of(array) == array_type =>
						{
							self.step(op, value)
						}
						_ => true,
					},
					View::Index { target: array, .. }
						if lowering.analysis.type_of(array) == array_type =>
					{
						!self.changed_elsewhere(node)
					}
					View::Call { callee, .. } => {
						let View::Access { target, .. } = lowering.view(callee) else {
							return true;
						};
						if lowering.analysis.type_of(target) != array_type {
							return true;
						}
						let Some(info) = lowering.analysis.call(node) else {
							return false;
						};
						let Callee::Builtin(id) = info.callee else {
							return true;
						};
						match fa_analyzer::builtins::get(id).path {
							"Array#push" => info.arguments.iter().all(|argument| match argument {
								Argument::Given(value) => self.proves(*value),
								Argument::Receiver(_) => true,
								_ => false,
							}),
							"Array#pop" | "Array#get" | "Array#length" | "Array#reserve"
							| "Array#isEmpty" | "Array#clear" => true,
							_ => false,
						}
					}
					_ => true,
				}
			}),
			Claim::MapValues(map_type) => (0..count).all(|at| {
				let node = NodeId::new(at);
				match lowering.view(node) {
					View::Assignment { target, value, .. } => match lowering.view(target) {
						View::Index { target: map, .. }
							if lowering.analysis.type_of(map) == map_type =>
						{
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					},
					View::CompoundAssignment { op, target, value } => match lowering.view(target) {
						View::Index { target: map, .. }
							if lowering.analysis.type_of(map) == map_type =>
						{
							self.step(op, value)
						}
						_ => true,
					},
					View::Index { target: map, .. }
						if lowering.analysis.type_of(map) == map_type =>
					{
						!self.changed_elsewhere(node)
					}
					View::Use { .. } | View::Reuse { .. } | View::Provide { .. } => true,
					View::Call { callee, .. } => {
						let View::Access { target, .. } = lowering.view(callee) else {
							return true;
						};
						if lowering.analysis.type_of(target) != map_type {
							return true;
						}
						let Some(Callee::Builtin(id)) =
							lowering.analysis.call(node).map(|info| info.callee)
						else {
							return true;
						};
						matches!(
							fa_analyzer::builtins::get(id).path,
							"Map#get" | "Map#delete" | "Map#has" | "Map#size" | "Map#reserve"
						)
					}
					View::Array { .. } | View::Object { .. }
						if lowering.analysis.type_of(node) == map_type =>
					{
						matches!(lowering.view(node), View::Array { items } if items.is_empty())
					}
					_ => true,
				}
			}),
		}
	}

	/// Whether a function is only called by name (`f(…)`, `x.f(…)`), never
	/// used as a value whose calls the claims of its parameters wouldn't see.
	/// A method is matched by its name, whatever its type.
	fn called_directly(&self, function: NodeId) -> bool {
		let lowering = self.lowering;
		let Some(symbol) = lowering
			.analysis
			.function(function)
			.and_then(|info| info.symbol)
		else {
			return false;
		};
		let name = lowering.analysis.symbols.symbol(symbol).name;
		(0..lowering.tree.nodes.len()).all(|at| {
			let node = NodeId::new(at);
			let named = match lowering.analysis.resolution(node) {
				Resolution::Local(used) => used == symbol && node != function,
				Resolution::Method(_, method) => method == name,
				_ => false,
			};
			if !named || lowering.analysis.symbols.symbol_of_decl(node) == Some(symbol) {
				return true;
			}
			// the callee: the name itself, or the access whose last member it is
			let mut callee = node;
			let parent = self.parents.get(at).copied().unwrap_or(usize::MAX);
			if parent != usize::MAX
				&& matches!(lowering.view(NodeId::new(parent)),
					View::Access { members, .. } if members.last() == Some(node))
			{
				callee = NodeId::new(parent);
			}
			let call = self.parents.get(callee.index()).copied().unwrap_or(usize::MAX);
			call != usize::MAX
				&& matches!(lowering.view(NodeId::new(call)), View::Call { callee: called, .. } if called == callee)
		})
	}

	/// Whether a compound assignment keeps a place's values under the goal:
	/// never for an index; for a never-negative place, adding (or, with
	/// overflow checks, multiplying by) a never-negative value, and without
	/// overflow checks only a step of at most one (see [`Goal::NonNegative`]).
	fn step(&mut self, op: ArithOp, value: NodeId) -> bool {
		match self.goal {
			Goal::Index { .. } => false,
			Goal::NonNegative { checked: true } => {
				matches!(op, ArithOp::Add | ArithOp::Multiply) && self.proves(value)
			}
			Goal::NonNegative { checked: false } => {
				op == ArithOp::Add
					&& self
						.lowering
						.analysis
						.interval(value)
						.map(|interval| (interval.lo, interval.hi))
						.or(match self.lowering.view(value) {
							View::Integer(value) => Some((value, value)),
							_ => None,
						})
						.is_some_and(|(lo, hi)| lo >= ethnum::I256::ZERO && hi <= ethnum::I256::ONE)
			}
		}
	}

	/// Whether a place (a node designating it) may be changed under another
	/// name: the value of a `use`, `mutable use`, `reuse` or `provide`, or
	/// an argument (or receiver) of a call whose callee changes that
	/// parameter, or of a function value.
	fn changed_elsewhere(&self, place: NodeId) -> bool {
		let lowering = self.lowering;
		let mut at = place.index();
		// through groups
		let parent = loop {
			let parent = self.parents.get(at).copied().unwrap_or(usize::MAX);
			if parent == usize::MAX {
				return false;
			}
			if matches!(lowering.view(NodeId::new(parent)), View::Group(_)) {
				at = parent;
				continue;
			}
			break NodeId::new(parent);
		};
		let place = NodeId::new(at);
		match lowering.view(parent) {
			View::Use { value, mutable, .. } => mutable && value == place,
			View::Reuse { value, .. } | View::Provide { value, .. } => value == place,
			// a named argument: the call is one level up
			View::Assignment { value, .. } if value == Some(place) => {
				let grand = self
					.parents
					.get(parent.index())
					.copied()
					.unwrap_or(usize::MAX);
				grand != usize::MAX && self.changed_by_call(NodeId::new(grand), place)
			}
			View::Call { .. } => self.changed_by_call(parent, place),
			// the receiver of a method call: `place.method(…)`
			View::Access { target, .. } if target == place => {
				let grand = self
					.parents
					.get(parent.index())
					.copied()
					.unwrap_or(usize::MAX);
				grand != usize::MAX
					&& matches!(lowering.view(NodeId::new(grand)), View::Call { callee, .. } if callee == parent)
					&& self.changed_by_call(NodeId::new(grand), place)
			}
			_ => false,
		}
	}

	/// Whether a call may change the argument `place` through its parameter.
	fn changed_by_call(&self, call: NodeId, place: NodeId) -> bool {
		let lowering = self.lowering;
		let Some(info) = lowering.analysis.call(call) else {
			return false;
		};
		let Some(position) = info.arguments.iter().position(|argument| match argument {
			Argument::Given(value) | Argument::Receiver(value) => *value == place,
			Argument::Variadic(values) => values.contains(&place),
			_ => false,
		}) else {
			return false;
		};
		match info.callee {
			Callee::Function(function) => match lowering.analysis.function(function) {
				Some(function) => lowering.changed_parameters(function.ty).contains(&position),
				None => true,
			},
			Callee::Value => {
				let View::Call { callee, .. } = lowering.view(call) else {
					return true;
				};
				lowering
					.changed_parameters(lowering.analysis.type_of(callee))
					.contains(&position)
			}
			Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_) => false,
		}
	}

	/// The name of the field a member node designates.
	fn field_name(&self, member: NodeId) -> Option<fa_analyzer::ids::Sym> {
		let Resolution::Field(shape, index) = self.lowering.analysis.resolution(member) else {
			return None;
		};
		self.lowering
			.types()
			.shape_fields(shape)
			.get(index as usize)
			.map(|field| field.name)
	}

	/// Whether a place is the name of a local.
	fn names(&self, target: NodeId, symbol: SymbolId) -> bool {
		matches!(self.lowering.view(target), View::Name(_))
			&& self.lowering.analysis.resolution(target) == Resolution::Local(symbol)
	}

	/// The type owning the field `name` an assignment target writes, if it
	/// writes a field of that name (`x.name`, `a[i].name`).
	fn field_target(&self, target: NodeId, name: fa_analyzer::ids::Sym) -> Option<TypeId> {
		let lowering = self.lowering;
		let View::Access {
			target: owner,
			members,
		} = lowering.view(target)
		else {
			return None;
		};
		let last = members.last()?;
		if self.field_name(last) != Some(name) {
			return None;
		}
		if members.len() == 1 {
			return Some(lowering.analysis.type_of(owner));
		}
		// a deeper chain: the owner's type is not at hand, assume the worst
		Some(TypeId::UNKNOWN)
	}

	/// Whether a node is in the body of a function (not in a nested one).
	fn inside(&self, node: NodeId, function: NodeId) -> bool {
		let mut at = node.index();
		while let Some(&parent) = self.parents.get(at) {
			if parent == usize::MAX {
				return false;
			}
			if parent == function.index() {
				return true;
			}
			if matches!(
				self.lowering.view(NodeId::new(parent)),
				View::Function { .. } | View::Lambda { .. }
			) && parent != function.index()
				&& !self.is_own_lambda(parent, function)
			{
				return false;
			}
			at = parent;
		}
		false
	}

	/// Whether a lambda node is the function's own (its body's holder).
	fn is_own_lambda(&self, lambda: usize, function: NodeId) -> bool {
		matches!(self.lowering.view(function), View::Function { value, .. } if value.index() == lambda)
	}
}
