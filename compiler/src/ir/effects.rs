//! Effect analysis, for the optimiser only. Pessimistic: a function is **effectful**
//! unless it can be cheaply proven pure. Being wrong in the safe direction only costs a
//! missed optimisation; the reverse would miscompile. See `docs/design/ir.md`.
//!
//! Because the language is immutable there is no read/write-of-mutable-memory category —
//! the whole lattice is two states, `pure` vs `effectful`. A function is pure iff every
//! instruction is pure and every callee is pure; the callee condition is a monotonic
//! fixpoint over the call graph.

use super::repr::Repr;
use super::ssa::{Func, Op, PrimOp, Program};
use std::collections::{HashMap, HashSet};

/// Whether a native symbol has an observable effect. A native's body is opaque to the
/// compiler, so this is not an analysis: it reports what the declaration *claimed*.
/// `pure_natives` is the set of symbols whose `@native` declaration also carried `@pure`,
/// and everything outside it is effectful.
///
/// The polarity is the load-bearing part. Silence means effectful, so forgetting `@pure`
/// costs an optimisation and nothing else, while a wrong `@pure` licenses DCE to delete a
/// call that mattered. The rule this replaced inferred purity from the symbol's spelling
/// and deleted a resource construction along with the cleanup that construction existed to
/// schedule; the test at the bottom of this file pins the direction.
pub fn native_is_effectful(symbol: &str, pure_natives: &HashSet<String>) -> bool {
    !pure_natives.contains(symbol)
}

/// Purity for every function in the program, keyed by name. A name that is *absent* is not
/// "unknown" but effectful: `op_is_effectful` reads a missing entry as false-purity, which
/// is how a call to something outside the lowered program — a native, an instance that has
/// not been monomorphised yet — stays un-eliminable.
///
/// The fixpoint starts optimistic and only ever removes purity, which is what lets
/// recursion terminate *and* be classified usefully: a self- or mutually-recursive
/// function is provisionally pure while its own body is examined, so a pure recursive
/// function stays pure instead of demoting itself on the first look at its own call.
/// Starting pessimistic would be sound but would mark every cycle effectful.
///
/// Keying by name means the mangled names monomorphisation produces must be distinct. Two
/// functions sharing one would share a verdict — the merge lands on effectful if either is,
/// so the result stays safe, but a pure instance would be needlessly pinned.
pub fn analyze(program: &Program) -> HashMap<String, bool> {
    // Start optimistic (every function pure), then knock out any that do something
    // effectful or reach one that does, to a fixpoint. Monotone, so it converges.
    let mut pure: HashMap<String, bool> = program.funcs.iter().map(|f| (f.name.clone(), true)).collect();

    loop {
        let mut changed = false;
        for f in &program.funcs {
            if !pure.get(&f.name).copied().unwrap_or(false) {
                continue; // already effectful
            }
            let effectful = f.blocks.iter().any(|b| {
                b.insts.iter().any(|inst| op_is_effectful(f, &inst.op, &pure, &program.pure_natives))
            });
            if effectful {
                pure.insert(f.name.clone(), false);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    pure
}

/// Whether an op has an effect that must be preserved -- so DCE may not drop it even if
/// its result is unused, and CSE may not share it. `pure` maps each function to whether
/// it is pure.
///
/// The catch-all `false` arm is what makes this cheap and also what makes it a place to be
/// careful: allocation, projection, arithmetic and the comparison ops are genuinely pure
/// functions of their operands, and `Retain`/`Release` land here too. Those two are never
/// at risk from DCE despite the verdict, because they have no result and DCE only
/// considers instructions whose result is unused — and refcount insertion runs after the
/// optimiser regardless. `throw` is a terminator, not an `Op`, so it has no arm here and
/// is never a deletion candidate.
pub fn op_is_effectful(
    f: &Func,
    op: &Op,
    pure: &HashMap<String, bool>,
    pure_natives: &HashSet<String>,
) -> bool {
    match op {
        // Talks to the world, or reaches something that might. A native is opaque, so
        // this is what it *declared*: no `@pure`, no elimination.
        Op::Native { symbol, .. } => native_is_effectful(symbol, pure_natives),
        // A direct call is effectful iff its callee is; an unknown callee (not in the
        // program -- e.g. a not-yet-lowered instance) is assumed effectful.
        Op::Call { func, .. } => !pure.get(func).copied().unwrap_or(false),
        // An indirect call cannot be seen through: pessimistically effectful.
        Op::CallClosure { .. } => true,
        // Indexing traps -- out of bounds for a list, absent key for a map -- and a trap
        // ends the program, which is as observable as an effect gets. Deleting one because
        // nobody reads the element is deleting the check: `xs[10]` as a statement ran
        // clean past the end of a three-element list.
        Op::Index { .. } => true,
        // i64 arithmetic traps too, on overflow and on division by zero. The operand repr
        // is what decides it: the f64 forms follow IEEE and produce an infinity or a NaN
        // rather than trapping, so they stay pure and stay eliminable. That distinction is
        // worth the lookup — calling all arithmetic effectful would make almost every
        // function effectful and leave DCE with nothing it may remove, while calling it
        // all pure deleted `1 / 0`.
        Op::Prim(
            PrimOp::Add | PrimOp::Sub | PrimOp::Mul | PrimOp::Div | PrimOp::Rem | PrimOp::Neg,
            operands,
        ) => operands.iter().any(|&v| matches!(f.value_repr(v), Repr::I64)),
        // Everything else is a pure function of its operands.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::lower::lower_module;
    use crate::typecheck::{check::check_module, Env};
    use crate::{lexer, parser};

    fn analyze_src(src: &str) -> HashMap<String, bool> {
        let tokens = lexer::lex(src).expect("lexes");
        let (module, e) = parser::parse(&tokens, src.len());
        assert!(e.is_empty());
        let module = module.expect("parses");
        let mut env = Env::build(&module);
        assert!(env.errors().is_empty(), "{:?}", env.errors());
        let (result, errs) = check_module(&mut env, &module);
        assert!(errs.is_empty(), "{errs:?}");
        analyze(&lower_module(&env, &result, &module, &[]))
    }

    #[test]
    fn io_is_effectful_and_reaches_its_callers() {
        let e = analyze_src(
            "@native(\"neon_io_println\") fn println(s: str)
             fn shout(s: str) { println(s); }
             fn calls_io(s: str) { shout(s); }",
        );
        assert_eq!(e.get("shout"), Some(&false), "calls io -> effectful");
        assert_eq!(e.get("calls_io"), Some(&false), "reaches io transitively");
    }

    /// `i64` arithmetic traps -- on overflow, and on division by zero -- so a function
    /// doing it is effectful and a call to it may not be deleted for having an unused
    /// result. `f64` follows IEEE and produces an infinity or a NaN instead of trapping,
    /// so it stays pure and stays eliminable.
    ///
    /// The distinction is worth the operand check. Calling all arithmetic effectful would
    /// make almost every function effectful and leave dead-code elimination with nothing
    /// to remove; calling it all pure deleted `1 / 0`.
    #[test]
    fn i64_arithmetic_traps_and_f64_does_not() {
        let e = analyze_src(
            "fn double(x: i64) -> i64 { x + x }
             fn scale(x: f64) -> f64 { x * 2.0 }
             fn compare(a: i64, b: i64) -> bool { a < b }",
        );
        assert_eq!(e.get("double"), Some(&false), "i64 `+` can overflow-trap");
        assert_eq!(e.get("scale"), Some(&true), "f64 arithmetic cannot trap");
        assert_eq!(e.get("compare"), Some(&true), "a comparison cannot trap");
    }

    #[test]
    fn a_pure_native_stays_pure() {
        let e = analyze_src(
            "@pure @native(\"neon_str_concat\") fn concat(a: str, b: str) -> str
             fn greet(n: str) -> str { concat(\"hi \", n) }",
        );
        assert_eq!(e.get("greet"), Some(&true), "string concat is declared pure");
    }

    /// The polarity that matters: an unannotated native is effectful, so a caller of one
    /// is effectful too and its calls survive DCE. Guessing purity from the symbol's
    /// spelling — the rule this replaced — deleted a resource construction and with it the
    /// cleanup that construction existed to schedule.
    #[test]
    fn an_unannotated_native_is_effectful() {
        let e = analyze_src(
            "@native(\"neon_str_concat\") fn concat(a: str, b: str) -> str
             fn greet(n: str) -> str { concat(\"hi \", n) }",
        );
        assert_eq!(e.get("greet"), Some(&false), "no `@pure` means effectful");
    }
}
