local lspconfig = require('lspconfig')
local capabilities = require('cmp_nvim_lsp').default_capabilities()

-- Configure LSP handlers for better UI
vim.lsp.handlers['textDocument/hover'] = vim.lsp.with(vim.lsp.handlers.hover, {
  border = 'rounded',
  max_width = 80,
})

vim.lsp.handlers['textDocument/signatureHelp'] = vim.lsp.with(vim.lsp.handlers.signature_help, {
  border = 'rounded',
  max_width = 80,
})

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
lspconfig.nil_ls.setup((function()
  return {
    capabilities = capabilities,
    settings = {
      ['nil'] = {
        formatting = { command = { 'nixfmt' } },
      },
    },
  }
end)())

-- Lua LSP
lspconfig.lua_ls.setup((function()
  return {
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
  }
end)())

-- Ruby LSP
-- Disabled auto-start as it's now handled by shadowenv configuration
lspconfig.ruby_lsp.setup({
  autostart = false,
})

-- NEW KEYBINDING SYSTEM
-- Navigation uses g prefix, actions use leader prefix

-- LSP keymaps (only when LSP attaches)
vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('UserLspConfig', {}),
  callback = function(ev)
    local bufnr = ev.buf

    vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'

    -- GO-TO NAVIGATION (g prefix) - Direct jumps to locations
    map(bufnr, 'n', 'gd', vim.lsp.buf.definition, 'Go to definition')
    map(bufnr, 'n', 'gD', vim.lsp.buf.declaration, 'Go to declaration')
    map(bufnr, 'n', 'gi', vim.lsp.buf.implementation, 'Go to implementation')
    map(bufnr, 'n', 'gy', vim.lsp.buf.type_definition, 'Go to type definition')
    map(bufnr, 'n', 'gr', vim.lsp.buf.references, 'Go to references')

    -- CODE ACTIONS (leader c) - Operations that modify or analyze code
    map(bufnr, 'n', '<leader>ca', vim.lsp.buf.code_action, 'Code action')
    map(bufnr, 'v', '<leader>ca', vim.lsp.buf.code_action, 'Code action')
    map(bufnr, 'n', '<leader>cR', vim.lsp.buf.rename, 'Code Rename')

    -- Code hover (special case - not navigation, but inspection)
    map(bufnr, 'n', '<leader>ch', function()
      vim.lsp.buf.hover({ border = 'rounded' })
    end, 'Code hover')
    map(bufnr, 'n', '<leader>cH', function()
      vim.lsp.buf.signature_help({ border = 'rounded' })
    end, 'Code Help (signature)')

    -- Code calls (sub-domain)
    map(bufnr, 'n', '<leader>cci', vim.lsp.buf.incoming_calls, 'Code calls incoming')
    map(bufnr, 'n', '<leader>cco', vim.lsp.buf.outgoing_calls, 'Code calls outgoing')

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

