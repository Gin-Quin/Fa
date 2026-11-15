use std::sync::Once;

use inkwell::OptimizationLevel;
use inkwell::targets::{
	CodeModel, InitializationConfig, RelocMode, Target as LlvmTarget, TargetMachine, TargetTriple,
};

use crate::error::Error;

unsafe extern "C" {
	/// `shim/thin_bitcode.cpp`: constructors in `.init_array` (ELF).
	fn fa_use_init_array(machine: llvm_sys::target_machine::LLVMTargetMachineRef);
}

/// Makes a machine compile thread-local variables as emulated ones (calls
/// of `__emutls_get_address`), for an object the JIT loads (`jit.rs`).
pub(crate) fn use_emulated_tls(machine: &TargetMachine) {
	unsafe extern "C" {
		/// `shim/thin_bitcode.cpp`.
		fn fa_use_emulated_tls(machine: llvm_sys::target_machine::LLVMTargetMachineRef);
	}
	// SAFETY: the machine is alive
	unsafe { fa_use_emulated_tls(machine.as_mut_ptr()) };
}

pub use crate::options::{OptLevel, Options, Target};

impl Target {
	pub fn triple(self) -> TargetTriple {
		match self {
			// Tests can cross-compile with a matching CC and FA_NATIVE_TRIPLE.
			Target::Native | Target::NativeFreestanding => {
				match std::env::var("FA_NATIVE_TRIPLE") {
					Ok(triple) if !triple.is_empty() => TargetTriple::create(&triple),
					_ => TargetMachine::get_default_triple(),
				}
			}
			Target::Wasm => TargetTriple::create("wasm32-unknown-unknown"),
		}
	}

	/// Creates the target machine. No option targets the host CPU: the
	/// native baseline is the architecture's generic CPU, and wasm enables
	/// only the features every current browser and runtime supports.
	pub fn machine(self, options: &Options) -> Result<TargetMachine, Error> {
		initialize_targets();
		let triple = self.triple();
		let name = triple.as_str().to_string_lossy();
		let x86_64 = name.starts_with("x86_64");
		if self == Target::NativeFreestanding
			&& !(name.contains("linux")
				&& (x86_64 || name.starts_with("aarch64") || name.starts_with("arm64")))
		{
			return Err(Error::Llvm(
				"the freestanding target requires Linux x86-64 or AArch64".to_string(),
			));
		}
		let (cpu, features, reloc_mode) = match self {
			Target::Native if x86_64 => ("x86-64", "", RelocMode::PIC),
			Target::Native => ("generic", "", RelocMode::PIC),
			Target::NativeFreestanding => (
				if x86_64 { "x86-64" } else { "generic" },
				"",
				RelocMode::Static,
			),
			Target::Wasm => (
				"generic",
				"+bulk-memory,+mutable-globals,+sign-ext,+nontrapping-fptoint",
				RelocMode::Static,
			),
		};
		let target = LlvmTarget::from_triple(&triple)?;
		let machine = target
			.create_target_machine(
				&triple,
				cpu,
				features,
				codegen_level(options),
				reloc_mode,
				CodeModel::Default,
			)
			.ok_or_else(|| Error::Llvm(format!("no target machine for {triple}")))?;
		// SAFETY: the machine is alive
		unsafe { fa_use_init_array(machine.as_mut_ptr()) };
		Ok(machine)
	}
}

/// The code generation level of the options.
fn codegen_level(options: &Options) -> OptimizationLevel {
	if options.size {
		return OptimizationLevel::Default;
	}
	match options.opt_level {
		OptLevel::O0 => OptimizationLevel::None,
		OptLevel::O1 => OptimizationLevel::Less,
		OptLevel::O2 => OptimizationLevel::Default,
		OptLevel::O3 => OptimizationLevel::Aggressive,
	}
}
fn initialize_targets() {
	static INITIALIZE: Once = Once::new();
	INITIALIZE.call_once(|| {
		let config = InitializationConfig::default();
		LlvmTarget::initialize_x86(&config);
		LlvmTarget::initialize_aarch64(&config);
		LlvmTarget::initialize_webassembly(&config);
	});
}
