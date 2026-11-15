//! The proofs over the stores of a program's places, the facts both
//! backends read to drop index checks ([`index_facts`]: the index proofs of
//! fields and the never-negative places) and to read a field once
//! ([`written_fields`], natively). Builds per unit publish each unit's part
//! of the claims ([`unit_claims`], merged by [`resolve_claims`]) through the
//! whole-program facts of `fa-codegen-llvm` (`facts.rs`).
//! A [`ProofFile`] is one analyzed file; a [`Prover`] establishes a
//! [`Goal`] for the values of its [`Claim`] places, a greatest fixpoint.

use std::collections::{BTreeMap, BTreeSet};

use fa_parser::TypedSyntaxTree;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{Analysis, Argument, Callee, FieldValue, ForeignArg, NodeFlags};
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::stable_names::stable_type_name;
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, ArithOp, View};
use crate::types::{CollKind, PathRoot, TypeFlags, TypeKind, TypeStore};

/// The fact of a unit whose values may come from outside the program
/// ([`ProofFile::opaque`]: JSON parsing, reading a C pointer, C code
/// writing through a pointer, or bodies checked under several key-loop
/// analyses): the objects, arrays and maps it builds or
/// gives away hold any integers, so no claim of a field, items or values
/// holds (`unit_claims`).
pub const OPAQUE: &str = "opaque";

/// The separator of the parts of a claim's line in the facts (`goal`,
/// place, dependencies): type spellings may hold `|` (unions).
pub const SEPARATOR: char = '\u{1f}';

/// The separator of the dependencies of a claim's line.
const DEPENDENCIES: char = '\u{1e}';

/// A place whose every stored value must be an index of the array.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Claim {
	/// The field `name` of the objects of a type.
	Field(TypeId, crate::ids::Sym),
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
pub struct ProofFile<'a> {
	pub(crate) tree: &'a TypedSyntaxTree,
	pub(crate) analysis: &'a Analysis,
	types: &'a TypeStore,
	/// The parent of every node (`usize::MAX` for the root).
	pub(crate) parents: Vec<usize>,
	/// The locals that a reference designates a part of.
	referenced: FxHashSet<SymbolId>,
	/// The file's module, naming its own places in the facts.
	module: String,
	/// Whether values may come from outside the program (see [`OPAQUE`]).
	opaque: bool,
}

impl<'a> ProofFile<'a> {
	pub fn new(tree: &'a TypedSyntaxTree, analysis: &'a Analysis, types: &'a TypeStore) -> Self {
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
			opaque: false,
		};
		file.opaque = file.compute_opaque();
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
	pub fn with_module(mut self, module: &str) -> Self {
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

	/// Whether values may come from outside the program (see [`OPAQUE`]).
	pub(crate) fn opaque(&self) -> bool {
		self.opaque
	}

	fn compute_opaque(&self) -> bool {
		// A key loop has a separate analysis for each key, with different
		// types and stores at the same node indices. Its main analysis is
		// not a summary of all those stores. Until it is, a proof may not
		// choose one key's stores as the invariant of the whole program.
		if !self.analysis.key_loop_bodies.is_empty() {
			return true;
		}
		let builds = (0..self.tree.nodes.len()).any(|at| {
			matches!(self.analysis.call(NodeId::new(at)).map(|info| info.callee),
				Some(Callee::Builtin(id))
					if matches!(crate::builtins::get(id).path, "Json.parse" | "Pointer#read"))
		});
		// C code writing a value that isn't a lone integer (a local's own
		// claim sees the call: `changed_elsewhere`)
		let written = self.analysis.foreign_args.iter().any(|(&node, argument)| {
			matches!(
				argument,
				ForeignArg::Address { mutated: true } | ForeignArg::Array { mutated: true }
			) && self.types.as_int(self.analysis.type_of(node)).is_none()
		});
		builds || written
	}

	/// The object type of a type (through a recursive reference), if it is
	/// one without type parameters.
	pub(crate) fn object_type(&self, ty: TypeId) -> Option<TypeId> {
		let ty = self.types.unfold(ty);
		(matches!(self.types.kind(ty), TypeKind::Object(_))
			&& !self
				.types
				.flags(ty)
				.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN))
		.then_some(ty)
	}

	/// Whether the objects of `source` may become objects of `owner`, so
	/// that their field values become the owner's: the same type, a type
	/// with every field of the owner (a wider object copied or viewed as
	/// one, an instance of a generic type), or a type that isn't a known
	/// object (a type parameter, an unknown owner).
	fn may_become(&self, source: TypeId, owner: TypeId) -> bool {
		let Some(source) = self.object_type(source) else {
			return true;
		};
		let Some(owner) = self.object_type(owner) else {
			return true;
		};
		source == owner || {
			let names: FxHashSet<crate::ids::Sym> = self
				.types
				.fields_of(source)
				.iter()
				.map(|field| field.name)
				.collect();
			self.types
				.fields_of(owner)
				.iter()
				.all(|field| names.contains(&field.name))
		}
	}

	/// Whether a collection type of a store may be the claimed one: the same
	/// type, or a type with parameters (generic code storing into any).
	fn may_be(&self, ty: TypeId, claimed: TypeId) -> bool {
		ty == claimed
			|| self
				.types
				.flags(ty)
				.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN)
	}

	/// Whether the values of a type may hold a value of the claimed type
	/// (itself, a field, an item, a member, through optionals and unions).
	fn contains(&self, ty: TypeId, claimed: TypeId) -> bool {
		let types = self.types;
		let mut seen = FxHashSet::default();
		let mut stack = vec![ty];
		while let Some(at) = stack.pop() {
			if at == claimed {
				return true;
			}
			if !seen.insert(at) {
				continue;
			}
			match types.kind(at) {
				TypeKind::Object(_) => {
					stack.extend(types.fields_of(at).iter().map(|field| field.ty))
				}
				TypeKind::Rec(_) => stack.extend(types.rec_target(at)),
				TypeKind::Union(list) | TypeKind::Tuple(list) => {
					stack.extend(types.list_items(list).iter().copied())
				}
				TypeKind::Member(nominal, index) => stack.push(types.payload(nominal, index)),
				TypeKind::NamedUnion(nominal, _) => {
					for index in 0..types.sym_list(types.nominal(nominal).members).len() {
						stack.push(types.payload(nominal, index as u16));
					}
				}
				TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
					stack.push(item);
					stack.push(key);
				}
				TypeKind::Pointer(item)
				| TypeKind::Sync(_, item)
				| TypeKind::Ref(item, _)
				| TypeKind::CArray(item, _) => stack.push(item),
				TypeKind::Param(_) | TypeKind::ImplicitParam(..) => return true,
				_ => {}
			}
		}
		false
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
	program_failed: Option<&'p dyn Fn(&str) -> bool>,
	/// Built without overflow checks: an overflow wraps around, so the
	/// intervals of the analysis (which assume a checked overflow stops the
	/// program) don't hold for the values of narrow integer types, which
	/// wrap after a few steps.
	wraps: bool,
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
			wraps: matches!(goal, Goal::NonNegative { checked: false }),
		}
	}

	/// A prover of a program built without overflow checks (see
	/// [`Prover::wraps`]).
	pub(crate) fn wrapping(mut self, wraps: bool) -> Self {
		self.wraps |= wraps;
		self
	}

	/// The interval of an integer node the proof may rely on: none for a
	/// type narrower than 64 bits when overflows wrap.
	fn interval(&self, node: NodeId) -> Option<crate::types::Interval> {
		let file = self.file;
		if self.wraps
			&& file
				.types()
				.as_int(file.analysis.type_of(node))
				.is_some_and(|range| range.max < ethnum::I256::from(i64::MAX))
		{
			return None;
		}
		file.analysis.interval(node)
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
	pub(crate) fn with_program(mut self, failed: &'p dyn Fn(&str) -> bool) -> Self {
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
		// until no claim fails and no new claim is found (the stores of a
		// claim checked last may depend on claims not checked yet)
		loop {
			let before = (self.failed.len(), self.claims.len());
			for &root in roots {
				self.proves(root);
			}
			let claims: Vec<Claim> = self.claims.iter().copied().collect();
			for claim in claims {
				if !self.failed.contains(&claim) && !self.holds(claim) {
					self.failed.insert(claim);
				}
			}
			if (self.failed.len(), self.claims.len()) == before {
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
		if let Some(interval) = self.interval(node)
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
			// `value catch error => fallback`: either one (a handler block may
			// end in a statement)
			View::Catch {
				expression,
				handler,
				..
			} => {
				!matches!(file.view(handler), View::Block { .. })
					&& self.proves(expression)
					&& self.proves(handler)
			}
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
						match crate::builtins::get(id).path {
							"Map#get" => self.claim(Claim::MapValues(receiver)),
							"Array#pop" | "Array#get" => self.claim(Claim::Items(receiver)),
							_ => false,
						}
					}
					// a conversion that may wrap (no overflow check) keeps a
					// non-negative value non-negative only into an unsigned
					// type or one of 64 bits or more
					Callee::Conversion(target) => {
						let keeps = matches!(self.goal, Goal::NonNegative { checked: true })
							|| file.types().as_int(target).is_some_and(|range| {
								range.min >= ethnum::I256::ZERO
									|| range.max >= ethnum::I256::from(i64::MAX)
							});
						keeps
							&& match info.arguments.first() {
								Some(Argument::Given(value)) => self.proves(*value),
								_ => false,
							}
					}
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
				}) || matches!(file.view(second), View::Integer(value) if value == ethnum::I256::ONE);
				let length = match file.view(first) {
					View::Access { target, members } if members.len() == 1 => {
						self.is_array(target)
							&& matches!(file.analysis.resolution(members.first().expect("a member")),
								Resolution::Builtin(id) if crate::builtins::get(id).path == "Array#length")
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
		// the places of a type parameter or of an unknown type stand for
		// places of other types, whose stores the claim wouldn't see
		let generic = |ty: TypeId| {
			self.file
				.types()
				.flags(ty)
				.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN)
		};
		match claim {
			Claim::Field(owner, _) if self.file.object_type(owner).is_none() => return false,
			Claim::Items(ty) | Claim::MapValues(ty) if generic(ty) => return false,
			_ => {}
		}
		if let Some(failed) = self.program_failed {
			let goal = self.goal_name();
			if failed(OPAQUE) {
				return false;
			}
			// An index belongs to this file's private array. Its own stores
			// still need the local fixpoint; another file supplying values
			// of the same place prevents that proof altogether.
			if matches!(self.goal, Goal::Index { .. }) {
				return self.file.key(claim).is_some_and(|key| {
					!failed(&format!("i{SEPARATOR}{key}")) && !self.failed.contains(&claim)
				});
			}
			let failed_key = |key: &str| failed(&format!("{goal}{SEPARATOR}{key}"));
			return match self.file.key(claim) {
				Some(key) => {
					!failed_key(&key)
						&& match claim {
							Claim::Field(_, name) => {
								!failed_key(&format!(
									"field:*.{}",
									self.file.types().syms.resolve(name)
								)) && !failed_key("field:*.*")
							}
							Claim::Items(_) => !failed_key("items:*"),
							Claim::MapValues(_) => !failed_key("values:*"),
							_ => true,
						}
				}
				None => false,
			};
		}
		!self.failed.contains(&claim)
	}

	/// The name of the goal in the facts.
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
		// the items or values of a generic collection type stand for those
		// of other types
		if let Claim::Items(ty) | Claim::MapValues(ty) = claim
			&& file
				.types()
				.flags(ty)
				.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN)
		{
			return false;
		}
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
				let ty = file.analysis.symbols.symbol(symbol).ty;
				initial
					&& (0..count).all(|at| {
						let node = NodeId::new(at);
						match file.view(node) {
							View::Assignment { target, .. }
								if self
									.destructured(target, &|place| self.names(place, symbol)) =>
							{
								false
							}
							View::Assignment { target, value, .. }
								if self.names(target, symbol) =>
							{
								value.is_some_and(|value| self.proves(value))
							}
							View::CompoundAssignment { op, target, value }
								if self.names(target, symbol) =>
							{
								self.step(op, value, ty)
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
				let Some(symbol) = file
					.analysis
					.function(function)
					.and_then(|info| info.params.get(index))
					.map(|param| param.symbol)
				else {
					return false;
				};
				self.called_directly(function)
					&& (0..count).all(|at| {
						let node = NodeId::new(at);
						// a store to the parameter itself (its declaration, `from: UInt32`,
						// is an assignment without value, a named argument one in a call)
						if let View::Assignment {
							target,
							value: Some(_),
							..
						}
						| View::CompoundAssignment { target, .. } = file.view(node)
							&& !matches!(
								self.parents.get(at).map(|&parent| parent != usize::MAX
									&& matches!(file.view(NodeId::new(parent)), View::Call { .. })),
								Some(true)
							) && (self.names(target, symbol)
							|| self.destructured(target, &|place| self.names(place, symbol)))
						{
							return false;
						}
						if matches!(file.view(node), View::Name(_))
							&& self.names(node, symbol)
							&& self.changed_elsewhere(node)
						{
							return false;
						}
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
			Claim::Field(owner, name) => {
				// a field of a known object type; the objects built out of
				// the program's sight (JSON, C) hold anything
				if file.object_type(owner).is_none() || file.opaque() {
					return false;
				}
				(0..count).all(|at| {
					let node = NodeId::new(at);
					// every literal whose objects may become the owner's
					if let Some(object) = file.analysis.object(node)
						&& file.may_become(object.ty, owner)
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
					let matches = |prover: &Self, target: NodeId| {
						prover
							.field_target(target, name)
							.is_some_and(|target_owner| file.may_become(target_owner, owner))
					};
					match file.view(node) {
						View::Assignment { target, value, .. } => {
							if self.destructured(target, &|place| matches(self, place)) {
								return false;
							}
							!matches(self, target) || value.is_some_and(|value| self.proves(value))
						}
						View::CompoundAssignment { op, target, value } => {
							!matches(self, target) || self.step(op, value, self.place_type(target))
						}
						View::UpdateFields { .. } => false,
						// a reference to the field, or the field given to a
						// parameter that the callee changes
						View::Access { .. } => {
							!matches(self, node) || !self.changed_elsewhere(node)
						}
						_ => true,
					}
				})
			}
			Claim::Items(array_type) => {
				if file.opaque() {
					return false;
				}
				(0..count).all(|at| {
					let node = NodeId::new(at);
					if !self.produced_soundly(node, array_type) {
						return false;
					}
					let stored = |target: NodeId| {
						matches!(file.view(target), View::Index { target: array, .. }
							if file.may_be(file.analysis.type_of(array), array_type))
					};
					match file.view(node) {
						View::Array { items }
							if file.may_be(file.analysis.type_of(node), array_type) =>
						{
							items.iter().all(|item| self.proves(item))
						}
						View::Assignment { target, value, .. } => {
							if self.destructured(target, &stored) {
								return false;
							}
							!stored(target) || value.is_some_and(|value| self.proves(value))
						}
						View::CompoundAssignment { op, target, value } => {
							!stored(target) || self.step(op, value, self.place_type(target))
						}
						View::Index { .. } if stored(node) => !self.changed_elsewhere(node),
						View::Call { callee, .. } => self.collection_call(node, callee, array_type),
						_ => true,
					}
				})
			}
			Claim::MapValues(map_type) => {
				if file.opaque() {
					return false;
				}
				(0..count).all(|at| {
					let node = NodeId::new(at);
					if !self.produced_soundly(node, map_type) {
						return false;
					}
					let stored = |target: NodeId| {
						matches!(file.view(target), View::Index { target: map, .. }
							if file.may_be(file.analysis.type_of(map), map_type))
					};
					match file.view(node) {
						View::Assignment { target, value, .. } => {
							if self.destructured(target, &stored) {
								return false;
							}
							!stored(target) || value.is_some_and(|value| self.proves(value))
						}
						View::CompoundAssignment { op, target, value } => {
							!stored(target) || self.step(op, value, self.place_type(target))
						}
						View::Index { .. } if stored(node) => !self.changed_elsewhere(node),
						View::Call { callee, .. } => self.collection_call(node, callee, map_type),
						// a literal: the second item of each `(key, value)`
						View::Array { items } | View::Object { items }
							if file.may_be(file.analysis.type_of(node), map_type) =>
						{
							items.iter().all(|item| match file.view(item) {
								View::Tuple { items } if items.len() == 2 => {
									self.proves(items.get(1).expect("two items"))
								}
								_ => false,
							})
						}
						_ => true,
					}
				})
			}
		}
	}

	/// Whether a value of a type holding the claimed collection type is
	/// built only from values the claim sees: a node of such a type that a
	/// call produces is the result of a function of the program (its
	/// returned values are seen), a built-in selecting among the values of
	/// a receiver already holding the type, or a member built with its
	/// data; and no value of another type is converted into it. A built-in
	/// building a new collection (`map`, `Map.from`, `split`…), a generic
	/// function (its literals are of another type) or a function value
	/// fails the claim.
	fn produced_soundly(&mut self, node: NodeId, claimed: TypeId) -> bool {
		let file = self.file;
		let ty = file.analysis.type_of(node);
		if let Some(target) = file.analysis.coercion(node)
			&& target != ty
			&& file.contains(target, claimed)
			&& !file.contains(ty, claimed)
		{
			return false;
		}
		if !matches!(file.view(node), View::Call { .. })
			|| file.analysis.has_flags(node, NodeFlags::TYPE_EXPR)
			|| !file.contains(ty, claimed)
		{
			return true;
		}
		let Some(info) = file.analysis.call(node) else {
			return false;
		};
		match info.callee {
			Callee::Function(function) => {
				!file.analysis.extern_calls.contains_key(&node)
					&& file
						.analysis
						.function(function)
						.is_some_and(|function| function.generics.is_empty())
			}
			Callee::Member(_) => true,
			Callee::Builtin(id) => {
				let View::Call { callee, .. } = file.view(node) else {
					return false;
				};
				let View::Access { target, .. } = file.view(callee) else {
					return false;
				};
				let path = crate::builtins::get(id).path;
				// a collection of the claimed type built from a literal (`Map<K,
				// V>.from([], capacity = 4)`): its items, or the values of its
				// `(key, value)` items, are the literal's
				if file.analysis.has_flags(target, NodeFlags::TYPE_EXPR)
					&& ty == claimed
					&& path.ends_with(".from")
				{
					let map = matches!(
						file.types().kind(claimed),
						TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
					);
					let source = match info.arguments.first() {
						Some(Argument::Given(source)) => *source,
						_ => return false,
					};
					let View::Array { items } = file.view(source) else {
						return false;
					};
					let items: Vec<NodeId> = items.iter().collect();
					return items.into_iter().all(|item| match file.view(item) {
						View::Tuple { items } if map && items.len() == 2 => {
							self.proves(items.get(1).expect("two items"))
						}
						_ if !map => self.proves(item),
						_ => false,
					});
				}
				!file.analysis.has_flags(target, NodeFlags::TYPE_EXPR)
					&& file.contains(file.analysis.type_of(target), claimed)
					&& matches!(
						crate::builtins::get(id).path,
						"Array#get"
							| "Array#pop" | "Array#find"
							| "Array#take" | "Array#filter"
							| "Array#sort" | "Array#reverse"
							| "Map#get" | "Map#take"
							| "Map#find" | "Map#filter"
							| "Map#getOrInsert"
					)
			}
			Callee::Conversion(_) | Callee::Value => false,
		}
	}

	/// Whether a call of a built-in method on a collection keeps the claim
	/// of its items (or values): a receiver that may be of the claimed type
	/// (the type, or a type parameter) only read, or changed by removing or
	/// moving its values, or given proven values (`push`, `insert`).
	fn collection_call(&mut self, node: NodeId, callee: NodeId, claimed: TypeId) -> bool {
		let file = self.file;
		let View::Access { target, .. } = file.view(callee) else {
			return true;
		};
		if file.analysis.has_flags(target, NodeFlags::TYPE_EXPR)
			|| !file.may_be(file.analysis.type_of(target), claimed)
		{
			return true;
		}
		let Some(info) = file.analysis.call(node) else {
			return false;
		};
		let Callee::Builtin(id) = info.callee else {
			return true;
		};
		let builtin = crate::builtins::get(id);
		if crate::builtins::effects::effects_of(builtin.path).is_empty() {
			return true;
		}
		match builtin.path {
			// every value given (`insert(at, value)`: the position too, a
			// conservative check)
			"Array#push" | "Array#insert" => info.arguments.iter().all(|argument| match argument {
				Argument::Given(value) => self.proves(*value),
				Argument::Variadic(values) => values.iter().all(|value| self.proves(*value)),
				Argument::Receiver(_) => true,
				_ => false,
			}),
			"Array#pop"
			| "Array#take"
			| "Array#delete"
			| "Array#deleteWhere"
			| "Array#clear"
			| "Array#reserve"
			| "Array#sortInPlace"
			| "Array#reverseInPlace"
			| "Map#delete"
			| "Map#deleteWhere"
			| "Map#clear"
			| "Map#reserve"
			| "Map#take" => true,
			_ => false,
		}
	}

	/// Whether an assignment's target is a destructuring (`(a, b) = …`) one
	/// of whose places the claim is about.
	fn destructured(&self, target: NodeId, place: &dyn Fn(NodeId) -> bool) -> bool {
		let file = self.file;
		let mut top = target;
		while let View::Group(inner) = file.view(top) {
			top = inner;
		}
		if !matches!(
			file.view(top),
			View::Tuple { .. }
				| View::Array { .. }
				| View::TuplePattern { .. }
				| View::ArrayPattern { .. }
				| View::ObjectPattern { .. }
		) {
			return false;
		}
		let mut stack = vec![top.index()];
		while let Some(at) = stack.pop() {
			if at != top.index() && place(NodeId::new(at)) {
				return true;
			}
			file.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		false
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
	/// overflow checks only a step of at most one in a place of 64 bits or
	/// more (see [`Goal::NonNegative`]): a narrower place wraps to a negative
	/// value after a few steps (`Int8`: 128).
	fn step(&mut self, op: ArithOp, value: NodeId, place: TypeId) -> bool {
		match self.goal {
			Goal::Index { .. } => false,
			Goal::NonNegative { checked: true } => {
				matches!(op, ArithOp::Add | ArithOp::Multiply) && self.proves(value)
			}
			Goal::NonNegative { checked: false } => {
				op == ArithOp::Add
					&& self
						.file
						.types()
						.as_int(place)
						.is_some_and(|range| range.max >= ethnum::I256::from(i64::MAX))
					&& self
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
	/// values of the collections it indexes, calls methods of, stores into
	/// or builds, and its integer locals; a value of a type parameter
	/// counts as an integer (generic code may store integers).
	fn candidates(&self) -> Vec<Claim> {
		let file = self.file;
		let types = file.types();
		let integer = |ty: TypeId| {
			types.as_int(ty).is_some()
				|| types
					.flags(ty)
					.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN)
		};
		let collection = |ty: TypeId, out: &mut Vec<Claim>| {
			if let TypeKind::Collection(kind, item, _) = types.kind(ty)
				&& integer(item)
			{
				out.push(match kind {
					CollKind::Map | CollKind::GrowingMap => Claim::MapValues(ty),
					_ => Claim::Items(ty),
				});
			}
		};
		let mut out = Vec::new();
		for at in 0..file.tree.nodes.len() {
			let node = NodeId::new(at);
			if let Some(object) = file.analysis.object(node) {
				for (name, value) in &object.fields {
					if let FieldValue::Given(value) | FieldValue::Default(value) = value
						&& integer(file.analysis.type_of(*value))
					{
						out.push(Claim::Field(object.ty, *name));
					}
				}
			}
			match file.view(node) {
				View::Access { target, members }
					if members.len() == 1 && integer(file.analysis.type_of(node)) =>
				{
					if let Some(name) = self.field_name(members.first().expect("a member")) {
						out.push(Claim::Field(file.analysis.type_of(target), name));
					}
				}
				View::Index { target, .. } => collection(file.analysis.type_of(target), &mut out),
				View::Array { .. } | View::Object { .. } => {
					collection(file.analysis.type_of(node), &mut out)
				}
				View::Call { callee, .. } => {
					if let View::Access { target, .. } = file.view(callee) {
						collection(file.analysis.type_of(target), &mut out);
					}
					collection(file.analysis.type_of(node), &mut out);
				}
				View::Name(_) if integer(file.analysis.type_of(node)) => {
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

	/// The key of a claim in a unit's part of the facts: a field of an
	/// owner that isn't a known object type is `field:*.name` (it may be
	/// any owner's), the items or values of a generic collection type
	/// `items:*` or `values:*`.
	fn unit_key(&self, claim: Claim) -> Option<String> {
		let file = self.file;
		let types = file.types();
		let generic = |ty: TypeId| {
			types
				.flags(ty)
				.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN)
		};
		match claim {
			Claim::Field(owner, name) if file.object_type(owner).is_none() => {
				Some(format!("field:*.{}", types.syms.resolve(name)))
			}
			Claim::Items(ty) if generic(ty) => Some("items:*".to_string()),
			Claim::MapValues(ty) if generic(ty) => Some("values:*".to_string()),
			_ => file.key(claim),
		}
	}

	/// The names of the fields a unit's file may change with an owner the
	/// proofs don't know (`a.b.f = …`, a reference to `a.b.f`, a field of a
	/// value of a type parameter, `x .= { … }` or a destructuring store of
	/// a field for every field: `*`).
	fn unknown_field_writes(&mut self) -> BTreeSet<String> {
		let file = self.file;
		let mut names = BTreeSet::new();
		for at in 0..file.tree.nodes.len() {
			let node = NodeId::new(at);
			let (target, write) = match file.view(node) {
				View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
					if self.destructured(target, &|place| {
						matches!(file.view(place), View::Access { members, .. }
							if members.last().is_some_and(|member| self.field_name(member).is_some()))
					}) {
						names.insert("*".to_string());
					}
					(target, true)
				}
				View::UpdateFields { .. } => {
					names.insert("*".to_string());
					continue;
				}
				View::Access { .. } => (node, false),
				_ => continue,
			};
			let View::Access {
				target: owner,
				members,
			} = file.view(target)
			else {
				continue;
			};
			if members.len() < 2 && file.object_type(file.analysis.type_of(owner)).is_some() {
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
	fn field_name(&self, member: NodeId) -> Option<crate::ids::Sym> {
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
	fn field_target(&self, target: NodeId, name: crate::ids::Sym) -> Option<TypeId> {
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

/// The writes of a unit's files that may change a field after the literal
/// that built its object (`facts::WRITTEN_FIELDS`, `design/decisions.md`,
/// "Fields never written after construction"): `field:T.f` for a store to
/// the field `f` of a `T` (an assignment, a compound assignment, a reference
/// to it that a callee or a `mutable use` may change, a built-in method
/// changing it), `field:*.f` when the owner is not known (a deeper chain),
/// and `whole:T` for a store of a whole value that may hold a `T` (an
/// assignment of any type containing it, `.=`, a built-in method moving or
/// replacing the items of a collection of them); `whole:*` when such a type
/// has no stable name or is a type parameter. A field is never written after
/// construction when none of `field:T.f`, `field:*.f`, `whole:T` and
/// `whole:*` is in the program's union of these sets ([`frozen_field`]).
pub fn written_fields(files: &[ProofFile<'_>]) -> BTreeSet<String> {
	let mut out = BTreeSet::new();
	for file in files {
		let prover = Prover::new(file, Goal::NonNegative { checked: true });
		prover.written_fields(&mut out);
	}
	out
}

/// Whether a field is never written after construction, by the program's
/// [`written_fields`].
pub fn frozen_field(
	written: &BTreeSet<String>,
	types: &TypeStore,
	owner: TypeId,
	name: crate::ids::Sym,
) -> bool {
	let owner = types.unfold(owner);
	let Some(key) = stable_type_name(types, owner) else {
		return false;
	};
	let field = types.syms.resolve(name);
	let written_as = |key: &str| {
		written.contains(&format!("whole:{key}"))
			|| written.contains(&format!("field:{key}.{field}"))
	};
	if written.contains("whole:*")
		|| written.contains(&format!("field:*.{field}"))
		|| written_as(&key)
	{
		return false;
	}
	// an object of a wider type viewed as an owner (`self` a view of it)
	let names: BTreeSet<&str> = types
		.fields_of(owner)
		.iter()
		.map(|field| types.syms.resolve(field.name))
		.collect();
	!written.iter().any(|line| {
		let Some(rest) = line.strip_prefix(SHAPE_PREFIX) else {
			return false;
		};
		let Some((wide, wide_names)) = rest.split_once(SEPARATOR) else {
			return false;
		};
		let wide_names: BTreeSet<&str> = wide_names.split(',').collect();
		wide != key && names.is_subset(&wide_names) && written_as(wide)
	})
}

/// The prefix of the lines of [`written_fields`] giving the names of the
/// fields of a type: `shape\u{1f}<type>\u{1f}<names>` (`,`-separated).
const SHAPE_PREFIX: &str = "shape\u{1f}";

/// The line of [`SHAPE_PREFIX`] of an object type with a stable name.
fn shape_line(types: &TypeStore, ty: TypeId) -> Option<String> {
	let ty = types.unfold(ty);
	if !matches!(types.kind(ty), TypeKind::Object(_)) {
		return None;
	}
	let key = stable_type_name(types, ty)?;
	let mut names: Vec<&str> = types
		.fields_of(ty)
		.iter()
		.map(|field| types.syms.resolve(field.name))
		.collect();
	names.sort_unstable();
	Some(format!("{SHAPE_PREFIX}{key}{SEPARATOR}{}", names.join(",")))
}

/// Adds `whole:T` for every object type a value of `ty` may hold (itself,
/// its fields, items, members), `whole:*` for a type parameter or a type
/// without a stable name.
fn whole_writes(types: &TypeStore, ty: TypeId, out: &mut BTreeSet<String>) {
	let mut seen = FxHashSet::default();
	let mut stack = vec![ty];
	while let Some(at) = stack.pop() {
		if !seen.insert(at) {
			continue;
		}
		match types.kind(at) {
			TypeKind::Object(_) => {
				match stable_type_name(types, at) {
					Some(name) => out.insert(format!("whole:{name}")),
					None => out.insert("whole:*".to_string()),
				};
				out.extend(shape_line(types, at));
				stack.extend(types.fields_of(at).iter().map(|field| field.ty));
			}
			TypeKind::Rec(_) => stack.extend(types.rec_target(at)),
			TypeKind::Union(list) | TypeKind::Tuple(list) => {
				stack.extend(types.list_items(list).iter().copied())
			}
			TypeKind::Member(nominal, index) => stack.push(types.payload(nominal, index)),
			TypeKind::NamedUnion(nominal, _) => {
				for index in 0..types.sym_list(types.nominal(nominal).members).len() {
					stack.push(types.payload(nominal, index as u16));
				}
			}
			TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
				stack.push(item);
				stack.push(key);
			}
			TypeKind::Pointer(item)
			| TypeKind::Sync(_, item)
			| TypeKind::Ref(item, _)
			| TypeKind::CArray(item, _) => stack.push(item),
			TypeKind::Param(_)
			| TypeKind::ImplicitParam(..)
			| TypeKind::Unknown
			| TypeKind::ErrorTop => {
				out.insert("whole:*".to_string());
			}
			_ => {}
		}
	}
}

impl Prover<'_> {
	/// The type of the values a place holds (an assignment's target has no
	/// type of its own): a local's declared type, a field's, an item's;
	/// `UNKNOWN` otherwise.
	fn place_type(&self, place: NodeId) -> TypeId {
		let file = self.file;
		let ty = file.analysis.type_of(place);
		if !ty.is_unset() && ty != TypeId::UNKNOWN {
			return ty;
		}
		match file.view(place) {
			View::Group(inner) => self.place_type(inner),
			View::Name(_) => match file.analysis.resolution(place) {
				Resolution::Local(symbol) => file.analysis.symbols.symbol(symbol).ty,
				_ => TypeId::UNKNOWN,
			},
			View::Access { members, .. } => {
				let Some(last) = members.last() else {
					return TypeId::UNKNOWN;
				};
				let ty = file.analysis.type_of(last);
				if !ty.is_unset() {
					return ty;
				}
				let Resolution::Field(shape, index) = file.analysis.resolution(last) else {
					return TypeId::UNKNOWN;
				};
				file.types()
					.shape_fields(shape)
					.get(index as usize)
					.map_or(TypeId::UNKNOWN, |field| field.ty)
			}
			View::Index { target, .. } => match file.types().kind(self.place_type(target)) {
				TypeKind::Collection(_, item, _) => item,
				_ => TypeId::UNKNOWN,
			},
			_ => TypeId::UNKNOWN,
		}
	}

	/// See [`written_fields`].
	fn written_fields(&self, out: &mut BTreeSet<String>) {
		let file = self.file;
		let types = file.types();
		// the field a place designates: `x.f` (its owner), `a.b.f` (unknown)
		// (an owner that isn't a known object type, a type parameter's, may
		// be any)
		let owner_key = |ty: TypeId| {
			file.object_type(ty)
				.and_then(|ty| stable_type_name(types, ty))
		};
		let field_of = |place: NodeId| -> Option<(String, Option<String>)> {
			let View::Access { target, members } = file.view(place) else {
				return None;
			};
			let name = self.field_name(members.last()?)?;
			let owner = (members.len() == 1)
				.then(|| owner_key(file.analysis.type_of(target)))
				.flatten();
			let name = types.syms.resolve(name);
			let shape = (members.len() == 1)
				.then(|| shape_line(types, file.analysis.type_of(target)))
				.flatten();
			Some(match owner {
				Some(owner) => (format!("field:{owner}.{name}"), shape),
				None => (format!("field:*.{name}"), None),
			})
		};
		// the sides of relations: the runtime links and unlinks them
		for &(owner, name) in file.analysis.relation_sides.keys() {
			let name = types.syms.resolve(name);
			match owner_key(owner) {
				Some(owner) => out.insert(format!("field:{owner}.{name}")),
				None => out.insert(format!("field:*.{name}")),
			};
			out.extend(shape_line(types, owner));
		}
		// a value C code writes through a pointer
		for (&node, argument) in &file.analysis.foreign_args {
			if matches!(
				argument,
				ForeignArg::Address { mutated: true } | ForeignArg::Array { mutated: true }
			) {
				whole_writes(types, file.analysis.type_of(node), out);
			}
		}
		for at in 0..file.tree.nodes.len() {
			let node = NodeId::new(at);
			match file.view(node) {
				View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
					// a named argument or a field of a literal is no store
					let parent = self.parents.get(at).copied().unwrap_or(usize::MAX);
					if parent != usize::MAX
						&& matches!(
							file.view(NodeId::new(parent)),
							View::Call { .. } | View::Object { .. }
						) {
						continue;
					}
					// a store: to a local, a field or an item (a declaration's
					// name, a parameter list or a member of a type is none)
					let mut place = target;
					while let View::Group(inner) = file.view(place) {
						place = inner;
					}
					match file.view(place) {
						View::Name(_) => {
							if !matches!(file.analysis.resolution(place), Resolution::Local(_)) {
								continue;
							}
						}
						View::Access { members, .. } => {
							if !members.last().is_some_and(|member| {
								matches!(file.analysis.resolution(member), Resolution::Field(..))
							}) {
								continue;
							}
						}
						View::Index { .. } => {}
						// a destructuring store
						View::Tuple { .. }
						| View::Array { .. }
						| View::TuplePattern { .. }
						| View::ArrayPattern { .. }
						| View::ObjectPattern { .. } => {
							out.insert("whole:*".to_string());
							continue;
						}
						_ => continue,
					}
					if let Some((key, shape)) = field_of(place) {
						out.insert(key);
						out.extend(shape);
					}
					whole_writes(types, self.place_type(place), out);
				}
				View::UpdateFields { target, .. } => {
					whole_writes(types, self.place_type(target), out);
				}
				// a field changed under another name
				View::Access { .. } => {
					if let Some((key, shape)) = field_of(node)
						&& self.changed_elsewhere(node)
					{
						out.insert(key);
						out.extend(shape);
						whole_writes(types, file.analysis.type_of(node), out);
					}
				}
				_ => {}
			}
			// a built-in method changing its receiver (`a.f.push(…)`: the
			// field `f`), or moving or replacing the items of a collection
			if let Some(info) = file.analysis.call(node)
				&& let Callee::Builtin(id) = info.callee
				&& let View::Call { callee, .. } = file.view(node)
				&& let View::Access { target, members } = file.view(callee)
			{
				use crate::builtins::effects::{BuiltinEffect, effects_of};
				let effects = effects_of(crate::builtins::get(id).path);
				if effects.is_empty() {
					continue;
				}
				// the receiver: the access without its last member
				let count = members.len();
				let receiver_type = match count {
					0 | 1 => file.analysis.type_of(target),
					_ => file
						.analysis
						.type_of(members.get(count - 2).expect("a member")),
				};
				if count >= 2
					&& let Some(name) = self.field_name(members.get(count - 2).expect("a member"))
				{
					let owner = (count == 2)
						.then(|| owner_key(file.analysis.type_of(target)))
						.flatten();
					let name = types.syms.resolve(name);
					if owner.is_some() {
						out.extend(shape_line(types, file.analysis.type_of(target)));
					}
					out.insert(match owner {
						Some(owner) => format!("field:{owner}.{name}"),
						None => format!("field:*.{name}"),
					});
				}
				// adding an item replaces none (`push` may move the items of
				// an array, not change them)
				let builtin = crate::builtins::get(id);
				let moves = !matches!(builtin.name(), "push" | "reserve" | "add" | "getOrInsert")
					&& effects.iter().any(|effect| {
						!matches!(
							effect,
							BuiltinEffect::MutatesSize
								| BuiltinEffect::MutatesLength
								| BuiltinEffect::MutatesCapacity
						)
					});
				if moves {
					whole_writes(types, receiver_type, out);
				}
			}
		}
	}
}

/// A unit's part of the program's claims for the never-negative places, in
/// a build per unit (`facts::PROOF_CLAIMS`), each line's parts separated
/// by [`SEPARATOR`]: for each goal (`n` with overflow checks, `w` without)
/// and each place its files store to or read, `goal key dependencies` (the
/// keys of the places its stores read, separated by `\u{1e}`: the place
/// holds when they all do) or `goal key !` (a store of the unit is not
/// proven). A write to a field of an unknown owner is `goal field:*.name !`;
/// a store into a generic collection `goal items:* …` (or `values:*`). For
/// the object types whose fields it claims, `fields type names` (the names
/// of its fields, `,`-separated): the objects of a type with every field of
/// another may become objects of that one (a wider object, an instance of a
/// generic type). `unit place module` names every file that may supply a
/// place's values (including its reads conservatively), and
/// `unit array:type module` names the files mentioning an array type of
/// objects. [`resolve_claims`] merges them: places and arrays mentioned by
/// several files cannot carry one file's private index invariant.
pub fn unit_claims(files: &[ProofFile<'_>]) -> BTreeSet<String> {
	claims_of_goals(files, &[true, false])
}

/// [`unit_claims`] of one goal: with overflow checks (`n`) or without
/// (`w`), for a build that reads only the goal of its own checks.
pub fn unit_claims_of_goal(files: &[ProofFile<'_>], overflow_checks: bool) -> BTreeSet<String> {
	claims_of_goals(files, &[overflow_checks])
}

/// [`unit_claims`] of some goals (`checked` of [`Goal::NonNegative`]).
fn claims_of_goals(files: &[ProofFile<'_>], goals: &[bool]) -> BTreeSet<String> {
	let mut out = BTreeSet::new();
	for file in files {
		if file.opaque() {
			out.insert(OPAQUE.to_string());
		}
		let types = file.types();
		// A literal also supplies fields filled from a spread, an omitted
		// optional or another file's constant default. Those have no local
		// integer expression from which `candidates` could discover them.
		for object in file.analysis.objects.values() {
			let prover = Prover::new(file, Goal::NonNegative { checked: true });
			for &(name, value) in &object.fields {
				if let Some(key) = prover.unit_key(Claim::Field(object.ty, name)) {
					out.insert(format!("unit{SEPARATOR}{key}{SEPARATOR}{}", file.module));
					// `holds(Field)` cannot prove these values from a local
					// expression either. Publish its failure even when no
					// local read made the field a candidate.
					if !matches!(value, FieldValue::Given(_) | FieldValue::Default(_)) {
						for &checked in goals {
							let goal = if checked { "n" } else { "w" };
							out.insert(format!("{goal}{SEPARATOR}{key}{SEPARATOR}!"));
						}
					}
				}
			}
			if let Some(line) = shape_line(types, object.ty) {
				out.insert(format!("fields{}", &line["shape".len()..]));
			}
		}
		for &checked in goals {
			let mut prover = Prover::new(file, Goal::NonNegative { checked }).per_unit();
			let goal = prover.goal_name();
			let mut pending: Vec<Claim> = prover.candidates();
			let mut seen: FxHashSet<Claim> = FxHashSet::default();
			while let Some(claim) = pending.pop() {
				if !seen.insert(claim) {
					continue;
				}
				let Some(key) = prover.unit_key(claim) else {
					continue;
				};
				// Include reads too: this conservative set ensures that a
				// file claiming a place always names itself as one of its
				// users, even when it stores no value there locally.
				if matches!(
					claim,
					Claim::Field(..) | Claim::Items(_) | Claim::MapValues(_)
				) {
					out.insert(format!("unit{SEPARATOR}{key}{SEPARATOR}{}", file.module));
				}
				if let Claim::Field(owner, _) = claim
					&& let Some(owner) = file.object_type(owner)
					&& let Some(name) = stable_type_name(types, owner)
				{
					let mut names: Vec<&str> = types
						.fields_of(owner)
						.iter()
						.map(|field| types.syms.resolve(field.name))
						.collect();
					names.sort_unstable();
					out.insert(format!(
						"fields{SEPARATOR}{name}{SEPARATOR}{}",
						names.join(",")
					));
				}
				prover.claims.clear();
				let holds = prover.holds(claim);
				let mut dependencies: Vec<String> = Vec::new();
				let mut unnamed = false;
				for &dependency in &prover.claims {
					match prover.unit_key(dependency) {
						Some(key) => dependencies.push(key),
						None => unnamed = true,
					}
					pending.push(dependency);
				}
				dependencies.sort();
				dependencies.dedup();
				let edge = if holds && !unnamed {
					dependencies.join(&DEPENDENCIES.to_string())
				} else {
					"!".to_string()
				};
				out.insert(format!("{goal}{SEPARATOR}{key}{SEPARATOR}{edge}"));
			}
			// writes to a field of an unknown owner
			for name in prover.unknown_field_writes() {
				out.insert(format!("{goal}{SEPARATOR}field:*.{name}{SEPARATOR}!"));
				out.insert(format!(
					"unit{SEPARATOR}field:*.{name}{SEPARATOR}{}",
					file.module
				));
			}
		}
		// An array family is local only if no other file mentions its
		// type. Its local walk checks every use and every parameter too.
		let mut seen = FxHashSet::default();
		for at in 0..file.tree.nodes.len() {
			let ty = file.analysis.type_of(NodeId::new(at));
			if let TypeKind::Collection(CollKind::Array, item, _) = types.kind(ty)
				&& matches!(types.kind(item), TypeKind::Object(_))
				&& seen.insert(ty)
				&& let Some(name) = stable_type_name(types, ty)
			{
				out.insert(format!(
					"unit{SEPARATOR}array:{name}{SEPARATOR}{}",
					file.module
				));
			}
		}
	}
	out
}

/// The failed claims of the program from the merged [`unit_claims`]: a
/// place fails when a unit couldn't prove one of its stores, or when a place
/// it depends on fails (the greatest fixpoint: every other place holds). A
/// field of a type depends on the same field of every type with all of its
/// fields; a failed `field:*.name` fails that field of every type, a failed
/// `items:*` (`values:*`) the items (values) of every type. The result
/// holds `goal key` (separated by [`SEPARATOR`]) for each failed place, and
/// [`OPAQUE`]. It also holds `i key` for a field, collection or array type
/// used by several files, including wider shapes and generic stores.
pub fn resolve_claims(edges: &BTreeSet<String>) -> BTreeSet<String> {
	let mut failed: BTreeSet<String> = BTreeSet::new();
	// the places depending on each place, by goal
	let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
	let mut queue: Vec<String> = Vec::new();
	let mut shapes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
	let mut goals: BTreeSet<String> = BTreeSet::new();
	let mut units: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
	for edge in edges {
		if edge == OPAQUE {
			failed.insert(OPAQUE.to_string());
			continue;
		}
		let mut parts = edge.splitn(3, SEPARATOR);
		let (Some(goal), Some(key), Some(rest)) = (parts.next(), parts.next(), parts.next()) else {
			continue;
		};
		if goal == "fields" {
			shapes.insert(
				key.to_string(),
				rest.split(',').map(str::to_string).collect(),
			);
			continue;
		}
		if goal == "unit" {
			units
				.entry(key.to_string())
				.or_default()
				.insert(rest.to_string());
			continue;
		}
		goals.insert(goal.to_string());
		let place = format!("{goal}{SEPARATOR}{key}");
		if rest == "!" {
			queue.push(place);
			continue;
		}
		for dependency in rest
			.split(DEPENDENCIES)
			.filter(|dependency| !dependency.is_empty())
		{
			dependents
				.entry(format!("{goal}{SEPARATOR}{dependency}"))
				.or_default()
				.push(place.clone());
		}
	}
	// the objects of a wider type may become objects of a narrower one
	for (narrow, narrow_names) in &shapes {
		for (wide, wide_names) in &shapes {
			if narrow == wide || !narrow_names.is_subset(wide_names) {
				continue;
			}
			for goal in &goals {
				for name in narrow_names {
					dependents
						.entry(format!("{goal}{SEPARATOR}field:{wide}.{name}"))
						.or_default()
						.push(format!("{goal}{SEPARATOR}field:{narrow}.{name}"));
				}
			}
			for name in narrow_names {
				if let Some(writers) = units.get(&format!("field:{wide}.{name}")).cloned() {
					units
						.entry(format!("field:{narrow}.{name}"))
						.or_default()
						.extend(writers);
				}
			}
		}
	}
	// Generic stores and stores through an unknown owner may reach every
	// concrete place they stand for, regardless of its type's spelling.
	let stars: Vec<(String, BTreeSet<String>)> = units
		.iter()
		.filter(|(key, _)| key.starts_with("field:*.") || *key == "items:*" || *key == "values:*")
		.map(|(key, writers)| (key.clone(), writers.clone()))
		.collect();
	for (key, writers) in &mut units {
		for (star, extra) in &stars {
			let matches = if let Some(name) = star.strip_prefix("field:*.") {
				key.starts_with("field:") && (name == "*" || key.ends_with(&format!(".{name}")))
			} else {
				key.starts_with(star.trim_end_matches('*'))
			};
			if matches {
				writers.extend(extra.iter().cloned());
			}
		}
		if writers.len() > 1 {
			failed.insert(format!("i{SEPARATOR}{key}"));
		}
	}
	let propagate = |queue: &mut Vec<String>, failed: &mut BTreeSet<String>| {
		while let Some(place) = queue.pop() {
			if !failed.insert(place.clone()) {
				continue;
			}
			if let Some(places) = dependents.get(&place) {
				queue.extend(places.iter().cloned());
			}
		}
	};
	propagate(&mut queue, &mut failed);
	// a star fails every place it stands for: their dependents are found
	// by name
	let stars: Vec<(String, String, String)> = failed
		.iter()
		.filter_map(|place| {
			let (goal, key) = place.split_once(SEPARATOR)?;
			let (prefix, suffix) = if let Some(name) = key.strip_prefix("field:*.") {
				("field:", format!(".{name}"))
			} else if key == "items:*" {
				("items:", String::new())
			} else if key == "values:*" {
				("values:", String::new())
			} else {
				return None;
			};
			Some((goal.to_string(), prefix.to_string(), suffix))
		})
		.collect();
	if !stars.is_empty() {
		let mut queue: Vec<String> = dependents
			.keys()
			.filter(|place| {
				stars.iter().any(|(goal, prefix, suffix)| {
					place.starts_with(&format!("{goal}{SEPARATOR}{prefix}"))
						&& (suffix == ".*" || place.ends_with(suffix.as_str()))
				})
			})
			.cloned()
			.collect();
		propagate(&mut queue, &mut failed);
	}
	failed
}

/// [`resolve_claims`] for `FactKind::finish`.
pub fn resolve_claims_owned(edges: BTreeSet<String>) -> BTreeSet<String> {
	resolve_claims(&edges)
}

/// What the proofs of a file establish for the backends (`index_facts`):
/// the `array[index]` nodes whose index is proven in bounds (no check), and
/// the integer nodes proven never negative (no count from the end).
#[derive(Debug, Default)]
pub struct IndexFacts {
	/// The index nodes proven.
	proven: FxHashSet<NodeId>,
	/// The integer nodes proven never negative.
	non_negative: FxHashSet<NodeId>,
}

impl IndexFacts {
	/// Whether the index of this `array[index]` node is proven in bounds.
	pub fn proven_index(&self, node: NodeId) -> bool {
		self.proven.contains(&node)
	}

	/// Whether an integer node is proven never negative.
	pub fn proven_non_negative(&self, node: NodeId) -> bool {
		self.non_negative.contains(&node)
	}
}

/// Which places the proofs of a file see.
#[derive(Copy, Clone)]
pub enum ProofScope<'a> {
	/// None: no whole-program claims are available.
	Nothing,
	/// The file is the whole program: the index proofs and the
	/// never-negative places, over its own stores.
	WholeFile,
	/// A file of a program: private-array indices and never-negative places, by
	/// the program's failed claims ([`unit_claims`] of every unit, merged
	/// by [`resolve_claims`]; `failed` says whether a key is among them),
	/// the file named by its module. A place used by another file cannot
	/// carry a private array's index invariant.
	Unit {
		module: &'a str,
		failed: &'a dyn Fn(&str) -> bool,
	},
}

/// The interval of an integer node a backend may rely on to drop a check
/// or a count from the end: none when overflows wrap (no overflow checks)
/// and its type is narrower than 64 bits. The analysis assumes that an
/// overflow stops the program, and a narrow value wraps after a few steps
/// (`Int8`: 128 increments).
pub fn reliable_interval(
	analysis: &Analysis,
	types: &TypeStore,
	node: NodeId,
	overflow_checks: bool,
) -> Option<crate::types::Interval> {
	if !overflow_checks
		&& types
			.as_int(analysis.type_of(node))
			.is_some_and(|range| range.max < ethnum::I256::from(i64::MAX))
	{
		return None;
	}
	analysis.interval(node)
}

/// The index proofs and the never-negative places of a file
/// (`design/decisions.md`, "Index proofs for fields" and "Never-negative
/// places"), for both backends.
///
/// An array `X` is the only array of its type in the program: one `let` or
/// `mutable` of the file, every other value of the type being a parameter
/// that receives `X` at every call (directly, or from another such
/// parameter); `X` is never assigned, and is only indexed, measured, pushed
/// to, reserved or iterated (nothing removes an item). Its initial length
/// `k` is the length of its literal (0 otherwise). An index expression is
/// proven when it is an integer known to be in `0..k`, `X.length - 1` with
/// `k >= 1`, or a read of a place every store of which is proven (a
/// [`Claim`]): a greatest fixpoint. Values stay indices once proven, since
/// `X` never shrinks.
pub fn index_facts(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	types: &TypeStore,
	scope: ProofScope<'_>,
	overflow_checks: bool,
) -> IndexFacts {
	let mut result = IndexFacts::default();
	match scope {
		ProofScope::Nothing => result,
		ProofScope::Unit { module, failed } => {
			let file = ProofFile::new(tree, analysis, types).with_module(module);
			result.non_negative = non_negative(&file, Some(failed), overflow_checks);
			result.proven = proven_indices(&file, Some(failed), overflow_checks);
			result
		}
		ProofScope::WholeFile => {
			let file = ProofFile::new(tree, analysis, types);
			// `Json.parse` or C code builds objects, arrays and maps out of
			// any integers
			if file.opaque() {
				return result;
			}
			result.non_negative = non_negative(&file, None, overflow_checks);
			result.proven = proven_indices(&file, None, overflow_checks);
			result
		}
	}
}

/// The index nodes proven in bounds (see [`index_facts`]).
fn proven_indices(
	file: &ProofFile<'_>,
	program: Option<&dyn Fn(&str) -> bool>,
	overflow_checks: bool,
) -> FxHashSet<NodeId> {
	let analysis = file.analysis;
	let types = file.types();
	let count = file.tree.nodes.len();
	let mut proven = FxHashSet::default();
	// the candidate arrays: a let or mutable of an array of objects
	let mut by_type: FxHashMap<TypeId, Vec<SymbolId>> = FxHashMap::default();
	for (index, symbol) in analysis.symbols.symbols().iter().enumerate() {
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
	let mut by_type: Vec<(TypeId, Vec<SymbolId>)> = by_type.into_iter().collect();
	by_type.sort_by_key(|(ty, _)| *ty);
	for (array_type, symbols) in by_type {
		let [array] = symbols[..] else {
			continue;
		};
		if let Some(failed) = program
			&& (file.exported_value(array)
				|| stable_type_name(types, array_type)
					.is_none_or(|name| failed(&format!("i{SEPARATOR}array:{name}"))))
		{
			continue;
		}
		let Some(arrays) = array_family(file, array, array_type, program.is_some()) else {
			continue;
		};
		let declaration = analysis.symbols.symbol(array).decl_node;
		let initial = match file.view(declaration) {
			View::Declaration {
				value: Some(value), ..
			} => match file.view(value) {
				View::Array { items } => items.len() as u64,
				_ => 0,
			},
			_ => continue,
		};
		let mut prover = Prover::new(
			file,
			Goal::Index {
				arrays: &arrays,
				initial,
			},
		)
		.wrapping(!overflow_checks);
		if let Some(failed) = program {
			prover = prover.with_program(failed);
		}
		// the indices of the array, then the fixpoint of their claims
		let indices: Vec<(NodeId, NodeId)> = (0..count)
			.filter_map(|at| match file.view(NodeId::new(at)) {
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
				proven.insert(node);
			}
		}
	}
	proven
}

/// The never-negative proofs of the indices that may be negative by their
/// types and intervals: the index of every `x[index]` and every argument
/// of a built-in `get`.
fn non_negative(
	file: &ProofFile<'_>,
	program: Option<&dyn Fn(&str) -> bool>,
	overflow_checks: bool,
) -> FxHashSet<NodeId> {
	let analysis = file.analysis;
	let types = file.types();
	let mut roots = Vec::new();
	for at in 0..file.tree.nodes.len() {
		let node = NodeId::new(at);
		match file.view(node) {
			View::Index { index, .. } => roots.push(index),
			View::Call { .. } => {
				let Some(info) = analysis.call(node) else {
					continue;
				};
				let Callee::Builtin(id) = info.callee else {
					continue;
				};
				if !crate::builtins::get(id).path.ends_with("#get") {
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
	roots.retain(|&root| {
		types.as_int(analysis.type_of(root)).is_some()
			&& reliable_interval(analysis, types, root, overflow_checks)
				.is_none_or(|interval| interval.lo < ethnum::I256::ZERO)
	});
	if roots.is_empty() {
		return FxHashSet::default();
	}
	let goal = Goal::NonNegative {
		checked: overflow_checks,
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

/// The array and the parameters that receive it, when it is the only array
/// of its type and never shrinks.
fn array_family(
	file: &ProofFile<'_>,
	array: SymbolId,
	array_type: TypeId,
	per_unit: bool,
) -> Option<FxHashSet<SymbolId>> {
	let analysis = file.analysis;
	let parents = &file.parents;
	let count = file.tree.nodes.len();
	let declaration = analysis.symbols.symbol(array).decl_node;
	let initializer = match file.view(declaration) {
		View::Declaration { value, .. } => value,
		_ => None,
	};
	let parameter = |symbol: SymbolId| {
		matches!(
			analysis.symbols.symbol(symbol).kind,
			SymbolKind::Parameter { .. }
		)
	};
	let mut family: FxHashSet<SymbolId> = FxHashSet::default();
	family.insert(array);
	let mut names = Vec::new();
	for at in 0..count {
		let node = NodeId::new(at);
		if analysis.type_of(node) != array_type
			|| analysis.has_flags(node, NodeFlags::TYPE_EXPR)
			|| Some(node) == initializer
			|| node == declaration
		{
			continue;
		}
		match file.view(node) {
			View::Name(_) => match analysis.resolution(node) {
				Resolution::Local(symbol) => {
					if symbol != array && !parameter(symbol) {
						return None;
					}
					if parameter(symbol) {
						family.insert(symbol);
					}
					names.push(node);
				}
				_ => return None,
			},
			// a parameter's own node
			_ if analysis.symbols.symbol_of_decl(node).is_some_and(parameter) => {}
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
		let fine = match file.view(NodeId::new(parent)) {
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
				match analysis.resolution(members.first().expect("a member")) {
					Resolution::Builtin(id) => matches!(
						crate::builtins::get(id).path,
						"Array#length" | "Array#push" | "Array#reserve" | "Array#get"
					),
					_ => false,
				}
			}
			View::Call { .. } => {
				let info = analysis.call(NodeId::new(parent))?;
				let Callee::Function(function) = info.callee else {
					return None;
				};
				let fn_info = analysis.function(function)?;
				info.arguments
					.iter()
					.enumerate()
					.all(|(position, argument)| {
						!matches!(argument, Argument::Given(value) if *value == name)
							|| fn_info.params.get(position).is_some_and(|param| {
								analysis.symbols.symbol(param.symbol).ty == array_type
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
		if per_unit && !file.private_function(function) {
			return None;
		}
		let SymbolKind::Parameter { index } = analysis.symbols.symbol(symbol).kind else {
			return None;
		};
		for at in 0..count {
			let node = NodeId::new(at);
			if let Resolution::Local(callee) = analysis.resolution(node)
				&& analysis.symbols.symbol(callee).decl_node == function
			{
				let parent = parents.get(at).copied().unwrap_or(usize::MAX);
				let direct = parent != usize::MAX
					&& matches!(file.view(NodeId::new(parent)), View::Call { callee, .. } if callee == node);
				if !direct {
					return None;
				}
			}
			let Some(info) = analysis.call(node) else {
				continue;
			};
			if info.callee != Callee::Function(function) {
				continue;
			}
			let given = match info.arguments.get(index as usize) {
				Some(Argument::Given(value)) => *value,
				_ => return None,
			};
			let member = matches!(file.view(given), View::Name(_))
				&& matches!(analysis.resolution(given),
					Resolution::Local(passed) if family.contains(&passed));
			if !member {
				return None;
			}
		}
	}
	Some(family)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use super::{DEPENDENCIES, OPAQUE, SEPARATOR, resolve_claims};

	/// Lines written with `|` between the parts and `;` between the
	/// dependencies.
	fn set(items: &[&str]) -> BTreeSet<String> {
		items
			.iter()
			.map(|item| {
				item.replace('|', &SEPARATOR.to_string())
					.replace(';', &DEPENDENCIES.to_string())
			})
			.collect()
	}

	fn has(failed: &BTreeSet<String>, place: &str) -> bool {
		failed.contains(&place.replace('|', &SEPARATOR.to_string()))
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
		failing.extend(set(&["n|local:m:2|!"]));
		assert_eq!(
			resolve_claims(&failing),
			set(&["n|local:m:1", "n|local:m:2"])
		);
	}

	#[test]
	fn a_field_written_with_an_unknown_owner_fails_under_every_owner() {
		let mut edges = set(&[
			"n|field:T.at|",
			"n|field:U.at|",
			"n|field:U.to|",
			"n|local:m:1|field:T.at",
			"n|field:*.at|!",
		]);
		edges.insert(OPAQUE.to_string());
		let failed = resolve_claims(&edges);
		// the places reading such a field fail; the consumer checks the
		// field's own `field:*.at` (`Prover::claim`)
		assert!(has(&failed, "n|field:T.at") && has(&failed, "n|field:*.at"));
		assert!(has(&failed, "n|local:m:1") && failed.contains(OPAQUE));
		assert!(!has(&failed, "n|field:U.to"));
	}

	#[test]
	fn a_wider_type_fails_the_fields_of_the_narrower_ones() {
		// `{a, b}` may become a `{a}`: the store of `{a, b}.a` reaches `{a}.a`
		let edges = set(&[
			"fields|{a: Int64}|a",
			"fields|{a: Int64, b: Int64}|a,b",
			"fields|{c: Int64}|c",
			"n|field:{a: Int64}.a|",
			"n|field:{c: Int64}.c|",
			"n|local:m:1|field:{a: Int64}.a",
			"n|field:{a: Int64, b: Int64}.a|!",
		]);
		let failed = resolve_claims(&edges);
		assert!(has(&failed, "n|field:{a: Int64}.a"), "{failed:?}");
		assert!(has(&failed, "n|local:m:1"), "{failed:?}");
		assert!(!has(&failed, "n|field:{c: Int64}.c"), "{failed:?}");
	}

	#[test]
	fn a_store_into_a_generic_collection_fails_every_collection() {
		let edges = set(&[
			"n|items:Array<Int64>|",
			"n|values:Map<Int64, Int64>|",
			"n|local:m:1|items:Array<Int64>",
			"n|items:*|!",
		]);
		let failed = resolve_claims(&edges);
		assert!(has(&failed, "n|local:m:1"), "{failed:?}");
		assert!(!has(&failed, "n|values:Map<Int64, Int64>"), "{failed:?}");
	}

	#[test]
	fn indices_require_places_and_arrays_private_to_one_file() {
		let edges = set(&[
			"unit|field:T.next|own",
			"unit|field:T.next|other",
			"unit|field:T.back|own",
			"unit|array:Array<T>|own",
			"unit|array:Array<T>|other",
		]);
		let failed = resolve_claims(&edges);
		assert!(has(&failed, "i|field:T.next"));
		assert!(has(&failed, "i|array:Array<T>"));
		assert!(!has(&failed, "i|field:T.back"));
	}

	#[test]
	fn foreign_wider_and_unknown_stores_prevent_private_field_proofs() {
		let edges = set(&[
			"fields|T|next",
			"fields|Wide|extra,next",
			"unit|field:T.next|own",
			"unit|field:Wide.next|other",
			"unit|items:Array<UInt32>|own",
			"unit|items:*|generic",
			"unit|values:Map<Int32, UInt32>|own",
			"unit|field:T.back|own",
			"unit|field:*.back|unknown",
		]);
		let failed = resolve_claims(&edges);
		assert!(has(&failed, "i|field:T.next"));
		assert!(has(&failed, "i|field:T.back"));
		assert!(has(&failed, "i|items:Array<UInt32>"));
		assert!(!has(&failed, "i|values:Map<Int32, UInt32>"));
	}

	#[test]
	fn a_units_local_index_fixpoint_keeps_foreign_field_stores_checked() {
		use crate::ids::NodeId;
		use crate::syntax::{self, View};

		use super::{ProofFile, ProofScope, index_facts, unit_claims};

		let source = "type Link = { next: UInt32 }\nmutable links: Array<Link> = [{ next = 0 }]\nfunction hop = (items: Array<Link>, from: UInt32): UInt32 throws IndexError => try items[try items[from].next].next\nconsole.log(hop(links, 0) catch error => 99)\n";
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let types = session.types();
		let proof = ProofFile::new(tree, analysis, types).with_module("own");
		let mut edges = unit_claims(&[proof]);
		let indices: Vec<_> = (0..tree.nodes.len())
			.map(NodeId::new)
			.filter(|&node| matches!(syntax::view(tree, node), View::Index { .. }))
			.collect();
		assert_eq!(indices.len(), 2);
		let prove = |edges: &BTreeSet<String>| {
			let failed = resolve_claims(edges);
			let failed = |key: &str| failed.contains(key);
			index_facts(
				tree,
				analysis,
				types,
				ProofScope::Unit {
					module: "own",
					failed: &failed,
				},
				true,
			)
		};
		let facts = prove(&edges);
		assert!(
			indices.iter().all(|&node| facts.proven_index(node)),
			"{facts:?}"
		);

		let (other, other_file) = crate::analyze_source(
			"type Wide = { next: UInt32, extra: Integer }\nlet wide = Wide { next = 1000000, extra = 1 }\nconsole.log(wide)\n",
		);
		edges.extend(unit_claims(&[ProofFile::new(
			other.tree(other_file),
			other.analysis(other_file),
			other.types(),
		)
		.with_module("other")]));
		let facts = prove(&edges);
		// The parameter passed zero is still proven locally; the index
		// read from a field that another file may store is not.
		assert_eq!(
			indices
				.iter()
				.filter(|&&node| facts.proven_index(node))
				.count(),
			1,
			"{facts:?}"
		);
	}
}
