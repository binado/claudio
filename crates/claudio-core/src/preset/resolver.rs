//! Backwards-compatible facade for the legacy import path `claudio_core::preset::resolver`.
//!
//! The implementation lives in `crate::preset::resolve`.

pub use crate::env::extract_variable_references;
pub use crate::preset::resolve::*;
