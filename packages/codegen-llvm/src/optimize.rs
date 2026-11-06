//! Expose the return paths hidden by a final phi before tail-recursion
//! elimination. `prefix + (child?.walk() ?? 0)` otherwise keeps the last
//! recursive call behind a phi, so LLVM cannot turn it into an accumulator
//! loop. Only a short integer return suffix is duplicated, on its existing
//! incoming edges: no expression or effect is evaluated on an extra path.

use inkwell::basic_block::BasicBlock;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::TargetMachine;
use inkwell::values::{
	AnyValueEnum, AsValueRef, BasicValue, BasicValueEnum, FunctionValue, InstructionOpcode,
	InstructionValue, PhiValue,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Error, OptLevel, Options};

pub(crate) fn expose_tail_recursion(
	module: &Module<'_>,
	machine: &TargetMachine,
	options: &Options,
) -> Result<(), Error> {
	if options.size || !matches!(options.opt_level, OptLevel::O2 | OptLevel::O3) {
		return Ok(());
	}
	if duplicate_return_paths(module) {
		module.verify()?;
		module.run_passes(
			"function(tailcallelim,instcombine<no-verify-fixpoint>,simplifycfg)",
			machine,
			PassBuilderOptions::create(),
		)?;
	}
	Ok(())
}

fn scalar_suffix(opcode: InstructionOpcode) -> bool {
	matches!(
		opcode,
		InstructionOpcode::Add
			| InstructionOpcode::Sub
			| InstructionOpcode::Mul
			| InstructionOpcode::And
			| InstructionOpcode::Or
			| InstructionOpcode::Xor
			| InstructionOpcode::Shl
			| InstructionOpcode::LShr
			| InstructionOpcode::AShr
			| InstructionOpcode::Trunc
			| InstructionOpcode::ZExt
			| InstructionOpcode::SExt
			| InstructionOpcode::ICmp
			| InstructionOpcode::Select
			| InstructionOpcode::Freeze
			| InstructionOpcode::Return
	)
}

fn used_only_here(instruction: InstructionValue<'_>, block: BasicBlock<'_>) -> bool {
	let mut next = instruction.get_first_use();
	while let Some(usage) = next {
		// Inkwell represents an integer-producing instruction as IntValue,
		// and only a void-producing one as InstructionValue.
		let user = match usage.get_user() {
			AnyValueEnum::InstructionValue(instruction) => Some(instruction),
			other => BasicValueEnum::try_from(other)
				.ok()
				.and_then(|value| value.as_instruction_value()),
		};
		if !user.is_some_and(|user| user.get_parent() == Some(block)) {
			return false;
		}
		next = usage.get_next_use();
	}
	true
}

fn recursive_call(instruction: InstructionValue<'_>, function: FunctionValue<'_>) -> bool {
	instruction.get_opcode() == InstructionOpcode::Call
		// SAFETY: `instruction` is a call; both references belong to this module.
		&& unsafe { llvm_sys::core::LLVMGetCalledValue(instruction.as_value_ref()) }
			== function.as_value_ref()
}

fn duplicate_return_paths(module: &Module<'_>) -> bool {
	let context = module.get_context();
	let mut changed = false;
	for function in module.get_functions() {
		if !function.get_name().to_bytes().starts_with(b"fa.")
			|| !function
				.get_type()
				.get_return_type()
				.is_some_and(|ty| ty.is_int_type())
		{
			continue;
		}
		for block in function.get_basic_blocks() {
			let instructions: Vec<_> = block.get_instructions().collect();
			if instructions.len() > 8
				|| !instructions
					.last()
					.is_some_and(|last| last.get_opcode() == InstructionOpcode::Return)
			{
				continue;
			}
			let phis: Vec<_> = instructions
				.iter()
				.take_while(|instruction| instruction.get_opcode() == InstructionOpcode::Phi)
				.map(|&instruction| PhiValue::try_from(instruction).expect("a phi"))
				.collect();
			let Some(first) = phis.first() else {
				continue;
			};
			let suffix = &instructions[phis.len()..];
			if !suffix
				.iter()
				.all(|instruction| scalar_suffix(instruction.get_opcode()))
				|| !instructions
					.iter()
					.all(|&instruction| used_only_here(instruction, block))
			{
				continue;
			}
			let incoming: Vec<_> = first.get_incomings().map(|(_, block)| block).collect();
			let unique: FxHashSet<_> = incoming.iter().copied().collect();
			if !(2..=4).contains(&incoming.len())
				|| unique.len() != incoming.len()
				|| !phis.iter().all(|phi| {
					phi.count_incoming() as usize == incoming.len()
						&& phi
							.get_incomings()
							.all(|(_, predecessor)| unique.contains(&predecessor))
				}) || !incoming.iter().all(|predecessor| {
				predecessor
					.get_terminator()
					.is_some_and(|terminator| terminator.get_opcode() == InstructionOpcode::Br)
			}) || !incoming.iter().any(|predecessor| {
				predecessor
					.get_instructions()
					.any(|instruction| recursive_call(instruction, function))
			}) {
				continue;
			}
			for predecessor in incoming {
				let mut bindings: FxHashMap<_, _> = phis
					.iter()
					.map(|phi| {
						let value = phi
							.get_incomings()
							.find_map(|(value, from)| (from == predecessor).then_some(value))
							.expect("the phi has every predecessor");
						(phi.as_basic_value(), value)
					})
					.collect();
				let branch = predecessor.get_terminator().expect("a branch");
				let direct = branch.get_num_operands() == 1;
				let builder = context.create_builder();
				if direct {
					builder.position_before(&branch);
				} else {
					// Split only the return edge of a conditional predecessor.
					// Its other edge must keep its original computation.
					let path = context.append_basic_block(function, "return_path");
					builder.position_at_end(path);
					// SAFETY: `branch` is a branch of this function; `path`
					// belongs to it and replaces only the edge to `block`.
					unsafe {
						for index in 0..llvm_sys::core::LLVMGetNumSuccessors(branch.as_value_ref())
						{
							if llvm_sys::core::LLVMGetSuccessor(branch.as_value_ref(), index)
								== block.as_mut_ptr()
							{
								llvm_sys::core::LLVMSetSuccessor(
									branch.as_value_ref(),
									index,
									path.as_mut_ptr(),
								);
							}
						}
					}
				}
				for &instruction in suffix {
					let cloned = instruction.explicit_clone();
					for index in 0..instruction.get_num_operands() {
						if let Some(operand) = instruction
							.get_operand(index)
							.and_then(|operand| operand.value())
							&& let Some(&value) = bindings.get(&operand)
						{
							cloned.set_operand(index, value);
						}
					}
					builder.insert_instruction(&cloned, None);
					if instruction.get_opcode() != InstructionOpcode::Return {
						// SAFETY: the allowed non-return instructions produce a
						// basic value. Each clone has the same type as its source.
						unsafe {
							bindings.insert(
								BasicValueEnum::new(instruction.as_value_ref()),
								BasicValueEnum::new(cloned.as_value_ref()),
							);
						}
					}
				}
				if direct {
					branch.erase_from_basic_block();
				}
			}
			// SAFETY: all predecessors now return through their own copy;
			// no instruction of the removed block had a user outside it.
			unsafe { block.delete().expect("a block of this function") };
			changed = true;
		}
	}
	changed
}
