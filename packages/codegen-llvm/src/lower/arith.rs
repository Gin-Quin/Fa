//! Numbers: integer arithmetic with exactly the checks of the analyzer's
//! bits (values brief §6.7, design §3.6), floats, percentages, comparisons
//! and logic.
//!
//! An integer arithmetic tree is computed in the compute type of its
//! [`ArithInfo`] (`i64`, or `i128` signed or unsigned): each node is checked
//! with `llvm.*.with.overflow` when it has `OVERFLOW_NATIVE`, and carries
//! `nsw` (`nuw`) otherwise; the root is range-checked against its sink when
//! it has `SINK_CHECK`, then truncated to the sink's storage.

use ethnum::I256;
use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{ArithOp, CompareOp, CompareOps, LogicOp, NodeList, View};
use fa_analyzer::types::{ComputeType, IntStorage};
use inkwell::values::{BasicMetadataValueEnum, FloatValue, IntValue};
use inkwell::{FloatPredicate, IntPredicate};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

/// The representation of a compute type.
fn compute_repr(compute: ComputeType) -> IntRepr {
	match compute {
		ComputeType::Int64 => IntRepr::I64,
		ComputeType::Int128 => IntRepr {
			bits: 128,
			signed: true,
		},
		ComputeType::UInt128 => IntRepr {
			bits: 128,
			signed: false,
		},
	}
}

/// The narrowest type of 16 or 32 bits holding `low..=high`: unsigned when
/// `low` is not negative (`nuw` steps and unsigned divisions are then exact).
fn narrow_repr(low: i128, high: i128) -> Option<IntRepr> {
	[16, 32].into_iter().find_map(|bits| {
		if low >= 0 && high < 1 << bits {
			Some(IntRepr {
				bits,
				signed: false,
			})
		} else if low >= -(1 << (bits - 1)) && high < 1 << (bits - 1) {
			Some(IntRepr { bits, signed: true })
		} else {
			None
		}
	})
}

struct ArithmeticBounds {
	result: (i128, i128),
	work: (i128, i128),
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// An integer constant of `bits` bits (two's complement of `value`).
	pub(crate) fn const_int(&self, bits: u32, value: I256) -> IntValue<'ctx> {
		let ty = self.int_type(bits);
		let wide = value.as_i128() as u128;
		if bits <= 64 {
			ty.const_int(wide as u64, false)
		} else {
			ty.const_int_arbitrary_precision(&[wide as u64, (wide >> 64) as u64])
		}
	}

	pub(crate) fn integer_literal(
		&mut self,
		value: I256,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		Ok(match self.node_repr(node)? {
			Repr::Int(int) => Val::new(Repr::Int(int), self.const_int(int.bits, value)),
			Repr::BigInt => self.big_constant(value),
			Repr::F64 => Val::new(
				Repr::F64,
				self.context.f64_type().const_float(value.as_f64()),
			),
			Repr::F32 => Val::new(
				Repr::F32,
				self.context
					.f32_type()
					.const_float(value.as_f64() as f32 as f64),
			),
			_ => return Err(self.unsupported(node.index(), "this integer literal")),
		})
	}

	/// The compute representation of an integer arithmetic node.
	fn compute_of(&self, node: NodeId, fallback: Repr) -> IntRepr {
		match self.analysis.arith(node) {
			Some(info) => compute_repr(info.compute),
			None => match fallback {
				Repr::Int(int) if int.bits > 64 => int,
				_ => IntRepr::I64,
			},
		}
	}

	/// The name of the type a compute-type overflow is reported against.
	fn compute_name(&self, node: NodeId, compute: IntRepr) -> &'static str {
		match (compute.bits, compute.signed) {
			(128, true) => "Int128",
			(128, false) => "UInt128",
			_ => match self
				.types()
				.as_int(self.type_of(node))
				.map(|int| int.storage)
			{
				Some(IntStorage::I64) => "Int64",
				_ => "Integer",
			},
		}
	}

	/// An integer operand converted to the compute representation.
	fn int_operand(
		&mut self,
		node: NodeId,
		compute: IntRepr,
	) -> Result<Option<IntValue<'ctx>>, Error> {
		let value = self.expr(node)?;
		match value.repr {
			Repr::Int(int) => Ok(Some(self.int_to(value.int(), int, compute)?)),
			Repr::Never => Ok(None),
			_ => Err(self.unsupported(node.index(), "this operand")),
		}
	}

	pub(crate) fn arithmetic(
		&mut self,
		node: NodeId,
		op: ArithOp,
		operands: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		// `a + b`, `a - b` of field sets (fields.rs)
		if self.is_fields(self.concrete_type(self.type_of(node))) {
			return self.fields_arithmetic(node, op, operands);
		}
		let repr = self.node_repr(node)?;
		match repr {
			Repr::Int(_) => {
				let compute = self.compute_of(node, repr);
				// a narrower type when the storage of the operands bounds every
				// step (bytes summed in 16 bits vectorize as Rust's `u16`)
				let narrow = (compute == IntRepr::I64)
					.then(|| self.arithmetic_bounds(node))
					.flatten()
					.and_then(|bounds| narrow_repr(bounds.work.0, bounds.work.1));
				let work = narrow.unwrap_or(compute);
				let mut values = Vec::new();
				for operand in operands.iter() {
					match self.int_operand(operand, work)? {
						Some(value) => values.push(value),
						None => return Ok(Val::never()),
					}
				}
				let mut result;
				if op.is_right_associative() {
					result = values.pop().expect("an operand");
					while let Some(left) = values.pop() {
						result = self.int_step(node, op, left, result, work)?;
					}
				} else {
					let mut values = values.into_iter();
					result = values.next().expect("an operand");
					for right in values {
						result = self.int_step(node, op, result, right, work)?;
					}
				}
				let result = self.int_to(result, work, compute)?;
				Ok(Val::new(Repr::Int(compute), result))
			}
			Repr::BigInt => self.big_arithmetic(node, op, operands),
			Repr::F64 | Repr::F32 | Repr::Percentage => self.float_arithmetic(op, operands, repr),
			Repr::Str if op == ArithOp::Add => {
				let mut values = Vec::new();
				for operand in operands.iter() {
					let value = self.expr(operand)?;
					if value.repr == Repr::Never {
						return Ok(Val::never());
					}
					values.push((value, operand));
				}
				let mut values = values.into_iter();
				let (mut result, first) = values.next().expect("an operand");
				// a local read for the last time (a string being built, whose
				// block has room) grows in place; else each `+` makes a string
				let owned = matches!(self.view(first), View::Name(_))
					&& self.take_local(result, first)?.is_some();
				for (right, _) in values {
					result = if owned {
						self.append_in_place(result, right)?
					} else {
						self.concat(result, right)?
					};
				}
				Ok(if owned { self.fresh(result) } else { result })
			}
			_ => Err(self.unsupported(node.index(), &format!("`{}` on these values", op.symbol()))),
		}
	}

	/// Bounds of the result and of every value needed to compute it (`+`, `-`, `*`, and
	/// `//`, `%%` of non-negative values by a positive divisor) without checks,
	/// from the storage of its operands only (never from the analyzer's facts,
	/// which a wrapping build may break): `None` when unknown.
	/// A small result does not permit truncating a dividend, divisor or earlier
	/// intermediate. Keep the result bounds separate for enclosing expressions,
	/// whose children are evaluated in their own independently proven widths.
	fn arithmetic_bounds(&self, node: NodeId) -> Option<ArithmeticBounds> {
		let View::Arithmetic { op, operands } = self.view(node) else {
			return None;
		};
		let flags = self.flags(node);
		if flags.intersects(
			NodeFlags::OVERFLOW_NATIVE | NodeFlags::DIVISOR_CHECK | NodeFlags::MIN_DIV_GUARD,
		) {
			return None;
		}
		let bounds: Vec<(i128, i128)> = operands
			.iter()
			.map(|operand| self.storage_bounds(operand))
			.collect::<Option<_>>()?;
		let (mut low, mut high) = *bounds.first()?;
		let mut work = bounds.iter().fold((low, high), |(low, high), &(a, b)| {
			(low.min(a), high.max(b))
		});
		for &(a, b) in &bounds[1..] {
			(low, high) = match op {
				ArithOp::Add => (low.checked_add(a)?, high.checked_add(b)?),
				ArithOp::Subtract => (low.checked_sub(b)?, high.checked_sub(a)?),
				ArithOp::Multiply => {
					let products = [
						low.checked_mul(a)?,
						low.checked_mul(b)?,
						high.checked_mul(a)?,
						high.checked_mul(b)?,
					];
					(*products.iter().min()?, *products.iter().max()?)
				}
				ArithOp::IntegerDivide if low >= 0 && a >= 1 => (0, high),
				ArithOp::Remainder if low >= 0 && a >= 1 => (0, high.min(b - 1)),
				_ => return None,
			};
			if low < -(1 << 31) || high >= 1 << 31 {
				return None;
			}
			work = (work.0.min(low), work.1.max(high));
		}
		Some(ArithmeticBounds {
			result: (low, high),
			work,
		})
	}

	/// The bounds of an integer operand: a literal, a narrowed arithmetic
	/// node, else the range of its storage (a `UInt8` is `0..=255`).
	fn storage_bounds(&self, node: NodeId) -> Option<(i128, i128)> {
		match self.view(node) {
			View::Group(inner) => return self.storage_bounds(inner),
			View::Integer(value) => {
				let value = value.as_i128();
				return Some((value, value));
			}
			View::Arithmetic { .. } => return Some(self.arithmetic_bounds(node)?.result),
			_ => {}
		}
		let Ok(Repr::Int(int)) = self.node_repr(node) else {
			return None;
		};
		if int.bits >= 64 {
			return None;
		}
		Some(if int.signed {
			(-(1i128 << (int.bits - 1)), (1i128 << (int.bits - 1)) - 1)
		} else {
			(0, (1i128 << int.bits) - 1)
		})
	}

	/// One step of an integer arithmetic node, with the checks of its bits.
	fn int_step(
		&mut self,
		node: NodeId,
		op: ArithOp,
		a: IntValue<'ctx>,
		b: IntValue<'ctx>,
		compute: IntRepr,
	) -> Result<IntValue<'ctx>, Error> {
		let flags = self.flags(node);
		let checked = flags.contains(NodeFlags::OVERFLOW_NATIVE);
		match op {
			ArithOp::Add | ArithOp::Subtract | ArithOp::Multiply => {
				if checked {
					return self.checked_op(node, op, a, b, compute);
				}
				if !self.overflow_checks() {
					// two's complement: no `nsw`, no undefined behavior
					return Ok(match op {
						ArithOp::Add => self.builder.build_int_add(a, b, "")?,
						ArithOp::Subtract => self.builder.build_int_sub(a, b, "")?,
						_ => self.builder.build_int_mul(a, b, "")?,
					});
				}
				Ok(match (op, compute.signed) {
					(ArithOp::Add, true) => self.builder.build_int_nsw_add(a, b, "")?,
					(ArithOp::Add, false) => self.builder.build_int_nuw_add(a, b, "")?,
					(ArithOp::Subtract, true) => self.builder.build_int_nsw_sub(a, b, "")?,
					(ArithOp::Subtract, false) => self.builder.build_int_nuw_sub(a, b, "")?,
					(_, true) => self.builder.build_int_nsw_mul(a, b, "")?,
					(_, false) => self.builder.build_int_nuw_mul(a, b, "")?,
				})
			}
			ArithOp::IntegerDivide | ArithOp::Remainder => {
				self.floor_division(node, op, a, b, compute)
			}
			ArithOp::Power => self.power(node, a, b, compute, checked),
			ArithOp::Divide => Err(self.unsupported(node.index(), "an integer `/`")),
		}
	}

	/// `llvm.{s,u}{add,sub,mul}.with.overflow`, and a cold branch to the stop.
	fn checked_op(
		&mut self,
		node: NodeId,
		op: ArithOp,
		a: IntValue<'ctx>,
		b: IntValue<'ctx>,
		compute: IntRepr,
	) -> Result<IntValue<'ctx>, Error> {
		let name = match (op, compute.signed) {
			(ArithOp::Add, true) => "llvm.sadd.with.overflow",
			(ArithOp::Add, false) => "llvm.uadd.with.overflow",
			(ArithOp::Subtract, true) => "llvm.ssub.with.overflow",
			(ArithOp::Subtract, false) => "llvm.usub.with.overflow",
			(_, true) => "llvm.smul.with.overflow",
			(_, false) => "llvm.umul.with.overflow",
		};
		let ty = self.int_type(compute.bits);
		let intrinsic = self.intrinsic(name, &[ty.into()])?;
		let result = self
			.call_value(intrinsic, &[a.into(), b.into()])?
			.into_struct_value();
		let value = self
			.builder
			.build_extract_value(result, 0, "")?
			.into_int_value();
		let overflowed = self
			.builder
			.build_extract_value(result, 1, "")?
			.into_int_value();
		let type_name = self.compute_name(node, compute);
		let site = self.site(type_name, node);
		self.guard(overflowed, |this| {
			if compute.bits <= 64 && compute.signed {
				// the exact value, at 128 bits, for the message
				let i128 = this.context.i128_type();
				let wide_a = this.builder.build_int_s_extend(a, i128, "")?;
				let wide_b = this.builder.build_int_s_extend(b, i128, "")?;
				let exact = match op {
					ArithOp::Add => this.builder.build_int_add(wide_a, wide_b, "")?,
					ArithOp::Subtract => this.builder.build_int_sub(wide_a, wide_b, "")?,
					_ => this.builder.build_int_mul(wide_a, wide_b, "")?,
				};
				this.overflow_stop(exact, true, site)
			} else {
				this.call_noreturn("fa_overflow_unknown", &[site.into()])
			}
		})?;
		Ok(value)
	}

	/// Calls `fa_overflow` with a 128-bit value (or narrower, extended).
	fn overflow_stop(
		&self,
		value: IntValue<'ctx>,
		signed: bool,
		site: inkwell::values::PointerValue<'ctx>,
	) -> Result<(), Error> {
		let arguments = self.value_halves(value, signed, site)?;
		self.call_noreturn("fa_overflow", &arguments)
	}

	/// The arguments `(low, high, signed, site)` of the runtime's overflow
	/// and conversion errors.
	fn value_halves(
		&self,
		value: IntValue<'ctx>,
		signed: bool,
		site: inkwell::values::PointerValue<'ctx>,
	) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, Error> {
		let i128 = self.context.i128_type();
		let i64 = self.context.i64_type();
		let width = value.get_type().get_bit_width();
		let wide = if width < 128 {
			if signed {
				self.builder.build_int_s_extend(value, i128, "")?
			} else {
				self.builder.build_int_z_extend(value, i128, "")?
			}
		} else {
			value
		};
		let low = self.builder.build_int_truncate(wide, i64, "")?;
		let shifted = self
			.builder
			.build_right_shift(wide, i128.const_int(64, false), false, "")?;
		let high = self.builder.build_int_truncate(shifted, i64, "")?;
		let signed = self.context.i32_type().const_int(signed as u64, false);
		Ok(vec![low.into(), high.into(), signed.into(), site.into()])
	}

	/// `a // b` and `a %% b`: floor division, with the divisor check
	/// (`DivisionByZero`) and the `MIN // -1` guards of their bits.
	fn floor_division(
		&mut self,
		node: NodeId,
		op: ArithOp,
		a: IntValue<'ctx>,
		b: IntValue<'ctx>,
		compute: IntRepr,
	) -> Result<IntValue<'ctx>, Error> {
		let flags = self.flags(node);
		let ty = a.get_type();
		if flags.contains(NodeFlags::DIVISOR_CHECK) {
			let zero = self
				.builder
				.build_int_compare(IntPredicate::EQ, b, ty.const_zero(), "")?;
			self.guard(zero, |this| {
				this.raise_runtime("fa_error_division_by_zero", &[])
			})?;
		}
		if !compute.signed {
			return Ok(if op == ArithOp::IntegerDivide {
				self.builder.build_int_unsigned_div(a, b, "")?
			} else {
				self.builder.build_int_unsigned_rem(a, b, "")?
			});
		}
		let minus_one = ty.const_all_ones();
		let mut divisor = b;
		if flags.contains(NodeFlags::MIN_DIV_GUARD) {
			let is_minus_one =
				self.builder
					.build_int_compare(IntPredicate::EQ, b, minus_one, "")?;
			if op == ArithOp::IntegerDivide && !self.overflow_checks() {
				// `MIN // -1` wraps to `MIN`: divide by 1 instead of running `sdiv MIN, -1`
				let minimum = self.const_int(compute.bits, I256::ONE << (compute.bits - 1));
				let is_minimum =
					self.builder
						.build_int_compare(IntPredicate::EQ, a, minimum, "")?;
				let overflows = self.builder.build_and(is_minimum, is_minus_one, "")?;
				divisor = self
					.builder
					.build_select(overflows, ty.const_int(1, false), b, "")?
					.into_int_value();
			} else if op == ArithOp::IntegerDivide {
				// `MIN // -1` overflows
				let minimum = self.const_int(compute.bits, I256::ONE << (compute.bits - 1));
				let is_minimum =
					self.builder
						.build_int_compare(IntPredicate::EQ, a, minimum, "")?;
				let overflows = self.builder.build_and(is_minimum, is_minus_one, "")?;
				let site = self.site(self.compute_name(node, compute), node);
				self.guard(overflows, |this| {
					if compute.bits <= 64 {
						let exact = this.const_int(128, I256::ONE << (compute.bits - 1));
						this.overflow_stop(exact, true, site)
					} else {
						this.call_noreturn("fa_overflow_unknown", &[site.into()])
					}
				})?;
			} else {
				// `x %% -1` is 0, without executing `srem MIN, -1`
				divisor = self
					.builder
					.build_select(is_minus_one, ty.const_int(1, false), b, "")?
					.into_int_value();
			}
		}
		if compute.bits == 64
			&& let Some(result) = self.invariant_signed_division(node, op, a, divisor)?
		{
			return Ok(result);
		}
		let quotient = self.builder.build_int_signed_div(a, divisor, "")?;
		let remainder = self.builder.build_int_signed_rem(a, divisor, "")?;
		// round towards −∞: adjust when the remainder is non-zero and its
		// sign differs from the divisor's
		let nonzero =
			self.builder
				.build_int_compare(IntPredicate::NE, remainder, ty.const_zero(), "")?;
		let signs = self.builder.build_xor(remainder, divisor, "")?;
		let differ =
			self.builder
				.build_int_compare(IntPredicate::SLT, signs, ty.const_zero(), "")?;
		let adjust = self.builder.build_and(nonzero, differ, "")?;
		Ok(if op == ArithOp::IntegerDivide {
			let lower = self
				.builder
				.build_int_sub(quotient, ty.const_int(1, false), "")?;
			self.builder
				.build_select(adjust, lower, quotient, "")?
				.into_int_value()
		} else {
			let raised = self.builder.build_int_add(remainder, divisor, "")?;
			self.builder
				.build_select(adjust, raised, remainder, "")?
				.into_int_value()
		})
	}

	/// `a ** b` on integers: `fa_pow_i64` of the runtime (square and multiply,
	/// checked when the node has `OVERFLOW_NATIVE`; a negative exponent
	/// raises `ArithmeticError`).
	fn power(
		&mut self,
		node: NodeId,
		a: IntValue<'ctx>,
		b: IntValue<'ctx>,
		compute: IntRepr,
		checked: bool,
	) -> Result<IntValue<'ctx>, Error> {
		if compute.bits > 64 {
			return Err(self.unsupported(node.index(), "`**` on 128-bit integers"));
		}
		if self.flags(node).contains(NodeFlags::EXPONENT_CHECK) {
			let negative = self.builder.build_int_compare(
				IntPredicate::SLT,
				b,
				b.get_type().const_zero(),
				"",
			)?;
			self.guard(negative, |this| {
				this.raise_runtime("fa_error_negative_exponent", &[b.into()])
			})?;
		}
		let site = self.site(self.compute_name(node, compute), node);
		let checked = self.context.i32_type().const_int(checked as u64, false);
		let function = self.runtime.function("fa_pow_i64")?;
		Ok(self
			.call_value(function, &[a.into(), b.into(), checked.into(), site.into()])?
			.into_int_value())
	}

	/// The range check of a `SINK_CHECK` root against its sink type, then the
	/// value in the sink's storage.
	pub(crate) fn sink_check(
		&mut self,
		value: Val<'ctx>,
		target: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Int(from) = value.repr else {
			return Ok(value);
		};
		let Some(int) = self.types().as_int(target).copied() else {
			return Err(self.unsupported(node.index(), "this range check"));
		};
		let raw = value.int();
		let ty = raw.get_type();
		// the range of the representation, to skip impossible comparisons
		let (lowest, highest) = if from.signed {
			(
				-(I256::ONE << (from.bits - 1)),
				(I256::ONE << (from.bits - 1)) - 1,
			)
		} else {
			(I256::ZERO, (I256::ONE << from.bits) - 1)
		};
		let mut failed: Option<IntValue<'ctx>> = None;
		if int.min > lowest {
			let predicate = if from.signed {
				IntPredicate::SLT
			} else {
				IntPredicate::ULT
			};
			let below = self.builder.build_int_compare(
				predicate,
				raw,
				self.const_int(from.bits, int.min),
				"",
			)?;
			failed = Some(below);
		}
		if int.max < highest {
			let predicate = if from.signed {
				IntPredicate::SGT
			} else {
				IntPredicate::UGT
			};
			let above = self.builder.build_int_compare(
				predicate,
				raw,
				self.const_int(from.bits, int.max),
				"",
			)?;
			failed = Some(match failed {
				Some(below) => self.builder.build_or(below, above, "")?,
				None => above,
			});
		}
		if let Some(failed) = failed {
			let name = self.session.display_type(target);
			let site = self.site(&name, node);
			self.guard(failed, |this| this.overflow_stop(raw, from.signed, site))?;
		}
		let _ = ty;
		let to = IntRepr::of(int.storage);
		Ok(Val::new(Repr::Int(to), self.int_to(raw, from, to)?))
	}

	/// `-x`.
	pub(crate) fn negate(&mut self, node: NodeId, operand: NodeId) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		match repr {
			Repr::Int(_) => {
				let compute = self.compute_of(node, repr);
				let Some(value) = self.int_operand(operand, compute)? else {
					return Ok(Val::never());
				};
				let zero = value.get_type().const_zero();
				let result = self.int_step(node, ArithOp::Subtract, zero, value, compute)?;
				Ok(Val::new(Repr::Int(compute), result))
			}
			Repr::F64 | Repr::F32 | Repr::Percentage => {
				let value = self.expr(operand)?;
				let value = self.float_value(value, repr, operand)?;
				Ok(Val::new(repr, self.builder.build_float_neg(value, "")?))
			}
			Repr::BigInt => self.big_negate(operand),
			_ => Err(self.unsupported(node.index(), "`-` on this value")),
		}
	}

	/// `x.abs()`.
	pub(crate) fn absolute(&mut self, node: NodeId, operand: NodeId) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		match repr {
			Repr::Int(_) => {
				let compute = self.compute_of(node, repr);
				let Some(value) = self.int_operand(operand, compute)? else {
					return Ok(Val::never());
				};
				let zero = value.get_type().const_zero();
				let negative =
					self.builder
						.build_int_compare(IntPredicate::SLT, value, zero, "")?;
				let negated = self.int_step(node, ArithOp::Subtract, zero, value, compute)?;
				let result = self.builder.build_select(negative, negated, value, "")?;
				Ok(Val::new(Repr::Int(compute), result))
			}
			Repr::F64 | Repr::F32 | Repr::Percentage => {
				let value = self.expr(operand)?;
				let value = self.float_value(value, repr, operand)?;
				let fabs = self.intrinsic("llvm.fabs", &[value.get_type().into()])?;
				Ok(Val::new(repr, self.call_value(fabs, &[value.into()])?))
			}
			Repr::BigInt => {
				let value = self.expr(operand)?;
				if value.repr == Repr::Never {
					return Ok(value);
				}
				self.big_absolute(value)
			}
			_ => Err(self.unsupported(node.index(), "`abs` on this value")),
		}
	}

	/// `(x)%` and `50%`: the fraction, a `double`.
	pub(crate) fn percentage(&mut self, node: NodeId, operand: NodeId) -> Result<Val<'ctx>, Error> {
		let f64 = self.context.f64_type();
		match self.view(operand) {
			View::Integer(value) => {
				return Ok(Val::new(
					Repr::Percentage,
					f64.const_float(value.as_f64() / 100.0),
				));
			}
			View::Float(value) => {
				return Ok(Val::new(Repr::Percentage, f64.const_float(value / 100.0)));
			}
			_ => {}
		}
		let value = self.expr(operand)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let value = self.float_value(value, Repr::F64, node)?;
		let fraction = self
			.builder
			.build_float_div(value, f64.const_float(100.0), "")?;
		Ok(Val::new(Repr::Percentage, fraction))
	}

	/// A number as a float of the representation `to` (`F64`, `F32`, or
	/// `Percentage`, a `double`).
	fn float_value(
		&self,
		value: Val<'ctx>,
		to: Repr,
		node: NodeId,
	) -> Result<FloatValue<'ctx>, Error> {
		let target = if to == Repr::F32 {
			Repr::F32
		} else {
			Repr::F64
		};
		let from = if value.repr == Repr::Percentage {
			Val {
				destination: None,
				repr: Repr::F64,
				value: value.value,
			}
		} else {
			value
		};
		Ok(self
			.convert(from, target, node.index())?
			.value
			.expect("a float")
			.into_float_value())
	}

	fn float_arithmetic(
		&mut self,
		op: ArithOp,
		operands: NodeList<'_>,
		repr: Repr,
	) -> Result<Val<'ctx>, Error> {
		let mut values = Vec::new();
		let mut mixed = false;
		for operand in operands.iter() {
			let value = self.expr(operand)?;
			if value.repr == Repr::Never {
				return Ok(Val::never());
			}
			mixed |= value.repr == Repr::Percentage && repr == Repr::F32;
			values.push(value);
		}
		// `Float32 * Percentage` is computed on doubles, then rounded (as on JavaScript)
		let work = if mixed { Repr::F64 } else { repr };
		let mut floats = Vec::new();
		for (value, operand) in values.into_iter().zip(operands.iter()) {
			floats.push(self.float_value(value, work, operand)?);
		}
		let mut result;
		if op.is_right_associative() {
			result = floats.pop().expect("an operand");
			while let Some(left) = floats.pop() {
				result = self.float_step(op, left, result)?;
			}
		} else {
			let mut floats = floats.into_iter();
			result = floats.next().expect("an operand");
			for right in floats {
				result = self.float_step(op, result, right)?;
			}
		}
		if mixed {
			result = self
				.builder
				.build_float_trunc(result, self.context.f32_type(), "")?;
		}
		Ok(Val::new(repr, result))
	}

	fn float_step(
		&self,
		op: ArithOp,
		a: FloatValue<'ctx>,
		b: FloatValue<'ctx>,
	) -> Result<FloatValue<'ctx>, Error> {
		let ty = a.get_type();
		Ok(match op {
			ArithOp::Add => self.builder.build_float_add(a, b, "")?,
			ArithOp::Subtract => self.builder.build_float_sub(a, b, "")?,
			ArithOp::Multiply => self.builder.build_float_mul(a, b, "")?,
			ArithOp::Divide => self.builder.build_float_div(a, b, "")?,
			ArithOp::IntegerDivide | ArithOp::Remainder => {
				let quotient = self.builder.build_float_div(a, b, "")?;
				let floor = self.intrinsic("llvm.floor", &[ty.into()])?;
				let floored = self
					.call_value(floor, &[quotient.into()])?
					.into_float_value();
				if op == ArithOp::IntegerDivide {
					floored
				} else {
					let product = self.builder.build_float_mul(floored, b, "")?;
					self.builder.build_float_sub(a, product, "")?
				}
			}
			ArithOp::Power => {
				let pow = self.intrinsic("llvm.pow", &[ty.into()])?;
				self.call_value(pow, &[a.into(), b.into()])?
					.into_float_value()
			}
		})
	}

	/// `target op= value` on a local.
	pub(crate) fn compound_assignment(
		&mut self,
		node: NodeId,
		op: ArithOp,
		target: NodeId,
		value: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// a local, a field, an item: the place is computed once
		let slot = self.place(target)?;
		let current = self.load(slot)?;
		// `set += flags`, `set -= flags` (fields.rs)
		if self.is_fields(self.concrete_type(self.type_of(target))) {
			let right = self.expr(value)?;
			if right.repr == Repr::Never {
				return Ok(right);
			}
			let result = self.fields_step(op, current, right)?;
			self.store(slot, result, node)?;
			return Ok(Val::void());
		}
		let result = match slot.repr {
			Repr::Int(storage) => {
				let compute = self.compute_of(node, slot.repr);
				let left = self.int_to(current.int(), storage, compute)?;
				let Some(right) = self.int_operand(value, compute)? else {
					return Ok(Val::never());
				};
				let result = self.int_step(node, op, left, right, compute)?;
				let result = Val::new(Repr::Int(compute), result);
				match self.analysis.coercion(node) {
					Some(sink) if self.flags(node).contains(NodeFlags::SINK_CHECK) => {
						self.sink_check(result, sink, node)?
					}
					_ => result,
				}
			}
			Repr::F64 | Repr::F32 => {
				let right = self.expr(value)?;
				if right.repr == Repr::Never {
					return Ok(Val::never());
				}
				let mixed = right.repr == Repr::Percentage && slot.repr == Repr::F32;
				let work = if mixed { Repr::F64 } else { slot.repr };
				let left = self.float_value(current, work, target)?;
				let right = self.float_value(right, work, value)?;
				let mut result = self.float_step(op, left, right)?;
				if mixed {
					result = self
						.builder
						.build_float_trunc(result, self.context.f32_type(), "")?;
				}
				Val::new(slot.repr, result)
			}
			// `total += 1n`: the new value replaces the old one, which is dropped
			Repr::BigInt => {
				let Some(right) = self.big_operand(value)? else {
					return Ok(Val::never());
				};
				let result = self.big_step(node, op, current.int(), right)?;
				let result = Val::new(Repr::BigInt, result);
				self.unregister(result);
				self.emit_drop(current)?;
				result
			}
			Repr::Percentage => {
				let right = self.expr(value)?;
				let left = self.float_value(current, Repr::F64, target)?;
				let right = self.float_value(right, Repr::F64, value)?;
				Val::new(Repr::Percentage, self.float_step(op, left, right)?)
			}
			Repr::Str if op == ArithOp::Add => {
				if let View::Template { parts } = self.view(value) {
					if let Some(&(counter, end, inclusive)) =
						self.reserved_appends.get(&(self.file, node))
						&& let Some(part) = self.integer_part(parts)
					{
						let integer = self.expr(part)?;
						if integer.repr == Repr::Never {
							return Ok(integer);
						}
						let inline_capable = self.inline_reserved.get(&(self.file, node)).copied();
						self.reserved_integer_append(
							slot.pointer,
							integer,
							counter,
							end,
							inclusive,
							inline_capable,
						)?;
						return Ok(Val::void());
					}
					return self.append_template(slot.pointer, target, parts);
				}
				if let Some(plan) = self.destination_call(value, target) {
					return self.append_call_into(slot.pointer, target, value, plan);
				}
				let right = self.expr(value)?;
				if right.repr == Repr::Never {
					return Ok(Val::never());
				}
				// a choice of literals of one length: a constant-length copy
				let right = match self.literal_shape(value) {
					Some(shape) if !matches!(self.view(value), View::String(_)) => {
						// a static string: nothing to drop
						self.unregister(right);
						self.with_literal_shape(right, shape)?
					}
					_ => right,
				};
				if let Some(&(counter, end, inclusive)) =
					self.reserved_appends.get(&(self.file, node))
				{
					let inline_capable = self.inline_reserved.get(&(self.file, node)).copied();
					self.reserved_append(
						slot.pointer,
						right,
						counter,
						end,
						inclusive,
						inline_capable,
					)?;
					return Ok(Val::void());
				}
				self.append_string(slot.pointer, right)?;
				return Ok(Val::void());
			}
			_ => return Err(self.unsupported(node.index(), "this compound assignment")),
		};
		self.store(slot, result, node)?;
		Ok(Val::void())
	}

	// ── comparisons and logic ───────────────────────────────────────────

	/// Two values in a common representation, for a comparison.
	fn common(
		&self,
		left: Val<'ctx>,
		right: Val<'ctx>,
		node: NodeId,
	) -> Result<(Val<'ctx>, Val<'ctx>), Error> {
		let expand = |value: Val<'ctx>| {
			if let Repr::Rec(target) = value.repr
				&& self.compressed_union_link(value.repr).is_some()
			{
				self.unfold_rec(value, target, node.index())
			} else {
				Ok(value)
			}
		};
		let (left, right) = (expand(left)?, expand(right)?);
		let float = |repr: Repr| matches!(repr, Repr::F64 | Repr::F32 | Repr::Percentage);
		Ok(match (left.repr, right.repr) {
			(Repr::Int(a), Repr::Int(b)) => {
				let common = Repr::Int(a.common(b));
				(
					self.convert(left, common, node.index())?,
					self.convert(right, common, node.index())?,
				)
			}
			// a `BigInt` and an integer (`==`): the integer converted
			(Repr::BigInt, Repr::Int(int)) => {
				let right = self.big_from_int(right.int(), int)?;
				(left, right)
			}
			(Repr::Int(int), Repr::BigInt) => (self.big_from_int(left.int(), int)?, right),
			(a, b) if float(a) || float(b) => {
				let work = if a == Repr::F32 && b == Repr::F32 {
					Repr::F32
				} else {
					Repr::F64
				};
				(
					Val::new(work, self.float_value(left, work, node)?),
					Val::new(work, self.float_value(right, work, node)?),
				)
			}
			_ => (left, right),
		})
	}

	/// Compares two lowered values: `predicate` for integers (signed form),
	/// the ordered float predicates, the byte order of strings.
	fn compare(
		&mut self,
		left: Val<'ctx>,
		right: Val<'ctx>,
		op: Option<CompareOp>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let (left, right) = self.common(left, right, node)?;
		// a `GrowingArray` compares like an array of its items
		let (left, right) = (self.growing_view(left)?, self.growing_view(right)?);
		match left.repr {
			Repr::Int(int) => {
				let predicate = match (op, int.signed) {
					(None, _) => IntPredicate::EQ,
					(Some(CompareOp::Less), true) => IntPredicate::SLT,
					(Some(CompareOp::Less), false) => IntPredicate::ULT,
					(Some(CompareOp::LessOrEqual), true) => IntPredicate::SLE,
					(Some(CompareOp::LessOrEqual), false) => IntPredicate::ULE,
					(Some(CompareOp::Greater), true) => IntPredicate::SGT,
					(Some(CompareOp::Greater), false) => IntPredicate::UGT,
					(Some(CompareOp::GreaterOrEqual), true) => IntPredicate::SGE,
					(Some(CompareOp::GreaterOrEqual), false) => IntPredicate::UGE,
				};
				Ok(self
					.builder
					.build_int_compare(predicate, left.int(), right.int(), "")?)
			}
			Repr::BigInt => self.big_compare(left, right, op),
			Repr::Bool | Repr::Enum(_) if op.is_none() => {
				Ok(self
					.builder
					.build_int_compare(IntPredicate::EQ, left.int(), right.int(), "")?)
			}
			Repr::F64 | Repr::F32 => {
				let predicate = match op {
					None => FloatPredicate::OEQ,
					Some(CompareOp::Less) => FloatPredicate::OLT,
					Some(CompareOp::LessOrEqual) => FloatPredicate::OLE,
					Some(CompareOp::Greater) => FloatPredicate::OGT,
					Some(CompareOp::GreaterOrEqual) => FloatPredicate::OGE,
				};
				let a = left.value.expect("a float").into_float_value();
				let b = right.value.expect("a float").into_float_value();
				let equal = self.builder.build_float_compare(predicate, a, b, "")?;
				if op.is_none() && self.frame().same_value_zero {
					let a_nan = self
						.builder
						.build_float_compare(FloatPredicate::UNO, a, a, "")?;
					let b_nan = self
						.builder
						.build_float_compare(FloatPredicate::UNO, b, b, "")?;
					let both = self.builder.build_and(a_nan, b_nan, "")?;
					return Ok(self.builder.build_or(equal, both, "")?);
				}
				Ok(equal)
			}
			Repr::Str if right.repr == Repr::Str => {
				if op.is_none() {
					return self.strings_equal(left, right);
				}
				let (a, a_length) = self.string_parts(left)?;
				let (b, b_length) = self.string_parts(right)?;
				let arguments = [a.into(), a_length.into(), b.into(), b_length.into()];
				let i32 = self.context.i32_type();
				match op {
					None => {
						let equal = self.runtime.function("fa_string_equal")?;
						let result = self.call_value(equal, &arguments)?.into_int_value();
						Ok(self.builder.build_int_compare(
							IntPredicate::NE,
							result,
							i32.const_zero(),
							"",
						)?)
					}
					Some(op) => {
						let compare = self.runtime.function("fa_string_compare")?;
						let result = self.call_value(compare, &arguments)?.into_int_value();
						let predicate = match op {
							CompareOp::Less => IntPredicate::SLT,
							CompareOp::LessOrEqual => IntPredicate::SLE,
							CompareOp::Greater => IntPredicate::SGT,
							CompareOp::GreaterOrEqual => IntPredicate::SGE,
						};
						Ok(self.builder.build_int_compare(
							predicate,
							result,
							i32.const_zero(),
							"",
						)?)
					}
				}
			}
			// structural equality: every field, every item
			Repr::Object(ty) if op.is_none() => {
				let left_struct = left.value.expect("an object").into_struct_value();
				let right_value = self.convert(right, left.repr, node.index())?;
				let right_struct = right_value.value.expect("an object").into_struct_value();
				let fields = self.object_fields(ty);
				let mut equal = self.context.bool_type().const_int(1, false);
				for (index, repr) in fields.into_iter().enumerate() {
					// relation sides don't count (D21)
					if matches!(repr, Repr::Link(_) | Repr::Links(_)) {
						continue;
					}
					let a = self
						.builder
						.build_extract_value(left_struct, index as u32, "")?;
					let b = self
						.builder
						.build_extract_value(right_struct, index as u32, "")?;
					let same = self.values_equal_any(Val::new(repr, a), Val::new(repr, b), node)?;
					equal = self.builder.build_and(equal, same, "")?;
				}
				Ok(equal)
			}
			Repr::Tuple(ty) if op.is_none() && self.inline_info(ty).is_some() => {
				let (item, _) = self.inline_info(ty).expect("an inline array");
				let left = self.inline_view(left, ty)?;
				let right = self.inline_view(right, ty)?;
				self.arrays_equal(left, right, item, node)
			}
			Repr::Tuple(ty) if op.is_none() => {
				let left_struct = left.value.expect("a tuple").into_struct_value();
				let right_struct = right.value.expect("a tuple").into_struct_value();
				let mut equal = self.context.bool_type().const_int(1, false);
				let items = self.tuple_items(ty);
				let right_items = match right.repr {
					Repr::Tuple(other) => self.tuple_items(other),
					_ => items.clone(),
				};
				for (index, (item, right_item)) in items.into_iter().zip(right_items).enumerate() {
					let repr = self.repr_of(item, node.index())?;
					let right_repr = self.repr_of(right_item, node.index())?;
					let a = self
						.builder
						.build_extract_value(left_struct, index as u32, "")?;
					let b = self
						.builder
						.build_extract_value(right_struct, index as u32, "")?;
					let same =
						self.values_equal_any(Val::new(repr, a), Val::new(right_repr, b), node)?;
					equal = self.builder.build_and(equal, same, "")?;
				}
				Ok(equal)
			}
			Repr::Bytes if op.is_none() => {
				let right = self.convert(right, Repr::Bytes, node.index())?;
				let equal = self
					.call_value(
						self.runtime.function("fa_bytes_equal")?,
						&[
							left.value.expect("Bytes").into(),
							right.value.expect("Bytes").into(),
						],
					)?
					.into_int_value();
				Ok(self.builder.build_int_compare(
					IntPredicate::NE,
					equal,
					equal.get_type().const_zero(),
					"",
				)?)
			}
			Repr::Array(item_type) if op.is_none() => {
				self.arrays_equal(left, right, item_type, node)
			}
			Repr::Map(_) | Repr::Set(_) if op.is_none() => self.tables_equal(left, right, node),
			// pools and columnars: their elements in slot order
			Repr::Pool(item_type) | Repr::Columnar(item_type) if op.is_none() => {
				let right = self.convert(right, left.repr, node.index())?;
				let left = self.pool_view(left, node)?;
				let right = self.pool_view(right, node)?;
				self.arrays_equal(left, right, item_type, node)
			}
			Repr::Rec(target) if op.is_none() => {
				let right = self.convert(right, left.repr, node.index())?;
				let edge = left.repr;
				let target = self.repr_of(target, node.index())?;
				let compare = self.nested_equals_function(target)?;
				let left = self.edge_address(&self.builder, edge, left.value.expect("an edge"))?;
				let right =
					self.edge_address(&self.builder, edge, right.value.expect("an edge"))?;
				let equal = self
					.builder
					.build_call(compare, &[left.into(), right.into()], "")?
					.try_as_basic_value()
					.basic()
					.expect("equality")
					.into_int_value();
				Ok(self.builder.build_int_compare(
					IntPredicate::NE,
					equal,
					self.context.i32_type().const_zero(),
					"",
				)?)
			}
			Repr::Enum(_) if op.is_none() => {
				Ok(self
					.builder
					.build_int_compare(IntPredicate::EQ, left.int(), right.int(), "")?)
			}
			// values of unions: the same member, equal data (objects.rs)
			_ if op.is_none()
				&& [left.repr, right.repr]
					.iter()
					.any(|repr| matches!(repr, Repr::Union | Repr::InlineUnion(_))) =>
			{
				self.unions_equal(left, right, node)
			}
			_ => Err(self.unsupported(node.index(), "comparing these values")),
		}
	}

	/// Two arrays of the same length with equal items.
	pub(super) fn arrays_equal(
		&mut self,
		left: Val<'ctx>,
		right: Val<'ctx>,
		item_type: TypeId,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let item = self.item_repr(item_type, node)?;
		let a = left.value.expect("an array").into_pointer_value();
		let b = right.value.expect("an array").into_pointer_value();
		self.materialize_array(a, item)?;
		self.materialize_array(b, item)?;
		let (a_items, a_length) = self.array_parts(a)?;
		let (b_items, b_length) = self.array_parts(b)?;
		let result = self.alloca(Repr::Bool, "equal")?;
		let same_length =
			self.builder
				.build_int_compare(IntPredicate::EQ, a_length, b_length, "")?;
		self.builder.build_store(result.pointer, same_length)?;
		let compare = self.append_block("compare_items");
		let end = self.append_block("compared");
		self.builder
			.build_conditional_branch(same_length, compare, end)?;
		self.builder.position_at_end(compare);
		let item_type_llvm = self.llvm_type(item);
		let counter = self.alloca(Repr::Int(IntRepr::I64), "item")?;
		let i64 = self.context.i64_type();
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("compare_item");
		let body = self.append_block("compare_item_body");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let index = self.load(counter)?.int();
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, index, a_length, "")?;
		self.builder.build_conditional_branch(more, body, end)?;
		self.builder.position_at_end(body);
		let a_pointer = unsafe {
			self.builder
				.build_in_bounds_gep(item_type_llvm, a_items, &[index], "")?
		};
		let b_pointer = unsafe {
			self.builder
				.build_in_bounds_gep(item_type_llvm, b_items, &[index], "")?
		};
		// A cached declaration breaks recursive Object -> Array -> Object
		// expansion while preserving the enclosing comparison's NaN rule.
		let equal = self.nested_equals_function(item)?;
		let same = self
			.builder
			.build_call(equal, &[a_pointer.into(), b_pointer.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("equality")
			.into_int_value();
		let same = self.builder.build_int_compare(
			IntPredicate::NE,
			same,
			self.context.i32_type().const_zero(),
			"",
		)?;
		self.builder.build_store(result.pointer, same)?;
		let next = self
			.builder
			.build_int_nsw_add(index, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		let continue_block = self.append_block("compare_next");
		self.builder
			.build_conditional_branch(same, continue_block, end)?;
		self.builder.position_at_end(continue_block);
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(end);
		Ok(self.load(result)?.int())
	}

	/// `a == b` on two lowered values, optionals included.
	pub(crate) fn values_equal_any(
		&mut self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let optional = |repr: Repr| matches!(repr, Repr::None | Repr::Optional(_));
		if optional(a.repr) || optional(b.repr) {
			return self.optional_equal(a, b, node);
		}
		self.compare(a, b, None, node)
	}

	/// `a == b` on two lowered values (not optionals).
	pub(crate) fn values_equal(
		&mut self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		self.compare(a, b, None, node)
	}

	/// `a < b` (`SLT`) or `a <= b` (`SLE`) on two lowered values.
	pub(crate) fn values_order(
		&mut self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		predicate: IntPredicate,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let op = if predicate == IntPredicate::SLT {
			CompareOp::Less
		} else {
			CompareOp::LessOrEqual
		};
		self.compare(a, b, Some(op), node)
	}

	pub(crate) fn equality(
		&mut self,
		left: NodeId,
		right: NodeId,
		negated: bool,
	) -> Result<Val<'ctx>, Error> {
		for (indexed, literal) in [(left, right), (right, left)] {
			if let Some(indexed) = self.string_unit_node(indexed)
				&& let Some(text) = self.constant_text(literal)
				&& text.encode_utf16().count() == 1
			{
				let expected = text.encode_utf16().next().expect("one code unit");
				let unit = self.string_unit_index(indexed)?;
				if unit.repr == Repr::Never {
					return Ok(unit);
				}
				let predicate = if negated {
					IntPredicate::NE
				} else {
					IntPredicate::EQ
				};
				return Ok(Val::new(
					Repr::Bool,
					self.builder.build_int_compare(
						predicate,
						unit.int(),
						self.context
							.i16_type()
							.const_int(u64::from(expected), false),
						"",
					)?,
				));
			}
		}
		let a = self.expr(left)?;
		let b = self.expr(right)?;
		if a.repr == Repr::Never || b.repr == Repr::Never {
			return Ok(Val::never());
		}
		let optional = |repr: Repr| matches!(repr, Repr::None | Repr::Optional(_));
		if optional(a.repr) || optional(b.repr) {
			let equal = self.optional_equal(a, b, left)?;
			let result = if negated {
				self.builder.build_not(equal, "")?
			} else {
				equal
			};
			return Ok(Val::new(Repr::Bool, result));
		}
		let float = matches!(a.repr, Repr::F64 | Repr::F32 | Repr::Percentage);
		let equal = self.compare(a, b, None, left)?;
		let result = if !negated {
			equal
		} else if float {
			// `NaN != NaN`
			let (a, b) = self.common(a, b, left)?;
			self.builder.build_float_compare(
				FloatPredicate::UNE,
				a.value.expect("a float").into_float_value(),
				b.value.expect("a float").into_float_value(),
				"",
			)?
		} else {
			self.builder.build_not(equal, "")?
		};
		Ok(Val::new(Repr::Bool, result))
	}

	/// `a == b` when one side is an optional or `none`: both absent, or both
	/// present and equal.
	fn optional_equal(
		&mut self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let (a_present, a_value) = self.optional_split(a)?;
		let (b_present, b_value) = self.optional_split(b)?;
		let same_presence =
			self.builder
				.build_int_compare(IntPredicate::EQ, a_present, b_present, "")?;
		let (Some(a_value), Some(b_value)) = (a_value, b_value) else {
			// one side is `none`: equal when the other is absent
			return Ok(same_presence);
		};
		// Absent owning payloads contain null pointers, not valid empty
		// collections. Compare their values only when both are present.
		let present = self.builder.build_and(a_present, b_present, "")?;
		let start = self.builder.get_insert_block().expect("a block");
		let compare = self.append_block("compare_optional_values");
		let end = self.append_block("compared_optionals");
		self.builder
			.build_conditional_branch(present, compare, end)?;
		self.builder.position_at_end(compare);
		let mark = self.mark();
		let equal = self.compare(a_value, b_value, None, node)?;
		self.end_scope(mark, None)?;
		let compared = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let result = self
			.builder
			.build_phi(self.context.bool_type(), "optional_equal")?;
		result.add_incoming(&[(&same_presence, start), (&equal, compared)]);
		Ok(result.as_basic_value().into_int_value())
	}

	/// `a < b <= c`: `a < b and b <= c`, each operand evaluated once, from
	/// left to right, stopping at the first false comparison.
	pub(crate) fn comparison(
		&mut self,
		operands: NodeList<'_>,
		operators: CompareOps<'_>,
	) -> Result<Val<'ctx>, Error> {
		let result = self.alloca(Repr::Bool, "comparison")?;
		let bool_type = self.context.bool_type();
		self.builder
			.build_store(result.pointer, bool_type.const_zero())?;
		let end = self.append_block("end_comparison");
		let first = operands.first().expect("an operand");
		let mut left = self.expr(first)?;
		let mut left_node = first;
		// the operands after the second are evaluated conditionally: their
		// temporaries are dropped on each way to the end
		let mark = self.mark();
		for (index, op) in operators.iter().enumerate() {
			let right_node = operands.get(index + 1).expect("an operand");
			let right = self.expr(right_node)?;
			if left.repr == Repr::Never || right.repr == Repr::Never {
				return Ok(Val::never());
			}
			let holds = self.compare(left, right, Some(op), left_node)?;
			if index + 1 == operators.len() {
				self.builder.build_store(result.pointer, holds)?;
				self.drop_to(mark)?;
				self.builder.build_unconditional_branch(end)?;
			} else {
				let next = self.append_block("comparison");
				let failed = self.append_block("comparison_failed");
				self.builder.build_conditional_branch(holds, next, failed)?;
				self.builder.position_at_end(failed);
				self.drop_to(mark)?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(next);
			}
			left = right;
			left_node = right_node;
		}
		self.cleanups.borrow_mut().truncate(mark);
		self.builder.position_at_end(end);
		self.load(result)
	}

	/// `and`, `or`: short-circuit.
	pub(crate) fn logic(
		&mut self,
		op: LogicOp,
		operands: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let result = self.alloca(Repr::Bool, "logic")?;
		let end = self.append_block("end_logic");
		let count = operands.len();
		for (index, operand) in operands.iter().enumerate() {
			let value = self.condition(operand)?;
			self.builder.build_store(result.pointer, value)?;
			if index + 1 == count {
				self.builder.build_unconditional_branch(end)?;
			} else {
				let next = self.append_block("logic");
				match op {
					LogicOp::And => self.builder.build_conditional_branch(value, next, end)?,
					LogicOp::Or => self.builder.build_conditional_branch(value, end, next)?,
				};
				self.builder.position_at_end(next);
			}
		}
		self.builder.position_at_end(end);
		self.load(result)
	}

	/// An LLVM intrinsic, by name and overloaded types.
	pub(crate) fn intrinsic(
		&self,
		name: &str,
		types: &[inkwell::types::BasicTypeEnum<'ctx>],
	) -> Result<inkwell::values::FunctionValue<'ctx>, Error> {
		inkwell::intrinsics::Intrinsic::find(name)
			.and_then(|intrinsic| intrinsic.get_declaration(self.module, types))
			.ok_or_else(|| Error::Llvm(format!("unknown intrinsic {name}")))
	}
}
