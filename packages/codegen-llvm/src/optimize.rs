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

unsafe extern "C" {
	fn fa_specialize_frozen_strings(module: llvm_sys::prelude::LLVMModuleRef) -> u32;
}

/// Version complete lowered bodies, retaining all original ABI and ownership
/// decisions. The marker is emitted only from the whole-program field proof.
pub(crate) fn specialize_ascii_methods(
	module: &Module<'_>,
	machine: &TargetMachine,
	options: &Options,
) -> Result<(), Error> {
	if options.size || options.opt_level != OptLevel::O3 {
		return Ok(());
	}
	// SAFETY: the module and every candidate body belong to this context.
	if unsafe { fa_specialize_frozen_strings(module.as_mut_ptr()) } != 0 {
		module.verify()?;
		module.run_passes(
			"function(sroa,instcombine<no-verify-fixpoint>,early-cse,simplifycfg,instcombine<no-verify-fixpoint>)",
			machine,
			PassBuilderOptions::create(),
		)?;
		module.verify()?;
	}
	Ok(())
}

#[cfg(test)]
mod ascii_tests {
	use inkwell::context::Context;
	use inkwell::memory_buffer::MemoryBuffer;
	use inkwell::values::AnyValue;

	#[test]
	fn ascii_variant_folds_unicode_reads_before_inlining() {
		let context = Context::create();
		let source = r#"
target datalayout = "e-p:64:64-i64:64"
declare i64 @string_code_unit_slow(ptr, i64, i64, i64)
define fastcc i64 @scan(ptr %self, i64 %index) #0 {
entry:
  %address = getelementptr i8, ptr %self, i64 16
  %capacity = load i64, ptr %address
  %ascii = icmp slt i64 %capacity, 0
  br i1 %ascii, label %byte, label %unicode
byte:
  %raw = load i8, ptr %self
  %wide = zext i8 %raw to i64
  br label %done
unicode:
  %unit = call i64 @string_code_unit_slow(ptr %self, i64 0, i64 %capacity, i64 %index)
  br label %done
done:
  %result = phi i64 [ %wide, %byte ], [ %unit, %unicode ]
  ret i64 %result
}
attributes #0 = { "fa.frozen.ascii"="0,16,Owner:0" }
"#;
		let buffer = MemoryBuffer::create_from_memory_range_copy(source.as_bytes(), "versions");
		let module = context.create_module_from_ir(buffer).expect("test module");
		let options = crate::Options::default();
		let machine = crate::Target::Native.machine(&options).expect("target");
		super::specialize_ascii_methods(&module, &machine, &options).expect("specialization");
		module.verify().expect("specialized module");
		let ascii = module
			.get_function("scan.ascii")
			.expect("ASCII body")
			.print_to_string()
			.to_string();
		assert!(!ascii.contains("@string_code_unit_slow"), "{ascii}");
		assert!(ascii.contains("load i8"), "{ascii}");
		let unicode = module
			.get_function("scan.unicode")
			.expect("Unicode body")
			.print_to_string()
			.to_string();
		assert!(unicode.contains("@string_code_unit_slow"), "{unicode}");
		assert!(!unicode.contains("load i8"), "{unicode}");
	}

	#[test]
	fn clones_preserve_function_addresses_and_redirect_only_same_receiver() {
		let context = Context::create();
		let source = r#"
target datalayout = "e-p:64:64-i64:64"
declare i64 @string_code_unit_slow(ptr, i64, i64, i64)
declare void @remember(ptr)
define fastcc i64 @scan(ptr %self, i64 %index) #0 {
entry:
  call void @remember(ptr @scan)
  %unit = call i64 @string_code_unit_slow(ptr %self, i64 0, i64 0, i64 %index)
  %next = call fastcc i64 @scan(ptr %self, i64 %unit)
  %other = call fastcc i64 @scan(ptr null, i64 %next)
  ret i64 %other
}
attributes #0 = { "fa.frozen.ascii"="0,16,Owner:0" }
"#;
		let buffer = MemoryBuffer::create_from_memory_range_copy(source.as_bytes(), "versions");
		let module = context.create_module_from_ir(buffer).expect("test module");
		// SAFETY: all candidates and their cloned bodies belong to module.
		assert_eq!(
			unsafe { super::fa_specialize_frozen_strings(module.as_mut_ptr()) },
			1
		);
		module.verify().expect("versioned ABI");
		let ir = module.print_to_string().to_string();
		assert!(ir.contains("musttail call fastcc i64 @scan.ascii("), "{ir}");
		for name in ["ascii", "unicode"] {
			let start = ir
				.find(&format!("define internal fastcc i64 @scan.{name}("))
				.expect("variant");
			let body = &ir[start..start + ir[start..].find("\n}").expect("end")];
			assert!(body.contains("call void @remember(ptr @scan)"), "{body}");
			assert!(
				body.contains(&format!("call fastcc i64 @scan.{name}(ptr %self")),
				"{body}"
			);
			assert!(body.contains("call fastcc i64 @scan(ptr null"), "{body}");
		}
	}
}

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
