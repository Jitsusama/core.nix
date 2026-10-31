-- Says the file this Neovim has open to the kitty window it runs in, as
-- that window's user variable neovim_file, so Neovim's Recent in the
-- launcher leads with it: open-now reads it back with kitty's remote
-- control (../open-with/src/open.rs). Only a file buffer counts, never a
-- terminal, help or plugin buffer, so going into one keeps the file said;
-- wiping the file said out clears it, and so does leaving. Outside kitty it
-- says nothing.
local M = {}

local said = nil

-- Hands the sequence to the terminal Neovim draws in, which kitty reads
-- the variable from; an unset one is the variable's name alone. Before
-- the terminal's attached nothing can be said, so nothing is kept as said
-- and UIEnter says it.
local function say(path)
  if path == said or #vim.api.nvim_list_uis() == 0 then
    return
  end
  said = path
  local value = path and ('=' .. vim.base64.encode(path)) or ''
  pcall(vim.api.nvim_ui_send, ('\027]1337;SetUserVar=neovim_file%s\007'):format(value))
end

-- The whole path of a buffer holding a file, or nil for any other.
local function fileOf(buf)
  local name = vim.api.nvim_buf_get_name(buf)
  if vim.bo[buf].buftype ~= '' or name == '' or name:find('://', 1, true) then
    return nil
  end
  return vim.fn.fnamemodify(name, ':p')
end

function M.setup()
  if not vim.env.KITTY_WINDOW_ID then
    return
  end
  local group = vim.api.nvim_create_augroup('custom_showing', { clear = true })
  vim.api.nvim_create_autocmd({ 'UIEnter', 'BufEnter', 'BufFilePost' }, {
    group = group,
    callback = function()
      local path = fileOf(vim.api.nvim_get_current_buf())
      if path then
        say(path)
      end
    end,
  })
  vim.api.nvim_create_autocmd('BufWipeout', {
    group = group,
    callback = function(event)
      if said and fileOf(event.buf) == said then
        say(nil)
      end
    end,
  })
  vim.api.nvim_create_autocmd('VimLeavePre', {
    group = group,
    callback = function()
      say(nil)
    end,
  })
end

return M
