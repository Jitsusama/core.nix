-- fzf-lua finds things for most of the keys in keys.nix, and answers every
-- vim.ui.select, so what's chosen from a list is chosen the same way.
local ok, fzf = pcall(require, 'fzf-lua')
if not ok then
  vim.notify('Failed to load fzf-lua', vim.log.levels.ERROR)
  return
end

fzf.setup({
  winopts = {
    preview = {
      scrollbar = 'float', -- float looks better than border
    },
  },
  files = {
    rg_opts = [[ --color=never --files --hidden --follow -g '!.git']],
  },
  live_grep = {
    multiprocess = true,
  },
})

fzf.register_ui_select()
