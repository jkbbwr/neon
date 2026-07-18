use crate::source;
use color_eyre::eyre::Result;
use neon_compiler::diagnostic::Renderer;
use neon_compiler::ir::{self, ssa::print, Stage};
use neon_compiler::typecheck::env::Unit;
use neon_compiler::typecheck::Env;
use neon_compiler::{lexer, parser};
use std::ffi::OsString;
use std::path::PathBuf;

/// Emit the IR for a source file at the requested pipeline stage.
pub fn run(file: &OsString, stage: Stage) -> Result<()> {
    let path = PathBuf::from(file);
    let src = source::read(&path)?;
    let mut r = Renderer::for_stderr(&path, &src);

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

    let std_sources = crate::stdlib::sources()?;
    let std_modules =
        neon_compiler::stdlib::parse(&std_sources).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
    let mut modules: Vec<(Vec<String>, &_)> =
        std_modules.iter().map(|(p, m)| (p.clone(), m)).collect();
    modules.push((Vec::new(), &module));

    let mut env = Env::build_with(&modules, Unit::RootApplication);
    if !env.errors().is_empty() {
        for e in env.errors() {
            r.eprint_full(e.span.clone(), &e.to_string(), &e.labels(), e.help().as_deref());
        }
        std::process::exit(1);
    }
    let (result, errs) = neon_compiler::typecheck::check::check_module(&mut env, &module);
    if !errs.is_empty() {
        for e in &errs {
            r.eprint_full(e.span.clone(), &e.to_string(), &e.labels(), e.help().as_deref());
        }
        std::process::exit(1);
    }

    let program = ir::compile(&env, &result, &module, stage);
    print!("{}", print::program(&program));
    Ok(())
}
