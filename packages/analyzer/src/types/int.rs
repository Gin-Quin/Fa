//! Integer types and interval arithmetic (values brief §6.4–§6.7).
//!
//! Bounds are `I256` (decision A8). A bound at or beyond ±2^250 stands for
//! ±∞ ([`INFINITY`]): every operation saturates there, and treats such a
//! bound as infinite. Sums and products of 64-bit bounds are exact; products
//! of 128-bit bounds (up to 2^254) saturate, which is sound since ±∞ lies
//! beyond every compute type.
//!
//! The operations implement the normative table of values brief §6.5. One
//! deviation, for soundness: `a ** b` with `0 ≤ a1` takes the hull of the four
//! corners, which is the table's `[a1^b1, a2^b2]` except when `a1 = 0`,
//! `b1 = 0 < b2`, where the table's lower bound `1` is wrong (`0 ** 1 = 0`).

use std::ops::{Add, Mul, Neg, Sub};

use ethnum::I256;
use smallvec::SmallVec;

/// The bound standing for +∞ (−∞ is its negation): 2^250.
pub const INFINITY: I256 = I256::from_words(1 << 122, 0);

/// The largest length of a collection or a string, 2^48 (decision
/// "Collection lengths are at most 2^48"): no machine addresses more, so
/// every length, index and count starts from `0..=LENGTH_MAX`, and sums and
/// products of a few of them are exact in 64 bits.
pub const LENGTH_MAX: I256 = I256::from_words(0, 1 << 48);

/// Clamps a value to `[-INFINITY, INFINITY]`.
#[inline]
pub fn saturate(value: I256) -> I256 {
	value.clamp(-INFINITY, INFINITY)
}

#[inline]
fn is_infinite(value: I256) -> bool {
	value >= INFINITY || value <= -INFINITY
}

#[inline]
fn signum(value: I256) -> i32 {
	value.signum().as_i32()
}

/// `2^bits - 1` and friends, as `I256`.
fn pow2(bits: u32) -> I256 {
	I256::ONE << bits
}

/// The storage of an integer type: its native width and JavaScript class
/// (types brief D2). `Integer` is `Default`: the range of `Int64`, a
/// JavaScript number; `Int64` written explicitly is `I64`, a `bigint`.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub enum IntStorage {
	/// `Integer`.
	Default,
	I8,
	I16,
	I32,
	I64,
	I128,
	U8,
	U16,
	U32,
	U64,
	U128,
}

impl IntStorage {
	/// Every storage.
	pub const ALL: [IntStorage; 11] = [
		IntStorage::Default,
		IntStorage::I8,
		IntStorage::I16,
		IntStorage::I32,
		IntStorage::I64,
		IntStorage::I128,
		IntStorage::U8,
		IntStorage::U16,
		IntStorage::U32,
		IntStorage::U64,
		IntStorage::U128,
	];

	/// The type name of the full range.
	pub fn name(self) -> &'static str {
		match self {
			IntStorage::Default => "Integer",
			IntStorage::I8 => "Int8",
			IntStorage::I16 => "Int16",
			IntStorage::I32 => "Int32",
			IntStorage::I64 => "Int64",
			IntStorage::I128 => "Int128",
			IntStorage::U8 => "UInt8",
			IntStorage::U16 => "UInt16",
			IntStorage::U32 => "UInt32",
			IntStorage::U64 => "UInt64",
			IntStorage::U128 => "UInt128",
		}
	}

	/// The width in bits.
	pub fn bits(self) -> u32 {
		match self {
			IntStorage::I8 | IntStorage::U8 => 8,
			IntStorage::I16 | IntStorage::U16 => 16,
			IntStorage::I32 | IntStorage::U32 => 32,
			IntStorage::Default | IntStorage::I64 | IntStorage::U64 => 64,
			IntStorage::I128 | IntStorage::U128 => 128,
		}
	}

	/// Whether the storage is signed.
	pub fn is_signed(self) -> bool {
		!matches!(
			self,
			IntStorage::U8 | IntStorage::U16 | IntStorage::U32 | IntStorage::U64 | IntStorage::U128
		)
	}

	/// Whether values are JavaScript `bigint`s (64 bits and wider, written explicitly).
	pub fn is_js_bigint(self) -> bool {
		matches!(
			self,
			IntStorage::I64 | IntStorage::U64 | IntStorage::I128 | IntStorage::U128
		)
	}

	/// The full range of the storage.
	pub fn range(self) -> Interval {
		let bits = self.bits();
		if self.is_signed() {
			Interval::new(-pow2(bits - 1), pow2(bits - 1) - 1)
		} else {
			Interval::new(I256::ZERO, pow2(bits) - 1)
		}
	}

	/// The base type of an operation mixing two storages (values brief §6.4):
	/// the one whose range contains the other; `Int64` wins over `Integer`;
	/// `None` when neither contains the other (`E-NUM-15` at a sink without
	/// expected type).
	pub fn common_base(self, other: IntStorage) -> Option<IntStorage> {
		if self == other {
			return Some(self);
		}
		let (a, b) = (self.range(), other.range());
		match (a.contains_interval(b), b.contains_interval(a)) {
			(true, true) => Some(if self == IntStorage::Default {
				other
			} else {
				self
			}),
			(true, false) => Some(self),
			(false, true) => Some(other),
			(false, false) => None,
		}
	}
}

/// An integer type: a range and a storage. A literal type has `min == max`.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct IntType {
	/// The lowest value.
	pub min: I256,
	/// The highest value.
	pub max: I256,
	/// The native width and JavaScript class.
	pub storage: IntStorage,
}

impl IntType {
	/// The full range of a storage: `Integer`, `Int8`…
	pub fn full(storage: IntStorage) -> Self {
		let range = storage.range();
		Self {
			min: range.lo,
			max: range.hi,
			storage,
		}
	}

	/// The interval of the type (its bounds are finite: no saturation).
	pub fn interval(&self) -> Interval {
		Interval::saturated(self.min, self.max)
	}

	/// The value of a literal type.
	pub fn literal(&self) -> Option<I256> {
		(self.min == self.max).then_some(self.min)
	}

	/// Whether it is the full range of its storage.
	pub fn is_full(&self) -> bool {
		self.interval() == self.storage.range()
	}
}

/// How an interval relates to a target range (values brief §6.4, §6.7).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Fit {
	/// Every value fits: no check.
	Contained,
	/// Some values may not fit: a runtime check, or `E-NUM-14` for a non-arithmetic value.
	Partial,
	/// No value fits: a certain overflow, `E-NUM-10`.
	Disjoint,
}

/// A closed interval of mathematical integers, `lo <= hi`. A bound at
/// ±[`INFINITY`] is unbounded.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Interval {
	/// The lowest value.
	pub lo: I256,
	/// The highest value.
	pub hi: I256,
}

/// Which way an ambiguous bound is rounded: towards −∞ for a lower bound.
#[derive(Copy, Clone, Eq, PartialEq)]
enum Round {
	Down,
	Up,
}

impl Round {
	fn infinity(self) -> I256 {
		match self {
			Round::Down => -INFINITY,
			Round::Up => INFINITY,
		}
	}
}

fn add_bound(a: I256, b: I256, round: Round) -> I256 {
	match (is_infinite(a), is_infinite(b)) {
		(false, false) => saturate(a + b),
		(true, false) => saturate(a),
		(false, true) => saturate(b),
		(true, true) if signum(a) == signum(b) => saturate(a),
		(true, true) => round.infinity(),
	}
}

fn mul_bound(a: I256, b: I256) -> I256 {
	if a == I256::ZERO || b == I256::ZERO {
		return I256::ZERO;
	}
	let sign = signum(a) * signum(b);
	if is_infinite(a) || is_infinite(b) {
		return if sign > 0 { INFINITY } else { -INFINITY };
	}
	match a.checked_mul(b) {
		Some(product) => saturate(product),
		None if sign > 0 => INFINITY,
		None => -INFINITY,
	}
}

/// `⌊a / b⌋` for `b ≠ 0`, with infinities. Pushes one or more candidates.
fn floor_div_bound(a: I256, b: I256, out: &mut SmallVec<[I256; 8]>) {
	debug_assert!(b != I256::ZERO);
	match (is_infinite(a), is_infinite(b)) {
		(false, false) => {
			let quotient = a / b;
			let adjust = (a % b != I256::ZERO) && ((a < I256::ZERO) != (b < I256::ZERO));
			out.push(if adjust { quotient - 1 } else { quotient });
		}
		(true, false) => out.push(if signum(a) * signum(b) > 0 {
			INFINITY
		} else {
			-INFINITY
		}),
		(false, true) => {
			// a / ±∞ tends to 0 from the side of the sign of a / b.
			let same_sign = (a >= I256::ZERO) == (b > I256::ZERO);
			out.push(if a == I256::ZERO || same_sign {
				I256::ZERO
			} else {
				I256::MINUS_ONE
			});
		}
		(true, true) => {
			// ∞ / ∞ is any value of that sign: cover both ends.
			let positive = signum(a) * signum(b) > 0;
			out.extend([
				I256::ZERO,
				I256::MINUS_ONE,
				if positive { INFINITY } else { -INFINITY },
			]);
		}
	}
}

/// `base ^ exponent` for `base >= 0` and `exponent >= 0`, with `0^0 = 1`.
fn pow_bound(base: I256, exponent: I256) -> I256 {
	debug_assert!(base >= I256::ZERO && exponent >= I256::ZERO);
	if exponent == I256::ZERO {
		return I256::ONE;
	}
	if base <= I256::ONE {
		return base;
	}
	if is_infinite(base) || is_infinite(exponent) {
		return INFINITY;
	}
	let mut result = I256::ONE;
	let mut factor = base;
	let mut exponent = exponent;
	loop {
		if exponent & I256::ONE == I256::ONE {
			result = mul_bound(result, factor);
		}
		exponent >>= 1;
		if exponent == I256::ZERO || result >= INFINITY {
			return result;
		}
		factor = mul_bound(factor, factor);
	}
}

impl Interval {
	/// The interval `[lo, hi]`, saturated. Panics in debug builds when `lo > hi`.
	pub fn new(lo: I256, hi: I256) -> Self {
		debug_assert!(lo <= hi, "empty interval [{lo}, {hi}]");
		Self {
			lo: saturate(lo),
			hi: saturate(hi),
		}
	}

	/// The interval `[lo, hi]` of bounds already within ±[`INFINITY`] (the
	/// bounds of other intervals, of a type): [`Interval::new`] without the
	/// clamping, which the hot paths of the flow analysis would repeat.
	#[inline]
	fn saturated(lo: I256, hi: I256) -> Self {
		debug_assert!(lo <= hi, "empty interval [{lo}, {hi}]");
		debug_assert!(saturate(lo) == lo && saturate(hi) == hi);
		Self { lo, hi }
	}

	/// The interval `[lo, hi]` from `i128` bounds.
	pub fn from_i128(lo: i128, hi: i128) -> Self {
		Self::new(I256::from(lo), I256::from(hi))
	}

	/// The single value `value`.
	pub fn exact(value: I256) -> Self {
		Self::new(value, value)
	}

	/// `[−∞, +∞]`.
	pub fn full() -> Self {
		Self::new(-INFINITY, INFINITY)
	}

	/// The hull of a list of candidate bounds.
	fn hull_of(values: &[I256]) -> Self {
		let lo = values.iter().copied().min().expect("no candidate bound");
		let hi = values.iter().copied().max().expect("no candidate bound");
		Self::new(lo, hi)
	}

	/// The value of a single-value interval.
	pub fn as_exact(self) -> Option<I256> {
		(self.lo == self.hi && !is_infinite(self.lo)).then_some(self.lo)
	}

	/// Whether the lower bound is −∞.
	pub fn is_unbounded_below(self) -> bool {
		self.lo <= -INFINITY
	}

	/// Whether the upper bound is +∞.
	pub fn is_unbounded_above(self) -> bool {
		self.hi >= INFINITY
	}

	/// Whether `value` is in the interval.
	pub fn contains(self, value: I256) -> bool {
		self.lo <= value && value <= self.hi
	}

	/// Whether every value of `other` is in the interval.
	pub fn contains_interval(self, other: Interval) -> bool {
		self.lo <= other.lo && other.hi <= self.hi
	}

	/// Whether the intervals share no value.
	pub fn is_disjoint(self, other: Interval) -> bool {
		self.hi < other.lo || other.hi < self.lo
	}

	/// Whether the intervals overlap or are adjacent (`[0, 5]` and `[6, 9]`).
	pub fn touches(self, other: Interval) -> bool {
		!(add_bound(self.hi, I256::ONE, Round::Up) < other.lo
			|| add_bound(other.hi, I256::ONE, Round::Up) < self.lo)
	}

	/// The common values.
	pub fn intersect(self, other: Interval) -> Option<Interval> {
		let lo = self.lo.max(other.lo);
		let hi = self.hi.min(other.hi);
		(lo <= hi).then(|| Interval::saturated(lo, hi))
	}

	/// The smallest interval containing both.
	pub fn hull(self, other: Interval) -> Interval {
		Interval::saturated(self.lo.min(other.lo), self.hi.max(other.hi))
	}

	/// How the interval fits a target range.
	pub fn fit(self, target: Interval) -> Fit {
		if target.contains_interval(self) {
			Fit::Contained
		} else if self.is_disjoint(target) {
			Fit::Disjoint
		} else {
			Fit::Partial
		}
	}

	/// The part below zero and the part above zero, zero excluded.
	pub fn split_signs(self) -> (Option<Interval>, Option<Interval>) {
		let negative = self.intersect(Interval::new(-INFINITY, I256::MINUS_ONE));
		let positive = self.intersect(Interval::new(I256::ONE, INFINITY));
		(negative, positive)
	}

	/// `a // b` (floor division): `b` split into its negative and positive
	/// parts, min/max of `⌊ai / bj⌋` over the corners of each part, hull of
	/// the parts. `None` when `b` is exactly zero (`E-NUM-12`).
	pub fn floor_div(self, b: Interval) -> Option<Interval> {
		let (negative, positive) = b.split_signs();
		let mut candidates = SmallVec::<[I256; 8]>::new();
		for part in [negative, positive].into_iter().flatten() {
			for numerator in [self.lo, self.hi] {
				for denominator in [part.lo, part.hi] {
					floor_div_bound(numerator, denominator, &mut candidates);
				}
			}
		}
		(!candidates.is_empty()).then(|| Self::hull_of(&candidates))
	}

	/// `a %% b` (remainder with the sign of the divisor): `[0, b2 - 1]` for the
	/// positive part of `b`, narrowed to `a` when `0 <= a1` and `a2 < b1`;
	/// `[b1 + 1, 0]` for the negative part, narrowed to `a` when `a2 <= 0`
	/// and `b2 < a1`; hull of the parts. `None` when `b` is exactly zero.
	pub fn modulo(self, b: Interval) -> Option<Interval> {
		let (negative, positive) = b.split_signs();
		let positive = positive.map(|part| {
			if self.lo >= I256::ZERO && self.hi < part.lo {
				self
			} else {
				Interval::new(I256::ZERO, add_bound(part.hi, I256::MINUS_ONE, Round::Up))
			}
		});
		let negative = negative.map(|part| {
			if self.hi <= I256::ZERO && part.hi < self.lo {
				self
			} else {
				Interval::new(add_bound(part.lo, I256::ONE, Round::Down), I256::ZERO)
			}
		});
		match (negative, positive) {
			(Some(a), Some(b)) => Some(a.hull(b)),
			(a, b) => a.or(b),
		}
	}

	/// `a ** b` for `b1 >= 0` (`None` otherwise: the exponent check, or
	/// `E-NUM-10` when `b` is entirely negative). Both exact: the exact value.
	/// `a1 >= 0`: the hull of the corners (see the module documentation).
	/// Otherwise `[-M, M]` with `M = max(|a1|, |a2|) ^ b2`.
	pub fn pow(self, b: Interval) -> Option<Interval> {
		if b.lo < I256::ZERO {
			return None;
		}
		if self.lo >= I256::ZERO {
			return Some(Self::hull_of(&[
				pow_bound(self.lo, b.lo),
				pow_bound(self.lo, b.hi),
				pow_bound(self.hi, b.lo),
				pow_bound(self.hi, b.hi),
			]));
		}
		if let (Some(base), Some(exponent)) = (self.as_exact(), b.as_exact()) {
			let magnitude = pow_bound(base.abs(), exponent);
			let odd = exponent & I256::ONE == I256::ONE;
			return Some(Interval::exact(if odd { -magnitude } else { magnitude }));
		}
		let magnitude = pow_bound(self.lo.abs().max(self.hi.abs()), b.hi);
		Some(Interval::new(-magnitude, magnitude))
	}

	/// `x.abs()`: `[0, max(|a1|, |a2|)]`, or `[min(|a1|, |a2|), …]` when `0 ∉ a`.
	pub fn abs(self) -> Interval {
		let (low, high) = (self.lo.abs(), self.hi.abs());
		let top = low.max(high);
		if self.contains(I256::ZERO) {
			Interval::new(I256::ZERO, top)
		} else {
			Interval::new(low.min(high), top)
		}
	}

	/// `x.clamp(m, M)`: `[min(max(a1, m1), M1), min(max(a2, m2), M2)]`.
	pub fn clamp(self, min: Interval, max: Interval) -> Interval {
		let lo = self.lo.max(min.lo).min(max.lo);
		let hi = self.hi.max(min.hi).min(max.hi);
		Interval::new(lo, hi.max(lo))
	}

	/// `Math.min(a, b)`: `[min(a1, b1), min(a2, b2)]`.
	pub fn min(self, b: Interval) -> Interval {
		Interval::new(self.lo.min(b.lo), self.hi.min(b.hi))
	}

	/// `Math.max(a, b)`: `[max(a1, b1), max(a2, b2)]`.
	pub fn max(self, b: Interval) -> Interval {
		Interval::new(self.lo.max(b.lo), self.hi.max(b.hi))
	}
}

/// `a + b`: `[a1 + b1, a2 + b2]`.
impl Add for Interval {
	type Output = Interval;

	fn add(self, b: Interval) -> Interval {
		Interval::new(
			add_bound(self.lo, b.lo, Round::Down),
			add_bound(self.hi, b.hi, Round::Up),
		)
	}
}

/// `a - b`: `[a1 - b2, a2 - b1]`.
impl Sub for Interval {
	type Output = Interval;

	fn sub(self, b: Interval) -> Interval {
		self + -b
	}
}

/// `-a`: `[-a2, -a1]`.
impl Neg for Interval {
	type Output = Interval;

	fn neg(self) -> Interval {
		Interval::new(-self.hi, -self.lo)
	}
}

/// `a * b`: min/max of the four corner products.
impl Mul for Interval {
	type Output = Interval;

	fn mul(self, b: Interval) -> Interval {
		Interval::hull_of(&[
			mul_bound(self.lo, b.lo),
			mul_bound(self.lo, b.hi),
			mul_bound(self.hi, b.lo),
			mul_bound(self.hi, b.hi),
		])
	}
}

/// The type in which the nodes of an arithmetic tree are computed (values brief §6.7).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ComputeType {
	/// Every operand's storage is within `Int64`.
	Int64,
	/// An operand is a `UInt64` or an `Int128`.
	Int128,
	/// An operand is a `UInt128`: every node is checked against `UInt128`.
	UInt128,
}

impl ComputeType {
	/// The compute type of a tree whose operands have these storages.
	pub fn of(storages: impl IntoIterator<Item = IntStorage>) -> Self {
		let mut compute = ComputeType::Int64;
		for storage in storages {
			match storage {
				IntStorage::U128 => return ComputeType::UInt128,
				IntStorage::U64 | IntStorage::I128 => compute = ComputeType::Int128,
				_ => {}
			}
		}
		compute
	}

	/// The range of the compute type.
	pub fn range(self) -> Interval {
		match self {
			ComputeType::Int64 => IntStorage::I64.range(),
			ComputeType::Int128 => IntStorage::I128.range(),
			ComputeType::UInt128 => IntStorage::U128.range(),
		}
	}
}

/// The range of number-represented integers on JavaScript: ±(2^53 − 1).
pub fn js_safe_range() -> Interval {
	let max = pow2(53) - 1;
	Interval::new(-max, max)
}

#[cfg(test)]
mod tests {
	use std::ops::{Add, Mul, Neg, Sub};

	use super::*;

	fn i(lo: i128, hi: i128) -> Interval {
		Interval::from_i128(lo, hi)
	}

	fn n(value: i128) -> I256 {
		I256::from(value)
	}

	fn int8() -> Interval {
		IntStorage::I8.range()
	}

	#[test]
	fn storage_ranges() {
		assert_eq!(int8(), i(-128, 127));
		assert_eq!(IntStorage::U8.range(), i(0, 255));
		assert_eq!(IntStorage::I16.range(), i(-32768, 32767));
		assert_eq!(IntStorage::U32.range(), i(0, u32::MAX as i128));
		assert_eq!(
			IntStorage::Default.range(),
			i(i64::MIN as i128, i64::MAX as i128)
		);
		assert_eq!(IntStorage::I64.range(), IntStorage::Default.range());
		assert_eq!(IntStorage::U64.range(), i(0, u64::MAX as i128));
		assert_eq!(IntStorage::I128.range(), i(i128::MIN, i128::MAX));
		assert_eq!(IntStorage::U128.range().hi, I256::from(u128::MAX));
		assert_eq!(IntStorage::U128.range().lo, I256::ZERO);
	}

	#[test]
	fn storage_classes() {
		assert!(!IntStorage::Default.is_js_bigint());
		assert!(IntStorage::I64.is_js_bigint());
		assert!(IntStorage::U64.is_js_bigint());
		assert!(!IntStorage::U32.is_js_bigint());
		assert!(IntStorage::I8.is_signed());
		assert!(!IntStorage::U128.is_signed());
		assert_eq!(IntStorage::U16.bits(), 16);
	}

	#[test]
	fn common_base_types() {
		use IntStorage::*;
		assert_eq!(I8.common_base(I32), Some(I32));
		assert_eq!(I32.common_base(I8), Some(I32));
		assert_eq!(Default.common_base(I64), Some(I64));
		assert_eq!(I64.common_base(Default), Some(I64));
		assert_eq!(I8.common_base(U8), None);
		assert_eq!(U8.common_base(I16), Some(I16));
		assert_eq!(U64.common_base(I64), None);
		assert_eq!(U64.common_base(I128), Some(I128));
		assert_eq!(Default.common_base(Default), Some(Default));
	}

	#[test]
	fn infinity_is_saturating() {
		assert_eq!(saturate(INFINITY + INFINITY), INFINITY);
		assert_eq!(Interval::full().lo, -INFINITY);
		let big = Interval::exact(INFINITY);
		assert_eq!(big.add(i(5, 5)).hi, INFINITY);
		assert_eq!(big.mul(big).hi, INFINITY);
		assert!(Interval::full().is_unbounded_below() && Interval::full().is_unbounded_above());
	}

	#[test]
	fn addition_and_subtraction() {
		assert_eq!(i(1, 2).add(i(10, 20)), i(11, 22));
		assert_eq!(i(1, 2).sub(i(10, 20)), i(-19, -8));
		// values brief §6.3: `a + 1000` with `a: Int8`
		assert_eq!(int8().add(i(1000, 1000)), i(872, 1127));
		// an infinite bound stays infinite
		let open = Interval::new(n(0), INFINITY);
		assert_eq!(open.add(i(-5, 5)), Interval::new(n(-5), INFINITY));
		assert_eq!(open.sub(open), Interval::full());
	}

	#[test]
	fn negation() {
		assert_eq!(int8().neg(), i(-127, 128));
		assert_eq!(i(-3, 5).neg(), i(-5, 3));
		assert_eq!(IntStorage::U8.range().neg(), i(-255, 0));
	}

	#[test]
	fn multiplication_corners() {
		assert_eq!(i(-2, 3).mul(i(-5, 4)), i(-15, 12));
		assert_eq!(i(2, 3).mul(i(4, 5)), i(8, 15));
		assert_eq!(int8().mul(int8()), i(-16256, 16384));
		assert_eq!(i(0, 0).mul(Interval::full()), i(0, 0));
		// Int64 bounds: products need more than 64 bits, computed exactly
		let int64 = IntStorage::I64.range();
		let product = int64.mul(int64);
		assert_eq!(product.hi, n(i64::MIN as i128) * n(i64::MIN as i128));
		assert_eq!(product.lo, n(i64::MIN as i128) * n(i64::MAX as i128));
		// 128-bit products reach 2^254: saturated to ±∞, still beyond every compute type
		let int128 = IntStorage::I128.range();
		assert_eq!(int128.mul(int128), Interval::full());
		let uint128 = IntStorage::U128.range();
		assert_eq!(uint128.mul(uint128), Interval::new(n(0), INFINITY));
		assert_eq!(
			uint128.mul(uint128).fit(ComputeType::UInt128.range()),
			Fit::Partial
		);
		let below = Interval::new(n(0), pow2(124));
		assert_eq!(below.mul(below).hi, pow2(248));
	}

	#[test]
	fn int64_products_exceed_the_compute_type() {
		let int64 = IntStorage::I64.range();
		let product = int64.mul(int64);
		assert_eq!(product.fit(ComputeType::Int64.range()), Fit::Partial);
		assert_eq!(product.fit(ComputeType::Int128.range()), Fit::Contained);
		let sum = IntStorage::U64.range().add(IntStorage::U64.range());
		assert_eq!(sum.hi, n(u64::MAX as i128) * 2);
		assert_eq!(sum.fit(ComputeType::Int128.range()), Fit::Contained);
	}

	#[test]
	fn floor_division() {
		assert_eq!(i(7, 7).floor_div(i(2, 2)), Some(i(3, 3)));
		assert_eq!(i(-7, -7).floor_div(i(2, 2)), Some(i(-4, -4)));
		assert_eq!(i(7, 7).floor_div(i(-2, -2)), Some(i(-4, -4)));
		assert_eq!(i(-7, -7).floor_div(i(-2, -2)), Some(i(3, 3)));
		// `(a + b) // 2` with Int8 operands fits in Int8 (values brief §6.7)
		let sum = int8().add(int8());
		assert_eq!(sum.floor_div(i(2, 2)), Some(int8()));
		// divisor spanning zero: both parts
		assert_eq!(i(-10, 10).floor_div(i(-2, 5)), Some(i(-10, 10)));
		assert_eq!(i(10, 20).floor_div(i(-1, 1)), Some(i(-20, 20)));
		// divisor exactly zero
		assert_eq!(i(1, 5).floor_div(i(0, 0)), None);
		// Int64.minValue // -1 overflows Int64
		let int64 = IntStorage::I64.range();
		let quotient = int64.floor_div(i(-1, -1)).unwrap();
		assert_eq!(quotient.hi, -int64.lo);
		assert_eq!(quotient.fit(int64), Fit::Partial);
	}

	#[test]
	fn floor_division_with_infinities() {
		let positive = Interval::new(n(0), INFINITY);
		assert_eq!(
			positive.floor_div(i(2, 2)),
			Some(Interval::new(n(0), INFINITY))
		);
		assert_eq!(
			i(5, 5).floor_div(Interval::new(n(1), INFINITY)),
			Some(i(0, 5))
		);
		assert_eq!(
			i(-5, -5).floor_div(Interval::new(n(1), INFINITY)),
			Some(i(-5, -1))
		);
		assert_eq!(
			positive.floor_div(positive),
			Some(Interval::new(n(-1), INFINITY))
		);
	}

	#[test]
	fn remainder() {
		assert_eq!(i(7, 7).modulo(i(3, 3)), Some(i(0, 2)));
		// narrowed to `a` when 0 <= a1 and a2 < b1
		assert_eq!(i(0, 3).modulo(i(10, 20)), Some(i(0, 3)));
		assert_eq!(i(0, 15).modulo(i(10, 20)), Some(i(0, 19)));
		assert_eq!(i(-5, 5).modulo(i(4, 4)), Some(i(0, 3)));
		// negative divisor: [b1 + 1, 0], narrowed symmetrically
		assert_eq!(i(-5, 5).modulo(i(-4, -4)), Some(i(-3, 0)));
		assert_eq!(i(-3, 0).modulo(i(-10, -5)), Some(i(-3, 0)));
		// both parts
		assert_eq!(i(-50, 50).modulo(i(-4, 8)), Some(i(-3, 7)));
		assert_eq!(i(1, 5).modulo(i(0, 0)), None);
		// %% -1 is always 0
		assert_eq!(IntStorage::I64.range().modulo(i(-1, -1)), Some(i(0, 0)));
		assert_eq!(
			i(3, 9).modulo(Interval::new(n(1), INFINITY)).unwrap().hi,
			INFINITY
		);
	}

	#[test]
	fn power() {
		assert_eq!(i(2, 2).pow(i(10, 10)), Some(i(1024, 1024)));
		assert_eq!(i(-2, -2).pow(i(3, 3)), Some(i(-8, -8)));
		assert_eq!(i(-2, -2).pow(i(2, 2)), Some(i(4, 4)));
		assert_eq!(i(0, 0).pow(i(0, 0)), Some(i(1, 1)));
		assert_eq!(i(2, 3).pow(i(1, 2)), Some(i(2, 9)));
		// the table's lower bound would be 0^0 = 1, but 0 ** 1 = 0
		assert_eq!(i(0, 5).pow(i(0, 3)), Some(i(0, 125)));
		assert_eq!(i(0, 0).pow(i(0, 2)), Some(i(0, 1)));
		// negative base: [-M, M]
		assert_eq!(i(-3, 2).pow(i(0, 3)), Some(i(-27, 27)));
		// negative exponent possible: the exponent check
		assert_eq!(i(2, 2).pow(i(-1, 3)), None);
		// 2 ** 60 is exact, beyond JavaScript's safe range (E-NUM-19 on JS)
		let big = i(2, 2).pow(i(60, 60)).unwrap();
		assert_eq!(big, i(1 << 60, 1 << 60));
		assert_eq!(big.fit(js_safe_range()), Fit::Disjoint);
		// saturation
		assert_eq!(i(2, 2).pow(i(1000, 1000)).unwrap().hi, INFINITY);
		assert_eq!(i(1, 1).pow(Interval::new(n(0), INFINITY)), Some(i(1, 1)));
		assert_eq!(
			i(2, 2).pow(Interval::new(n(0), INFINITY)).unwrap(),
			Interval::new(n(1), INFINITY)
		);
	}

	#[test]
	fn absolute_value() {
		assert_eq!(int8().abs(), i(0, 128));
		assert_eq!(i(-5, -2).abs(), i(2, 5));
		assert_eq!(i(3, 9).abs(), i(3, 9));
		assert_eq!(i(-2, 9).abs(), i(0, 9));
		// Int8 -128 → 128 doesn't fit Int8: a stop at the sink
		assert_eq!(int8().abs().fit(int8()), Fit::Partial);
	}

	#[test]
	fn clamp_min_max() {
		assert_eq!(int8().clamp(i(0, 0), i(10, 10)), i(0, 10));
		assert_eq!(i(5, 50).clamp(i(0, 0), i(10, 10)), i(5, 10));
		assert_eq!(i(1, 5).min(i(3, 10)), i(1, 5));
		assert_eq!(i(1, 5).max(i(3, 10)), i(3, 10));
		assert_eq!(i(-4, 2).max(i(0, 0)), i(0, 2));
	}

	#[test]
	fn fits() {
		assert_eq!(i(0, 100).fit(int8()), Fit::Contained);
		assert_eq!(i(0, 200).fit(int8()), Fit::Partial);
		assert_eq!(i(200, 200).fit(int8()), Fit::Disjoint);
		// values brief: `let e: Int8 = 100 + 100` is a certain overflow
		assert_eq!(i(100, 100).add(i(100, 100)).fit(int8()), Fit::Disjoint);
		// `let c: Int16 = a + b` with Int8 operands needs no check
		assert_eq!(
			int8().add(int8()).fit(IntStorage::I16.range()),
			Fit::Contained
		);
		// `let d: Int8 = a + b` is checked at runtime
		assert_eq!(int8().add(int8()).fit(int8()), Fit::Partial);
		// Int64 bounds exactly
		let int64 = IntStorage::I64.range();
		assert_eq!(int64.fit(IntStorage::Default.range()), Fit::Contained);
		assert_eq!(Interval::exact(int64.hi + 1).fit(int64), Fit::Disjoint);
		assert_eq!(IntStorage::U64.range().fit(int64), Fit::Partial);
	}

	#[test]
	fn set_operations() {
		assert!(i(0, 5).touches(i(6, 9)));
		assert!(!i(0, 5).touches(i(7, 9)));
		assert!(i(0, 5).touches(i(3, 4)));
		assert_eq!(i(0, 5).intersect(i(3, 9)), Some(i(3, 5)));
		assert_eq!(i(0, 5).intersect(i(6, 9)), None);
		assert_eq!(i(0, 5).hull(i(8, 9)), i(0, 9));
		assert_eq!(i(-3, 3).split_signs(), (Some(i(-3, -1)), Some(i(1, 3))));
		assert_eq!(i(0, 0).split_signs(), (None, None));
		assert_eq!(i(4, 4).as_exact(), Some(n(4)));
		assert_eq!(i(4, 5).as_exact(), None);
	}

	#[test]
	fn compute_types() {
		use IntStorage::*;
		assert_eq!(ComputeType::of([I8, I8]), ComputeType::Int64);
		assert_eq!(ComputeType::of([Default, I64, U32]), ComputeType::Int64);
		assert_eq!(ComputeType::of([I8, U64]), ComputeType::Int128);
		assert_eq!(ComputeType::of([I128]), ComputeType::Int128);
		assert_eq!(ComputeType::of([I128, U128, I8]), ComputeType::UInt128);
		assert_eq!(ComputeType::UInt128.range(), U128.range());
		assert_eq!(js_safe_range().hi, n((1 << 53) - 1));
	}

	#[test]
	fn int_types() {
		let int8 = IntType::full(IntStorage::I8);
		assert!(int8.is_full());
		assert_eq!(int8.literal(), None);
		let literal = IntType {
			min: n(42),
			max: n(42),
			storage: IntStorage::Default,
		};
		assert_eq!(literal.literal(), Some(n(42)));
		assert!(!literal.is_full());
	}
}
