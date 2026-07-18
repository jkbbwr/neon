use crate::ir::lower::lower_module;
use crate::ir::opt::optimize;
use crate::ir::ssa::print;
use crate::typecheck::{check::check_module, Env};
use crate::{lexer, parser};

fn optimized(src: &str) -> String {
    let tokens = lexer::lex(src).expect("lexes");
    let (module, e) = parser::parse(&tokens, src.len());
    assert!(e.is_empty());
    let module = module.expect("parses");
    let mut env = Env::build(&module);
    assert!(env.errors().is_empty(), "{:?}", env.errors());
    let (result, errs) = check_module(&mut env, &module);
    assert!(errs.is_empty(), "{errs:?}");
    let mut program = lower_module(&env, &result, &module);
    optimize(&mut program);
    print::program(&program)
}

#[test]
fn constant_arithmetic_folds_to_a_single_constant() {
    let ir = optimized("fn f() -> i64 { 2 + 3 * 4 }");
    // 2 + 3*4 folds to 14, and the dead intermediate constants are removed.
    assert!(ir.contains("const.i64 14"), "{ir}");
    assert!(!ir.contains("prim.add"), "the add is folded away: {ir}");
    assert!(!ir.contains("prim.mul"), "the mul is folded away: {ir}");
}

#[test]
fn a_dead_pure_computation_is_removed() {
    let ir = optimized("fn f(x: i64) -> i64 { let unused = x * x; x }");
    // `unused` is pure and never read, so its multiply is dropped.
    assert!(!ir.contains("prim.mul"), "dead multiply removed: {ir}");
}

#[test]
fn an_effectful_call_is_kept_even_if_its_result_is_unused() {
    let ir = optimized(
        "@native(\"neon_io_println\") fn println(s: str) -> i64
         fn f() { let ignored = println(\"hi\"); }",
    );
    // println does I/O; its result is unused but the call must remain.
    assert!(ir.contains("neon_io_println"), "{ir}");
}

#[test]
fn overflowing_constant_arithmetic_is_left_for_the_runtime() {
    let ir = optimized("fn f() -> i64 { 9223372036854775807 + 1 }");
    // Folding would change behaviour if the runtime traps, so it is not folded.
    assert!(ir.contains("prim.add"), "overflow left unfolded: {ir}");
}
