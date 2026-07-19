//! Running the front end for an editor rather than for a build.
//!
//! The CLI's `check` renders diagnostics to stderr and calls `exit(1)` at the first stage
//! that fails, which is right for a build and useless here: an editor needs the errors as
//! *data*, and needs them for a file that does not compile — that is the only time anyone
//! is looking. So this walks the same pipeline and collects instead of exiting.
//!
//! It stops at the first stage that produces errors, deliberately. Parse errors make the
//! AST a guess, and type errors derived from a guessed AST are noise that sends people
//! chasing problems they do not have. One real error beats twenty invented ones.

use neon_compiler::typecheck::env::Unit;
use neon_compiler::typecheck::Env;
use neon_compiler::{ast, expand, lexer, parser, stdlib};
use std::ops::Range;

/// One diagnostic, in byte offsets. Converting to LSP's line/column pairs is the
/// protocol layer's job — this stays in the compiler's own coordinate system so nothing
/// here has to think about UTF-16.
pub struct Diagnostic {
    pub span: Range<usize>,
    pub message: String,
    /// Extra spans that explain the error, each with its own note. Rendered as related
    /// information so an editor can offer to jump to them.
    pub labels: Vec<(Range<usize>, String)>,
    pub help: Option<String>,
}

impl Diagnostic {
    fn plain(span: Range<usize>, message: String) -> Self {
        Diagnostic { span, message, labels: Vec::new(), help: None }
    }
}

/// The stdlib, parsed once and reused for every check in the session.
///
/// **Why this cache is sound.** The obvious worry is two compilations sharing mutable
/// state and one leaking into the other's diagnostics. They cannot here, because nothing
/// downstream can mutate these modules: `Env::build_with` and `check::check_all` both take
/// `&[(Vec<String>, &ast::Module)]`, the AST has no interior mutability, and the `Env` that
/// accumulates every bit of inference is built fresh per check and dropped with it. The
/// cached value is immutable input, not carried-over results.
///
/// The `ExprId` numbering is the other half. `parse_from(sources, 0)` numbers the stdlib
/// `0..next_id` and reports `next_id`; each check then renumbers the *user's* fresh module
/// from that same base. So every check sees exactly the id assignment an uncached run
/// would have produced — the numbering is a pure function of the sources, and the sources
/// are toolchain data that cannot change while the server is running. A stdlib that does
/// change means a toolchain that changed, which means a restart.
///
/// What is *not* cached is the `Env`. Declaration and body resolution are global and
/// ordered across all modules at once (a stdlib fn may name a user type), so the stdlib's
/// half of an `Env` is not separable from the user's. Caching it would mean sharing
/// inference state between compilations, which is the unsound thing this comment opens by
/// ruling out. Parsing is the part that is genuinely independent, so parsing is the part
/// that is cached.
struct Cached {
    modules: Vec<(Vec<String>, ast::Module)>,
    next_id: u32,
}

/// The front end, bound to one session's stdlib.
pub struct Analyzer {
    /// `None` when the toolchain could not be found. The server tells the user about that
    /// separately; see `main.rs`.
    cached: Option<Cached>,
    config: expand::Config,
}

impl Analyzer {
    /// Parse the stdlib once. An unparseable stdlib is a broken toolchain rather than a
    /// user error, so it is reported as such and the server continues syntax-only.
    pub fn new(std_sources: &[(String, String)]) -> Result<Analyzer, String> {
        let (modules, next_id) = stdlib::parse_from(std_sources, 0)?;
        Ok(Analyzer { cached: Some(Cached { modules, next_id }), config: config() })
    }

    /// Lexer and parser diagnostics only, for a session with no usable toolchain.
    ///
    /// The checker is skipped rather than run against an empty stdlib on purpose: with no
    /// `std` in scope every single name in a normal file is undefined, and the resulting
    /// wall of red says nothing true about the user's code.
    pub fn syntax_only() -> Analyzer {
        Analyzer { cached: None, config: config() }
    }

    /// Everything the front end can say about one file, in pipeline order.
    pub fn diagnostics(&self, src: &str) -> Vec<Diagnostic> {
        let tokens = match lexer::lex(src) {
            Ok(t) => t,
            Err(errors) => {
                return errors
                    .iter()
                    .map(|e| Diagnostic::plain(e.span.clone(), e.to_string()))
                    .collect()
            }
        };

        let (module, errors) = parser::parse(&tokens, src.len());
        if !errors.is_empty() {
            return errors
                .iter()
                .map(|e| Diagnostic::plain(e.span.clone(), e.to_string()))
                .collect();
        }
        let Some(mut module) = module else { return Vec::new() };

        let (expanded, _meta, expand_errors) = expand::expand(module, &self.config);
        if !expand_errors.is_empty() {
            return expand_errors
                .iter()
                .map(|e| Diagnostic::plain(e.span.clone(), e.message.clone()))
                .collect();
        }
        module = expanded;

        let Some(cached) = &self.cached else { return Vec::new() };

        // Numbering the stdlib first and the user's module after it keeps every `ExprId`
        // in the compilation unique, which is what lets one `TypecheckResult` cover both.
        ast::number_exprs_from(&mut module, cached.next_id);

        let mut modules: Vec<(Vec<String>, &ast::Module)> =
            cached.modules.iter().map(|(p, m)| (p.clone(), m)).collect();
        modules.push((Vec::new(), &module));

        // `RootApplication` because an editor is nearly always looking at a program: it is
        // the stricter of the two (a library has no `main` to demand), so this errs toward
        // showing a diagnostic rather than hiding one.
        let mut env = Env::build_with(&modules, Unit::RootApplication);
        if !env.errors().is_empty() {
            return env.errors().iter().map(convert).collect();
        }

        let (_result, errs) = neon_compiler::typecheck::check::check_all(&mut env, &modules);
        errs.iter().map(convert).collect()
    }
}

/// The active `@cfg` keys are this machine's, matching what a build here would do. An
/// editor showing code as live that the local target would drop is worse than the reverse,
/// since the dropped branch is the one nobody is checking.
fn config() -> expand::Config {
    expand::Config::with([
        std::env::consts::OS.to_string(),
        std::env::consts::ARCH.to_string(),
    ])
}

/// A checker error, with the labels and help it carries. Both are dropped by the plain
/// path above because lexer and parser errors have neither.
fn convert(e: &neon_compiler::typecheck::env::TypeError) -> Diagnostic {
    Diagnostic {
        span: e.span.clone(),
        message: e.to_string(),
        labels: e.labels(),
        help: e.help(),
    }
}
