//! Strength reduction for a divisor invariant in a loop. For d > 1,
//! m = floor(2^64 / d), q = high64(x*m) underestimates floor(x/d) by at
//! most one. The remainder supplies that one exact correction. Preparing m
//! is pure and nontrapping, so LICM can hoist it even out of conditional code.

use fa_analyzer::analysis::{Argument, CallInfo};
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{ArithOp, View};
use inkwell::IntPredicate;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, IntValue};

use super::values::{IntRepr, Repr};
use super::{Function, Lowering, Val};
use crate::error::Error;
use crate::target::Target;

impl<'ctx> Lowering<'_, 'ctx> {
	/// A scalar immutable binding declared outside the innermost loop. A
	/// constant divisor is left to LLVM's ordinary constant-division lowering,
	/// except for signed remainders by a positive constant: the guarded
	/// nonnegative path avoids the signed remainder's floor adjustment.
	fn invariant_divisor(&self, at: NodeId, mut divisor: NodeId, constant_remainder: bool) -> bool {
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
		if !matches!(data.kind, SymbolKind::Let | SymbolKind::Static) {
			return false;
		}
		let exact = |node: NodeId| {
			self.analysis
				.interval(node)
				.filter(|interval| interval.lo == interval.hi)
				.map(|interval| interval.lo)
		};
		let constant = self
			.types()
			.as_int(data.ty)
			.filter(|ty| ty.min == ty.max)
			.map(|ty| ty.min)
			.or_else(|| exact(divisor))
			.or_else(|| match self.view(data.decl_node) {
				View::Declaration {
					value: Some(value), ..
				} => exact(value),
				_ => None,
			});
		if constant.is_some_and(|value| !constant_remainder || value <= ethnum::I256::ZERO) {
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

	/// After `let d = …` (64 bits, outside any loop over it): the reciprocal
	/// of `d`, so that a division by `d` in a later loop finds it computed
	/// before the loop (LLVM removes it when no division uses it).
	pub(super) fn prepare_reciprocal(
		&mut self,
		symbol: SymbolId,
		value: Val<'ctx>,
	) -> Result<(), Error> {
		if self.target == Target::Wasm
			|| self.analysis.symbols.symbol(symbol).kind != SymbolKind::Let
			|| value.repr != Repr::Int(IntRepr::I64)
		{
			return Ok(());
		}
		let Some(raw) = value.value else {
			return Ok(());
		};
		// only a divisor: the right operand of `//` or `%%`, or an argument
		// (of a helper like `below(random, bound)`)
		let parents = self.parents();
		let divisor = self.uses_of(symbol).into_iter().any(|read| {
			let mut at = read.index();
			loop {
				let parent = parents.get(at).copied().unwrap_or(usize::MAX);
				if parent == usize::MAX {
					return false;
				}
				match self.view(NodeId::new(parent)) {
					View::Group(_) => at = parent,
					View::Arithmetic {
						op: ArithOp::IntegerDivide | ArithOp::Remainder,
						operands,
					} => return operands.get(1).is_some_and(|right| right.index() == at),
					View::Call { callee, .. } => return callee.index() != at,
					_ => return false,
				}
			}
		});
		drop(parents);
		if !divisor {
			return Ok(());
		}
		let reciprocal = self.reciprocal(raw.into_int_value())?;
		let function = self.frame().function;
		self.reciprocals.insert((symbol, function), reciprocal);
		Ok(())
	}

	/// The reciprocal of a divisor node: the one prepared at its declaration
	/// in this function, else one computed here.
	fn divisor_reciprocal(
		&self,
		divisor: NodeId,
		d: IntValue<'ctx>,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let mut node = divisor;
		while let View::Group(inner) = self.view(node) {
			node = inner;
		}
		if let Resolution::Local(symbol) = self.analysis.resolution(node)
			&& let Some(&(safe, m)) = self.reciprocals.get(&(symbol, self.frame().function))
			&& safe.get_type() == d.get_type()
		{
			// the divisor actually used: `x %% -1` divides by 1 instead
			// (`arith.rs`), which the `d == 1` case of the quotient handles
			// whatever m is; otherwise it is the declared value
			let i64 = self.context.i64_type();
			let zero = self
				.builder
				.build_int_compare(IntPredicate::EQ, d, i64.const_zero(), "")?;
			let actual = self
				.builder
				.build_select(zero, i64.const_int(1, false), d, "")?
				.into_int_value();
			return Ok((actual, m));
		}
		self.reciprocal(d)
	}

	/// The divisor made safe (1 for 0; the operation's guard still handles 0)
	/// and m = floor(2^64 / d), with one 64-bit div/rem pair. LLVM 18 never
	/// speculates a division by a variable (`isSafeToSpeculativelyExecute`),
	/// so this is emitted where every iteration runs it (before a branch), and
	/// LICM hoists it as an instruction guaranteed to execute.
	fn reciprocal(&self, d: IntValue<'ctx>) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
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
		Ok((safe, m))
	}

	/// floor(x / d) and x mod d from [`Lowering::reciprocal`].
	fn reciprocal_apply(
		&self,
		x: IntValue<'ctx>,
		(safe, m): (IntValue<'ctx>, IntValue<'ctx>),
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let b = &self.builder;
		let i64 = self.context.i64_type();
		let zero = i64.const_zero();
		let one = i64.const_int(1, false);
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
		if !self.invariant_divisor(node, *divisor, false) {
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
		let reciprocal = self.divisor_reciprocal(*divisor, d)?;
		let (_, r) = self.reciprocal_apply(x, reciprocal)?;
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
			|| !self.invariant_divisor(
				node,
				operands.get(1).expect("two operands"),
				op == ArithOp::Remainder,
			) {
			return Ok(None);
		}
		let i64 = self.context.i64_type();
		let reciprocal = self.divisor_reciprocal(operands.get(1).expect("two operands"), b)?;
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
		let (q, r) = self.reciprocal_apply(a, reciprocal)?;
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
