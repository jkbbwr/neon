//! The Neon compiler: source -> C11.

pub mod ast;
pub mod lexer;
pub mod parser;

/// Placeholder for the pipeline entry point.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
