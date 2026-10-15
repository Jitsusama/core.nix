-- conform.nvim formats the code, when Space c f asks (keys.nix).
local conform = require('conform')

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

-- Add command to check formatter info
vim.api.nvim_create_user_command('ConformInfo', function()
  require('conform').info()
end, { desc = 'Show conform formatter info' })
