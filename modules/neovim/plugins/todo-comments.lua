-- Highlights TODO, FIXME, HACK and the like in comments; [ n and ] n step
-- between them, and Space s t finds every one (keys.nix).

local ok, todo_comments = pcall(require, 'todo-comments')
if not ok then
  vim.notify('Failed to load todo-comments.nvim', vim.log.levels.ERROR)
  return
end

todo_comments.setup({})
