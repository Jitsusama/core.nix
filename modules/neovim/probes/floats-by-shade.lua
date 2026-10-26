require('later').load_all()
local wrong = {}
local function say(line)
  table.insert(wrong, line)
end
local function solid(where, border)
  if border ~= 'solid' then
    say(where .. ' has the border ' .. vim.inspect(border))
  end
end
-- A float Neovim drew: its border all spaces, and nothing through it.
local function look(where, win)
  if not win or not vim.api.nvim_win_is_valid(win) then
    return say(where .. ' opened no float')
  end
  local border = vim.api.nvim_win_get_config(win).border
  local spaces = type(border) == 'table' and #border == 8
  for _, part in ipairs(type(border) == 'table' and border or {}) do
    if (type(part) == 'table' and part[1] or part) ~= ' ' then
      spaces = false
    end
  end
  if not spaces then
    say(where .. ' is drawn with the border ' .. vim.inspect(border))
  end
  if vim.wo[win].winblend ~= 0 then
    say(where .. ' shows what is behind it')
  end
end

if vim.o.pumblend ~= 0 then
  say('the suggestions show what is behind them')
end
local namespace = vim.api.nvim_create_namespace('probe')
vim.diagnostic.set(namespace, 0, {
  { lnum = 0, col = 0, message = 'a problem', severity = vim.diagnostic.severity.WARN },
})
local _, problem = vim.diagnostic.open_float()
look('a problem shown in full', problem)
pcall(vim.api.nvim_win_close, problem, true)
local _, note = vim.lsp.util.open_floating_preview({ 'a note' }, 'markdown', {})
look("a language server's note", note)
pcall(vim.api.nvim_win_close, note, true)

vim.notify('a notice', vim.log.levels.INFO)
local notice
-- A busy machine can take seconds to show it; the wait ends once it has.
vim.wait(60000, function()
  for _, win in ipairs(vim.api.nvim_list_wins()) do
    if vim.bo[vim.api.nvim_win_get_buf(win)].filetype == 'snacks_notif' then
      notice = win
    end
  end
  return notice ~= nil
end)
look('a notice', notice)
if notice then
  local first = vim.api.nvim_buf_get_lines(vim.api.nvim_win_get_buf(notice), 0, 1, false)[1]
  local mark = require('theme').glyphs.note
  if first:sub(1, #mark) ~= mark then
    say('a notice reads "' .. first .. '", not its mark and then its words')
  end
end

local window = require('cmp').get_config().window
solid('the suggestions', window.completion.border)
solid("a suggestion's notes", window.documentation.border)
-- aerial puts its settings in place when first used.
require('aerial').sync_load()
local aerial = require('aerial.config')
solid('the outline', aerial.nav.border)
solid('the outline beside the cursor', aerial.float.border)
if aerial.nav.win_opts.winblend ~= 0 then
  say('the outline shows what is behind it')
end
solid("a change's preview", require('gitsigns.config').config.preview_config.border)

-- The finder: on the floats' shade, nothing dimmed behind it, and
-- its title at the list's edge rather than a space in.
local finder = require('fzf-lua.config').globals.winopts
solid('the finder', finder.border)
solid("the finder's preview", finder.preview.border)
if finder.backdrop ~= false then
  say('the finder dims what is behind it')
end
local shade = vim.api.nvim_get_hl(0, { name = 'NormalFloat', link = false }).bg
if vim.api.nvim_get_hl(0, { name = 'FzfLuaNormal', link = false }).bg ~= shade then
  say("the finder isn't on the floats' shade")
end
local titled = require('fzf-lua.utils').nvim_open_win(vim.api.nvim_create_buf(false, true), false, {
  relative = 'editor',
  row = 1,
  col = 1,
  width = 20,
  height = 2,
  title = { { ' Files ', 'FloatTitle' } },
})
local title = vim.api.nvim_win_get_config(titled).title
if title[1][1] ~= 'Files' then
  say('the finder is titled "' .. title[1][1] .. '", not from its edge')
end

-- While a float has the keys, the status line names the file under it.
vim.api.nvim_set_current_win(titled)
local named = {}
for _, piece in ipairs((require('status').layout(200))) do
  table.insert(named, piece[2])
end
if not table.concat(named):find('probe.lua', 1, true) then
  say('the status line names "' .. table.concat(named) .. '" over a float, not the file under it')
end
io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
vim.cmd('qa!')
