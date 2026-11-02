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
	/// the module imports what it needs from the `fa` module of the
	/// JavaScript loader written next to it.
	Wasm,
	/// Linux x86-64 without libc (an option of `native`, for the smallest
	/// programs): the runtime makes the system calls itself, and `ld.lld`
	/// links a static executable. Printing floats needs [`Target::Native`].
	NativeFreestanding,
}

impl Target {
	/// The name of the target on the command line.
	pub fn name(self) -> &'static str {
		match self {
			Target::Native => "native",
			Target::Wasm => "wasm",
			Target::NativeFreestanding => "native-freestanding",
		}
	}

	/// Parses a target name of the command line.
	pub fn from_name(name: &str) -> Option<Target> {
		match name {
			"native" => Some(Target::Native),
			"wasm" => Some(Target::Wasm),
			"native-freestanding" | "freestanding" => Some(Target::NativeFreestanding),
			_ => None,
		}
	}

	pub fn triple(self) -> TargetTriple {
		match self {
			Target::Native => TargetMachine::get_default_triple(),
			Target::NativeFreestanding => TargetTriple::create("x86_64-unknown-linux-gnu"),
			Target::Wasm => TargetTriple::create("wasm32-unknown-unknown"),
		}
	}

	/// Whether the target links the C library (`snprintf`, `strtod`, `pow`…).
	pub fn has_libc(self) -> bool {
		self == Target::Native
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
			Target::Native if cfg!(target_arch = "x86_64") => ("x86-64", "", RelocMode::PIC),
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
				options.codegen_level(),
				reloc_mode,
				CodeModel::Default,
			)
			.ok_or_else(|| Error::Llvm(format!("no target machine for {triple}")))
	}
}

/// The optimization level of the mid-end and of the code generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum OptLevel {
	/// No mid-end pass but dead-code removal, fast instruction selection.
	O0,
	O1,
	O2,
	/// The default.
	#[default]
	O3,
}

/// How to compile a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Options {
	/// The optimization level (`O3` by default).
	pub opt_level: OptLevel,
	/// Optimize for size: the `default<Oz>` pipeline, and the linker flags of
	/// the smallest output (stripped, no page-aligned segments).
	pub size: bool,
	/// Writes the optimized LLVM IR next to the output (`<output>.ll`).
	pub emit_ir: bool,
}

impl Options {
	/// The new pass manager pipeline. At `O0`, only the unused runtime
	/// functions are removed.
	pub fn pipeline(&self) -> &'static str {
		if self.size {
			return "default<Oz>";
		}
		match self.opt_level {
			OptLevel::O0 => "globaldce",
			OptLevel::O1 => "default<O1>",
			OptLevel::O2 => "default<O2>",
			OptLevel::O3 => "default<O3>",
		}
	}

	/// Whether the linker removes unused sections.
	pub fn optimized(&self) -> bool {
		self.size || self.opt_level != OptLevel::O0
	}

	fn codegen_level(&self) -> OptimizationLevel {
		if self.size {
			return OptimizationLevel::Default;
		}
		match self.opt_level {
			OptLevel::O0 => OptimizationLevel::None,
			OptLevel::O1 => OptimizationLevel::Less,
			OptLevel::O2 => OptimizationLevel::Default,
			OptLevel::O3 => OptimizationLevel::Aggressive,
		}
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
