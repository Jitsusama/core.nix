-- Highlights and provides navigation for TODO, FIXME, HACK, etc. comments

local ok, todo_comments = pcall(require, 'todo-comments')
if not ok then
  vim.notify('Failed to load todo-comments.nvim', vim.log.levels.ERROR)
  return
end

-- Configure todo-comments with minimal changes from defaults
todo_comments.setup({
  colors = {
    error = { 'DiagnosticError', 'ErrorMsg', '#fb4934' }, -- gruvbox red (default: #DC2626)
    warning = { 'DiagnosticWarn', 'WarningMsg', '#fabd2f' }, -- gruvbox yellow (default: #FBBF24)
    info = { 'DiagnosticInfo', '#83a598' }, -- gruvbox blue (default: #2563EB)
    hint = { 'DiagnosticHint', '#8ec07c' }, -- gruvbox aqua (default: #10B981)
    default = { 'Identifier', '#d3869b' }, -- gruvbox purple (default: #7C3AED)
    test = { 'Identifier', '#fe8019' }, -- gruvbox orange (default: #FF00FF)
  },
})

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
