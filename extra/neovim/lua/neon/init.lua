--- Neovim support for the Neon language.
---
--- Call `require('neon').setup{}` to start the language server for `.neon` buffers.
--- Filetype detection, syntax and indent work with no setup call at all -- they are
--- plain runtime files.
---
--- The language server (`neon-lsp`) advertises exactly two capabilities:
--- publishDiagnostics and document formatting. There is no hover, no completion, no
--- go-to-definition and no rename, and this plugin does not configure keymaps that
--- pretend otherwise. See the module docs in `lsp/src/main.rs`.

local M = {}

--- @class neon.Config
--- @field cmd? string[] The server command. Default: `{ 'neon-lsp' }`.
--- @field sysroot? string Value for `NEON_SYSROOT`. See `resolve_sysroot`.
--- @field root_markers? string[] Files that identify a project root.
--- @field settings? table Passed through to the server as `workspace/configuration`.
--- @field on_attach? fun(client: table, bufnr: integer)
--- @field capabilities? table Client capabilities, e.g. from a completion plugin.
--- @field format_on_save? boolean Default `false`.
--- @field autostart? boolean Default `true`.
--- @field warn_on_missing_sysroot? boolean Default `true`.

--- @type neon.Config
local defaults = {
  cmd = { 'neon-lsp' },
  sysroot = nil,
  -- `neon.toml` is the manifest a Neon project is rooted at (see cli/src/project.rs).
  -- `.git` is the fallback for a loose file inside a repository.
  root_markers = { 'neon.toml', '.git' },
  settings = {},
  format_on_save = false,
  autostart = true,
  warn_on_missing_sysroot = true,
}

--- @type neon.Config
M.config = vim.deepcopy(defaults)

local function notify(msg, level)
  vim.notify('[neon] ' .. msg, level or vim.log.levels.INFO)
end

--- The sysroot to hand the server, or nil.
---
--- `neon-lsp` reads `NEON_SYSROOT` and expects a directory containing `stdlib/`. If
--- it is unset or wrong, the server does NOT fail -- `load_stdlib` returns an empty
--- list and the checker is skipped entirely, leaving only lexer and parser
--- diagnostics. That degradation is silent from the editor's side, which is exactly
--- the failure mode worth being loud about here.
---
--- Order of preference:
---   1. `config.sysroot`, as given.
---   2. `NEON_SYSROOT` already in the environment.
---   3. Nothing. The caller warns.
--- @return string|nil sysroot, string source
function M.resolve_sysroot()
  local configured = M.config.sysroot
  if type(configured) == 'function' then
    configured = configured()
  end
  if configured and configured ~= '' then
    return vim.fn.expand(configured), 'config.sysroot'
  end

  local env = vim.env.NEON_SYSROOT
  if env and env ~= '' then
    return env, 'NEON_SYSROOT'
  end

  return nil, 'unset'
end

--- Whether a resolved sysroot actually contains a `stdlib/` directory.
--- This mirrors what `load_stdlib` does: it joins `stdlib` onto the root and reads
--- it. A path without that subdirectory is as good as no path at all.
function M.sysroot_is_valid(root)
  return root ~= nil and vim.fn.isdirectory(root .. '/stdlib') == 1
end

local function project_root(bufnr)
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == '' then
    return vim.uv and vim.uv.cwd() or vim.loop.cwd()
  end
  local found = vim.fs.find(M.config.root_markers, {
    upward = true,
    path = vim.fs.dirname(name),
  })[1]
  if found then
    return vim.fs.dirname(found)
  end
  return vim.fs.dirname(name)
end

--- The environment the server is launched with: the editor's, plus NEON_SYSROOT.
local function server_env()
  local root = M.resolve_sysroot()
  if root then
    return { NEON_SYSROOT = root }
  end
  return nil
end

local warned = false

local function warn_sysroot_once()
  if warned or not M.config.warn_on_missing_sysroot then
    return
  end
  warned = true
  local root, source = M.resolve_sysroot()
  if root == nil then
    notify(
      'NEON_SYSROOT is not set. neon-lsp will report only lexer and parser errors -- '
        .. 'type errors will be missing entirely. Set it in your config '
        .. "(require('neon').setup{ sysroot = '/path/to/toolchain' }) or in your shell.",
      vim.log.levels.WARN
    )
  elseif not M.sysroot_is_valid(root) then
    notify(
      ('%s points at %q, which has no stdlib/ subdirectory. neon-lsp will skip type checking.')
        :format(source, root),
      vim.log.levels.WARN
    )
  end
end

--- Does this Neovim have the `vim.lsp.config` / `vim.lsp.enable` API?
---
--- Added in Neovim 0.11. Checked by feature rather than by version number, because
--- a version check would also have to be right about nightlies.
local function has_lsp_config_api()
  return type(vim.lsp) == 'table'
    and type(rawget(vim.lsp, 'config')) ~= 'nil'
    and type(rawget(vim.lsp, 'enable')) == 'function'
end

M.has_lsp_config_api = has_lsp_config_api

--- The bits of the config both code paths share.
local function base_config()
  return {
    cmd = M.config.cmd,
    filetypes = { 'neon' },
    settings = M.config.settings,
    capabilities = M.config.capabilities,
  }
end

local function attach(client, bufnr)
  if M.config.format_on_save then
    local group = vim.api.nvim_create_augroup('NeonFormatOnSave' .. bufnr, { clear = true })
    vim.api.nvim_create_autocmd('BufWritePre', {
      group = group,
      buffer = bufnr,
      desc = 'neon: format with neon-lsp before writing',
      callback = function()
        -- A file that does not parse yields no edits at all (the server returns an
        -- empty list rather than an error), so this is safe mid-edit.
        vim.lsp.buf.format({ bufnr = bufnr, id = client.id, timeout_ms = 3000 })
      end,
    })
  end
  if M.config.on_attach then
    M.config.on_attach(client, bufnr)
  end
end

--- Start the server for a buffer, on Neovim versions without `vim.lsp.enable`.
--- `vim.lsp.start` has existed since 0.8.
local function start_legacy(bufnr)
  local cfg = base_config()
  cfg.name = 'neon-lsp'
  cfg.root_dir = project_root(bufnr)
  cfg.cmd_env = server_env()
  cfg.on_attach = attach
  cfg.filetypes = nil -- not a `vim.lsp.start` field
  vim.lsp.start(cfg, { bufnr = bufnr })
end

--- Configure and enable the language server.
--- @param opts neon.Config|nil
function M.setup(opts)
  M.config = vim.tbl_deep_extend('force', vim.deepcopy(defaults), opts or {})

  if vim.fn.executable(M.config.cmd[1]) == 0 then
    notify(
      ('%q is not on $PATH. Build it with `cargo build --release -p neon-lsp` and either '
        .. 'add it to $PATH or set `cmd` to its absolute path.'):format(M.config.cmd[1]),
      vim.log.levels.WARN
    )
    return
  end

  if not M.config.autostart then
    return
  end

  if has_lsp_config_api() then
    -- Neovim 0.11+. `vim.lsp.enable` attaches on FileType for the configured
    -- filetypes; no autocmd of ours is involved.
    local cfg = base_config()
    cfg.cmd_env = server_env()
    cfg.root_markers = M.config.root_markers
    cfg.on_attach = attach
    vim.lsp.config['neon-lsp'] = cfg
    vim.lsp.enable('neon-lsp')
    vim.api.nvim_create_autocmd('FileType', {
      pattern = 'neon',
      group = vim.api.nvim_create_augroup('NeonLsp', { clear = true }),
      desc = 'neon: sysroot sanity check',
      callback = warn_sysroot_once,
    })
    -- `setup` may run after a .neon buffer is already open, in which case the
    -- FileType event for it has been and gone.
    for _, buf in ipairs(vim.api.nvim_list_bufs()) do
      if vim.api.nvim_buf_is_loaded(buf) and vim.bo[buf].filetype == 'neon' then
        warn_sysroot_once()
        break
      end
    end
  else
    -- Neovim 0.8 - 0.10.
    vim.api.nvim_create_autocmd('FileType', {
      pattern = 'neon',
      group = vim.api.nvim_create_augroup('NeonLsp', { clear = true }),
      desc = 'neon: start neon-lsp',
      callback = function(args)
        warn_sysroot_once()
        start_legacy(args.buf)
      end,
    })
    -- `setup` may be called after the first .neon buffer already exists.
    for _, buf in ipairs(vim.api.nvim_list_bufs()) do
      if vim.api.nvim_buf_is_loaded(buf) and vim.bo[buf].filetype == 'neon' then
        warn_sysroot_once()
        start_legacy(buf)
      end
    end
  end
end

--- Print what the plugin resolved. For "why is this not working".
function M.info()
  local root, source = M.resolve_sysroot()
  local lines = {
    'neon.nvim',
    ('  command:      %s'):format(table.concat(M.config.cmd, ' ')),
    ('  executable:   %s'):format(
      vim.fn.executable(M.config.cmd[1]) == 1 and vim.fn.exepath(M.config.cmd[1]) or 'NOT FOUND'
    ),
    ('  sysroot:      %s (from %s)'):format(root or 'unset', source),
    ('  stdlib/ ok:   %s'):format(tostring(M.sysroot_is_valid(root))),
    ('  lsp api:      %s'):format(has_lsp_config_api() and 'vim.lsp.enable (0.11+)' or 'vim.lsp.start'),
    '  capabilities: diagnostics, formatting (that is all the server advertises)',
  }
  notify(table.concat(lines, '\n'))
end

return M
