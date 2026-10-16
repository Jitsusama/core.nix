local capabilities = require('cmp_nvim_lsp').default_capabilities()
local keys = require('keys')

-- A project's root is the nearest folder holding any of these, beside the
-- prompt's own markers, so a server reads the project the prompt names
-- and never the whole of a repository holding many.
local function nearest(markers)
  return { vim.list_extend(markers, require('where').roots) }
end

-- Nix LSP
vim.lsp.config('nil_ls', {
  cmd = { 'nil' },
  filetypes = { 'nix' },
  root_markers = nearest({ 'flake.nix' }),
  capabilities = capabilities,
  settings = {
    ['nil'] = {
      formatting = { command = { 'nixfmt' } },
    },
  },
})
vim.lsp.enable('nil_ls')

-- Lua LSP
vim.lsp.config('lua_ls', {
  cmd = { 'lua-language-server' },
  filetypes = { 'lua' },
  root_markers = nearest({
    '.luarc.json',
    '.luarc.jsonc',
    '.luacheckrc',
    '.stylua.toml',
    'stylua.toml',
    'selene.toml',
    'selene.yml',
  }),
  capabilities = capabilities,
  settings = {
    Lua = {
      runtime = { version = 'LuaJIT' },
      diagnostics = { globals = { 'vim' } },
      -- Neovim's own runtime is the library: its API and vim.uv, under two
      -- hundred files, where every plugin and grammar took a minute to
      -- read. A project's own .luarc.json comes before this.
      workspace = {
        library = { vim.env.VIMRUNTIME, '${3rd}/luv/library' },
        checkThirdParty = false,
      },
      telemetry = { enable = false },
      format = {
        enable = true,
        defaultConfig = {
          indent_style = 'space',
          indent_size = '2',
        },
      },
    },
  },
})
vim.lsp.enable('lua_ls')

-- Ruby LSP
-- Disabled auto-start as it's now handled by shadowenv configuration
vim.lsp.config('ruby_lsp', {
  cmd = { 'ruby-lsp' },
  filetypes = { 'ruby' },
  root_markers = { 'Gemfile', '.git' },
})

keys.act('code.inlayHints', function()
  local enabled = vim.lsp.inlay_hint.is_enabled()
  vim.lsp.inlay_hint.enable(not enabled)
  vim.notify(
    enabled and 'Inlay hints disabled' or 'Inlay hints enabled',
    vim.log.levels.INFO,
    { title = 'Code' }
  )
end)

keys.act('code.folders', function()
  local folders = vim.lsp.buf.list_workspace_folders()
  if #folders == 0 then
    vim.notify('No workspace folders configured', vim.log.levels.INFO, { title = 'Workspace' })
  else
    vim.notify(
      'Workspace folders:\n' .. table.concat(folders, '\n'),
      vim.log.levels.INFO,
      { title = 'Workspace' }
    )
  end
end)

-- A language server's arrival brings code's keys (keys.nix) to its buffer.
vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('UserLspConfig', {}),
  callback = function(ev)
    local bufnr = ev.buf

    vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'
    keys.attach('code', bufnr)

    -- Document highlight on cursor hold
    local group = vim.api.nvim_create_augroup('lsp_document_highlight', { clear = false })
    vim.api.nvim_clear_autocmds({ buffer = bufnr, group = group })
    vim.api.nvim_create_autocmd({ 'CursorHold', 'CursorHoldI' }, {
      group = group,
      buffer = bufnr,
      callback = vim.lsp.buf.document_highlight,
    })
    vim.api.nvim_create_autocmd({ 'CursorMoved', 'CursorMovedI' }, {
      group = group,
      buffer = bufnr,
      callback = vim.lsp.buf.clear_references,
    })
  end,
})
