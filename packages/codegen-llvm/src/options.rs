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

	/// The target as the language names it (the value of `target`):
	/// `native` or `wasm`.
	pub fn language_target(self) -> &'static str {
		match self {
			Target::Wasm => "wasm",
			Target::Native | Target::NativeFreestanding => "native",
		}
	}

	/// The target the analysis is for (`Session::set_target`): the
	/// language's, or `native-freestanding`, a variant of `native` without
	/// a C library (`AN004` on the files and the sockets).
	pub fn analysis_target(self) -> &'static str {
		match self {
			Target::NativeFreestanding => "native-freestanding",
			other => other.language_target(),
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
	/// The asynchronous mode (`design/async.md` A5–A7): `None` detects it
	/// from the entry (a reachable `start`, or an `await` that may
	/// suspend); `compile_program` takes `Program::asynchronous`.
	pub asynchronous: Option<bool>,
}

/// The C libraries a program links (foreign code: `build.link` and
/// `build.libraryPaths` of `package.fa`): `-l<name>` (or the path of a
/// library or an object file), searched in `paths` too.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Libraries {
	/// The libraries: names (`sqlite3`), or paths of `.a`, `.so` or `.o` files.
	pub link: Vec<String>,
	/// Extra folders where the libraries are searched (and found at run time).
	pub paths: Vec<std::path::PathBuf>,
	/// Build the program as a C library instead of an executable
	/// (`build.cLibrary`).
	pub c_library: Option<CLibrary>,
}

/// A program built as a C library (`build.cLibrary`, foreign-code.md,
/// "Exposing a library to C"): `lib<prefix>.a`, `lib<prefix>.so` and the
/// header, written next to the output; the module values are initialized
/// when the library is loaded, and the functions of the namespace the API
/// file exports are C functions named `<prefix>_<name in snake case>`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CLibrary {
	/// The header's file name (`farm.h`).
	pub header: String,
	/// The prefix of the C names (`farm`).
	pub prefix: String,
	/// The API file, as written in the configuration (`library/capi.foreign.fa`).
	pub api: String,
}

impl CLibrary {
	/// The module of the API file (`library/capi.foreign`).
	pub fn api_module(&self) -> String {
		let path = self.api.replace('\\', "/");
		let path = path.trim_start_matches("./");
		path.strip_suffix(".fa").unwrap_or(path).to_string()
	}

	/// The static library's file name (`libfarm.a`).
	pub fn static_name(&self) -> String {
		format!("lib{}.a", self.prefix)
	}

	/// The shared library's file name (`libfarm.so`, `libfarm.dylib` on macOS).
	pub fn shared_name(&self) -> String {
		if cfg!(target_os = "macos") {
			format!("lib{}.dylib", self.prefix)
		} else {
			format!("lib{}.so", self.prefix)
		}
	}
}

impl Default for Options {
	fn default() -> Self {
		Options {
			opt_level: OptLevel::default(),
			size: false,
			emit_ir: false,
			overflow_checks: true,
			leak_check: false,
			asynchronous: None,
		}
	}
}

/// Passes after the standard pipeline, for loops with bounds checks: the
/// inductive range check elimination of LLVM (`irce`, not in the default
/// pipeline) splits a loop whose checks are `0 <= i + k < length` (the
/// common path of an index check, `arrays.rs`) into a short loop before,
/// a check-free main loop, and a short loop after; the main loop's
/// invariant loads (the header of a destination array) then leave it, a
/// second round takes out the checks they bounded, and the main loop is
/// vectorized (`blur`). `reassociate` puts the sums back into chains, so
/// that `instcombine` narrows byte arithmetic to 16 bits. Nontrivial loop
/// unswitching separates invariant representation choices (inline strings
/// or owned buffers) once before their character loops.
macro_rules! loop_checks {
	() => {
		"function(irce,simplifycfg,instcombine<no-verify-fixpoint>,loop-mssa(licm,simple-loop-unswitch<nontrivial>),irce,simplifycfg,reassociate,instcombine<no-verify-fixpoint>,loop-vectorize,instcombine<no-verify-fixpoint>,simplifycfg)"
	};
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
			OptLevel::O2 => concat!("default<O2>,", loop_checks!()),
			OptLevel::O3 => concat!("default<O3>,", loop_checks!()),
		}
	}

	/// The pipeline of a module with coroutines: at `O0` too, the coroutine
	/// passes split them (code generation aborts on a coroutine left whole).
	pub fn coroutine_pipeline(&self) -> &'static str {
		match self.pipeline() {
			"globaldce" => "coro-early,cgscc(coro-split),coro-cleanup,globaldce",
			pipeline => pipeline,
		}
	}

	/// The dev profile (`O0`): the event loop warns about long turns.
	pub fn dev(&self) -> bool {
		!self.size && self.opt_level == OptLevel::O0
	}

	/// Whether the linker removes unused sections.
	pub fn optimized(&self) -> bool {
		self.size || self.opt_level != OptLevel::O0
	}
}
