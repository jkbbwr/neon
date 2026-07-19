//! Zed extension glue for Neon.
//!
//! The only job here is producing the command that starts `neon-lsp`. Everything the
//! server can actually do is declared in `extension.toml`, and it is two things:
//! diagnostics and document formatting.
//!
//! The one piece of real logic is `NEON_SYSROOT`. Without it the server still starts, but
//! `load_stdlib` in `lsp/src/main.rs` returns nothing and the checker is skipped entirely,
//! so the editor silently shows only lexer and parser errors. That failure is invisible
//! from inside Zed -- the server looks healthy -- so this resolves the sysroot eagerly and
//! from the most specific source available.

use std::collections::HashMap;
use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

const SERVER_BINARY: &str = "neon-lsp";

/// A file that exists in a Neon sysroot and nowhere else, used to recognise one.
const SYSROOT_MARKER: &str = "stdlib/prelude.neon";

struct NeonExtension;

impl zed::Extension for NeonExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok();
        let binary = settings.as_ref().and_then(|settings| settings.binary.as_ref());

        // An explicit setting wins over `$PATH`, so a checkout can be pointed at its own
        // freshly built server without touching the user's environment.
        let command = binary
            .and_then(|binary| binary.path.clone())
            .or_else(|| worktree.which(SERVER_BINARY))
            .ok_or_else(|| {
                format!(
                    "`{SERVER_BINARY}` was not found on $PATH. Build it with \
                     `cargo build --release -p neon-lsp` and put the binary on your $PATH, \
                     or set `lsp.neon-lsp.binary.path` in your Zed settings."
                )
            })?;

        let args = binary
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();

        // Start from the user's shell environment: the server is a normal toolchain
        // process and a sysroot exported from a shell profile should reach it.
        let mut env: HashMap<String, String> = worktree.shell_env().into_iter().collect();
        if let Some(configured) = binary.and_then(|binary| binary.env.clone()) {
            env.extend(configured);
        }

        // Only fill in a sysroot nobody specified. An explicit one, from either source
        // above, is a deliberate choice and is left alone.
        if !env.contains_key("NEON_SYSROOT") {
            if let Some(sysroot) = detect_sysroot(worktree) {
                env.insert("NEON_SYSROOT".to_string(), sysroot);
            }
        }

        Ok(zed::Command {
            command,
            args,
            env: env.into_iter().collect(),
        })
    }
}

/// The sysroot to use when the environment did not name one.
///
/// Opening the Neon repository itself is the common case -- that worktree *is* a sysroot,
/// since it contains `stdlib/` -- and detecting it means the extension works in a fresh
/// checkout with no configuration at all.
///
/// A worktree without the marker yields nothing rather than a guess. Pointing the server
/// at a directory that is not a sysroot buys nothing over leaving the variable unset -- it
/// degrades identically -- and an unset variable is the state the server's own docs
/// describe, so it is the easier one to recognise when diagnosing a quiet session.
fn detect_sysroot(worktree: &zed::Worktree) -> Option<String> {
    worktree
        .read_text_file(SYSROOT_MARKER)
        .ok()
        .map(|_| worktree.root_path())
}

zed::register_extension!(NeonExtension);
