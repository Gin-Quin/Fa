//! The proofs over the stores of a program's places (`lower/index_proofs.rs`
//! uses them for the index proofs of fields and the never-negative places;
//! `facts.rs` publishes each unit's part of them for builds per unit).
//! A [`ProofFile`] is one analyzed file; a [`Prover`] establishes a
//! [`Goal`] for the values of its [`Claim`] places, a greatest fixpoint.

use fa_analyzer::analysis::{Analysis, Argument, Callee, FieldValue};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{self, ArithOp, View};
use fa_analyzer::types::{CollKind, PathRoot, TypeKind, TypeStore};
use fa_parser::TypedSyntaxTree;
use rustc_hash::FxHashSet;
use std::collections::{BTreeMap, BTreeSet};

use crate::facts::stable_type_name;

/// The fact of a unit that parses JSON: the objects, arrays and maps it
/// builds hold any integers, so no claim holds (`unit_claims`).
pub(crate) const JSON_PARSE: &str = "json";

/// A place whose every stored value must be an index of the array.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Claim {
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

/// What a proof establishes of the values stored in its places.
#[derive(Copy, Clone)]
pub(crate) enum Goal<'p> {
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
pub(crate) const STEP_BOUND: u64 = 1 << 32;

/// An analyzed file, with what the proofs read of it.
pub(crate) struct ProofFile<'a> {
	pub(crate) tree: &'a TypedSyntaxTree,
	pub(crate) analysis: &'a Analysis,
	types: &'a TypeStore,
	/// The parent of every node (`usize::MAX` for the root).
	pub(crate) parents: Vec<usize>,
	/// The locals that a reference designates a part of.
	referenced: FxHashSet<SymbolId>,
	/// The file's module, naming its own places in the facts.
	module: String,
}

impl<'a> ProofFile<'a> {
	pub(crate) fn new(
		tree: &'a TypedSyntaxTree,
		analysis: &'a Analysis,
		types: &'a TypeStore,
	) -> Self {
		let mut parents = vec![usize::MAX; tree.nodes.len()];
		for (index, node) in tree.nodes.iter().enumerate() {
			node.for_each_child(|child| {
				if child < parents.len() {
					parents[child] = index;
				}
			});
		}
		let mut file = ProofFile {
			tree,
			analysis,
			types,
			parents,
			referenced: FxHashSet::default(),
			module: String::new(),
		};
		// the locals below the value of a `use`, `reuse` or `provide`
		for index in 0..tree.nodes.len() {
			let value = match file.view(NodeId::new(index)) {
				View::Use { value, .. }
				| View::Reuse { value, .. }
				| View::Provide { value, .. } => value,
				_ => continue,
			};
			let mut current = value;
			loop {
				current = match file.view(current) {
					View::Group(inner) => inner,
					View::Access { target, .. } | View::Index { target, .. } => target,
					View::Call { callee, .. } => callee,
					_ => break,
				};
			}
			if let Resolution::Local(symbol) = analysis.resolution(current) {
				file.referenced.insert(symbol);
			}
		}
		file
	}

	/// The file named by its module, for the keys of its own places.
	pub(crate) fn with_module(mut self, module: &str) -> Self {
		self.module = module.to_string();
		self
	}

	pub(crate) fn view(&self, node: NodeId) -> View<'a> {
		syntax::view(self.tree, node)
	}

	/// The key of a claim's place in the facts: a field, the items or the
	/// values of a type by its stable name (the same in every unit); a
	/// local, a parameter or a result by its file and node. `None` for a
	/// type without a stable name.
	pub(crate) fn key(&self, claim: Claim) -> Option<String> {
		let types = self.types;
		Some(match claim {
			Claim::Field(owner, name) => format!(
				"field:{}.{}",
				stable_type_name(types, owner)?,
				types.syms.resolve(name)
			),
			Claim::Items(ty) => format!("items:{}", stable_type_name(types, ty)?),
			Claim::MapValues(ty) => format!("values:{}", stable_type_name(types, ty)?),
			Claim::Return(function) => format!("return:{}:{}", self.module, function.index()),
			Claim::Param(function, index) => {
				format!("param:{}:{}#{index}", self.module, function.index())
			}
			Claim::Local(symbol) => format!(
				"local:{}:{}",
				self.module,
				self.analysis.symbols.symbol(symbol).decl_node.index()
			),
		})
	}

	/// Whether a function is a module-level declaration of the file that it
	/// doesn't export: only the file calls it.
	fn private_function(&self, function: NodeId) -> bool {
		let parent = self
			.parents
			.get(function.index())
			.copied()
			.unwrap_or(usize::MAX);
		parent != usize::MAX && matches!(self.view(NodeId::new(parent)), View::Module { .. })
	}

	/// Whether a local is a module-level value the file exports (other
	/// files may write it).
	fn exported_value(&self, symbol: SymbolId) -> bool {
		let declaration = self.analysis.symbols.symbol(symbol).decl_node;
		let parent = self
			.parents
			.get(declaration.index())
			.copied()
			.unwrap_or(usize::MAX);
		parent != usize::MAX
			&& matches!(
				self.view(NodeId::new(parent)),
				View::ExportValue { .. } | View::ExportNamespace(_)
			)
	}

	pub(crate) fn types(&self) -> &'a TypeStore {
		self.types
	}

	fn referenced_locals(&self) -> &FxHashSet<SymbolId> {
		&self.referenced
	}

	/// The positions of the parameters a function type changes (`mutates`,
	/// `invalidates`).
	fn changed_parameters(&self, ty: TypeId) -> Vec<usize> {
		let TypeKind::Function(sig) = self.types.kind(ty) else {
			return Vec::new();
		};
		let (mutates, invalidates) = self.types.effects(self.types.sig(sig).effects);
		mutates
			.iter()
			.chain(invalidates)
			.filter_map(|path| match self.types.path_data(*path).0.root {
				PathRoot::Param(index) => Some(index as usize),
				PathRoot::SelfParam => Some(0),
				_ => None,
			})
			.collect()
	}

	/// The `function` node a parameter belongs to.
	pub(crate) fn function_of_param(&self, symbol: SymbolId) -> Option<NodeId> {
		let mut at = self.analysis.symbols.symbol(symbol).decl_node.index();
		loop {
			let parent = *self.parents.get(at)?;
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
pub(crate) struct Prover<'p> {
	file: &'p ProofFile<'p>,
	parents: &'p [usize],
	goal: Goal<'p>,
	failed: FxHashSet<Claim>,
	claims: FxHashSet<Claim>,
	/// A build per unit (`facts.rs`): the places other units may write
	/// fail here, and, when given, the claims failed in the whole program
	/// (the merged facts, by key) decide every claim.
	per_unit: bool,
	program_failed: Option<crate::facts::Consulted<'p>>,
}

impl<'p> Prover<'p> {
	/// A prover of a goal over a file, with no claim failed yet.
	pub(crate) fn new(file: &'p ProofFile<'p>, goal: Goal<'p>) -> Self {
		Prover {
			file,
			parents: &file.parents,
			goal,
			failed: FxHashSet::default(),
			claims: FxHashSet::default(),
			per_unit: false,
			program_failed: None,
		}
	}

	/// A prover of a unit's part of the program's claims: a parameter of a
	/// function other files may call and a value they may write are not
	/// seen whole.
	pub(crate) fn per_unit(mut self) -> Self {
		self.per_unit = true;
		self
	}

	/// A prover deciding every claim by the program's failed claims
	/// ([`unit_claims`], merged and resolved by [`resolve_claims`]).
	pub(crate) fn with_program(mut self, failed: crate::facts::Consulted<'p>) -> Self {
		self.per_unit = true;
		self.program_failed = Some(failed);
		self
	}
}

impl<'p> Prover<'p> {
	pub(crate) fn is_array(&self, node: NodeId) -> bool {
		let Goal::Index { arrays, .. } = self.goal else {
			return false;
		};
		matches!(self.file.view(node), View::Name(_))
			&& matches!(self.file.analysis.resolution(node),
				Resolution::Local(symbol) if arrays.contains(&symbol))
	}

	/// The greatest fixpoint of the claims the roots depend on: every claim
	/// holds until one of its stores is not proven.
	pub(crate) fn fixpoint(&mut self, roots: &[NodeId]) {
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
	pub(crate) fn proves(&mut self, node: NodeId) -> bool {
		let file = self.file;
		if let Some(interval) = file.analysis.interval(node)
			&& self.within(interval.lo, interval.hi)
		{
			return true;
		}
		if matches!(self.goal, Goal::NonNegative { .. })
			&& let Some(range) = file.types().as_int(file.analysis.type_of(node))
			&& self.within(range.min, range.max)
		{
			return true;
		}
		match file.view(node) {
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
			View::Name(_) => match file.analysis.resolution(node) {
				Resolution::Local(symbol) => {
					let data = file.analysis.symbols.symbol(symbol);
					match data.kind {
						SymbolKind::Let | SymbolKind::Mutable => self.claim(Claim::Local(symbol)),
						SymbolKind::Parameter { index } => match file.function_of_param(symbol) {
							Some(function) => self.claim(Claim::Param(function, index as usize)),
							None => false,
						},
						_ => false,
					}
				}
				_ => false,
			},
			View::Access { target, members } if members.len() == 1 => {
				let member = members.first().expect("a member");
				match self.field_name(member) {
					Some(name) => {
						let owner = file.analysis.type_of(target);
						self.claim(Claim::Field(owner, name))
					}
					None => false,
				}
			}
			View::Index { target, .. } => {
				let ty = file.analysis.type_of(target);
				matches!(
					file.types().kind(ty),
					TypeKind::Collection(CollKind::Array, ..)
				) && self.claim(Claim::Items(ty))
			}
			View::Call { callee, .. } => {
				let Some(info) = file.analysis.call(node) else {
					return false;
				};
				match info.callee {
					Callee::Function(function) => self.claim(Claim::Return(function)),
					Callee::Builtin(id) => {
						let View::Access { target, .. } = file.view(callee) else {
							return false;
						};
						let receiver = file.analysis.type_of(target);
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
				let one = file.analysis.interval(second).is_some_and(|interval| {
					interval.lo == interval.hi && interval.lo == ethnum::I256::ONE
				});
				let length = match file.view(first) {
					View::Access { target, members } if members.len() == 1 => {
						self.is_array(target)
							&& matches!(file.analysis.resolution(members.first().expect("a member")),
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
		if let Some(failed) = self.program_failed {
			let goal = self.goal_name();
			if failed.contains(JSON_PARSE) {
				return false;
			}
			return match self.file.key(claim) {
				Some(key) => {
					!failed.contains(&format!("{goal}|{key}"))
						&& !matches!(claim, Claim::Field(_, name)
							if failed.contains(&format!("{goal}|field:*.{}", self.file.types().syms.resolve(name)))
								|| failed.contains(&format!("{goal}|field:*.*")))
				}
				None => false,
			};
		}
		!self.failed.contains(&claim)
	}

	/// The name of the goal in the facts (never-negative only).
	fn goal_name(&self) -> &'static str {
		match self.goal {
			Goal::NonNegative { checked: true } => "n",
			Goal::NonNegative { checked: false } => "w",
			Goal::Index { .. } => "i",
		}
	}

	/// Whether every store of a claim's place is proven.
	fn holds(&mut self, claim: Claim) -> bool {
		let file = self.file;
		let count = file.tree.nodes.len();
		// per unit: other files call the functions they can name, and write
		// the values exported to them
		if self.per_unit {
			match claim {
				Claim::Param(function, _) if !file.private_function(function) => return false,
				Claim::Local(symbol) if file.exported_value(symbol) => return false,
				_ => {}
			}
		}
		match claim {
			Claim::Local(symbol) => {
				if file.referenced_locals().contains(&symbol) {
					return false;
				}
				let declaration = file.analysis.symbols.symbol(symbol).decl_node;
				let initial = match file.view(declaration) {
					View::Declaration {
						value: Some(value), ..
					} => self.proves(value),
					_ => false,
				};
				initial
					&& (0..count).all(|at| {
						let node = NodeId::new(at);
						match file.view(node) {
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
						match file.analysis.call(node) {
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
				let Some(info) = file.analysis.function(function) else {
					return false;
				};
				let body = info.body;
				let last = match file.view(body) {
					View::Block { statements } => statements.last(),
					_ => Some(body),
				};
				let last_ok = match last {
					Some(last) if !matches!(file.view(last), View::Return(_)) => self.proves(last),
					_ => true,
				};
				last_ok
					&& (0..count).all(|at| match file.view(NodeId::new(at)) {
						View::Return(value) if self.inside(NodeId::new(at), function) => {
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					})
			}
			Claim::Field(owner, name) => (0..count).all(|at| {
				let node = NodeId::new(at);
				if let Some(object) = file.analysis.object(node)
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
				match file.view(node) {
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
				match file.view(node) {
					View::Array { items } if file.analysis.type_of(node) == array_type => {
						items.iter().all(|item| self.proves(item))
					}
					View::Assignment { target, value, .. } => match file.view(target) {
						View::Index { target: array, .. }
							if file.analysis.type_of(array) == array_type =>
						{
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					},
					View::CompoundAssignment { op, target, value } => match file.view(target) {
						View::Index { target: array, .. }
							if file.analysis.type_of(array) == array_type =>
						{
							self.step(op, value)
						}
						_ => true,
					},
					View::Index { target: array, .. }
						if file.analysis.type_of(array) == array_type =>
					{
						!self.changed_elsewhere(node)
					}
					View::Call { callee, .. } => {
						let View::Access { target, .. } = file.view(callee) else {
							return true;
						};
						if file.analysis.type_of(target) != array_type {
							return true;
						}
						let Some(info) = file.analysis.call(node) else {
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
				match file.view(node) {
					View::Assignment { target, value, .. } => match file.view(target) {
						View::Index { target: map, .. }
							if file.analysis.type_of(map) == map_type =>
						{
							value.is_some_and(|value| self.proves(value))
						}
						_ => true,
					},
					View::CompoundAssignment { op, target, value } => match file.view(target) {
						View::Index { target: map, .. }
							if file.analysis.type_of(map) == map_type =>
						{
							self.step(op, value)
						}
						_ => true,
					},
					View::Index { target: map, .. } if file.analysis.type_of(map) == map_type => {
						!self.changed_elsewhere(node)
					}
					View::Use { .. } | View::Reuse { .. } | View::Provide { .. } => true,
					View::Call { callee, .. } => {
						let View::Access { target, .. } = file.view(callee) else {
							return true;
						};
						if file.analysis.type_of(target) != map_type {
							return true;
						}
						let Some(Callee::Builtin(id)) =
							file.analysis.call(node).map(|info| info.callee)
						else {
							return true;
						};
						matches!(
							fa_analyzer::builtins::get(id).path,
							"Map#get" | "Map#delete" | "Map#has" | "Map#size" | "Map#reserve"
						)
					}
					View::Array { .. } | View::Object { .. }
						if file.analysis.type_of(node) == map_type =>
					{
						matches!(file.view(node), View::Array { items } if items.is_empty())
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
		let file = self.file;
		let Some(symbol) = file
			.analysis
			.function(function)
			.and_then(|info| info.symbol)
		else {
			return false;
		};
		let name = file.analysis.symbols.symbol(symbol).name;
		(0..file.tree.nodes.len()).all(|at| {
			let node = NodeId::new(at);
			let named = match file.analysis.resolution(node) {
				Resolution::Local(used) => used == symbol && node != function,
				Resolution::Method(_, method) => method == name,
				_ => false,
			};
			if !named || file.analysis.symbols.symbol_of_decl(node) == Some(symbol) {
				return true;
			}
			// the callee: the name itself, or the access whose last member it is
			let mut callee = node;
			let parent = self.parents.get(at).copied().unwrap_or(usize::MAX);
			if parent != usize::MAX
				&& matches!(file.view(NodeId::new(parent)),
					View::Access { members, .. } if members.last() == Some(node))
			{
				callee = NodeId::new(parent);
			}
			let call = self
				.parents
				.get(callee.index())
				.copied()
				.unwrap_or(usize::MAX);
			call != usize::MAX
				&& matches!(file.view(NodeId::new(call)), View::Call { callee: called, .. } if called == callee)
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
						.file
						.analysis
						.interval(value)
						.map(|interval| (interval.lo, interval.hi))
						.or(match self.file.view(value) {
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
		let file = self.file;
		let mut at = place.index();
		// through groups
		let parent = loop {
			let parent = self.parents.get(at).copied().unwrap_or(usize::MAX);
			if parent == usize::MAX {
				return false;
			}
			if matches!(file.view(NodeId::new(parent)), View::Group(_)) {
				at = parent;
				continue;
			}
			break NodeId::new(parent);
		};
		let place = NodeId::new(at);
		match file.view(parent) {
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
					&& matches!(file.view(NodeId::new(grand)), View::Call { callee, .. } if callee == parent)
					&& self.changed_by_call(NodeId::new(grand), place)
			}
			_ => false,
		}
	}

	/// Whether a call may change the argument `place` through its parameter.
	fn changed_by_call(&self, call: NodeId, place: NodeId) -> bool {
		let file = self.file;
		let Some(info) = file.analysis.call(call) else {
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
			Callee::Function(function) => match file.analysis.function(function) {
				Some(function) => file.changed_parameters(function.ty).contains(&position),
				None => true,
			},
			Callee::Value => {
				let View::Call { callee, .. } = file.view(call) else {
					return true;
				};
				file.changed_parameters(file.analysis.type_of(callee))
					.contains(&position)
			}
			Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_) => false,
		}
	}

	/// The places a unit's file stores to or reads (`unit_claims`): the
	/// integer fields of its object literals and accesses, the items and
	/// values of the arrays and maps it indexes, pushes to or builds, and
	/// its integer locals.
	fn candidates(&self) -> Vec<Claim> {
		let file = self.file;
		let types = file.types();
		let integer = |node: NodeId| types.as_int(file.analysis.type_of(node)).is_some();
		let mut out = Vec::new();
		for at in 0..file.tree.nodes.len() {
			let node = NodeId::new(at);
			if let Some(object) = file.analysis.object(node) {
				for (name, value) in &object.fields {
					if let FieldValue::Given(value) | FieldValue::Default(value) = value
						&& integer(*value)
					{
						out.push(Claim::Field(object.ty, *name));
					}
				}
			}
			match file.view(node) {
				View::Access { target, members } if members.len() == 1 && integer(node) => {
					if let Some(name) = self.field_name(members.first().expect("a member")) {
						out.push(Claim::Field(file.analysis.type_of(target), name));
					}
				}
				View::Index { target, .. } if integer(node) => {
					let ty = file.analysis.type_of(target);
					match types.kind(ty) {
						TypeKind::Collection(CollKind::Array, ..) => out.push(Claim::Items(ty)),
						TypeKind::Collection(CollKind::Map, ..) => out.push(Claim::MapValues(ty)),
						_ => {}
					}
				}
				View::Array { .. } => {
					let ty = file.analysis.type_of(node);
					match types.kind(ty) {
						TypeKind::Collection(CollKind::Array, item, _)
							if types.as_int(item).is_some() =>
						{
							out.push(Claim::Items(ty))
						}
						TypeKind::Collection(CollKind::Map, item, _)
							if types.as_int(item).is_some() =>
						{
							out.push(Claim::MapValues(ty))
						}
						_ => {}
					}
				}
				View::Call { callee, .. } => {
					if let View::Access { target, .. } = file.view(callee) {
						let ty = file.analysis.type_of(target);
						match types.kind(ty) {
							TypeKind::Collection(CollKind::Array, item, _)
								if types.as_int(item).is_some() =>
							{
								out.push(Claim::Items(ty))
							}
							TypeKind::Collection(CollKind::Map, item, _)
								if types.as_int(item).is_some() =>
							{
								out.push(Claim::MapValues(ty))
							}
							_ => {}
						}
					}
				}
				View::Name(_) if integer(node) => {
					if let Resolution::Local(symbol) = file.analysis.resolution(node)
						&& matches!(
							file.analysis.symbols.symbol(symbol).kind,
							SymbolKind::Let | SymbolKind::Mutable
						) {
						out.push(Claim::Local(symbol));
					}
				}
				_ => {}
			}
		}
		out
	}

	/// The names of the fields a unit's file may change with an owner the
	/// proofs don't know (`a.b.f = …`, a reference to `a.b.f`, `x .= { … }`
	/// for every field: `*`).
	fn unknown_field_writes(&mut self) -> BTreeSet<String> {
		let file = self.file;
		let mut names = BTreeSet::new();
		for at in 0..file.tree.nodes.len() {
			let node = NodeId::new(at);
			let (target, write) = match file.view(node) {
				View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
					(target, true)
				}
				View::UpdateFields { .. } => {
					names.insert("*".to_string());
					continue;
				}
				View::Access { .. } => (node, false),
				_ => continue,
			};
			let View::Access { members, .. } = file.view(target) else {
				continue;
			};
			if members.len() < 2 {
				continue;
			}
			let Some(name) = members.last().and_then(|member| self.field_name(member)) else {
				continue;
			};
			if write || self.changed_elsewhere(node) {
				names.insert(file.types().syms.resolve(name).to_string());
			}
		}
		names
	}

	/// The name of the field a member node designates.
	fn field_name(&self, member: NodeId) -> Option<fa_analyzer::ids::Sym> {
		let Resolution::Field(shape, index) = self.file.analysis.resolution(member) else {
			return None;
		};
		self.file
			.types()
			.shape_fields(shape)
			.get(index as usize)
			.map(|field| field.name)
	}

	/// Whether a place is the name of a local.
	fn names(&self, target: NodeId, symbol: SymbolId) -> bool {
		matches!(self.file.view(target), View::Name(_))
			&& self.file.analysis.resolution(target) == Resolution::Local(symbol)
	}

	/// The type owning the field `name` an assignment target writes, if it
	/// writes a field of that name (`x.name`, `a[i].name`).
	fn field_target(&self, target: NodeId, name: fa_analyzer::ids::Sym) -> Option<TypeId> {
		let file = self.file;
		let View::Access {
			target: owner,
			members,
		} = file.view(target)
		else {
			return None;
		};
		let last = members.last()?;
		if self.field_name(last) != Some(name) {
			return None;
		}
		if members.len() == 1 {
			return Some(file.analysis.type_of(owner));
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
				self.file.view(NodeId::new(parent)),
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
		matches!(self.file.view(function), View::Function { value, .. } if value.index() == lambda)
	}
}

/// A unit's part of the program's claims for the never-negative places, in
/// a build per unit (`facts::PROOF_CLAIMS`): for each goal (`n` with
/// overflow checks, `w` without) and each place its files store to or
/// read, `goal|key|dependencies` (the keys of the places its stores read,
/// `;`-separated: the place holds when they all do) or `goal|key|!` (a
/// store of the unit is not proven). A write to a field of an unknown owner
/// is `goal|field:*.name|!`. [`resolve_claims`] merges them.
pub(crate) fn unit_claims(files: &[ProofFile<'_>]) -> BTreeSet<String> {
	let mut out = BTreeSet::new();
	for file in files {
		let parses = (0..file.tree.nodes.len()).any(|at| {
			matches!(file.analysis.call(NodeId::new(at)).map(|info| info.callee),
				Some(Callee::Builtin(id)) if fa_analyzer::builtins::get(id).path == "Json.parse")
		});
		if parses {
			out.insert(JSON_PARSE.to_string());
		}
		for checked in [true, false] {
			let mut prover = Prover::new(file, Goal::NonNegative { checked }).per_unit();
			let goal = prover.goal_name();
			let mut pending: Vec<Claim> = prover.candidates();
			let mut seen: FxHashSet<Claim> = FxHashSet::default();
			while let Some(claim) = pending.pop() {
				if !seen.insert(claim) {
					continue;
				}
				let Some(key) = file.key(claim) else {
					continue;
				};
				prover.claims.clear();
				let holds = prover.holds(claim);
				let mut dependencies: Vec<String> = Vec::new();
				let mut unnamed = false;
				for &dependency in &prover.claims {
					match file.key(dependency) {
						Some(key) => dependencies.push(key),
						None => unnamed = true,
					}
					pending.push(dependency);
				}
				dependencies.sort();
				dependencies.dedup();
				let edge = if holds && !unnamed {
					dependencies.join(";")
				} else {
					"!".to_string()
				};
				out.insert(format!("{goal}|{key}|{edge}"));
			}
			// writes to a field of an unknown owner
			for name in prover.unknown_field_writes() {
				out.insert(format!("{goal}|field:*.{name}|!"));
			}
		}
	}
	out
}

/// The failed claims of the program from the merged [`unit_claims`]: a
/// place fails when a unit couldn't prove one of its stores, or when a place
/// it depends on fails (the greatest fixpoint: every other place holds).
/// The result holds `goal|key` for each failed place, and [`JSON_PARSE`].
pub(crate) fn resolve_claims(edges: &BTreeSet<String>) -> BTreeSet<String> {
	let mut failed: BTreeSet<String> = BTreeSet::new();
	// the places depending on each place, by goal
	let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
	let mut queue: Vec<String> = Vec::new();
	for edge in edges {
		if edge == JSON_PARSE {
			failed.insert(JSON_PARSE.to_string());
			continue;
		}
		let mut parts = edge.splitn(3, '|');
		let (Some(goal), Some(key), Some(rest)) = (parts.next(), parts.next(), parts.next()) else {
			continue;
		};
		let place = format!("{goal}|{key}");
		if rest == "!" {
			queue.push(place);
			continue;
		}
		for dependency in rest.split(';').filter(|dependency| !dependency.is_empty()) {
			dependents
				.entry(format!("{goal}|{dependency}"))
				.or_default()
				.push(place.clone());
		}
	}
	while let Some(place) = queue.pop() {
		if !failed.insert(place.clone()) {
			continue;
		}
		if let Some(places) = dependents.get(&place) {
			queue.extend(places.iter().cloned());
		}
	}
	// a field written with an unknown owner fails under every owner: its
	// dependents are found by name
	let stars: Vec<(String, String)> = failed
		.iter()
		.filter_map(|place| {
			let (goal, key) = place.split_once('|')?;
			let name = key.strip_prefix("field:*.")?;
			Some((goal.to_string(), format!(".{name}")))
		})
		.collect();
	if !stars.is_empty() {
		let mut queue: Vec<String> = dependents
			.keys()
			.filter(|place| {
				stars.iter().any(|(goal, suffix)| {
					place.starts_with(&format!("{goal}|field:"))
						&& (suffix == ".*" || place.ends_with(suffix.as_str()))
				})
			})
			.cloned()
			.collect();
		while let Some(place) = queue.pop() {
			if !failed.insert(place.clone()) {
				continue;
			}
			if let Some(places) = dependents.get(&place) {
				queue.extend(places.iter().cloned());
			}
		}
	}
	failed
}

/// [`resolve_claims`] for `FactKind::finish`.
pub(crate) fn resolve_claims_owned(edges: BTreeSet<String>) -> BTreeSet<String> {
	resolve_claims(&edges)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use super::{JSON_PARSE, resolve_claims};

	fn set(items: &[&str]) -> BTreeSet<String> {
		items.iter().map(|item| item.to_string()).collect()
	}

	#[test]
	fn a_failed_place_fails_the_places_that_read_it() {
		let edges = set(&[
			"n|field:T.a|",
			"n|field:T.b|field:T.a",
			"n|field:T.c|field:T.b;local:m:3",
			"n|local:m:3|",
			"n|field:T.a|items:A",
			"n|items:A|!",
			"w|field:T.a|",
		]);
		assert_eq!(
			resolve_claims(&edges),
			set(&["n|field:T.a", "n|field:T.b", "n|field:T.c", "n|items:A"])
		);
	}

	#[test]
	fn a_cycle_of_places_holds_unless_one_fails() {
		let edges = set(&["n|local:m:1|local:m:2", "n|local:m:2|local:m:1"]);
		assert!(resolve_claims(&edges).is_empty());
		let mut failing = edges.clone();
		failing.insert("n|local:m:2|!".to_string());
		assert_eq!(
			resolve_claims(&failing),
			set(&["n|local:m:1", "n|local:m:2"])
		);
	}

	#[test]
	fn a_field_written_with_an_unknown_owner_fails_under_every_owner() {
		let edges = set(&[
			"n|field:T.at|",
			"n|field:U.at|",
			"n|field:U.to|",
			"n|local:m:1|field:T.at",
			"n|field:*.at|!",
			JSON_PARSE,
		]);
		let failed = resolve_claims(&edges);
		// the places reading such a field fail; the consumer checks the
		// field's own `field:*.at` (`Prover::claim`)
		assert!(failed.contains("n|field:T.at") && failed.contains("n|field:*.at"));
		assert!(failed.contains("n|local:m:1") && failed.contains(JSON_PARSE));
		assert!(!failed.contains("n|field:U.to"));
	}
}
