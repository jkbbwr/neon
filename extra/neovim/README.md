# neon.nvim

Neovim support for the [Neon](../../) language: filetype detection, syntax
highlighting, indentation, and a client for the `neon-lsp` language server.

This lives under `extra/`, deliberately outside the cargo workspace. It is a plain
Neovim runtime directory — nothing here is built.

## What you actually get

| Feature | Status |
| --- | --- |
| `.neon` filetype detection | yes |
| Syntax highlighting (vim regex) | yes |
| Indentation | yes |
| `commentstring`, `comments`, `formatoptions` | yes |
| Diagnostics from `neon-lsp` | yes |
| Formatting (`gq`-independent; `vim.lsp.buf.format`) | yes |
| Hover, completion, go-to-definition, rename, signature help | **no** |

That last row is not an oversight. `neon-lsp` advertises exactly two server
capabilities — `publishDiagnostics` and `documentFormattingProvider` — and nothing
else (see the module docs at the top of `lsp/src/main.rs`, which explain why). This
plugin does not configure keymaps or handlers for capabilities the server does not
have, because an editor that appears to offer go-to-definition and then does nothing
is worse than one that never offered it.

There is **no tree-sitter grammar for Neon**. None exists. Highlighting here is a
hand-written vim regex syntax file derived from `compiler/src/lexer/token.rs`; do not
expect `nvim-treesitter` to have anything to install.

## Requirements

- Neovim **0.11+** for the LSP integration on the modern `vim.lsp.config` /
  `vim.lsp.enable` path. Neovim **0.8–0.10** falls back to `vim.lsp.start` driven by
  a `FileType` autocmd. The plugin picks the path by feature-detecting
  `vim.lsp.enable`, not by parsing a version string.
- The `neon-lsp` binary on `$PATH` (or an absolute path in `cmd`).
- `nvim-lspconfig` is **not** required and is not used. If you already run it for
  other languages that is fine — the two do not interact, since this plugin
  registers a server named `neon-lsp` through the built-in API only.

Syntax, indent and filetype settings work on any Neovim with `vim.filetype.add`
(0.7+) and need no `setup()` call.

## Building the language server

From the repository root:

```sh
cargo build --release -p neon-lsp
```

The binary lands at `target/release/neon-lsp`. Put it on `$PATH`, or point `cmd` at
it directly.

## The sysroot — read this part

`neon-lsp` reads the environment variable **`NEON_SYSROOT`**, which must name a
directory containing a `stdlib/` subdirectory.

If it is unset or wrong, **the server does not fail**. `load_stdlib` in
`lsp/src/main.rs` returns an empty list, the type checker is skipped entirely, and
you get lexer and parser diagnostics only. Every type error in your file silently
disappears. This is the single most likely reason for "the LSP seems to work but
never catches anything", so the plugin warns once per session when it cannot find a
valid sysroot, and `:NeonInfo` reports what it resolved.

Resolution order:

1. the `sysroot` option passed to `setup()` (a string, or a function returning one —
   run through `expand()`, so `~` works);
2. `NEON_SYSROOT` already present in Neovim's environment;
3. nothing, and you get the warning.

Whatever is resolved is passed to the server process as `NEON_SYSROOT` via `cmd_env`.

For an installed toolchain the sysroot is the prefix that holds `stdlib/`, `include/`
and `lib/` — the same directory the `neon` CLI probes (`cli/src/sysroot.rs`). For a
development checkout, the repository root works, because `stdlib/` sits there.

## Install

### lazy.nvim

From a local checkout of this repository:

```lua
{
  dir = "/path/to/neon/extra/neovim",
  ft = "neon",
  opts = {
    -- Point at wherever stdlib/ lives. Omit if NEON_SYSROOT is set in your shell.
    sysroot = "/path/to/neon",
  },
}
```

If the plugin is published as its own repository, swap `dir` for the usual short
name. Note `ft = "neon"` defers loading until you open a Neon file; `opts` makes
lazy.nvim call `require("neon").setup(opts)` for you.

Configure it explicitly instead if you prefer:

```lua
{
  dir = "/path/to/neon/extra/neovim",
  ft = "neon",
  config = function()
    require("neon").setup({
      sysroot = vim.env.NEON_SYSROOT or "/path/to/neon",
      cmd = { "neon-lsp" },
      format_on_save = true,
    })
  end,
}
```

### packer.nvim

```lua
use({
  "/path/to/neon/extra/neovim",
  ft = { "neon" },
  config = function()
    require("neon").setup({ sysroot = "/path/to/neon" })
  end,
})
```

### No plugin manager

```vim
set runtimepath+=/path/to/neon/extra/neovim
```

then `lua require('neon').setup({ sysroot = '/path/to/neon' })` in your `init.lua`.

## Options

```lua
require("neon").setup({
  -- The server command. Use an absolute path if it is not on $PATH.
  cmd = { "neon-lsp" },

  -- NEON_SYSROOT for the server. String, or a function returning one.
  sysroot = nil,

  -- Upward search for the project root. `neon.toml` is the Neon manifest
  -- (cli/src/project.rs); `.git` is the fallback for a loose file.
  root_markers = { "neon.toml", ".git" },

  -- Run the server's formatter on :w. Off by default.
  -- Safe mid-edit: a file that does not parse yields an empty edit list rather
  -- than an error, so a half-written line is left exactly as you typed it.
  format_on_save = false,

  -- Called with (client, bufnr) when the server attaches.
  on_attach = nil,

  -- Merged into the client capabilities, e.g. from a completion plugin.
  -- Note this cannot conjure capabilities the *server* lacks.
  capabilities = nil,

  settings = {},
  autostart = true,
  warn_on_missing_sysroot = true,
})
```

## Commands

- `:NeonInfo` — print the resolved server path, sysroot, whether `stdlib/` was found
  there, and which LSP API path is in use. Start here when something is off.

Formatting is `vim.lsp.buf.format()`, or set `format_on_save = true`. There is no
`:NeonFormat` wrapper; the built-in is the same thing with a better name.

## Syntax details

`syntax/neon.vim` is transcribed from `compiler/src/lexer/token.rs` (`Token::keyword`
for the reserved words, the `Display` impl for the punctuation) and
`compiler/src/expand.rs` (`lookup`, for annotations). Specifically:

- All 49 reserved words, including the word-spelled bitwise operators `band`, `bor`,
  `bxor`, `bnot`, `bsl`, `bsr`.
- `enum` is **not** highlighted, because it is **not** a keyword — token.rs says so
  explicitly. Sum types are unions of records.
- Atoms `:name`, using the lexer's actual rule: a `:` opens an atom only when an
  identifier follows immediately *and* one does not precede immediately. That is what
  keeps `x: i64` an annotation, `x:y` punctuation, and `std::io` a path.
- String interpolation `"a #{expr} b"`, nesting arbitrarily, including strings inside
  the hole.
- Escapes `\n \r \t \0 \\ \' \" \#`, `\xNN`, `\u{...}` — the set `escape()` accepts.
- Numbers with `0x`/`0o`/`0b` prefixes and `_` separators; floats require a digit
  after the dot, so `x.0` (field access) and `xs..1` (spread) are not floats.
- Comments `//`, doc comments `///` (but not `////`), and `/* */` block comments,
  which **nest** — the lexer supports nesting and so does the syntax file.
- The five valid annotations `@native @cfg @doc @runtime @pure` are highlighted as
  preprocessor directives; any other `@name` is highlighted as an **error**, matching
  the compiler, which rejects it.

Only `i64`, `f64`, `str` and `bool` are highlighted as primitive types — those are
what `primitive()` in `compiler/src/typecheck/resolve.rs` recognises — plus `any`,
which the parser treats as its own type-spec kind. Prelude names (`List`, `Map`,
`Display`, `Error`, `Ord`, `Ordering`, `IndexError`) are highlighted as types because
they are in scope in every file without a `use`.

## Indentation

`indent/neon.vim` is a brace-matching heuristic, not a parser. It deliberately does
not use `cindent`, which mishandles Neon twice over: `cindent` reads `:` as a label
(breaking both `x: i64` and the atom `:ok`) and treats a leading `#` as a
preprocessor line (colliding with `#{` interpolation).

It was checked against every file in `stdlib/` — 11 files, all formatted by
`neon fmt`. `gg=G` changes zero lines in all of them.

Multi-line strings, and braces inside a `/* */` block that spans lines, can still
confuse it. Reindenting such a region by hand is the workaround; nothing here fights
you afterwards.

## What is verified, and what is not

Verified by running Neovim 0.12.4 headless against this directory:

- filetype detection, `commentstring`, `shiftwidth`, syntax and indent all load;
- every syntax group listed above resolves to the right highlight on a test file
  exercising each construct;
- `gg=G` is a no-op on all 11 stdlib files;
- `neon-lsp` attaches to a `.neon` buffer, reports `documentFormattingProvider =
  true`, receives `NEON_SYSROOT` through `cmd_env`, and publishes diagnostics.

**Not** verified: the Neovim 0.8–0.10 `vim.lsp.start` fallback path. It is written
against the documented API but has not been executed, because only 0.12 was available
here. If you are on 0.10 or older and it misbehaves, that is the code to look at
(`start_legacy` in `lua/neon/init.lua`).
