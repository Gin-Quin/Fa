//! Effects in the walk (advanced brief §3, types brief §4.3): places of
//! expressions, `mutates`/`invalidates` in signatures, effects of writes
//! and calls, their coverage by the declared effects, the iteration rule.
//!
//! - An expression designating a place (`player.score`, `bodies[j].vx`, a
//!   `use` alias, a `for` variable over a collection) has evaluated flow places
//!   ([`Walker::places_of`]): a root symbol and steps, an index step carrying
//!   the identity of its index (advanced brief §5.4).
//! - Every write and every call with effects goes through
//!   [`Walker::record_effect`]: the facts at and below the place go, the
//!   iteration rule is checked (`REF012`), the references it invalidates are
//!   marked (`references.rs`), and an effect on a parameter, `self` or a
//!   module-level value is accumulated in the function being walked.
//! - A named function with a block body, or with declared effects, must
//!   declare every effect it has (`EFF001`–`EFF004`); a single-expression
//!   body, a lambda, a test and the top level infer theirs. The function
//!   type carries the effects (`FnSig::effects`), substituted at each call.
//! - A lambda's effects on places it captures are effects of the code that
//!   creates it (it runs during the call it is given to); a local function's
//!   are applied at each of its calls.

use std::sync::Arc;

use ethnum::I256;

use crate::analysis::{Argument, Callee};
use crate::builtins::effects::BuiltinEffect;
use crate::check::{FnCx, Walker};
use crate::diagnostics::codes::advanced::{
	EFF001, EFF002, EFF003, EFF004, EFF005, EFF007, EFF008, EFF009, EFF011, EFF014, REF012, REF014,
	REF015,
};
use crate::effects::{covers_any, normalized_set, path_text, set_covers};
use crate::flow::{Identity, PlaceRoot, Step};
use crate::ids::{EffectSetId, NodeId, PathId, PlaceId, SymbolId, TypeId};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, ArithOp, NodeList, View};
use crate::types::{CollKind, ConstValue, PathRoot, PathStep, TypeKind};

/// An effect of the function being walked, on a place of its flow.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EffectEntry {
	/// The place changed.
	pub place: PlaceId,
	/// `invalidates` (elements removed or moved), else `mutates`.
	pub invalidates: bool,
	/// The write or the call.
	pub node: NodeId,
	/// The effect comes from a call (`EFF001`), not a write (`EFF003`).
	pub call: bool,
}

/// Who the root of a place belongs to, seen from the function being walked.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Owner {
	/// A local of the function: not an effect of the function.
	Local,
	/// A parameter (or `self`) of the function.
	Param,
	/// A local or parameter of an enclosing function, captured.
	Captured,
	/// A module-level value.
	Module,
	/// Code outside any function (the top level), or not a value.
	Outside,
}

/// Whether a place may be written.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Writable {
	/// A `mutable` root, a parameter (its effects are declared or inferred).
	Yes,
	/// An immutable root (`let`, a `for` copy, a pattern binding…).
	No(SymbolId),
}

impl Walker<'_> {
	// ── places ────────────────────────────────────────────────────────────

	/// Every place the expression may designate in the current flow state.
	/// Consumers of effects and ownership must inspect the complete set.
	pub(crate) fn places_of(&mut self, n: NodeId) -> Vec<PlaceId> {
		let n = self.strip_groups(n);
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, n) {
			View::Try(inner) => self.places_of(inner),
			View::Call { .. } => self
				.reference_results
				.get(&n)
				.map(|value| value.places.clone())
				.unwrap_or_default(),
			View::Name(_) => match self.a.resolution(n) {
				Resolution::Local(symbol) => self.symbol_places(symbol),
				_ => Vec::new(),
			},
			View::Access { target, members } => {
				self.access_places(target, &members.iter().collect::<Vec<_>>())
			}
			View::Index { target, index } => {
				let places = self.places_of(target);
				let step = if self.type_of(self.strip_groups(index)) == TypeId::RANGE {
					Step::All
				} else {
					Step::Index(self.reference_index_identity(index, Some(self.type_of(target))))
				};
				places
					.into_iter()
					.map(|place| self.places.child(place, step))
					.collect()
			}
			_ => self.evaluated_origins(n).places,
		}
	}
	pub(crate) fn access_places(&mut self, target: NodeId, members: &[NodeId]) -> Vec<PlaceId> {
		let mut members = members.iter().copied();
		let mut places = self.places_of(target);
		while places.is_empty() {
			let Some(member) = members.next() else {
				return places;
			};
			let Resolution::Local(symbol) = self.a.resolution(member) else {
				return places;
			};
			if !self.is_namespace(symbol) {
				places = self.symbol_places(symbol);
				break;
			}
		}
		for member in members {
			places = places
				.into_iter()
				.filter_map(|place| {
					let place = self.reference_field_target(place);
					self.member_step(place, member)
				})
				.collect();
		}
		places
	}

	/// A singleton proof for numeric facts and narrowing only.
	pub(crate) fn place_of(&mut self, node: NodeId) -> Option<PlaceId> {
		let places = self.places_of(node);
		if let [place] = places.as_slice() {
			Some(*place)
		} else {
			None
		}
	}

	pub(crate) fn symbol_places(&mut self, symbol: SymbolId) -> Vec<PlaceId> {
		if let Some(state) = self.body.flow.ref_state(symbol) {
			return self.facts.refs.get(state).places.to_vec();
		}
		// Fixed iteration aliases and captures have a single entry place.
		if let Some(&place) = self.aliases.get(&symbol) {
			return vec![place];
		}
		match self.a.symbols.symbol(symbol).kind {
			SymbolKind::Function
			| SymbolKind::Type
			| SymbolKind::TypeParameter
			| SymbolKind::Namespace
			| SymbolKind::Test
			| SymbolKind::NameAlias => Vec::new(),
			_ => vec![self.places.symbol(symbol)],
		}
	}

	pub(crate) fn symbol_target(&mut self, symbol: SymbolId) -> Option<PlaceId> {
		let places = self.symbol_places(symbol);
		if let [place] = places.as_slice() {
			Some(*place)
		} else {
			None
		}
	}

	/// The place of a member of `place`: a field, `.length`, `.size`.
	pub(crate) fn member_step(&mut self, place: PlaceId, member: NodeId) -> Option<PlaceId> {
		let View::Name(name) = self.view(member) else {
			return None;
		};
		let text = self.tree.name(name).to_string();
		let step = match self.a.resolution(member) {
			Resolution::Field(..) => Step::Field(self.s.types.syms.intern(&text)),
			Resolution::Builtin(builtin) => {
				let path = crate::builtins::get(builtin).path;
				if path.ends_with("#length") {
					Step::Length
				} else if path.ends_with("#size") {
					Step::Size
				} else if path.ends_with("#capacity") {
					Step::Capacity
				} else if path.starts_with("Range#") || path == "Guard#value" {
					Step::Field(self.s.types.syms.intern(&text))
				} else {
					return None;
				}
			}
			_ => return None,
		};
		Some(self.places.child(place, step))
	}

	/// The identity of an index (advanced brief §5.4): its exact value, an
	/// immutable binding plus an offset, `c.length` plus an offset, else a
	/// snapshot of its interval.
	pub(crate) fn identity_of(&mut self, index: NodeId) -> Identity {
		let index = self.strip_groups(index);
		// a string or boolean key of a map
		if matches!(self.view(index), View::String(_) | View::Boolean(_))
			&& let Some(key) = self.key_constant(index)
		{
			return Identity::Key(key);
		}
		let set = self.value_set(index);
		if let Some(value) = set.as_ref().and_then(|set| set.as_exact())
			&& let Ok(value) = i64::try_from(value)
		{
			return Identity::Exact(value);
		}
		if let Some(identity) = self.symbolic_identity(index) {
			return identity;
		}
		match set {
			Some(set) if set.hull != crate::types::Interval::full() => {
				Identity::Interval(self.facts.interval(set.hull))
			}
			_ => Identity::Unknown,
		}
	}

	/// References save the evaluated sequence index. A negative value is not
	/// numerically distinct from its normalized positive position; map keys keep
	/// their raw value. Length-based indices retain an interval snapshot.
	pub(super) fn reference_index_identity(
		&mut self,
		index: NodeId,
		container: Option<TypeId>,
	) -> Identity {
		let identity = self.identity_of(index);
		self.normalize_reference_index(identity, container)
	}

	fn normalize_reference_index(
		&mut self,
		identity: Identity,
		container: Option<TypeId>,
	) -> Identity {
		if container.is_some_and(|ty| {
			matches!(
				self.s.types.kind(ty),
				TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
			)
		}) {
			return identity;
		}
		match identity {
			Identity::Exact(value) if value < 0 => Identity::FromEnd(value),
			Identity::Symbol(symbol, offset)
				if self
					.identity_interval(identity)
					.is_none_or(|range| range.lo < I256::ZERO) =>
			{
				Identity::NormalizedSymbol(symbol, offset)
			}
			Identity::Interval(id) if self.facts.interval_value(id).lo < I256::ZERO => {
				Identity::Unknown
			}
			Identity::LengthOf(place, offset)
				if self
					.identity_interval(identity)
					.is_some_and(|range| range.lo >= I256::ZERO) =>
			{
				Identity::LengthSnapshot(place, offset, self.body.flow.track_reference_epoch(place))
			}
			Identity::LengthOf(..) => Identity::Unknown,
			other => other,
		}
	}

	/// `i`, `i + 1`, `c.length - 1` (an immutable `i`).
	fn symbolic_identity(&mut self, index: NodeId) -> Option<Identity> {
		let tree = Arc::clone(&self.tree);
		let (base, offset) = match syntax::view(&tree, index) {
			View::Arithmetic {
				op: op @ (ArithOp::Add | ArithOp::Subtract),
				operands,
			} if operands.len() == 2 => {
				let (left, right) = (operands.first()?, operands.iter().nth(1)?);
				let View::Integer(value) = syntax::view(&tree, self.strip_groups(right)) else {
					return None;
				};
				let value = i32::try_from(value).ok()?;
				(left, if op == ArithOp::Add { value } else { -value })
			}
			_ => (index, 0),
		};
		if let Some(collection) = self.length_access(base) {
			let place = self.symbol_target(collection)?;
			return Some(Identity::LengthOf(place, offset));
		}
		let symbol = self.read_local(base)?;
		matches!(
			self.a.symbols.symbol(symbol).kind,
			SymbolKind::Let
				| SymbolKind::Parameter { .. }
				| SymbolKind::LoopVariable { copy: true }
		)
		.then_some(Identity::Symbol(symbol, offset))
	}

	/// The root symbol of a place.
	pub(crate) fn place_symbol(&self, place: PlaceId) -> Option<SymbolId> {
		match self.places.get(place).root {
			PlaceRoot::Symbol(symbol) => Some(symbol),
			_ => None,
		}
	}

	/// The steps of a place, from its root.
	pub(crate) fn place_steps(&self, place: PlaceId) -> Vec<Step> {
		let mut steps = Vec::new();
		let mut current = place;
		loop {
			let entry = self.places.get(current);
			match (entry.parent, entry.step) {
				(Some(parent), Some(step)) => {
					steps.push(step);
					current = parent;
				}
				_ => break,
			}
		}
		steps.reverse();
		steps
	}

	/// Prints a place for messages: `bodies[j].vx`, `scores[..]`.
	pub(crate) fn place_text(&self, place: PlaceId) -> String {
		let mut text = self.place_symbol(place).map_or_else(
			|| "this value".to_string(),
			|symbol| {
				let name = self.a.symbols.symbol(symbol).name;
				self.s.types.syms.resolve(name).to_string()
			},
		);
		for step in self.place_steps(place) {
			match step {
				Step::Field(name) => {
					text.push('.');
					text.push_str(self.s.types.syms.resolve(name));
				}
				Step::Index(Identity::Exact(value)) => text.push_str(&format!("[{value}]")),
				Step::Index(Identity::Symbol(symbol, offset)) => {
					let name = self.a.symbols.symbol(symbol).name;
					let name = self.s.types.syms.resolve(name);
					match offset {
						0 => text.push_str(&format!("[{name}]")),
						k if k > 0 => text.push_str(&format!("[{name} + {k}]")),
						k => text.push_str(&format!("[{name} - {}]", -k)),
					}
				}
				Step::Index(_) | Step::All | Step::Range(..) | Step::Child => text.push_str("[..]"),
				Step::Length => text.push_str(".length"),
				Step::Size => text.push_str(".size"),
				Step::Capacity => text.push_str(".capacity"),
				Step::Payload(_) | Step::Characters => {}
			}
		}
		text
	}

	/// Whether a place may be written: its root is `mutable` or a parameter.
	pub(crate) fn place_writable(&self, place: PlaceId) -> Writable {
		let Some(symbol) = self.place_symbol(place) else {
			return Writable::Yes;
		};
		let entry = self.a.symbols.symbol(symbol);
		if self.readonly_place(place) {
			return Writable::No(symbol);
		}
		if self.s.types.is_recovering(entry.ty) {
			return Writable::Yes;
		}
		match entry.kind {
			SymbolKind::Mutable
			| SymbolKind::Reactive
			| SymbolKind::Parameter { .. }
			| SymbolKind::SelfParameter
			| SymbolKind::Requirement { .. }
			| SymbolKind::Use
			| SymbolKind::MutableUse
			| SymbolKind::LoopVariable { copy: false } => Writable::Yes,
			_ => Writable::No(symbol),
		}
	}

	pub(super) fn readonly_place(&self, mut place: PlaceId) -> bool {
		// Inspect the complete projection before following a formal's owner. A
		// dependent object parameter may itself contain a set field.
		for _ in 0..=self.reference_parameters.len() {
			while let Some(parent) = self.places.parent(place) {
				if matches!(
					self.places.get(place).step,
					Some(Step::Index(_) | Step::All)
				) && self.place_type(parent).is_some_and(|ty| {
					matches!(
						self.s.types.kind(ty),
						TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, ..)
					)
				}) {
					return true;
				}
				place = parent;
			}
			let Some(owner) = self
				.place_symbol(place)
				.and_then(|symbol| self.reference_parameters.get(&symbol))
			else {
				return false;
			};
			place = *owner;
		}
		true
	}

	// ── owners ────────────────────────────────────────────────────────────

	/// Who a symbol belongs to, seen from the innermost function.
	fn owner_of(&self, symbol: SymbolId) -> Owner {
		let Some(top) = self.body.functions.last() else {
			return Owner::Outside;
		};
		let entry = self.a.symbols.symbol(symbol);
		let is_param = matches!(
			entry.kind,
			SymbolKind::Parameter { .. }
				| SymbolKind::SelfParameter
				| SymbolKind::Requirement { .. }
		);
		let mut current = Some(entry.scope);
		while let Some(scope) = current {
			if scope == top.scope {
				return if is_param { Owner::Param } else { Owner::Local };
			}
			let data = self.a.symbols.scope(scope);
			// a local of an enclosing function (its body may be walked alone: A11)
			if data.kind == ScopeKind::Function
				|| self.body.functions.iter().any(|cx| cx.scope == scope)
			{
				return Owner::Captured;
			}
			// a value of another file (its proxy) is a module-level value too:
			// its writes are effects, declared and seen by the callers
			if matches!(
				data.kind,
				ScopeKind::Module | ScopeKind::Namespace | ScopeKind::Imports
			) {
				return if matches!(
					entry.kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
				) {
					Owner::Module
				} else {
					Owner::Outside
				};
			}
			current = data.parent;
		}
		Owner::Outside
	}

	/// Whether the innermost function is a lambda or a local function: its
	/// effects on captured places belong to the code around it.
	fn in_closure(&self) -> bool {
		let depth = self.body.functions.len();
		depth > 1
			|| self
				.body
				.functions
				.last()
				.is_some_and(|cx| cx.symbol.is_none() && !cx.test)
	}

	// ── recording effects ─────────────────────────────────────────────────

	/// A write of `place` (an assignment, a compound assignment, `.=`).
	pub(crate) fn write_effect(&mut self, target: NodeId, n: NodeId) {
		if !self.body.flow.reachable {
			return;
		}
		let evaluated = self.evaluated_origins(target);
		for bound in evaluated.bounds {
			if bound.formal.is_none() {
				self.record_bounded_effect(
					bound.place,
					Some(self.type_of(target)),
					target,
					n,
					false,
				);
			}
		}
		for place in self.places_of(target) {
			self.record_effect(place, false, n, false);
			// `map[key] = value` may add the key: `mutates self[key], self.size`
			if let View::Index { target: map, .. } = self.view(self.strip_groups(target))
				&& self.map_key(self.type_of(self.strip_groups(map))).is_some()
				&& let Some(parent) = self.places.parent(place)
			{
				let size = self.places.child(parent, Step::Size);
				self.record_effect(size, false, n, true);
			}
		}
	}

	fn protect_reference_argument(&mut self, argument: NodeId) -> Vec<SymbolId> {
		let protected = self
			.root_symbol(argument)
			.filter(|&symbol| self.body.flow.ref_state(symbol).is_some())
			.map(|symbol| self.body.flow.reference_equivalents(symbol))
			.unwrap_or_default();
		std::mem::replace(&mut self.effect_protected_references, protected)
	}

	fn record_bounded_effect(
		&mut self,
		owner: PlaceId,
		ty: Option<TypeId>,
		argument: NodeId,
		n: NodeId,
		call: bool,
	) {
		let protected = self.protect_reference_argument(argument);
		let effect_type = std::mem::replace(&mut self.effect_type, ty);
		self.record_effect(owner, false, n, call);
		self.effect_type = effect_type;
		self.effect_protected_references = protected;
	}

	/// An effect on `place` at node `n` (advanced brief §5.7 steps 6–8):
	/// facts, iteration rule, references, accumulation and coverage.
	pub(crate) fn record_effect(
		&mut self,
		place: PlaceId,
		invalidates: bool,
		n: NodeId,
		call: bool,
	) {
		if !self.body.flow.reachable {
			return;
		}
		// in a parallel body: only the iteration's own places (concurrency.rs)
		self.check_parallel_write(place, n, call);
		// a place a pending task uses, or one a started task changes (tasks.rs)
		self.task_write(place, n);
		let symbol = self.place_symbol(place);
		let owner = symbol.map_or(Owner::Outside, |symbol| self.owner_of(symbol));
		// a write of a whole local: its new facts are the assignment's (numbers.rs)
		let whole = self.places.parent(place).is_none();
		if call || !whole {
			self.drop_place_facts(place);
		}
		// a closure's effect on a captured place is checked where it runs
		let deferred =
			matches!(owner, Owner::Captured) || (owner == Owner::Module && self.in_closure());
		if !deferred {
			self.check_iteration(place, invalidates, n, call);
			self.invalidate_references(place, invalidates, n);
		}
		self.advance_reference_epochs(place, invalidates, n);
		if matches!(owner, Owner::Local | Owner::Outside) {
			return;
		}
		let entry = EffectEntry {
			place,
			invalidates,
			node: n,
			call,
		};
		let Some(top) = self.body.functions.last_mut() else {
			return;
		};
		if !top.effects.contains(&entry) {
			top.effects.push(entry);
		}
		if matches!(owner, Owner::Param | Owner::Module) {
			self.check_coverage(entry);
		}
	}

	/// An implied effect of a relation write or delete, `mutates C[..].r`
	/// (advanced brief §8.6, `relations.rs`): the facts go and the iteration
	/// rule applies, but relation sides are flat (no local reference points
	/// into them) and the effect is covered by the declared one it comes
	/// from, so it invalidates nothing and isn't accumulated.
	pub(crate) fn record_implied_effect(&mut self, place: PlaceId, n: NodeId, cause: &str) {
		if !self.body.flow.reachable {
			return;
		}
		self.check_parallel_write(place, n, true);
		self.drop_place_facts(place);
		let owner = self
			.place_symbol(place)
			.map_or(Owner::Outside, |symbol| self.owner_of(symbol));
		if matches!(owner, Owner::Captured) || (owner == Owner::Module && self.in_closure()) {
			return;
		}
		let iterating = self.body.flow.iterating.clone();
		for collection in iterating {
			if !self.may_be_at_or_below(collection, place) {
				continue;
			}
			let message = format!(
				"{cause} also changes `{}` (the other side of the relation), while the loop iterates over `{}`: collect the changes, and apply them after the loop",
				self.place_text(place),
				self.place_text(collection)
			);
			let diagnostic = self.diagnostic(&REF012, n, message);
			self.report_flow(diagnostic);
			return;
		}
	}

	/// The facts a change of `place` makes stale: at and below it, the
	/// length of the collection whose `.length`/`.size` it is, the
	/// relations `i < c.length` to a collection at or below it.
	pub(crate) fn drop_place_facts(&mut self, place: PlaceId) {
		let entry = self.places.get(place);
		let target = match (entry.step, entry.parent) {
			(Some(Step::Length | Step::Size), Some(parent)) => parent,
			_ => place,
		};
		// a payload changed in place: the place is still present, still the
		// same member (the narrowing contract, analysis.rs `NARROWED`)
		let narrowed = self
			.kept_narrowings
			.contains(&target)
			.then(|| self.body.flow.fact(target))
			.flatten()
			.map(|fact| self.facts.fact_value(fact))
			.filter(|fact| !fact.narrowed.is_unset());
		self.body.flow.mutate(&self.places, target);
		if let Some(narrowed) = narrowed {
			let fact = crate::flow::Fact {
				narrowed: narrowed.narrowed,
				narrowed_at: narrowed.narrowed_at,
				..crate::flow::Fact::TOP
			};
			let id = self.facts.fact(fact);
			self.body.flow.set_fact(target, id);
		}
		if self.growing_only {
			return;
		}
		let related: Vec<(PlaceId, crate::flow::Fact)> = self
			.body
			.flow
			.facts
			.iter()
			.map(|(place, fact)| (place, self.facts.fact_value(fact)))
			.filter(|(_, fact)| {
				fact.below
					.is_some_and(|(collection, _)| self.places.is_at_or_below(collection, target))
			})
			.collect();
		for (place, mut fact) in related {
			fact.below = None;
			let id = self.facts.fact(fact);
			self.body.flow.set_fact(place, id);
		}
	}

	fn advance_reference_epochs(&mut self, place: PlaceId, invalidates: bool, node: NodeId) {
		if self.body.flow.reference_epochs.is_empty() {
			return;
		}
		let entry = self.places.get(place);
		let changed = match entry.step {
			Some(Step::Length | Step::Size) => entry.parent,
			Some(Step::Capacity) => None,
			Some(Step::Index(_) | Step::Range(..)) if invalidates => entry.parent,
			_ if invalidates
				|| self
					.effect_type
					.or_else(|| self.place_type(place))
					.is_none_or(|ty| !self.is_flat(ty)) =>
			{
				Some(place)
			}
			_ => None,
		};
		let Some(changed) = changed else {
			return;
		};
		let changed = self.canonical_reference_place(changed);
		let epochs = self.body.flow.reference_epochs.clone();
		for (index, &(place, epoch)) in epochs.iter().enumerate() {
			let normalized = self.canonical_reference_place(place);
			if self.may_be_at_or_below(normalized, changed) {
				self.body.flow.reference_epochs[index].1 = if epoch == 0 {
					node.0.saturating_add(1)
				} else {
					u32::MAX
				};
			}
		}
	}

	/// The effect path of a place for the innermost function: its root is a
	/// parameter, `self` or a module-level value.
	pub(super) fn effect_path(&mut self, place: PlaceId) -> Option<PathId> {
		let symbol = self.place_symbol(place)?;
		let entry = self.a.symbols.symbol(symbol);
		let root = match entry.kind {
			SymbolKind::SelfParameter => PathRoot::SelfParam,
			SymbolKind::Parameter { index } => PathRoot::Param(index),
			SymbolKind::Requirement { index } => PathRoot::Requirement(index),
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive => {
				PathRoot::ModuleValue(self.module_key(symbol))
			}
			_ => return None,
		};
		let mut steps = Vec::new();
		for step in self.place_steps(place) {
			steps.push(match step {
				Step::Field(name) => PathStep::Field(name),
				Step::Length => PathStep::Length,
				Step::Size => PathStep::Size,
				Step::Capacity => PathStep::Capacity,
				Step::Index(Identity::Exact(value) | Identity::FromEnd(value)) => {
					PathStep::IndexConst(self.s.types.constant(ConstValue::Int(I256::from(value))))
				}
				Step::Index(Identity::Symbol(index, 0) | Identity::NormalizedSymbol(index, 0)) => {
					match self.a.symbols.symbol(index).kind {
						SymbolKind::Parameter { index } => PathStep::IndexParam(index),
						_ => PathStep::All,
					}
				}
				Step::Index(Identity::Key(key))
					if !matches!(self.s.types.const_value(key), ConstValue::Binding(_)) =>
				{
					PathStep::IndexConst(key)
				}
				Step::Payload(_) | Step::Characters => continue,
				Step::Index(_) | Step::Range(..) | Step::All | Step::Child => PathStep::All,
			});
		}
		Some(self.s.types.path(root, &steps))
	}

	/// The identity of a module-level value in effect paths.
	pub(crate) fn module_key(&mut self, symbol: SymbolId) -> crate::ids::DeclKey {
		// the export of another file: its own key (a function of this file
		// may change it, `mutates people`)
		let key = match self.modules.proxies.contains_key(&symbol)
			|| self.modules.aliases.contains_key(&symbol)
		{
			true => self
				.external_value_key(symbol)
				.unwrap_or_else(|| self.symbol_key(symbol)),
			false => self.symbol_key(symbol),
		};
		self.module_values.insert(key, symbol);
		key
	}

	/// The name of a parameter of the innermost function, for messages.
	fn effect_param_name(&self, index: u16) -> Option<String> {
		// the parameters' scope: the function's, or the current one while
		// its signature is being read
		let scope = self
			.body
			.functions
			.last()
			.map_or(self.body.scope, |cx| cx.scope);
		let mut current = Some(scope);
		while let Some(scope) = current {
			let data = self.a.symbols.scope(scope);
			for &symbol in &data.symbols {
				let entry = self.a.symbols.symbol(symbol);
				let found = match entry.kind {
					SymbolKind::Parameter { index: i } => i == index,
					SymbolKind::Requirement { index: i } => {
						i.checked_add(crate::effects::REQUIREMENT_NAMES) == Some(index)
					}
					_ => false,
				};
				if found {
					return Some(self.s.types.syms.resolve(entry.name).to_string());
				}
			}
			if data.kind == ScopeKind::Function {
				return None;
			}
			current = data.parent;
		}
		None
	}

	/// `EFF001`–`EFF004`: an effect of a function that must declare its
	/// effects and doesn't cover this one.
	fn check_coverage(&mut self, entry: EffectEntry) {
		let Some(declared) = self.body.functions.last().and_then(|cx| cx.coverage) else {
			return;
		};
		let Some(path) = self.effect_path(entry.place) else {
			return;
		};
		if set_covers(&self.s.types, declared, path, entry.invalidates) {
			return;
		}
		let canonical = self.canonical_reference_place(entry.place);
		if let Some(path) = self.effect_path(canonical)
			&& set_covers(&self.s.types, declared, path, entry.invalidates)
		{
			return;
		}
		let text = path_text(&self.s.types, path, &|index| self.effect_param_name(index));
		let root = {
			let (data, _) = self.s.types.path_data(path);
			data.root
		};
		let (mutates, invalidated) = self.s.types.effects(declared);
		let same_root: Vec<PathId> = mutates
			.iter()
			.chain(invalidated)
			.copied()
			.filter(|&declared| self.s.types.path_data(declared).0.root == root)
			.collect();
		let function = self.function_label();
		let root_text = {
			let base = self.s.types.path(root, &[]);
			path_text(&self.s.types, base, &|index| self.effect_param_name(index))
		};
		let (code, message) = if entry.invalidates {
			(
				&EFF002,
				format!(
					"{function} invalidates `{text}`, but its signature doesn't say so: add `invalidates {text}` (or `mutates {root_text}`)"
				),
			)
		} else if let Some(&first) = same_root.first() {
			let declared_text =
				path_text(&self.s.types, first, &|index| self.effect_param_name(index));
			(
				&EFF004,
				format!(
					"only `{declared_text}` is declared as mutated, but {function} also changes `{text}`: declare `mutates {text}`"
				),
			)
		} else if entry.call {
			(
				&EFF001,
				format!(
					"this call mutates `{text}`, but the signature of {function} doesn't say so: add `mutates {root_text}`"
				),
			)
		} else {
			(
				&EFF003,
				format!(
					"`{root_text}` is not declared as mutated: add `mutates {root_text}` to the signature of {function}"
				),
			)
		};
		let diagnostic = self.diagnostic(code, entry.node, message);
		self.report_flow(diagnostic);
		if let Some(cx) = self.body.functions.last_mut() {
			cx.uncovered = true;
		}
	}

	/// "`name`" or "this function", for messages.
	fn function_label(&self) -> String {
		self.body
			.functions
			.last()
			.and_then(|cx| cx.symbol)
			.map_or_else(
				|| "this function".to_string(),
				|symbol| {
					let name = self.a.symbols.symbol(symbol).name;
					format!("`{}`", self.s.types.syms.resolve(name))
				},
			)
	}

	// ── the iteration rule ────────────────────────────────────────────────

	/// `REF012` (advanced brief §5.8): an effect on a collection being
	/// iterated, or above it, or on its length; writing its elements is fine.
	fn check_iteration(&mut self, place: PlaceId, invalidates: bool, n: NodeId, call: bool) {
		if self.body.flow.iterating.is_empty() {
			return;
		}
		let iterating = self.body.flow.iterating.clone();
		let entry = self.places.get(place);
		let measured = match (entry.step, entry.parent) {
			(Some(Step::Length | Step::Size | Step::Capacity), Some(parent)) => Some(parent),
			_ => None,
		};
		for collection in iterating {
			let breaks = if let Some(parent) = measured {
				self.may_be_at_or_below(collection, parent)
					&& self.may_be_at_or_below(parent, collection)
			} else if invalidates {
				// an element of the collection, or the collection or above
				self.may_overlap(place, collection)
			} else {
				self.may_be_at_or_below(collection, place)
			};
			if !breaks {
				continue;
			}
			let what = self.place_text(collection);
			let message = if call {
				format!(
					"this call changes `{}` while the loop iterates over `{what}`: collect the changes, and apply them after the loop",
					self.place_text(place)
				)
			} else {
				format!(
					"`{what}` can't be replaced while the loop iterates over it: change its elements, or apply the change after the loop"
				)
			};
			let diagnostic = self.diagnostic(&REF012, n, message);
			self.report_flow(diagnostic);
			return;
		}
	}

	// ── distinctness (advanced brief §5.4) ───────────────────────────────

	/// May `r` designate `p` or a place below it?
	pub(crate) fn may_be_at_or_below(&self, r: PlaceId, p: PlaceId) -> bool {
		if self.places.get(r).root != self.places.get(p).root {
			return false;
		}
		let (r_steps, p_steps) = (self.place_steps(r), self.place_steps(p));
		if r_steps.len() < p_steps.len() {
			return false;
		}
		!r_steps
			.iter()
			.zip(&p_steps)
			.any(|(a, b)| self.steps_distinct(*a, *b))
	}

	/// May two places overlap (one at or below the other)?
	pub(crate) fn may_overlap(&self, a: PlaceId, b: PlaceId) -> bool {
		if self.places.get(a).root != self.places.get(b).root {
			return false;
		}
		let (a_steps, b_steps) = (self.place_steps(a), self.place_steps(b));
		!a_steps
			.iter()
			.zip(&b_steps)
			.any(|(x, y)| self.steps_distinct(*x, *y))
	}

	/// Whether two steps from the same place are proven to lead to different places.
	pub(super) fn steps_distinct(&self, a: Step, b: Step) -> bool {
		match (a, b) {
			(Step::Field(x), Step::Field(y)) => x != y,
			(Step::Field(_), _) | (_, Step::Field(_)) => true,
			(Step::Capacity, Step::Capacity) => false,
			(Step::Capacity, _) | (_, Step::Capacity) => true,
			(Step::Length | Step::Size, Step::Length | Step::Size) => false,
			(Step::Length | Step::Size, _) | (_, Step::Length | Step::Size) => true,
			(Step::Index(x), Step::Index(y)) => self.identities_distinct(x, y),
			(Step::Index(x), Step::Range(start, end, inclusive))
			| (Step::Range(start, end, inclusive), Step::Index(x)) => {
				self.outside_range(x, start, end, inclusive)
			}
			_ => false,
		}
	}

	/// Whether two index identities are proven different.
	fn identities_distinct(&self, a: Identity, b: Identity) -> bool {
		match (a, b) {
			(Identity::Symbol(x, i), Identity::Symbol(y, j)) if x == y => i != j,
			(Identity::LengthOf(x, i), Identity::LengthOf(y, j)) if x == y => i != j,
			(Identity::LengthSnapshot(x, i, a), Identity::LengthSnapshot(y, j, b))
				if x == y && a == b && a != u32::MAX =>
			{
				i != j
			}
			(Identity::LengthSnapshot(x, i, epoch), Identity::LengthOf(y, j))
			| (Identity::LengthOf(y, j), Identity::LengthSnapshot(x, i, epoch))
				if x == y && epoch != u32::MAX && epoch == self.body.flow.reference_epoch(x) =>
			{
				i != j
			}
			(Identity::Fresh(x), Identity::Fresh(y)) => x != y,
			(Identity::Key(x), Identity::Key(y)) => x != y,
			(Identity::Existing(before), Identity::Fresh(after))
			| (Identity::Fresh(after), Identity::Existing(before)) => after > before,
			_ => match (self.identity_interval(a), self.identity_interval(b)) {
				(Some(x), Some(y)) => x.is_disjoint(y),
				_ => false,
			},
		}
	}

	/// Whether an index is proven out of `[start..end)` (or `..=`).
	fn outside_range(
		&self,
		index: Identity,
		start: Identity,
		end: Identity,
		inclusive: bool,
	) -> bool {
		if let (Identity::Symbol(x, a), Identity::Symbol(y, b)) = (index, start)
			&& x == y && a < b
		{
			return true;
		}
		if let (Identity::Symbol(x, a), Identity::Symbol(y, b)) = (index, end)
			&& x == y && (a > b || (a == b && !inclusive))
		{
			return true;
		}
		let Some(value) = self.identity_interval(index) else {
			return false;
		};
		let below = self
			.identity_interval(start)
			.is_some_and(|start| value.hi < start.lo);
		let above = self.identity_interval(end).is_some_and(|end| {
			if inclusive {
				value.lo > end.hi
			} else {
				value.lo >= end.hi
			}
		});
		below || above
	}

	/// The values an identity may take here.
	fn identity_interval(&self, identity: Identity) -> Option<crate::types::Interval> {
		let offset = |interval: crate::types::Interval, k: i32| {
			crate::types::Interval::new(interval.lo + I256::from(k), interval.hi + I256::from(k))
		};
		match identity {
			Identity::Exact(value) => Some(crate::types::Interval::exact(I256::from(value))),
			Identity::Interval(id) => Some(self.facts.interval_value(id)),
			Identity::Symbol(symbol, k) => {
				let fact = self
					.places
					.symbol_place(symbol)
					.and_then(|place| self.body.flow.fact(place))
					.or_else(|| self.visible_binding(symbol));
				let int = fact.map_or(crate::ids::IntervalSetId::NONE, |fact| {
					self.facts.fact_value(fact).int
				});
				if int == crate::ids::IntervalSetId::NONE {
					let ty = self.a.symbols.symbol(symbol).ty;
					return self.s.types.interval_of(ty).map(|range| offset(range, k));
				}
				Some(offset(self.facts.interval_set_value(int).hull, k))
			}
			Identity::LengthSnapshot(place, offset, epoch) => {
				if epoch != u32::MAX && epoch == self.body.flow.reference_epoch(place) {
					self.identity_interval(Identity::LengthOf(place, offset))
				} else {
					None
				}
			}
			Identity::LengthOf(place, k) => {
				let fact = self.body.flow.fact(place).or_else(|| {
					self.place_symbol(place)
						.and_then(|symbol| self.visible_binding(symbol))
				})?;
				let length = self.facts.fact_value(fact).length;
				(length != crate::ids::IntervalId::NONE)
					.then(|| offset(self.facts.interval_value(length), k))
			}
			Identity::Fresh(_)
			| Identity::Existing(_)
			| Identity::Key(_)
			| Identity::Unknown
			| Identity::FromEnd(_)
			| Identity::NormalizedSymbol(..) => None,
		}
	}

	// ── signatures ────────────────────────────────────────────────────────

	/// The `mutates` and `invalidates` paths written in a signature
	/// (advanced brief §3.3): `EFF008` for an unknown root, field or index,
	/// `EFF005` for a whole primitive parameter, `EFF009` for `invalidates`
	/// of a whole value. Type part: once per function.
	pub(crate) fn declared_effects(
		&mut self,
		lambda: NodeId,
		mutates: NodeList<'_>,
		invalidates: NodeList<'_>,
	) -> EffectSetId {
		if let Some(&set) = self.declared_effect_sets.get(&lambda) {
			return set;
		}
		let mut mutated = Vec::new();
		let mut invalidated = Vec::new();
		for (paths, kind) in [(mutates, false), (invalidates, true)] {
			for node in paths.iter() {
				let Some((path, ty)) = self.signature_path(node) else {
					continue;
				};
				let reference_parameter = self.declared_reference_parameter(path);
				let (data, steps) = self.s.types.path_data(path);
				if reference_parameter.is_none()
					&& steps.is_empty()
					&& !kind && !matches!(data.root, PathRoot::ModuleValue(_))
					&& self.is_primitive(ty)
				{
					let text =
						path_text(&self.s.types, path, &|index| self.effect_param_name(index));
					let diagnostic = self.diagnostic(
						&EFF005,
						node,
						format!(
							"`{text}` is a `{}`: a primitive value can't be mutated, return the new value instead",
							self.display(ty)
						),
					);
					self.report(diagnostic);
					if let View::Name(_) = self.view(self.strip_groups(node))
						&& let Resolution::Local(symbol) =
							self.a.resolution(self.strip_groups(node))
					{
						self.rejected_effect_roots.insert(symbol);
					}
					continue;
				}
				let names_element = steps.iter().any(|step| {
					matches!(
						step,
						PathStep::All
							| PathStep::IndexConst(_)
							| PathStep::IndexParam(_)
							| PathStep::IndexFrom(_)
							| PathStep::Last
					)
				});
				if kind
					&& !names_element
					&& reference_parameter.is_none()
					&& !matches!(self.s.types.kind(ty), TypeKind::Ref(..))
					&& !self.s.types.is_recovering(ty)
				{
					let text =
						path_text(&self.s.types, path, &|index| self.effect_param_name(index));
					let base = self.s.types.path(data.root, &[]);
					let root =
						path_text(&self.s.types, base, &|index| self.effect_param_name(index));
					let diagnostic = self.diagnostic(
						&EFF009,
						node,
						format!(
							"`invalidates` names elements (`{text}[index]`, `{text}[..]`): a change of `{text}` itself is `mutates {root}`"
						),
					);
					self.report(diagnostic);
					continue;
				}
				if kind {
					if let Some(symbol) = reference_parameter
						&& let View::Assignment {
							annotation: Some(annotation),
							..
						} = self.view(self.a.symbols.symbol(symbol).decl_node)
						&& let Some((owner, _)) = self
							.reference_annotation(annotation)
							.and_then(|owner| self.signature_path(owner))
					{
						mutated.push(self.extend_path(owner, PathStep::Size));
					}
					invalidated.push(path);
				} else {
					mutated.push(path);
				}
			}
		}
		let set = normalized_set(&mut self.s.types, &mutated, &invalidated);
		self.declared_effect_sets.insert(lambda, set);
		set
	}

	/// A path of a signature and the declared type it designates.
	pub(super) fn signature_path(&mut self, node: NodeId) -> Option<(PathId, TypeId)> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, node) {
			View::Group(inner) => self.signature_path(inner),
			View::Name(name) => {
				let text = tree.name(name).to_string();
				let sym = self.s.types.syms.intern(&text);
				let found = self.a.symbols.lookup(self.body.scope, sym);
				let root = found.and_then(|symbol| {
					let entry = self.a.symbols.symbol(symbol);
					let ty = entry.ty;
					match entry.kind {
						SymbolKind::SelfParameter => Some((PathRoot::SelfParam, ty)),
						SymbolKind::Parameter { index } => Some((PathRoot::Param(index), ty)),
						SymbolKind::Requirement { index } => {
							Some((PathRoot::Requirement(index), ty))
						}
						SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
							if matches!(
								self.a.symbols.scope(entry.scope).kind,
								ScopeKind::Module | ScopeKind::Namespace
							) =>
						{
							Some((PathRoot::ModuleValue(self.module_key(symbol)), ty))
						}
						// the export of another file (`export mutable`)
						SymbolKind::Let | SymbolKind::Mutable
							if self.a.symbols.scope(entry.scope).kind == ScopeKind::Imports =>
						{
							let ty = self.proxy_type(symbol, node).unwrap_or(TypeId::UNKNOWN);
							Some((PathRoot::ModuleValue(self.module_key(symbol)), ty))
						}
						_ => None,
					}
				});
				let Some((root, ty)) = root else {
					let diagnostic = self.diagnostic(
						&EFF008,
						node,
						format!(
							"`{text}` is not a parameter, `self` or a module-level value: an effect path starts at one of them"
						),
					);
					self.report(diagnostic);
					return None;
				};
				if let Some(symbol) = found {
					self.set_resolution(node, Resolution::Local(symbol));
					self.set_type(node, ty);
				}
				Some((self.s.types.path(root, &[]), ty))
			}
			View::Access { target, members } => {
				let all: Vec<NodeId> = members.iter().collect();
				// a value of another file reached through namespaces
				// (`mutates library.people`): rooted at its key
				let (mut path, mut ty, rest) = match self.namespace_target(target) {
					Some(_) => {
						self.check_module_access(node, target, members);
						let found = all
							.iter()
							.enumerate()
							.find_map(|(index, &member)| match self.a.resolution(member) {
								Resolution::Local(symbol) if !self.is_namespace(symbol) => {
									Some((index, symbol))
								}
								_ => None,
							});
						let Some((index, symbol)) = found.filter(|&(_, symbol)| {
							matches!(
								self.a.symbols.symbol(symbol).kind,
								SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
							)
						}) else {
							let text = tree.text(node.index()).to_string();
							let diagnostic = self.diagnostic(
								&EFF008,
								node,
								format!(
									"`{text}` is not a parameter, `self` or a module-level value: an effect path starts at one of them"
								),
							);
							self.report(diagnostic);
							return None;
						};
						let ty = self.proxy_type(symbol, node).unwrap_or(TypeId::UNKNOWN);
						let root = PathRoot::ModuleValue(self.module_key(symbol));
						(self.s.types.path(root, &[]), ty, all[index + 1..].to_vec())
					}
					None => {
						let (path, ty) = self.signature_path(target)?;
						(path, ty, all.clone())
					}
				};
				for member in rest {
					let View::Name(name) = syntax::view(&tree, member) else {
						return None;
					};
					let text = tree.name(name).to_string();
					let (step, next) = match (text.as_str(), self.s.types.kind(ty)) {
						("length", TypeKind::String) => (PathStep::Length, TypeId::INTEGER),
						("length" | "size", TypeKind::Collection(kind, ..)) => {
							// `length` on the collections with positions, `size`
							// on the others (a set, a map, a pool, a columnar);
							// the other one is reported, and read as the right one
							let positioned = matches!(
								kind,
								CollKind::Array
									| CollKind::GrowingArray | CollKind::OrderedColumnar
							);
							let (has, step) = if positioned {
								("length", PathStep::Length)
							} else {
								("size", PathStep::Size)
							};
							if text != has {
								let owner = path_text(&self.s.types, path, &|index| {
									self.effect_param_name(index)
								});
								let diagnostic = self.diagnostic(
									&EFF008,
									member,
									format!(
										"a `{}` has a `{has}`, not a `{text}`: write `{owner}.{has}`",
										kind.name()
									),
								);
								self.report(diagnostic);
							}
							(step, TypeId::INTEGER)
						}
						("capacity", TypeKind::Collection(..) | TypeKind::String) => {
							(PathStep::Capacity, TypeId::INTEGER)
						}
						// a type not known here (already reported): the step as written
						("length", _) if self.s.types.is_recovering(ty) => {
							(PathStep::Length, TypeId::INTEGER)
						}
						("size", _) if self.s.types.is_recovering(ty) => {
							(PathStep::Size, TypeId::INTEGER)
						}
						(_, _) if self.s.types.is_recovering(ty) => (
							PathStep::Field(self.s.types.syms.intern(&text)),
							TypeId::UNKNOWN,
						),
						_ => {
							let sym = self.s.types.syms.intern(&text);
							let field = self
								.s
								.types
								.fields_of(ty)
								.iter()
								.find(|field| field.name == sym)
								.map(|field| field.ty);
							match field {
								// a reverse side changes only through its direct side
								Some(_) if self.is_reverse_side(ty, &text) => {
									let diagnostic = self.diagnostic(
										&EFF014,
										member,
										format!(
											"`{text}` is a reverse relation side: it changes only through its direct side, whose write implies it. Declare the direct side instead"
										),
									);
									self.report(diagnostic);
									return None;
								}
								Some(field) => (PathStep::Field(sym), field),
								None => {
									if !self.s.types.is_recovering(ty) {
										let owner = path_text(&self.s.types, path, &|index| {
											self.effect_param_name(index)
										});
										let diagnostic = self.diagnostic(
											&EFF008,
											member,
											format!(
												"`{owner}` has no field `{text}` (a `{}`)",
												self.display(ty)
											),
										);
										self.report(diagnostic);
									}
									return None;
								}
							}
						}
					};
					path = self.extend_path(path, step);
					ty = next;
				}
				Some((path, ty))
			}
			View::Index { target, index } => {
				let (path, ty) = self.signature_path(target)?;
				let item = match self.s.types.kind(ty) {
					TypeKind::Collection(kind, item, key) => {
						if kind == CollKind::Map {
							let _ = key;
						}
						item
					}
					TypeKind::String => TypeId::CHARACTER,
					_ => {
						if !self.s.types.is_recovering(ty) {
							let diagnostic = self.diagnostic(
								&EFF008,
								node,
								format!(
									"a `{}` has no elements: `[…]` in an effect path names elements of a collection",
									self.display(ty)
								),
							);
							self.report(diagnostic);
						}
						return None;
					}
				};
				let step = match syntax::view(&tree, self.strip_groups(index)) {
					View::Integer(value) => {
						PathStep::IndexConst(self.s.types.constant(ConstValue::Int(value)))
					}
					View::Name(name) => {
						let sym = self.s.types.syms.intern(tree.name(name));
						match self
							.a
							.symbols
							.lookup(self.body.scope, sym)
							.map(|symbol| self.a.symbols.symbol(symbol).kind)
						{
							Some(SymbolKind::Parameter { index }) => PathStep::IndexParam(index),
							_ => PathStep::All,
						}
					}
					_ => PathStep::All,
				};
				Some((self.extend_path(path, step), item))
			}
			_ => {
				let diagnostic = self.diagnostic(
					&EFF008,
					node,
					"an effect path is a parameter, `self` or a module-level value, then fields and elements, ex: `mutates player.score`",
				);
				self.report(diagnostic);
				None
			}
		}
	}

	fn extend_path(&mut self, path: PathId, step: PathStep) -> PathId {
		let (data, steps) = self.s.types.path_data(path);
		let mut steps = steps.to_vec();
		steps.push(step);
		self.s.types.path(data.root, &steps)
	}

	/// A value that can't be mutated in place: numbers, booleans,
	/// characters, strings, unions of them, enumerations.
	fn is_primitive(&self, ty: TypeId) -> bool {
		match self.s.types.kind(ty) {
			TypeKind::Int(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigInt
			| TypeKind::BigIntLit(_)
			| TypeKind::Percentage
			| TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..)
			| TypeKind::None => true,
			TypeKind::Union(list) => self
				.s
				.types
				.list_items(list)
				.iter()
				.all(|&member| self.is_primitive(member)),
			_ => false,
		}
	}

	// ── the end of a function ─────────────────────────────────────────────

	/// The effects of a function whose body was just walked: the declared
	/// ones, else the inferred ones (its parameters' and module-level
	/// values'). `EFF011` for a declared path the body never uses. A
	/// lambda's effects on captured places become effects of the code around
	/// it; a local function's are kept for its calls.
	pub(crate) fn finish_effects(&mut self, cx: &FnCx, declaration: Option<NodeId>) -> EffectSetId {
		// the effects as paths of this function
		let saved = std::mem::take(&mut self.body.functions);
		self.body.functions.push(cx.clone());
		let mut own: Vec<(PathId, bool)> = Vec::new();
		let mut coverage_own = Vec::new();
		let mut captured: Vec<EffectEntry> = Vec::new();
		for entry in &cx.effects {
			let canonical = self.canonical_reference_place(entry.place);
			if let Some(path) = self.effect_path(canonical) {
				coverage_own.push((path, entry.invalidates));
			}
			let Some(symbol) = self.place_symbol(entry.place) else {
				continue;
			};
			match self.owner_of(symbol) {
				Owner::Param => {
					if let Some(path) = self.effect_path(entry.place) {
						own.push((path, entry.invalidates));
					}
				}
				Owner::Module if cx.symbol.is_some() => {
					if let Some(path) = self.effect_path(entry.place) {
						own.push((path, entry.invalidates));
					}
				}
				Owner::Module | Owner::Captured => captured.push(*entry),
				Owner::Local | Owner::Outside => {}
			}
		}
		let set = match cx.declared {
			Some(declared) => {
				// EFF011: a declared path nothing in the body changes
				if cx.coverage.is_some() && !cx.uncovered && self.body_complete(cx) {
					let unused: Vec<(PathId, bool)> = std::iter::once(declared)
						.chain(cx.extra_effect_contract)
						.flat_map(|contract| {
							let (mutates, invalidated) = self.s.types.effects(contract);
							mutates
								.iter()
								.map(|&path| (path, false))
								.chain(invalidated.iter().map(|&path| (path, true)))
						})
						.filter(|&(path, invalidates)| {
							!covers_any(&self.s.types, path, invalidates, &own)
								&& !covers_any(&self.s.types, path, invalidates, &coverage_own)
						})
						.collect();
					for (path, invalidates) in unused {
						let text =
							path_text(&self.s.types, path, &|index| self.effect_param_name(index));
						let keyword = if invalidates {
							"invalidates"
						} else {
							"mutates"
						};
						let at = cx.signature_node.unwrap_or(cx.node);
						if !self.reported_once.insert((at, "EFF011")) {
							continue;
						}
						let diagnostic = self.diagnostic(
							&EFF011,
							at,
							format!(
								"`{keyword} {text}` is declared, but {} never changes it: remove it",
								self.function_label()
							),
						);
						self.report(diagnostic);
					}
				}
				declared
			}
			None => {
				let mutates: Vec<PathId> = own.iter().filter(|e| !e.1).map(|e| e.0).collect();
				let invalidates: Vec<PathId> = own.iter().filter(|e| e.1).map(|e| e.0).collect();
				normalized_set(&mut self.s.types, &mutates, &invalidates)
			}
		};
		self.body.functions = saved;
		match declaration {
			// a local function: its captured effects happen at its calls
			Some(declaration) if !captured.is_empty() => {
				self.closure_effects.insert(declaration, captured);
			}
			Some(_) => {}
			// a lambda: they happen where it is created (it runs in the call it
			// is given to), replayed by `check_lambda` in the enclosing context
			None => {
				self.closure_effects.insert(cx.node, captured);
			}
		}
		set
	}

	/// Whether no unsupported construct was met in a body (`EFF011` needs all of it).
	fn body_complete(&self, cx: &FnCx) -> bool {
		cx.unsupported_at_start == self.unsupported_seen
	}

	// ── calls ─────────────────────────────────────────────────────────────

	/// The effects of a call (advanced brief §5.7): the callee's effects with
	/// its parameters replaced by the arguments' places (a temporary
	/// argument drops the effect), `EFF007` for an immutable argument of a
	/// mutated parameter, then [`Walker::record_effect`]. A local function's
	/// effects on the places it captures happen at its calls, and when it is
	/// given to a callee.
	pub(crate) fn apply_call_effects(
		&mut self,
		n: NodeId,
		callee: NodeId,
		sig_effects: EffectSetId,
	) {
		if !self.body.flow.reachable {
			return;
		}
		let Some(info) = self.a.calls.get(&n).cloned() else {
			return;
		};
		let receiver = match self.view(self.strip_groups(callee)) {
			View::Access { target, .. } => Some(target),
			_ => None,
		};
		let name = self.callee_name(callee);
		// the local functions it calls or is given: the references they
		// capture are read here
		let mut closures = self.called_closures(&info);
		closures.extend(
			self.evaluated_origins(callee)
				.closures
				.into_iter()
				.filter_map(|origin| origin.callable.local())
				.map(|lambda| {
					self.a
						.functions
						.get(&lambda)
						.and_then(|info| info.declaration)
						.unwrap_or(lambda)
				}),
		);
		closures.sort_unstable();
		closures.dedup();
		for &function in &closures {
			self.check_captured_references(n, function);
		}
		// the places the call changes, with the argument each one comes from
		let mut bound_protections: rustc_hash::FxHashMap<(PlaceId, NodeId), bool> =
			rustc_hash::FxHashMap::default();
		let mut changed: Vec<(PlaceId, bool, Option<NodeId>)> = Vec::new();
		let mut builtin_effects: Vec<BuiltinEffect> = Vec::new();
		let mut receiver_places = Vec::new();
		// the places changed in place, never replaced: they keep their narrowing
		let mut kept: Vec<PlaceId> = Vec::new();
		match info.callee {
			Callee::Builtin(builtin) => {
				let path = crate::builtins::get(builtin).path;
				// a growing collection shares the paths of its counterpart:
				// the receiver's kind decides (`GrowingArray.push` moves nothing)
				let receiver_kind =
					self.callee_receiver_type(callee)
						.and_then(|ty| match self.s.types.kind(ty) {
							crate::types::TypeKind::Collection(kind, ..) => Some(kind),
							_ => None,
						});
				builtin_effects = match receiver_kind {
					Some(kind) => {
						let method = path.rsplit('#').next().unwrap_or(path);
						crate::builtins::effects::effects_of_kind(kind, method)
					}
					None => crate::builtins::effects::effects_of(path),
				};
				if let Some(receiver) = receiver.filter(|_| !builtin_effects.is_empty()) {
					for base in self.callee_receiver_places(callee) {
						let method = path.rsplit('#').next().unwrap_or(path).to_string();
						// links and unlinks of relations (relations.rs)
						self.check_relation_call(n, path, callee, &info.arguments);
						// `pool.delete(element)`: an element of this very pool (REF020)
						if matches!(
							path,
							"Pool#delete" | "Pool#take" | "Columnar#delete" | "Columnar#take"
						) && let Some(Argument::Given(element)) = info.arguments.first()
						{
							self.check_element_of(callee, *element, &method);
						}
						if self.check_mutable_argument(base, receiver, &method, true) {
							receiver_places.push(base);
							changed.push((base, false, Some(receiver)));
						}
					}
				}
				// `stream.read(buffer)` mutates its argument (std_library.rs)
				if path == "TcpStream#read"
					&& let Some(Argument::Given(buffer)) = info.arguments.first()
				{
					for place in self.places_of(*buffer) {
						if self.check_mutable_argument(place, *buffer, "read", false) {
							changed.push((place, false, Some(*buffer)));
						}
					}
				}
				// a value stored into a collection escapes (closures)
				if crate::builtins::effects::stores_arguments(path) {
					// a collection that outlives the function (tasks.rs)
					let outside = self
						.callee_receiver_places(callee)
						.into_iter()
						.filter_map(|place| self.place_symbol(place))
						.any(|symbol| !self.is_function_local(symbol));
					for argument in &info.arguments {
						if let Argument::Given(value) = argument {
							if outside {
								self.task_stored(*value);
							}
							self.escape(*value, n);
							// an item stored is a copy, maybe a move (moves.rs)
							self.note_copy_site(*value);
						}
					}
				}
			}
			Callee::Function(_) | Callee::Value => {
				let (mutates, invalidates) = {
					let (m, i) = self.s.types.effects(sig_effects);
					(m.to_vec(), i.to_vec())
				};
				let paths: Vec<(PathId, bool)> = mutates
					.into_iter()
					.map(|path| (path, false))
					.chain(invalidates.into_iter().map(|path| (path, true)))
					.collect();
				// the declared type of each path, in the callee
				let callee_params: Vec<TypeId> = match info.callee {
					Callee::Function(declaration) => self
						.a
						.functions
						.get(&declaration)
						.map(|function| {
							function
								.params
								.iter()
								.map(|param| self.a.symbols.symbol(param.symbol).ty)
								.collect()
						})
						.unwrap_or_default(),
					_ => match self.s.types.kind(self.type_of(callee)) {
						TypeKind::Function(sig) => self
							.s
							.types
							.params(self.s.types.sig(sig).params)
							.iter()
							.map(|param| param.ty)
							.collect(),
						_ => Vec::new(),
					},
				};
				let mut path_types: Vec<Option<TypeId>> = Vec::new();
				for &(path, _) in &paths {
					path_types.push(self.path_type(path, &callee_params));
				}
				let mut refused: Vec<PlaceId> = Vec::new();
				let mut checked: Vec<PlaceId> = Vec::new();
				for (position, (path, invalidates)) in paths.into_iter().enumerate() {
					let places =
						self.substitute_effect_places(path, &info.arguments, receiver, (n, &name));
					if places.is_empty() {
						self.temporary_mutated(path, &info.arguments, &name);
					}
					for (place, base, argument) in places {
						if self
							.evaluated_origins(argument)
							.bounds
							.iter()
							.any(|bound| bound.formal.is_none() && bound.place == place)
						{
							let keeps_root =
								!invalidates || !self.s.types.path_data(path).1.is_empty();
							bound_protections
								.entry((place, argument))
								.and_modify(|old| *old &= keeps_root)
								.or_insert(keeps_root);
						}
						// a parameter of the payload's type changes the payload only:
						// a parameter is never replaced (`EFF006`); one typed
						// `Array?` or a union may stand for the optional itself
						if matches!(
							self.s.types.path_data(path).0.root,
							PathRoot::Param(_) | PathRoot::SelfParam
						) && path_types[position].is_some_and(|ty| !self.is_union_like(ty))
						{
							kept.push(base);
						}
						if let Some(ty) = path_types[position] {
							self.effect_types.insert(place, ty);
						}
						if refused.contains(&base) {
							continue;
						}
						if !checked.contains(&base) {
							checked.push(base);
							let method = receiver.is_some_and(|receiver| {
								self.strip_groups(receiver) == self.strip_groups(argument)
							});
							// a C function's immutable argument is `FFI010` (foreign.rs)
							let foreign = self.a.extern_calls.contains_key(&n);
							if foreign && self.place_writable(base) != Writable::Yes
								|| !foreign
									&& !self.check_mutable_argument(base, argument, &name, method)
							{
								refused.push(base);
								continue;
							}
						}
						// a `readonly` field the callee writes (structural.rs)
						if let Some(declared) = path_types[position] {
							self.check_readonly_written(argument, path, declared, &name);
						}
						// a collection that owner fields point into (relations.rs)
						let (data, steps) = self.s.types.path_data(path);
						if matches!(data.root, PathRoot::Param(_))
							&& !steps.iter().any(|step| {
								matches!(step, PathStep::Field(_) | PathStep::Private(_))
							}) {
							self.check_owned_collection_alias(argument);
						}
						changed.push((place, invalidates, Some(argument)));
					}
				}
			}
			_ => {}
		}
		// the alias rule (advanced brief §5.9): a changed place can't overlap
		// another argument
		if !changed.is_empty() {
			// `pool.delete(element)` designates an element of the pool: that's its job
			let element_argument = matches!(
				info.callee,
				Callee::Builtin(builtin) if matches!(
					crate::builtins::get(builtin).path,
					"Pool#delete" | "Pool#take" | "Columnar#delete" | "Columnar#take"
				)
			);
			let arguments: Vec<Argument> = if element_argument {
				info.arguments.iter().skip(1).cloned().collect()
			} else {
				info.arguments.clone()
			};
			self.check_aliasing(n, &name, &changed, &arguments, receiver);
			if let Callee::Function(declaration) = info.callee {
				self.check_module_read_aliasing(n, &name, declaration, &changed, &arguments);
			} else if info.callee == Callee::Value {
				self.check_external_read_aliasing(n, callee, &name, &changed, &arguments);
			}
		}
		// the relation sides the changes also write (relations.rs, §8.6, D47)
		let implied = if receiver_places.is_empty() {
			self.implied_by_changes(&changed)
		} else {
			Vec::new()
		};
		if !implied.is_empty() {
			let used: Vec<NodeId> = info
				.arguments
				.iter()
				.flat_map(|argument| match argument {
					Argument::Given(node) => vec![*node],
					Argument::Variadic(nodes) => nodes.clone(),
					Argument::Receiver(_) => receiver.into_iter().collect(),
					_ => Vec::new(),
				})
				.collect();
			self.check_implied_aliasing(n, &name, &implied, &used);
		}
		// the effects
		self.kept_narrowings = kept;
		if !receiver_places.is_empty() {
			let bounds = self.evaluated_origins(callee).bounds;
			for &effect in &builtin_effects {
				let receiver_ty = self.callee_receiver_type(callee);
				let ty = match effect {
					BuiltinEffect::MutatesSize
					| BuiltinEffect::MutatesLength
					| BuiltinEffect::MutatesCapacity => Some(TypeId::INTEGER),
					BuiltinEffect::MutatesKey(_) => {
						receiver_ty.and_then(|ty| match self.s.types.kind(ty) {
							TypeKind::Collection(_, item, _) => Some(item),
							_ => None,
						})
					}
					_ => receiver_ty,
				};
				for bound in &bounds {
					if bound.formal.is_none() {
						self.record_bounded_effect(bound.place, ty, callee, n, true);
					}
				}
			}
			for base in receiver_places {
				self.kept_narrowings.push(base);
				let length = self.place_length(base);
				// `push` and `insert` only add elements: `i < c.length` holds
				self.growing_only = matches!(
					info.callee,
					Callee::Builtin(builtin) if matches!(
						crate::builtins::get(builtin).path,
						"Array#push" | "Array#insert" | "OrderedColumnar#add"
					)
				);
				for &effect in &builtin_effects {
					self.builtin_effect(effect, base, &info.arguments, n);
				}
				self.growing_only = false;
				if let (Callee::Builtin(builtin), Some(length)) = (info.callee, length) {
					let path = crate::builtins::get(builtin).path;
					self.track_length(base, path, length);
				}
			}
		} else {
			for (place, invalidates, argument) in changed {
				let previous = argument
					.filter(|&argument| bound_protections.get(&(place, argument)) == Some(&true))
					.map(|argument| self.protect_reference_argument(argument));
				self.effect_type = self.effect_types.get(&place).copied();
				self.record_effect(place, invalidates, n, true);
				self.effect_type = None;
				if let Some(previous) = previous {
					self.effect_protected_references = previous;
				}
			}
			let cause = format!("`{name}`");
			for place in implied {
				self.record_implied_effect(place, n, &cause);
			}
		}
		self.kept_narrowings.clear();
		// a local function called or given: its effects on captured places
		for declaration in closures {
			if let Some(entries) = self.closure_effects.get(&declaration).cloned() {
				for entry in entries {
					self.record_effect(entry.place, entry.invalidates, n, true);
				}
			}
		}
	}

	/// The local functions a call runs or is given (by declaration).
	fn called_closures(&self, info: &crate::analysis::CallInfo) -> Vec<NodeId> {
		let mut closures = Vec::new();
		if let Callee::Function(declaration) = info.callee {
			closures.push(declaration);
		}
		for argument in &info.arguments {
			if let Argument::Given(value) = argument {
				if matches!(self.view(self.strip_groups(*value)), View::Lambda { .. }) {
					continue;
				}
				closures.extend(
					self.evaluated_origins(*value)
						.closures
						.into_iter()
						.filter_map(|origin| origin.callable.local())
						.map(|lambda| {
							self.a
								.functions
								.get(&lambda)
								.and_then(|info| info.declaration)
								.unwrap_or(lambda)
						}),
				);
			}
		}
		closures.sort_unstable();
		closures.dedup();
		closures
	}

	/// `REF013` (advanced brief §5.9): the call changes a place that another
	/// of its arguments may designate (`numbers.push(numbers[0])`,
	/// `swap(team, team)`). Reading a length or a size is never an overlap.
	fn check_aliasing(
		&mut self,
		n: NodeId,
		callee: &str,
		changed: &[(PlaceId, bool, Option<NodeId>)],
		arguments: &[Argument],
		receiver: Option<NodeId>,
	) {
		let mut used: Vec<NodeId> = Vec::new();
		for argument in arguments {
			match argument {
				Argument::Given(node) => used.push(*node),
				Argument::Variadic(nodes) => used.extend(nodes.iter().copied()),
				Argument::Receiver(_) => used.extend(receiver),
				_ => {}
			}
		}
		// a callback capturing a reference into a changed place (types brief
		// po-12: `ps.deleteWhere((person) => person.age < ana.age)`)
		for &argument in &used {
			let mut lambda = self.strip_groups(argument);
			// a local function given by name
			if let View::Name(_) = self.view(lambda)
				&& let Resolution::Local(symbol) = self.a.resolution(lambda)
				&& let Some(declaration) = self.direct_function(symbol)
				&& let Some(info) = self.a.functions.get(&declaration)
			{
				lambda = info.lambda;
			}
			if !matches!(self.view(lambda), View::Lambda { .. }) {
				continue;
			}
			// the references the function reads (top-level locals are not
			// captures, so the captures the walk recorded don't say it): a
			// scan of the function given, only when a call that changes
			// places is given a function literal, bounded by its size
			let mut captures: Vec<SymbolId> = Vec::new();
			let mut stack = vec![lambda];
			while let Some(node) = stack.pop() {
				if let Resolution::Local(symbol) = self.a.resolution(node)
					&& self.aliases.contains_key(&symbol)
					&& !captures.contains(&symbol)
				{
					captures.push(symbol);
				}
				stack.extend(syntax::children(&self.tree, node));
			}
			for symbol in captures {
				for captured in self.symbol_places(symbol) {
					for &(place, _, _) in changed {
						if !self.may_overlap(place, captured) {
							continue;
						}
						let changed_text = self.place_text(place);
						let name = self
							.s
							.types
							.syms
							.resolve(self.a.symbols.symbol(symbol).name)
							.to_string();
						let message = format!(
							"`{callee}` changes `{changed_text}`, and the function given to it reads `{name}`, which may designate an element of it: copy what the function needs first with `let`"
						);
						let diagnostic = self.diagnostic(
							&crate::diagnostics::codes::advanced::REF013,
							n,
							message,
						);
						self.report_flow(diagnostic);
						return;
					}
				}
			}
		}
		for &(place, _, from) in changed {
			for &argument in &used {
				if Some(self.strip_groups(argument)) == from.map(|from| self.strip_groups(from)) {
					continue;
				}
				for other in self.places_of(argument) {
					if matches!(
						self.places.get(other).step,
						Some(Step::Length | Step::Size | Step::Capacity)
					) || !self.may_overlap(place, other)
					{
						continue;
					}
					if self.declared_reference_overlap(n, from, argument) {
						continue;
					}
					let changed_text = self.place_text(place);
					let argument_text = self.tree.text(argument.index()).to_string();
					let message = if other != place && self.may_be_at_or_below(other, place) {
						format!(
							"`{callee}` changes `{changed_text}`, and its argument `{argument_text}` is inside it: copy it first with `let`"
						)
					} else {
						format!(
							"`{callee}` changes `{changed_text}`, which its argument `{argument_text}` may also designate: pass different values, or copy one first with `let`"
						)
					};
					let diagnostic =
						self.diagnostic(&crate::diagnostics::codes::advanced::REF013, n, message);
					self.report_flow(diagnostic);
					return;
				}
			}
		}
	}

	/// `REF013` for an argument that a callee of this file changes, while
	/// the callee also reads a module-level value it may designate
	/// (reference-checking.md, "Arguments that alias").
	fn check_module_read_aliasing(
		&mut self,
		n: NodeId,
		callee: &str,
		declaration: NodeId,
		changed: &[(PlaceId, bool, Option<NodeId>)],
		arguments: &[Argument],
	) {
		let Some(changed) = self.changed_arguments(changed, arguments) else {
			return;
		};
		let Some(body) = self.a.functions.get(&declaration).map(|info| info.body) else {
			return;
		};
		// the module-level `mutable` values its body names
		let mut read: Vec<SymbolId> = Vec::new();
		let mut stack = vec![body];
		while let Some(node) = stack.pop() {
			if let View::Name(_) = self.view(node)
				&& !self
					.a
					.has_flags(node, crate::analysis::NodeFlags::TYPE_EXPR)
				&& let Resolution::Local(symbol) = self.a.resolution(node)
				&& self.a.symbols.symbol(symbol).kind == SymbolKind::Mutable
				&& self.is_module_level(symbol)
				&& !read.contains(&symbol)
			{
				read.push(symbol);
			}
			stack.extend(syntax::children(&self.tree, node));
		}
		self.report_read_aliasing(n, callee, &changed, read);
	}

	/// The changes of a call to its parameters' arguments, not to its
	/// module values (`None`: none).
	fn changed_arguments(
		&self,
		changed: &[(PlaceId, bool, Option<NodeId>)],
		arguments: &[Argument],
	) -> Option<Vec<(PlaceId, bool, Option<NodeId>)>> {
		let given: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(node) => Some(*node),
				_ => None,
			})
			.collect();
		let changed: Vec<(PlaceId, bool, Option<NodeId>)> = changed
			.iter()
			.copied()
			.filter(|&(_, _, from)| from.is_some_and(|from| given.contains(&from)))
			.collect();
		(!changed.is_empty()).then_some(changed)
	}

	/// [`Walker::check_module_read_aliasing`] for a function of another
	/// file: the module-level `mutable` values its declaration uses, from
	/// its file's interface (review finding 11).
	fn check_external_read_aliasing(
		&mut self,
		n: NodeId,
		callee: NodeId,
		name: &str,
		changed: &[(PlaceId, bool, Option<NodeId>)],
		arguments: &[Argument],
	) {
		let Some(changed) = self.changed_arguments(changed, arguments) else {
			return;
		};
		let symbol = match self.view(self.strip_groups(callee)) {
			View::Name(_) => self.a.resolution(self.strip_groups(callee)),
			View::Access { members, .. } => members
				.last()
				.map_or(Resolution::Unresolved, |member| self.a.resolution(member)),
			_ => Resolution::Unresolved,
		};
		let Resolution::Local(symbol) = symbol else {
			return;
		};
		let Some((module, decl)) = self.proxy_decl(symbol) else {
			return;
		};
		let Some(uses) = self
			.s
			.interface(module)
			.and_then(|interface| interface.declared.get(&decl))
			.map(|declared| declared.uses.clone())
		else {
			return;
		};
		let mut read: Vec<SymbolId> = Vec::new();
		for key in uses {
			let owner = self.s.types.decl_key_data(key).module;
			let mutable = self.s.interface(owner).is_some_and(|interface| {
				interface
					.keys
					.get(&key)
					.and_then(|keyed| interface.declared.get(&keyed.node))
					.is_some_and(|declared| declared.kind == SymbolKind::Mutable)
			});
			if !mutable {
				continue;
			}
			let value = match self.module_values.get(&key) {
				Some(&symbol) => Some(symbol),
				None => self.module_value_proxy(key),
			};
			if let Some(value) = value
				&& !read.contains(&value)
			{
				read.push(value);
			}
		}
		self.report_read_aliasing(n, name, &changed, read);
	}

	/// `REF013` when a changed argument may overlap a module-level value
	/// the callee reads.
	fn report_read_aliasing(
		&mut self,
		n: NodeId,
		callee: &str,
		changed: &[(PlaceId, bool, Option<NodeId>)],
		read: Vec<SymbolId>,
	) {
		for symbol in read {
			let module_places = self.symbol_places(symbol);
			for &(place, _, _) in changed {
				if !module_places
					.iter()
					.any(|&module_place| self.may_overlap(place, module_place))
				{
					continue;
				}
				let name = self
					.s
					.types
					.syms
					.resolve(self.a.symbols.symbol(symbol).name)
					.to_string();
				let message = format!(
					"`{callee}` changes `{}`, and also reads `{name}`, which it may designate: pass a copy, or let `{callee}` use `{name}` directly",
					self.place_text(place)
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::advanced::REF013, n, message);
				self.report_flow(diagnostic);
				return;
			}
		}
	}

	/// Whether a symbol is declared outside every function of the file.
	fn is_module_level(&self, symbol: SymbolId) -> bool {
		let mut current = Some(self.a.symbols.symbol(symbol).scope);
		while let Some(scope) = current {
			let data = self.a.symbols.scope(scope);
			if data.kind == ScopeKind::Function {
				return false;
			}
			current = data.parent;
		}
		true
	}

	/// The local a place expression starts from: `hero` in `hero.power`.
	fn root_symbol(&self, n: NodeId) -> Option<SymbolId> {
		let mut n = self.strip_groups(n);
		loop {
			match self.view(n) {
				View::Name(_) => {
					return match self.a.resolution(n) {
						Resolution::Local(symbol) => Some(symbol),
						_ => None,
					};
				}
				View::Access { target, .. } | View::Index { target, .. } => {
					n = self.strip_groups(target);
				}
				_ => return None,
			}
		}
	}

	/// The name of a callee for messages: `push`, `addBonus`.
	fn callee_name(&self, callee: NodeId) -> String {
		let callee = self.strip_groups(callee);
		match self.view(callee) {
			View::Access { members, .. } => members
				.last()
				.map(|member| self.tree.text(member.index()).to_string())
				.unwrap_or_default(),
			_ => self.tree.text(callee.index()).to_string(),
		}
	}

	/// A callee's effect path with its root replaced by the argument's place:
	/// the place, the argument's place, and the argument node.
	pub(super) fn substitute_places(
		&mut self,
		path: PathId,
		arguments: &[Argument],
		receiver: Option<NodeId>,
		call: (NodeId, &str),
	) -> Vec<(PlaceId, PlaceId, NodeId)> {
		let (data, steps) = self.s.types.path_data(path);
		let steps = steps.to_vec();
		let tree = Arc::clone(&self.tree);
		let argument_node = |index: usize| -> Option<NodeId> {
			match arguments.get(index)? {
				Argument::Given(node) => Some(*node),
				Argument::Receiver(callee) => match syntax::view(&tree, *callee) {
					View::Access { target, .. } => Some(target),
					_ => None,
				},
				_ => None,
			}
		};
		let (bases, node) = match data.root {
			PathRoot::Param(index) => {
				let Some(node) = argument_node(index as usize) else {
					return Vec::new();
				};
				let place = match arguments.get(index as usize) {
					Some(Argument::Receiver(callee)) => self.callee_receiver_places(*callee),
					_ => self.evaluated_origins(node).places,
				};
				(place, node)
			}
			PathRoot::SelfParam => {
				let Some(node) = argument_node(0).or(receiver) else {
					return Vec::new();
				};
				// `a.b.method()`: the receiver is `a.b`, not `a`
				let place = match arguments.first() {
					Some(Argument::Receiver(callee)) => self.callee_receiver_places(*callee),
					_ => self.evaluated_origins(node).places,
				};
				(place, node)
			}
			PathRoot::ModuleValue(key) => {
				let symbol = match self.module_values.get(&key) {
					Some(&symbol) => symbol,
					// a value of another file this one doesn't name: a proxy of it
					None => match self.module_value_proxy(key) {
						Some(symbol) => symbol,
						None => return Vec::new(),
					},
				};
				(self.symbol_places(symbol), receiver.unwrap_or(NodeId(0)))
			}
			PathRoot::Requirement(index) => {
				let provider = self.call_providers.get(index as usize).copied().flatten();
				let places = self.mutated_requirement_places(call.0, call.1, provider);
				(places, call.0)
			}
		};
		bases
			.into_iter()
			.map(|base| {
				let mut place = base;
				for &step in &steps {
					let step = match step {
						PathStep::Field(name) | PathStep::Private(name) => Step::Field(name),
						PathStep::Length => Step::Length,
						PathStep::Size => Step::Size,
						PathStep::Capacity => Step::Capacity,
						PathStep::All => Step::Index(Identity::Unknown),
						PathStep::IndexConst(value) => match self.s.types.const_value(value) {
							ConstValue::Int(value) => Step::Index(
								i64::try_from(value).map_or(Identity::Unknown, Identity::Exact),
							),
							ConstValue::String(_) | ConstValue::Bool(_) => {
								Step::Index(Identity::Key(value))
							}
							_ => Step::Index(Identity::Unknown),
						},
						PathStep::IndexParam(index) => match argument_node(index as usize) {
							Some(node) => Step::Index(self.identity_of(node)),
							None => Step::Index(Identity::Unknown),
						},
						PathStep::IndexFrom(index) => {
							let start = argument_node(index as usize)
								.map_or(Identity::Unknown, |node| self.identity_of(node));
							Step::Range(start, Identity::Unknown, false)
						}
						PathStep::Last => Step::Index(Identity::LengthOf(place, -1)),
					};
					let step = match step {
						Step::Index(identity) => Step::Index(
							self.normalize_reference_index(identity, self.place_type(place)),
						),
						other => other,
					};
					place = self.places.child(place, step);
				}
				(place, base, node)
			})
			.collect()
	}

	/// Effects through an opaque result can reach any descendant of its
	/// owner. Formal dependent parameters retain their distinct narrow roots.
	fn substitute_effect_places(
		&mut self,
		path: PathId,
		arguments: &[Argument],
		receiver: Option<NodeId>,
		call: (NodeId, &str),
	) -> Vec<(PlaceId, PlaceId, NodeId)> {
		let mut places = self.substitute_places(path, arguments, receiver, call);
		for (_, base, argument) in places.clone() {
			for bound in self.evaluated_origins(argument).bounds {
				if bound.formal.is_none() {
					places.push((bound.place, base, argument));
				}
			}
		}
		places.sort_unstable();
		places.dedup();
		places
	}

	/// The places a call of a function of the program changes by its own
	/// effects (`mutates`, `invalidates`), with whether it invalidates them:
	/// a lambda argument may run after them, so its entry facts drop them
	/// (`functions.rs`). Arguments not checked yet have no place: skipped.
	pub(crate) fn callee_effect_places(
		&mut self,
		n: NodeId,
		callee: NodeId,
		effects: EffectSetId,
		arguments: &[Argument],
	) -> Vec<(PlaceId, bool, NodeId)> {
		let paths: Vec<(PathId, bool)> = {
			let (mutates, invalidates) = self.s.types.effects(effects);
			mutates
				.iter()
				.map(|&path| (path, false))
				.chain(invalidates.iter().map(|&path| (path, true)))
				.collect()
		};
		if paths.is_empty() {
			return Vec::new();
		}
		let receiver = match self.view(self.strip_groups(callee)) {
			View::Access { target, .. } => Some(target),
			_ => None,
		};
		let name = self.callee_name(callee);
		paths
			.into_iter()
			.flat_map(|(path, invalidates)| {
				self.substitute_effect_places(path, arguments, receiver, (n, &name))
					.into_iter()
					.map(move |(place, _, _)| (place, invalidates, n))
			})
			.collect()
	}

	/// `EF012`: an object literal given to a parameter the callee mutates:
	/// the mutation would be lost (defining-functions.md, "Mutating
	/// parameters"). An array literal stays allowed (`add([name])`).
	fn temporary_mutated(&mut self, path: PathId, arguments: &[Argument], callee: &str) {
		let PathRoot::Param(index) = self.s.types.path_data(path).0.root else {
			return;
		};
		let Some(Argument::Given(argument)) = arguments.get(index as usize) else {
			return;
		};
		let argument = self.strip_groups(*argument);
		let temporary = matches!(
			self.view(argument),
			View::Object { .. } | View::TypedObject { .. }
		);
		if !temporary || !self.places_of(argument).is_empty() {
			return;
		}
		let message = format!(
			"`{callee}` mutates this argument, but it is a temporary value: the mutation would be lost (store it in a `mutable` first)"
		);
		let diagnostic =
			self.diagnostic(&crate::diagnostics::codes::types::EF012, argument, message);
		self.report_flow(diagnostic);
	}

	/// `EFF007`: a mutated argument must be a writable place. Returns whether it is.
	fn check_mutable_argument(
		&mut self,
		base: PlaceId,
		node: NodeId,
		callee: &str,
		method: bool,
	) -> bool {
		// a set element or a map key is read-only (`CL031`, collections.rs)
		if let Some(symbol) = self.root_symbol(node)
			&& (self.readonly_elements.contains(&symbol)
				|| self.readonly_place(base)
				|| self.evaluated_origins(node).readonly)
		{
			let name = self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name)
				.to_string();
			let message = format!(
				"`{callee}` changes `{}`, but `{name}` is a key or an element of a set: it can't be modified; delete it and add the new value",
				self.place_text(base)
			);
			let diagnostic =
				self.diagnostic(&crate::diagnostics::codes::types::CL031, node, message);
			self.report_flow(diagnostic);
			return false;
		}
		let Writable::No(symbol) = self.place_writable(base) else {
			return true;
		};
		let name = self
			.s
			.types
			.syms
			.resolve(self.a.symbols.symbol(symbol).name)
			.to_string();
		let kind = self.a.symbols.symbol(symbol).kind;
		let how = match kind {
			SymbolKind::Let => "declared with `let`: declare it with `mutable`",
			SymbolKind::LoopVariable { .. } => "a copy made by the loop",
			_ => "immutable",
		};
		let message = if method {
			format!(
				"`{callee}` changes `{}`, but `{name}` is {how}",
				self.place_text(base)
			)
		} else {
			format!("`{callee}` mutates its argument, but `{name}` is {how}")
		};
		let diagnostic = self.diagnostic(&EFF007, node, message);
		self.report_flow(diagnostic);
		false
	}

	/// One effect of a built-in method on its receiver's place.
	fn builtin_effect(
		&mut self,
		effect: BuiltinEffect,
		base: PlaceId,
		arguments: &[Argument],
		n: NodeId,
	) {
		let argument = |index: u8| -> Option<NodeId> {
			arguments
				.iter()
				.filter_map(|argument| match argument {
					Argument::Given(node) => Some(*node),
					_ => None,
				})
				.nth(index as usize)
		};
		match effect {
			BuiltinEffect::MutatesSelf => self.record_effect(base, false, n, true),
			BuiltinEffect::MutatesSize => {
				let size = self.places.child(base, Step::Size);
				self.record_effect(size, false, n, true);
			}
			BuiltinEffect::MutatesLength => {
				let length = self.places.child(base, Step::Length);
				self.record_effect(length, false, n, true);
			}
			BuiltinEffect::MutatesCapacity => {
				let capacity = self.places.child(base, Step::Capacity);
				self.record_effect(capacity, false, n, true);
			}
			BuiltinEffect::InvalidatesLast => {
				let last = self
					.places
					.child(base, Step::Index(Identity::LengthOf(base, -1)));
				self.record_effect(last, true, n, true);
				let length = self.places.child(base, Step::Length);
				self.record_effect(length, false, n, true);
			}
			BuiltinEffect::InvalidatesFrom(index) => {
				let start =
					argument(index).map_or(Identity::Unknown, |node| self.identity_of(node));
				let from = self
					.places
					.child(base, Step::Range(start, Identity::Unknown, false));
				self.record_effect(from, true, n, true);
				let length = self.places.child(base, Step::Length);
				self.record_effect(length, false, n, true);
			}
			BuiltinEffect::InvalidatesKey(index) | BuiltinEffect::MutatesKey(index) => {
				let key = argument(index).map_or(Identity::Unknown, |node| self.identity_of(node));
				let element = self.places.child(base, Step::Index(key));
				let invalidates = matches!(effect, BuiltinEffect::InvalidatesKey(_));
				self.record_effect(element, invalidates, n, true);
				let size = self.places.child(base, Step::Size);
				self.record_effect(size, false, n, true);
			}
			BuiltinEffect::InvalidatesArgument(index) => {
				// `pool.delete(element)`: the references that may designate
				// the element (its place, from the reference given)
				let mut elements = argument(index)
					.map(|node| self.places_of(node))
					.unwrap_or_default();
				if elements.is_empty() {
					elements.push(self.places.child(base, Step::Index(Identity::Unknown)));
				}
				for element in elements {
					self.record_effect(element, true, n, true);
				}
				let size = self.places.child(base, Step::Size);
				self.record_effect(size, false, n, true);
			}
		}
	}

	// ── aliases ───────────────────────────────────────────────────────────

	/// Whether a symbol designates another place: a `use`, a `mutable use`,
	/// a `for` variable over a collection.
	pub(crate) fn is_alias(&self, symbol: SymbolId) -> bool {
		matches!(
			self.a.symbols.symbol(symbol).kind,
			SymbolKind::Use | SymbolKind::MutableUse | SymbolKind::LoopVariable { copy: false }
		)
	}

	/// Whether the place an alias designates may be written.
	pub(crate) fn alias_writable(&self, symbol: SymbolId) -> bool {
		// a set element or a map key is read-only (`CL031`, collections.rs)
		if self.readonly_elements.contains(&symbol) {
			return false;
		}
		if self.s.types.is_recovering(self.a.symbols.symbol(symbol).ty) {
			return true;
		}
		if let Some(state) = self.body.flow.ref_state(symbol) {
			let state = self.facts.refs.get(state);
			let places = &state.places;
			return !state.readonly
				&& !places.is_empty()
				&& places
					.iter()
					.all(|&place| self.place_writable(place) == Writable::Yes);
		}
		match self.aliases.get(&symbol) {
			Some(&place) => self.place_writable(place) == Writable::Yes,
			None => false,
		}
	}

	/// `REF015` / `REF014`: a write through an alias of an immutable place.
	/// Type part. Returns whether it reported.
	pub(crate) fn report_read_only_alias(
		&mut self,
		target: NodeId,
		symbol: SymbolId,
		below: bool,
	) -> bool {
		if !self.is_alias(symbol) {
			return false;
		}
		let entry = self.a.symbols.symbol(symbol);
		let name = self.s.types.syms.resolve(entry.name).to_string();
		let loop_variable = matches!(entry.kind, SymbolKind::LoopVariable { .. });
		let designated = self
			.body
			.flow
			.ref_state(symbol)
			.and_then(|state| {
				self.facts
					.refs
					.get(state)
					.places
					.iter()
					.copied()
					.find(|&place| self.place_writable(place) != Writable::Yes)
			})
			.or_else(|| self.aliases.get(&symbol).copied());
		let (code, message) = match designated {
			Some(place) => {
				let text = self.place_text(place);
				let root = self
					.place_symbol(place)
					.map(|root| {
						self.s
							.types
							.syms
							.resolve(self.a.symbols.symbol(root).name)
							.to_string()
					})
					.unwrap_or_else(|| text.clone());
				if loop_variable {
					// values brief T-LOOP-43: a write of an immutable binding
					let code = if below {
						&crate::diagnostics::codes::values::E_DECL_4
					} else {
						&crate::diagnostics::codes::values::E_DECL_3
					};
					(
						code,
						format!(
							"`{name}` is an element of `{root}`, which is immutable: declare `{root}` with `mutable` to change its elements"
						),
					)
				} else {
					(
						&REF015,
						format!(
							"`{name}` designates `{text}`, and `{root}` is immutable: the alias is read-only"
						),
					)
				}
			}
			None => (
				&REF014,
				format!("`{name}` is an element of a temporary collection: it can't be written"),
			),
		};
		let diagnostic = self.diagnostic(code, target, message);
		self.report(diagnostic);
		true
	}

	// ── closures ──────────────────────────────────────────────────────────

	/// Records what a closure captures (`FnInfo::captures`), passes on to the
	/// enclosing function what it captures in turn, and keeps the captured
	/// locals it writes (`REF021` if it escapes).
	pub(crate) fn finish_captures(
		&mut self,
		cx: &FnCx,
		lambda: NodeId,
		declaration: Option<NodeId>,
	) {
		for key in [Some(lambda), declaration].into_iter().flatten() {
			if let Some(info) = self.a.functions.get_mut(&key) {
				info.captures = cx.captures.clone();
			}
		}
		for &symbol in &cx.captures {
			if self.is_captured(symbol)
				&& let Some(parent) = self.body.functions.last_mut()
				&& !parent.captures.contains(&symbol)
			{
				parent.captures.push(symbol);
			}
		}
		let writes: Vec<(SymbolId, NodeId)> = cx
			.effects
			.iter()
			.filter_map(|entry| {
				let symbol = self.place_symbol(entry.place)?;
				cx.captures
					.contains(&symbol)
					.then_some((symbol, entry.node))
			})
			.collect();
		if !writes.is_empty() {
			self.closure_writes.insert(lambda, writes);
		}
	}

	/// A closure that escapes (stored, returned): it copies what it
	/// captures, so it can't write any of it (`REF021`). `value` is a lambda,
	/// or the name of a local function; `at` where it escapes.
	pub(crate) fn escape(&mut self, value: NodeId, at: NodeId) {
		let origins = self.evaluated_origins(value);
		for closure in origins.closures {
			if let Some(lambda) = closure.callable.local() {
				self.escape_closure(lambda, at);
			}
		}
	}

	/// A closure (a lambda node, or a `watch` node or `derived` declaration,
	/// which copy their captures too) that escapes: `REF021` on its writes
	/// of what it captures.
	pub(crate) fn escape_closure(&mut self, lambda: NodeId, at: NodeId) {
		self.a.escapes.insert(lambda);
		self.check_escaping_references(lambda, at);
		// requirements are not captured (requirements.rs)
		self.check_stored_lambda(lambda);
		let Some(writes) = self.closure_writes.get(&lambda).cloned() else {
			return;
		};
		let line = {
			let start = self.span(at).start as usize;
			self.tree.source[..start.min(self.tree.source.len())]
				.matches('\n')
				.count() + 1
		};
		for (symbol, write) in writes {
			if !self.reported_once.insert((write, "REF021")) {
				continue;
			}
			let name = self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name)
				.to_string();
			let message = format!(
				"`{name}` is captured by copy, since the closure escapes (line {line}): the write would be lost. Return the new value, or keep the state in a field"
			);
			let diagnostic =
				self.diagnostic(&crate::diagnostics::codes::advanced::REF021, write, message);
			self.report(diagnostic);
		}
	}

	// ── lengths ───────────────────────────────────────────────────────────

	/// The length fact of a local collection's place, when known.
	fn place_length(&mut self, place: PlaceId) -> Option<crate::types::Interval> {
		if self.places.parent(place).is_some() {
			return None;
		}
		let symbol = self.place_symbol(place)?;
		self.length_of(symbol)
	}

	/// The length of an array after one of its methods changed it: one more
	/// after `push`/`insert`, one less after `pop` (at least 0), `delete` and
	/// `take` (which fail on an empty array), 0 after `clear`.
	fn track_length(&mut self, place: PlaceId, path: &str, before: crate::types::Interval) {
		let one = I256::ONE;
		let after = match path {
			"Array#push" | "Array#insert" | "OrderedColumnar#add" => {
				crate::types::Interval::new(before.lo + one, before.hi.saturating_add(one))
			}
			"Array#pop" | "OrderedColumnar#pop" => crate::types::Interval::new(
				(before.lo - one).max(I256::ZERO),
				(before.hi - one).max(I256::ZERO),
			),
			"Array#delete" | "Array#take" | "OrderedColumnar#delete" | "OrderedColumnar#take"
				if before.hi >= one =>
			{
				crate::types::Interval::new((before.lo - one).max(I256::ZERO), before.hi - one)
			}
			"Array#clear" | "OrderedColumnar#clear" => crate::types::Interval::exact(I256::ZERO),
			_ => return,
		};
		let mut fact = self
			.body
			.flow
			.fact(place)
			.map_or(crate::flow::Fact::TOP, |fact| self.facts.fact_value(fact));
		fact.length = self.facts.interval(after);
		let id = self.facts.fact(fact);
		self.body.flow.set_fact(place, id);
	}

	/// The declared type a callee's effect path designates: its parameter's
	/// type, then the fields and elements of its steps (`None`: not known,
	/// the caller's types are used).
	fn path_type(&self, path: PathId, params: &[TypeId]) -> Option<TypeId> {
		let (data, steps) = self.s.types.path_data(path);
		let mut ty = match data.root {
			PathRoot::Param(index) => *params.get(index as usize)?,
			PathRoot::SelfParam => *params.first()?,
			_ => return None,
		};
		for step in steps {
			ty = match *step {
				PathStep::Field(name) | PathStep::Private(name) => self
					.s
					.types
					.fields_of(ty)
					.iter()
					.find(|field| field.name == name)
					.map(|field| field.ty)?,
				PathStep::Length | PathStep::Size | PathStep::Capacity => TypeId::INTEGER,
				PathStep::All
				| PathStep::IndexConst(_)
				| PathStep::IndexParam(_)
				| PathStep::IndexFrom(_)
				| PathStep::Last => match self.s.types.kind(ty) {
					TypeKind::Collection(_, item, _) => item,
					TypeKind::String => TypeId::STRING,
					_ => return None,
				},
			};
		}
		(!self.s.types.is_recovering(ty)).then_some(ty)
	}
}
