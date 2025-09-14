-- Fugitive Git integration configuration
-- Action-first workflow keybindings

local function map(mode, keys, func, desc)
  vim.keymap.set(mode, keys, func, { desc = desc })
end

-- STATUS (standalone for quick access) - Now handled by fzf-lua

-- Additional COMMIT operations
map('n', '<leader>gcg', ':Git log --graph --oneline<CR>', 'Git commit graph')

-- COMMIT operations (c prefix)
map('n', '<leader>gcc', ':Git commit<CR>', 'Git commit create')
map('n', '<leader>gcn', ':Git commit<CR>', 'Git commit new')
map('n', '<leader>gca', ':Git commit --amend<CR>', 'Git commit amend')
map('n', '<leader>gc!', ':Git commit --amend --no-edit<CR>', 'Git commit amend (no edit)')

-- BRANCH operations (b prefix)
map('n', '<leader>gbc', ':Git checkout ', 'Git branch checkout')
map('n', '<leader>gb-', ':Git checkout -<CR>', 'Git branch previous')
map('n', '<leader>gbn', ':Git checkout -b ', 'Git branch new')
map('n', '<leader>gbd', ':Git branch -d ', 'Git branch delete')
map('n', '<leader>gbD', ':Git branch -D ', 'Git branch delete (force)')

-- ARCHIVE/STASH operations (a prefix - reorganized from gt to ga)
map('n', '<leader>gas', ':Git stash<CR>', 'Git archive save')
map('n', '<leader>gap', ':Git stash pop<CR>', 'Git archive pop')
map('n', '<leader>gal', ':Git stash apply<CR>', 'Git archive apply')
map('n', '<leader>gad', ':Git stash drop<CR>', 'Git archive drop')

-- TAG operations (t prefix - now available for tags)
map('n', '<leader>gtn', function()
  local tag_name = vim.fn.input('Tag name: ')
  if tag_name ~= '' then
    vim.cmd('Git tag ' .. tag_name)
  end
end, 'Git tag new')
map('n', '<leader>gtd', function()
  local tag_name = vim.fn.input('Delete tag: ')
  if tag_name ~= '' then
    vim.cmd('Git tag -d ' .. tag_name)
  end
end, 'Git tag delete')

-- NETWORK operations (n prefix) - sync with remote
map('n', '<leader>gnp', ':Git push<CR>', 'Git network push')
map('n', '<leader>gnl', ':Git pull<CR>', 'Git network pull')
map('n', '<leader>gnf', ':Git fetch<CR>', 'Git network fetch')

-- Quick actions in fugitive status buffer
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'fugitive',
  callback = function()
    local opts = { buffer = true }
    vim.keymap.set('n', 'cc', ':Git commit<CR>', opts)
    vim.keymap.set('n', 'ca', ':Git commit --amend<CR>', opts)
    vim.keymap.set('n', 'ce', ':Git commit --amend --no-edit<CR>', opts)
    vim.keymap.set('n', 'pp', ':Git push<CR>', opts)
    vim.keymap.set('n', 'pf', ':Git push --force-with-lease<CR>', opts)
    vim.keymap.set('n', 'pu', ':Git push -u origin HEAD<CR>', opts)
    vim.keymap.set('n', 'rb', ':Git rebase -i ', opts)
    vim.keymap.set('n', 'ri', ':Git rebase -i HEAD~', opts)
  end,
})

-- Register with which-key if available
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    -- Commit operations
    { '<leader>gcn', desc = 'Git commit new' },
    { '<leader>gca', desc = 'Git commit amend' },
    { '<leader>gc!', desc = 'Git commit amend (no edit)' },
    { '<leader>gcg', desc = 'Git commit graph' },

    -- Branch operations
    { '<leader>gbc', desc = 'Git branch checkout' },
    { '<leader>gb-', desc = 'Git branch previous' },
    { '<leader>gbn', desc = 'Git branch new' },
    { '<leader>gbd', desc = 'Git branch delete' },
    { '<leader>gbD', desc = 'Git branch delete (force)' },

    -- Archive/Stash operations (reorganized from gt to ga)
    { '<leader>gas', desc = 'Git archive save' },
    { '<leader>gap', desc = 'Git archive pop' },
    { '<leader>gal', desc = 'Git archive apply' },
    { '<leader>gad', desc = 'Git archive drop' },

    -- Tag operations (now available in gt)
    { '<leader>gtn', desc = 'Git tag new' },
    { '<leader>gtd', desc = 'Git tag delete' },

    -- Network operations
    { '<leader>gnp', desc = 'Git network push' },
    { '<leader>gnl', desc = 'Git network pull' },
    { '<leader>gnf', desc = 'Git network fetch' },
  })
end

-- In git status window, these mappings are available:
-- - : Stage/unstage file or hunk
-- = : Toggle inline diff
-- > / < : Navigate to next/previous file
-- X : Discard changes
-- cc : Commit
-- ca : Commit amend
-- dd : Diff
-- dv : Vertical diff split
