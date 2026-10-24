-- Every plugin loaded first, so none replaces the stand-ins after.
require('later').load_all()
local keys = require('keys')
local wrong = {}
local function say(line)
  table.insert(wrong, line)
end
local offered, rows, chosen, told, titled
vim.ui.select = function(items, opts, on_choice)
  offered, rows, titled = {}, #items, opts.prompt
  for _, item in ipairs(items) do
    offered[item.rest] = opts.format_item(item)
  end
  chosen = function(rest)
    for _, item in ipairs(items) do
      if item.rest == rest then
        return on_choice(item)
      end
    end
  end
end
vim.notify = function(message)
  told = message
end

local looks = 0
for _, key in ipairs(keys.keys) do
  if key.chord:match('^Space u %S+$') and key.chord ~= 'Space u Space' then
    looks = looks + 1
  end
end
keys.find('Space u', 'looks')
local count = vim.tbl_count(offered or {})
if count ~= looks then
  say('Space u Space offers ' .. count .. ' keys, not its ' .. looks)
end
local function paint(role, words)
  local hex = require('theme')[role]
  return ('\27[38;2;%d;%d;%dm%s\27[0m'):format(
    tonumber(hex:sub(2, 3), 16),
    tonumber(hex:sub(4, 5), 16),
    tonumber(hex:sub(6, 7), 16),
    words
  )
end
local row = (offered or {}).w or ''
local said = row:match('^' .. vim.pesc(paint('strong', 'Wrap long lines, or not')) .. ' +')
if not said or row ~= said .. paint('text', 'w') then
  say('Space u Space shows w as "' .. vim.inspect(row) .. '"')
end
local columns = {}
for rest, line in pairs(offered or {}) do
  local plain = line:gsub('\27%[[%d;]*m', '')
  columns[#plain - #rest] = true
end
if vim.tbl_count(columns) ~= 1 then
  say('Space u Space puts its keys in ' .. vim.tbl_count(columns) .. ' columns, not one')
end
if titled ~= 'Looks' then
  say('Space u Space is titled "' .. tostring(titled) .. '"')
end
local wrapped = vim.wo.wrap
chosen('w')
if vim.wo.wrap == wrapped then
  say('choosing w from Space u Space left wrapping as it was')
end

-- In an issue, its comments' keys, once each, though octo's other
-- windows give the same chords keys of their own.
vim.cmd('enew')
require('octo.utils').apply_mappings('issue', 0)
offered = nil
keys.find('Space g h c', 'comments')
local comments = vim.tbl_keys(offered or {})
table.sort(comments)
if table.concat(comments, ' ') ~= 'Return h i r w' or rows ~= 5 then
  say(
    'Space g h c Space in an issue offers '
      .. tostring(rows)
      .. ': '
      .. table.concat(comments, ', ')
  )
end
-- And in a review's change, its own comment key, not an issue's.
vim.cmd('enew')
require('octo.utils').apply_mappings('review_diff', 0)
offered = nil
keys.find('Space g h c', 'comments')
local comment = (offered or {}).Return or ''
if not comment:find('Comment on the line', 1, true) then
  say('Space g h c Space in a change offers Return as "' .. comment .. '"')
end
vim.cmd('bwipeout!')
vim.cmd('bwipeout!')

offered = nil
keys.find('Space g s', 'stage')
if offered ~= nil then
  say('Space g s Space offers git keys in a file git does not track')
elseif told ~= 'No key for stage works here' then
  say('Space g s Space with nothing to offer said "' .. tostring(told) .. '"')
end
io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
vim.cmd('qa!')
