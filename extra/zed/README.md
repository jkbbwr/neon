# Neon for Zed

Registers `.neon` files as a language and starts [`neon-lsp`](../../lsp) against them.

**There is no syntax highlighting.** That is not an oversight; see
[Why there is no highlighting](#why-there-is-no-highlighting) below. What you get is:

| Feature | Source |
| --- | --- |
| `.neon` recognised as the "Neon" language | `languages/neon/config.toml` |
| Inline diagnostics | `neon-lsp` (`publishDiagnostics`) |
| Format buffer / format on save | `neon-lsp` (`textDocument/formatting`) |
| `//` and `///` comment toggling and continuation | `languages/neon/config.toml` |
| `/* */` block comments, bracket matching, 4-space indent | `languages/neon/config.toml` |

`neon-lsp` advertises exactly two capabilities — diagnostics and formatting — and this
extension declares nothing beyond them. There is no hover, completion, go-to-definition or
rename, because the server does not implement them. See the module docs in
`lsp/src/main.rs` for why that list is deliberately short.

## Install as a dev extension

The extension is a Rust crate compiled to WebAssembly. `extra/` is excluded from the root
Cargo workspace (see the root `Cargo.toml`) precisely so this crate is not built for the
host target as a workspace member.

1. Install the wasm target once:

   ```sh
   rustup target add wasm32-wasip2
   ```

2. Build `neon-lsp` and put it somewhere on your `$PATH`:

   ```sh
   cargo build --release -p neon-lsp
   # then, e.g.
   ln -sf "$PWD/target/release/neon-lsp" ~/.local/bin/neon-lsp
   ```

3. In Zed, open the command palette and run **`zed: install dev extension`**, then select
   this directory (`extra/zed`). Zed compiles the crate itself; you do not need to run
   `cargo build` here.

After editing anything in this directory, run **`zed: reload extensions`**.

## Pointing it at the toolchain

Two things must be found: the server binary, and the sysroot.

### The binary

Resolved in this order:

1. `lsp.neon-lsp.binary.path` in your Zed settings.
2. `neon-lsp` on `$PATH`.

If neither resolves, the extension fails with a message naming both options rather than
launching something that does not exist.

### `NEON_SYSROOT` — read this one

`neon-lsp` loads the standard library from `$NEON_SYSROOT/stdlib`. If that variable is
unset or wrong, **the server still starts and still reports diagnostics** — but
`load_stdlib` returns nothing, the type checker is skipped entirely, and you silently get
lexer and parser errors only. Nothing in the editor indicates this. If Neon files show
syntax errors but never type errors, this is why.

The extension resolves it in this order:

1. `lsp.neon-lsp.binary.env.NEON_SYSROOT` in your Zed settings.
2. `NEON_SYSROOT` inherited from your shell environment.
3. Auto-detected: if the worktree root contains `stdlib/prelude.neon`, the worktree root is
   used. Opening this repository therefore works with no configuration at all.

If none apply, the variable is left unset rather than guessed at.

To set it explicitly, in Zed's `settings.json`:

```json
{
  "lsp": {
    "neon-lsp": {
      "binary": {
        "path": "/absolute/path/to/neon-lsp",
        "env": { "NEON_SYSROOT": "/absolute/path/to/neon/checkout" }
      }
    }
  }
}
```

To format on save, add:

```json
{
  "languages": {
    "Neon": { "format_on_save": "on" }
  }
}
```

A file that does not parse is left untouched by the formatter — it reprints from the AST,
so a half-written line produces no edits rather than an error popup.

## Why there is no highlighting

Zed highlights via tree-sitter, and a grammar must be fetched from a **git repository plus
revision**:

```toml
[grammars.neon]
repository = "https://github.com/..."
rev = "..."
```

`GrammarManifestEntry` in Zed's source makes `repository` and `rev` required and offers no
local-path option, so a grammar cannot be vendored into this directory and used. This
repository currently has no published remote hosting a Neon grammar, so there is no honest
value to put there.

Zed's `LanguageConfig.grammar` field is `Option<Arc<str>>`, so omitting it is supported
rather than a hack: the language registers, the file type is recognised, and the language
server attaches. Only tree-sitter-driven features (highlighting, structural selection,
code folding by syntax) are absent.

### Prior art, if you want to fix this

A tree-sitter grammar for Neon *does* exist, written against the **predecessor** repository
(`github.com/jkbbwr/neon`, at `extra/tree-sitter-neon`). It is not wired up here because:

- it targets the older language and is missing at least `marker`, `bench` and
  `assert_throws`, and has no rule for string interpolation (`"#{expr}"`);
- it lives in a different repository from this one.

Wiring it up means publishing a grammar at a reachable `repository` + `rev`, updating it
against `compiler/src/lexer/token.rs` (the authoritative keyword list), adding
`highlights.scm` / `folds.scm` / `indents.scm` under `languages/neon/`, and adding the
`[grammars.neon]` block plus `grammar = "neon"` in `languages/neon/config.toml`. None of
that is done, and none of it is claimed here.

## What has and has not been verified

Verified on this machine:

- `cargo build --release --target wasm32-wasip2` succeeds and produces a wasm component;
  `cargo clippy` is clean. Resolved `zed_extension_api` 0.7.0, the current published
  release and the same version other installed extensions use.
- Both TOML files parse, and every key used was checked field-by-field against Zed's
  `ExtensionManifest`, `GrammarManifestEntry` and `LanguageConfig` structs in Zed's source
  rather than against the documentation. (The docs claim `grammar` is required in a
  language config; the source says otherwise, and the source is what runs.)

Not verified:

- **The extension has not been loaded into a running Zed.** No `zed` binary was reachable
  from this shell, so "installs and attaches successfully" is reasoned from Zed's source,
  not observed. Confirm with `zed: install dev extension` and check the language server
  logs (`zed: open log`).
