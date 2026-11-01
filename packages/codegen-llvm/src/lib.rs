//! Proof of concept of Fa's LLVM backend.
//!
//! There is no lowering from a Fa tree yet: [`Program`] builds a few tiny
//! modules by hand (a recursive Fibonacci with checked overflow, a hello
//! world, a string concatenation), with the minimal runtime that the compiler
//! emits as LLVM IR into the same module ([`runtime`]). [`compile`] optimizes
//! the module with the new pass manager, emits an object for the host or for
//! `wasm32-unknown-unknown`, and links it (`cc` or `wasm-ld`).
//!
//! The LLVM code only exists when `LLVM_SYS_181_PREFIX` was set at build time
//! (see `scripts/setup-llvm.sh` and `build.rs`): check [`llvm_available`].
//! See `design/llvm-backend-plan.md` for the plan this prototype de-risks.

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
mod program;
#[cfg(fa_llvm)]
pub mod runtime;
#[cfg(fa_llvm)]
mod target;

#[cfg(fa_llvm)]
pub use compile::{Artifact, Timings, compile, run};
#[cfg(fa_llvm)]
pub use error::Error;
#[cfg(fa_llvm)]
pub use program::Program;
#[cfg(fa_llvm)]
pub use target::{Profile, Target};
