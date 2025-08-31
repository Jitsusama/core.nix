local claudecode = require('claudecode')

claudecode.setup({
  diff_opts = {
    vertical_split = false, -- Use horizontal splits for better width in Zellij (default: true)
  },
})

local augroup = vim.api.nvim_create_augroup('ClaudeCode', { clear = true })

vim.api.nvim_create_autocmd('FocusGained', {
  group = augroup,
  pattern = '*',
  callback = function()
    vim.cmd('checktime')
  end,
  desc = 'Check for file changes when Neovim gains focus',
})