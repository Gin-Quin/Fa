//! A bounded nonnegative decimal appended directly to a writable String.
//! Two digits share one division and one table load; unaligned stores are
//! explicit because the preceding text may have any byte length.

use inkwell::values::{BasicValue, IntValue, PointerValue};

use super::Lowering;
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn write_small_decimal(
		&mut self,
		destination: PointerValue<'ctx>,
		wide: IntValue<'ctx>,
		count: u32,
	) -> Result<(), Error> {
		debug_assert!(count <= 9);
		let i32 = self.context.i32_type();
		// The caller's < 10^count branch proves this conversion exact.
		let value = self
			.builder
			.build_int_truncate_or_bit_cast(wide, i32, "decimal")?;
		if !count.is_multiple_of(2) {
			let power = 10u64.pow(count - 1);
			let leading = if power == 1 {
				value
			} else {
				self.builder
					.build_int_unsigned_div(value, i32.const_int(power, false), "")?
			};
			let byte = self
				.builder
				.build_int_truncate(leading, self.context.i8_type(), "")?;
			let byte = self.builder.build_or(
				byte,
				self.context.i8_type().const_int(b'0' as u64, false),
				"",
			)?;
			self.builder.build_store(destination, byte)?;
		}
		let pairs: Vec<u8> = (0..100u8)
			.flat_map(|value| [b'0' + value / 10, b'0' + value % 10])
			.collect();
		let table = self.constant_bytes(&pairs);
		for position in (count % 2..count).step_by(2) {
			let power = 10u64.pow(count - 2 - position);
			let mut pair = if power == 1 {
				value
			} else {
				self.builder
					.build_int_unsigned_div(value, i32.const_int(power, false), "")?
			};
			if position != 0 {
				pair = self
					.builder
					.build_int_unsigned_rem(pair, i32.const_int(100, false), "")?;
			}
			let offset = self
				.builder
				.build_int_mul(pair, i32.const_int(2, false), "")?;
			let offset = self
				.builder
				.build_int_z_extend_or_bit_cast(offset, self.usize, "")?;
			let source = unsafe {
				self.builder
					.build_in_bounds_gep(self.context.i8_type(), table, &[offset], "")?
			};
			let pair = self
				.builder
				.build_load(self.context.i16_type(), source, "digit_pair")?;
			pair.as_instruction_value()
				.expect("a load")
				.set_alignment(1)
				.map_err(|error| Error::Llvm(error.to_string()))?;
			let target = unsafe {
				self.builder.build_in_bounds_gep(
					self.context.i8_type(),
					destination,
					&[self.usize.const_int(u64::from(position), false)],
					"",
				)?
			};
			self.builder
				.build_store(target, pair)?
				.set_alignment(1)
				.map_err(|error| Error::Llvm(error.to_string()))?;
		}
		Ok(())
	}
}
