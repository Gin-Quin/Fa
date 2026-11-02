//! Numbers: float, big integer and percentage literals, arithmetic trees,
//! intervals and overflow bits, compound assignments, integer sinks,
//! conversions and `abs()` (design §3.6, values brief §6).
//!
//! The type part of an arithmetic node follows the result table of values
//! brief §6.6: an integer node has the **base type** of its operands (the full
//! range of their common storage); its values are a flow fact, recomputed at
//! every visit from the operands' current intervals (`Analysis::arith`,
//! `Analysis::intervals`). The flow part sets the check bits the backends read
//! (`OVERFLOW_NATIVE`, `OVERFLOW_JS`, `DIVISOR_CHECK`, `EXPONENT_CHECK`,
//! `MIN_DIV_GUARD`, `CAN_FAIL`), and [`Walker::sink_integer`] sets `SINK_CHECK`
//! on arithmetic roots that may not fit their sink (values brief D-4, D-89).

use std::sync::Arc;

use ethnum::I256;
use smallvec::SmallVec;

use crate::analysis::{ArithInfo, NodeFlags};
use crate::check::{PlaceInfo, Walker};
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::values::{
	E_NUM_6, E_NUM_7, E_NUM_8, E_NUM_10, E_NUM_11, E_NUM_12, E_NUM_14, E_NUM_15, E_NUM_16,
	E_NUM_18, E_NUM_21, W_NUM_1,
};
use crate::flow::IntervalSet;
use crate::ids::{FactId, NodeId, PlaceId, SymbolId, TypeId};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, ArithOp, NodeList, View};
use crate::types::int::{Fit, js_safe_range};
use crate::types::{ComputeType, ConstValue, FloatBits, IntStorage, Interval, TypeKind};

/// The kind of a number type (values brief §1: integer types of every width
/// are one kind).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum NumKind {
	/// An integer type of this storage.
	Int(IntStorage),
	/// `Float` or `Float32`.
	Float(FloatBits),
	/// `BigInt`.
	BigInt,
	/// `Percentage`.
	Percentage,
	/// `String` or `Character` (`+` concatenates).
	Text,
	/// A type being recovered from: no further diagnostic.
	Recovering,
	/// Anything else.
	Other,
}

/// The check bits recomputed by the flow part of an arithmetic node.
const ARITH_BITS: NodeFlags = NodeFlags::OVERFLOW_NATIVE
	.union(NodeFlags::OVERFLOW_JS)
	.union(NodeFlags::DIVISOR_CHECK)
	.union(NodeFlags::EXPONENT_CHECK)
	.union(NodeFlags::MIN_DIV_GUARD)
	.union(NodeFlags::CAN_FAIL)
	.union(NodeFlags::CONST_FOLDED);

/// What the flow part of an integer operation found.
#[derive(Default)]
struct StepFlags {
	flags: NodeFlags,
	/// A step is certainly outside the compute type.
	certain_overflow: bool,
	/// A divisor is exactly zero.
	zero_divisor: bool,
	/// An exponent is certainly negative.
	negative_exponent: bool,
}

/// The smallest signed storage containing both (an `Int8 + UInt8` mix, after `E-NUM-15`).
fn covering_storage(a: IntStorage, b: IntStorage) -> IntStorage {
	let range = a.range().hull(b.range());
	[
		IntStorage::I16,
		IntStorage::I32,
		IntStorage::I64,
		IntStorage::I128,
	]
	.into_iter()
	.find(|storage| storage.range().contains_interval(range))
	.unwrap_or(IntStorage::I128)
}

/// Whether two storages need a conversion at runtime (another width or another
/// JavaScript class).
fn representation_changes(a: IntStorage, b: IntStorage) -> bool {
	a.bits() != b.bits() || a.is_signed() != b.is_signed() || a.is_js_bigint() != b.is_js_bigint()
}

fn describe(interval: Interval) -> String {
	match interval.as_exact() {
		Some(value) => value.to_string(),
		None => format!(
			"{} to {}",
			bound(interval.lo, "−∞"),
			bound(interval.hi, "+∞")
		),
	}
}

fn bound(value: I256, infinite: &str) -> String {
	if value >= crate::types::int::INFINITY || value <= -crate::types::int::INFINITY {
		infinite.to_string()
	} else {
		value.to_string()
	}
}

impl Walker<'_> {
	// ── kinds ─────────────────────────────────────────────────────────────

	/// The number kind of a type.
	pub(crate) fn num_kind(&self, ty: TypeId) -> NumKind {
		if ty.is_unset() || self.s.types.is_recovering(ty) {
			return NumKind::Recovering;
		}
		match self.s.types.kind(ty) {
			TypeKind::Int(id) => NumKind::Int(self.s.types.int(id).storage),
			TypeKind::Float(bits) | TypeKind::FloatLit(bits, _) => NumKind::Float(bits),
			TypeKind::BigInt | TypeKind::BigIntLit(_) => NumKind::BigInt,
			TypeKind::Percentage => NumKind::Percentage,
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => NumKind::Text,
			TypeKind::Never => NumKind::Recovering,
			_ => NumKind::Other,
		}
	}

	/// The node with its parentheses removed.
	/// The node with its parentheses (and a `try`, which has the value of its
	/// operand) removed.
	pub(crate) fn strip_groups(&self, n: NodeId) -> NodeId {
		let mut current = n;
		while let View::Group(inner) | View::Try(inner) = syntax::view(&self.tree, current) {
			current = inner;
		}
		current
	}

	/// Whether a node is a number literal or a constant expression of number
	/// literals: it takes the kind of the other operand (values brief §6.3).
	pub(crate) fn is_number_literal(&self, n: NodeId) -> bool {
		match syntax::view(&self.tree, n) {
			View::Integer(_) | View::Float(_) => true,
			View::Group(inner) | View::Negate(inner) => self.is_number_literal(inner),
			View::Arithmetic { operands, .. } => operands
				.iter()
				.all(|operand| self.is_number_literal(operand)),
			_ => false,
		}
	}

	/// Whether a node (groups skipped) is an integer arithmetic node: the root
	/// of a tree when it reaches a sink.
	fn is_arith_node(&self, n: NodeId) -> bool {
		self.a.arith.contains_key(&self.strip_groups(n))
	}

	// ── intervals and facts ───────────────────────────────────────────────

	/// The flow place of a local symbol.
	pub(crate) fn symbol_place(&mut self, symbol: SymbolId) -> PlaceId {
		self.places.symbol(symbol)
	}

	/// The integer values of a local symbol here: its fact, or its type's range.
	pub(crate) fn symbol_values(&mut self, symbol: SymbolId) -> Option<IntervalSet> {
		let ty = self.a.symbols.symbol(symbol).ty;
		let range = self.s.types.interval_of(ty)?;
		let place = self.symbol_place(symbol);
		let fact = self
			.body
			.flow
			.fact(place)
			.or_else(|| self.visible_binding(symbol));
		let set = fact
			.map(|fact| self.facts.fact_value(fact).int)
			.filter(|int| *int != crate::ids::IntervalSetId::NONE)
			.map(|int| self.facts.interval_set_value(int).clone());
		Some(match set {
			Some(set) if range.contains_interval(set.hull) => set,
			Some(set) => set
				.restrict(range)
				.unwrap_or_else(|| IntervalSet::of(range)),
			None => IntervalSet::of(range),
		})
	}

	/// The fact of an immutable binding, when visible here: not across a
	/// function boundary (decision A11: a function body sees no facts of the
	/// places it captures, whatever the order of analysis).
	pub(crate) fn visible_binding(&self, symbol: SymbolId) -> Option<FactId> {
		let fact = self.facts.binding(symbol)?;
		let target = self.a.symbols.symbol(symbol).scope;
		let mut current = Some(self.body.scope);
		while let Some(scope) = current {
			if scope == target {
				return Some(fact);
			}
			let entry = self.a.symbols.scope(scope);
			if entry.kind == ScopeKind::Function {
				return None;
			}
			current = entry.parent;
		}
		None
	}

	/// The fact of an integer set for a symbol of type `ty` (`TOP` when it says
	/// nothing beyond the type).
	fn values_fact(&mut self, ty: TypeId, set: Option<IntervalSet>) -> FactId {
		let range = self.s.types.interval_of(ty);
		let set = set.and_then(|set| match range {
			Some(range) => set.restrict(range),
			None => Some(set),
		});
		match set {
			Some(set) if Some(set.hull) != range || !set.excluded.is_empty() => {
				self.facts.int_fact(set)
			}
			_ => FactId(0),
		}
	}

	/// Sets the integer values of a local symbol here (`None`: nothing known
	/// beyond its type and its binding).
	pub(crate) fn set_symbol_values(&mut self, symbol: SymbolId, set: Option<IntervalSet>) {
		let ty = self.a.symbols.symbol(symbol).ty;
		let place = self.symbol_place(symbol);
		let mut fact = self.values_fact(ty, set);
		// a narrowing of the place stays (narrowing.rs)
		if let Some(current) = self.body.flow.fact(place) {
			let narrowed = self.facts.fact_value(current).narrowed;
			if !narrowed.is_unset() {
				let mut value = self.facts.fact_value(fact);
				value.narrowed = narrowed;
				fact = self.facts.fact(value);
			}
		}
		self.body.flow.assign(&self.places, place, fact);
	}

	/// The integer values of an expression in the current flow state: the fact
	/// of a local read, the interval of an arithmetic node or a conversion,
	/// else the range of its type. `None` for a non-integer.
	pub(crate) fn value_set(&mut self, n: NodeId) -> Option<IntervalSet> {
		let n = self.strip_groups(n);
		let ty = self.type_of(n);
		let range = self.s.types.interval_of(ty)?;
		if let View::Name(_) = syntax::view(&self.tree, n)
			&& let Resolution::Local(symbol) = self.a.resolution(n)
		{
			let set = self.symbol_values(symbol)?;
			self.record_interval(n, set.hull, range);
			return Some(set);
		}
		// `c.length` of a local with a length fact (proofs.rs)
		if let Some(collection) = self.length_access(n)
			&& let Some(length) = self.length_of(collection)
			&& let Some(inside) = length.intersect(range)
		{
			return Some(IntervalSet::of(inside));
		}
		Some(match self.a.intervals.get(&n) {
			Some(&interval) => IntervalSet::of(interval),
			None => IntervalSet::of(range),
		})
	}

	/// Records the values of an integer node when narrower than its type (hover, proofs).
	pub(crate) fn record_interval(&mut self, n: NodeId, interval: Interval, range: Interval) {
		if interval == range {
			self.a.intervals.remove(&n);
		} else {
			self.a.intervals.insert(n, interval);
		}
	}

	/// The flow part of a binding or an assignment: the place of `symbol` now
	/// holds the value of node `value` (its interval, for an integer; nothing
	/// known otherwise). Called by `decl.rs` after the type part.
	pub(crate) fn bind_value(&mut self, symbol: SymbolId, value: NodeId) {
		self.bind_or_assign(symbol, value, false);
	}

	/// The flow part of an assignment: like a binding, and the assigned value
	/// narrows a local of a union type (values brief §14, T-NAR-8).
	pub(crate) fn assign_value(&mut self, symbol: SymbolId, value: NodeId) {
		self.bind_or_assign(symbol, value, true);
	}

	fn bind_or_assign(&mut self, symbol: SymbolId, value: NodeId, assignment: bool) {
		// a collection assigned: the relations `i < c.length` to it end (proofs.rs)
		if assignment && self.body.flow.reachable {
			let place = self.symbol_place(symbol);
			self.drop_relations_to(place);
		}
		if !self.body.flow.reachable {
			return;
		}
		let (ty, kind) = {
			let symbol = self.a.symbols.symbol(symbol);
			(symbol.ty, symbol.kind)
		};
		let set = match self.s.types.as_int(ty) {
			Some(_) => self.value_set(value),
			None => None,
		};
		// a declaration has its declared type; an assignment narrows
		let narrowed = if assignment {
			let value_ty = self.type_of(self.strip_groups(value));
			self.assignment_narrowing(ty, value_ty)
		} else {
			TypeId::UNSET
		};
		let length = self.binding_length(symbol, value);
		let with_parts = |w: &mut Self, fact: FactId| {
			if narrowed.is_unset() && length == crate::ids::IntervalId::NONE {
				return fact;
			}
			let mut parts = w.facts.fact_value(fact);
			if !narrowed.is_unset() {
				parts.narrowed = narrowed;
			}
			parts.length = length;
			w.facts.fact(parts)
		};
		if kind == SymbolKind::Let {
			// immutable: the fact holds in the whole scope, out of the flow states
			let fact = self.values_fact(ty, set);
			let fact = with_parts(self, fact);
			self.facts.bind(symbol, fact);
			let place = self.symbol_place(symbol);
			if fact != FactId(0) {
				self.places.mark_bound(place);
			}
			self.body.flow.remove_fact(place);
		} else {
			let place = self.symbol_place(symbol);
			let fact = self.values_fact(ty, set);
			let fact = with_parts(self, fact);
			self.body.flow.assign(&self.places, place, fact);
		}
	}

	// ── sinks ─────────────────────────────────────────────────────────────

	/// The integer type a sink of type `target` checks a value against: the
	/// target itself, or the only integer member of a union (`Integer?`).
	pub(crate) fn int_target(&self, target: TypeId) -> Option<TypeId> {
		match self.s.types.kind(target) {
			TypeKind::Int(_) => Some(target),
			TypeKind::Union(list) => {
				let mut members = self
					.s
					.types
					.list_items(list)
					.iter()
					.copied()
					.filter(|&member| self.s.types.as_int(member).is_some());
				let first = members.next()?;
				members.next().is_none().then_some(first)
			}
			_ => None,
		}
	}

	/// Whether `sink` checks this value as an integer range ([`Walker::sink_integer`]).
	pub(crate) fn is_integer_sink(&self, actual: TypeId, target: TypeId) -> bool {
		self.s.types.as_int(actual).is_some() && self.int_target(target).is_some()
	}

	/// An integer value (node `n`) lands in a sink of type `target` (values brief
	/// §6.4, design §3.6): no flag when its interval fits; `SINK_CHECK` and a
	/// coercion for an arithmetic root that may not fit (D-89); `E-NUM-10` when
	/// it can't fit (`E-NUM-6` for a literal); `E-NUM-14` for any other value
	/// that may not fit. Called by `sink` at every visit.
	pub(crate) fn sink_integer(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		let root = self.strip_groups(n);
		self.a.coercions.remove(&root);
		self.clear_flag(root, NodeFlags::SINK_CHECK);
		let Some(target) = self.int_target(target) else {
			let message = format!(
				"a `{}` may not fit in a `{}`: check its range first, or convert it",
				self.display(actual),
				self.display(target)
			);
			let diagnostic = self.diagnostic(&E_NUM_14, n, message);
			self.report_flow(diagnostic);
			return;
		};
		if !self.body.flow.reachable {
			return;
		}
		let Some(mut set) = self.value_set(n) else {
			return;
		};
		// an arithmetic root is checked against its compute type first
		// (`OVERFLOW_*`): only the values passing that check reach the sink
		if let Some(info) = self.a.arith.get(&root) {
			match set.restrict(info.compute.range()) {
				Some(inside) => set = inside,
				None => return, // a certain overflow, reported at the node
			}
		}
		let target_type = *self.s.types.as_int(target).expect("an integer target");
		let range = target_type.interval();
		let source = self.s.types.as_int(actual).map(|int| int.storage);
		if range.contains_interval(set.hull) {
			if source.is_some_and(|storage| representation_changes(storage, target_type.storage)) {
				self.a.coercions.insert(root, target);
			}
			return;
		}
		let literal = matches!(syntax::view(&self.tree, root), View::Integer(_));
		if set.restrict(range).is_none() {
			let (code, what) = if literal {
				(&E_NUM_6, "this literal".to_string())
			} else {
				(&E_NUM_10, "this value".to_string())
			};
			let message = format!(
				"{what} is {}, which doesn't fit in a `{}` ({})",
				describe(set.hull),
				self.display(target),
				describe(range)
			);
			let diagnostic = self.diagnostic(code, n, message);
			self.report_flow(diagnostic);
		} else if self.is_arith_node(root) {
			self.set_flag(root, NodeFlags::SINK_CHECK);
			self.a.coercions.insert(root, target);
		} else {
			let text = self.tree.text(root.index()).to_string();
			let message = format!(
				"`{text}` may be {}, which doesn't fit in a `{}` ({}): narrow it with an `if`, or convert it with `{}({text})`",
				describe(set.hull),
				self.display(target),
				describe(range),
				self.display(target)
			);
			let diagnostic = self.diagnostic(&E_NUM_14, n, message);
			self.report_flow(diagnostic);
		}
	}

	/// The sink of an unannotated binding (values brief §6.4): an integer value
	/// lands in its base type; `E-NUM-15` when its operands have no common
	/// base type, `E-NUM-7` for a literal beyond `Integer`. Also for the other
	/// sinks without expected type (a method receiver, an interpolation).
	pub(crate) fn sink_unannotated(&mut self, value: NodeId, actual: TypeId) {
		if self.s.types.as_int(actual).is_none() || self.s.types.is_recovering(actual) {
			return;
		}
		let root = self.strip_groups(value);
		if let Some((a, b)) = self.ambiguous_base(root) {
			let message = format!(
				"an `{}` and a `{}` have no common type: annotate the result, ex: `let value: {} = …`",
				a.name(),
				b.name(),
				covering_storage(a, b).name()
			);
			let diagnostic = self.diagnostic(&E_NUM_15, value, message);
			self.report_flow(diagnostic);
			return;
		}
		if let View::Integer(literal) = syntax::view(&self.tree, root)
			&& !IntStorage::Default.range().contains(literal)
		{
			let message =
				format!("{literal} doesn't fit in an `Integer`: write `{literal}n` for a `BigInt`");
			let diagnostic = self.diagnostic(&E_NUM_7, value, message);
			self.report_flow(diagnostic);
			return;
		}
		let base = self.s.types.widen(actual);
		self.sink_integer(value, actual, base);
	}

	/// The two storages of an integer arithmetic node that have no common base
	/// type (`Int8 + UInt8`), if any.
	fn ambiguous_base(&self, n: NodeId) -> Option<(IntStorage, IntStorage)> {
		let View::Arithmetic { op, operands } = syntax::view(&self.tree, n) else {
			return None;
		};
		if op == ArithOp::Divide {
			return None;
		}
		let storages = self.operand_storages(operands);
		let mut base = *storages.first()?;
		for &storage in &storages[1..] {
			match base.common_base(storage) {
				Some(common) => base = common,
				None => return Some((base, storage)),
			}
		}
		None
	}

	/// The storages that decide the base type of an integer operation: those of
	/// the operands that are not literals (a literal contributes its value
	/// only, D-94), or of every operand when all are literals.
	fn operand_storages(&self, operands: NodeList<'_>) -> SmallVec<[IntStorage; 4]> {
		let storage = |w: &Self, operand: NodeId| {
			w.s.types
				.as_int(w.type_of(w.strip_groups(operand)))
				.map(|int| int.storage)
		};
		let non_literal: SmallVec<[IntStorage; 4]> = operands
			.iter()
			.filter(|&operand| !self.is_number_literal(operand))
			.filter_map(|operand| storage(self, operand))
			.collect();
		if !non_literal.is_empty() {
			return non_literal;
		}
		operands
			.iter()
			.filter_map(|operand| storage(self, operand))
			.collect()
	}

	// ── literals ──────────────────────────────────────────────────────────

	/// A float literal: `Float`, or `Float32` where one is expected (rounded to
	/// nearest); `E-NUM-8` when it rounds to infinity.
	pub(super) fn check_float(
		&mut self,
		n: NodeId,
		value: f64,
		expected: Option<TypeId>,
	) -> TypeId {
		self.memo(n, |w| {
			if !value.is_finite() {
				let diagnostic = w.diagnostic(
					&E_NUM_8,
					n,
					"this literal is too large for a `Float`: it would be infinite",
				);
				w.report(diagnostic);
				return TypeId::FLOAT;
			}
			let literal = w.s.types.float_literal(FloatBits::F64, value);
			let Some(expected) = expected else {
				return literal;
			};
			match w.s.types.adapt_literal(literal, expected) {
				Some(adapted) => adapted,
				None if w.num_kind(expected) == NumKind::Float(FloatBits::F32) => {
					let diagnostic = w.diagnostic(
						&E_NUM_8,
						n,
						"this literal is too large for a `Float32`: it would be infinite",
					);
					w.report(diagnostic);
					TypeId::FLOAT32
				}
				None => literal,
			}
		})
	}

	/// A big integer literal (`12n`, `0xFFn`, `-3n`).
	pub(super) fn check_big_int(&mut self, n: NodeId, text: &str) -> TypeId {
		self.memo(n, |w| {
			let digits: String = text
				.trim_end_matches('n')
				.chars()
				.filter(|&c| c != '_')
				.collect();
			let (negative, digits) = match digits.strip_prefix('-') {
				Some(rest) => (true, rest.to_string()),
				None => (false, digits),
			};
			let (radix, digits) = match digits.get(..2) {
				Some("0x") => (16, &digits[2..]),
				Some("0o") => (8, &digits[2..]),
				Some("0b") => (2, &digits[2..]),
				_ => (10, &digits[..]),
			};
			let decimal = match I256::from_str_radix(digits, radix) {
				Ok(value) => (if negative { -value } else { value }).to_string(),
				Err(_) => format!("{}{digits}", if negative { "-" } else { "" }),
			};
			let sym = w.s.types.syms.intern(&decimal);
			let constant = w.s.types.constant(ConstValue::BigInt(sym));
			w.s.types.intern(TypeKind::BigIntLit(constant))
		})
	}

	/// `50%`, `(x)%`: a `Percentage` from an integer or a float (values brief D-45).
	pub(super) fn check_percentage(&mut self, n: NodeId, value: NodeId) -> TypeId {
		let actual = self.check(value, None);
		self.memo(n, |w| {
			match w.num_kind(actual) {
				NumKind::Int(_) | NumKind::Float(_) | NumKind::Recovering => {}
				_ => {
					let message = format!(
						"`%` makes a `Percentage` from an integer or a float, not from a `{}`",
						w.display(actual)
					);
					let diagnostic = w.diagnostic(&E_NUM_16, n, message);
					w.report(diagnostic);
				}
			}
			TypeId::PERCENTAGE
		})
	}

	// ── arithmetic ────────────────────────────────────────────────────────

	/// `+ - * / // %% **`: the operands fold left, `**` right (see `syntax::View::Arithmetic`).
	pub(super) fn check_arithmetic(
		&mut self,
		n: NodeId,
		op: ArithOp,
		operands: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let nodes: SmallVec<[NodeId; 4]> = operands.iter().collect();
		let types = self.check_operands(&nodes, expected);
		let ty = self.memo(n, |w| w.arithmetic_type(n, op, operands, &types));
		if self.s.types.as_int(ty).is_some() {
			self.arithmetic_flow(n, op, operands, ty);
		} else {
			self.clear_arith(n);
		}
		if types.iter().all(|&ty| self.num_kind(ty) == NumKind::BigInt) {
			self.big_int_divisor(n, op, operands);
		}
		ty
	}

	/// Checks the operands of an operation: the others first, in order, then
	/// the number literals, which take the kind of the others (values brief
	/// §6.3; literals have no flow effect, so the order doesn't matter).
	pub(crate) fn check_operands(
		&mut self,
		operands: &[NodeId],
		expected: Option<TypeId>,
	) -> SmallVec<[TypeId; 4]> {
		let mut types: SmallVec<[TypeId; 4]> = SmallVec::from_elem(TypeId::UNSET, operands.len());
		let mut other_kind = None;
		for (index, &operand) in operands.iter().enumerate() {
			if !self.is_number_literal(operand) {
				let ty = self.check(operand, None);
				types[index] = ty;
				if other_kind.is_none() && !matches!(self.num_kind(ty), NumKind::Recovering) {
					other_kind = Some(ty);
				}
			}
		}
		// all literals: a float literal gives its kind to the integer ones (`1 == 1.0`)
		if other_kind.is_none() && expected.is_none() {
			for (index, &operand) in operands.iter().enumerate() {
				if self.is_float_literal(operand) {
					let ty = self.check(operand, None);
					types[index] = ty;
					other_kind = other_kind.or(Some(ty));
				}
			}
		}
		let literal_expected = match other_kind {
			Some(other) => match self.num_kind(other) {
				NumKind::Float(_) | NumKind::BigInt => Some(self.s.types.widen(other)),
				_ => None,
			},
			None => expected.filter(|&expected| {
				matches!(
					self.num_kind(expected),
					NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt
				)
			}),
		};
		for (index, &operand) in operands.iter().enumerate() {
			if types[index].is_unset() {
				types[index] = self.check(operand, literal_expected);
			}
		}
		types
	}

	/// Whether a node is a float literal, possibly negated or parenthesized.
	fn is_float_literal(&self, n: NodeId) -> bool {
		match syntax::view(&self.tree, n) {
			View::Float(_) => true,
			View::Group(inner) | View::Negate(inner) => self.is_float_literal(inner),
			_ => false,
		}
	}

	/// The result type of an arithmetic node (values brief §6.6). Type part.
	fn arithmetic_type(
		&mut self,
		n: NodeId,
		op: ArithOp,
		operands: NodeList<'_>,
		types: &[TypeId],
	) -> TypeId {
		let kinds: SmallVec<[NumKind; 4]> = types.iter().map(|&ty| self.num_kind(ty)).collect();
		// `+` with a string operand joins strings (strings.rs)
		if op == ArithOp::Add && kinds.contains(&NumKind::Text) {
			let pairs: SmallVec<[(NodeId, TypeId); 4]> =
				operands.iter().zip(types.iter().copied()).collect();
			return self.string_concatenation(&pairs);
		}
		if kinds.contains(&NumKind::Recovering) {
			return TypeId::UNKNOWN;
		}
		// integers: the base type of the non-literal operands
		if kinds.iter().all(|kind| matches!(kind, NumKind::Int(_))) {
			if op == ArithOp::Divide {
				return TypeId::FLOAT;
			}
			let storages = self.operand_storages(operands);
			let mut base = storages.first().copied().unwrap_or(IntStorage::Default);
			for &storage in storages.iter().skip(1) {
				base = base
					.common_base(storage)
					.unwrap_or_else(|| covering_storage(base, storage));
			}
			return TypeId::of_storage(base);
		}
		// fold the other kinds pairwise
		let mut result = kinds[0];
		let mut result_ty = types[0];
		for (index, &kind) in kinds.iter().enumerate().skip(1) {
			match self.combine(op, result, kind) {
				Ok((combined, ty)) => {
					result = combined;
					result_ty = ty;
				}
				Err(()) => {
					let operand = operands.get(index).unwrap_or(n);
					self.report_kind_mix(n, op, result_ty, types[index], operand);
					return TypeId::UNKNOWN;
				}
			}
		}
		result_ty
	}

	/// The kind and type of `a op b` for two operand kinds (not both integers).
	fn combine(&mut self, op: ArithOp, a: NumKind, b: NumKind) -> Result<(NumKind, TypeId), ()> {
		use ArithOp as O;
		use NumKind as K;
		let float = |bits: FloatBits| match bits {
			FloatBits::F32 => TypeId::FLOAT32,
			FloatBits::F64 => TypeId::FLOAT,
		};
		match (a, b) {
			(K::Int(_), K::Int(_)) if op == O::Divide => {
				Ok((K::Float(FloatBits::F64), TypeId::FLOAT))
			}
			(K::Int(x), K::Int(y)) => {
				let base = x.common_base(y).unwrap_or_else(|| covering_storage(x, y));
				Ok((K::Int(base), TypeId::of_storage(base)))
			}
			(K::Float(x), K::Float(y)) if x == y => Ok((K::Float(x), float(x))),
			(K::BigInt, K::BigInt) if op == O::Divide => {
				Ok((K::Float(FloatBits::F64), TypeId::FLOAT))
			}
			(K::BigInt, K::BigInt) => Ok((K::BigInt, TypeId::BIG_INT)),
			(K::Percentage, K::Percentage) => match op {
				O::Add | O::Subtract | O::Multiply => Ok((K::Percentage, TypeId::PERCENTAGE)),
				O::Divide => Ok((K::Float(FloatBits::F64), TypeId::FLOAT)),
				_ => Err(()),
			},
			(K::Float(bits), K::Percentage) | (K::Percentage, K::Float(bits))
				if op == O::Multiply =>
			{
				Ok((K::Float(bits), float(bits)))
			}
			(K::Text, K::Text) if op == O::Add => Ok((K::Text, TypeId::STRING)),
			_ => Err(()),
		}
	}

	/// `E-NUM-18` (an operand isn't a number) or `E-NUM-11` (number kinds mixed).
	fn report_kind_mix(
		&mut self,
		n: NodeId,
		op: ArithOp,
		left: TypeId,
		right: TypeId,
		operand: NodeId,
	) {
		// an optional or a union operand (narrowing.rs)
		if self.union_operand(operand, right) || self.union_operand(n, left) {
			return;
		}
		let (left_kind, right_kind) = (self.num_kind(left), self.num_kind(right));
		let not_number = |kind: NumKind| matches!(kind, NumKind::Other | NumKind::Text);
		let (left_name, right_name) = (
			self.display(self.s.types.widen(left)),
			self.display(self.s.types.widen(right)),
		);
		let diagnostic = if not_number(left_kind) || not_number(right_kind) {
			let culprit = if not_number(left_kind) {
				&left_name
			} else {
				&right_name
			};
			let message = format!(
				"`{}` doesn't apply to a `{culprit}`: it needs numbers",
				op.symbol()
			);
			self.diagnostic(&E_NUM_18, n, message)
		} else {
			let fix = match (left_kind, right_kind) {
				(NumKind::Int(_), NumKind::Float(_)) => {
					let text = self.tree.text(n.index()).to_string();
					format!(": convert the integer, ex: `Float(…)`, in `{text}`")
				}
				(NumKind::Float(FloatBits::F32), NumKind::Float(FloatBits::F64))
				| (NumKind::Float(FloatBits::F64), NumKind::Float(FloatBits::F32)) => {
					": convert one side with `Float(x)` or `Float32(x)`".to_string()
				}
				(NumKind::Percentage, NumKind::Int(_)) | (NumKind::Int(_), NumKind::Percentage) => {
					": write `2.0` for a float result, or `100%` for a percentage".to_string()
				}
				(NumKind::BigInt, _) | (_, NumKind::BigInt) => {
					": convert with `BigInt(i)`, or `Integer(b)`".to_string()
				}
				_ => String::new(),
			};
			let message = format!(
				"cannot use `{}` between a `{left_name}` and a `{right_name}`{fix}",
				op.symbol()
			);
			let _ = operand;
			self.diagnostic(&E_NUM_11, n, message)
		};
		self.report(diagnostic);
	}

	/// Removes the arithmetic info of a node that is not (or no longer) an
	/// integer operation here.
	fn clear_arith(&mut self, n: NodeId) {
		if self.a.arith.remove(&n).is_some() {
			self.clear_flag(n, ARITH_BITS);
			self.a.intervals.remove(&n);
		}
	}

	/// Marks the arithmetic node under `child` (if any) as an inner node of a tree.
	fn mark_inner(&mut self, child: NodeId) {
		let child = self.strip_groups(child);
		if let Some(info) = self.a.arith.get_mut(&child) {
			info.root = false;
		}
	}

	/// The compute type of a node over operands of these types (values brief
	/// §6.7), at least that of its arithmetic children.
	fn compute_type(&self, operands: impl IntoIterator<Item = NodeId>) -> ComputeType {
		let mut storages: SmallVec<[IntStorage; 4]> = SmallVec::new();
		let mut compute = ComputeType::Int64;
		for operand in operands {
			let operand = self.strip_groups(operand);
			if let Some(int) = self.s.types.as_int(self.type_of(operand)) {
				storages.push(int.storage);
			}
			if let Some(info) = self.a.arith.get(&operand) {
				compute = wider(compute, info.compute);
			}
		}
		wider(compute, ComputeType::of(storages))
	}

	/// The flow part of an integer arithmetic node: folds the operands'
	/// intervals, sets the check bits and the arithmetic info (design §3.6).
	fn arithmetic_flow(&mut self, n: NodeId, op: ArithOp, operands: NodeList<'_>, ty: TypeId) {
		let mut sets: SmallVec<[IntervalSet; 4]> = SmallVec::new();
		for operand in operands.iter() {
			match self.value_set(operand) {
				Some(set) => sets.push(set),
				None => {
					self.clear_arith(n);
					return;
				}
			}
			self.mark_inner(operand);
		}
		self.clear_flag(n, ARITH_BITS);
		if !self.body.flow.reachable || sets.is_empty() {
			self.a.arith.remove(&n);
			return;
		}
		let compute = self.compute_type(operands.iter());
		let storage = self
			.s
			.types
			.as_int(ty)
			.map_or(IntStorage::Default, |int| int.storage);
		let mut steps = StepFlags::default();
		let interval = if op.is_right_associative() {
			let mut iter = sets.iter().rev();
			let mut acc = iter.next().expect("an operand").clone();
			for base in iter {
				let value = self.step(op, base, &acc, compute, storage, &mut steps);
				acc = IntervalSet::of(value);
			}
			acc.hull
		} else {
			let mut iter = sets.iter();
			let mut acc = iter.next().expect("an operand").clone();
			for right in iter {
				let value = self.step(op, &acc, right, compute, storage, &mut steps);
				acc = IntervalSet::of(value);
			}
			acc.hull
		};
		if operands
			.iter()
			.all(|operand| self.is_number_literal(operand))
		{
			steps.flags |= NodeFlags::CONST_FOLDED;
		}
		self.finish_arith(n, interval, compute, ty, steps);
	}

	/// Records the result of an integer operation node and reports its certain failures.
	fn finish_arith(
		&mut self,
		n: NodeId,
		interval: Interval,
		compute: ComputeType,
		ty: TypeId,
		steps: StepFlags,
	) {
		self.set_flag(n, steps.flags & !NodeFlags::CAN_FAIL);
		// the errors of a divisor or an exponent not proven (errors.rs)
		if steps.flags.contains(NodeFlags::DIVISOR_CHECK) {
			let error = self.builtin_error("DivisionByZero");
			self.can_fail(n, error);
		}
		if steps.flags.contains(NodeFlags::EXPONENT_CHECK) {
			let error = self.builtin_error("ArithmeticError");
			self.can_fail(n, error);
		}
		// a root until its parent (checked after it) says otherwise
		self.a.arith.insert(
			n,
			ArithInfo {
				interval,
				compute,
				root: true,
			},
		);
		if let Some(range) = self.s.types.interval_of(ty) {
			self.record_interval(n, interval, range);
		}
		if steps.zero_divisor {
			let diagnostic = self.diagnostic(
				&E_NUM_12,
				n,
				"this divides by zero: the divisor is always 0",
			);
			self.report_flow(diagnostic);
		} else if steps.negative_exponent {
			let diagnostic = self.diagnostic(
				&E_NUM_10,
				n,
				"the exponent is always negative: an integer power needs an exponent of 0 or more",
			);
			self.report_flow(diagnostic);
		} else if steps.certain_overflow {
			let message = format!(
				"this always overflows: it is {}, beyond the `{}` it is computed in",
				describe(interval),
				compute_name(compute)
			);
			let diagnostic = self.diagnostic(&E_NUM_10, n, message);
			self.report_flow(diagnostic);
		}
	}

	/// One step `a op b` of an integer operation: its interval, and the check
	/// bits it needs.
	fn step(
		&mut self,
		op: ArithOp,
		a: &IntervalSet,
		b: &IntervalSet,
		compute: ComputeType,
		storage: IntStorage,
		steps: &mut StepFlags,
	) -> Interval {
		let c = compute.range();
		let value = match op {
			ArithOp::Add => a.hull + b.hull,
			ArithOp::Subtract => a.hull - b.hull,
			ArithOp::Multiply => a.hull * b.hull,
			ArithOp::IntegerDivide | ArithOp::Remainder => {
				// exactly 0: `E-NUM-12`, not a check
				if b.contains(I256::ZERO) && b.as_exact().is_none() {
					steps.flags |= NodeFlags::DIVISOR_CHECK;
				}
				let signed = compute != ComputeType::UInt128;
				if signed && a.contains(c.lo) && b.contains(I256::MINUS_ONE) {
					steps.flags |= NodeFlags::MIN_DIV_GUARD;
				}
				// both split the divisor at 0 (excluded)
				let result = if op == ArithOp::IntegerDivide {
					a.hull.floor_div(b.hull)
				} else {
					a.hull.modulo(b.hull)
				};
				match result {
					Some(result) => result,
					None => {
						steps.zero_divisor = true;
						return c;
					}
				}
			}
			ArithOp::Power => {
				if b.hull.hi < I256::ZERO {
					steps.negative_exponent = true;
					return c;
				}
				if b.hull.lo < I256::ZERO {
					steps.flags |= NodeFlags::EXPONENT_CHECK;
				}
				let exponent = b
					.hull
					.intersect(Interval::new(I256::ZERO, crate::types::int::INFINITY))
					.expect("a non-negative part");
				a.hull.pow(exponent).unwrap_or(c)
			}
			ArithOp::Divide => Interval::full(),
		};
		match value.fit(c) {
			Fit::Contained => {}
			Fit::Partial => steps.flags |= NodeFlags::OVERFLOW_NATIVE,
			Fit::Disjoint => {
				steps.flags |= NodeFlags::OVERFLOW_NATIVE;
				steps.certain_overflow = true;
			}
		}
		if !storage.is_js_bigint() && !js_safe_range().contains_interval(value) {
			steps.flags |= NodeFlags::OVERFLOW_JS;
		}
		value
	}

	/// `//` and `%%` on `BigInt`: only a literal proves the divisor non-zero (values brief §6.11).
	fn big_int_divisor(&mut self, n: NodeId, op: ArithOp, operands: NodeList<'_>) {
		self.clear_flag(n, NodeFlags::DIVISOR_CHECK | NodeFlags::CAN_FAIL);
		if !matches!(op, ArithOp::IntegerDivide | ArithOp::Remainder) || !self.body.flow.reachable {
			return;
		}
		for divisor in operands.iter().skip(1) {
			let ty = self.type_of(self.strip_groups(divisor));
			let literal = match self.s.types.kind(ty) {
				TypeKind::BigIntLit(constant) => match self.s.types.const_value(constant) {
					ConstValue::BigInt(sym) => Some(self.s.types.syms.resolve(sym) == "0"),
					_ => None,
				},
				_ => None,
			};
			match literal {
				Some(true) => {
					let diagnostic =
						self.diagnostic(&E_NUM_12, n, "this divides by zero: the divisor is 0");
					self.report_flow(diagnostic);
				}
				Some(false) => {}
				None => {
					self.set_flag(n, NodeFlags::DIVISOR_CHECK);
					let error = self.builtin_error("DivisionByZero");
					self.can_fail(n, error);
				}
			}
		}
	}

	/// `-x`: the operand's type (an integer: its base type, the interval negated).
	pub(super) fn check_negate(
		&mut self,
		n: NodeId,
		value: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let expected = expected.filter(|&ty| {
			matches!(
				self.num_kind(ty),
				NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage
			)
		});
		let actual = self.check(value, expected.filter(|_| self.is_number_literal(value)));
		let ty = self.memo(n, |w| match w.num_kind(actual) {
			NumKind::Int(storage) => TypeId::of_storage(storage),
			NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage => w.s.types.widen(actual),
			NumKind::Recovering => TypeId::UNKNOWN,
			NumKind::Text | NumKind::Other => {
				let message = format!(
					"`-` doesn't apply to a `{}`: it needs a number",
					w.display(w.s.types.widen(actual))
				);
				let diagnostic = w.diagnostic(&E_NUM_18, n, message);
				w.report(diagnostic);
				TypeId::UNKNOWN
			}
		});
		if self.s.types.as_int(ty).is_some() {
			self.unary_flow(n, value, ty, |set| -set.hull);
		} else {
			self.clear_arith(n);
		}
		ty
	}

	/// The flow part of a unary integer operation (`-x`, `x.abs()`).
	fn unary_flow(
		&mut self,
		n: NodeId,
		value: NodeId,
		ty: TypeId,
		f: impl Fn(&IntervalSet) -> Interval,
	) {
		let Some(set) = self.value_set(value) else {
			self.clear_arith(n);
			return;
		};
		self.mark_inner(value);
		self.clear_flag(n, ARITH_BITS);
		if !self.body.flow.reachable {
			self.a.arith.remove(&n);
			return;
		}
		let compute = self.compute_type([value]);
		let storage = self
			.s
			.types
			.as_int(ty)
			.map_or(IntStorage::Default, |int| int.storage);
		let interval = f(&set);
		let mut steps = StepFlags::default();
		match interval.fit(compute.range()) {
			Fit::Contained => {}
			Fit::Partial => steps.flags |= NodeFlags::OVERFLOW_NATIVE,
			Fit::Disjoint => {
				steps.flags |= NodeFlags::OVERFLOW_NATIVE;
				steps.certain_overflow = true;
			}
		}
		if !storage.is_js_bigint() && !js_safe_range().contains_interval(interval) {
			steps.flags |= NodeFlags::OVERFLOW_JS;
		}
		if self.is_number_literal(value) {
			steps.flags |= NodeFlags::CONST_FOLDED;
		}
		self.finish_arith(n, interval, compute, ty, steps);
	}

	// ── compound assignment ───────────────────────────────────────────────

	/// `target op= value`: `target = target op value` with the target's path
	/// evaluated once (values brief §6.15). An integer compound assignment is
	/// an arithmetic root whose sink is the target's declared type.
	pub(super) fn check_compound_assignment(
		&mut self,
		n: NodeId,
		op: ArithOp,
		target: NodeId,
		value: NodeId,
	) -> TypeId {
		let place = self.check_place(target);
		let expected = match self.num_kind(place.ty) {
			NumKind::Int(_) | NumKind::Float(_) | NumKind::BigInt => {
				Some(self.s.types.widen(place.ty))
			}
			_ => None,
		};
		let literal = self.is_number_literal(value);
		let int_place = matches!(self.num_kind(place.ty), NumKind::Int(_));
		let actual = self.check(value, expected.filter(|_| literal && !int_place));
		self.memo(n, |w| {
			w.check_assignable(target, &place);
			w.compound_type(n, op, (target, value), &place, actual);
			TypeId::VOID
		});
		self.compound_flow(n, op, &place, value, actual);
		TypeId::VOID
	}

	/// The type part of a compound assignment: the result of `target op value`
	/// must be accepted by the target (`E-NUM-21`).
	fn compound_type(
		&mut self,
		n: NodeId,
		op: ArithOp,
		(target, value): (NodeId, NodeId),
		place: &PlaceInfo,
		actual: TypeId,
	) {
		let (left, right) = (self.num_kind(place.ty), self.num_kind(actual));
		// `text += value` joins strings (strings.rs)
		if op == ArithOp::Add && left == NumKind::Text {
			self.string_concatenation(&[(target, place.ty), (value, actual)]);
			return;
		}
		if left == NumKind::Recovering || right == NumKind::Recovering {
			return;
		}
		let result = match (left, right) {
			(NumKind::Int(_), NumKind::Int(_)) if op == ArithOp::Divide => TypeId::FLOAT,
			(NumKind::Int(_), NumKind::Int(_)) => return,
			_ => match self.combine(op, left, right) {
				Ok((_, ty)) => ty,
				Err(()) => {
					self.report_kind_mix(n, op, place.ty, actual, n);
					return;
				}
			},
		};
		if self
			.s
			.types
			.assignable(result, place.ty, crate::types::Mode::Copy)
			.is_err()
		{
			let message = format!(
				"`{}=` gives a `{}`, which a `{}` can't hold: write `x = x {} …` into a `{}` value",
				op.symbol(),
				self.display(result),
				self.display(place.ty),
				op.symbol(),
				self.display(result)
			);
			let diagnostic = self.diagnostic(&E_NUM_21, n, message);
			self.report(diagnostic);
		}
	}

	/// The flow part of a compound assignment: interval, check bits, the sink
	/// check against the target's type, the new fact of the target.
	fn compound_flow(
		&mut self,
		n: NodeId,
		op: ArithOp,
		place: &PlaceInfo,
		value: NodeId,
		actual: TypeId,
	) {
		let int = match (self.num_kind(place.ty), self.num_kind(actual)) {
			(NumKind::Int(_), NumKind::Int(_)) => op != ArithOp::Divide,
			_ => false,
		};
		self.a.coercions.remove(&n);
		self.clear_flag(n, ARITH_BITS | NodeFlags::SINK_CHECK);
		let symbol = place.symbol.filter(|_| place.assignable);
		if !int || !self.body.flow.reachable {
			self.a.arith.remove(&n);
			if let Some(symbol) = symbol
				&& self.body.flow.reachable
			{
				self.set_symbol_values(symbol, None);
			}
			return;
		}
		let current = match symbol {
			Some(symbol) => self.symbol_values(symbol),
			None => self.s.types.interval_of(place.ty).map(IntervalSet::of),
		};
		let (Some(current), Some(right)) = (current, self.value_set(value)) else {
			return;
		};
		self.mark_inner(value);
		let target_storage = self
			.s
			.types
			.as_int(place.ty)
			.map_or(IntStorage::Default, |int| int.storage);
		let compute = wider(
			ComputeType::of([target_storage]),
			self.compute_type([value]),
		);
		let mut steps = StepFlags::default();
		let interval = self.step(op, &current, &right, compute, target_storage, &mut steps);
		self.finish_arith(n, interval, compute, place.ty, steps);
		// the sink: the target's declared type, for the values that pass the compute type
		let range = self
			.s
			.types
			.interval_of(place.ty)
			.expect("an integer target");
		let Some(checked) = interval.intersect(compute.range()) else {
			// a certain overflow, reported at the node
			if let Some(symbol) = symbol {
				self.set_symbol_values(symbol, None);
			}
			return;
		};
		match checked.fit(range) {
			Fit::Contained => {}
			Fit::Partial => {
				self.set_flag(n, NodeFlags::SINK_CHECK);
				self.a.coercions.insert(n, place.ty);
			}
			Fit::Disjoint => {
				let message = format!(
					"the result is {}, which doesn't fit in a `{}` ({})",
					describe(interval),
					self.display(place.ty),
					describe(range)
				);
				let diagnostic = self.diagnostic(&E_NUM_10, n, message);
				self.report_flow(diagnostic);
			}
		}
		if let Some(symbol) = symbol {
			self.set_symbol_values(symbol, Some(IntervalSet::of(checked)));
		}
	}

	// ── conversions and number methods ────────────────────────────────────

	/// `x.abs()`, handled here instead of `calls.rs`; `None` when the call is
	/// something else.
	pub(crate) fn check_number_method(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, callee) {
			View::Access { target, members } if members.len() == 1 && arguments.is_empty() => {
				let member = members.first()?;
				let View::Name(name) = syntax::view(&tree, member) else {
					return None;
				};
				(tree.name(name) == "abs").then(|| self.check_abs(n, target))
			}
			_ => None,
		}
	}

	/// `T(value)` for a number type `T` (values brief §6.4), called by
	/// `calls.rs` at every visit once `value` (of type `actual`) is checked:
	/// never fails, or `CONVERSION_CHECK` (+ `CAN_FAIL` from an integer)
	/// when the value isn't proven to fit, or `E-NUM-10` when it can't fit;
	/// `E-NUM-16` when the conversion doesn't exist, `W-NUM-1` when it is useless.
	pub(super) fn check_conversion(
		&mut self,
		n: NodeId,
		argument: NodeId,
		source: TypeId,
		target: TypeId,
	) {
		self.clear_flag(n, NodeFlags::CONVERSION_CHECK | NodeFlags::CAN_FAIL);
		self.a.intervals.remove(&n);
		if !self.conversion_diagnostics(n, target, argument, source) {
			return;
		}
		let ty = target;
		let Some(range) = self.s.types.interval_of(ty) else {
			return;
		};
		if !self.body.flow.reachable {
			return;
		}
		match self.num_kind(source) {
			NumKind::Int(_) => {
				let Some(set) = self.value_set(argument) else {
					return;
				};
				match set.restrict(range) {
					None => {
						let message = format!(
							"this conversion always fails: the value is {}, outside of `{}` ({})",
							describe(set.hull),
							self.display(ty),
							describe(range)
						);
						let diagnostic = self.diagnostic(&E_NUM_10, n, message);
						self.report_flow(diagnostic);
					}
					Some(inside) => {
						if !range.contains_interval(set.hull) {
							self.set_flag(n, NodeFlags::CONVERSION_CHECK);
							let error = self.builtin_error("ArithmeticError");
							self.can_fail(n, error);
						}
						self.record_interval(n, inside.hull, range);
					}
				}
			}
			NumKind::BigInt => {
				let exact = match self.s.types.kind(source) {
					TypeKind::BigIntLit(constant) => match self.s.types.const_value(constant) {
						ConstValue::BigInt(sym) => {
							self.s.types.syms.resolve(sym).parse::<I256>().ok()
						}
						_ => None,
					},
					_ => None,
				};
				match exact {
					Some(value) if range.contains(value) => {
						self.record_interval(n, Interval::exact(value), range)
					}
					Some(value) => {
						let message = format!(
							"this conversion always fails: {value} is outside of `{}` ({})",
							self.display(ty),
							describe(range)
						);
						let diagnostic = self.diagnostic(&E_NUM_10, n, message);
						self.report_flow(diagnostic);
					}
					None => {
						self.set_flag(n, NodeFlags::CONVERSION_CHECK);
						let error = self.builtin_error("ArithmeticError");
						self.can_fail(n, error);
					}
				}
			}
			NumKind::Float(_) => {
				// a stop when NaN or out of range (decisions.md), unless a literal fits
				let literal = match self.s.types.kind(source) {
					TypeKind::FloatLit(_, constant) => match self.s.types.const_value(constant) {
						ConstValue::Float(bits) => Some(f64::from_bits(bits).trunc()),
						_ => None,
					},
					_ => None,
				};
				let lo = range.lo.as_f64();
				let hi = range.hi.as_f64();
				match literal {
					Some(value) if value >= lo && value <= hi => {
						self.record_interval(n, Interval::exact(I256::from(value as i128)), range)
					}
					Some(_) => {
						let message = format!(
							"this conversion always fails: the value is outside of `{}` ({})",
							self.display(ty),
							describe(range)
						);
						let diagnostic = self.diagnostic(&E_NUM_10, n, message);
						self.report_flow(diagnostic);
					}
					None => self.set_flag(n, NodeFlags::CONVERSION_CHECK),
				}
			}
			_ => {}
		}
	}

	/// The diagnostics of a conversion (each visit, buffered in loops): `E-NUM-16` when it doesn't exist,
	/// `W-NUM-1` when it is useless.
	/// `E-NUM-16`, `W-NUM-1`; whether the conversion exists.
	fn conversion_diagnostics(
		&mut self,
		n: NodeId,
		target: TypeId,
		argument: NodeId,
		source: TypeId,
	) -> bool {
		use NumKind as K;
		let (to, from) = (self.num_kind(target), self.num_kind(source));
		if to == K::Recovering || from == K::Recovering {
			return false;
		}
		let exists = matches!(
			(to, from),
			(K::Int(_), K::Int(_) | K::BigInt | K::Float(_))
				| (
					K::Float(_),
					K::Int(_) | K::Float(_) | K::Percentage | K::BigInt
				) | (K::BigInt, K::Int(_) | K::BigInt)
				| (K::Percentage, K::Int(_) | K::Float(_))
		) || (matches!(to, K::Int(_))
			&& matches!(
				self.s.types.kind(source),
				TypeKind::Enum(..) | TypeKind::EnumMember(..)
			));

		if !exists {
			let hint = match (to, from) {
				(K::BigInt, K::Float(_)) => ": round it first, ex: `BigInt(Integer.round(x))`",
				(K::Int(_), K::Percentage) => ": convert it to a `Float` first, ex: `Float(p)`",
				_ => "",
			};
			let message = format!(
				"there is no conversion from `{}` to `{}`{hint}",
				self.display(self.s.types.widen(source)),
				self.display(target)
			);
			let diagnostic = self.diagnostic(&E_NUM_16, n, message);
			self.report_flow(diagnostic);
			return false;
		}
		let useless = match (to, from) {
			_ if self.is_number_literal(argument) => false,
			(K::Int(a), K::Int(b)) => {
				let source_range = self.s.types.interval_of(source).expect("an integer");
				let target_range = self.s.types.interval_of(target).expect("an integer");
				target_range.contains_interval(source_range) && a.is_js_bigint() == b.is_js_bigint()
			}
			(K::Float(a), K::Float(b)) => a == b,
			(K::BigInt, K::BigInt) => true,
			_ => false,
		};
		if useless {
			let message = format!(
				"this conversion does nothing: a `{}` is already accepted where a `{}` is expected",
				self.display(source),
				self.display(target)
			);
			let diagnostic = self.diagnostic(&W_NUM_1, n, message);
			self.report_flow(diagnostic);
		}
		true
	}

	/// `x.abs()`: the receiver's type; for an integer, an arithmetic node
	/// (values brief §6.5, D-89: `Int8` `-128` gives 128, checked at the sink).
	fn check_abs(&mut self, n: NodeId, target: NodeId) -> TypeId {
		let actual = self.check(target, None);
		let ty = self.memo(n, |w| match w.num_kind(actual) {
			NumKind::Int(storage) => TypeId::of_storage(storage),
			NumKind::Float(_) | NumKind::BigInt | NumKind::Percentage => w.s.types.widen(actual),
			NumKind::Recovering => TypeId::UNKNOWN,
			NumKind::Text | NumKind::Other => {
				let message = format!(
					"`abs` doesn't apply to a `{}`: it needs a number",
					w.display(w.s.types.widen(actual))
				);
				let diagnostic = w.diagnostic(&AN001, n, message);
				w.report(diagnostic);
				TypeId::UNKNOWN
			}
		});
		if self.s.types.as_int(ty).is_some() {
			self.unary_flow(n, target, ty, |set| set.hull.abs());
		}
		ty
	}
}

/// The larger of two compute types.
fn wider(a: ComputeType, b: ComputeType) -> ComputeType {
	let rank = |compute: ComputeType| match compute {
		ComputeType::Int64 => 0,
		ComputeType::Int128 => 1,
		ComputeType::UInt128 => 2,
	};
	if rank(b) > rank(a) { b } else { a }
}

fn compute_name(compute: ComputeType) -> &'static str {
	match compute {
		ComputeType::Int64 => "Int64",
		ComputeType::Int128 => "Int128",
		ComputeType::UInt128 => "UInt128",
	}
}

// ── index bounds: hook called by `objects.rs` / `expr.rs` (minimal version written by E1) ──

impl Walker<'_> {
	/// The bounds of a range `a..b` (values brief §12, D-41): integers
	/// accepted by `Integer` (`E-RNG-2` for floats, strings…). Its type is
	/// `Range`; the slicing and iteration rules are with their users.
	pub(super) fn check_range_bounds(
		&mut self,
		n: NodeId,
		start: Option<NodeId>,
		end: Option<NodeId>,
	) -> TypeId {
		let mut wrong = None;
		for bound in [start, end].into_iter().flatten() {
			let ty = self.check(bound, Some(TypeId::INTEGER));
			match self.num_kind(ty) {
				NumKind::Int(_) => self.sink(bound, ty, TypeId::INTEGER),
				NumKind::Recovering => {}
				_ => wrong = wrong.or(Some((bound, ty))),
			}
		}
		self.memo(n, |w| {
			if let Some((bound, ty)) = wrong {
				let message = format!(
					"the bounds of a range are integers, not a `{}`",
					w.display(w.s.types.widen(ty))
				);
				let diagnostic =
					w.diagnostic(&crate::diagnostics::codes::values::E_RNG_2, bound, message);
				w.report(diagnostic);
			}
			TypeId::RANGE
		})
	}
}
