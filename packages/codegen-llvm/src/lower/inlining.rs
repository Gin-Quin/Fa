//! Inline the small scalar base case of a direct recursive call. The expensive
//! branch remains a call. This avoids a call frame for leaves without unrolling
//! the recursive computation or duplicating argument evaluation.

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum};

use super::values::Repr;
use super::{FAST_CALLING_CONVENTION, Function, Lowering};
use crate::error::Error;

impl<'ctx> Lowering<'_, 'ctx> {
	/// A single expression in braces, or the expression returned by it.
	fn base_expression(&self, mut node: NodeId) -> Option<NodeId> {
		loop {
			match self.view(node) {
				View::Block { statements } if statements.len() == 1 => node = statements.first()?,
				View::Group(inner) => node = inner,
				View::Return(Some(inner)) => node = inner,
				View::Block { .. } | View::Return(_) => return None,
				_ => return Some(node),
			}
		}
	}

	/// Only scalar operations and reads of parameters. No effects, captures,
	/// allocations, local declarations, or recursive evaluation of the guard.
	fn small_base_expression(&self, node: NodeId, function: &Function<'ctx>) -> bool {
		let mut stack = vec![node.index()];
		let mut count = 0;
		while let Some(at) = stack.pop() {
			count += 1;
			if count > 16 {
				return false;
			}
			match self.view(NodeId::new(at)) {
				View::Integer(_)
				| View::Float(_)
				| View::Boolean(_)
				| View::Group(_)
				| View::Arithmetic { .. }
				| View::Negate(_)
				| View::Comparison { .. }
				| View::Equality { .. }
				| View::Not(_)
				| View::Logic { .. } => {}
				View::Name(_) if matches!(self.analysis.resolution(NodeId::new(at)), Resolution::Local(symbol) if function.info.params.iter().any(|p| p.symbol == symbol)) =>
					{}
				_ => return false,
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		true
	}

	pub(super) fn recursive_base_call(
		&mut self,
		function: &Function<'ctx>,
		target: inkwell::values::FunctionValue<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let mut shortcut = None;
		if self.frame().owner == Some(function.declaration)
			&& self.functions.values().any(|f| f.value == function.value)
			&& !self.tasks.asynchronous
			&& !function.lambda
			&& !function.throws
			&& !self.needs_drop(function.ret)
			&& function.ret.has_value()
			&& function.params.len() == arguments.len()
			&& function.params.iter().all(|(r, default)| {
				!default && matches!(r, Repr::Int(_) | Repr::Bool | Repr::F32 | Repr::F64)
			}) && function.by_pointer.iter().all(|p| !p)
			&& !function
				.info
				.sig
				.is_some_and(|sig| self.types().sig(sig).is_async)
			&& let Some(body) = self.base_expression(function.info.body)
			&& let View::If {
				condition,
				then_body,
				else_body: Some(_),
				compile_time: false,
			} = self.view(body)
			&& let Some(leaf) = self.base_expression(then_body)
			&& self.small_base_expression(condition, function)
			&& self.small_base_expression(leaf, function)
		{
			// `range` metadata belongs to the call, not the phi we make below.
			let ranged_result = matches!(function.ret, Repr::Int(_))
				&& function.info.sig.is_some_and(|sig| {
					self.types()
						.as_int(self.types().sig(sig).ret)
						.is_some_and(|range| !range.is_full())
				});
			if !ranged_result {
				shortcut = Some((condition, leaf));
			}
		}
		let Some((condition, leaf)) = shortcut else {
			let call = self.builder.build_call(target, arguments, "")?;
			call.set_call_convention(FAST_CALLING_CONVENTION);
			return Ok(call.try_as_basic_value().basic());
		};
		// Arguments have already been evaluated, once, in source order. Bind
		// the callee's scalar parameters to those exact values for the guard.
		let mut saved = Vec::new();
		for ((param, (repr, _)), argument) in function
			.info
			.params
			.iter()
			.zip(&function.params)
			.zip(arguments)
		{
			let slot = self.alloca(*repr, "base_argument")?;
			let raw = BasicValueEnum::try_from(*argument).expect("a scalar argument");
			self.builder.build_store(slot.pointer, raw)?;
			saved.push((param.symbol, self.slots.insert(param.symbol, slot)));
		}
		let result = (|| {
			let test = self
				.expr(condition)?
				.value
				.expect("a scalar guard")
				.into_int_value();
			let base = self.append_block("recursive_base");
			let recurse = self.append_block("recursive_call");
			let end = self.append_block("recursive_result");
			self.builder.build_conditional_branch(test, base, recurse)?;
			self.builder.position_at_end(base);
			let leaf = self.expr(leaf)?;
			let leaf = self
				.convert(leaf, function.ret, condition.index())?
				.value
				.expect("a scalar leaf");
			let base_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(recurse);
			let call = self.builder.build_call(target, arguments, "")?;
			call.set_call_convention(FAST_CALLING_CONVENTION);
			let recursive = call.try_as_basic_value().basic().expect("a scalar result");
			let recursive_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(end);
			let result = self.builder.build_phi(
				self.llvm_type(function.ret).as_basic_type_enum(),
				"base_or_call",
			)?;
			result.add_incoming(&[(&leaf, base_end), (&recursive, recursive_end)]);
			Ok(Some(result.as_basic_value()))
		})();
		for (symbol, slot) in saved {
			if let Some(slot) = slot {
				self.slots.insert(symbol, slot);
			} else {
				self.slots.remove(&symbol);
			}
		}
		result
	}
}
