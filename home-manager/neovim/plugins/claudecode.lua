local claudecode = require('claudecode')

claudecode.setup({
  diff_opts = {
    vertical_split = false, -- Use horizontal splits for better width in Zellij (default: true)
  },
  terminal = {
    split_side = 'left',
    split_width_percentage = 0.50,
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

-- ========================================
-- AI/CLAUDE KEYBINDINGS
-- ========================================

-- Core Operations
vim.keymap.set('n', '<leader>aa', '<cmd>ClaudeCodeFocus<CR>', { desc = 'AI focus terminal', silent = true })
vim.keymap.set({ 'n', 'v' }, '<leader>as', '<cmd>ClaudeCodeSend<CR>', { desc = 'AI send context', silent = true })

-- Diff Management
vim.keymap.set('n', '<leader>ad', '<cmd>ClaudeCodeDiffAccept<CR>', { desc = 'AI diff accept', silent = true })
vim.keymap.set('n', '<leader>ar', '<cmd>ClaudeCodeDiffDeny<CR>', { desc = 'AI diff reject', silent = true })

-- Optional: Model Selection
vim.keymap.set('n', '<leader>am', '<cmd>ClaudeCodeSelectModel<CR>', { desc = 'AI model select', silent = true })

-- ========================================
-- WHICH-KEY INTEGRATION
-- ========================================

-- Load which-key and register AI mappings
local ok, which_key = pcall(require, 'which-key')
if ok then
  which_key.add({
    -- Individual AI mappings with proper icons
    { '<leader>aa', desc = 'AI focus terminal', icon = '🔎' },
    { '<leader>as', desc = 'AI send context', icon = '📤', mode = { 'n', 'v' } },
    { '<leader>ad', desc = 'AI diff accept', icon = '✅' },
    { '<leader>ar', desc = 'AI diff reject', icon = '❌' },
    { '<leader>am', desc = 'AI model select', icon = '⚙️' },
  })
end

