//! The shared front end: read a source file, run it through lexing, parsing, annotation
//! expansion, declaration building (with the stdlib) and type-checking. On any error it
//! renders diagnostics and exits; on success it hands back what codegen needs. Used by
//! every verb that has to type-check.

use crate::source;
use color_eyre::eyre::{eyre, Result};
use neon_compiler::diagnostic::Renderer;
use neon_compiler::typecheck::env::Unit;
use neon_compiler::typecheck::result::TypecheckResult;
use neon_compiler::typecheck::Env;
use neon_compiler::{ast, lexer, parser};
use std::path::Path;

/// A checked program, ready to lower.
pub struct Checked {
    pub env: Env,
    pub result: TypecheckResult,
    pub module: ast::Module,
    /// The stdlib, by module path. Kept because its function *bodies* have to be lowered:
    /// the stdlib is real Neon code now, not only `@native` signatures.
    pub libs: Vec<(Vec<String>, ast::Module)>,
}

/// Type-check a source file, exiting with rendered diagnostics on any error.
pub fn check(path: &Path, lib: bool) -> Result<Checked> {
    let src = source::read(path)?;
    let mut r = Renderer::for_stderr(path, &src);

    let tokens = match lexer::lex(&src) {
        Ok(t) => t,
        Err(errors) => {
            for e in &errors {
                r.eprint(e.span.clone(), &e.to_string());
            }
            std::process::exit(1);
        }
    };
    let (module, errors) = parser::parse(&tokens, src.len());
    if !errors.is_empty() {
        for e in &errors {
            r.eprint(e.span.clone(), &e.to_string());
        }
        std::process::exit(1);
    }
    let module = module.expect("no errors means a module");

    let config = neon_compiler::expand::Config::with([
        std::env::consts::OS.to_string(),
        std::env::consts::ARCH.to_string(),
    ]);
    let (module, _meta, expand_errors) = neon_compiler::expand::expand(module, &config);
    if !expand_errors.is_empty() {
        for e in &expand_errors {
            r.eprint(e.span.clone(), &e.message);
        }
        std::process::exit(1);
    }

    // The stdlib is numbered first and the program after it, so every `ExprId` in the
    // compilation is unique — one `TypecheckResult` covers both, and stdlib bodies can be
    // checked and lowered like any other code.
    let std_sources = crate::stdlib::sources()?;
    let (std_modules, next_id) =
        neon_compiler::stdlib::parse_from(&std_sources, 0).map_err(|e| eyre!("{e}"))?;
    let mut module = module;
    neon_compiler::ast::number_exprs_from(&mut module, next_id);

    let mut modules: Vec<(Vec<String>, &_)> =
        std_modules.iter().map(|(p, m)| (p.clone(), m)).collect();
    modules.push((Vec::new(), &module));

    let unit = if lib { Unit::Library } else { Unit::RootApplication };
    let mut env = Env::build_with(&modules, unit);
    if !env.errors().is_empty() {
        for e in env.errors() {
            r.eprint_full(e.span.clone(), &e.to_string(), &e.labels(), e.help().as_deref());
        }
        std::process::exit(1);
    }
    let (result, errs) = neon_compiler::typecheck::check::check_all(&mut env, &modules);

    // Checking produces errors through *two* channels, and both have to be read. The
    // returned `errs` are the checker's own; resolving a type annotation raises through
    // `Env::error` instead, so an unknown type written inside a function body lands in
    // `env.errors()` and nowhere else. Only the pre-check `env.errors()` was consulted
    // above, so those were dropped: `let x: NoSuchType = 5` compiled clean, and the poison
    // type it produced reached codegen, where the backend's guard fired and the user got
    // an internal compiler error instead of "unknown type". Every bad type name written in
    // a body behaved that way, not just the one that exposed it.
    let mut errs = errs;
    errs.extend(env.errors().iter().cloned());
    if !errs.is_empty() {
        for e in &errs {
            r.eprint_full(e.span.clone(), &e.to_string(), &e.labels(), e.help().as_deref());
        }
        std::process::exit(1);
    }
    Ok(Checked { env, result, module, libs: std_modules })
}
