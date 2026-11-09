-- Tells the kitty window this Neovim runs in where its own msgpack-RPC
-- server listens, as that window's user variable neovim_server, so the
-- launcher can ask it what it holds and go to one of its files: open-now
-- reads it back with kitty's remote control and asks Neovim there
-- (../open-with/src/open.rs and neovim.rs). It's the server every Neovim
-- starts as it starts, at v:servername, so it's there from the first
-- frame and nothing has to load first. It's said once the terminal has
-- attached and cleared on leaving, so a window left to a shell never
-- names a Neovim that's gone. Outside kitty it says nothing.
local M = {}

local said = nil

-- Hands the sequence to the terminal Neovim draws in, which kitty reads
-- the variable from; an unset one is the variable's name alone. Before
-- the terminal's attached nothing can be said, so nothing is kept as said
-- and UIEnter says it.
local function say(server)
  if server == said or #vim.api.nvim_list_uis() == 0 then
    return
  end
  said = server
  local value = server and ('=' .. vim.base64.encode(server)) or ''
  pcall(vim.api.nvim_ui_send, ('\027]1337;SetUserVar=neovim_server%s\007'):format(value))
end

function M.setup()
  if not vim.env.KITTY_WINDOW_ID then
    return
  end
  local group = vim.api.nvim_create_augroup('custom_showing', { clear = true })
  vim.api.nvim_create_autocmd('UIEnter', {
    group = group,
    callback = function()
      if vim.v.servername ~= '' then
        say(vim.v.servername)
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
