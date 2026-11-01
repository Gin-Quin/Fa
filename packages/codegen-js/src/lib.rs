//! `fa-codegen-js`: Fa's JavaScript backend.
//!
//! For now, only the runtime: the helpers that compiled programs call
//! (integer checks, floor division, grapheme-based strings, printing, deep
//! equality, errors and stops, collections). They are plain ES2022 modules
//! under `runtime/`, embedded here, and extracted one by one so that a
//! program only contains what it uses: see [`runtime`].

pub mod runtime;

pub use runtime::{collect_helpers, collect_helpers_inline, helper_dependencies, runtime_helper};
