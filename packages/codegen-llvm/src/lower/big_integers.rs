//! `BigInt` (numbers page, "Big integers"): an `i64` word, inline when it
//! fits in 63 bits (odd: the value shifted left by one, plus one), else the
//! address of a block of 32-bit limbs and a sign (the runtime's `FaBig`,
//! `fa_runtime.c`). The form is canonical, so that equal values have equal
//! words when one of them is inline. Small values never allocate; a literal
//! beyond 63 bits is a constant block (capacity 0: never freed, shared by
//! copies). Every runtime operation borrows its operands and returns an
//! owned value, a temporary of the statement like a built string.

use ethnum::I256;
use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{ArithOp, CompareOp, NodeList};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::module::Linkage;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FloatValue, IntValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

/// The largest inline value, 2^62 - 1.
const SMALL_MAX: i128 = (1 << 62) - 1;
/// The smallest inline value, -2^62.
const SMALL_MIN: i128 = -(1 << 62);

/// The magnitude written by a literal (`12_345n`, `0xFFn`), in 32-bit limbs,
/// least significant first.
fn literal_limbs(text: &str) -> Option<Vec<u32>> {
	let text = text.trim_end_matches('n');
	let (digits, radix) = match text.get(..2) {
		Some("0x" | "0X") => (&text[2..], 16),
		Some("0b" | "0B") => (&text[2..], 2),
		Some("0o" | "0O") => (&text[2..], 8),
		_ => (text, 10),
	};
	let mut limbs: Vec<u32> = Vec::new();
	for character in digits.chars().filter(|&character| character != '_') {
		let digit = character.to_digit(radix)?;
		let mut carry = u64::from(digit);
		for limb in limbs.iter_mut() {
			let value = u64::from(*limb) * u64::from(radix) + carry;
			*limb = value as u32;
			carry = value >> 32;
		}
		if carry != 0 {
			limbs.push(carry as u32);
		}
	}
	Some(limbs)
}

/// The magnitude of an integer in 32-bit limbs.
fn magnitude_limbs(value: I256) -> Vec<u32> {
	let mut magnitude = if value < 0 { -value } else { value };
	let mut limbs = Vec::new();
	while magnitude != 0 {
		limbs.push((magnitude & I256::from(u32::MAX)).as_u32());
		magnitude >>= 32;
	}
	limbs
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The word of an inline value.
	fn big_inline(&self, value: i128) -> IntValue<'ctx> {
		let word = ((value as i64 as u64) << 1) | 1;
		self.context.i64_type().const_int(word, false)
	}

	/// A constant value from its sign and magnitude: inline when it fits,
	/// else a constant block of the program.
	fn big_constant_limbs(&mut self, negative: bool, mut limbs: Vec<u32>) -> Val<'ctx> {
		while limbs.last() == Some(&0) {
			limbs.pop();
		}
		let magnitude: Option<u64> = match limbs.as_slice() {
			[] => Some(0),
			[low] => Some(u64::from(*low)),
			[low, high] => Some(u64::from(*low) | (u64::from(*high) << 32)),
			_ => None,
		};
		if let Some(magnitude) = magnitude {
			let value = if negative {
				-(magnitude as i128)
			} else {
				magnitude as i128
			};
			if (SMALL_MIN..=SMALL_MAX).contains(&value) {
				return Val::new(Repr::BigInt, self.big_inline(value));
			}
		}
		let i32 = self.context.i32_type();
		let limb_values: Vec<IntValue<'ctx>> = limbs
			.iter()
			.map(|&limb| i32.const_int(u64::from(limb), false))
			.collect();
		let header = [
			i32.const_int(limbs.len() as u64, false),
			// capacity 0: a constant, never freed
			i32.const_zero(),
			i32.const_int(u64::from(negative), false),
			i32.const_zero(),
		];
		let array = i32.const_array(&limb_values);
		let ty = self.context.struct_type(
			&[
				i32.into(),
				i32.into(),
				i32.into(),
				i32.into(),
				array.get_type().into(),
			],
			false,
		);
		let value = ty.const_named_struct(&[
			header[0].into(),
			header[1].into(),
			header[2].into(),
			header[3].into(),
			array.into(),
		]);
		let global = self.module.add_global(ty, None, "fa.big");
		global.set_initializer(&value);
		global.set_constant(true);
		global.set_linkage(Linkage::Private);
		global.set_unnamed_addr(true);
		global.set_alignment(8);
		let word = global
			.as_pointer_value()
			.const_to_int(self.context.i64_type());
		Val::new(Repr::BigInt, word)
	}

	/// A `BigInt` constant of an exact value.
	pub(crate) fn big_constant(&mut self, value: I256) -> Val<'ctx> {
		let limbs = magnitude_limbs(value);
		self.big_constant_limbs(value < 0, limbs)
	}

	/// A literal `12n`.
	pub(crate) fn big_literal(&mut self, text: &str, node: NodeId) -> Result<Val<'ctx>, Error> {
		let (negative, digits) = match text.strip_prefix('-') {
			Some(digits) => (true, digits),
			None => (false, text),
		};
		let Some(limbs) = literal_limbs(digits) else {
			return Err(self.unsupported(node.index(), "this `BigInt` literal"));
		};
		Ok(self.big_constant_limbs(negative, limbs))
	}

	/// The result of a runtime operation returning a value: a temporary.
	fn big_call(
		&self,
		name: &str,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Val<'ctx>, Error> {
		let function = self.runtime.function(name)?;
		let word = self.call_value(function, arguments)?;
		Ok(self.fresh(Val::new(Repr::BigInt, word)))
	}

	/// An integer as a `BigInt` (exact), a temporary.
	pub(crate) fn big_from_int(
		&self,
		value: IntValue<'ctx>,
		from: IntRepr,
	) -> Result<Val<'ctx>, Error> {
		let word = self.big_from_int_with(&self.builder, value, from)?;
		Ok(self.fresh(Val::new(Repr::BigInt, word)))
	}

	pub(super) fn big_from_int_with(
		&self,
		builder: &Builder<'ctx>,
		value: IntValue<'ctx>,
		from: IntRepr,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let call = |name, args: &[BasicMetadataValueEnum<'ctx>]| -> Result<_, Error> {
			Ok(builder
				.build_call(self.runtime.function(name)?, args, "")?
				.try_as_basic_value()
				.basic()
				.expect("a BigInt"))
		};
		let i64 = self.context.i64_type();
		if from.bits <= 64 {
			let (name, wide) = match (from.signed, from.bits < 64) {
				(true, true) => (
					"fa_bigint_from_i64",
					builder.build_int_s_extend(value, i64, "")?,
				),
				(true, false) => ("fa_bigint_from_i64", value),
				(false, true) => (
					"fa_bigint_from_u64",
					builder.build_int_z_extend(value, i64, "")?,
				),
				(false, false) => ("fa_bigint_from_u64", value),
			};
			return call(name, &[wide.into()]);
		}
		let low = builder.build_int_truncate(value, i64, "")?;
		let shifted =
			builder.build_right_shift(value, value.get_type().const_int(64, false), false, "")?;
		let high = builder.build_int_truncate(shifted, i64, "")?;
		let signed = self
			.context
			.i32_type()
			.const_int(u64::from(from.signed), false);
		call(
			"fa_bigint_from_128",
			&[low.into(), high.into(), signed.into()],
		)
	}

	/// The word of a `BigInt` operand (an integer converted).
	pub(crate) fn big_operand(&mut self, node: NodeId) -> Result<Option<IntValue<'ctx>>, Error> {
		let value = self.expr(node)?;
		match value.repr {
			Repr::BigInt => Ok(Some(value.int())),
			Repr::Int(int) => Ok(Some(self.big_from_int(value.int(), int)?.int())),
			Repr::Never => Ok(None),
			_ => Err(self.unsupported(node.index(), "this operand")),
		}
	}

	/// `a + b`, `a - b`, `a * b`, `a // b`, `a %% b`, `a ** b` on `BigInt`s.
	pub(crate) fn big_arithmetic(
		&mut self,
		node: NodeId,
		op: ArithOp,
		operands: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let mut values = Vec::new();
		for operand in operands.iter() {
			match self.big_operand(operand)? {
				Some(value) => values.push(value),
				None => return Ok(Val::never()),
			}
		}
		let mut result;
		if op.is_right_associative() {
			result = values.pop().expect("an operand");
			while let Some(left) = values.pop() {
				result = self.big_step(node, op, left, result)?;
			}
		} else {
			let mut values = values.into_iter();
			result = values.next().expect("an operand");
			for right in values {
				result = self.big_step(node, op, result, right)?;
			}
		}
		Ok(Val::new(Repr::BigInt, result))
	}

	pub(crate) fn big_step(
		&mut self,
		node: NodeId,
		op: ArithOp,
		a: IntValue<'ctx>,
		b: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let flags = self.flags(node);
		let name = match op {
			ArithOp::Add => "fa_bigint_add",
			ArithOp::Subtract => "fa_bigint_subtract",
			ArithOp::Multiply => "fa_bigint_multiply",
			ArithOp::IntegerDivide | ArithOp::Remainder => {
				if flags.contains(NodeFlags::DIVISOR_CHECK) {
					let zero = self.builder.build_int_compare(
						IntPredicate::EQ,
						b,
						self.big_inline(0),
						"",
					)?;
					self.guard(zero, |this| {
						this.raise_runtime("fa_error_division_by_zero", &[])
					})?;
				}
				if op == ArithOp::IntegerDivide {
					"fa_bigint_floor_divide"
				} else {
					"fa_bigint_floor_modulo"
				}
			}
			ArithOp::Power => {
				if flags.contains(NodeFlags::EXPONENT_CHECK) {
					let sign = self.runtime.function("fa_bigint_sign")?;
					let sign = self.call_value(sign, &[b.into()])?.into_int_value();
					let negative = self.builder.build_int_compare(
						IntPredicate::SLT,
						sign,
						sign.get_type().const_zero(),
						"",
					)?;
					self.guard(negative, |this| {
						this.raise_runtime("fa_error_big_negative_exponent", &[b.into()])
					})?;
				}
				"fa_bigint_power"
			}
			ArithOp::Divide => return Err(self.unsupported(node.index(), "`/` on `BigInt`")),
		};
		Ok(self.big_call(name, &[a.into(), b.into()])?.int())
	}

	/// `-a` on a `BigInt`.
	pub(crate) fn big_negate(&mut self, operand: NodeId) -> Result<Val<'ctx>, Error> {
		let Some(value) = self.big_operand(operand)? else {
			return Ok(Val::never());
		};
		self.big_call("fa_bigint_negate", &[value.into()])
	}

	/// `a.abs()` on a `BigInt`.
	pub(crate) fn big_absolute(&mut self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		self.big_call("fa_bigint_abs", &[value.int().into()])
	}

	/// The nearest `Float` of a `BigInt` (`Number(a)` on JavaScript).
	pub(super) fn big_to_float_with(
		&self,
		builder: &Builder<'ctx>,
		value: Val<'ctx>,
	) -> Result<FloatValue<'ctx>, Error> {
		let function = self.runtime.function("fa_bigint_to_f64")?;
		Ok(builder
			.build_call(function, &[value.int().into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a float")
			.into_float_value())
	}

	/// Compares two `BigInt`s: `==` without an operator.
	pub(crate) fn big_compare(
		&self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		op: Option<CompareOp>,
	) -> Result<IntValue<'ctx>, Error> {
		let i32 = self.context.i32_type();
		let Some(op) = op else {
			let function = self.runtime.function("fa_bigint_equal")?;
			let equal = self
				.call_value(function, &[a.int().into(), b.int().into()])?
				.into_int_value();
			return Ok(self.builder.build_int_compare(
				IntPredicate::NE,
				equal,
				i32.const_zero(),
				"",
			)?);
		};
		let function = self.runtime.function("fa_bigint_compare")?;
		let order = self
			.call_value(function, &[a.int().into(), b.int().into()])?
			.into_int_value();
		let predicate = match op {
			CompareOp::Less => IntPredicate::SLT,
			CompareOp::LessOrEqual => IntPredicate::SLE,
			CompareOp::Greater => IntPredicate::SGT,
			CompareOp::GreaterOrEqual => IntPredicate::SGE,
		};
		Ok(self
			.builder
			.build_int_compare(predicate, order, i32.const_zero(), "")?)
	}

	/// `Integer(big)` and the other integer conversions of a `BigInt`:
	/// `ArithmeticError` when it doesn't fit and `checked`.
	pub(crate) fn big_to_int(
		&mut self,
		node: NodeId,
		value: Val<'ctx>,
		target: TypeId,
		to: IntRepr,
		checked: bool,
	) -> Result<Val<'ctx>, Error> {
		let wide = IntRepr {
			bits: 128,
			signed: true,
		};
		let slot = self.alloca(Repr::Int(wide), "big")?;
		let function = self.runtime.function("fa_bigint_to_128")?;
		let fits = self
			.call_value(function, &[value.int().into(), slot.pointer.into()])?
			.into_int_value();
		let raw = self.load(slot)?.int();
		if checked {
			let range = self
				.types()
				.as_int(target)
				.copied()
				.ok_or_else(|| self.unsupported(node.index(), "this conversion"))?;
			let clamp = |bound: I256| {
				let bound = bound.clamp(I256::from(i128::MIN), I256::from(i128::MAX));
				self.const_int(128, bound)
			};
			let below =
				self.builder
					.build_int_compare(IntPredicate::SLT, raw, clamp(range.min), "")?;
			let above =
				self.builder
					.build_int_compare(IntPredicate::SGT, raw, clamp(range.max), "")?;
			let outside = self.builder.build_or(below, above, "")?;
			let not_fitting = self.builder.build_int_compare(
				IntPredicate::EQ,
				fits,
				fits.get_type().const_zero(),
				"",
			)?;
			let failed = self.builder.build_or(outside, not_fitting, "")?;
			let name = self.session.display_type(target);
			let site = self.site(&name, node);
			let word = value.int();
			self.guard(failed, |this| {
				this.raise_runtime("fa_error_big_conversion", &[word.into(), site.into()])
			})?;
		}
		Ok(Val::new(Repr::Int(to), self.int_to(raw, wide, to)?))
	}
}
