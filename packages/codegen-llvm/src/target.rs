use std::sync::Once;

use inkwell::OptimizationLevel;
use inkwell::targets::{
	CodeModel, InitializationConfig, RelocMode, Target as LlvmTarget, TargetMachine, TargetTriple,
};

use crate::error::Error;

/// The targets of the backend (`fa build --target=native` and `--target=wasm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
	/// The host: x86-64 or AArch64, linked with `cc` against the system libc.
	Native,
	/// `wasm32-unknown-unknown`, linked with `wasm-ld`. No libc and no WASI:
	/// the module imports what it needs from the `fa` module of its host.
	Wasm,
}

/// The internal profiles of `design/compilation-with-llvm.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Profile {
	/// No mid-end passes, fast instruction selection.
	Dev,
	/// `default<O2>`.
	Release,
	/// `default<O3>`.
	Max,
	/// `default<Oz>`, and the linker flags for the smallest output.
	Size,
}

impl Profile {
	/// The new pass manager pipeline, or `None` when no pass runs.
	pub fn pipeline(self) -> Option<&'static str> {
		match self {
			Profile::Dev => None,
			Profile::Release => Some("default<O2>"),
			Profile::Max => Some("default<O3>"),
			Profile::Size => Some("default<Oz>"),
		}
	}

	fn codegen_level(self) -> OptimizationLevel {
		match self {
			Profile::Dev => OptimizationLevel::None,
			Profile::Release | Profile::Size => OptimizationLevel::Default,
			Profile::Max => OptimizationLevel::Aggressive,
		}
	}
}

impl Target {
	pub fn triple(self) -> TargetTriple {
		match self {
			Target::Native => TargetMachine::get_default_triple(),
			Target::Wasm => TargetTriple::create("wasm32-unknown-unknown"),
		}
	}

	/// The file extension of the linked output.
	pub fn output_extension(self) -> &'static str {
		match self {
			Target::Native => "",
			Target::Wasm => "wasm",
		}
	}

	/// Creates the target machine. No profile targets the host CPU: the
	/// native baseline is the architecture's generic CPU, and wasm enables
	/// only the features every current browser and runtime supports.
	pub fn machine(self, profile: Profile) -> Result<TargetMachine, Error> {
		initialize_targets();
		let triple = self.triple();
		let (cpu, features, reloc_mode) = match self {
			Target::Native if cfg!(target_arch = "x86_64") => ("x86-64", "", RelocMode::PIC),
			Target::Native => ("generic", "", RelocMode::PIC),
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
				profile.codegen_level(),
				reloc_mode,
				CodeModel::Default,
			)
			.ok_or_else(|| Error::Llvm(format!("no target machine for {triple}")))
	}
}

fn initialize_targets() {
	static INITIALIZE: Once = Once::new();
	INITIALIZE.call_once(|| {
		let config = InitializationConfig::default();
		#[cfg(target_arch = "x86_64")]
		LlvmTarget::initialize_x86(&config);
		#[cfg(target_arch = "aarch64")]
		LlvmTarget::initialize_aarch64(&config);
		LlvmTarget::initialize_webassembly(&config);
	});
}
