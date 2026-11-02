//! Desugarings shared by the backends: they read the syntax and the analysis
//! and give a construct in a form closer to the generated code, so that the
//! targets can't diverge (design/decisions.md, "Backends (after the backend
//! review)"). A backend adopts them as it is touched: the JavaScript backend
//! uses all of them; the LLVM backend, `when` for now.
//!
//! - [`when`]: the decision list of a `when`, and the names an arm binds.
//! - [`index`]: how `items[index]` is checked and normalized.
//! - [`copies`]: when a stored value is copied (D15, moves).
//! - [`calls`]: the order in which the arguments of a call are evaluated.

pub mod calls;
pub mod copies;
pub mod index;
pub mod when;
