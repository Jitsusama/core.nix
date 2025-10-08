local capabilities = require('cmp_nvim_lsp').default_capabilities()

-- Unified helper function for setting keymaps
-- If first argument is a number, it's treated as a buffer number
local function map(mode_or_bufnr, ...)
  local args = { ... }
  local opts = {}

  if type(mode_or_bufnr) == 'number' then
    local bufnr, mode, keys, func, desc = mode_or_bufnr, args[1], args[2], args[3], args[4]
    opts.buffer = bufnr
    opts.desc = desc
    vim.keymap.set(mode, keys, func, opts)
  else
    local mode, keys, func, desc = mode_or_bufnr, args[1], args[2], args[3]
    opts.desc = desc
    vim.keymap.set(mode, keys, func, opts)
  end
end

-- Nix LSP
vim.lsp.config('nil_ls', {
  cmd = { 'nil' },
  filetypes = { 'nix' },
  root_markers = { 'flake.nix', '.git' },
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
  root_markers = {
    '.luarc.json',
    '.luarc.jsonc',
    '.luacheckrc',
    '.stylua.toml',
    'stylua.toml',
    'selene.toml',
    'selene.yml',
    '.git',
  },
  capabilities = capabilities,
  settings = {
    Lua = {
      runtime = { version = 'LuaJIT' },
      diagnostics = { globals = { 'vim' } },
      workspace = {
        library = vim.api.nvim_get_runtime_file('', true),
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

-- NEW KEYBINDING SYSTEM
-- Navigation uses g prefix, actions use leader prefix

-- LSP keymaps (only when LSP attaches)
vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('UserLspConfig', {}),
  callback = function(ev)
    local bufnr = ev.buf

    vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'

    -- GO-TO NAVIGATION (g prefix) - Handled by fzf-lua plugin

    -- CODE ACTIONS (leader c) - Operations that modify or analyze code
    -- Note: <leader>ca, <leader>cci, <leader>cco now handled by fzf-lua plugin
    map(bufnr, 'n', '<leader>cR', vim.lsp.buf.rename, 'Code Rename')

    -- Code hover (special case - not navigation, but inspection)
    map(bufnr, 'n', '<leader>ch', function()
      vim.lsp.buf.hover({ border = 'rounded', max_width = 80 })
    end, 'Code hover')
    map(bufnr, 'n', '<leader>cH', function()
      vim.lsp.buf.signature_help({ border = 'rounded', max_width = 80 })
    end, 'Code Help (signature)')

    -- Code calls (sub-domain) - Handled by fzf-lua plugin

    -- Code lens operations (sub-domain)
    map(bufnr, 'n', '<leader>clr', vim.lsp.codelens.run, 'Code lens run')
    map(bufnr, 'n', '<leader>clf', vim.lsp.codelens.refresh, 'Code lens refresh')

    -- Code toggles (sub-domain)
    map(bufnr, 'n', '<leader>cti', function()
      local enabled = vim.lsp.inlay_hint.is_enabled()
      vim.lsp.inlay_hint.enable(not enabled)
      vim.notify(enabled and 'Inlay hints disabled' or 'Inlay hints enabled', vim.log.levels.INFO, { title = 'Code' })
    end, 'Code toggle inlay hints')

    -- WORKSPACE MANAGEMENT (<leader>k)
    map(bufnr, 'n', '<leader>ka', vim.lsp.buf.add_workspace_folder, 'Workspace add folder')
    map(bufnr, 'n', '<leader>kr', vim.lsp.buf.remove_workspace_folder, 'Workspace remove folder')
    map(bufnr, 'n', '<leader>kl', function()
      local folders = vim.lsp.buf.list_workspace_folders()
      if #folders == 0 then
        vim.notify('No workspace folders configured', vim.log.levels.INFO, { title = 'Workspace' })
      else
        vim.notify('Workspace folders:\n' .. table.concat(folders, '\n'), vim.log.levels.INFO, { title = 'Workspace' })
      end
    end, 'Workspace list folders')

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

-- LSP server status introspection
vim.keymap.set('n', '<leader>il', function()
  vim.cmd('LspInfo')
end, { desc = 'Introspect LSP' })
