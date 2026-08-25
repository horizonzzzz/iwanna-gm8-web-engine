//! Shared GML source lowering.
//!
//! This is the single implementation of the `GML text -> LoweredLogicStatement`
//! translation. `iwm-parser` uses it at package build time; `iwm-runtime-core`
//! uses it at run time for `execute_file()` / `execute_string()`, where the GML
//! source only exists once the game has written it.
//!
//! The crate deliberately depends on nothing but `iwm-runtime-model` so it stays
//! usable from the `wasm32-unknown-unknown` runtime build.

mod expression;
mod source;
mod statement;
mod syntax;

pub use expression::lower_expr;
pub use source::{looks_like_gml_source, lower_source};
pub use statement::lower_statement;
