//! `fa-analyzer`: the single-pass semantic analyzer of Fa.
//!
//! It resolves names, infers types, infers and checks effects, tracks flow
//! facts (narrowing, integer ranges, reference validity) and emits every
//! diagnostic, in one walk over the parser's `TypedSyntaxTree`. The design is
//! `design/analyzer-design.md`.
//!
//! Milestone M0 provides the foundations the walk is built on:
//!
//! - [`ids`]: the `u32` handles;
//! - [`analysis`]: the per-file side tables and the node flags read by the backends;
//! - [`intern`]: symbol and value interning;
//! - [`diagnostics`]: diagnostics and the code registry;
//! - [`types`]: the [`TypeStore`](types::TypeStore), integer intervals, unions, assignability, display;
//! - [`flow`]: places, hash-consed facts and the flow state;
//! - [`session`]: [`analyze_source`], which only parses for now.

pub mod analysis;
pub mod diagnostics;
pub mod flow;
pub mod ids;
pub mod intern;
pub mod session;
pub mod types;

pub use diagnostics::{Diagnostic, Severity};
pub use session::{Session, analyze_source};
