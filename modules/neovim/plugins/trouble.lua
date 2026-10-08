local trouble = require('trouble')

-- Helper function for setting keymaps
local function map(mode, keys, func, desc)
  vim.keymap.set(mode, keys, func, { desc = desc })
end

-- Fold icons
local icons = {
  chevron_down = vim.fn.nr2char(0xf078),
  chevron_right = vim.fn.nr2char(0xf054),
}

-- Configure Trouble.nvim for better diagnostics display
trouble.setup({
  -- Position of the list (bottom, top, left, right)
  position = 'bottom',
  height = 10, -- height of the trouble list when position is top or bottom
  width = 50, -- width of the list when position is left or right

  -- Icons configuration
  icons = true,
  mode = 'workspace_diagnostics', -- default mode

  -- Fold configuration
  fold_open = icons.chevron_down,
  fold_closed = icons.chevron_right,

  -- Key mappings in Trouble window
  action_keys = {
    close = 'q', -- close the list
    cancel = '<esc>', -- cancel the preview and go back
    refresh = 'r', -- manually refresh
    jump = { '<cr>', '<tab>' }, -- jump to the diagnostic or open / close folds
    open_split = { '<c-x>' }, -- open in a horizontal split
    open_vsplit = { '<c-v>' }, -- open in a vertical split
    open_tab = { '<c-t>' }, -- open in a new tab
    jump_close = { 'o' }, -- jump and close trouble
    toggle_mode = 'm', -- toggle between modes
    toggle_preview = 'P', -- toggle auto preview
    hover = 'K', -- opens a small popup with the full multiline diagnostic
    preview = 'p', -- preview the diagnostic location
    close_folds = { 'zM', 'zm' }, -- close all folds
    open_folds = { 'zR', 'zr' }, -- open all folds
    toggle_fold = { 'zA', 'za' }, -- toggle fold of current file
    previous = 'k', -- previous item
    next = 'j', -- next item
  },

  -- Automatically open & close trouble
  auto_open = false,
  auto_close = false,
  auto_preview = true,
  auto_fold = false,

  -- Use diagnostic signs in the sign column
  use_diagnostic_signs = true,
})

-- ERROR DOMAIN: Diagnostic and error management keybindings
-- Per KEYBINDINGS.md documentation

-- DOCUMENTED: Lists using Trouble (static diagnostic view)
map('n', '<leader>eq', function()
  trouble.toggle('quickfix')
end, 'Error quickfix')

-- Register with which-key - DOCUMENTED KEYBINDINGS ONLY
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { '<leader>eq', desc = 'Error quickfix' },
  })
end
