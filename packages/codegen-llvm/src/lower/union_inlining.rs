//! Expand a scalar leaf of a directly recursive union traversal at its
//! recursive calls, as the callee's dispatch would select it: the leaf member
//! is tested on already evaluated arguments; other members remain calls. The
//! leaf is scalar arithmetic over its fields, parameters and constants; no
//! call, allocation, cleanup or `try` belongs to the expanded arm.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::lower::when::{Decision, DecisionList, Test, arm_bindings, decision_list};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue};

use super::values::{Repr, Val};
use super::{FAST_CALLING_CONVENTION, Function, Lowering};
use crate::error::Error;

fn scalar(repr: Repr) -> bool {
	matches!(repr, Repr::Int(_) | Repr::Bool | Repr::F32 | Repr::F64)
}

/// Operations of a leaf: none calls, throws or allocates. A failed
/// arithmetic check stops the program the same way in the caller.
const LEAF_OPERATIONS: usize = 16;

impl<'ctx> Lowering<'_, 'ctx> {
	/// Scalar arithmetic over the operands of [`Self::scalar_union_operand`].
	fn scalar_union_leaf(
		&self,
		node: NodeId,
		function: &Function<'ctx>,
		bindings: &[SymbolId],
		data: TypeId,
	) -> bool {
		let mut stack = vec![node];
		let mut count = 0;
		while let Some(node) = stack.pop() {
			count += 1;
			if count > LEAF_OPERATIONS || !scalar(self.node_repr(node).unwrap_or(Repr::Never)) {
				return false;
			}
			match self.view(node) {
				View::Group(_) | View::Arithmetic { .. } | View::Negate(_) => self.tree.nodes
					[node.index()]
				.for_each_child(|child| stack.push(NodeId::new(child))),
				_ if self.scalar_union_operand(node, function, bindings, data) => {}
				_ => return false,
			}
		}
		true
	}

	fn scalar_union_operand(
		&self,
		node: NodeId,
		function: &Function<'ctx>,
		bindings: &[SymbolId],
		data: TypeId,
	) -> bool {
		match self.view(node) {
			View::Integer(_) | View::Float(_) | View::Boolean(_) => true,
			View::Name(_) => {
				let local =
					match self.analysis.resolution(node) {
						Resolution::Local(symbol) => {
							bindings.contains(&symbol)
								|| function.info.params.iter().zip(&function.params).any(
									|(param, (repr, _))| param.symbol == symbol && scalar(*repr),
								)
						}
						_ => false,
					};
				let constant =
					self.analysis
						.constant(node)
						.or_else(|| match self.analysis.resolution(node) {
							Resolution::Local(symbol) => self.analysis.module_constant(symbol),
							_ => None,
						});
				local
					|| matches!(
						constant,
						Some(
							fa_interpreter::Value::Integer(_)
								| fa_interpreter::Value::Float(_)
								| fa_interpreter::Value::Boolean(_)
						)
					)
			}
			View::Access { target, members } if members.len() == 1 => {
				let payload = self.node_repr(target).ok() == Some(Repr::Object(data))
					|| matches!(self.types().kind(self.type_of(target)), TypeKind::Member(owner, index) if self.member_data(owner, index) == Some(data));
				matches!(self.view(target), View::Name(_))
					&& matches!(self.analysis.resolution(target), Resolution::Local(symbol) if bindings.contains(&symbol))
					&& payload && matches!(
					self.analysis
						.resolution(members.first().expect("one field")),
					Resolution::Field(..)
				)
			}
			_ => false,
		}
	}

	/// The first arm whose member has only scalar fields and whose value is a
	/// scalar leaf. Earlier arms only test other members, which exclude it:
	/// testing its member first selects the same arm.
	fn union_leaf_arm(
		&self,
		list: &DecisionList<'_>,
		function: &Function<'ctx>,
	) -> Option<(usize, TypeId, Vec<SymbolId>, NodeId)> {
		for (position, decision) in list.decisions.iter().enumerate() {
			let [Test::IsType(member)] = decision.tests.as_slice() else {
				return None;
			};
			if let Some((bindings, leaf)) = self.union_leaf(decision, *member, function) {
				return Some((position, *member, bindings, leaf));
			}
		}
		None
	}

	fn union_leaf(
		&self,
		decision: &Decision<'_>,
		member: TypeId,
		function: &Function<'ctx>,
	) -> Option<(Vec<SymbolId>, NodeId)> {
		let TypeKind::Member(nominal, index) = self.types().kind(member) else {
			return None;
		};
		let data = self.member_data(nominal, index)?;
		if self.types().fields_of(data).is_empty()
			|| self.destroy_function(data).is_some()
			|| !self.object_fields(data).iter().all(|r| scalar(*r))
		{
			return None;
		}
		let bindings: Vec<_> = decision
			.binding
			.map(|binding| arm_bindings(self.tree, self.analysis, binding))
			.unwrap_or_default()
			.into_iter()
			.map(|binding| binding.symbol)
			.collect();
		if bindings
			.iter()
			.any(|symbol| self.shared.contains_key(symbol) || self.references.contains_key(symbol))
		{
			return None;
		}
		let leaf = self.base_expression(decision.value)?;
		self.scalar_union_leaf(leaf, function, &bindings, data)
			.then_some((bindings, leaf))
	}

	pub(super) fn recursive_union_base(
		&mut self,
		function: &Function<'ctx>,
		target: FunctionValue<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		if self.frame().owner != Some(function.declaration)
			|| !self.functions.values().any(|f| f.value == function.value)
			|| self.tasks.asynchronous
			|| function.lambda
			|| !function.captures.is_empty()
			|| !function.bindings.is_empty()
			|| !function.values.is_empty()
			|| !scalar(function.ret)
			|| function.params.len() != arguments.len()
			|| function
				.params
				.iter()
				.any(|(r, default)| *default || (!scalar(*r) && *r != Repr::Union))
			|| function.by_pointer.iter().any(|p| *p)
			|| function.borrowed.iter().any(|p| *p)
			|| function.info.params.iter().any(|p| {
				self.shared.contains_key(&p.symbol)
					|| self.references.contains_key(&p.symbol)
					|| self.is_moved_parameter(p.symbol)
			}) || function.info.sig.is_some_and(|sig| {
			self.types().sig(sig).is_async
				|| (self.overflow_checks()
					&& self
						.types()
						.as_int(self.types().sig(sig).ret)
						.is_some_and(|range| !range.is_full()))
		}) {
			return Ok(None);
		}
		let Some(body) = self.base_expression(function.info.body) else {
			return Ok(None);
		};
		let Ok(list) = decision_list(self.tree, self.analysis, body) else {
			return Ok(None);
		};
		if !matches!(self.view(list.subject), View::Name(_)) {
			return Ok(None);
		}
		let Resolution::Local(subject_symbol) = self.analysis.resolution(list.subject) else {
			return Ok(None);
		};
		let Some(parameter) = function
			.info
			.params
			.iter()
			.position(|p| p.symbol == subject_symbol)
		else {
			return Ok(None);
		};
		if function.params[parameter].0 != Repr::Union {
			return Ok(None);
		}
		let Some((position, member, bindings, leaf)) = self.union_leaf_arm(&list, function) else {
			return Ok(None);
		};
		let first = &list.decisions[position];
		// Do not bypass a narrowing return boundary. The original body
		// remains responsible for its checked storage/range conversion.
		let lossless = match (self.node_repr(leaf)?, function.ret) {
			(Repr::Int(from), Repr::Int(to)) => {
				from.bits <= to.bits
					&& (!from.signed || to.signed)
					&& (from.bits < to.bits || from.signed == to.signed)
			}
			(Repr::F32, Repr::F64) => true,
			(from, to) => from == to,
		};
		if !lossless {
			return Ok(None);
		}

		let mut saved = Vec::new();
		for ((param, (repr, _)), argument) in function
			.info
			.params
			.iter()
			.zip(&function.params)
			.zip(arguments)
		{
			let slot = self.alloca(*repr, "leaf_argument")?;
			self.builder.build_store(
				slot.pointer,
				BasicValueEnum::try_from(*argument).expect("a value argument"),
			)?;
			saved.push((param.symbol, self.slots.insert(param.symbol, slot)));
		}
		for &symbol in &bindings {
			let repr = self.repr_of(self.analysis.symbols.symbol(symbol).ty, leaf.index())?;
			let slot = self.alloca(repr, "leaf_binding")?;
			saved.push((symbol, self.slots.insert(symbol, slot)));
		}
		let result = (|| {
			let subject = Val::new(
				Repr::Union,
				BasicValueEnum::try_from(arguments[parameter]).expect("a union argument"),
			);
			let test = self.box_is_typed(subject, member, Some(list.subject_type))?;
			let base = self.append_block("union_leaf");
			let recurse = self.append_block("union_recursive_call");
			let end = self.append_block("union_recursive_result");
			self.builder.build_conditional_branch(test, base, recurse)?;
			self.builder.position_at_end(base);
			if let Some(binding) = first.binding {
				self.bind_arm(subject, binding, &first.tests)?;
			}
			let value = self.expr(leaf)?;
			let value = self
				.convert(value, function.ret, leaf.index())?
				.value
				.expect("a scalar leaf");
			let raw_type = target
				.get_type()
				.get_return_type()
				.expect("a scalar result ABI");
			// A throwing function returns its value with a null error.
			let value = if function.throws {
				self.builder
					.build_insert_value(raw_type.into_struct_type().const_zero(), value, 0, "")?
					.as_basic_value_enum()
			} else {
				value
			};
			let base_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(recurse);
			let call = self.builder.build_call(target, arguments, "")?;
			call.set_call_convention(FAST_CALLING_CONVENTION);
			let recursive = call.try_as_basic_value().basic().expect("a scalar result");
			let recursive_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(end)?;
			self.builder.position_at_end(end);
			let phi = self.builder.build_phi(raw_type, "union_base_or_call")?;
			phi.add_incoming(&[(&value, base_end), (&recursive, recursive_end)]);
			Ok(Some(phi.as_basic_value()))
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
