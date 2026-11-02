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

#[cfg(fa_llvm)]
mod compile;
#[cfg(fa_llvm)]
mod error;
#[cfg(fa_llvm)]
mod link;
#[cfg(fa_llvm)]
mod lower;
#[cfg(fa_llvm)]
pub mod runtime;
#[cfg(fa_llvm)]
mod target;

#[cfg(fa_llvm)]
pub use compile::{Artifact, Timings, compile, compile_file, run};
#[cfg(fa_llvm)]
pub use error::Error;
#[cfg(fa_llvm)]
pub use link::javascript_runtime;
#[cfg(fa_llvm)]
pub use target::{OptLevel, Options, Target};
