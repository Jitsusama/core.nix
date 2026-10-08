local conform = require('conform')

-- Helper function for setting keymaps
local function map(mode, keys, func, desc)
  vim.keymap.set(mode, keys, func, { desc = desc })
end

-- Configure conform.nvim for formatting
conform.setup({
  -- Define formatters by filetype
  formatters_by_ft = {
    -- Nix files
    nix = { 'nixfmt' },

    -- Lua files
    lua = { 'stylua' },

    -- Web development
    javascript = { 'prettier' },
    javascriptreact = { 'prettier' },
    typescript = { 'prettier' },
    typescriptreact = { 'prettier' },
    json = { 'prettier' },
    jsonc = { 'prettier' },
    yaml = { 'prettier' },
    markdown = { 'prettier' },
    html = { 'prettier' },
    css = { 'prettier' },
    scss = { 'prettier' },

    -- Python
    python = { 'black', 'isort' },

    -- Go
    go = { 'gofmt', 'goimports' },

    -- Rust
    rust = { 'rustfmt' },

    -- Shell scripts
    sh = { 'shfmt' },
    bash = { 'shfmt' },

    -- Use LSP fallback for other filetypes
    ['_'] = { 'trim_whitespace', 'trim_newlines' },
  },

  -- Format on save configuration
  format_on_save = function(bufnr)
    -- Disable format on save for specific filetypes
    local ignore_filetypes = { 'sql', 'java' }
    if vim.tbl_contains(ignore_filetypes, vim.bo[bufnr].filetype) then
      return
    end

    -- Disable with a global variable
    if vim.g.disable_autoformat or vim.b[bufnr].disable_autoformat then
      return
    end

    return {
      timeout_ms = 500,
      lsp_fallback = true,
      quiet = false, -- Show errors if formatting fails
    }
  end,

  -- Configure specific formatters
  formatters = {
    nixfmt = {
      -- Use the Nix-provided nixfmt
      command = 'nixfmt',
    },
    stylua = {
      -- Use the Nix-provided stylua
      command = 'stylua',
      args = {
        '--indent-type',
        'Spaces',
        '--indent-width',
        '2',
        '--quote-style',
        'AutoPreferSingle',
        '-',
      },
    },
    shfmt = {
      prepend_args = { '-i', '2', '-ci' }, -- 2 space indent, indent case statements
    },
  },

  -- Notify on format errors
  notify_on_error = true,
})

-- CODE DOMAIN: Formatting keybindings
map({ 'n', 'v' }, '<leader>cf', function()
  conform.format({
    async = true,
    lsp_fallback = true,
  })
end, 'Code format')

map({ 'n', 'v' }, '<leader>cF', function()
  conform.format({
    formatters = { 'injected' },
    timeout_ms = 3000,
  })
end, 'Code format (injected languages)')

-- Register with which-key
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { '<leader>cf', desc = 'Code format', mode = { 'n', 'v' } },
    { '<leader>cF', desc = 'Code format (injected)', mode = { 'n', 'v' } },
  })
end

-- Add command to check formatter info
vim.api.nvim_create_user_command('ConformInfo', function()
  require('conform').info()
end, { desc = 'Show conform formatter info' })

