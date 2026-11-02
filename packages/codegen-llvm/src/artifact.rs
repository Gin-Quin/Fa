//! What [`compile`](crate::compile) produces.

use std::path::PathBuf;
use std::time::Duration;

use crate::options::{Options, Target};

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
	/// The executable, or the `.wasm` module.
	pub output: PathBuf,
	/// The JavaScript loader of a wasm module (`<name>.mjs`), which runs it
	/// with Node, Bun or Deno: `node <name>.mjs arguments…`.
	pub loader: Option<PathBuf>,
	/// The optimized IR, with [`Options::emit_ir`].
	pub ir: Option<PathBuf>,
	pub output_size: u64,
	pub timings: Timings,
}
