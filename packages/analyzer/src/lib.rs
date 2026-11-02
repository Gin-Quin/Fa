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
//! - [`session`]: the session and [`analyze_source`].
//!
//! M1 adds the walk:
//!
//! - [`syntax`]: the view layer over the parser's nodes (the only module naming them);
//! - [`symbols`]: symbols, scopes and resolutions;
//! - [`builtins`]: the prelude as Rust tables;
//! - `check`: the walk itself, one handler per [`syntax::View`] variant;
//! - [`ide`]: the queries of editors (hover, definition, completions);
//! - [`lower`]: desugarings shared by the backends (the decision list of a `when`).

pub mod analysis;
pub mod builtins;
pub mod check;
pub mod comptime;
pub mod configuration;
pub mod diagnostics;
pub mod effects;
pub mod flow;
pub mod ide;
pub mod ids;
pub mod intern;
pub mod lower;
pub mod modules;
pub mod mounts;
pub mod parallel;
mod program;
pub mod session;
pub mod symbols;
pub mod syntax;
pub mod types;

pub use diagnostics::{Diagnostic, Severity};
pub use modules::{CodebaseResolver, FileRole, ModuleEntry, ModuleResolver, Program};
pub use session::{Session, analyze_source};
