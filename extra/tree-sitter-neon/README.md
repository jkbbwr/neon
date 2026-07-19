# tree-sitter-neon

A [tree-sitter](https://tree-sitter.github.io) grammar for Neon.

The grammar is derived from the compiler, not from the documentation:

| Source | What it settles |
| --- | --- |
| `compiler/src/lexer/token.rs` | the token alphabet and the reserved-word list |
| `compiler/src/lexer/mod.rs` | literal forms, comments, string interpolation |
| `compiler/src/parser/mod.rs` | the grammar itself, in chumsky |
| `compiler/src/ops.rs` | the one binary-operator precedence table |

`ops.rs` matters most. It is the single ladder that both the parser and the
formatter read — a second copy is what once made the formatter reprint
`1 - (2 - 3)` as `1 - 2 - 3`. The `PREC` table at the top of `grammar.js` is
that table transcribed, level for level. **If `ops.rs` changes, change it here
too**, and re-run the precedence tests in `test/corpus/expressions.txt`.

## Status

Parses **250 of the 251** `.neon` files in `tests/lang/` and `stdlib/` with no
`ERROR` or `MISSING` node. The one exception is
`tests/lang/strings/interpolation_unterminated_fails.neon`, which is a
`//@ compile-fail` test containing a deliberately unterminated interpolation —
an error node there is the correct answer.

`tree-sitter generate` is clean: no unresolved conflicts, and no unnecessary
ones. The five declared conflicts are each a genuine local ambiguity that the
compiler also has to resolve, and each is commented in `grammar.js`.

## Building

```sh
npm install          # only needed for the tree-sitter CLI
tree-sitter generate # regenerate src/ from grammar.js
tree-sitter test     # run test/corpus
tree-sitter build    # build a shared object for local use
```

`src/` (`parser.c`, `scanner.c`, `grammar.json`, `node-types.json`) is
committed, as is conventional, so a consumer needs no build step and no
tree-sitter CLI.

To re-check the corpus:

```sh
for f in $(find ../../tests/lang ../../stdlib -name '*.neon'); do
  tree-sitter parse -q "$f" >/dev/null || echo "FAIL $f"
done
```

### The external scanner

`src/scanner.c` exists for exactly one reason: Neon's block comments **nest**,
so a commented-out block containing a comment does not end early. No regular
expression can count, so the nesting depth is tracked in C. Any consumer must
compile `scanner.c` alongside `parser.c`.

## Editor setup

Everything below assumes the grammar is fetched from this repository at
`extra/tree-sitter-neon`, and that `scanner.c` is compiled in.

### Neovim

```lua
require('nvim-treesitter.parsers').get_parser_configs().neon = {
  install_info = {
    url = 'https://github.com/jkbbwr/neon2',
    location = 'extra/tree-sitter-neon',
    files = { 'src/parser.c', 'src/scanner.c' },
  },
  filetype = 'neon',
}
vim.filetype.add({ extension = { neon = 'neon' } })
```

Copy `queries/` into `~/.config/nvim/queries/neon/`, or let
nvim-treesitter pick them up from the installed parser directory.

### Zed

A Zed extension points at the grammar in `extension.toml` and copies
`queries/highlights.scm` into the extension's `languages/neon/` directory. Zed
reads a narrower set of capture names than Neovim; see the divergence note
below.

### Anything using the `tree-sitter` CLI

`tree-sitter.json` declares the scope, file types and highlight query, so
`tree-sitter highlight some.neon` works once this directory is on your
`parser-directories`.

## Queries

| File | Verified how |
| --- | --- |
| `queries/highlights.scm` | `tree-sitter highlight --check` over all 251 corpus files, zero failures |
| `queries/indents.scm` | compiles and captures correctly under `tree-sitter query` |
| `queries/textobjects.scm` | compiles and captures correctly under `tree-sitter query` |

There is deliberately **no `injections.scm`**. The obvious candidate would be
string interpolation, but `"a #{expr} b"` is not an injection: the lexer emits
the hole as a real token run and the grammar parses `expr` as ordinary Neon, so
`(interpolation)` already contains first-class expression nodes. Injecting a
second parser there would be strictly worse.

`indents.scm` and `textobjects.scm` are verified to compile and to capture the
nodes they name. They are *not* verified against Neovim's indent or textobject
behaviour end to end, because that needs a Neovim harness this repository does
not have. Treat them as a good starting point rather than as tested.

### Capture-name divergence

The queries target the capture names Neovim and Zed share. Two things differ:

- **Ordering.** Neovim and Zed both let a *later* matching pattern override an
  earlier one, so `highlights.scm` goes general → specific: the broad
  `(identifier) @variable` is near the top and every later pattern narrows it.
  The `tree-sitter highlight` CLI resolves ties the other way, so its output is
  coarser than what an editor shows. That is a CLI limitation, not a query bug.
- **Vocabulary.** Neovim understands the fine-grained names used here
  (`@keyword.conditional`, `@keyword.repeat`, `@keyword.exception`,
  `@variable.member`, `@type.definition`, `@number.float`, `@module`). Zed maps
  unknown captures to nothing, so on Zed those fall back to unstyled. If you
  care, add coarse duplicates (`@keyword`, `@property`, `@type`, `@number`) in
  the Zed extension's copy of the file.

## Known divergences from the compiler

Three places where this grammar deliberately does not match
`compiler/src/parser/mod.rs`. None of them affects the corpus.

1. **`{}` is a block, not an empty record literal.** The compiler's
   `atom_expr` tries `record_lit` before `block_expr`, so a bare `{}` in
   expression position is an empty record. Here a record literal requires
   either a path or at least one field, which is what keeps `{ .. }` and a
   block apart without a second copy of the expression grammar.

2. **Turbofish arguments are restricted.** `f[T](x)` and `xs[i]` are the same
   tokens until the `(` after the `]`. Allowing arbitrary types inside the
   brackets made that ambiguity combinatorial, so `turbofish_arguments` accepts
   only types that cannot begin like a parenthesised expression: names, paths,
   generics, atoms, `any`, `null`, and unions/intersections/negations of those.
   `f[(i64) -> str]()` does not parse; name the type with an alias. Nothing in
   the corpus writes one.

3. **Condition position is handled by GLR, not by a second grammar.** The
   compiler builds a whole second expression grammar (`cond`) with record
   literals switched off, so `while a { }` cannot read `a { }` as an empty
   record. Here both readings are explored and the record reading dies for want
   of a block, which reaches the same answer without doubling every rule. As in
   the compiler, parenthesise to get a record literal back: `while (a { }) { }`.

## Relationship to the older grammar

The predecessor repository (`jkbbwr/neon`, `extra/tree-sitter-neon` at
`fc53a03`) also has a Neon grammar. This one is **written fresh, not derived
from it**, and the node names are **not compatible**.

That grammar describes an older language: it has `enum_declaration`,
`enum_pattern`, `if_let_expr`, `let_expr`, `map_init`, `list_pattern`, `sigil`
and `type_nullable`, none of which exist in Neon now, and it has no rule for
string interpolation, `marker`, `bench` or `assert_throws`. Its precedence
ladder also disagrees with `ops.rs` — it has a `cast` level and no `orelse`
level at all. Adapting it would have meant rewriting every rule anyway, without
the benefit of having checked each one against the current parser.

Its naming convention differs throughout (`binary_expr`, `call_expr`,
`int_literal`, `string_literal`, `type_spec`, `module_path` against this
grammar's `binary_expression`, `call_expression`, `integer`, `string`, `_type`,
`path`). **Any editor configuration written against the old grammar — including
an installed Zed extension — needs its queries updated, not just repointed.**
