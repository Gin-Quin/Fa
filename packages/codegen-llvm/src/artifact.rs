//! What [`compile`](crate::compile) produces.

use std::path::PathBuf;
use std::time::Duration;

use crate::options::{Options, Target};

/// The additional outputs of `build.cLibrary`.
#[derive(Clone, Debug)]
pub struct CLibraryArtifacts {
	/// A native static library; wasm exports a module instead.
	pub archive: Option<PathBuf>,
	pub header: PathBuf,
}

/// The time of each phase of [`compile`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Timings {
	/// Lowering the analyzed tree to IR, with the runtime.
	pub build: Duration,
	/// `LLVMVerifyModule`.
	pub verify: Duration,
	/// The new pass manager pipeline.
	pub optimize: Duration,
	/// Instruction selection and object emission.
	pub emit: Duration,
	/// `cc`, `ld.lld` or `wasm-ld`.
	pub link: Duration,
}

impl Timings {
	pub fn total(&self) -> Duration {
		self.build + self.verify + self.optimize + self.emit + self.link
	}
}

/// A compiled and linked program.
#[derive(Clone, Debug)]
pub struct Artifact {
	pub target: Target,
	pub options: Options,
	/// The executable, the `.wasm` module, or the shared library of a C library.
	pub output: PathBuf,
	/// The JavaScript loader of a wasm module (`<name>.mjs`), which runs it
	/// with Node or Bun: `node <name>.mjs arguments…`.
	pub loader: Option<PathBuf>,
	/// The optimized IR, with [`Options::emit_ir`].
	pub ir: Option<PathBuf>,
	/// A C library (`output` is its shared library or wasm module).
	pub c_library: Option<CLibraryArtifacts>,
	pub output_size: u64,
	pub timings: Timings,
}
