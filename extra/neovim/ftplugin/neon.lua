-- Buffer-local settings for Neon.
--
-- Everything here is what the buffer needs to behave sanely with no tree-sitter
-- grammar and no language server: comments, indent width, and the `formatoptions`
-- that make `gq` and auto-wrapping respect comment leaders.
--
-- There is no Neon tree-sitter parser in existence. Highlighting comes from
-- `syntax/neon.vim`, indentation from `indent/neon.vim`.

if vim.b.did_ftplugin then
  return
end
vim.b.did_ftplugin = true

local bo = vim.bo
local o = vim.opt_local

-- Comments. The lexer accepts `//` line comments, `///` doc comments, and `/* */`
-- block comments which *nest* (see `next_trivia` in compiler/src/lexer/mod.rs).
bo.commentstring = "// %s"

-- Ordering matters: Neovim tries these left to right, so the three-slash doc form
-- must precede the two-slash form or `///` would be matched as `//` plus a stray
-- slash. `s`/`m`/`e` describe the start, middle and end of the block form.
bo.comments = "s1:/*,mb:*,ex:*/,:///,://"

-- `formatoptions`:
--   c  wrap comments at textwidth        j  join comment lines intelligently
--   r  continue the leader on <CR>       q  allow `gq` to format comments
--   o  continue the leader on o/O        n  recognise numbered lists
-- `t` is deliberately absent: auto-wrapping code, as opposed to comments, is
-- almost never wanted.
o.formatoptions:remove("t")
o.formatoptions:append("croqnj")

-- Four spaces, no tabs -- matching `neon fmt`'s output.
bo.expandtab = true
bo.shiftwidth = 4
bo.softtabstop = 4
bo.tabstop = 4

-- `-` is not part of an identifier in Neon, but `_` is; the default `iskeyword`
-- already covers that. Atoms are written `:name`, so make `:` count for `*`-search
-- and `w`-motions over an atom... deliberately NOT done: it would break `x: i64`
-- annotations under `*`. Left as a note so it is not "fixed" by accident.

-- Where `gf` looks. Module paths are `::`-separated rather than filesystem paths,
-- so this only helps for literal relative paths; it is a small win, not a claim
-- that `gf` follows `use std::io`.
o.suffixesadd:prepend(".neon")

-- Match `[[`, `]]`, and `%` across the block-ish keywords the language has.
-- `matchit` is a built-in plugin; this is a no-op when it is not loaded.
vim.b.match_words = table.concat({
  [[\<if\>:\<else\>]],
  [[\<try\>:\<catch\>]],
}, ",")

vim.b.undo_ftplugin = table.concat({
  "setlocal commentstring< comments< formatoptions<",
  "expandtab< shiftwidth< softtabstop< tabstop< suffixesadd<",
  "| unlet! b:match_words b:did_ftplugin",
}, " ")
