-- User commands. Loaded automatically; does not start anything on its own.

if vim.g.loaded_neon then
  return
end
vim.g.loaded_neon = true

vim.api.nvim_create_user_command('NeonInfo', function()
  require('neon').info()
end, { desc = 'Show the resolved neon-lsp command and sysroot' })
