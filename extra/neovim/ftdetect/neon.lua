-- Filetype detection for Neon source files.
--
-- `vim.filetype.add` is the modern mechanism (Neovim 0.7+) and is preferred over an
-- autocmd because it participates in the same lookup table as the built-in rules
-- rather than racing them.
vim.filetype.add({
  extension = {
    neon = "neon",
  },
})
