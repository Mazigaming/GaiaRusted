//! Semantic analysis: from syntax to meaning.
//!
//! ```text
//! AST ──derive──▶ AST ──defs──▶ declarations ──check──▶ typed tree
//! ```
//!
//! * [`derive`] expands `#[derive]` attributes into ordinary impls.
//! * [`defs`] records every declaration and resolves names and imports.
//! * [`context`] answers questions about declarations: signatures, field
//!   types, which impl applies to a type.
//! * [`check`] infers and checks the types of a function body, producing
//!   the typed tree of [`thir`].
//!
//! Generic functions are checked once per set of concrete type arguments,
//! on demand, starting from `main`. A generic function nobody calls is never
//! checked; one called with three different types is compiled three times.

pub mod check;
pub mod const_eval;
pub mod context;
pub mod defs;
pub mod derive;
pub mod impl_trait;
pub mod infer;
pub mod thir;
pub mod ty;

pub use context::Context;
