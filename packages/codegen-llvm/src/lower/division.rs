//! Strength reduction for a divisor invariant in a loop. For d > 1,
//! m = floor(2^64 / d), q = high64(x*m) underestimates floor(x/d) by at
//! most one. The remainder supplies that one exact correction. Preparing m
//! is pure and nontrapping, so LICM can hoist it even out of conditional code.

use fa_analyzer::analysis::{Argument, CallInfo};
use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{ArithOp, View};
use inkwell::IntPredicate;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, IntValue};

use super::values::Repr;
use super::{Function, Lowering};
use crate::error::Error;
use crate::target::Target;

impl<'ctx> Lowering<'_, 'ctx> {
	/// A scalar immutable binding declared outside the innermost loop. A
	/// constant divisor is left to LLVM's ordinary constant-division lowering.
	fn invariant_divisor(&self, at: NodeId, mut divisor: NodeId) -> bool {
		if self.target == Target::Wasm || self.frame().loops.is_empty() {
			return false;
		}
		while let View::Group(inner) = self.view(divisor) {
			divisor = inner;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(divisor) else {
			return false;
		};
		let data = self.analysis.symbols.symbol(symbol);
		if !matches!(data.kind, SymbolKind::Let | SymbolKind::Static)
			|| self.types().as_int(data.ty).is_some_and(|t| t.min == t.max)
		{
			return false;
		}
		let point = self.tree.spans[at.index()];
		let declaration = self.tree.spans[data.decl_node.index()];
		let bodies = self.loop_bodies.get_or_init(|| {
			(0..self.tree.nodes.len())
				.filter_map(|index| match self.view(NodeId::new(index)) {
					View::For { body, .. }
					| View::While { body, .. }
					| View::WhileLet { body, .. }
					| View::Loop(body) => Some(self.tree.spans[body.index()]),
					_ => None,
				})
				.collect()
		});
		bodies
			.iter()
			.filter(|span| span.start <= point.start && span.end >= point.end)
			.min_by_key(|span| span.end - span.start)
			.is_some_and(|span| declaration.end <= span.start)
	}

	/// Both results of an exact unsigned division. m is prepared with one
	/// 64-bit div/rem pair rather than a 128-bit division. d=0 is made safe
	/// here for speculation; the original operation's guard still handles it.
	fn reciprocal_div_rem(
		&self,
		x: IntValue<'ctx>,
		d: IntValue<'ctx>,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let b = &self.builder;
		let i64 = self.context.i64_type();
		let zero = i64.const_zero();
		let one = i64.const_int(1, false);
		let safe = b
			.build_select(
				b.build_int_compare(IntPredicate::EQ, d, zero, "")?,
				one,
				d,
				"",
			)?
			.into_int_value();
		let m0 = b.build_int_unsigned_div(i64.const_all_ones(), safe, "reciprocal")?;
		let mr = b.build_int_unsigned_rem(i64.const_all_ones(), safe, "")?;
		let last = b.build_int_sub(safe, one, "")?;
		let exact = b.build_int_compare(IntPredicate::EQ, mr, last, "")?;
		let extra = b.build_int_z_extend(exact, i64, "")?;
		let m = b.build_int_add(m0, extra, "")?;
		let i128 = self.context.i128_type();
		let wide_x = b.build_int_z_extend(x, i128, "")?;
		let wide_m = b.build_int_z_extend(m, i128, "")?;
		let product = b.build_int_mul(wide_x, wide_m, "")?;
		let high = b.build_right_shift(product, i128.const_int(64, false), false, "")?;
		let q = b.build_int_truncate(high, i64, "")?;
		let d_one = b.build_int_compare(IntPredicate::EQ, safe, one, "")?;
		let q = b.build_select(d_one, x, q, "")?.into_int_value();
		let used = b.build_int_mul(q, safe, "")?;
		let r = b.build_int_sub(x, used, "")?;
		let correction = b.build_int_compare(IntPredicate::UGE, r, safe, "")?;
		let q = b.build_int_add(q, b.build_int_z_extend(correction, i64, "")?, "")?;
		let subtract = b.build_select(correction, safe, zero, "")?.into_int_value();
		let r = b.build_int_sub(r, subtract, "")?;
		Ok((q, r))
	}

	/// A direct helper `random %% bound` called with an invariant bound.
	/// The arguments and their conversions have already been evaluated.
	pub(super) fn invariant_remainder_call(
		&mut self,
		function: &Function<'ctx>,
		node: NodeId,
		info: &CallInfo,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		if !self.functions.values().any(|f| f.value == function.value)
			|| function.lambda
			|| function.throws
			|| function.params.len() != 2
			|| arguments.len() != 2
			|| function.by_pointer.iter().any(|p| *p)
			|| function.params.iter().any(|(_, d)| *d)
			|| self.tasks.asynchronous
			|| function
				.info
				.sig
				.is_some_and(|sig| self.types().sig(sig).is_async)
		{
			return Ok(None);
		}
		let (Repr::Int(x), Repr::Int(d), Repr::Int(result)) =
			(function.params[0].0, function.params[1].0, function.ret)
		else {
			return Ok(None);
		};
		if x.bits != 64
			|| x.signed
			|| d.bits != 64
			|| !d.signed
			|| result.bits != 64
			|| !result.signed
		{
			return Ok(None);
		}
		let bound = self
			.analysis
			.symbols
			.symbol(function.info.params[1].symbol)
			.ty;
		if !self
			.types()
			.as_int(bound)
			.is_some_and(|t| t.min > ethnum::I256::ZERO)
		{
			return Ok(None);
		}
		let Some(Argument::Given(divisor)) = info.arguments.get(1) else {
			return Ok(None);
		};
		if !self.invariant_divisor(node, *divisor) {
			return Ok(None);
		}
		let mut body = function.info.body;
		while let View::Block { statements } = self.view(body) {
			if statements.len() != 1 {
				return Ok(None);
			}
			body = statements.first().expect("one expression");
		}
		let View::Arithmetic {
			op: ArithOp::Remainder,
			operands,
		} = self.view(body)
		else {
			return Ok(None);
		};
		if operands.len() != 2
			|| !operands
				.iter()
				.zip(&function.info.params)
				.all(|(node, param)| {
					self.analysis.resolution(node) == Resolution::Local(param.symbol)
				}) {
			return Ok(None);
		}
		let x = BasicValueEnum::try_from(arguments[0])
			.expect("an argument")
			.into_int_value();
		let d = BasicValueEnum::try_from(arguments[1])
			.expect("an argument")
			.into_int_value();
		let (_, r) = self.reciprocal_div_rem(x, d)?;
		Ok(Some(r.into()))
	}

	/// Nonnegative signed division has the same fast quotient. Signed values
	/// outside that case keep the general floor-division implementation.
	pub(super) fn invariant_signed_division(
		&mut self,
		node: NodeId,
		op: ArithOp,
		a: IntValue<'ctx>,
		b: IntValue<'ctx>,
	) -> Result<Option<IntValue<'ctx>>, Error> {
		let View::Arithmetic { operands, .. } = self.view(node) else {
			return Ok(None);
		};
		if operands.len() != 2
			|| !self.invariant_divisor(node, operands.get(1).expect("two operands"))
		{
			return Ok(None);
		}
		let i64 = self.context.i64_type();
		let positive =
			self.builder
				.build_int_compare(IntPredicate::SGT, b, i64.const_zero(), "")?;
		let nonnegative =
			self.builder
				.build_int_compare(IntPredicate::SGE, a, i64.const_zero(), "")?;
		let fast = self.builder.build_and(positive, nonnegative, "")?;
		let quick = self.append_block("invariant_divisor");
		let general = self.append_block("signed_divisor");
		let end = self.append_block("divided");
		self.builder
			.build_conditional_branch(fast, quick, general)?;
		self.builder.position_at_end(quick);
		let (q, r) = self.reciprocal_div_rem(a, b)?;
		let quick_result = if op == ArithOp::IntegerDivide { q } else { r };
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(general);
		let q = self.builder.build_int_signed_div(a, b, "")?;
		let r = self.builder.build_int_signed_rem(a, b, "")?;
		let nonzero = self
			.builder
			.build_int_compare(IntPredicate::NE, r, i64.const_zero(), "")?;
		let signs = self.builder.build_xor(r, b, "")?;
		let differ =
			self.builder
				.build_int_compare(IntPredicate::SLT, signs, i64.const_zero(), "")?;
		let adjust = self.builder.build_and(nonzero, differ, "")?;
		let original = if op == ArithOp::IntegerDivide { q } else { r };
		let adjusted = if op == ArithOp::IntegerDivide {
			self.builder.build_int_sub(q, i64.const_int(1, false), "")?
		} else {
			self.builder.build_int_add(r, b, "")?
		};
		let original = self
			.builder
			.build_select(adjust, adjusted, original, "")?
			.into_int_value();
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let phi = self.builder.build_phi(i64, "quotient_or_remainder")?;
		phi.add_incoming(&[(&quick_result, quick), (&original, general)]);
		Ok(Some(phi.as_basic_value().into_int_value()))
	}
}
