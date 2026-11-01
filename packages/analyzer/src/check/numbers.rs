//! Numbers: float and big integer literals, arithmetic trees, intervals and
//! overflow bits, compound assignments, integer sinks (design §3.6).

use crate::check::Walker;
use crate::diagnostics::codes::values::{E_NUM_10, E_NUM_14};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{ArithOp, NodeList};

impl Walker<'_> {
	/// A float literal.
	pub(super) fn check_float(
		&mut self,
		n: NodeId,
		_value: f64,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "float literals")
	}

	/// A big integer literal.
	pub(super) fn check_big_int(&mut self, n: NodeId, _text: &str) -> TypeId {
		self.unsupported(n, "big integers")
	}

	/// `+ - * / // %% **`: the operands fold left, `**` right (see `syntax::View::Arithmetic`).
	pub(super) fn check_arithmetic(
		&mut self,
		n: NodeId,
		_op: ArithOp,
		_operands: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "arithmetic")
	}

	/// `-x`
	pub(super) fn check_negate(
		&mut self,
		n: NodeId,
		_value: NodeId,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.unsupported(n, "negation")
	}

	/// `50%`
	pub(super) fn check_percentage(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "percentages")
	}

	/// `target += value` … Minimal for now: an integer place, a mutable target,
	/// the value checked against the target type. Intervals and overflow bits: E2.
	pub(super) fn check_compound_assignment(
		&mut self,
		n: NodeId,
		op: ArithOp,
		target: NodeId,
		value: NodeId,
	) -> TypeId {
		let place = self.check_place(target);
		let integer = self.s.types.as_int(place.ty).is_some();
		let expected = integer.then_some(place.ty);
		let actual = self.check(value, expected);
		if integer && self.s.types.as_int(actual).is_none() {
			self.sink(value, actual, place.ty);
		}
		self.memo(n, |w| {
			w.check_assignable(target, &place);
			let recovering = w.s.types.is_recovering(place.ty) || w.s.types.is_recovering(actual);
			if !integer && !recovering {
				let what = format!("`{}` on a `{}`", op.symbol(), w.display(place.ty));
				return w.unsupported_here(n, &what);
			}
			TypeId::VOID
		})
	}

	/// `AN001` for the node itself, without walking its children again.
	fn unsupported_here(&mut self, n: NodeId, what: &str) -> TypeId {
		let message = format!("{what} is not supported yet by the analyzer");
		let diagnostic = self.diagnostic(&crate::diagnostics::codes::analyzer::AN001, n, message);
		self.report(diagnostic);
		TypeId::VOID
	}

	/// An integer value whose type's range is not contained in its sink's
	/// (called by `sink`). Minimal: `E-NUM-10` for a literal out of range,
	/// `E-NUM-14` otherwise. Intervals, `SINK_CHECK` and coercions: E2.
	pub(super) fn sink_integer(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		let literal = self.s.types.as_int(actual).and_then(|int| int.literal());
		let (code, message) = match literal {
			Some(value) => (
				&E_NUM_10,
				format!("{value} doesn't fit in a `{}`", self.display(target)),
			),
			None => (
				&E_NUM_14,
				format!(
					"a `{}` may not fit in a `{}`: check its range first, or convert it",
					self.display(actual),
					self.display(target)
				),
			),
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
	}
}

// ── conversions: hook called by `calls.rs` (minimal version written by E1) ──

impl Walker<'_> {
	/// `Int8(x)`, `Integer(f)`, `Float(n)`: the flow part of a conversion of
	/// `value` (of type `actual`) to the number type `target`, at every visit.
	/// `calls.rs` resolves the call, checks `value` and types the call as
	/// `target`; this decides whether the conversion exists (`E-NUM-16`) and
	/// whether it can fail (`CONVERSION_CHECK` and `CAN_FAIL` on `n`, set and
	/// cleared at each visit). Minimal: the range of `actual`'s type, not yet
	/// the interval facts of `value`.
	pub(super) fn check_conversion(
		&mut self,
		n: NodeId,
		value: NodeId,
		actual: TypeId,
		target: TypeId,
	) {
		use crate::analysis::NodeFlags;
		use crate::diagnostics::codes::values::E_NUM_16;
		use crate::types::TypeKind;

		self.clear_flag(n, NodeFlags::CONVERSION_CHECK | NodeFlags::CAN_FAIL);
		if self.s.types.is_recovering(actual) {
			return;
		}
		let types = &self.s.types;
		let exists = matches!(
			(types.kind(actual), types.kind(target)),
			(
				TypeKind::Int(_) | TypeKind::Float(_) | TypeKind::FloatLit(..),
				TypeKind::Int(_) | TypeKind::Float(_)
			) | (
				TypeKind::Int(_) | TypeKind::BigInt | TypeKind::BigIntLit(_),
				TypeKind::BigInt | TypeKind::Int(_)
			)
		);
		if !exists {
			let message = format!(
				"there is no conversion from `{}` to `{}`",
				self.display(actual),
				self.display(target)
			);
			let diagnostic = self.diagnostic(&E_NUM_16, value, message);
			self.report_flow(diagnostic);
			return;
		}
		let fits = match (types.as_int(actual), types.as_int(target)) {
			(Some(from), Some(to)) => to.interval().contains_interval(from.interval()),
			(Some(_), None) => true,
			_ => false,
		};
		if !fits {
			self.set_flag(n, NodeFlags::CONVERSION_CHECK | NodeFlags::CAN_FAIL);
		}
	}
}
