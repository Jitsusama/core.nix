-- Brings one buffer forward in a running Neovim, asked over its own
-- msgpack-RPC server by `open-now neovim go` (neovim.rs) once kitty has
-- brought its window forward: the window showing it in the tab Joel is
-- in, else in the first tab that shows it, else the window he's in given
-- it, since a hidden buffer is shown nowhere. A float he's in, such as a
-- picker, is left for the window under it, so the buffer never lands in
-- one. Answers whether the buffer was still there.
local buffer = ...
if not vim.api.nvim_buf_is_valid(buffer) then
  return false
end

local function tiled(window)
  return vim.api.nvim_win_get_config(window).relative == ''
end

local here = vim.api.nvim_get_current_tabpage()
local chosen = nil
for _, window in ipairs(vim.fn.win_findbuf(buffer)) do
  if tiled(window) and (chosen == nil or vim.api.nvim_win_get_tabpage(window) == here) then
    chosen = window
    if vim.api.nvim_win_get_tabpage(window) == here then
      break
    end
  end
end

if chosen then
  vim.api.nvim_set_current_tabpage(vim.api.nvim_win_get_tabpage(chosen))
  vim.api.nvim_set_current_win(chosen)
else
  if not tiled(0) then
    pcall(vim.cmd.wincmd, 'p')
  end
  vim.api.nvim_set_current_buf(buffer)
end
return true
