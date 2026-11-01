//! Desugarings shared by the backends: they read the syntax and the analysis
//! and give a construct in a form closer to the generated code.
//!
//! - [`when`]: the decision list of a `when`.

pub mod when;
