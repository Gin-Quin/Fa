use std::sync::Once;

use inkwell::OptimizationLevel;
use inkwell::targets::{
	CodeModel, InitializationConfig, RelocMode, Target as LlvmTarget, TargetMachine, TargetTriple,
};

use crate::error::Error;

pub use crate::options::{OptLevel, Options, Target};

impl Target {
	pub fn triple(self) -> TargetTriple {
		match self {
			// `FA_NATIVE_TRIPLE`: another architecture's triple, for the tests of
			// its C ABI (`tests/abi.rs`: the IR only, nothing links)
			Target::Native => match std::env::var("FA_NATIVE_TRIPLE") {
				Ok(triple) if !triple.is_empty() => TargetTriple::create(&triple),
				_ => TargetMachine::get_default_triple(),
			},
			Target::NativeFreestanding => TargetTriple::create("x86_64-unknown-linux-gnu"),
			Target::Wasm => TargetTriple::create("wasm32-unknown-unknown"),
		}
	}

	/// Creates the target machine. No option targets the host CPU: the
	/// native baseline is the architecture's generic CPU, and wasm enables
	/// only the features every current browser and runtime supports.
	pub fn machine(self, options: &Options) -> Result<TargetMachine, Error> {
		initialize_targets();
		if self == Target::NativeFreestanding
			&& !(cfg!(target_arch = "x86_64") && cfg!(target_os = "linux"))
		{
			return Err(Error::Llvm(
				"the freestanding target is only implemented for Linux x86-64".to_string(),
			));
		}
		let triple = self.triple();
		let (cpu, features, reloc_mode) = match self {
			Target::Native
				if triple
					.as_str()
					.to_str()
					.is_ok_and(|triple| triple.starts_with("x86_64")) =>
			{
				("x86-64", "", RelocMode::PIC)
			}
			Target::Native => ("generic", "", RelocMode::PIC),
			Target::NativeFreestanding => ("x86-64", "", RelocMode::Static),
			Target::Wasm => (
				"generic",
				"+bulk-memory,+mutable-globals,+sign-ext,+nontrapping-fptoint",
				RelocMode::Static,
			),
		};
		let target = LlvmTarget::from_triple(&triple)?;
		target
			.create_target_machine(
				&triple,
				cpu,
				features,
				codegen_level(options),
				reloc_mode,
				CodeModel::Default,
			)
			.ok_or_else(|| Error::Llvm(format!("no target machine for {triple}")))
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
