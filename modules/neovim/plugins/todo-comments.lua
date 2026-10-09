-- Highlights and provides navigation for TODO, FIXME, HACK, etc. comments

local ok, todo_comments = pcall(require, 'todo-comments')
if not ok then
  vim.notify('Failed to load todo-comments.nvim', vim.log.levels.ERROR)
  return
end

todo_comments.setup({})

vim.keymap.set('n', ']n', function()
  todo_comments.jump_next()
end, { desc = 'Next note (todo)' })

vim.keymap.set('n', '[n', function()
  todo_comments.jump_prev()
end, { desc = 'Previous note (todo)' })

vim.keymap.set('n', '<leader>st', '<cmd>TodoQuickFix<cr>', {
  desc = 'Search todos',
  silent = true,
})

local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { '[n', icon = '📝' },
    { ']n', icon = '📝' },
    { '<leader>st', icon = '🔍' },
  })
end
