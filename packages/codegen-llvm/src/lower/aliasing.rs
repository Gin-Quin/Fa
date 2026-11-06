//! Disjoint references into one array, told to LLVM (scoped alias metadata).
//!
//! In `for body, i in items { for j in i + 1..items.length { use other =
//! items[j] … } }`, `body` designates item `i` and `other` item `j > i`: they
//! never overlap, but LLVM can't know it, so a write through `other` makes
//! it reload `body`'s fields (nbody's inner loop). The pattern is read from
//! the syntax: the variable and the index of a `for` over an array local,
//! an inner `for` over a range starting at the index plus a positive
//! literal, and a `use` of the array at the inner variable (the array can't
//! move meanwhile: the analyzer invalidates references when it changes).
//!
//! Every load and store through each of the two references in the outer
//! loop's body then gets its own alias scope and the other's as `noalias`,
//! and each round of the outer loop declares the two scopes again
//! (`llvm.experimental.noalias.scope.decl`), so that the claim is made per
//! round: item `i` of one round may be item `j` of an earlier one.

use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{ArithOp, View};
use fa_analyzer::types::int::LENGTH_MAX;
use inkwell::basic_block::BasicBlock;
use inkwell::values::{
	AsValueRef, BasicMetadataValueEnum, InstructionOpcode, MetadataValue, PointerValue,
};

use crate::error::Error;
use crate::lower::Lowering;

/// The two scopes of a pair of disjoint references.
#[derive(Copy, Clone)]
pub(crate) struct DisjointScopes<'ctx> {
	/// `!{scope}` of each reference.
	first: MetadataValue<'ctx>,
	second: MetadataValue<'ctx>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The `use` of `items[j]` in a nested `for j in index + k..…` (k ≥ 1)
	/// of the body of `for variable, index in items` (`node`): its symbol.
	pub(crate) fn disjoint_reference(&self, node: NodeId, body: NodeId) -> Option<SymbolId> {
		let View::For { iterable, .. } = self.view(node) else {
			return None;
		};
		let Resolution::Local(items) = self.analysis.resolution(iterable) else {
			return None;
		};
		let index = self.analysis.symbols.symbol_of_decl(node)?;
		let mut stack = vec![body.index()];
		while let Some(current) = stack.pop() {
			let current_node = NodeId::new(current);
			match self.view(current_node) {
				// the array changes: no claim
				View::Assignment { target, .. } | View::CompoundAssignment { target, .. }
					if self.analysis.resolution(target) == Resolution::Local(items) =>
				{
					return None;
				}
				View::For {
					pattern: Some(pattern),
					iterable: inner,
					body: inner_body,
					..
				} => {
					if let Some(found) =
						self.disjoint_inner(items, index, pattern, inner, inner_body)
					{
						return Some(found);
					}
				}
				_ => {}
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		None
	}

	/// `for j in index + k..… { … use other = items[j] … }`.
	fn disjoint_inner(
		&self,
		items: SymbolId,
		index: SymbolId,
		pattern: NodeId,
		iterable: NodeId,
		body: NodeId,
	) -> Option<SymbolId> {
		let View::Range {
			start: Some(start), ..
		} = self.view(iterable)
		else {
			return None;
		};
		let View::Arithmetic {
			op: ArithOp::Add,
			operands,
		} = self.view(start)
		else {
			return None;
		};
		let operands: Vec<NodeId> = operands.iter().collect();
		let [base, offset] = operands.as_slice() else {
			return None;
		};
		if self.analysis.resolution(*base) != Resolution::Local(index) {
			return None;
		}
		let View::Integer(offset) = self.view(*offset) else {
			return None;
		};
		// `i` is an array position, at most LENGTH_MAX. In unchecked builds
		// a larger positive offset could wrap below `i`, including to a
		// negative index normalizing back to the same element. Metadata
		// cannot rely on the overflow stopping the program.
		if offset < ethnum::I256::ONE || offset > ethnum::I256::from(i64::MAX) - LENGTH_MAX {
			return None;
		}
		let variable = self.analysis.symbols.symbol_of_decl(pattern)?;
		let mut stack = vec![body.index()];
		while let Some(current) = stack.pop() {
			if let View::Use {
				value,
				mutable: false,
				..
			} = self.view(NodeId::new(current))
				&& let View::Index {
					target,
					index: position,
				} = self.view(value)
				&& self.analysis.resolution(target) == Resolution::Local(items)
				&& self.analysis.resolution(position) == Resolution::Local(variable)
			{
				return self.analysis.symbols.symbol_of_decl(NodeId::new(current));
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		None
	}

	/// Two new alias scopes in a new domain.
	pub(crate) fn disjoint_scopes(&mut self) -> DisjointScopes<'ctx> {
		self.alias_domains += 1;
		let number = self.alias_domains;
		let domain = self.context.metadata_node(&[self
			.context
			.metadata_string(&format!("fa.domain.{number}"))
			.into()]);
		let scope = |name: &str| {
			let scope = self.context.metadata_node(&[
				self.context
					.metadata_string(&format!("fa.scope.{number}.{name}"))
					.into(),
				domain.into(),
			]);
			self.context.metadata_node(&[scope.into()])
		};
		DisjointScopes {
			first: scope("first"),
			second: scope("second"),
		}
	}

	/// Declares the two scopes for the round of the loop that starts here.
	pub(crate) fn declare_disjoint_scopes(
		&self,
		scopes: DisjointScopes<'ctx>,
	) -> Result<(), Error> {
		let intrinsic =
			inkwell::intrinsics::Intrinsic::find("llvm.experimental.noalias.scope.decl")
				.ok_or_else(|| Error::Llvm("no `llvm.experimental.noalias.scope.decl`".into()))?;
		let function = intrinsic
			.get_declaration(self.module, &[])
			.ok_or_else(|| Error::Llvm("no `llvm.experimental.noalias.scope.decl`".into()))?;
		for list in [scopes.first, scopes.second] {
			self.builder.build_call(
				function,
				&[BasicMetadataValueEnum::MetadataValue(list)],
				"",
			)?;
		}
		Ok(())
	}

	/// Tags the loads and stores of `blocks` whose address derives from one
	/// of the two references.
	pub(crate) fn tag_disjoint(
		&self,
		blocks: &[BasicBlock<'ctx>],
		bases: (PointerValue<'ctx>, PointerValue<'ctx>),
		scopes: DisjointScopes<'ctx>,
	) -> Result<(), Error> {
		let scope_kind = self.context.get_kind_id("alias.scope");
		let noalias_kind = self.context.get_kind_id("noalias");
		for block in blocks {
			let mut instruction = block.get_first_instruction();
			while let Some(at) = instruction {
				let address = match at.get_opcode() {
					InstructionOpcode::Load => at.get_operand(0),
					InstructionOpcode::Store => at.get_operand(1),
					_ => None,
				};
				if let Some(address) = address.and_then(|operand| operand.value()) {
					let address = address.into_pointer_value();
					let tags = if derives_from(address, bases.0) {
						Some((scopes.first, scopes.second))
					} else if derives_from(address, bases.1) {
						Some((scopes.second, scopes.first))
					} else {
						None
					};
					if let Some((own, other)) = tags {
						at.set_metadata(own, scope_kind)
							.map_err(|error| Error::Llvm(error.to_string()))?;
						at.set_metadata(other, noalias_kind)
							.map_err(|error| Error::Llvm(error.to_string()))?;
					}
				}
				instruction = at.get_next_instruction();
			}
		}
		Ok(())
	}
}

/// Whether an address is `base`, or computed from it by `getelementptr`s.
fn derives_from(pointer: PointerValue<'_>, base: PointerValue<'_>) -> bool {
	let mut current = pointer;
	loop {
		if current.as_value_ref() == base.as_value_ref() {
			return true;
		}
		let Some(instruction) = current.as_instruction() else {
			return false;
		};
		if instruction.get_opcode() != InstructionOpcode::GetElementPtr {
			return false;
		}
		let Some(next) = instruction
			.get_operand(0)
			.and_then(|operand| operand.value())
		else {
			return false;
		};
		current = next.into_pointer_value();
	}
}
