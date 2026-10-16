-- fzf-lua finds things for most of the keys in keys.nix, and answers every
-- vim.ui.select, so what's chosen from a list is chosen the same way.
local ok, fzf = pcall(require, 'fzf-lua')
if not ok then
  vim.notify('Failed to load fzf-lua', vim.log.levels.ERROR)
  return
end

local glyphs = require('theme').glyphs

-- The finder is a float like any other, set apart by its shade with a step
-- of it round the list and the preview, and nothing dimmed behind it. What
-- it finds is its title, at the list's left edge; the status line names the
-- folder, and the chosen row names the file the preview shows.
fzf.setup({
  winopts = {
    border = 'solid',
    backdrop = false,
    title_pos = 'left',
    title_flags = false,
    preview = {
      border = 'solid',
      title = false,
      scrollbar = 'float',
    },
  },
  -- fzf draws in the theme, from the highlights the colour scheme gives.
  fzf_colors = true,
  -- What's typed after the prompt's own mark, the count on the right, and
  -- the chosen row shown by its shade alone.
  fzf_opts = {
    ['--prompt'] = glyphs.prompt .. ' ',
    ['--pointer'] = ' ',
    ['--marker'] = glyphs.done,
  },
  defaults = {
    -- A file by its name, then its folder; its mark in the text's colour
    -- rather than one of its own.
    formatter = 'path.filename_first',
    color_icons = false,
    prompt = glyphs.prompt .. ' ',
  },
  files = {
    cwd_prompt = false,
    rg_opts = [[ --color=never --files --hidden --follow -g '!.git']],
  },
  live_grep = {
    multiprocess = true,
  },
})

-- A choice is a finder like the rest: what's chosen is its title, over the
-- prompt's own mark, and the number fzf-lua keeps to know which was chosen
-- stays out of sight.
fzf.register_ui_select(function(ui_opts)
  local asked = ui_opts.prompt and vim.trim((ui_opts.prompt:gsub(':%s*$', ''))) or ''
  return {
    prompt = glyphs.prompt .. ' ',
    winopts = { title = asked ~= '' and asked or nil },
    fzf_opts = { ['--delimiter'] = '[.] ', ['--with-nth'] = '2..' },
  }
end)

-- fzf-lua pads each title with a space either side, which would set it a
-- step right of the prompt under it. Every finder's window is opened and
-- moved through these two, so its title is trimmed there.
local utils = require('fzf-lua.utils')
local function trimmed(config)
  local title = type(config) == 'table' and config.title
  if type(title) == 'string' then
    config.title = vim.trim(title)
  elseif type(title) == 'table' and #title > 0 then
    title[1][1] = title[1][1]:gsub('^%s+', '')
    title[#title][1] = title[#title][1]:gsub('%s+$', '')
  end
  return config
end
local open, move = utils.nvim_open_win, utils.win_set_config
utils.nvim_open_win = function(buf, enter, config)
  return open(buf, enter, trimmed(config))
end
utils.win_set_config = function(win, config)
  return move(win, trimmed(config))
end
