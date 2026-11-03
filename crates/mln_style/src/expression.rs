//! `mln::style::expression`: re-exports `//crates/mln_expression` (owned by
//! `expression`), so that style code can write `crate::expression::X` like
//! the C++ `expression::X`.

pub use mln_expression::expression::{GlobalStateMap, collect_global_state_refs, global_state_refs_intersect};
pub use mln_expression::*;
