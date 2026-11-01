//! The language server of Fa.
//!
//! - `server`: the LSP handlers (tower-lsp), which forward everything to the analysis thread.
//! - `worker`: the analysis thread, which owns the documents and the backend, debounces the
//!   edits and publishes the diagnostics.
//! - `backend`: the `AnalysisBackend` trait, implemented by `ParseBackend` (syntax only) and,
//!   later, by the analyzer's `Session`.
//! - `document` and `line_index`: the text of the documents, and the conversions between byte
//!   offsets and LSP positions (UTF-8, UTF-16 or UTF-32 columns).
//! - `features`: symbols, folding, semantic tokens, selection ranges, completion, diagnostics.

pub mod backend;
pub mod document;
pub mod features;
pub mod line_index;
pub mod server;
pub mod worker;

pub use backend::{AnalysisBackend, ParseBackend};
pub use server::{FaLanguageServer, service};
