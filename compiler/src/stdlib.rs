//! Turning stdlib source into prefixed modules.
//!
//! Pure, per the filesystem rule: the CLI and the test harness read the files and
//! hand the text here. `stdlib/std/io.neon` becomes the module `std::io`, by path —
//! there is no `mod std { }` wrapper in the source.

use crate::ast::Module;
use crate::{lexer, parser};

/// The module prefix a stdlib-relative path denotes: `std/io.neon` → `["std","io"]`,
/// `std/collections/list.neon` → `["std","collections","list"]`.
pub fn module_path(rel: &str) -> Vec<String> {
    let rel = rel.strip_suffix(".neon").unwrap_or(rel);
    // The prelude is declared at the root, so `Display`, `Ordering` and the rest
    // resolve by their short names from any module without a `use` — which is what
    // being in the prelude means.
    if rel == "prelude" {
        return Vec::new();
    }
    rel.split(['/', '\\']).filter(|s| !s.is_empty()).map(String::from).collect()
}

/// Parse `(relative-path, source)` pairs into prefixed modules.
///
/// A stdlib file that does not lex or parse is a broken toolchain, not a user error,
/// so it is an `Err` naming the file rather than a diagnostic.
pub fn parse(sources: &[(String, String)]) -> Result<Vec<(Vec<String>, Module)>, String> {
    let mut out = Vec::with_capacity(sources.len());
    for (rel, src) in sources {
        let tokens = lexer::lex(src).map_err(|e| format!("stdlib `{rel}` did not lex: {e:?}"))?;
        let (module, errors) = parser::parse(&tokens, src.len());
        if !errors.is_empty() {
            return Err(format!("stdlib `{rel}` did not parse: {errors:?}"));
        }
        let module = module.ok_or_else(|| format!("stdlib `{rel}` produced no module"))?;
        out.push((module_path(rel), module));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_path_from_relative() {
        assert_eq!(module_path("std/io.neon"), vec!["std", "io"]);
        assert_eq!(module_path("std/collections/list.neon"), vec!["std", "collections", "list"]);
        // The prelude declares at the root, so its short names need no `use`.
        assert_eq!(module_path("prelude.neon"), Vec::<String>::new());
        assert_eq!(module_path("std/prelude.neon"), vec!["std", "prelude"]);
    }

    #[test]
    fn parses_a_native_signature() {
        let src = "@native(\"neon_io_println\") fn println(s: str)".to_string();
        let loaded = parse(&[("std/io.neon".to_string(), src)]).expect("parses");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].0, vec!["std", "io"]);
    }
}
