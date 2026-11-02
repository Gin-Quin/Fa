//! The targets and options of the backend: plain data, available without LLVM
//! (the `fa` command parses them before knowing whether LLVM is linked).

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

	/// Whether the target links the C library (`snprintf`, `strtod`, `pow`…).
	pub fn has_libc(self) -> bool {
		self == Target::Native
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Options {
	/// The optimization level (`O3` by default).
	pub opt_level: OptLevel,
	/// Optimize for size: the `default<Oz>` pipeline, and the linker flags of
	/// the smallest output (stripped, no page-aligned segments).
	pub size: bool,
	/// Writes the optimized LLVM IR next to the output (`<output>.ll`).
	pub emit_ir: bool,
	/// The integer overflow checks (`OVERFLOW_NATIVE`, `SINK_CHECK`); when
	/// off, integers wrap in two's complement (plain `add`, never `nsw`) and
	/// a narrowing sink truncates. Divisor, index, shift and conversion
	/// checks stay (design/decisions.md, "Overflow checks").
	pub overflow_checks: bool,
	/// Counts the live heap blocks, and makes a program that ends normally
	/// with blocks it didn't free print `leak: <count> blocks` and exit with
	/// 102 (the tests).
	pub leak_check: bool,
}

impl Default for Options {
	fn default() -> Self {
		Options {
			opt_level: OptLevel::default(),
			size: false,
			emit_ir: false,
			overflow_checks: true,
			leak_check: false,
		}
	}
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
}
