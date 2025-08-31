-- vim-rhubarb integration for GitHub permalink generation
-- Provides file-level GitHub operations using :GBrowse

-- File/Code operations (<leader>ghf) - GitHub permalinks only
vim.keymap.set('n', '<leader>ghfy', function()
  vim.cmd('.GBrowse!')
  print('Copied GitHub permalink to clipboard')
end, { desc = 'File yank permalink' })

vim.keymap.set('v', '<leader>ghfy', function()
  vim.cmd("'<,'>GBrowse!")
  print('Copied GitHub permalink for selection to clipboard')
end, { desc = 'File yank permalink (range)' })

vim.keymap.set('n', '<leader>ghfo', function()
  vim.cmd('.GBrowse')
end, { desc = 'File open browser' })

vim.keymap.set('v', '<leader>ghfo', function()
  vim.cmd("'<,'>GBrowse")
end, { desc = 'File open browser (range)' })

-- Register with which-key if available
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    -- File operations (both modes)
    { '<leader>ghfy', desc = 'File yank permalink', mode = { 'n', 'v' } },
    { '<leader>ghfo', desc = 'File open browser', mode = { 'n', 'v' } },
  })
end