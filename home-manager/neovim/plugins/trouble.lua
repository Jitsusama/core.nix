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

-- DOCUMENTED: Error Operations (Leader)
map('n', '<leader>ee', function()
  vim.diagnostic.open_float({ border = 'rounded' })
end, 'Error details')

-- DOCUMENTED: Lists using Trouble
map('n', '<leader>el', function()
  trouble.toggle('document_diagnostics')
end, 'Error list')

map('n', '<leader>eL', function()
  trouble.toggle('workspace_diagnostics')
end, 'Error List (workspace)')

map('n', '<leader>eq', function()
  trouble.toggle('quickfix')
end, 'Error quickfix')

map('n', '<leader>ea', function()
  trouble.toggle()
end, 'Error all')

-- DOCUMENTED: Error Toggles
map('n', '<leader>etd', function()
  if vim.diagnostic.is_disabled() then
    vim.diagnostic.enable()
    vim.notify('Diagnostics enabled', vim.log.levels.INFO)
  else
    vim.diagnostic.disable()
    vim.notify('Diagnostics disabled', vim.log.levels.INFO)
  end
end, 'Error toggle diagnostics')

map('n', '<leader>etv', function()
  local current = vim.diagnostic.config().virtual_text
  vim.diagnostic.config({ virtual_text = not current })
  vim.notify('Virtual text ' .. (current and 'disabled' or 'enabled'), vim.log.levels.INFO)
end, 'Error toggle virtual')

map('n', '<leader>ets', function()
  local current = vim.diagnostic.config().signs
  vim.diagnostic.config({ signs = not current })
  vim.notify('Diagnostic signs ' .. (current and 'disabled' or 'enabled'), vim.log.levels.INFO)
end, 'Error toggle signs')

-- DOCUMENTED: Error Navigation (Sequential)
-- [d / ]d - Previous/next diagnostic
map('n', '[d', function()
  vim.diagnostic.goto_prev({ float = { border = 'rounded' } })
end, 'Previous diagnostic')

map('n', ']d', function()
  vim.diagnostic.goto_next({ float = { border = 'rounded' } })
end, 'Next diagnostic')

-- [D / ]D - First/last diagnostic in file  
map('n', '[D', function()
  vim.diagnostic.goto_prev({ float = { border = 'rounded' }, wrap = false })
  -- Go to first diagnostic by going to start and finding first
  vim.cmd('normal! gg')
  vim.diagnostic.goto_next({ float = false, wrap = false })
end, 'First diagnostic')

map('n', ']D', function()
  vim.diagnostic.goto_next({ float = { border = 'rounded' }, wrap = false })
  -- Go to last diagnostic by going to end and finding last  
  vim.cmd('normal! G')
  vim.diagnostic.goto_prev({ float = false, wrap = false })
end, 'Last diagnostic')

-- Register with which-key - DOCUMENTED KEYBINDINGS ONLY
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { '<leader>ee', desc = 'Error details' },
    { '<leader>el', desc = 'Error list' },
    { '<leader>eL', desc = 'Error List (workspace)' },
    { '<leader>eq', desc = 'Error quickfix' },
    { '<leader>ea', desc = 'Error all' },
    { '<leader>etd', desc = 'Error toggle diagnostics' },
    { '<leader>etv', desc = 'Error toggle virtual' },
    { '<leader>ets', desc = 'Error toggle signs' },
  })
end
