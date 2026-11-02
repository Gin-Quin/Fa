//! Fa's LLVM backend: native executables and WebAssembly.
//!
//! [`compile`] lowers an analyzed file (`fa_analyzer`) to an LLVM module
//! (`lower`), links the runtime into it (`runtime`: a platform layer built as
//! IR, and the tier 1 runtime compiled from C to IR), optimizes it with the
//! new pass manager, emits an object for the host or for
//! `wasm32-unknown-unknown`, and links it (`cc`, `ld.lld` or `wasm-ld`). A
//! wasm module gets a JavaScript loader next to it.
//!
//! The LLVM code only exists when `LLVM_SYS_181_PREFIX` was set at build time
//! (see `scripts/setup-llvm.sh` and `build.rs`): check [`llvm_available`].
//! See `design/llvm-backend-plan.md`.

/// Whether this build of the crate links LLVM (`LLVM_SYS_181_PREFIX` was set).
pub const fn llvm_available() -> bool {
	cfg!(fa_llvm)
}

/// The message printed by the tests and examples when LLVM is not available.
pub const LLVM_MISSING: &str = "skipped: fa-codegen-llvm was built without LLVM; run scripts/setup-llvm.sh, export LLVM_SYS_181_PREFIX and rebuild";

mod artifact;
#[cfg(fa_llvm)]
mod compile;
mod error;
mod host;
#[cfg(fa_llvm)]
mod link;
#[cfg(fa_llvm)]
mod lower;
mod options;
#[cfg(fa_llvm)]
pub mod runtime;
#[cfg(fa_llvm)]
mod target;

pub use artifact::{Artifact, Timings};
#[cfg(fa_llvm)]
pub use compile::{
	compile, compile_file, compile_file_linking, compile_linking, compile_program,
	compile_program_linking, lowered_ir, run,
};
pub use error::Error;
pub use host::javascript_runtime;
pub use options::{Libraries, OptLevel, Options, Target};

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile(
	_session: &fa_analyzer::Session,
	_file: fa_analyzer::ids::FileId,
	_path: &str,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_program(
	_session: &fa_analyzer::Session,
	_program: &fa_analyzer::modules::Program,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_program_linking(
	_session: &fa_analyzer::Session,
	_program: &fa_analyzer::modules::Program,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_linking(
	_session: &fa_analyzer::Session,
	_file: fa_analyzer::ids::FileId,
	_path: &str,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}
