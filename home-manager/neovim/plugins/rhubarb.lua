-- vim-rhubarb integration for GitHub permalink generation
-- Provides file-level GitHub operations using :GBrowse

-- File/Code operations (<leader>ghf) - GitHub permalinks only
vim.keymap.set({ 'n', 'v' }, '<leader>ghfy', ':GBrowse!<CR>', { desc = 'File yank permalink' })
vim.keymap.set({ 'n', 'v' }, '<leader>ghfo', ':GBrowse<CR>', { desc = 'File open browser' })

-- Register with which-key if available
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    -- File operations (both modes)
    { '<leader>ghfy', desc = 'File yank permalink', mode = { 'n', 'v' } },
    { '<leader>ghfo', desc = 'File open browser', mode = { 'n', 'v' } },
  })
end

