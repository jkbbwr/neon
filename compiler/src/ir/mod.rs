//! The intermediate representation and its passes. See `docs/design/ir.md`.
//!
//! The pipeline: monomorphise → lower to SSA → optimise → insert refcounts → emit.
//! Everything here consumes what the checker already worked out (`TypecheckResult`)
//! and re-derives nothing.

pub mod effects;
pub mod lower;
pub mod opt;
pub mod refcount;
pub mod repr;
pub mod ssa;

use crate::ast::Module;
use crate::typecheck::env::Env;
use crate::typecheck::result::TypecheckResult;
use ssa::Program;

/// Which stage of the pipeline to stop at, for `neon ir`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Straight out of lowering and monomorphisation, before any pass.
    Lowered,
    /// After the optimiser.
    Optimised,
    /// After refcount insertion -- the IR that would be emitted.
    Final,
}

/// Run the IR pipeline to the requested stage: lower (with monomorphisation), then
/// optimise, then insert reference counts.
pub fn compile(env: &Env, result: &TypecheckResult, module: &Module, stage: Stage) -> Program {
    let mut program = lower::lower_module(env, result, module);
    if stage == Stage::Lowered {
        return program;
    }
    opt::optimize(&mut program);
    if stage == Stage::Optimised {
        return program;
    }
    refcount::insert(&mut program);
    program
}
